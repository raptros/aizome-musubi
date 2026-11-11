//! Display groups derived from live ownership, never persisted registry IDs.
use std::collections::HashMap;

use super::{classify::channel_sort_key, Direction, GlobalId, Graph, PortRecord, Properties};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GroupKey {
    MidiClient { bridge: GlobalId, client: String },
    Device(GlobalId),
    Client(GlobalId),
    Node(GlobalId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind { Hardware, Application, Effect, Virtual }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub key: GroupKey,
    pub kind: GroupKind,
    pub label: String,
    pub ports: Vec<GlobalId>,
}

fn prop<'a>(props: &'a Properties, key: &str) -> Option<&'a str> {
    props.get(key).map(String::as_str).filter(|v| !v.is_empty())
}

fn first<'a>(props: &'a Properties, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| prop(props, key))
}

pub fn node_label(graph: &Graph, node: GlobalId) -> String {
    graph.nodes.get(&node).and_then(|n| first(&n.props, &["node.description", "node.nick", "node.name", "media.name"]))
        .map(str::to_owned).unwrap_or_else(|| format!("Node {node}"))
}

pub fn port_label(port: &PortRecord) -> String {
    first(&port.props, &["audio.channel"]).or_else(|| {
        prop(&port.props, "port.alias").and_then(|alias| alias.rsplit(':').next()).filter(|tail| !tail.is_empty())
    }).or_else(|| prop(&port.props, "port.name"))
        .map(str::to_owned).unwrap_or_else(|| format!("Port {}", port.id))
}

fn bridge(node: &super::NodeRecord) -> bool {
    let class = prop(&node.props, "media.class").unwrap_or("");
    let name = prop(&node.props, "node.name").unwrap_or("");
    matches!(class, "Midi/Bridge" | "Midi-Bridge") || name.contains("Midi-Bridge")
}

fn seq_client(port: &PortRecord) -> Option<String> {
    let group = prop(&port.props, "port.group").and_then(|v| v.strip_prefix("client_"));
    group.or_else(|| first(&port.props, &["api.alsa.seq.client.id", "api.alsa.seq.client", "api.alsa.seq.client-id"]))
        .map(str::to_owned)
}

fn seq_label(port: &PortRecord) -> Option<&str> {
    first(&port.props, &["api.alsa.seq.client.name", "api.alsa.seq.client-name"])
        .or_else(|| prop(&port.props, "port.alias").and_then(|v| v.split_once(':').map(|(client, _)| client)).filter(|v| !v.is_empty()))
}

fn port_order_key(p: &PortRecord) -> (GlobalId, u8, &str, (u8, u32, &str), u32, &str, GlobalId) {
    (p.node_id.unwrap_or(0),
     match p.direction { Direction::Output => 0, Direction::Input => 1, Direction::Other => 2 },
     prop(&p.props, "port.group").unwrap_or(""),
     channel_sort_key(prop(&p.props, "audio.channel")),
     prop(&p.props, "port.id").and_then(|v| v.parse::<u32>().ok()).unwrap_or(u32::MAX),
     prop(&p.props, "port.name").unwrap_or(""), p.id)
}

impl Graph {
    pub fn group_key(&self, port: &PortRecord) -> Option<GroupKey> {
        let node = self.nodes.get(&port.node_id?)?;
        if bridge(node) {
            return Some(GroupKey::MidiClient { bridge: node.id, client: seq_client(port).unwrap_or_default() });
        }
        if let Some(id) = node.device_id.filter(|id| self.devices.contains_key(id)) { return Some(GroupKey::Device(id)); }
        if let Some(id) = node.client_id.filter(|id| self.clients.contains_key(id)) { return Some(GroupKey::Client(id)); }
        Some(GroupKey::Node(node.id))
    }

    pub fn groups(&self, direction: Direction) -> Vec<Group> {
        let mut groups: HashMap<GroupKey, Group> = HashMap::new();
        for port in self.ports.values().filter(|port| port.direction == direction) {
            let Some(key) = self.group_key(port) else { continue };
            let node = &self.nodes[&port.node_id.expect("attached port")];
            let (kind, label) = match &key {
                GroupKey::MidiClient { .. } => {
                    let label = if seq_label(port).is_some_and(|name| name.contains("Midi Through")) {
                        "Midi Through".to_owned()
                    } else { seq_label(port).unwrap_or("ALSA MIDI").to_owned() };
                    (if label == "Midi Through" { GroupKind::Virtual } else { GroupKind::Hardware }, label)
                }
                GroupKey::Device(id) => {
                    let device = &self.devices[id];
                    let label = first(&device.props, &["device.description", "device.nick", "device.name"])
                        .map(str::to_owned).unwrap_or_else(|| node_label(self, node.id));
                    (GroupKind::Hardware, label)
                }
                GroupKey::Client(id) => {
                    let client = &self.clients[id];
                    let label = prop(&client.props, "application.name").map(str::to_owned)
                        .unwrap_or_else(|| node_label(self, node.id));
                    (node_kind(node), label)
                }
                GroupKey::Node(_) => (node_kind(node), node_label(self, node.id)),
            };
            groups.entry(key.clone()).or_insert_with(|| Group { key, kind, label, ports: Vec::new() }).ports.push(port.id);
        }
        let mut groups: Vec<_> = groups.into_values().collect();
        // Duplicate human device descriptions must not produce indistinguishable boxes.
        let labels: Vec<_> = groups.iter().map(|g| g.label.clone()).collect();
        for group in &mut groups {
            if !matches!(group.key, GroupKey::Device(_)) || labels.iter().filter(|label| *label == &group.label).count() < 2 { continue; }
            let GroupKey::Device(id) = &group.key else { continue };
            let device = &self.devices[&id];
            if let Some(suffix) = first(&device.props, &["device.serial", "device.bus-path", "device.name"]) {
                group.label = format!("{} ({suffix})", group.label);
            } else { group.label = format!("{} ({id})", group.label); }
        }
        for group in &mut groups {
            group.ports.sort_by(|&a, &b| {
                let a = &self.ports[&a]; let b = &self.ports[&b];
                port_order_key(a).cmp(&port_order_key(b))
            });
        }
        groups.sort_by(|a, b| {
            let rank = |kind| match kind { GroupKind::Hardware => 0, GroupKind::Effect => 1, GroupKind::Virtual => 2, GroupKind::Application => 3 };
            rank(a.kind).cmp(&rank(b.kind)).then_with(|| a.label.cmp(&b.label))
                .then_with(|| a.ports.first().cmp(&b.ports.first()))
        });
        groups
    }
}

