//! Private PipeWire/Pulse only: `MUSUBI_LIVE=1 scripts/private-pipewire -- cargo test --test mixer_live`.
use aizome_musubi::{
    backend::{Backend, Command, Event, Handle, pipewire::PipeWireBackend},
    graph::Graph,
    mixer::model::{StripControl, strips},
};
use aizome_sys::{Wake, audio::Audio};
use std::time::{Duration, Instant};

fn until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    panic!("private audio state did not reach expected value");
}

fn drain(backend: &mut PipeWireBackend, graph: &mut Graph) {
    while let Some(event) = backend.next_event() {
        match event {
            Event::Delta(delta) => graph.apply(delta),
            Event::Connection(aizome_musubi::backend::Connection::Down(reason)) => panic!("private daemon: {reason}"),
            Event::CommandResult { result: Err(reason), .. } => panic!("native command: {reason}"),
            _ => {}
        }
    }
}

#[test]
fn pulse_and_native_props_read_back() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    let root = std::env::var("MUSUBI_PRIVATE_PIPEWIRE_ROOT").expect("private runtime required");
    let server = format!("unix:{root}/runtime/pulse/native");
    assert_eq!(std::env::var("PULSE_SERVER").unwrap(), server);
    assert_eq!(std::env::var("PIPEWIRE_RUNTIME_DIR").unwrap(), format!("{root}/runtime"));
    let audio = Audio::with_server(Some(server), Wake::none());
    let mut backend = PipeWireBackend::channel().unwrap();
    let mut graph = Graph::default();
    // All events are observed on the one backend thread.
    until(|| {
        drain(&mut backend, &mut graph);
        audio.snapshot().sinks.iter().any(|s| s.name == "musubi_test_speakers")
            && graph.nodes.values().any(|n| n.props.get("node.name").is_some_and(|s| s == "musubi_test_speakers"))
    });
    let index = audio.snapshot().sinks.iter().find(|s| s.name == "musubi_test_speakers").unwrap().index;
    let node =
        graph.nodes.values().find(|n| n.props.get("node.name").is_some_and(|s| s == "musubi_test_speakers")).unwrap();
    assert_eq!(node.props["object.serial"].parse::<u32>().unwrap(), index);
    assert!(
        strips(&audio.snapshot(), &graph)
            .iter()
            .any(|s| matches!(s.control, StripControl::Pulse { index: i, .. } if i == index))
    );
    audio.set_sink_volume(index, 0.5).wait_timeout(Duration::from_secs(5)).unwrap().unwrap();
    audio.set_sink_balance(index, 0.2).wait_timeout(Duration::from_secs(5)).unwrap().unwrap();
    audio.set_sink_mute(index, true).wait_timeout(Duration::from_secs(5)).unwrap().unwrap();
    until(|| {
        drain(&mut backend, &mut graph);
        let state = audio.snapshot();
        let pulse = state.sink(index).unwrap();
        let native = graph
            .nodes
            .values()
            .find(|n| n.props.get("node.name").is_some_and(|s| s == "musubi_test_speakers"))
            .unwrap();
        (pulse.volume - 0.5).abs() < 0.02
            && (pulse.balance - 0.2).abs() < 0.03
            && pulse.muted
            && native.native.muted == Some(true)
            && native.native.channel_volumes.as_ref().is_some_and(|v| v.iter().all(|gain| *gain < 0.9))
    });
    let native =
        graph.nodes.values().find(|n| n.props.get("node.name").is_some_and(|s| s == "musubi_native_only")).unwrap();
    assert!(
        !audio
            .snapshot()
            .sinks
            .iter()
            .chain(audio.snapshot().sources.iter())
            .any(|d| d.index.to_string() == native.props["object.serial"])
    );
    let handle = Handle { epoch: graph.epoch, id: native.id };
    until(|| {
        drain(&mut backend, &mut graph);
        graph.nodes.get(&handle.id).is_some_and(|n| n.native.writable && n.native.channel_volumes.is_some())
    });
    let channels = graph.nodes[&handle.id].native.channel_volumes.as_ref().unwrap().len();
    assert!(channels > 0);
    backend.command(Command::SetNodeVolume { node: handle, channel_volumes: vec![0.25; channels], request: 801 });
    until(|| {
        drain(&mut backend, &mut graph);
        graph.nodes[&handle.id]
            .native
            .channel_volumes
            .as_ref()
            .is_some_and(|v| v.iter().all(|x| (*x - 0.25).abs() < 0.01))
    });
    assert!(matches!(
        strips(&audio.snapshot(), &graph).iter().find(|s| s.node == Some(handle)).unwrap().control,
        StripControl::Native { .. }
    ));
    println!(
        "private Pulse sink index {index}: 50%, balance 0.2, mute; native node {}: 0.25 per channel observed",
        handle.id
    );
}
