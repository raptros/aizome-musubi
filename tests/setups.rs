use aizome_kit::testing::Harness;
use aizome_musubi::{
    app::{Msg, Musubi},
    backend::{Backend, Command, fake::FakeBackend},
};
use aizome_musubi::{
    graph::{
        ClientRecord, DeviceRecord, Direction, Graph, GraphDelta, LinkRecord, LinkState, NativeProps, NodeRecord,
        PortRecord, Properties, Upsert, identity::Resolution,
    },
    setups::{
        Active, End, Route, RouteStatus, Setup, VERSION,
        paths::Paths,
        plan,
        reconcile::{self, Action, State},
        store::Store,
    },
};
use std::time::{Duration, Instant};
use std::{cell::RefCell, rc::Rc};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "musubi-setups-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn props(pairs: &[(&str, &str)]) -> Properties {
    pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}
fn device(id: u32, name: &str, serial: Option<&str>) -> Upsert {
    let mut properties = props(&[("device.name", name)]);
    if let Some(serial) = serial {
        properties.insert("device.serial".into(), serial.into());
    }
    Upsert::Device(DeviceRecord { id, props: properties })
}
fn app(id: u32, name: &str, binary: &str) -> Upsert {
    Upsert::Client(ClientRecord {
        id,
        props: props(&[("application.name", name), ("application.process.binary", binary)]),
    })
}
fn node(id: u32, name: &str, device_id: Option<u32>, client_id: Option<u32>, media: &str) -> Upsert {
    Upsert::Node(NodeRecord {
        id,
        device_id,
        client_id,
        props: props(&[("node.name", name), ("media.name", media), ("media.class", "Audio/Source")]),
        native: NativeProps::default(),
    })
}
fn port(id: u32, node_id: u32, direction: Direction) -> Upsert {
    Upsert::Port(PortRecord {
        id,
        node_id: Some(node_id),
        direction,
        enum_formats: vec![],
        props: props(&[("port.name", "FL"), ("format.dsp", "32 bit float mono audio")]),
    })
}
fn link(id: u32, output: u32, input: u32) -> Upsert {
    Upsert::Link(LinkRecord { id, output, input, state: LinkState::Active, props: Properties::new() })
}
fn fixture(epoch: u64, upserts: Vec<Upsert>) -> Graph {
    let mut graph = Graph::default();
    graph.apply(GraphDelta { epoch, upserts, removals: vec![] });
    graph
}
fn setup(graph: &Graph, output: u32, input: u32) -> Setup {
    Setup {
        version: VERSION,
        id: "studio".into(),
        name: "Studio".into(),
        routes: vec![Route {
            output: graph.stable_port_id(&graph.ports[&output]).unwrap(),
            input: graph.stable_port_id(&graph.ports[&input]).unwrap(),
        }],
        keep_connected: true,
        exclusive: true,
    }
}
fn hardware(base: u32, serial: Option<&str>) -> Vec<Upsert> {
    vec![
        device(base, "usb-card", serial),
        node(base + 1, "usb.capture", Some(base), None, ""),
        port(base + 2, base + 1, Direction::Output),
    ]
}
fn target(base: u32) -> Vec<Upsert> {
    vec![
        device(base, "speakers", Some("speaker-serial")),
        node(base + 1, "speaker.input", Some(base), None, ""),
        port(base + 2, base + 1, Direction::Input),
    ]
}

#[test]
fn replug_changes_globals_and_object_serial_without_changing_identity() {
    let original = fixture(1, [hardware(10, Some("usb-serial")), target(20)].concat());
    let saved = setup(&original, 12, 22);
    let mut changed = [hardware(100, Some("usb-serial")), target(200)].concat();
    if let Upsert::Node(node) = &mut changed[1] {
        node.props.insert("object.serial".into(), "99999".into());
    }
    let graph = fixture(2, changed);
    assert_eq!(plan(&graph, &saved).routes, vec![RouteStatus::Linkable(102, 202)]);
}

#[test]
fn identical_usb_devices_without_serial_are_ambiguous() {
    let original = fixture(1, [hardware(10, None), target(20)].concat());
    let saved = setup(&original, 12, 22);
    let graph = fixture(2, [hardware(100, None), hardware(110, None), target(200), vec![link(300, 102, 202)]].concat());
    let result = plan(&graph, &saved);
    assert_eq!(result.routes, vec![RouteStatus::Ambiguous { which: End::Output, candidates: vec![102, 112] }]);
    assert!(result.foreign_links.is_empty());
}

#[test]
fn app_restart_matches_but_same_app_instances_are_ambiguous() {
    let stream = |client, n, p, media| {
        vec![
            app(client, "Player", "pw-play"),
            node(n, "generic-stream", None, Some(client), media),
            port(p, n, Direction::Output),
        ]
    };
    let original = fixture(1, [stream(1, 2, 3, "Track One"), target(20)].concat());
    let saved = setup(&original, 3, 22);
    let restarted = fixture(2, [stream(100, 101, 102, "Track Two"), target(200)].concat());
    assert_eq!(restarted.resolve(&saved.routes[0].output), Resolution::Unique(102));
    assert_eq!(plan(&restarted, &saved).routes, vec![RouteStatus::Linkable(102, 202)]);
    let doubled =
        fixture(3, [stream(100, 101, 102, "Track Two"), stream(110, 111, 112, "Another Track"), target(200)].concat());
    assert_eq!(
        plan(&doubled, &saved).routes,
        vec![RouteStatus::Ambiguous { which: End::Output, candidates: vec![102, 112] }]
    );
}

