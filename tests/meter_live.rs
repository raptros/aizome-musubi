//! One private PipeWire instance; never run against the host daemon.
use aizome_musubi::{
    backend::{Backend, Command, Connection, Event, Handle, pipewire::PipeWireBackend},
    graph::Graph,
};
use std::{
    process::Command as ProcessCommand,
    time::{Duration, Instant},
};

fn wait_for(label: &str, mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(12);
    while !check() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        std::thread::sleep(Duration::from_millis(40));
    }
}
fn dump_meters() -> usize {
    let dump = ProcessCommand::new("pw-dump").output().expect("private pw-dump");
    assert!(dump.status.success());
    let values: serde_json::Value = serde_json::from_slice(&dump.stdout).unwrap();
    values
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry["type"] == "PipeWire:Interface:Node"
                && (entry["info"]["props"]["musubi.meter"] == true || entry["props"]["musubi.meter"] == true)
        })
        .count()
}
/// Private PipeWire has no WirePlumber. Configure the stream's adapter
/// PortConfig and link its inputs to sink monitor outputs as policy would.
fn configure_meter() {
    let dump = ProcessCommand::new("pw-dump").output().unwrap();
    let values: serde_json::Value = serde_json::from_slice(&dump.stdout).unwrap();
    let id = values
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["type"] == "PipeWire:Interface:Node" && e["info"]["props"]["musubi.meter"] == true)
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    let config = "{ direction: Input, mode: dsp, monitor: true, format: { mediaType: audio, mediaSubtype: raw, format: F32P, rate: 48000, channels: 2, position: [ FL FR ] } }";
    let output =
        ProcessCommand::new("pw-cli").args(["set-param", &id.to_string(), "PortConfig", config]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
fn dump() -> Vec<serde_json::Value> {
    let dump = ProcessCommand::new("pw-dump").output().unwrap();
    serde_json::from_slice::<serde_json::Value>(&dump.stdout).unwrap().as_array().unwrap().clone()
}
fn node_id(name: &str) -> Option<u64> {
    dump()
        .iter()
        .find(|e| e["type"] == "PipeWire:Interface:Node" && e["info"]["props"]["node.name"] == name)
        .and_then(|e| e["id"].as_u64())
}
/// Links `from`'s ports in `from_dir` to `to`'s ports in `to_dir`, in id
/// order; false while either side has no ports yet.
fn link_ports(from: u64, from_dir: &str, to: u64, to_dir: &str) -> bool {
    let entries = dump();
    let ports = |node: u64, direction: &str| -> Vec<u64> {
        let mut ports: Vec<_> = entries
            .iter()
            .filter(|e| {
                e["type"] == "PipeWire:Interface:Port"
                    && e["info"]["props"]["node.id"].as_u64() == Some(node)
                    && e["info"]["direction"].as_str() == Some(direction)
            })
            .filter_map(|e| e["id"].as_u64())
            .collect();
        ports.sort_unstable();
        ports
    };
    let outputs = ports(from, from_dir);
    let inputs = ports(to, to_dir);
    if outputs.is_empty() || inputs.is_empty() {
        return false;
    }
    assert_eq!(outputs.len(), inputs.len(), "channel counts of {from} and {to}");
    for (output, input) in outputs.into_iter().zip(inputs) {
        let linked = ProcessCommand::new("pw-link").args([output.to_string(), input.to_string()]).output().unwrap();
        assert!(linked.status.success(), "{}", String::from_utf8_lossy(&linked.stderr));
    }
    true
}
fn link_monitor(sink: u32) -> bool {
    let meter = dump()
        .iter()
        .find(|e| e["type"] == "PipeWire:Interface:Node" && e["info"]["props"]["musubi.meter"] == true)
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    link_ports(sink as u64, "output", meter, "input")
}
fn drain(backend: &mut PipeWireBackend, graph: &mut Graph) {
    while let Some(event) = backend.next_event() {
        match event {
            Event::Delta(delta) => graph.apply(delta),
            Event::Connection(Connection::Down(reason)) => panic!("private PipeWire connection lost: {reason}"),
            _ => {}
        }
    }
}
#[test]
fn sink_monitor_tone_silence_and_stop() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    let root = std::env::var("MUSUBI_PRIVATE_PIPEWIRE_ROOT").expect("private daemon required");
    assert_eq!(std::env::var("PIPEWIRE_RUNTIME_DIR").unwrap(), format!("{root}/runtime"));
    let mut backend = PipeWireBackend::channel().unwrap();
    let mut graph = Graph::default();
    wait_for("Test Speakers sink", || {
        drain(&mut backend, &mut graph);
        graph.nodes.values().any(|n| n.props.get("node.name").is_some_and(|s| s == "musubi_test_speakers"))
    });
    let id = graph
        .nodes
        .values()
        .find(|n| n.props.get("node.name").is_some_and(|s| s == "musubi_test_speakers"))
        .unwrap()
        .id;
    let handle = Handle { epoch: graph.epoch, id };
    backend.command(Command::StartMeters(vec![handle]));
    wait_for("meter stream node in private pw-dump", || dump_meters() == 1);
    configure_meter();
    wait_for("meter input ports", || link_monitor(id));
    assert!(!graph.nodes.values().any(|n| n.props.get("musubi.meter").is_some_and(|v| v == "true")));
    // A private pw-cat tone; no host audio device is opened. Without
    // WirePlumber, configure and link its ports as policy would.
    let tone_file = format!("{root}/tone.raw");
    let tone: Vec<u8> = (0..48_000 * 20)
        .flat_map(|frame| {
            let sample = ((frame as f32 * 440. * std::f32::consts::TAU / 48_000.).sin() * 16_000.) as i16;
            [sample.to_le_bytes(), sample.to_le_bytes()].concat()
        })
        .collect();
    std::fs::write(&tone_file, tone).unwrap();
    let mut player = ProcessCommand::new("pw-cat")
        .args(["--playback", "--raw", "--rate", "48000", "--channels", "2", "--format", "s16", "--target", "0"])
        .args(["-P", "{ node.name = musubi_meter_tone }", &tone_file])
        .spawn()
        .expect("private pw-cat tone");
    wait_for("tone stream node", || node_id("musubi_meter_tone").is_some());
    let tone_node = node_id("musubi_meter_tone").unwrap();
    let config = "{ direction: Output, mode: dsp, format: { mediaType: audio, mediaSubtype: raw, format: F32P, rate: 48000, channels: 2, position: [ FL FR ] } }";
    let output = ProcessCommand::new("pw-cli")
        .args(["set-param", &tone_node.to_string(), "PortConfig", config])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    wait_for("tone linked to Test Speakers", || link_ports(tone_node, "output", id as u64, "input"));
    wait_for("nonzero sink monitor peak", || {
        backend
            .meters()
            .iter()
            .find(|m| m.node == handle)
            .is_some_and(|m| !m.paused && m.last_update.is_some() && m.peaks.iter().any(|&v| v > 0.01))
    });
    player.kill().unwrap();
    player.wait().unwrap();
    // With the tone gone the passive meter may stop receiving buffers (the
    // graph suspends); either silent buffers or read-side decay must show silence.
    wait_for("sink monitor silence", || {
        backend.meters().iter().find(|m| m.node == handle).is_some_and(|m| m.peaks.iter().all(|&v| v < 0.01))
    });
    backend.command(Command::StopMeters);
    wait_for("meter node torn down", || backend.meters().is_empty() && dump_meters() == 0);
    println!("private sink monitor observed tone, silence, and zero meter nodes after stop");
}
