//! Private PipeWire only: `MUSUBI_LIVE=1 scripts/private-pipewire -- cargo test --test clock_live`.
use aizome_musubi::backend::{Backend, ClockKey, Command, Connection, Event, pipewire::PipeWireBackend};
use aizome_musubi::clock::Settings;
use std::{process::Command as ProcessCommand, time::{Duration, Instant}};

fn metadata() -> String {
    let output = ProcessCommand::new("pw-metadata").args(["-n", "settings", "0"]).output().expect("private pw-metadata");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}
fn until(label: &str, backend: &mut PipeWireBackend, settings: &mut Settings, mut condition: impl FnMut(&Settings, &str) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        while let Some(event) = backend.next_event() {
            match event {
                Event::ClockSettings(value) => *settings = value,
                Event::Connection(Connection::Down(reason)) => panic!("private daemon: {reason}"),
                Event::CommandResult { result: Err(reason), .. } => panic!("clock command: {reason}"),
                _ => {}
            }
        }
        let server = metadata();
        if condition(settings, &server) { return; }
        assert!(Instant::now() < deadline, "timed out waiting for {label}: settings {settings:?}; server {server}");
        std::thread::sleep(Duration::from_millis(40));
    }
}
#[test]
fn force_quantum_and_automatic_round_trip() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") { return; }
    let root = std::env::var("MUSUBI_PRIVATE_PIPEWIRE_ROOT").expect("private runtime required");
    assert_eq!(std::env::var("PIPEWIRE_RUNTIME_DIR").unwrap(), format!("{root}/runtime"));
    let mut backend = PipeWireBackend::channel().unwrap();
    let mut settings = Settings::default();
    until("settings metadata", &mut backend, &mut settings, |s, _| s.rate.is_some() && s.quantum.is_some());
    backend.command(Command::SetClock { key: ClockKey::Quantum, value: Some(256), request: 1 });
    until("forced quantum 256", &mut backend, &mut settings, |s, server| {
        s.force_quantum == Some(256) && server.lines().any(|line| line.contains("clock.force-quantum") && line.contains("256"))
    });
    backend.command(Command::SetClock { key: ClockKey::Quantum, value: None, request: 2 });
    until("Automatic clears force quantum", &mut backend, &mut settings, |s, server| {
        s.force_quantum.is_none() && server.lines().any(|line| line.contains("clock.force-quantum") && line.contains("0"))
    });
    println!("private settings metadata: quantum forced 256 then Automatic 0");
}