#[test]
fn foreign_links_touch_only_uniquely_named_setup_ports() {
    let original = fixture(1, [hardware(10, Some("usb-serial")), target(20)].concat());
    let saved = setup(&original, 12, 22);
    let graph = fixture(
        2,
        [
            hardware(10, Some("usb-serial")),
            target(20),
            vec![
                device(30, "other-speakers", None),
                node(31, "other.input", Some(30), None, ""),
                port(32, 31, Direction::Input),
            ],
            hardware(40, Some("other")),
            vec![link(100, 12, 22), link(101, 12, 32), link(102, 42, 22), link(103, 42, 32)],
        ]
        .concat(),
    );
    let result = plan(&graph, &saved);
    assert_eq!(result.routes, vec![RouteStatus::Linked(100)]);
    assert_eq!(result.foreign_links, vec![101, 102]);
}

#[test]
fn negotiating_link_is_linked() {
    let initial = fixture(1, [hardware(10, Some("usb-serial")), target(20)].concat());
    let saved = setup(&initial, 12, 22);
    let graph = fixture(
        2,
        [
            hardware(10, Some("usb-serial")),
            target(20),
            vec![Upsert::Link(LinkRecord {
                id: 100,
                output: 12,
                input: 22,
                state: LinkState::Negotiating,
                props: Properties::new(),
            })],
        ]
        .concat(),
    );
    assert_eq!(plan(&graph, &saved).routes, vec![RouteStatus::Linked(100)]);
    assert!(reconcile::step(&graph, &saved, State::default(), Instant::now()).actions.is_empty());
}

#[test]
fn keep_recreates_after_node_replacement_without_repeating_inflight() {
    let initial = fixture(1, [hardware(10, Some("usb-serial")), target(20), vec![link(90, 12, 22)]].concat());
    let saved = setup(&initial, 12, 22);
    let now = Instant::now();
    assert!(reconcile::step(&initial, &saved, State::default(), now).actions.is_empty());
    let replaced = fixture(2, [hardware(100, Some("usb-serial")), target(200)].concat());
    let second = reconcile::step(&replaced, &saved, State::default(), now);
    assert_eq!(second.actions, vec![Action::CreateLink(102, 202)]);
    let third = reconcile::step(&replaced, &saved, second.state, now);
    assert!(third.actions.is_empty());
    let observed = fixture(2, [hardware(100, Some("usb-serial")), target(200), vec![link(300, 102, 202)]].concat());
    assert!(reconcile::step(&observed, &saved, third.state, now).actions.is_empty());
}

#[test]
fn exclusive_limits_each_stable_foreign_pair_and_notices() {
    let initial = fixture(1, [hardware(10, Some("usb-serial")), target(20)].concat());
    let mut saved = setup(&initial, 12, 22);
    saved.keep_connected = false;
    let now = Instant::now();
    let mut state = State::default();
    for attempt in 0..4 {
        let id = 100 + attempt;
        let graph = fixture(
            2,
            [
                hardware(10, Some("usb-serial")),
                target(20),
                vec![
                    device(30, "other", None),
                    node(31, "other.input", Some(30), None, ""),
                    port(32, 31, Direction::Input),
                    link(id, 12, 32),
                ],
            ]
            .concat(),
        );
        let result = reconcile::step(&graph, &saved, state, now + Duration::from_secs(attempt as u64));
        assert_eq!(result.actions, if attempt < 3 { vec![Action::RemoveLink(id)] } else { vec![] });
        assert_eq!(!result.notices.is_empty(), attempt == 3);
        state = result.state;
    }
}

#[test]
fn create_failure_backoff_and_ambiguous_route_no_action() {
    let graph = fixture(1, [hardware(10, Some("usb-serial")), target(20)].concat());
    let saved = setup(&graph, 12, 22);
    let now = Instant::now();
    let mut state = State::default();
    for (delay, previous) in [(1, 0), (5, 1), (30, 6)] {
        let attempted = reconcile::step(&graph, &saved, state, now + Duration::from_secs(previous));
        assert_eq!(attempted.actions, vec![Action::CreateLink(12, 22)]);
        state = attempted.state;
        reconcile::create_failed(&mut state, &saved.routes[0], now + Duration::from_secs(previous));
        assert_eq!(
            reconcile::next_retry(&state, now + Duration::from_secs(previous)),
            Some(Duration::from_secs(delay))
        );
        let waiting =
            reconcile::step(&graph, &saved, state, now + Duration::from_secs(previous) + Duration::from_millis(500));
        assert!(waiting.actions.is_empty());
        state = waiting.state;
    }
    let ambiguous =
        fixture(2, [hardware(10, Some("usb-serial")), hardware(100, Some("usb-serial")), target(20)].concat());
    assert!(reconcile::step(&ambiguous, &saved, State::default(), now).actions.is_empty());
}

