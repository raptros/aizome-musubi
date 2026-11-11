//! Keep the lock throughout the app's lifetime; never remove another process's live socket.
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::{fs::{OpenOptionsExt, PermissionsExt}, net::{UnixListener, UnixStream}}},
    path::PathBuf,
    time::Duration,
};

pub struct Instance {
    pub listener: UnixListener,
    _lock: File,
    path: PathBuf,
}

pub enum Claim {
    First(Instance),
    Handed,
}

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn claim() -> Result<Claim, String> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR is missing. Launch Musubi inside your desktop session.")?;
    claim_in(dir)
}

pub fn claim_in(dir: PathBuf) -> Result<Claim, String> {
    let path = dir.join("aizome-app.musubi.sock");
    let lock = OpenOptions::new()
        .read(true).write(true).create(true).truncate(false).mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join("aizome-app.musubi.lock"))
        .map_err(|_| "Could not open Musubi's instance lock")?;
    // SAFETY: `lock` owns this live descriptor; flock retains no pointer.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        for _ in 0..50 {
            if let Ok(mut socket) = UnixStream::connect(&path) {
                socket.set_write_timeout(Some(Duration::from_secs(1))).ok();
                socket.write_all(b"activate\n")
                    .map_err(|_| "Could not activate the running Musubi")?;
                return Ok(Claim::Handed);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        return Err("Musubi is already running but could not be reached.".into());
    }
    if path.exists() {
        std::fs::remove_file(&path).map_err(|_| "Could not replace Musubi's stale socket")?;
    }
    let listener = UnixListener::bind(&path).map_err(|_| "Could not create Musubi's activation socket")?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| "Could not protect Musubi's socket")?;
    Ok(Claim::First(Instance { listener, _lock: lock, path }))
}

pub fn serve(listener: UnixListener, notify: impl Fn() -> bool + Send + 'static) {
    std::thread::spawn(move || {
        for socket in listener.incoming() {
            let Ok(socket) = socket else { break };
            socket.set_read_timeout(Some(Duration::from_millis(500))).ok();
            let mut buffer = Vec::new();
            if socket.take(64).read_to_end(&mut buffer).is_ok()
                && buffer == b"activate\n"
                && !notify()
            {
                break;
            }
        }
    });
}
