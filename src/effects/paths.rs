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
    pub fn ensure_effects(&self) -> io::Result<()> {
        fs::create_dir_all(&self.effects)?;
        if !self.effects.canonicalize()?.is_dir() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "effects path is not a directory"));
        }
        Ok(())
    }
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
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
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
