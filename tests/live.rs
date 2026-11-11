//! Run with MUSUBI_LIVE=1 inside scripts/private-pipewire; never use the host daemon.
use aizome_kit::{Point, testing::Harness};
use aizome_musubi::{
    app::{Musubi, Page},
    matrix::layout::{COLUMN_HEADER, CellState, ROW_HEADER},
};
use aizome_musubi::{
    backend::{Backend, Command, Connection, Event, Handle, pipewire::PipeWireBackend},
    graph::{Direction, Graph, classify::PortKind},
};
use std::{
    process::Command as ProcessCommand,
    time::{Duration, Instant},
};

#[test]
fn harness_clicks_private_interface_to_speakers() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    let mut app = Musubi::default();
    app.live = true;
    let mut h = Harness::new(app);
    let w = h.window();
    // The test sink's outputs are monitor ports; show them, one line per channel.
    // Set through the app's own messages: launch loads persisted view options.
    h.send(aizome_musubi::app::Msg::Monitors(true));
    h.send(aizome_musubi::app::Msg::Pairs(false));
    // Each null sink has an input and a monitor output per channel: match the direction too.
    let port = |app: &Musubi, node: &str, channel: &str, direction: Direction| {
        app.graph
            .ports
            .values()
            .find(|p| {
                p.direction == direction
                    && p.props.get("audio.channel").is_some_and(|v| v == channel)
                    && p.node_id
                        .and_then(|id| app.graph.nodes.get(&id))
                        .and_then(|n| n.props.get("node.name"))
                        .is_some_and(|name| name == node)
            })
            .map(|p| p.id)
    };
    // Wait until the displayed layout (not only the graph) holds the route.
    assert!(
        h.settle(Duration::from_secs(15), |app| {
            match (
                port(app, "musubi_test_interface_out", "AUX0", Direction::Output),
                port(app, "musubi_test_speakers", "FL", Direction::Input),
            ) {
                (Some(from), Some(to)) => {
                    app.layout(Page::Audio).is_some_and(|l| l.cell_for_pair((from, to)).is_some())
                }
                _ => false,
            }
        }),
        "private graph did not show the route: epoch {}, nodes {}, ports {}, layout {:?}; sources {:?}; targets {:?}",
        h.program().graph.epoch,
        h.program().graph.nodes.len(),
        h.program().graph.ports.len(),
        h.program().layout(Page::Audio).map(|l| (l.row_lines.len(), l.col_lines.len())),
        h.program()
            .graph
            .nodes
            .values()
            .filter(|n| n.props.get("node.name").is_some_and(|name| name.contains("interface_out")))
            .map(|n| (&n.id, &n.props))
            .collect::<Vec<_>>(),
        h.program()
            .graph
            .nodes
            .values()
            .filter(|n| n.props.get("node.name").is_some_and(|name| name.contains("speakers")))
            .map(|n| (&n.id, &n.props))
            .collect::<Vec<_>>(),
    );
    let from = port(h.program(), "musubi_test_interface_out", "AUX0", Direction::Output).unwrap();
    let to = port(h.program(), "musubi_test_speakers", "FL", Direction::Input).unwrap();
    let layout = h.program().layout(Page::Audio).unwrap();
    let cell = layout.cell_for_pair((from, to)).unwrap();
    assert_eq!(layout.state(cell), CellState::Empty);
    let rect = h.find_id(w, "matrix").unwrap();
    let point = Point::new(
        rect.x0 + ROW_HEADER + layout.cols.offsets[cell.col] + 12.,
        rect.y0 + COLUMN_HEADER + layout.rows.offsets[cell.row] + 12.,
    );
    h.click(w, point);
    assert!(
        h.settle(Duration::from_secs(8), |app| app.graph.links_by_pair.contains_key(&(from, to))),
        "click did not produce an observed PipeWire link"
    );
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(cell), CellState::Connected);
    // A second click removes it again (and leaves the shared private daemon as it was).
    h.click(w, point);
    assert!(
        h.settle(Duration::from_secs(8), |app| !app.graph.links_by_pair.contains_key(&(from, to))),
        "second click did not remove the PipeWire link"
    );
    assert_eq!(h.program().layout(Page::Audio).unwrap().state(cell), CellState::Empty);
}

#[test]
fn enumerates_private_audio_midi_video() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    let backend = PipeWireBackend::channel().unwrap();
    let mut graph = Graph::default();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut up = false;
    while Instant::now() < deadline {
        match backend.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Connection(Connection::Up(_))) => up = true,
            Ok(Event::Connection(Connection::Down(reason))) => panic!("private PipeWire down: {reason}"),
            Ok(Event::Delta(delta)) => graph.apply(delta),
            _ => {}
        }
        if up
            && graph
                .nodes
                .values()
                .filter(|n| n.props.get("node.name").is_some_and(|s| s.starts_with("musubi_test_")))
                .count()
                >= 5
            && graph.ports_of_kind(PortKind::Midi).count() >= 2
            && graph.ports_of_kind(PortKind::Video).count() >= 1
        {
            break;
        }
    }
    assert!(up, "initial PipeWire sync did not finish");
    for name in ["musubi_test_speakers", "musubi_test_interface_out", "musubi_test_interface_in", "musubi_test_mic"] {
        assert!(graph.nodes.values().any(|n| n.props.get("node.name").is_some_and(|v| v == name)), "missing {name}");
    }
    assert!(graph.ports_of_kind(PortKind::Audio).count() >= 16);
    assert_eq!(graph.ports_of_kind(PortKind::Midi).count(), 2);
    assert_eq!(graph.ports_of_kind(PortKind::Video).count(), 1);
    for id in graph.ports_of_kind(PortKind::Midi).chain(graph.ports_of_kind(PortKind::Video)) {
        assert!(graph.group_key(&graph.ports[&id]).is_some(), "ungrouped port {id}");
    }
}

