//! Filter-chain spike, confined to scripts/private-pipewire's instance.
use aizome_musubi::{
    backend::{Backend, Command as BackendCommand, Event, Handle, pipewire::PipeWireBackend},
    effects::{
        Band, BandKind, Chain, Target,
        paths::Paths,
        service::{EffectsService, UnitState, Units},
    },
    graph::Graph,
};
use serde_json::Value;
use std::{
    fs,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

fn output(program: &str, args: &[&str]) -> String {
    let result = Command::new(program).args(args).output().unwrap();
    assert!(result.status.success(), "{program} {args:?}: {}", String::from_utf8_lossy(&result.stderr));
    String::from_utf8(result.stdout).unwrap()
}
fn dump() -> Vec<Value> {
    serde_json::from_str(&output("pw-dump", &[])).unwrap()
}
fn node(name: &str) -> Option<u32> {
    let name = match name {
        "musubi_spike_capture" => "musubi_eq.spike",
        "musubi_spike_playback" => "musubi_eq.spike.playback",
        other => other,
    };
    dump()
        .into_iter()
        .find(|v| v["type"] == "PipeWire:Interface:Node" && v["info"]["props"]["node.name"] == name)
        .and_then(|v| v["id"].as_u64())
        .map(|v| v as u32)
}
fn until(label: &str, mut f: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(12);
    while !f() {
        assert!(Instant::now() < end, "timed out: {label}");
        thread::sleep(Duration::from_millis(25));
    }
}
fn start(config: &str, log: &str) -> Child {
    Command::new("pipewire")
        .args(["-c", config])
        .stdout(fs::File::create(log).unwrap())
        .stderr(fs::OpenOptions::new().append(true).open(log).unwrap())
        .spawn()
        .unwrap()
}
fn ports(id: u32, direction: &str) -> Vec<u32> {
    let mut found: Vec<_> = dump()
        .into_iter()
        .filter(|v| {
            v["type"] == "PipeWire:Interface:Port"
                && (v["info"]["props"]["node.id"]
                    .as_u64()
                    .map(|n| n as u32)
                    .or_else(|| v["info"]["props"]["node.id"].as_str().and_then(|n| n.parse::<u32>().ok())))
                    == Some(id)
                && v["info"]["direction"] == direction
        })
        .filter_map(|v| v["id"].as_u64().map(|id| id as u32))
        .collect();
    found.sort_unstable();
    found
}
fn link(from: u32, to: u32) -> bool {
    let a = ports(from, "output");
    let b = ports(to, "input");
    if a.len() != 2 || b.len() != 2 {
        return false;
    }
    for (a, b) in a.iter().zip(b.iter()) {
        output("pw-link", &[&a.to_string(), &b.to_string()]);
    }
    true
}
fn configure(id: u32, direction: &str, monitor: bool) {
    let param = format!(
        "{{ direction: {direction}, mode: dsp, monitor: {monitor}, format: {{ mediaType: audio, mediaSubtype: raw, format: F32P, rate: 48000, channels: 2, position: [ FL FR ] }} }}"
    );
    output("pw-cli", &["set-param", &id.to_string(), "PortConfig", &param]);
}
fn drain(backend: &mut PipeWireBackend, graph: &mut Graph) {
    while let Some(event) = backend.next_event() {
        if let Event::Delta(delta) = event {
            graph.apply(delta);
        } else if let Event::CommandResult { request, result } = event {
            println!("effect command {request}: {result:?}");
            assert!(result.is_ok(), "{result:?}");
        }
    }
}
fn peak(backend: &PipeWireBackend, handle: Handle) -> Option<f32> {
    backend
        .meters()
        .iter()
        .find(|m| m.node == handle && !m.paused && m.last_update.is_some())
        .map(|m| m.peaks.iter().copied().fold(0.0, f32::max))
}
struct NoUnits;
impl Units for NoUnits {
    fn enable(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn start(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn stop(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn disable(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn listed(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
    fn state(&self, _: &str) -> Result<UnitState, String> {
        Ok(UnitState::default())
    }
    fn owned(&self, _: &str) -> Result<bool, String> {
        Ok(true)
    }
}

#[test]
fn chain_control_and_lifetime() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    let root = std::env::var("MUSUBI_PRIVATE_PIPEWIRE_ROOT").unwrap();
    assert_eq!(std::env::var("PIPEWIRE_RUNTIME_DIR").unwrap(), format!("{root}/runtime"));
    let service = EffectsService { paths: Paths::from_environment().unwrap(), units: NoUnits };
    let chain = Chain {
        id: "spike".into(),
        name: "Spike".into(),
        target: Target::Output("musubi_test_speakers".into()),
        bands: vec![Band { kind: BandKind::Peaking, freq: 1000.0, gain: 0.0, q: 1.0 }],
        bypass: false,
        enabled: true,
    };
    service.save(&chain).unwrap();
    let config = service.paths.conf("spike").unwrap().to_string_lossy().into_owned();
    let log = format!("{root}/effects.log");
    let mut helper = start(&config, &log);
    until("chain nodes", || node("musubi_spike_capture").is_some() && node("musubi_spike_playback").is_some());
    for name in ["musubi_spike_capture", "musubi_spike_playback"] {
        let id = node(name).unwrap();
        let props = output("pw-cli", &["enum-params", &id.to_string(), "Props"]);
        fs::write(format!("{root}/{name}-props.txt"), &props).unwrap();
        println!("{name} ({id}): {props}");
    }
    let mut backend = PipeWireBackend::channel().unwrap();
    let mut graph = Graph::default();
    until("backend sees chain", || {
        drain(&mut backend, &mut graph);
        graph.nodes.values().any(|n| n.props.get("node.name").is_some_and(|v| v == "musubi_eq.spike"))
    });
    drop(backend);
    assert!(node("musubi_spike_capture").is_some(), "helper must outlive Musubi's backend");
    let mut backend = PipeWireBackend::channel().unwrap();
    let mut graph = Graph::default();
    let capture = node("musubi_spike_capture").unwrap();
    let playback = node("musubi_spike_playback").unwrap();
    let sink = node("musubi_test_speakers").unwrap();
    until("backend reconnects", || {
        drain(&mut backend, &mut graph);
        graph.nodes.contains_key(&capture)
    });
    let handle = Handle { epoch: graph.epoch, id: capture };
    let sink_handle = Handle { epoch: graph.epoch, id: sink };
    backend.command(BackendCommand::StartMeters(vec![sink_handle]));
    until("meter node", || dump().iter().any(|v| v["info"]["props"]["musubi.meter"] == true));
    let meter =
        dump().iter().find(|v| v["info"]["props"]["musubi.meter"] == true).unwrap()["id"].as_u64().unwrap() as u32;
    configure(meter, "Input", true);
    until("meter ports", || link(sink, meter));
    if ports(capture, "input").is_empty() {
        configure(capture, "Input", false);
    }
    if ports(playback, "output").is_empty() {
        configure(playback, "Output", false);
    }
    until("chain playback ports", || link(playback, sink));
    let tone_file = format!("{root}/effect-tone.raw");
    let tone: Vec<u8> = (0..48_000 * 15)
        .flat_map(|frame| {
            let sample = ((frame as f32 * 1000. * std::f32::consts::TAU / 48_000.).sin() * 2500.) as i16;
            [sample.to_le_bytes(), sample.to_le_bytes()].concat()
        })
        .collect();
    fs::write(&tone_file, tone).unwrap();
    let mut player = Command::new("pw-cat")
        .args(["--playback", "--raw", "--rate", "48000", "--channels", "2", "--format", "s16", "--target", "0"])
        .args(["-P", "{ node.name = musubi_effect_tone }", &tone_file])
        .spawn()
        .unwrap();
    until("tone node", || node("musubi_effect_tone").is_some());
    let player_id = node("musubi_effect_tone").unwrap();
    configure(player_id, "Output", false);
    until("tone ports", || link(player_id, capture));
    until("flat effect tone peak", || peak(&backend, sink_handle).is_some_and(|p| p > 0.02));
    let flat = peak(&backend, sink_handle).unwrap();
    backend.command(BackendCommand::SetEffectParams {
        node: handle,
        params: vec![("left0:Gain".into(), 12.), ("right0:Gain".into(), 12.)],
        request: 1,
    });
    until("gain readback", || {
        drain(&mut backend, &mut graph);
        let props = output("pw-cli", &["enum-params", &capture.to_string(), "Props"]);
        props.contains("String \"left0:Gain\"\n        Float 12.000000")
            && props.contains("String \"right0:Gain\"\n        Float 12.000000")
    });
    let readback = output("pw-cli", &["enum-params", &capture.to_string(), "Props"]);
    fs::write(format!("{root}/capture-after-props.txt"), &readback).unwrap();
    until("boosted output peak", || peak(&backend, sink_handle).is_some_and(|p| p > flat * 3.5));
    let boosted = peak(&backend, sink_handle).unwrap();
    let db = 20. * (boosted / flat).log10();
    println!("effect peak flat={flat:.5} boosted={boosted:.5} change={db:.2} dB");
    assert!((db - 12.).abs() < 2., "expected ~12 dB gain, got {db:.2}");
    player.kill().unwrap();
    player.wait().unwrap();
    backend.command(BackendCommand::StopMeters);
    drop(backend);
    assert!(node("musubi_spike_capture").is_some(), "disconnect must not stop helper");
    let restart = Instant::now();
    helper.kill().unwrap();
    helper.wait().unwrap();
    until("chain removed", || node("musubi_spike_capture").is_none() && node("musubi_spike_playback").is_none());
    assert!(node("musubi_test_speakers").is_some(), "restart only this chain");
    let mut helper = start(&config, &log);
    until("chain returned", || node("musubi_spike_capture").is_some() && node("musubi_spike_playback").is_some());
    println!("effect helper restart gap={} ms", restart.elapsed().as_millis());
    helper.kill().unwrap();
    helper.wait().unwrap();
    until("restarted chain removed", || node("musubi_spike_capture").is_none());
    service.delete("spike").unwrap();
    assert!(!service.paths.conf("spike").unwrap().exists());
    assert!(!service.paths.json("spike").unwrap().exists());
    println!("helper log: {}", fs::read_to_string(log).unwrap());
}
