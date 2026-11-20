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
    graph::{GraphDelta, LinkRecord, LinkState, Properties, Upsert},
    matrix::layout::{Cell, CellState},
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
            rect.x0 + layout.row_header.min((rect.width() - 240.).max(0.)) + layout.cols.offsets[cell.col] + 12.,
            rect.y0 + layout.column_header.min((rect.height() - 240.).max(0.)) + layout.rows.offsets[cell.row] + 12.,
        ),
    )
}

#[test]
fn matrix_semantics_follow_focus_and_return_opens_row_inspector() {
    use aizome_kit::widget::{SemanticAction, SemanticRole};

    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let table = |h: &Harness<Musubi>| {
        h.semantics(w)
            .into_iter()
            .find(|(s, _)| s.role == SemanticRole::Table && s.label.as_deref() == Some("Audio connections"))
            .expect("matrix table")
            .0
    };
    assert_eq!(table(&h).value, None);
    let (_, point) = cell(&h, 100, 120);
    h.click(w, point);
    h.key(w, Key::Named(NamedKey::ArrowRight), Modifiers::NONE);
    let focused = table(&h);
    assert!(focused.value.as_deref().is_some_and(|v| v.contains(" → ") && v.contains(", not connected")));
    assert!(focused.actions.contains(&SemanticAction::Click));
    h.key(w, Key::Named(NamedKey::ArrowLeft), Modifiers::NONE);
    fake.borrow_mut().fail_next("permission denied");
    h.click(w, point);
    assert!(table(&h).value.as_deref().is_some_and(|v| v.ends_with("error: permission denied")));
    h.key(w, Key::Named(NamedKey::Enter), Modifiers::NONE);
    assert!(h.find_id(w, "inspector").is_some());
    let label = aizome_musubi::graph::group::port_label(&h.program().graph.ports[&100]);
    assert!(h.find(w, &label).is_some(), "Return should inspect the row port");
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
fn observed_link_states_gate_connect_results_and_show_failure_reason() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let (c, p) = cell(&h, 100, 104);
    fake.borrow_mut().defer_next_connect();
    h.click(w, p);
    let id = fake.borrow().graph.links_by_pair[&(100, 104)][0];
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Asked);
    assert_eq!(h.program().transactions[0].intents[0].result, None);
    fake.borrow_mut().transition_link(id, LinkState::Error("format negotiation failed".into()));
    while let Some(event) = fake.borrow_mut().next_event() {
        h.send(Msg::Backend(event));
    }
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Error);
    assert_eq!(h.program().transactions[0].intents[0].result, Some(Err("format negotiation failed".into())));
    h.move_to(w, p);
    assert!(h.program().status_sentence().unwrap().contains("format negotiation failed"));

    fake.borrow_mut().remove(id);
    while let Some(event) = fake.borrow_mut().next_event() {
        h.send(Msg::Backend(event));
    }
    fake.borrow_mut().defer_next_connect();
    h.click(w, p);
    let id = fake.borrow().graph.links_by_pair[&(100, 104)][0];
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Asked);
    fake.borrow_mut().transition_link(id, LinkState::Active);
    while let Some(event) = fake.borrow_mut().next_event() {
        h.send(Msg::Backend(event));
    }
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(c), CellState::Connected);
    assert_eq!(h.program().transactions.last().unwrap().intents[0].result, Some(Ok(())));
}

#[test]
fn focus_and_press_remain_valid_after_filter_shrinks_matrix() {
    let (mut h, _) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let (_, first) = cell(&h, 100, 104);
    h.click(w, first);
    for _ in 0..40 {
        h.key(w, Key::Named(NamedKey::ArrowDown), Modifiers::NONE);
    }
    h.send(Msg::From("USB".into()));
    h.key(w, Key::Named(NamedKey::ArrowDown), Modifiers::NONE);
    h.key(w, Key::Character(" ".into()), Modifiers::NONE);
    let focused = h
        .semantics(w)
        .into_iter()
        .find(|(s, _)| {
            s.role == aizome_kit::widget::SemanticRole::Table && s.label.as_deref() == Some("Audio connections")
        })
        .expect("matrix semantics")
        .0;
    assert!(focused.value.is_some());
}

