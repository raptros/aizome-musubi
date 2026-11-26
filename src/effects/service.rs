//! User systemd supervision. No process inherits the host's PipeWire session from here.
use super::{
    Chain,
    paths::{self, Paths},
};
use std::{collections::BTreeSet, fs, io};
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::OwnedObjectPath,
};

const DEST: &str = "org.freedesktop.systemd1";
const MANAGER: &str = "/org/freedesktop/systemd1";
const IFACE: &str = "org.freedesktop.systemd1.Manager";

pub trait Units {
    fn enable(&self, name: &str) -> Result<(), String>;
    fn start(&self, name: &str) -> Result<(), String>;
    fn stop(&self, name: &str) -> Result<(), String>;
    fn disable(&self, name: &str) -> Result<(), String>;
    fn listed(&self) -> Result<Vec<String>, String>;
    fn state(&self, name: &str) -> Result<UnitState, String>;
    /// Whether `name` is an instance of Musubi's own template (not a
    /// same-named unit someone else wrote).
    fn owned(&self, name: &str) -> Result<bool, String>;
    /// Clear a failed or start-limited state so a start is accepted again.
    fn reset_failed(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }
}

impl Units for Box<dyn Units> {
    fn enable(&self, name: &str) -> Result<(), String> {
        (**self).enable(name)
    }
    fn start(&self, name: &str) -> Result<(), String> {
        (**self).start(name)
    }
    fn stop(&self, name: &str) -> Result<(), String> {
        (**self).stop(name)
    }
    fn disable(&self, name: &str) -> Result<(), String> {
        (**self).disable(name)
    }
    fn listed(&self) -> Result<Vec<String>, String> {
        (**self).listed()
    }
    fn state(&self, name: &str) -> Result<UnitState, String> {
        (**self).state(name)
    }
    fn owned(&self, name: &str) -> Result<bool, String> {
        (**self).owned(name)
    }
    fn reset_failed(&self, name: &str) -> Result<(), String> {
        (**self).reset_failed(name)
    }
}

#[derive(Clone, Debug, Default)]
pub struct UnitState {
    pub active: String,
    pub sub: String,
    pub result: String,
}

