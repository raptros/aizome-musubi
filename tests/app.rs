use aizome_kit::{
    Appearance,
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_musubi::{
    app::{Msg, Musubi, Page},
    backend::{Backend, Connection, Event, fake::FakeBackend},
};

#[test]
fn pages_switch_by_sidebar_and_view_shortcut() {
    let mut h = Harness::new(Musubi::default());
    let w = h.window();
    for (title, page) in
        [("Audio", Page::Audio), ("MIDI", Page::Midi), ("MIDI 2.0", Page::Midi2), ("Video", Page::Video)]
    {
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
    while let Some(event) = fake.next_event() {
        h.send(Msg::Backend(event));
    }
    assert!(h.find(w, "ports ·").is_some() || h.find(w, "PipeWire: connected").is_some());
    h.send(Msg::Backend(Event::Connection(Connection::Down("private daemon stopped".into()))));
    assert!(h.find(w, "PipeWire unavailable.").is_some());
    h.send(Msg::Backend(Event::Connection(Connection::Up("restarted".into()))));
    assert!(h.find(w, "Audio 0 ports · 0 links").is_some());
}

#[test]
fn graph_pages_have_night_and_day_goldens() {
    for (theme, mode) in [(ThemeChoice::Night, "night"), (ThemeChoice::Day, "day")] {
        for (name, json, page, pairs) in [
            ("audio-studio", include_str!("fixtures/studio.json"), Page::Audio, true),
            ("audio-studio-channels", include_str!("fixtures/studio.json"), Page::Audio, false),
            ("midi", include_str!("fixtures/midi.json"), Page::Midi, false),
            ("video", include_str!("fixtures/video.json"), Page::Video, false),
            ("empty-page", "[]", Page::Video, false),
        ] {
            let mut app = Musubi::default();
            app.options[0].pairs = pairs;
            let mut h = Harness::with(app, Appearance { theme, reduced_motion: true, ..Appearance::default() }, true);
            let w = h.window();
            let mut backend = FakeBackend::from_dump(json, 1).unwrap();
            while let Some(event) = backend.next_event() {
                h.send(Msg::Backend(event));
            }
            h.send(Msg::Page(page));
            if name == "empty-page" {
                assert!(
                    h.find(w, "No video ports. Cameras appear here when PipeWire's camera support finds them.")
                        .is_some()
                );
            } else {
                assert!(h.find_id(w, "matrix").is_some(), "missing matrix on {name}");
            }
            assert_golden(&mut h, w, format!("tests/snapshots/{name}-{mode}.png"), 1.);
        }
    }
}

#[test]
fn empty_patchbay_in_night_and_day() {
    for (theme, name) in [(ThemeChoice::Night, "night"), (ThemeChoice::Day, "day")] {
        let mut h =
            Harness::with(Musubi::default(), Appearance { theme, reduced_motion: true, ..Appearance::default() }, true);
        assert!(h.theme().has_ui_font(), "Run with AIZOME_FONT_DIR set to Sekka's fonts");
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/empty-{name}.png"), 1.);
    }
}
