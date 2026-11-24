use aizome_musubi::{
    app::Musubi,
    instance::{self, Claim},
};
use aizome_musubi::{
    backend::{Backend, Event, Handle, pipewire::PipeWireBackend},
    effects::{
        Band, BandKind, Chain, Target,
        paths::Paths,
        service::{EffectsService, Systemd},
    },
    graph::Graph,
};
use std::{
    io, thread,
    time::{Duration, Instant},
};

fn effects(args: &[String]) -> io::Result<()> {
    let service =
        EffectsService { paths: Paths::from_environment()?, units: Systemd::user().map_err(io::Error::other)? };
    match args {
        [action] if action == "list" => {
            for chain in service.list()? {
                println!("{}\t{}\t{}", chain.id, if chain.enabled { "enabled" } else { "disabled" }, chain.name);
            }
            Ok(())
        }
        // Temporary VM creation/editing probes; P3-S3 replaces them with Effects-page input.
        [action, id, target] if action == "create" => {
            let chain = Chain {
                id: id.clone(),
                name: id.clone(),
                target: Target::Output(target.clone()),
                bands: vec![Band { kind: BandKind::Peaking, freq: 440.0, gain: 0.0, q: 1.0 }],
                bypass: false,
                enabled: true,
            };
            service.enable(&chain)
        }
        [action, id] if action == "enable" => service.enable(&service.load(id)?),
        [action, id] if action == "disable" => service.disable(id),
        [action, id] if action == "remove" => service.delete(id),
        [action] if action == "remove-all" => service.remove_all(),
        [action, id, value] if action == "boost" => {
            let gain: f32 = value.parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid gain"))?;
            let mut backend = PipeWireBackend::channel().map_err(io::Error::other)?;
            let mut graph = Graph::default();
            let deadline = Instant::now() + Duration::from_secs(15);
            let capture = loop {
                while let Some(event) = backend.next_event() {
                    if let Event::Delta(delta) = event {
                        graph.apply(delta);
                    }
                }
                if let Some(node) = graph
                    .nodes
                    .values()
                    .find(|node| node.props.get("node.name").is_some_and(|name| *name == format!("musubi_eq.{id}")))
                {
                    break Handle { epoch: graph.epoch, id: node.id };
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "EQ node not observed"));
                }
                thread::sleep(Duration::from_millis(15));
            };
            eprintln!(
                "effect capture id={} epoch={} initial={:?}",
                capture.id,
                capture.epoch,
                graph.nodes.get(&capture.id).map(|n| &n.native.effect_params)
            );
            let mut chain = service.load(id)?;
            let band = chain
                .bands
                .first_mut()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chain has no band"))?;
            band.gain = gain;
            let candidate = service.edit_band(&mut backend, capture, 42, id, 0, band.clone())?;
            eprintln!(
                "effect write request=42 node={} params={:?}",
                capture.id,
                [
                    (format!("left0:Freq"), candidate.bands[0].freq),
                    (format!("left0:Q"), candidate.bands[0].q),
                    (format!("left0:Gain"), candidate.bands[0].gain)
                ]
            );
            let mut acknowledged = false;
            loop {
                while let Some(event) = backend.next_event() {
                    match event {
                        Event::Delta(delta) => {
                            for change in &delta.upserts {
                                if let aizome_musubi::graph::Upsert::Node(node) = change {
                                    if node.id == capture.id {
                                        eprintln!(
                                            "effect Props delta node={} values={:?} writable={}",
                                            node.id, node.native.effect_params, node.native.writable
                                        );
                                    }
                                }
                            }
                            graph.apply(delta);
                        }
                        Event::CommandResult { request: 42, result } => {
                            result.map_err(io::Error::other)?;
                            acknowledged = true;
                            eprintln!("effect request=42 acknowledged");
                        }
                        _ => {}
                    }
                }
                if acknowledged {
                    if let Some(node) = graph.nodes.get(&capture.id) {
                        match service.save_observed_edit(&candidate, 0, &node.native.effect_params) {
                            Ok(()) => return Ok(()),
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                            Err(e) => return Err(e),
                        }
                    }
                }
                if Instant::now() >= deadline {
                    eprintln!(
                        "effect timeout node={} acknowledged={} last={:?}",
                        capture.id,
                        acknowledged,
                        graph.nodes.get(&capture.id).map(|n| &n.native.effect_params)
                    );
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "EQ Props read-back not observed"));
                }
                thread::sleep(Duration::from_millis(15));
            }
        }
        // VM audio probe: removed when the Effects page can drive an EQ through real input (P3-S3).
        [action, name] if action == "meter" => {
            let mut backend = PipeWireBackend::channel().map_err(io::Error::other)?;
            let mut graph = Graph::default();
            let deadline = Instant::now() + Duration::from_secs(12);
            let started = Instant::now();
            let mut metered = None;
            let mut peak = 0.0_f32;
            loop {
                while let Some(event) = backend.next_event() {
                    if let Event::Delta(delta) = event {
                        graph.apply(delta);
                    }
                }
                if metered.is_none() {
                    if let Some(node) = graph.nodes.values().find(|n| n.props.get("node.name") == Some(name)) {
                        let handle = Handle { epoch: graph.epoch, id: node.id };
                        backend.command(aizome_musubi::backend::Command::StartMeters(vec![handle]));
                        metered = Some(handle);
                    }
                }
                if let Some(handle) = metered {
                    for meter in
                        backend.meters().iter().filter(|m| m.node == handle && !m.paused && m.last_update.is_some())
                    {
                        peak = peak.max(meter.peaks.iter().copied().fold(0.0_f32, f32::max));
                    }
                }
                if peak > 0.0 && started.elapsed() >= Duration::from_secs(2) {
                    println!("{peak}");
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    if peak > 0.0 {
                        println!("{peak}");
                        return Ok(());
                    }
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "no observed meter samples"));
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "effects: list | remove <chain> | remove-all")),
    }
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|a| a == "effects") {
        if let Err(error) = effects(&arguments[1..]) {
            eprintln!("Musubi effects: {error}");
            std::process::exit(1);
        }
        return;
    }
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--version"), None) => {
            println!("aizome-musubi {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        (Some("--help" | "-h"), None) => {
            println!(
                "Musubi 結び — the PipeWire patchbay for Aizome\n\nUsage: aizome-musubi [--help | --version | effects list | effects remove <chain> | effects remove-all]"
            );
            return;
        }
        (None, None) => {}
        _ => {
            eprintln!("Use --help for usage.");
            std::process::exit(2);
        }
    }
    let mut app = Musubi::default();
    app.live = true;
    match instance::claim() {
        Ok(Claim::Handed) => return,
        Ok(Claim::First(instance)) => app.instance = Some(instance),
        Err(error) => {
            eprintln!("Musubi: {error}");
            std::process::exit(1);
        }
    }
    aizome_kit::run(app);
}
