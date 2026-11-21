//! Persisted identities intentionally omit ephemeral global IDs and object serials.
use super::{Direction, GlobalId, Graph, PortRecord};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StableId {
    Group {
        device_name: Option<String>,
        device_serial_or_bus: Option<String>,
        client_name: Option<String>,
        node_name: Option<String>,
        sequencer_client_name: Option<String>,
    },
    Port {
        device_name: Option<String>,
        device_serial_or_bus: Option<String>,
        node_name: Option<String>,
        direction: Direction,
        port_name: Option<String>,
        sequencer_client_name: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Unique(GlobalId),
    Ambiguous(Vec<GlobalId>),
    Missing,
}

fn property<'a>(props: &'a super::Properties, name: &str) -> Option<&'a str> {
    props.get(name).map(String::as_str).filter(|value| !value.is_empty())
}

impl Graph {
    fn device_identity(&self, node: &super::NodeRecord) -> (Option<String>, Option<String>) {
        let device = node.device_id.and_then(|id| self.devices.get(&id));
        (
            device.and_then(|v| property(&v.props, "device.name")).map(str::to_owned),
            device
                .and_then(|v| property(&v.props, "device.serial").or_else(|| property(&v.props, "device.bus-path")))
                .map(str::to_owned),
        )
    }

    fn seq_name(port: &PortRecord) -> Option<String> {
        property(&port.props, "api.alsa.seq.client.name")
            .or_else(|| property(&port.props, "api.alsa.seq.client"))
            .map(str::to_owned)
    }

    pub fn stable_port_id(&self, port: &PortRecord) -> Option<StableId> {
        let node = port.node_id.and_then(|id| self.nodes.get(&id))?;
        let (device_name, device_serial_or_bus) = self.device_identity(node);
        Some(StableId::Port {
            device_name,
            device_serial_or_bus,
            node_name: property(&node.props, "node.name").map(str::to_owned),
            direction: port.direction,
            port_name: property(&port.props, "port.name").map(str::to_owned),
            sequencer_client_name: Self::seq_name(port),
        })
    }

    pub fn stable_group_id(&self, node_id: GlobalId, sequencer_client_name: Option<&str>) -> Option<StableId> {
        let node = self.nodes.get(&node_id)?;
        let (device_name, device_serial_or_bus) = self.device_identity(node);
        Some(StableId::Group {
            device_name,
            device_serial_or_bus,
            client_name: node
                .client_id
                .and_then(|id| self.clients.get(&id))
                .and_then(|client| {
                    property(&client.props, "application.name").or_else(|| property(&client.props, "client.name"))
                })
                .map(str::to_owned),
            node_name: property(&node.props, "node.name").map(str::to_owned),
            sequencer_client_name: sequencer_client_name.map(str::to_owned),
        })
    }

    pub fn resolve(&self, target: &StableId) -> Resolution {
        let mut ids: Vec<_> = match target {
            StableId::Port { .. } => self
                .ports
                .values()
                .filter(|port| self.stable_port_id(port).as_ref() == Some(target))
                .map(|port| port.id)
                .collect(),
            StableId::Group { sequencer_client_name, .. } => self
                .nodes
                .values()
                .filter(|node| {
                    if self.stable_group_id(node.id, sequencer_client_name.as_deref()).as_ref() != Some(target) {
                        return false;
                    }
                    sequencer_client_name.as_ref().is_none_or(|name| {
                        self.ports
                            .values()
                            .any(|port| port.node_id == Some(node.id) && Self::seq_name(port).as_ref() == Some(name))
                    })
                })
                .map(|node| node.id)
                .collect(),
        };
        ids.sort_unstable();
        match ids.len() {
            0 => Resolution::Missing,
            1 => Resolution::Unique(ids[0]),
            _ => Resolution::Ambiguous(ids),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphDelta, NodeRecord, PortRecord, Properties, Upsert};
    #[test]
    fn ambiguous_names_never_guess() {
        let mut graph = Graph::default();
        let mut changes = Vec::new();
        for (node, port) in [(1, 2), (3, 4)] {
            changes.push(Upsert::Node(NodeRecord {
                id: node,
                client_id: None,
                device_id: None,
                props: Properties::from([("node.name".into(), "shared".into())]),
                native: Default::default(),
            }));
            changes.push(Upsert::Port(PortRecord {
                id: port,
                node_id: Some(node),
                direction: Direction::Output,
                enum_formats: vec![],
                props: Properties::from([("port.name".into(), "out".into())]),
            }));
        }
        graph.apply(GraphDelta { epoch: 1, upserts: changes, removals: vec![] });
        let id = graph.stable_port_id(&graph.ports[&2]).unwrap();
        assert_eq!(graph.resolve(&id), Resolution::Ambiguous(vec![2, 4]));
        graph.apply(GraphDelta { epoch: 1, upserts: vec![], removals: vec![3] });
        assert_eq!(graph.resolve(&id), Resolution::Unique(2));
        graph.apply(GraphDelta { epoch: 2, upserts: vec![], removals: vec![] });
        assert_eq!(graph.resolve(&id), Resolution::Missing);
    }
}