pub struct Systemd {
    connection: Connection,
}
impl Systemd {
    pub fn user() -> zbus::Result<Self> {
        Ok(Self { connection: Connection::session()? })
    }
    fn manager(&self) -> zbus::Result<Proxy<'_>> {
        Proxy::new(&self.connection, DEST, MANAGER, IFACE)
    }
}
impl Units for Systemd {
    fn enable(&self, name: &str) -> Result<(), String> {
        let _: (bool, Vec<(String, String, String)>) = self
            .manager()
            .map_err(|e| e.to_string())?
            .call("EnableUnitFiles", &(vec![name], false, true))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn start(&self, name: &str) -> Result<(), String> {
        let _: OwnedObjectPath = self
            .manager()
            .map_err(|e| e.to_string())?
            .call("StartUnit", &(name, "replace"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    /// `StopUnit` only queues a job: wait until the unit is inactive (or
    /// failed, or unloaded) before reporting it stopped.
    fn stop(&self, name: &str) -> Result<(), String> {
        let _: OwnedObjectPath = self
            .manager()
            .map_err(|e| e.to_string())?
            .call("StopUnit", &(name, "replace"))
            .map_err(|e| e.to_string())?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            match self.state(name) {
                Ok(state) if matches!(state.active.as_str(), "inactive" | "failed") => return Ok(()),
                Err(_) => return Ok(()), // unloaded: nothing of it runs
                Ok(state) if std::time::Instant::now() >= deadline => {
                    return Err(format!("{name} did not stop (still {})", state.active));
                }
                Ok(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }
    }
    fn disable(&self, name: &str) -> Result<(), String> {
        let _: Vec<(String, String, String)> = self
            .manager()
            .map_err(|e| e.to_string())?
            .call("DisableUnitFiles", &(vec![name], false))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn listed(&self) -> Result<Vec<String>, String> {
        let entries: Vec<(String, String)> =
            self.manager().map_err(|e| e.to_string())?.call("ListUnitFiles", &()).map_err(|e| e.to_string())?;
        Ok(entries.into_iter().map(|(name, _)| name).collect())
    }
    fn state(&self, name: &str) -> Result<UnitState, String> {
        let path: OwnedObjectPath =
            self.manager().map_err(|e| e.to_string())?.call("GetUnit", &(name,)).map_err(|e| e.to_string())?;
        let unit =
            Proxy::new(&self.connection, DEST, path, "org.freedesktop.systemd1.Unit").map_err(|e| e.to_string())?;
        let active = unit.get_property("ActiveState").map_err(|e| e.to_string())?;
        let sub = unit.get_property("SubState").map_err(|e| e.to_string())?;
        let service = Proxy::new(&self.connection, DEST, unit.path(), "org.freedesktop.systemd1.Service")
            .map_err(|e| e.to_string())?;
        let result = service.get_property("Result").map_err(|e| e.to_string())?;
        Ok(UnitState { active, sub, result })
    }
    fn owned(&self, name: &str) -> Result<bool, String> {
        let path: OwnedObjectPath =
            self.manager().map_err(|e| e.to_string())?.call("LoadUnit", &(name,)).map_err(|e| e.to_string())?;
        let unit =
            Proxy::new(&self.connection, DEST, path, "org.freedesktop.systemd1.Unit").map_err(|e| e.to_string())?;
        let fragment: String = unit.get_property("FragmentPath").map_err(|e| e.to_string())?;
        let description: String = unit.get_property("Description").map_err(|e| e.to_string())?;
        Ok((fragment.ends_with("/aizome-musubi-effect@.service") && description.starts_with("Musubi EQ effect"))
            || (name == crate::setups::keeper::UNIT
                && fragment.ends_with("/aizome-musubi-keeper.service")
                && description == "Musubi setup keeper"))
    }
    fn reset_failed(&self, name: &str) -> Result<(), String> {
        let _: () =
            self.manager().map_err(|e| e.to_string())?.call("ResetFailedUnit", &(name,)).map_err(|e| e.to_string())?;
        Ok(())
    }
}
/// Stop and disable are both attempted even if the other fails (D22).
pub fn keeper_unit<U: Units>(units: &U, enabled: bool) -> Result<(), String> {
    let name = crate::setups::keeper::UNIT;
    if !units.owned(name)? {
        return Err(format!("{name} is not Musubi's unit"));
    }
    if enabled {
        units.enable(name)?;
        units.start(name)
    } else {
        let mut errors = Vec::new();
        if let Err(error) = units.stop(name) {
            errors.push(error);
        }
        if let Err(error) = units.disable(name) {
            errors.push(error);
        }
        if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
    }
}

pub fn unit(id: &str) -> io::Result<String> {
    Ok(format!("aizome-musubi-effect@{}.service", paths::checked_id(id)?))
}
fn system_error(error: String) -> io::Error {
    io::Error::other(error)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Running,
    NodesMissing,
    TargetMissing,
    Failed(String),
    StartLimit,
    Stopped,
}
pub fn derive_status(state: &UnitState, nodes_present: bool, target_present: bool) -> Status {
    if state.result == "start-limit-hit" {
        Status::StartLimit
    } else if state.active == "failed" {
        Status::Failed(state.result.clone())
    } else if state.active != "active" {
        Status::Stopped
    } else if !nodes_present {
        Status::NodesMissing
    } else if !target_present {
        Status::TargetMissing
    } else {
        Status::Running
    }
}

pub struct EffectsService<U: Units> {
    pub paths: Paths,
    pub units: U,
}
impl<U: Units> EffectsService<U> {
    pub fn save(&self, chain: &Chain) -> io::Result<()> {
        chain.validate()?;
        self.paths.ensure_effects()?;
        paths::atomic_write(&self.paths.json(&chain.id)?, &serde_json::to_vec_pretty(chain)?)?;
        paths::atomic_write(&self.paths.conf(&chain.id)?, chain.config()?.as_bytes())
    }
    pub fn load(&self, id: &str) -> io::Result<Chain> {
        let chain: Chain = serde_json::from_slice(&fs::read(self.paths.json(id)?)?)?;
        chain.validate()?;
        if chain.id != id {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "chain id differs from filename"));
        }
        Ok(chain)
    }
    /// Enumerate only chain JSON files from Musubi's effects directory.
    pub fn list(&self) -> io::Result<Vec<Chain>> {
        let entries = match fs::read_dir(&self.paths.effects) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut chains = Vec::new();
        for entry in entries {
            let name = entry?.file_name();
            if std::path::Path::new(&name).extension().is_some_and(|ext| ext == "json") {
                let name = name
                    .into_string()
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 chain filename"))?;
                let id = name.strip_suffix(".json").expect("JSON suffix");
                chains.push(self.load(id)?);
            }
        }
        chains.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(chains)
    }
    /// Submit a band change to the existing backend; persist only after Props read-back.
    pub fn edit_band(
        &self,
        backend: &mut impl crate::backend::Backend,
        capture: crate::backend::Handle,
        request: crate::backend::RequestId,
        id: &str,
        index: usize,
        band: super::Band,
    ) -> io::Result<Chain> {
        let mut chain = self.load(id)?;
        let Some(slot) = chain.bands.get_mut(index) else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "band index out of range"));
        };
        *slot = band.clone();
        chain.validate()?;
        let gain = if chain.bypass { 0.0 } else { band.gain };
        let mut params = Vec::with_capacity(6);
        for side in ["left", "right"] {
            for (key, value) in [("Freq", band.freq), ("Q", band.q), ("Gain", gain)] {
                params.push((format!("{side}{index}:{key}"), value));
            }
        }
        backend.command(crate::backend::Command::SetEffectParams { node: capture, params, request });
        Ok(chain)
    }
    /// Call after command success and the capture node's subsequent observed Props delta.
    pub fn save_observed_edit(&self, chain: &Chain, index: usize, observed: &[(String, f32)]) -> io::Result<()> {
        let band = chain.bands.get(index).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "band index"))?;
        let gain = if chain.bypass { 0.0 } else { band.gain };
        for side in ["left", "right"] {
            for (key, expected) in [("Freq", band.freq), ("Q", band.q), ("Gain", gain)] {
                let name = format!("{side}{index}:{key}");
                if !observed.iter().any(|(actual, value)| *actual == name && (*value - expected).abs() < 0.001) {
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, "effect read-back has not matched"));
                }
            }
        }
        self.save(chain)
    }
    pub fn enable(&self, chain: &Chain) -> io::Result<()> {
        let mut chain = chain.clone();
        chain.enabled = true;
        self.save(&chain)?;
        let name = unit(&chain.id)?;
        self.units.enable(&name).map_err(system_error)?;
        self.units.start(&name).map_err(system_error)
    }
    pub fn disable(&self, id: &str) -> io::Result<()> {
        let name = unit(id)?;
        self.units.stop(&name).map_err(system_error)?;
        self.units.disable(&name).map_err(system_error)?;
        if self.paths.json(id)?.exists() {
            let mut chain = self.load(id)?;
            chain.enabled = false;
            self.save(&chain)?;
        }
        Ok(())
    }
    /// Stop (and wait for) and disable `id`'s unit, then remove its two
    /// files. Never parses the chain, so a damaged one is still deleted;
    /// tries every step and reports every failure together.
    pub fn delete(&self, id: &str) -> io::Result<()> {
        let mut errors = Vec::new();
        self.delete_into(id, &mut errors);
        finish(errors)
    }
    fn delete_into(&self, id: &str, errors: &mut Vec<String>) {
        let name = match unit(id) {
            Ok(name) => name,
            Err(e) => return errors.push(format!("{id}: {e}")),
        };
        for (step, result) in [("stop", self.units.stop(&name)), ("disable", self.units.disable(&name))] {
            if let Err(e) = result
                && !unit_absent(&e)
            {
                errors.push(format!("{step} {name}: {e}"));
            }
        }
        if let Err(e) = self.paths.check_effects() {
            return errors.push(e.to_string());
        }
        for file in [self.paths.json(id), self.paths.conf(id)].into_iter().flatten() {
            if let Err(e) = remove_if_present(&file) {
                errors.push(format!("{}: {e}", file.display()));
            }
        }
    }
    /// Units to clean up: instances of Musubi's template, plus any unit
    /// named for a chain Musubi has files for. A same-named unit someone
    /// else wrote is left alone.
    fn managed_units(&self, errors: &mut Vec<String>) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        match self.units.listed() {
            Ok(names) => {
                for name in names {
                    let Some(id) = name.strip_prefix("aizome-musubi-effect@").and_then(|s| s.strip_suffix(".service"))
                    else {
                        continue;
                    };
                    if paths::checked_id(id).is_err() {
                        continue;
                    }
                    let has_files = [self.paths.json(id), self.paths.conf(id)]
                        .into_iter()
                        .flatten()
                        .any(|p| fs::symlink_metadata(p).is_ok());
                    match self.units.owned(&name) {
                        Ok(owned) if owned || has_files => {
                            ids.insert(id.to_owned());
                        }
                        Ok(_) => {}
                        Err(e) if has_files => {
                            errors.push(format!("checking {name}: {e}"));
                            ids.insert(id.to_owned());
                        }
                        Err(e) => errors.push(format!("checking {name}: {e}")),
                    }
                }
            }
            Err(e) => errors.push(format!("listing units: {e}")),
        }
        ids
    }
    /// Musubi's chain files and temporaries in the effects directory (no
    /// symlinks followed), and the ids they name. Other entries are kept.
    fn managed_files(&self, errors: &mut Vec<String>) -> (BTreeSet<String>, Vec<std::path::PathBuf>) {
        let mut ids = BTreeSet::new();
        let mut temporaries = Vec::new();
        if let Err(e) = self.paths.check_effects() {
            errors.push(e.to_string());
            return (ids, temporaries);
        }
        let entries = match fs::read_dir(&self.paths.effects) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return (ids, temporaries),
            Err(e) => {
                errors.push(format!("{}: {e}", self.paths.effects.display()));
                return (ids, temporaries);
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if is_temporary(name) {
                temporaries.push(path);
            } else if let Some(id) = name.strip_suffix(".json").or_else(|| name.strip_suffix(".conf"))
                && paths::checked_id(id).is_ok()
            {
                ids.insert(id.to_owned());
            }
        }
        (ids, temporaries)
    }
    /// Exactly what `remove_all` will touch, for its confirmation.
    pub fn removal_manifest(&self) -> io::Result<Vec<String>> {
        let mut errors = Vec::new();
        let mut ids = self.managed_units(&mut errors);
        let (files, temporaries) = self.managed_files(&mut errors);
        ids.extend(files);
        let mut resources = BTreeSet::new();
        for id in ids {
            resources.insert(unit(&id)?);
            for file in [self.paths.json(&id)?, self.paths.conf(&id)?] {
                if fs::symlink_metadata(&file).is_ok() {
                    resources.insert(file.display().to_string());
                }
            }
        }
        resources.extend(temporaries.iter().map(|p| p.display().to_string()));
        if fs::symlink_metadata(&self.paths.clock).is_ok() {
            resources.insert(self.paths.clock.display().to_string());
        }
        Ok(resources.into_iter().collect())
    }
    /// Stop, disable and remove every chain Musubi manages, its temporaries,
    /// its directories once empty, and the clock drop-in. Tries everything;
    /// keeps files it doesn't own; reports every failure at the end.
    pub fn remove_all(&self) -> io::Result<()> {
        let mut errors = Vec::new();
        let mut ids = self.managed_units(&mut errors);
        let (files, temporaries) = self.managed_files(&mut errors);
        ids.extend(files);
        for id in &ids {
            self.delete_into(id, &mut errors);
        }
        if self.paths.check_effects().is_ok() {
            for path in temporaries {
                if let Err(e) = remove_if_present(&path) {
                    errors.push(format!("{}: {e}", path.display()));
                }
            }
            let owned = self.paths.effects.parent().unwrap_or(&self.paths.effects);
            for dir in [self.paths.effects.as_path(), owned] {
                // Left in place when it still holds files Musubi didn't make.
                if fs::read_dir(dir).is_ok_and(|mut d| d.next().is_none())
                    && let Err(e) = fs::remove_dir(dir)
                {
                    errors.push(format!("{}: {e}", dir.display()));
                }
            }
        }
        if let Err(e) = remove_if_present(&self.paths.clock) {
            errors.push(format!("{}: {e}", self.paths.clock.display()));
        }
        finish(errors)
    }
}