#[test]
fn undo_keeps_pending_connect_until_observation_settles() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    fake.borrow_mut().defer_next_connect();
    let (_, point) = cell(&h, 100, 104);
    h.click(h.window(), point);
    let id = fake.borrow().graph.links_by_pair[&(100, 104)][0];
    h.send(Msg::Undo);
    assert_eq!(fake.borrow().commands.len(), 1);
    assert!(h.program().status_sentence().unwrap().contains("waiting"));
    fake.borrow_mut().transition_link(id, LinkState::Active);
    while let Some(event) = fake.borrow_mut().next_event() {
        h.send(Msg::Backend(event));
    }
    h.send(Msg::Undo);
    assert!(matches!(fake.borrow().commands.last(), Some(Command::Disconnect { .. })));
    assert!(!h.program().graph.links_by_pair.contains_key(&(100, 104)));
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
fn undo_redo_pair_and_missing_port() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    h.send(Msg::Pairs(true));
    let w = h.window();
    let (_, point) = cell(&h, 130, 124);
    h.click(w, point);
    let count = fake.borrow().commands.len();
    assert_eq!(count, 2);
    h.command(w, 'z');
    assert_eq!(fake.borrow().commands.len(), count + 2);
    assert!(!h.program().graph.links_by_pair.contains_key(&(130, 124)));
    h.send(Msg::Redo);
    assert!(h.program().graph.links_by_pair.contains_key(&(130, 124)));
    fake.borrow_mut().remove(125);
    let event = fake.borrow_mut().next_event().unwrap();
    h.send(Msg::Backend(event));
    let count = fake.borrow().commands.len();
    h.send(Msg::Undo);
    assert_eq!(fake.borrow().commands.len(), count + 1);
    assert!(h.program().status_sentence().unwrap().contains("missing port"));
}

#[test]
fn inspector_unlink_is_undoable() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let (_, point) = cell(&h, 100, 120);
    h.click(w, point);
    h.send(Msg::Inspector);
    assert!(h.find_id(w, "inspector").is_some());
    assert!(h.find_id(w, "unlink:100:120").is_some());
    assert!(h.click_text(w, "Unlink"));
    assert!(matches!(fake.borrow().commands.last(), Some(Command::Disconnect { .. })));
    h.send(Msg::Undo);
    assert!(matches!(fake.borrow().commands.last(), Some(Command::Connect { .. })));
}

