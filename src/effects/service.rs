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
    fn stop(&self, name: &str) -> Result<(), String> {
        let _: OwnedObjectPath = self
            .manager()
            .map_err(|e| e.to_string())?
            .call("StopUnit", &(name, "replace"))
            .map_err(|e| e.to_string())?;
        Ok(())
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
    pub fn delete(&self, id: &str) -> io::Result<()> {
        self.disable(id)?;
        for file in [self.paths.json(id)?, self.paths.conf(id)?] {
            match fs::remove_file(file) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    /// Stops units discovered through systemd *and* local JSON/config files before removing our directory.
    pub fn remove_all(&self) -> io::Result<()> {
        let mut ids = BTreeSet::new();
        let mut temporaries = Vec::new();
        for name in self.units.listed().map_err(system_error)? {
            if let Some(id) = name.strip_prefix("aizome-musubi-effect@").and_then(|s| s.strip_suffix(".service")) {
                if paths::checked_id(id).is_ok() {
                    ids.insert(id.to_owned());
                }
            }
        }
        if self.paths.effects.exists() {
            for entry in fs::read_dir(&self.paths.effects)? {
                let entry = entry?;
                let path = entry.path();
                let temporary = path.file_name().and_then(|s| s.to_str()).is_some_and(|name| {
                    name.strip_prefix(".musubi-").and_then(|s| s.strip_suffix(".tmp")).is_some_and(|s| {
                        s.split_once('-').is_some_and(|(pid, counter)| {
                            !pid.is_empty()
                                && !counter.is_empty()
                                && pid.bytes().chain(counter.bytes()).all(|b| b.is_ascii_digit())
                        })
                    })
                });
                if temporary && entry.file_type()?.is_file() {
                    temporaries.push(path);
                    continue;
                }
                let ext = path.extension().and_then(|s| s.to_str());
                if !matches!(ext, Some("json" | "conf")) || !entry.file_type()?.is_file() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "unexpected file in effects directory"));
                }
                let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                paths::checked_id(id)?;
                ids.insert(id.to_owned());
            }
        }
        for id in ids {
            self.delete(&id)?;
        }
        for path in temporaries {
            fs::remove_file(path)?;
        }
        if self.paths.effects.exists() {
            fs::remove_dir(&self.paths.effects)?;
        }
        match fs::remove_file(&self.paths.clock) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Default)]
    struct Fake {
        calls: RefCell<Vec<String>>,
        units: RefCell<Vec<String>>,
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
            Ok(())
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