fn finish(errors: Vec<String>) -> io::Result<()> {
    if errors.is_empty() { Ok(()) } else { Err(io::Error::other(errors.join("; "))) }
}

fn remove_if_present(path: &std::path::Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// A chain whose unit was never enabled, or a system without the template:
/// nothing to stop or disable.
fn unit_absent(error: &str) -> bool {
    error.contains("NoSuchUnit") || error.contains("not loaded") || error.contains("does not exist")
}

fn is_temporary(name: &str) -> bool {
    name.strip_prefix(".musubi-").and_then(|s| s.strip_suffix(".tmp")).is_some_and(|s| {
        s.split_once('-').is_some_and(|(pid, counter)| {
            !pid.is_empty() && !counter.is_empty() && pid.bytes().chain(counter.bytes()).all(|b| b.is_ascii_digit())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Default)]
    struct Fake {
        calls: RefCell<Vec<String>>,
        units: RefCell<Vec<String>>,
        /// Same-named units Musubi didn't create.
        foreign: RefCell<Vec<String>>,
        /// Units whose stop fails.
        stuck: RefCell<Vec<String>>,
    }
    impl Units for Fake {
        fn enable(&self, n: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("enable {n}"));
            self.units.borrow_mut().push(n.into());
            Ok(())
        }
        fn start(&self, n: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("start {n}"));
            Ok(())
        }
        fn stop(&self, n: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("stop {n}"));
            if self.stuck.borrow().iter().any(|u| u == n) { Err(format!("{n} did not stop")) } else { Ok(()) }
        }
        fn disable(&self, n: &str) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("disable {n}"));
            self.units.borrow_mut().retain(|u| u != n);
            Ok(())
        }
        fn listed(&self) -> Result<Vec<String>, String> {
            Ok(self.units.borrow().clone())
        }
        fn state(&self, _: &str) -> Result<UnitState, String> {
            Ok(UnitState::default())
        }
        fn owned(&self, n: &str) -> Result<bool, String> {
            Ok(!self.foreign.borrow().iter().any(|u| u == n))
        }
    }
    fn chain(id: &str) -> Chain {
        Chain {
            id: id.into(),
            name: id.into(),
            target: super::super::Target::FollowDefault,
            bands: vec![],
            bypass: false,
            enabled: true,
        }
    }
    fn scratch(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("musubi-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }
    #[test]
    fn damaged_chain_still_deleted() {
        let root = scratch("damaged");
        let service = EffectsService { paths: Paths::under(root.clone()), units: Fake::default() };
        service.enable(&chain("one")).unwrap();
        fs::write(service.paths.json("one").unwrap(), "{ not json").unwrap();
        service.delete("one").unwrap();
        assert!(!service.paths.json("one").unwrap().exists());
        assert!(!service.paths.conf("one").unwrap().exists());
        assert!(service.units.units.borrow().is_empty());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn remove_all_continues_past_failures_and_keeps_what_it_does_not_own() {
        let root = scratch("remove-all");
        let service = EffectsService { paths: Paths::under(root.clone()), units: Fake::default() };
        service.enable(&chain("one")).unwrap();
        service.enable(&chain("two")).unwrap();
        // A stop that fails, a stray file, a unit someone else wrote, the drop-in.
        service.units.stuck.borrow_mut().push("aizome-musubi-effect@one.service".into());
        fs::write(service.paths.effects.join("notes.txt"), "mine").unwrap();
        service.units.units.borrow_mut().push("aizome-musubi-effect@theirs.service".into());
        service.units.foreign.borrow_mut().push("aizome-musubi-effect@theirs.service".into());
        fs::create_dir_all(service.paths.clock.parent().unwrap()).unwrap();
        fs::write(&service.paths.clock, "musubi").unwrap();
        let error = service.remove_all().unwrap_err().to_string();
        assert!(error.contains("one.service did not stop"), "{error}");
        // Every other step still ran.
        for id in ["one", "two"] {
            assert!(!service.paths.json(id).unwrap().exists());
            assert!(!service.paths.conf(id).unwrap().exists());
        }
        assert!(!service.paths.clock.exists());
        let calls = service.units.calls.borrow();
        assert!(calls.iter().any(|c| c == "disable aizome-musubi-effect@two.service"));
        assert!(!calls.iter().any(|c| c.contains("@theirs")), "{calls:?}");
        assert_eq!(fs::read_to_string(service.paths.effects.join("notes.txt")).unwrap(), "mine");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn symlinked_effects_directory_is_never_followed() {
        let root = scratch("symlink");
        let elsewhere = root.join("elsewhere");
        fs::create_dir_all(&elsewhere).unwrap();
        fs::write(elsewhere.join("victim.json"), "keep").unwrap();
        fs::write(elsewhere.join("victim.conf"), "keep").unwrap();
        let config = root.join("config");
        fs::create_dir_all(config.join("aizome-musubi")).unwrap();
        std::os::unix::fs::symlink(&elsewhere, config.join("aizome-musubi/effects")).unwrap();
        let service = EffectsService { paths: Paths::under(config), units: Fake::default() };
        assert!(service.save(&chain("one")).is_err());
        assert!(service.remove_all().is_err());
        assert!(service.delete("victim").is_err());
        assert_eq!(fs::read_to_string(elsewhere.join("victim.json")).unwrap(), "keep");
        assert_eq!(fs::read_to_string(elsewhere.join("victim.conf")).unwrap(), "keep");
        assert!(!elsewhere.join("one.json").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn managed_files_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let root = scratch("private");
        let service = EffectsService { paths: Paths::under(root.clone()), units: Fake::default() };
        service.save(&chain("one")).unwrap();
        let mode = |p: &std::path::Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&service.paths.json("one").unwrap()), 0o600);
        assert_eq!(mode(&service.paths.effects), 0o700);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn status_is_observed() {
        let state = UnitState { active: "active".into(), ..Default::default() };
        assert_eq!(derive_status(&state, true, true), Status::Running);
        assert_eq!(derive_status(&state, false, true), Status::NodesMissing);
        assert_eq!(derive_status(&state, true, false), Status::TargetMissing);
        assert_eq!(
            derive_status(
                &UnitState { active: "failed".into(), result: "exit-code".into(), ..Default::default() },
                false,
                false
            ),
            Status::Failed("exit-code".into())
        );
        assert_eq!(
            derive_status(&UnitState { result: "start-limit-hit".into(), ..Default::default() }, false, false),
            Status::StartLimit
        );
    }
    #[test]
    fn cleanup_is_scoped_and_ordered() {
        let root = std::env::temp_dir().join(format!(
            "musubi-effects-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let service = EffectsService { paths: Paths::under(root.clone()), units: Fake::default() };
        let chain = Chain {
            id: "one".into(),
            name: "One".into(),
            target: super::super::Target::FollowDefault,
            bands: vec![],
            bypass: false,
            enabled: true,
        };
        assert!(service.list().unwrap().is_empty());
        service.enable(&chain).unwrap();
        assert!(service.paths.conf("one").unwrap().exists());
        assert_eq!(service.list().unwrap()[0].id, "one");
        service.delete("one").unwrap();
        assert!(!service.paths.json("one").unwrap().exists());
        assert!(!service.paths.conf("one").unwrap().exists());
        assert!(service.list().unwrap().is_empty());
        service.enable(&chain).unwrap();
        fs::write(service.paths.effects.join(".musubi-123-0.tmp"), "incomplete").unwrap();
        fs::create_dir_all(service.paths.clock.parent().unwrap()).unwrap();
        fs::write(&service.paths.clock, "musubi").unwrap();
        fs::write(root.join("other.conf"), "keep").unwrap();
        service.remove_all().unwrap();
        assert!(!service.paths.effects.exists());
        assert!(service.list().unwrap().is_empty());
        assert!(!service.paths.clock.exists());
        assert_eq!(fs::read_to_string(root.join("other.conf")).unwrap(), "keep");
        assert!(service.units.units.borrow().is_empty());
        let calls = service.units.calls.borrow();
        assert!(calls.windows(2).any(|pair| pair[0].starts_with("stop ") && pair[1].starts_with("disable ")));
        fs::remove_dir_all(root).unwrap();
    }
}