fn node_kind(node: &super::NodeRecord) -> GroupKind {
    let props = &node.props;
    let name = prop(props, "factory.name").unwrap_or("");
    if name.contains("filter-chain") || prop(props, "node.virtual") == Some("true") { GroupKind::Effect }
    else if name.contains("loopback") || prop(props, "node.name").is_some_and(|n| n.contains("loopback")) { GroupKind::Virtual }
    else { GroupKind::Application }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::dump;
    fn graph(json: &str) -> Graph { let mut g = Graph::default(); g.apply(dump::parse(json, 1).unwrap()); g }
    #[test]
    fn studio_groups_and_labels() {
        let g = graph(include_str!("../../tests/fixtures/studio.json"));
        let sources = g.groups(Direction::Output);
        assert!(matches!(sources[0].kind, GroupKind::Hardware));
        assert!(sources.iter().any(|g| g.label == "USB Studio" && g.kind == GroupKind::Hardware));
        assert!(sources.iter().any(|g| g.label == "EQ" && g.kind == GroupKind::Effect));
        assert!(sources.iter().any(|g| g.label == "Firefox" && g.kind == GroupKind::Application));
        assert!(sources.iter().any(|g| g.label == "Bitwig" && g.ports.len() == 6));
        assert_eq!(port_label(&g.ports[&100]), "AUX0");
        assert_eq!(sources.iter().find(|group| group.label == "USB Studio").unwrap().ports, vec![100,101,102,103]);
    }
    #[test]
    fn bridge_splits_with_both_identifier_sources() {
        let g = graph(include_str!("../../tests/fixtures/midi.json"));
        let sources = g.groups(Direction::Output);
        assert!(sources.iter().any(|g| g.label == "Launchkey" && g.key == (GroupKey::MidiClient { bridge: 10, client: "24".into() })));
        assert!(sources.iter().any(|g| g.label == "Keystep" && g.key == (GroupKey::MidiClient { bridge: 10, client: "28".into() })));
        assert!(sources.iter().any(|g| g.label == "Midi Through" && g.kind == GroupKind::Virtual));
        assert!(sources.iter().any(|g| g.label == "ALSA MIDI"));
        assert!(sources.iter().any(|g| g.label == "Ardour"));
    }
    #[test]
    fn fallbacks_empty_and_numeric_channel_order() {
        let mut g = graph(include_str!("../../tests/fixtures/studio.json"));
        assert!(graph(include_str!("../../tests/fixtures/empty.json")).groups(Direction::Output).is_empty());
        assert_eq!(g.groups(Direction::Input).iter().find(|group| group.label == "Bitwig").unwrap().ports.len(), 8);
        let mut extra = g.ports[&100].clone();
        extra.id = 500;
        extra.props.insert("audio.channel".into(), "AUX10".into());
        extra.props.insert("port.id".into(), "10".into());
        g.apply(crate::graph::GraphDelta { epoch: 1, upserts: vec![crate::graph::Upsert::Port(extra)], removals: vec![] });
        let mut nine = g.ports[&100].clone();
        nine.id = 501;
        nine.props.insert("audio.channel".into(), "AUX9".into());
        nine.props.insert("port.id".into(), "9".into());
        g.apply(crate::graph::GraphDelta { epoch: 1, upserts: vec![crate::graph::Upsert::Port(nine)], removals: vec![] });
        assert_eq!(g.groups(Direction::Output).iter().find(|group| group.label == "USB Studio").unwrap().ports, vec![100,101,102,103,501,500]);
        g.devices.get_mut(&1).unwrap().props.remove("device.description");
        assert_eq!(g.groups(Direction::Output).iter().find(|group| group.key == GroupKey::Device(1)).unwrap().label, "alsa_card.usb.studio");
        g.devices.get_mut(&1).unwrap().props.insert("device.nick".into(), "Studio Nick".into());
        assert_eq!(g.groups(Direction::Output).iter().find(|group| group.key == GroupKey::Device(1)).unwrap().label, "Studio Nick");
        let node = g.nodes.get_mut(&12).unwrap();
        for name in ["node.description", "node.nick", "node.name"] { node.props.remove(name); }
        node.props.insert("media.name".into(), "Capture Media".into());
        assert_eq!(node_label(&g, 12), "Capture Media");
        g.nodes.get_mut(&12).unwrap().props.remove("media.name");
        assert_eq!(node_label(&g, 12), "Node 12");
        let port = g.ports.get_mut(&100).unwrap();
        port.props.remove("audio.channel");
        assert_eq!(port_label(port), "capture_AUX0");
        port.props.remove("port.alias");
        assert_eq!(port_label(port), "capture_AUX0");
        port.props.remove("port.name");
        assert_eq!(port_label(port), "Port 100");
    }
}