#[test]
fn activation_is_one_observed_undo_transaction_and_switch_stops_enforcement() {
    let tmp = Temp::new();
    let paths = Paths::under(tmp.path().into());
    let store = Store { paths: paths.clone() };
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
    let route = |from, to| {
        let fake = fake.borrow();
        let graph = &fake.graph;
        Route {
            output: graph.stable_port_id(&graph.ports[&from]).unwrap(),
            input: graph.stable_port_id(&graph.ports[&to]).unwrap(),
        }
    };
    let a = Setup {
        version: VERSION,
        id: "a".into(),
        name: "A".into(),
        routes: vec![route(100, 104), route(101, 105)],
        keep_connected: true,
        exclusive: false,
    };
    let b = Setup {
        version: VERSION,
        id: "b".into(),
        name: "B".into(),
        routes: vec![route(100, 120)],
        keep_connected: true,
        exclusive: false,
    };
    store.save(&a).unwrap();
    store.save(&b).unwrap();
    let mut h = Harness::new(Musubi::with_fake_setups(fake.clone(), paths));
    h.send(Msg::ActivateSetup("a".into()));
    assert_eq!(h.program().transactions.len(), 1);
    assert_eq!(h.program().transactions[0].intents.len(), 2);
    assert!(h.program().transactions[0].intents.iter().all(|intent| intent.observed));
    assert_eq!(store.active().unwrap().unwrap().id, "a");
    h.send(Msg::ActivateSetup("b".into()));
    assert_eq!(store.active().unwrap().unwrap().id, "b");
    let a_link = *h.program().graph.links_by_pair[&(100, 104)].first().unwrap();
    fake.borrow_mut().remove(a_link);
    loop {
        let event = fake.borrow_mut().next_event();
        let Some(event) = event else { break };
        h.send(Msg::Backend(event));
    }
    assert!(!h.program().graph.links_by_pair.contains_key(&(100, 104)));
    assert!(h.program().graph.links_by_pair.contains_key(&(100, 120)));
    h.send(Msg::DeactivateSetup);
    assert!(store.active().unwrap().is_none());
    assert!(h.program().graph.links_by_pair.contains_key(&(100, 120)));
    assert_eq!(fake.borrow().commands.iter().filter(|cmd| matches!(cmd, Command::Connect { .. })).count(), 3);
    h.send(Msg::Undo);
    assert!(!h.program().graph.links_by_pair.contains_key(&(100, 120)));
    h.send(Msg::Undo);
    assert!(!h.program().graph.links_by_pair.contains_key(&(101, 105)));
}

#[test]
fn damaged_definition_is_listed_and_removed_without_parsing() {
    let tmp = Temp::new();
    let store = Store { paths: Paths::under(tmp.path().into()) };
    store.paths.ensure().unwrap();
    fs::write(store.paths.json("broken").unwrap(), b"not JSON").unwrap();
    let entries = store.list().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "broken");
    assert!(entries[0].1.is_err());
    store.delete("broken").unwrap();
    assert!(store.list().unwrap().is_empty());
}

#[test]
fn symlinked_managed_directory_is_refused() {
    let tmp = Temp::new();
    let other = Temp::new();
    let paths = Paths::under(tmp.path().into());
    fs::create_dir_all(paths.root.parent().unwrap()).unwrap();
    symlink(other.path(), &paths.root).unwrap();
    let store = Store { paths };
    assert!(store.list().is_err());
    assert!(
        store
            .save(&Setup {
                version: VERSION,
                id: "foo".into(),
                name: "Foo".into(),
                routes: vec![],
                keep_connected: false,
                exclusive: false
            })
            .is_err()
    );
    assert!(store.remove_all().is_err());
    assert_eq!(fs::read_dir(other.path()).unwrap().count(), 0);
}

#[test]
fn remove_all_keeps_foreign_files_and_clears_active() {
    let tmp = Temp::new();
    let store = Store { paths: Paths::under(tmp.path().into()) };
    store
        .save(&Setup {
            version: VERSION,
            id: "studio".into(),
            name: "Studio".into(),
            routes: vec![],
            keep_connected: false,
            exclusive: false,
        })
        .unwrap();
    store.set_active(&Active { id: "studio".into() }).unwrap();
    fs::write(store.paths.setups.join("foreign.txt"), "user").unwrap();
    fs::write(store.paths.setups.join(".musubi-1-1.tmp"), "old").unwrap();
    assert_eq!(fs::metadata(store.paths.json("studio").unwrap()).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(fs::metadata(&store.paths.setups).unwrap().permissions().mode() & 0o777, 0o700);
    store.remove_all().unwrap();
    assert!(store.list().unwrap().is_empty());
    assert!(store.active().unwrap().is_none());
    assert_eq!(fs::read_to_string(store.paths.setups.join("foreign.txt")).unwrap(), "user");
}