#[test]
fn toolbar_options_change_live_layout_and_hidden_count() {
    let (mut h, _) = interactive(include_str!("fixtures/studio.json"));
    let base = h.program().layout(Page::Audio).unwrap().rows.labels.len();
    h.send(Msg::Pairs(true));
    let pairs = h.program().layout(Page::Audio).unwrap().rows.labels.len();
    assert!(pairs < base);
    h.send(Msg::Swap);
    assert!(h.program().layout(Page::Audio).unwrap().swapped);
    h.send(Msg::From("USB".into()));
    let layout = h.program().layout(Page::Audio).unwrap();
    assert!(layout.hidden_from > 0);
    assert!(layout.cols.labels.len() < base);
    assert!(h.find(h.window(), &format!("From “USB” hides {} ports", layout.hidden_from)).is_some());
    let mut monitor = h.program().graph.ports[&100].clone();
    monitor.id = 99999;
    monitor.props.insert("port.monitor".into(), "true".into());
    h.send(Msg::Backend(Event::Delta(GraphDelta { epoch: 1, upserts: vec![Upsert::Port(monitor)], removals: vec![] })));
    let before = h.program().layout(Page::Audio).unwrap().cols.labels.len();
    h.send(Msg::Monitors(true));
    assert!(h.program().options[0].monitors);
    assert_eq!(h.program().layout(Page::Audio).unwrap().cols.labels.len(), before + 1);
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
        upserts: vec![Upsert::Link(LinkRecord {
            id: 9999,
            output: 100,
            input: 104,
            state: LinkState::Active,
            props: Properties::new(),
        })],
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
fn stream_subtitle_bands_and_header_controls() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let layout = h.program().layout(Page::Audio).unwrap();
    let firefox = layout.rows.groups.iter().find(|g| g.name == "Firefox").unwrap();
    let pw_cat = layout.rows.groups.iter().find(|g| g.name == "pw-cat").unwrap();
    assert_eq!(layout.rows.subtitles.iter().filter(|(at, _)| *at >= firefox.start && *at < firefox.finish).count(), 2);
    assert_eq!(layout.rows.subtitles.iter().filter(|(at, _)| *at >= pw_cat.start && *at < pw_cat.finish).count(), 2);
    let rect = h.find_id(w, "matrix").unwrap();
    let band = layout.rows.subtitles.iter().find(|(_, text)| text == "Firefox Music Player").unwrap().0;
    let blank = Point::new(rect.x0 + layout.row_header + 12., rect.y0 + layout.column_header + band + 8.);
    let edge = Point::new(rect.x0 + layout.row_header, rect.y0 + layout.column_header + 60.);
    let old = layout.row_header;
    let commands = fake.borrow().commands.len();
    h.click(w, blank);
    assert_eq!(fake.borrow().commands.len(), commands, "subtitle band must not hit a cell");
    let (_, stream_cell) = cell(&h, 330, 104);
    h.click(w, stream_cell);
    assert!(fake.borrow().commands.len() > commands, "line after subtitle must hit its own cell");

    h.drag(w, edge, Point::new(edge.x + 40., edge.y), 3);
    assert!(h.program().layout(Page::Audio).unwrap().row_header > old);
    let edge = Point::new(rect.x0 + h.program().layout(Page::Audio).unwrap().row_header, edge.y);
    h.press(w, edge, PointerButton::Left, 2);
    h.release(w, edge, PointerButton::Left);
    let fitted = h.program().layout(Page::Audio).unwrap().fitted_headers().0;
    assert_eq!(h.program().layout(Page::Audio).unwrap().row_header, fitted);
    h.send(Msg::FullNames);
    assert!(h.program().options[0].full_names);
    h.send(Msg::Page(Page::Midi));
    assert!(!h.program().options[1].full_names);
    h.send(Msg::Page(Page::Audio));
    assert!(h.program().options[0].full_names);
    h.send(Msg::FullNames);
    assert!(!h.program().options[0].full_names);
}

