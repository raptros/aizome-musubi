use aizome_kit::{
    Appearance,
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_kit::{
    Point,
    sidestep_ui::{Key, Modifiers, NamedKey, PointerButton},
};
use aizome_musubi::{
    app::{Msg, Musubi, Page},
    backend::{Backend, Connection, Event, fake::FakeBackend},
};
use aizome_musubi::{
    backend::Command,
    graph::{GraphDelta, LinkRecord, Properties, Upsert},
    matrix::layout::{COLUMN_HEADER, Cell, CellState, ROW_HEADER},
};
use std::{cell::RefCell, rc::Rc};

fn interactive(json: &str) -> (Harness<Musubi>, Rc<RefCell<FakeBackend>>) {
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(json, 1).unwrap()));
    let h = Harness::new(Musubi::with_fake(fake.clone()));
    (h, fake)
}

fn cell(h: &Harness<Musubi>, from: u32, to: u32) -> (Cell, Point) {
    let layout = h.program().layout(h.program().page).unwrap();
    let cell = layout.cell_for_pair((from, to)).unwrap();
    let rect = h.find_id(h.window(), "matrix").unwrap();
    (
        cell,
        Point::new(
            rect.x0 + ROW_HEADER + layout.cols.offsets[cell.col] + 12.,
            rect.y0 + COLUMN_HEADER + layout.rows.offsets[cell.row] + 12.,
        ),
    )
}

#[test]
fn click_connect_disconnect_and_failure() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let (c, p) = cell(&h, 100, 120);
    h.click(w, p);
    assert!(
        matches!(fake.borrow().commands[0], Command::Connect { out, input, .. } if out.id == 100 && input.id == 120)
    );
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Connected);
    assert!(h.program().transactions[0].intents[0].observed);
    h.click(w, p);
    assert!(matches!(fake.borrow().commands[1], Command::Disconnect { .. }));
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Empty);
    fake.borrow_mut().fail_next("permission denied");
    h.click(w, p);
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Error);
    h.move_to(w, p);
    assert!(h.program().status_sentence().unwrap().contains("permission denied"));
}

#[test]
fn pairs_complete_partials_and_pair_to_mono() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    h.program_mut().options[0].pairs = true;
    h.send(Msg::Backend(Event::Delta(GraphDelta { epoch: 1, ..Default::default() })));
    let w = h.window();
    let (c, p) = cell(&h, 130, 124);
    h.click(w, p);
    let commands = fake.borrow().commands.clone();
    let ops = &commands;
    assert!(ops.iter().any(|op| matches!(op, Command::Connect { out, input, .. } if out.id == 130 && input.id == 124)));
    assert!(ops.iter().any(|op| matches!(op, Command::Connect { out, input, .. } if out.id == 131 && input.id == 125)));
    assert_eq!(h.program().transactions.last().unwrap().intents.len(), 2);
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Connected);
    let link = h.program().graph.links_by_pair[&(131, 125)][0];
    h.send(Msg::Backend(Event::Delta(GraphDelta { epoch: 1, removals: vec![link], ..Default::default() })));
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Half);
    let before = fake.borrow().commands.len();
    h.click(w, p);
    assert_eq!(fake.borrow().commands.len(), before + 1);
    assert!(
        matches!(fake.borrow().commands.last().unwrap(), Command::Connect { out, input, .. } if out.id == 131 && input.id == 125)
    );
    let (_, mono) = cell(&h, 130, 310);
    h.click(w, mono);
    assert!(
        fake.borrow()
            .commands
            .iter()
            .any(|op| matches!(op, Command::Connect { out, input, .. } if out.id == 130 && input.id == 310))
    );
    assert!(
        fake.borrow()
            .commands
            .iter()
            .any(|op| matches!(op, Command::Connect { out, input, .. } if out.id == 131 && input.id == 310))
    );
}

