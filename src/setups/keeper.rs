//! Headless setup enforcement. The backend owns its own PipeWire thread and reconnects.
use super::{
    Setup,
    paths::Paths,
    reconcile::{self, Action},
    store::Store,
};
use crate::{
    backend::{Backend, Command, Connection, Event, Handle, pipewire::PipeWireBackend},
    graph::{Graph, identity::Resolution},
};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io,
    os::fd::AsRawFd,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime},
};

pub const UNIT: &str = "aizome-musubi-keeper.service";
static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn signal_stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

/// A nonblocking advisory lock: only its holder can enforce. Keep the File alive.
pub struct Handover {
    file: File,
}
impl Handover {
    pub fn path() -> io::Result<PathBuf> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .filter(|s| !s.is_empty())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "XDG_RUNTIME_DIR is required"))?;
        let runtime = PathBuf::from(runtime);
        let dir = runtime.join("aizome-musubi");
        match fs::symlink_metadata(&dir) {
            Ok(meta) if !meta.is_dir() || meta.file_type().is_symlink() => {
                return Err(io::Error::other("unsafe setup lock directory"));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&dir)?;
                fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
            }
            Err(e) => return Err(e),
            _ => {}
        }
        if fs::metadata(&dir)?.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::other("setup lock directory is not private"));
        }
        Ok(dir.join("setups.lock"))
    }
    pub fn try_acquire(path: &Path) -> io::Result<Option<Self>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("unsafe setup lock file"));
        }
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == 0 {
            Ok(Some(Self { file }))
        } else {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::WouldBlock { Ok(None) } else { Err(e) }
        }
    }
    pub fn acquire(path: &Path) -> io::Result<Self> {
        loop {
            if let Some(lock) = Self::try_acquire(path)? {
                return Ok(lock);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
impl Drop for Handover {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[derive(PartialEq, Eq)]
struct Stamp {
    active: Option<SystemTime>,
    setup: Option<SystemTime>,
    id: Option<String>,
}
fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}
fn reload(store: &Store, stamp: &mut Option<Stamp>, setup: &mut Option<Setup>) -> io::Result<bool> {
    let active = store.active()?;
    let id = active.map(|a| a.id);
    let next = Stamp {
        active: modified(&store.paths.active),
        setup: id.as_deref().and_then(|id| store.paths.json(id).ok()).and_then(|p| modified(&p)),
        id: id.clone(),
    };
    if stamp.as_ref() == Some(&next) {
        return Ok(false);
    }
    let loaded = id.map(|id| store.load(&id)).transpose()?;
    *setup = loaded.filter(|s| s.keep_connected || s.exclusive);
    *stamp = Some(next);
    Ok(true)
}

pub fn run() -> io::Result<()> {
    let path = Handover::path()?;
    let store = Store { paths: Paths::from_environment()? };
    STOP.store(false, Ordering::Relaxed);
    unsafe {
        libc::signal(libc::SIGTERM, signal_stop as libc::sighandler_t);
        libc::signal(libc::SIGINT, signal_stop as libc::sighandler_t);
    }
    let mut backend = PipeWireBackend::channel().map_err(io::Error::other)?;
    let mut lock = None;
    let mut graph = Graph::default();
    let mut stamp = None;
    let mut setup = None;
    let mut state = reconcile::State::default();
    let mut requests = HashMap::new();
    let mut request = 0;
    while !STOP.load(Ordering::Relaxed) {
        if lock.is_none() {
            lock = Handover::try_acquire(&path)?;
        }
        if lock.is_some() && requests.is_empty() {
            match reload(&store, &mut stamp, &mut setup) {
                Ok(true) => {
                    state = reconcile::State::default();
                }
                Err(error) => {
                    eprintln!("Musubi keeper: setup reload: {error}");
                    setup = None;
                    stamp = None;
                }
                _ => {}
            }
        }
        let timeout = reconcile::next_retry(&state, Instant::now())
            .unwrap_or(Duration::from_millis(250))
            .min(Duration::from_millis(250));
        match backend.recv_timeout(timeout) {
            Ok(Event::Delta(delta)) => {
                graph.apply(delta);
            }
            Ok(Event::Connection(Connection::Down(reason))) => {
                eprintln!("Musubi keeper: PipeWire disconnected: {reason}");
                graph = Graph::default();
                state = reconcile::State::default();
                requests.clear();
            }
            Ok(Event::CommandResult { request, result: Err(reason) }) => {
                if let Some(action) = requests.remove(&request) {
                    match action {
                        Action::CreateLink(out, input) => {
                            if let Some(route) = setup.as_ref().and_then(|s: &Setup| {
                                s.routes.iter().find(|r| {
                                    graph.resolve(&r.output) == Resolution::Unique(out)
                                        && graph.resolve(&r.input) == Resolution::Unique(input)
                                })
                            }) {
                                reconcile::create_failed(&mut state, route, Instant::now());
                            }
                        }
                        Action::RemoveLink(id) => reconcile::remove_failed(&mut state, id),
                    }
                    eprintln!("Musubi keeper: enforcement failed: {reason}");
                }
            }
            Ok(Event::CommandResult { request, .. }) => {
                requests.remove(&request);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("PipeWire backend exited"));
            }
            _ => {}
        }
        if lock.is_none() {
            continue;
        }
        if let Some(setup) = &setup {
            let outcome = reconcile::step(&graph, setup, std::mem::take(&mut state), Instant::now());
            state = outcome.state;
            for notice in outcome.notices {
                eprintln!("Musubi keeper: {notice}");
            }
            for action in outcome.actions {
                request += 1;
                let epoch = graph.epoch;
                backend.command(match action {
                    Action::CreateLink(out, input) => Command::Connect {
                        out: Handle { epoch, id: out },
                        input: Handle { epoch, id: input },
                        confirmed: false,
                        request,
                    },
                    Action::RemoveLink(id) => Command::Disconnect { link: Handle { epoch, id }, request },
                });
                requests.insert(request, action);
            }
        }
        // Yield only after actions settle; a waiting GUI can then take over.
        if lock.is_some() && requests.is_empty() {
            lock = None;
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    backend.shutdown();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "musubi-keeper-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn lock_handover() {
        let dir = Temp::new();
        let path = dir.path().join("setups.lock");
        let first = Handover::try_acquire(&path).unwrap().unwrap();
        assert!(Handover::try_acquire(&path).unwrap().is_none());
        drop(first);
        assert!(Handover::try_acquire(&path).unwrap().is_some());
    }
    #[test]
    fn reload_on_setup_change() {
        let dir = Temp::new();
        let store = Store { paths: Paths::under(dir.path().to_path_buf()) };
        let mut stamp = None;
        let mut setup = None;
        assert!(reload(&store, &mut stamp, &mut setup).unwrap());
        assert!(!reload(&store, &mut stamp, &mut setup).unwrap());
        let item = Setup {
            version: 1,
            id: "one".into(),
            name: "one".into(),
            routes: vec![],
            keep_connected: true,
            exclusive: false,
        };
        store.save(&item).unwrap();
        store.set_active(&super::super::Active { id: item.id.clone() }).unwrap();
        assert!(reload(&store, &mut stamp, &mut setup).unwrap());
        assert_eq!(setup.as_ref().unwrap().name, "one");
        let mut changed = item;
        changed.name = "two".into();
        store.save(&changed).unwrap();
        assert!(reload(&store, &mut stamp, &mut setup).unwrap());
        assert_eq!(setup.as_ref().unwrap().name, "two");
        store.clear_active().unwrap();
        assert!(reload(&store, &mut stamp, &mut None).unwrap());
    }
}
