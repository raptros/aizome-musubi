//! Run with MUSUBI_LIVE=1 inside scripts/private-pipewire; never use the host daemon.
use std::time::{Duration, Instant};
use aizome_musubi::{backend::{Connection, Event, pipewire::PipeWireBackend}, graph::{Graph, classify::PortKind}};

#[test]
fn enumerates_private_audio_midi_video() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") { return; }
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
        if up && graph.nodes.values().filter(|n| n.props.get("node.name").is_some_and(|s| s.starts_with("musubi_test_"))).count() >= 5
            && graph.ports_of_kind(PortKind::Midi).count() >= 2
            && graph.ports_of_kind(PortKind::Video).count() >= 1 { break; }
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