#[test]
fn diagonal_drag_cancel_and_keyboard_line() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let (_, first) = cell(&h, 100, 104);
    let (_, second) = cell(&h, 101, 105);
    h.drag(w, first, second, 2);
    assert_eq!(h.program().transactions.len(), 1);
    assert!(
        fake.borrow()
            .commands
            .iter()
            .any(|op| matches!(op, Command::Connect { out, input, .. } if out.id == 100 && input.id == 104))
    );
    assert!(
        fake.borrow()
            .commands
            .iter()
            .any(|op| matches!(op, Command::Connect { out, input, .. } if out.id == 101 && input.id == 105))
    );
    let before = fake.borrow().commands.len();
    h.press(w, first, PointerButton::Left, 1);
    h.move_to(w, second);
    h.key(w, Key::Named(NamedKey::Escape), Modifiers::default());
    h.release(w, second, PointerButton::Left);
    assert_eq!(fake.borrow().commands.len(), before);
    h.click(w, first);
    h.key(w, Key::Character(" ".into()), Modifiers::SHIFT);
    h.key(w, Key::Named(NamedKey::ArrowRight), Modifiers::default());
    h.key(w, Key::Named(NamedKey::ArrowDown), Modifiers::default());
    h.key(w, Key::Character(" ".into()), Modifiers::default());
    assert_eq!(h.program().transactions.len(), 3);
}

#[test]
fn collapsed_cell_expands_and_relink_notice() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let row = h.program().layout(Page::Audio).unwrap().cell_for_pair((100, 104)).unwrap().row;
    let key = h.program().layout(Page::Audio).unwrap().rows.groups
        [h.program().layout(Page::Audio).unwrap().rows.group_of(row)]
    .key
    .clone()
    .unwrap();
    h.program_mut().options[0].folded.insert(key.clone());
    h.send(Msg::Backend(Event::Delta(GraphDelta { epoch: 1, ..Default::default() })));
    let (_, p) = cell(&h, 100, 104);
    h.click(w, p);
    assert!(!h.program().options[0].folded.contains(&key));
    assert!(fake.borrow().commands.is_empty());
    let (_, p) = cell(&h, 100, 104);
    h.click(w, p);
    h.click(w, p);
    h.send(Msg::Backend(Event::Delta(GraphDelta {
        epoch: 1,
        upserts: vec![Upsert::Link(LinkRecord { id: 9999, output: 100, input: 104, props: Properties::new() })],
        removals: vec![],
    })));
    assert!(h.program().status_sentence().unwrap().contains("WirePlumber connected"));
}

#[test]
fn midi_conversion_asks_once_and_cancel_does_not_send() {
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/midi.json"), 1).unwrap()));
    let mut app = Musubi::with_fake(fake.clone());
    app.options[2].show_midi_1 = true;
    let mut h = Harness::new(app);
    let w = h.window();
    h.send(Msg::Page(Page::Midi2));
    let (_, p) = cell(&h, 20, 41);
    h.click(w, p);
    assert!(fake.borrow().commands.is_empty());
    assert!(h.windows().len() > 1 || h.find(w, "MIDI conversion").is_some(), "conversion alert missing");
    assert!(h.click_text(w, "Cancel"));
    assert!(fake.borrow().commands.is_empty());
    h.click(w, p);
    assert!(h.click_text(w, "Connect"));
    assert!(matches!(fake.borrow().commands.last(), Some(Command::Connect { confirmed: true, .. })));
    assert_eq!(h.program().transactions.len(), 1);
}

#[test]
fn incompatible_video_cell_reports_reason_without_command() {
    let (mut h, fake) = interactive(include_str!("fixtures/video.json"));
    let w = h.window();
    h.send(Msg::Page(Page::Video));
    let mut port = h.program().graph.ports[&21].clone();
    port.props.insert("video.format.family".into(), "incompatible".into());
    h.send(Msg::Backend(Event::Delta(GraphDelta {
        epoch: 1,
        upserts: vec![Upsert::Port(port)],
        ..Default::default()
    })));
    let (c, p) = cell(&h, 20, 21);
    assert_eq!(h.program().layout(Page::Video).unwrap().state(c), CellState::NotAllowed);
    h.click(w, p);
    assert!(fake.borrow().commands.is_empty());
    assert!(h.program().status_sentence().unwrap().contains("video format families differ"));
}
#[test]
fn interaction_state_goldens() {
    let (mut h, _) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let (_, start) = cell(&h, 100, 104);
    let (_, end) = cell(&h, 101, 105);
    h.press(w, start, PointerButton::Left, 1);
    h.move_to(w, end);
    assert_golden(&mut h, w, "tests/snapshots/audio-drag-preview-night.png", 1.);
    h.key(w, Key::Named(NamedKey::Escape), Modifiers::default());
    h.release(w, end, PointerButton::Left);
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    fake.borrow_mut().fail_next("Permission denied by PipeWire");
    let (_, p) = cell(&h, 100, 104);
    h.click(w, p);
    h.move_to(w, p);
    assert_golden(&mut h, w, "tests/snapshots/audio-error-night.png", 1.);
}

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
