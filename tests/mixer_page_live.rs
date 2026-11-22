//! Harness drives the real app against the private PipeWire/Pulse service.
//! The private server has null sinks but no cards/profiles; real profile switching is a P2-S6 VM gate.
use aizome_kit::{
    Appearance, Point,
    sidestep_ui::{Key, Modifiers, NamedKey},
    testing::Harness,
};
use aizome_musubi::app::{Msg, Musubi, Page};
use aizome_sys::{Wake, audio::Audio};
use std::time::{Duration, Instant};

#[test]
fn keyboard_fader_mute_and_undo_read_back() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    let root = std::env::var("MUSUBI_PRIVATE_PIPEWIRE_ROOT").expect("private instance required");
    let server = format!("unix:{root}/runtime/pulse/native");
    assert_eq!(std::env::var("PULSE_SERVER").unwrap(), server);
    assert_eq!(std::env::var("PIPEWIRE_RUNTIME_DIR").unwrap(), format!("{root}/runtime"));
    let observer = Audio::with_server(Some(server), Wake::none());
    let mut app = Musubi::default();
    app.live = true;
    let mut h = Harness::with(app, Appearance::default(), true);
    let w = h.window();
    assert!(h.settle(Duration::from_secs(12), |app| {
        app.graph.nodes.values().any(|n| n.props.get("node.name").is_some_and(|v| v == "musubi_test_speakers"))
    }));
    let deadline = Instant::now() + Duration::from_secs(12);
    while observer.snapshot().sinks.iter().all(|s| s.name != "musubi_test_speakers") {
        assert!(Instant::now() < deadline, "Pulse sink not discovered");
        std::thread::sleep(Duration::from_millis(20));
    }
    h.send(Msg::Page(Page::Mixer));
    let sink = observer.snapshot().sinks.iter().find(|s| s.name == "musubi_test_speakers").unwrap().clone();
    let id = format!("fader-Output-{}", sink.index);
    let bounds = h.find_id(w, &id).expect("live Pulse sink strip");
    h.click(w, Point::new(bounds.x0 + 45., bounds.y1 - 16.)); // focus the value box without changing gain
    h.named(w, NamedKey::ArrowDown, Modifiers::NONE);
    let deadline = Instant::now() + Duration::from_secs(12);
    let lowered = loop {
        h.settle(Duration::from_millis(20), |_| false);
        let current = observer.snapshot().sinks.iter().find(|s| s.index == sink.index).unwrap().volume;
        if current < sink.volume - 0.001 {
            break current;
        }
        assert!(Instant::now() < deadline, "keyboard fader did not lower Pulse volume");
    };
    let display = h
        .semantics(w)
        .into_iter()
        .find(|(s, r)| s.label.as_deref() == Some("Volume") && *r == bounds)
        .and_then(|(s, _)| s.value)
        .expect("fader value box semantics");
    assert!(display.contains("dB"));
    println!("Pulse lowered: {} -> {}; strip value: {display}", sink.volume, lowered);
    h.key(w, Key::Character("m".into()), Modifiers::NONE);
    let deadline = Instant::now() + Duration::from_secs(12);
    while !observer.snapshot().sinks.iter().find(|s| s.index == sink.index).unwrap().muted {
        h.settle(Duration::from_millis(20), |_| false);
        assert!(Instant::now() < deadline, "M did not mute Pulse sink");
    }
    h.advance(Duration::from_millis(550));
    h.send(Msg::Undo); // mute
    h.send(Msg::Undo); // volume
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        h.settle(Duration::from_millis(20), |_| false);
        let current = observer.snapshot().sinks.iter().find(|s| s.index == sink.index).unwrap().clone();
        if (current.volume - sink.volume).abs() < 0.001 && !current.muted {
            break;
        }
        assert!(Instant::now() < deadline, "undo did not restore Pulse volume and mute: {current:?}");
    }
}
