use aizome_kit::{Appearance, testing::{Harness, assert_golden}, theme::ThemeChoice};
use aizome_musubi::{app::{Musubi, Msg, Page}, backend::{Backend, Connection, Event, fake::FakeBackend}};

#[test]
fn pages_switch_by_sidebar_and_view_shortcut() {
    let mut h = Harness::new(Musubi::default());
    let w = h.window();
    for (title, page) in [
        ("Audio", Page::Audio),
        ("MIDI", Page::Midi),
        ("MIDI 2.0", Page::Midi2),
        ("Video", Page::Video),
    ] {
        assert!(h.find_id(w, title).is_some(), "missing {title} sidebar item");
        assert!(h.click_text(w, title), "cannot click {title}");
        assert_eq!(h.program().page, page);
    }
    h.command(w, '2');
    assert_eq!(h.program().page, Page::Midi);
}

#[test]
fn fake_backend_updates_counts_and_unavailable_state() {
    let mut h = Harness::new(Musubi::default());
    let w = h.window();
    let mut fake = FakeBackend::from_dump(include_str!("fixtures/private.json"), 3).unwrap();
    while let Some(event) = fake.next_event() { h.send(Msg::Backend(event)); }
    assert!(h.find(w, "ports ·").is_some() || h.find(w, "PipeWire: connected").is_some());
    h.send(Msg::Backend(Event::Connection(Connection::Down("private daemon stopped".into()))));
    assert!(h.find(w, "PipeWire unavailable.").is_some());
    h.send(Msg::Backend(Event::Connection(Connection::Up("restarted".into()))));
    assert!(h.find(w, "0 ports · 0 links").is_some());
}

#[test]
fn empty_patchbay_in_night_and_day() {
    for (theme, name) in [(ThemeChoice::Night, "night"), (ThemeChoice::Day, "day")] {
        let mut h = Harness::with(
            Musubi::default(),
            Appearance { theme, reduced_motion: true, ..Appearance::default() },
            true,
        );
        assert!(h.theme().has_ui_font(), "Run with AIZOME_FONT_DIR set to Sekka's fonts");
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/empty-{name}.png"), 1.);
    }
}