#[test]
fn tooltip_delay_and_leave_clear_popup_and_page_clears_notice() {
    let (mut h, _) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let layout = h.program().layout(Page::Audio).unwrap();
    let rect = h.find_id(w, "matrix").unwrap();
    let firefox = layout.rows.groups.iter().find(|g| g.name == "Firefox").unwrap();
    let point = Point::new(rect.x0 + 35., rect.y0 + layout.column_header + firefox.start + 9.);
    h.move_to(w, point);
    h.advance(std::time::Duration::from_millis(649));
    let before = h.render_rgba(w, 1.).2;
    h.advance(std::time::Duration::from_millis(1));
    let shown = h.render_rgba(w, 1.).2;
    assert!(aizome_kit::testing::difference(&before, &shown, 0) > 0.);
    h.move_to(w, Point::new(rect.x0 + 10., rect.y0 + 10.));
    assert_eq!(before, h.render_rgba(w, 1.).2);
    h.send(Msg::Undo);
    h.send(Msg::Page(Page::Midi));
    assert!(!h.program().status_sentence().is_some_and(|s| s.contains("Undo")));
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
    assert!(h.program().layout(Page::Audio).is_some());
    h.send(Msg::Backend(Event::Connection(Connection::Down("private daemon stopped".into()))));
    assert!(h.find(w, "PipeWire isn't answering. Musubi keeps trying.").is_some());
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
fn stream_full_names_and_tooltip_goldens() {
    for (theme, mode) in [(ThemeChoice::Night, "night"), (ThemeChoice::Day, "day")] {
        let appearance = Appearance { theme, reduced_motion: true, ..Appearance::default() };
        let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
        let mut app = Musubi::with_fake(fake);
        app.options[0].pairs = true;
        let mut h = Harness::with(app, appearance, true);
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/audio-streams-{mode}.png"), 1.);
        h.send(Msg::FullNames);
        assert_golden(&mut h, w, format!("tests/snapshots/audio-full-names-{mode}.png"), 1.);
        h.send(Msg::FullNames);
        let layout = h.program().layout(Page::Audio).unwrap();
        let rect = h.find_id(w, "matrix").unwrap();
        let firefox = layout.rows.groups.iter().find(|g| g.name == "Firefox").unwrap();
        h.move_to(w, Point::new(rect.x0 + 35., rect.y0 + layout.column_header + firefox.start + 9.));
        h.advance(std::time::Duration::from_millis(650));
        assert_golden(&mut h, w, format!("tests/snapshots/audio-tooltip-{mode}.png"), 1.);
        h.move_to(w, Point::new(rect.x0 - 10., rect.y0 - 10.));
    }
}

#[test]
fn toolbar_inspector_goldens() {
    for (theme, mode) in [(ThemeChoice::Night, "night"), (ThemeChoice::Day, "day")] {
        for variant in ["swapped", "filtered", "inspector"] {
            let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
            let mut app = Musubi::with_fake(fake);
            app.options[0].pairs = true;
            let mut h = Harness::with(app, Appearance { theme, reduced_motion: true, ..Appearance::default() }, true);
            match variant {
                "swapped" => h.send(Msg::Swap),
                "filtered" => h.send(Msg::From("USB".into())),
                "inspector" => {
                    let (_, point) = cell(&h, 100, 120);
                    h.click(h.window(), point);
                    h.send(Msg::Inspector);
                }
                _ => unreachable!(),
            }
            let window = h.window();
            assert_golden(&mut h, window, format!("tests/snapshots/audio-studio-{variant}-{mode}.png"), 1.);
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

#[test]
fn list_selection_delete_undo_and_semantics() {
    let (mut h, _fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    let rows = h.program().graph.links.len();
    h.send(Msg::List(false));
    let (_, point) = cell(&h, 100, 120);
    h.click(w, point);
    h.send(Msg::List(true));
    assert_eq!(h.program().graph.links.len(), rows + 1);
    let semantics = h.semantics(w);
    assert!(semantics.iter().any(|(s, _)| s.role == aizome_kit::widget::SemanticRole::Table
        && s.value.as_deref().is_some_and(|v| v.contains("rows"))));
    assert!(h.find(w, "Connected").is_some());
    let layout = h.program().layout(Page::Audio).unwrap();
    let mut routes = aizome_musubi::routes::routes(&h.program().graph, layout, false, &[], &[]);
    aizome_musubi::routes::sort(&mut routes, 0, true);
    let index = routes.iter().position(|r| r.from == 100 && r.to == 120).unwrap();
    h.send(Msg::SelectRoutes(vec![index]));
    assert!(h.semantics(w).iter().any(|(s, _)| s.label.as_deref().is_some_and(|v| v.contains(" · "))));
    h.send(Msg::DeleteRoutes);
    assert!(!h.program().graph.links_by_pair.contains_key(&(100, 120)));
    h.send(Msg::Undo);
    assert!(h.program().graph.links_by_pair.contains_key(&(100, 120)));
}

#[test]
fn every_page_shows_only_its_toolbar_options() {
    let (mut h, _) = interactive(include_str!("fixtures/midi.json"));
    let w = h.window();
    for (page, audio, midi2) in [
        (Page::Audio, true, false),
        (Page::Midi, false, false),
        (Page::Midi2, false, true),
        (Page::Video, false, false),
    ] {
        h.send(Msg::Page(page));
        assert_eq!(h.find_id(w, "pairs-channels").is_some(), audio, "{page:?}");
        assert_eq!(h.find_id(w, "monitors").is_some(), audio, "{page:?}");
        assert_eq!(h.find_id(w, "show-midi-1").is_some(), midi2, "{page:?}");
    }
}

#[test]
fn midi2_toggle_and_page_specific_controls() {
    let (mut h, _) = interactive(include_str!("fixtures/midi.json"));
    let w = h.window();
    h.send(Msg::Page(Page::Midi2));
    assert!(h.find_id(w, "show-midi-1").is_some());
    assert!(h.find_id(w, "pairs-channels").is_none());
    assert!(h.find_id(w, "monitors").is_none());
    h.send(Msg::ShowMidi1(true));
    assert!(h.program().layout(Page::Midi2).unwrap().rows.groups.iter().any(|g| g.midi_1));
}

#[test]
fn keyboard_add_route_and_midi_confirmation() {
    let (mut h, fake) = interactive(include_str!("fixtures/studio.json"));
    let w = h.window();
    h.send(Msg::List(true));
    assert!(h.click_text(w, "Add Route…"));
    let sheet = *h.windows().last().unwrap();
    h.type_text(sheet, "AUX0");
    h.named(sheet, NamedKey::Tab, Modifiers::NONE);
    h.named(sheet, NamedKey::ArrowDown, Modifiers::NONE);
    h.named(sheet, NamedKey::Tab, Modifiers::NONE);
    h.type_text(sheet, "AUX0");
    h.named(sheet, NamedKey::Tab, Modifiers::NONE);
    h.named(sheet, NamedKey::ArrowDown, Modifiers::NONE);
    h.named(sheet, NamedKey::Tab, Modifiers::NONE);
    h.named(sheet, NamedKey::Tab, Modifiers::NONE);
    h.named(sheet, NamedKey::Enter, Modifiers::NONE);
    assert!(
        fake.borrow().commands.iter().any(|c| matches!(c, Command::Connect { .. })),
        "keyboard route did not connect"
    );
    assert!(h.semantics(w).iter().any(|(s, _)| s.label.as_deref() == Some("Connected")));
}

#[test]
fn persistence_options_and_fold_relaunch() {
    let temp = std::env::temp_dir().join(format!("musubi-s4d-{}", std::process::id()));
    std::fs::create_dir_all(&temp).unwrap();
    unsafe {
        std::env::set_var("MUSUBI_TEST_CONFIG", "1");
        std::env::set_var("XDG_CONFIG_HOME", temp.join("config"));
        std::env::set_var("XDG_STATE_HOME", temp.join("state"));
    }
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
    let mut h = Harness::new(Musubi::with_fake(fake));
    h.send(Msg::Pairs(true));
    h.send(Msg::Swap);
    h.send(Msg::List(true));
    h.send(Msg::Matrix(aizome_musubi::matrix::widget::MatrixMsg::HeaderSize(true, 232.)));
    h.send(Msg::Matrix(aizome_musubi::matrix::widget::MatrixMsg::HeaderSize(false, 204.)));
    h.send(Msg::FullNames);
    let key = h.program().layout(Page::Audio).unwrap().rows.groups[0].key.clone().unwrap();
    h.send(Msg::ContextAction(aizome_musubi::app::ContextAction::Fold(key)));
    h.advance(std::time::Duration::from_millis(400));
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 2).unwrap()));
    let h2 = Harness::new(Musubi::with_fake(fake));
    assert!(h2.program().options[0].pairs && h2.program().options[0].swap && h2.program().options[0].list);
    assert_eq!(h2.program().options[0].row_header, 232.);
    assert_eq!(h2.program().options[0].column_header, 204.);
    assert!(h2.program().options[0].full_names);
    assert!(!h2.program().options[0].folded.is_empty());
    drop((h, h2));
    unsafe {
        std::env::remove_var("MUSUBI_TEST_CONFIG");
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("XDG_STATE_HOME");
    }
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn s4d_night_day_goldens() {
    for (theme, mode) in [(ThemeChoice::Night, "night"), (ThemeChoice::Day, "day")] {
        let appearance = Appearance { theme, reduced_motion: true, ..Appearance::default() };
        let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
        let mut h = Harness::with(Musubi::with_fake(fake), appearance, true);
        h.send(Msg::List(true));
        h.send(Msg::From("USB".into()));
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/list-filtered-{mode}.png"), 1.);
        h.send(Msg::AddRoute);
        let sheet = *h.windows().last().unwrap();
        assert_golden(&mut h, sheet, format!("tests/snapshots/add-route-sheet-{mode}.png"), 1.);
        h.send(Msg::CloseRoute);
        let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/midi.json"), 1).unwrap()));
        let mut h = Harness::with(Musubi::with_fake(fake), appearance, true);
        h.send(Msg::Page(Page::Midi2));
        h.send(Msg::ShowMidi1(true));
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/midi2-show-midi1-{mode}.png"), 1.);
        let (_, p) = cell(&h, 20, 41);
        h.click(h.window(), p);
        assert_golden(&mut h, w, format!("tests/snapshots/midi2-confirm-{mode}.png"), 1.);
        let mut h = Harness::with(Musubi::default(), appearance, true);
        h.send(Msg::Backend(Event::Connection(Connection::Down("private daemon stopped".into()))));
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/unavailable-{mode}.png"), 1.);
    }
}
