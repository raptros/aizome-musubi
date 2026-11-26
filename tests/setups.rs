use aizome_musubi::{
    graph::{
        ClientRecord, DeviceRecord, Direction, Graph, GraphDelta, LinkRecord, LinkState, NativeProps, NodeRecord,
        PortRecord, Properties, Upsert, identity::Resolution,
    },
    setups::{Active, End, Route, RouteStatus, Setup, VERSION, paths::Paths, plan, store::Store},
};
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
