//! All user-configuration paths owned by the effects and clock features.
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct Paths {
    pub effects: PathBuf,
    pub clock: PathBuf,
}

impl Paths {
    pub fn from_environment() -> io::Result<Self> {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no configuration home"))?;
        Ok(Self::under(config))
    }
    pub fn under(config: PathBuf) -> Self {
        Self {
            effects: config.join("aizome-musubi/effects"),
            clock: config.join("pipewire/pipewire.conf.d/90-aizome-musubi-clock.conf"),
        }
    }
    pub fn json(&self, id: &str) -> io::Result<PathBuf> {
        Ok(self.effects.join(format!("{}.json", checked_id(id)?)))
    }
    pub fn conf(&self, id: &str) -> io::Result<PathBuf> {
        Ok(self.effects.join(format!("{}.conf", checked_id(id)?)))
    }
    /// Create Musubi's own directories (0700). Refuses a managed directory
    /// that is a symlink: writes and cleanup must never follow one elsewhere.
    pub fn ensure_effects(&self) -> io::Result<()> {
        self.check_effects()?;
        let owned = self.effects.parent().unwrap_or(&self.effects);
        for dir in [owned, self.effects.as_path()] {
            match fs::create_dir(dir) {
                Ok(()) => set_mode(dir, 0o700)?,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    fs::create_dir_all(dir)?;
                    set_mode(dir, 0o700)?;
                }
                Err(e) => return Err(e),
            }
        }
        self.check_effects()
    }
    /// Musubi's directories (`aizome-musubi`, `aizome-musubi/effects`) are
    /// absent or real directories, never symlinks.
    pub fn check_effects(&self) -> io::Result<()> {
        let owned = self.effects.parent().unwrap_or(&self.effects);
        for dir in [owned, self.effects.as_path()] {
            match fs::symlink_metadata(dir) {
                Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("{} is not a plain directory; Musubi won't write or delete through it", dir.display()),
                    ));
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

pub fn checked_id(id: &str) -> io::Result<&str> {
    if id.is_empty() || id.len() > 64 || !id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "chain id must contain only lowercase ASCII, digits or hyphens",
        ));
    }
    Ok(id)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".musubi-{}-{}.tmp", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let result = (|| {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        fs::File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn writes_stay_in_owned_paths() {
        let p = Paths::under(PathBuf::from("/tmp/musubi-path-test"));
        for id in ["../escape", "a.b", "", "a/b", "x@name", "UPPER"] {
            assert!(p.json(id).is_err());
            assert!(p.conf(id).is_err());
        }
        for id in ["eq", "eq-02"] {
            for path in [p.json(id).unwrap(), p.conf(id).unwrap()] {
                assert_eq!(path.parent(), Some(p.effects.as_path()));
                assert!(matches!(path.extension().and_then(|e| e.to_str()), Some("json" | "conf")));
            }
        }
        assert_eq!(
            p.clock,
            PathBuf::from("/tmp/musubi-path-test/pipewire/pipewire.conf.d/90-aizome-musubi-clock.conf")
        );
    }
}
