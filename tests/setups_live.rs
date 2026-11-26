//! Private PipeWire only: scripts/private-pipewire supplies isolated sockets and XDG paths.
use aizome_kit::testing::Harness;
use aizome_musubi::{
    app::{Msg, Musubi},
    graph::{Direction, GlobalId},
    setups::{Route, Setup, VERSION, paths::Paths, store::Store},
};
use std::{process::Command, time::Duration};

fn port(app: &Musubi, node: &str, channel: &str, direction: Direction) -> Option<GlobalId> {
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
}

fn pw_link(args: &[String]) {
    let output = Command::new("pw-link").args(args).output().expect("private pw-link");
    assert!(output.status.success(), "pw-link {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn keep_restores_removed_link_and_exclusive_removes_stray_link() {
    if std::env::var("MUSUBI_LIVE").as_deref() != Ok("1") {
        return;
    }
    assert!(std::env::var("MUSUBI_PRIVATE_PIPEWIRE_ROOT").is_ok(), "requires scripts/private-pipewire");
    // The private fixture has no WirePlumber: pw-cat streams have no configured
    // ports. Exercise its null sink's ports and the real link-factory instead.
    let mut app = Musubi::default();
    app.live = true;
    let mut h = Harness::new(app);
    let ports = |app: &Musubi| {
        (
            port(app, "musubi_test_interface_out", "AUX0", Direction::Output),
            port(app, "musubi_test_speakers", "FL", Direction::Input),
        )
    };
    assert!(h.settle(Duration::from_secs(15), |app| matches!(ports(app), (Some(_), Some(_)))));
    let (Some(out), Some(input)) = ports(h.program()) else { unreachable!() };
    let graph = &h.program().graph;
    let setup = Setup {
        version: VERSION,
        id: "live-keep".into(),
        name: "Live keep".into(),
        routes: vec![Route {
            output: graph.stable_port_id(&graph.ports[&out]).unwrap(),
            input: graph.stable_port_id(&graph.ports[&input]).unwrap(),
        }],
        keep_connected: true,
        exclusive: true,
    };
    let store = Store { paths: Paths::from_environment().unwrap() };
    store.save(&setup).unwrap();
    h.send(Msg::ActivateSetup(setup.id.clone()));
    assert!(
        h.settle(Duration::from_secs(8), |app| {
            let (Some(out), Some(input)) = ports(app) else { return false };
            app.graph.has_active_link((out, input))
        }),
        "activation link not observed"
    );
    let old_link = *h.program().graph.links_by_pair[&(out, input)].first().unwrap();
    pw_link(&["-d".into(), out.to_string(), input.to_string()]);
    assert!(
        h.settle(Duration::from_secs(15), |app| {
            let (Some(out), Some(input)) = ports(app) else { return false };
            app.graph.links_by_pair.get(&(out, input)).is_some_and(|ids| ids.iter().any(|id| *id != old_link))
        }),
        "keep did not relink the fixture ports"
    );
    let (Some(out), _) = ports(h.program()) else { unreachable!() };
    let stray = port(h.program(), "musubi_test_speakers", "FR", Direction::Input).unwrap();
    pw_link(&[out.to_string(), stray.to_string()]);
    assert!(
        h.settle(Duration::from_secs(8), |app| !app.graph.links_by_pair.contains_key(&(out, stray))),
        "exclusive did not remove stray link"
    );
    h.send(Msg::DeactivateSetup);
    // The private daemon and its fixture are destroyed by live.sh on exit.
}
