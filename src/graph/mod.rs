//! Plain-data PipeWire graph. Runtime IDs are valid only within `epoch`.

pub mod classify;
pub mod dump;
pub mod group;
pub mod identity;
pub mod legal;

use std::collections::{HashMap, HashSet};

pub type GlobalId = u32;
pub type Properties = HashMap<String, String>;

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeRecord {
    pub id: GlobalId,
    pub device_id: Option<GlobalId>,
    pub client_id: Option<GlobalId>,
    pub props: Properties,
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
pub struct LinkRecord {
    pub id: GlobalId,
    pub output: GlobalId,
    pub input: GlobalId,
    pub props: Properties,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataRecord {
    pub id: GlobalId,
    pub props: Properties,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
        }
        for upsert in delta.upserts {
            self.upsert(upsert);
        }
        self.attach_pending();
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
        Upsert::Link(LinkRecord { id, output, input, props: Properties::new() })
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
            })],
            vec![],
        ));
        assert_eq!(graph.kind(2), Some(classify::PortKind::Video));
    }
}
