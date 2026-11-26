//! File-only store. Keeper unit lifecycle is the caller's responsibility in P4-S3.
use super::{Active, Setup, VERSION, paths::Paths};
use crate::effects::paths::checked_id;
use std::{fs, io, path::Path};

pub struct Store {
    pub paths: Paths,
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
fn plain_file(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("{} is not a plain file", path.display())));
    }
    Ok(())
}
fn finish(errors: Vec<String>) -> io::Result<()> {
    if errors.is_empty() { Ok(()) } else { Err(io::Error::other(errors.join("; "))) }
}
impl Store {
    pub fn load(&self, id: &str) -> io::Result<Setup> {
        self.paths.check()?;
        let path = self.paths.json(id)?;
        plain_file(&path)?;
        let setup: Setup = serde_json::from_slice(&fs::read(path)?)?;
        if setup.version != VERSION || setup.id != id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported setup version or id differs from filename",
            ));
        }
        Ok(setup)
    }
    /// A malformed definition remains visible and individually removable.
    pub fn list(&self) -> io::Result<Vec<(String, io::Result<Setup>)>> {
        self.paths.check()?;
        let entries = match fs::read_dir(&self.paths.setups) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut result = Vec::new();
        for entry in entries {
            let name = entry?.file_name();
            if let Some(id) = name.to_str().and_then(|name| name.strip_suffix(".json"))
                && checked_id(id).is_ok()
            {
                result.push((id.to_owned(), self.load(id)));
            }
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(result)
    }
    pub fn save(&self, setup: &Setup) -> io::Result<()> {
        if setup.version != VERSION {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsupported setup version"));
        }
        let path = self.paths.json(&setup.id)?;
        self.paths.write(&path, &serde_json::to_vec_pretty(setup)?)
    }
    pub fn active(&self) -> io::Result<Option<Active>> {
        self.paths.check()?;
        match plain_file(&self.paths.active) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            result => result?,
        }
        let active: Active = serde_json::from_slice(&fs::read(&self.paths.active)?)?;
        checked_id(&active.id).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(active))
    }
    pub fn set_active(&self, active: &Active) -> io::Result<()> {
        checked_id(&active.id)?;
        self.paths.write(&self.paths.active, &serde_json::to_vec_pretty(active)?)
    }
    pub fn clear_active(&self) -> io::Result<()> {
        self.paths.check()?;
        remove_if_present(&self.paths.active)
    }
    /// Remove by filename even if the JSON is damaged; never follows a symlink.
    pub fn delete(&self, id: &str) -> io::Result<()> {
        self.paths.check()?;
        let file = self.paths.json(id)?;
        let mut errors = Vec::new();
        match self.active() {
            Ok(Some(active)) if active.id == id => {
                if let Err(e) = self.clear_active() {
                    errors.push(format!("active: {e}"));
                }
            }
            Err(e) => errors.push(format!("active: {e}")),
            _ => {}
        }
        if let Err(e) = remove_if_present(&file) {
            errors.push(format!("{}: {e}", file.display()));
        }
        finish(errors)
    }
    /// Removes managed definitions, temporary writes and active marker; foreign
    /// entries remain. Each removal is attempted even after an earlier failure.
    pub fn remove_all(&self) -> io::Result<()> {
        self.paths.check()?;
        let mut errors = Vec::new();
        match fs::read_dir(&self.paths.setups) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let name = entry.file_name();
                            let name = name.to_str().unwrap_or("");
                            let managed = name.strip_suffix(".json").is_some_and(|id| checked_id(id).is_ok())
                                || (name.starts_with(".musubi-") && name.ends_with(".tmp"));
                            if managed && let Err(e) = remove_if_present(&entry.path()) {
                                errors.push(format!("{}: {e}", entry.path().display()));
                            }
                        }
                        Err(e) => errors.push(e.to_string()),
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("{}: {e}", self.paths.setups.display())),
        }
        if let Err(e) = self.clear_active() {
            errors.push(format!("active: {e}"));
        }
        if fs::read_dir(&self.paths.setups).is_ok_and(|mut entries| entries.next().is_none())
            && let Err(e) = fs::remove_dir(&self.paths.setups)
        {
            errors.push(format!("{}: {e}", self.paths.setups.display()));
        }
        finish(errors)
    }
}