#[test]
fn links_and_recovers_private_daemon() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    let mut backend = PipeWireBackend::channel().unwrap();
    let mut graph = Graph::default();
    let mut up = false;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        match backend.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Connection(Connection::Up(_))) => up = true,
            Ok(Event::Delta(delta)) => graph.apply(delta),
            _ => {}
        }
        // Without a session manager, stream nodes (pw-cat) never get their ports configured,
        // so the sources here are the 4-channel test sink's monitor outputs.
        if up
            && graph.nodes.values().any(|n| n.props.get("node.name").is_some_and(|n| n == "musubi_test_speakers"))
            && graph.nodes.values().any(|n| n.props.get("node.name").is_some_and(|n| n == "musubi_test_interface_out"))
            && graph.ports.len() >= 16
        {
            break;
        }
    }
    assert!(up, "daemon never came up");
    let epoch = graph.epoch;
    let speakers = graph
        .nodes
        .values()
        .find(|n| n.props.get("node.name").is_some_and(|n| n == "musubi_test_speakers"))
        .unwrap()
        .id;
    let source = graph
        .nodes
        .values()
        .find(|n| n.props.get("node.name").is_some_and(|n| n == "musubi_test_interface_out"))
        .unwrap()
        .id;
    let ports = |node, direction| {
        let mut ids: Vec<_> = graph
            .ports
            .values()
            .filter(|p| p.node_id == Some(node) && p.direction == direction)
            .map(|p| p.id)
            .collect();
        ids.sort_by_key(|id| graph.ports[id].props.get("audio.channel").cloned());
        ids
    };
    let outputs: Vec<_> = ports(source, Direction::Output).into_iter().take(2).collect();
    let inputs = ports(speakers, Direction::Input);
    assert_eq!(outputs.len(), 2);
    assert_eq!(inputs.len(), 2);
    let old = Handle { epoch, id: outputs[0] };
    let input = Handle { epoch, id: inputs[0] };
    backend.command(Command::Connect {
        out: old,
        input: Handle { epoch, id: outputs[1] },
        confirmed: false,
        request: 10,
    });
    expect_result(&backend, &mut graph, 10, false);
    backend.command(Command::Connect {
        out: Handle { epoch: epoch.wrapping_sub(1), id: outputs[0] },
        input,
        confirmed: false,
        request: 11,
    });
    expect_result(&backend, &mut graph, 11, false);
    for i in 0..2 {
        backend.command(Command::Connect {
            out: Handle { epoch, id: outputs[i] },
            input: Handle { epoch, id: inputs[i] },
            confirmed: false,
            request: 12 + i as u64,
        });
        expect_result(&backend, &mut graph, 12 + i as u64, true);
    }
    let links: Vec<_> = (0..2)
        .map(|i| *graph.links_by_pair.get(&(outputs[i], inputs[i])).expect("link global not observed").last().unwrap())
        .collect();
    assert_eq!(links.len(), 2);
    for (i, id) in links.iter().enumerate() {
        backend.command(Command::Disconnect { link: Handle { epoch, id: *id }, request: 20 + i as u64 });
        expect_result(&backend, &mut graph, 20 + i as u64, true);
    }
    assert!(links.iter().all(|id| !graph.links.contains_key(id)));
    let pid = std::env::var("MUSUBI_PRIVATE_PIPEWIRE_PID").expect("private daemon pid");
    assert!(ProcessCommand::new("kill").arg("-TERM").arg(&pid).status().unwrap().success());
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut down = false;
    while Instant::now() < deadline {
        match backend.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Connection(Connection::Down(_))) => {
                down = true;
                break;
            }
            Ok(Event::Delta(delta)) => graph.apply(delta),
            _ => {}
        }
    }
    assert!(down, "daemon shutdown not observed");
    let mut replacement = ProcessCommand::new("pipewire").spawn().expect("restart private PipeWire");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut restarted = false;
    while Instant::now() < deadline {
        match backend.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Connection(Connection::Up(_))) => restarted = true,
            Ok(Event::Delta(delta)) => graph.apply(delta),
            _ => {}
        }
        if restarted && graph.epoch > epoch {
            break;
        }
    }
    backend.command(Command::Disconnect { link: Handle { epoch, id: links[0] }, request: 30 });
    expect_result(&backend, &mut graph, 30, false);
    assert!(restarted && graph.epoch > epoch, "reconnect did not advance epoch");
    replacement.kill().unwrap();
    replacement.wait().unwrap();
}

fn expect_result(backend: &PipeWireBackend, graph: &mut Graph, request: u64, success: bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match backend.recv_timeout(Duration::from_millis(100)) {
            Ok(Event::Delta(delta)) => graph.apply(delta),
            Ok(Event::CommandResult { request: id, result }) if id == request => {
                assert_eq!(result.is_ok(), success, "request {request}: {result:?}");
                return;
            }
            Ok(Event::Connection(Connection::Down(reason))) => {
                panic!("PipeWire down during request {request}: {reason}")
            }
            _ => {}
        }
    }
    panic!("request {request} timed out");
}
