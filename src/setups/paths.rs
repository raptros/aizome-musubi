//! D22 managed data paths; use the effects atomic writer and id convention.
use crate::effects::paths::{atomic_write, checked_id};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub setups: PathBuf,
    pub active: PathBuf,
}

impl Paths {
    pub fn from_environment() -> io::Result<Self> {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data home"))?;
        Ok(Self::under(data))
    }
    pub fn under(data: PathBuf) -> Self {
        let root = data.join("aizome/musubi");
        Self { setups: root.join("setups"), active: root.join("active.json"), root }
    }
    pub fn json(&self, id: &str) -> io::Result<PathBuf> {
        Ok(self.setups.join(format!("{}.json", checked_id(id)?)))
    }
    pub fn check(&self) -> io::Result<()> {
        for dir in [self.root.parent().expect("aizome parent"), self.root.as_path(), self.setups.as_path()] {
            match fs::symlink_metadata(dir) {
                Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
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
    pub fn ensure(&self) -> io::Result<()> {
        self.check()?;
        for dir in [self.root.parent().expect("aizome parent"), self.root.as_path(), self.setups.as_path()] {
            match fs::create_dir(dir) {
                Ok(()) => {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    fs::create_dir_all(dir)?;
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
                }
                Err(e) => return Err(e),
            }
        }
        self.check()
    }
    pub fn write(&self, path: &Path, body: &[u8]) -> io::Result<()> {
        self.ensure()?;
        atomic_write(path, body)
    }
}
