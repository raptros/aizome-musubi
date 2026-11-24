//! Plain-data PipeWire graph. Runtime IDs are valid only within `epoch`.

pub mod classify;
pub mod dump;
pub mod group;
pub mod identity;
pub mod legal;

use std::collections::{HashMap, HashSet};

pub type GlobalId = u32;
pub type Properties = HashMap<String, String>;
/// The meter's capture node is internal, including all its ports and policy links.
pub fn is_musubi_meter(props: &Properties) -> bool {
    props.get("application.id").is_some_and(|id| id == "org.aizome.Musubi")
        && props.get("musubi.meter").is_some_and(|marker| marker == "true")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Input,
    Output,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientRecord {
    pub id: GlobalId,
    pub props: Properties,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceRecord {
    pub id: GlobalId,
    pub props: Properties,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NodeRecord {
    pub id: GlobalId,
    pub device_id: Option<GlobalId>,
    pub client_id: Option<GlobalId>,
    pub props: Properties,
    pub native: NativeProps,
}
/// Observed node parameter, never a pending/requested value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeProps {
    pub channel_volumes: Option<Vec<f32>>,
    pub muted: Option<bool>,
    pub effect_params: Vec<(String, f32)>,
    pub writable: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortRecord {
    pub id: GlobalId,
    pub node_id: Option<GlobalId>,
    pub direction: Direction,
    /// Summary of the port's supported EnumFormat alternatives, not the node's format.
    pub enum_formats: Vec<classify::PortKind>,
    pub props: Properties,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkState {
    Active,
    Negotiating,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkRecord {
    pub id: GlobalId,
    pub output: GlobalId,
    pub input: GlobalId,
    pub state: LinkState,
    pub props: Properties,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataRecord {
    pub id: GlobalId,
    pub props: Properties,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Upsert {
    Client(ClientRecord),
    Device(DeviceRecord),
    Node(NodeRecord),
    Port(PortRecord),
    Link(LinkRecord),
    Metadata(MetadataRecord),
}
impl Upsert {
    pub fn id(&self) -> GlobalId {
        match self {
            Self::Client(v) => v.id,
            Self::Device(v) => v.id,
            Self::Node(v) => v.id,
            Self::Port(v) => v.id,
            Self::Link(v) => v.id,
            Self::Metadata(v) => v.id,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GraphDelta {
    pub epoch: u64,
    pub upserts: Vec<Upsert>,
    pub removals: Vec<GlobalId>,
}

#[derive(Default, Debug)]
pub struct Graph {
    pub epoch: u64,
    pub clients: HashMap<GlobalId, ClientRecord>,
    pub devices: HashMap<GlobalId, DeviceRecord>,
    pub nodes: HashMap<GlobalId, NodeRecord>,
    pub ports: HashMap<GlobalId, PortRecord>,
    pub links: HashMap<GlobalId, LinkRecord>,
    pub metadata: HashMap<GlobalId, MetadataRecord>,
    pub links_by_pair: HashMap<(GlobalId, GlobalId), Vec<GlobalId>>,
    pub ports_by_kind: HashMap<classify::PortKind, HashSet<GlobalId>>,
    pending_ports: HashMap<GlobalId, PortRecord>,
    pending_links: HashMap<GlobalId, LinkRecord>,
    hidden_nodes: HashSet<GlobalId>,
    hidden_ports: HashSet<GlobalId>,
}

impl Graph {
    pub fn apply(&mut self, delta: GraphDelta) {
        if delta.epoch < self.epoch {
            return;
        }
        if delta.epoch != self.epoch {
            *self = Self { epoch: delta.epoch, ..Self::default() };
        }
        for id in delta.removals {
            self.remove(id);
            self.hidden_nodes.remove(&id);
            self.hidden_ports.remove(&id);
        }
        for upsert in delta.upserts {
            match &upsert {
                Upsert::Node(node) if is_musubi_meter(&node.props) => {
                    self.remove(node.id);
                    self.hidden_nodes.insert(node.id);
                }
                Upsert::Port(port) if port.node_id.is_some_and(|id| self.hidden_nodes.contains(&id)) => {
                    self.remove(port.id);
                    self.hidden_ports.insert(port.id);
                }
                Upsert::Link(link)
                    if self.hidden_ports.contains(&link.input) || self.hidden_ports.contains(&link.output) => {}
                _ => self.upsert(upsert),
            }
        }
        // Port and Link globals can precede the meter Node/Port globals in a batch.
        let hidden_ports: Vec<_> = self
            .ports
            .values()
            .chain(self.pending_ports.values())
            .filter(|port| port.node_id.is_some_and(|id| self.hidden_nodes.contains(&id)))
            .map(|port| port.id)
            .collect();
        for id in hidden_ports {
            self.remove(id);
            self.hidden_ports.insert(id);
        }
        let hidden_links: Vec<_> = self
            .links
            .values()
            .chain(self.pending_links.values())
            .filter(|link| self.hidden_ports.contains(&link.input) || self.hidden_ports.contains(&link.output))
            .map(|link| link.id)
            .collect();
        for id in hidden_links {
            self.remove(id);
        }
        self.attach_pending();
    }

    pub fn has_active_link(&self, pair: (GlobalId, GlobalId)) -> bool {
        self.links_by_pair.get(&pair).is_some_and(|ids| {
            ids.iter().any(|id| self.links.get(id).is_some_and(|link| link.state == LinkState::Active))
        })
    }

    pub fn kind(&self, port: GlobalId) -> Option<classify::PortKind> {
        let port = self.ports.get(&port)?;
        Some(classify::classify(port, port.node_id.and_then(|id| self.nodes.get(&id))))
    }

    pub fn ports_of_kind(&self, kind: classify::PortKind) -> impl Iterator<Item = GlobalId> + '_ {
        self.ports_by_kind.get(&kind).into_iter().flat_map(|ids| ids.iter().copied())
    }

    fn upsert(&mut self, value: Upsert) {
        let id = value.id();
        match value {
            Upsert::Client(v) => {
                self.clients.insert(id, v);
            }
            Upsert::Device(v) => {
                self.devices.insert(id, v);
            }
            // Info updates must not cascade-delete existing children.
            Upsert::Node(v) => {
                self.nodes.insert(id, v);
            }
            Upsert::Port(v) => {
                self.ports.remove(&id);
                self.pending_ports.insert(id, v);
            }
            Upsert::Link(v) => {
                if let Some(previous) = self.links.remove(&id) {
                    let pair = (previous.output, previous.input);
                    if let Some(ids) = self.links_by_pair.get_mut(&pair) {
                        ids.retain(|&candidate| candidate != id);
                        if ids.is_empty() {
                            self.links_by_pair.remove(&pair);
                        }
                    }
                }
                self.pending_links.insert(id, v);
            }
            Upsert::Metadata(v) => {
                self.metadata.insert(id, v);
            }
        }
    }

    fn attach_pending(&mut self) {
        // Reclassification is needed if node properties arrive after a port's info.
        self.ports_by_kind.clear();
        let ready: Vec<_> = self
            .pending_ports
            .iter()
            .filter_map(|(&id, port)| port.node_id.filter(|parent| self.nodes.contains_key(parent)).map(|_| id))
            .collect();
        for id in ready {
            let port = self.pending_ports.remove(&id).expect("pending port");
            self.ports.insert(id, port);
        }
        for (&id, port) in &self.ports {
            let kind = classify::classify(port, port.node_id.and_then(|node| self.nodes.get(&node)));
            self.ports_by_kind.entry(kind).or_default().insert(id);
        }
        let ready: Vec<_> = self
            .pending_links
            .iter()
            .filter_map(|(&id, link)| {
                (self.ports.contains_key(&link.output) && self.ports.contains_key(&link.input)).then_some(id)
            })
            .collect();
        for id in ready {
            let link = self.pending_links.remove(&id).expect("pending link");
            self.links_by_pair.entry((link.output, link.input)).or_default().push(id);
            self.links.insert(id, link);
        }
    }

    fn remove(&mut self, id: GlobalId) {
        if let Some(link) = self.links.remove(&id) {
            let pair = (link.output, link.input);
            if let Some(ids) = self.links_by_pair.get_mut(&pair) {
                ids.retain(|&candidate| candidate != id);
                if ids.is_empty() {
                    self.links_by_pair.remove(&pair);
                }
            }
        }
        self.pending_links.remove(&id);
        if self.ports.remove(&id).is_some() || self.pending_ports.remove(&id).is_some() {
            let dependent: Vec<_> = self
                .links
                .values()
                .chain(self.pending_links.values())
                .filter(|link| link.input == id || link.output == id)
                .map(|link| link.id)
                .collect();
            for link in dependent {
                self.remove(link);
            }
        }
        if self.nodes.remove(&id).is_some() {
            let dependent: Vec<_> = self
                .ports
                .values()
                .chain(self.pending_ports.values())
                .filter(|port| port.node_id == Some(id))
                .map(|port| port.id)
                .collect();
            for port in dependent {
                self.remove(port);
            }
        }
        self.clients.remove(&id);
        self.devices.remove(&id);
        self.metadata.remove(&id);
        for ids in self.ports_by_kind.values_mut() {
            ids.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn props(entries: &[(&str, &str)]) -> Properties {
        entries.iter().map(|&(k, v)| (k.into(), v.into())).collect()
    }
    fn node(id: u32) -> Upsert {
        Upsert::Node(NodeRecord {
            id,
            client_id: None,
            device_id: None,
            props: props(&[("media.class", "Audio/Sink")]),
            native: Default::default(),
        })
    }
    fn port(id: u32, node_id: u32) -> Upsert {
        Upsert::Port(PortRecord {
            id,
            node_id: Some(node_id),
            direction: Direction::Input,
            enum_formats: vec![],
            props: Properties::new(),
        })
    }
    fn link(id: u32, output: u32, input: u32) -> Upsert {
        Upsert::Link(LinkRecord { id, output, input, state: LinkState::Active, props: Properties::new() })
    }
    fn delta(epoch: u64, upserts: Vec<Upsert>, removals: Vec<u32>) -> GraphDelta {
        GraphDelta { epoch, upserts, removals }
    }

    #[test]
    fn out_of_order_and_sparse_duplicate_links() {
        let mut graph = Graph::default();
        graph.apply(delta(1, vec![link(8, 2, 3), link(9, 2, 3), port(2, 1), port(3, 1)], vec![]));
        assert!(graph.ports.is_empty());
        assert!(graph.links.is_empty());
        graph.apply(delta(1, vec![node(1)], vec![]));
        assert_eq!(graph.links_by_pair[&(2, 3)].len(), 2);
        assert_eq!(graph.ports_of_kind(classify::PortKind::Audio).count(), 2);
        graph.apply(delta(1, vec![], vec![8]));
        assert_eq!(graph.links_by_pair[&(2, 3)], vec![9]);
    }
    #[test]
    fn cascade_and_epoch_discard() {
        let mut graph = Graph::default();
        graph.apply(delta(1, vec![node(1), port(2, 1), port(3, 1), link(4, 2, 3)], vec![]));
        graph.apply(delta(1, vec![], vec![1]));
        assert!(graph.ports.is_empty() && graph.links.is_empty() && graph.links_by_pair.is_empty());
        graph.apply(delta(2, vec![node(5)], vec![]));
        graph.apply(delta(1, vec![node(1)], vec![]));
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.nodes.contains_key(&5));
    }
    #[test]
    fn node_update_reclassifies_port() {
        let mut graph = Graph::default();
        graph.apply(delta(1, vec![node(1), port(2, 1)], vec![]));
        graph.apply(delta(
            1,
            vec![Upsert::Node(NodeRecord {
                id: 1,
                client_id: None,
                device_id: None,
                props: props(&[("media.class", "Video/Source")]),
                native: Default::default(),
            })],
            vec![],
        ));
        assert_eq!(graph.kind(2), Some(classify::PortKind::Video));
    }
    #[test]
    fn own_meter_node_ports_and_policy_links_are_never_visible() {
        let mut graph = Graph::default();
        let Upsert::Node(mut meter) = node(10) else { unreachable!() };
        meter.props.extend(props(&[("application.id", "org.aizome.Musubi"), ("musubi.meter", "true")]));
        graph.apply(delta(1, vec![node(1), port(2, 1), link(12, 2, 11), port(11, 10), Upsert::Node(meter)], vec![]));
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.ports.contains_key(&2));
        assert!(!graph.ports.contains_key(&11));
        assert!(graph.links.is_empty());
        graph.apply(delta(1, vec![link(13, 2, 11), port(14, 10)], vec![]));
        assert!(graph.links.is_empty() && !graph.ports.contains_key(&14));
        graph.apply(delta(2, vec![node(10), port(11, 10)], vec![]));
        assert!(graph.nodes.contains_key(&10) && graph.ports.contains_key(&11));
    }
}
