//! Preflight legality; the server still determines whether a request succeeds.
use super::{Direction, GlobalId, Graph, classify::PortKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    MissingPort,
    WrongDirection,
    UnknownKind,
    IncompatibleKinds,
    MidiConversion,
    IncompatibleVideoFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkLegality {
    Allowed,
    NeedsConfirmation(Reason),
    Forbidden(Reason),
}

fn video_family(port: &super::PortRecord) -> Option<&str> {
    port.props
        .get("video.format.family")
        .or_else(|| port.props.get("format.dsp"))
        .or_else(|| port.props.get("video.enum.family"))
        .map(String::as_str)
}

pub fn can_link(graph: &Graph, out: GlobalId, input: GlobalId) -> LinkLegality {
    use LinkLegality::{Allowed, Forbidden, NeedsConfirmation};
    use PortKind::{Audio, Midi, Ump, Video};
    let (Some(source), Some(target)) = (graph.ports.get(&out), graph.ports.get(&input)) else {
        return Forbidden(Reason::MissingPort);
    };
    if source.direction != Direction::Output || target.direction != Direction::Input {
        return Forbidden(Reason::WrongDirection);
    }
    let (Some(a), Some(b)) = (graph.kind(out), graph.kind(input)) else { return Forbidden(Reason::MissingPort) };
    if a == PortKind::Other || b == PortKind::Other {
        return Forbidden(Reason::UnknownKind);
    }
    match (a, b) {
        (Audio, Audio) | (Midi, Midi) | (Ump, Ump) => Allowed,
        (Ump, Midi) | (Midi, Ump) => NeedsConfirmation(Reason::MidiConversion),
        (Video, Video) => {
            // A video class by itself is not evidence of a compatible pixel family.
            match (video_family(source), video_family(target)) {
                (Some(a), Some(b)) if a == b => Allowed,
                _ => Forbidden(Reason::IncompatibleVideoFormat),
            }
        }
        _ => Forbidden(Reason::IncompatibleKinds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::dump;
    #[test]
    fn legality_matrix() {
        let mut g = Graph::default();
        g.apply(dump::parse(include_str!("../../tests/fixtures/studio.json"), 1).unwrap());
        assert_eq!(can_link(&g, 100, 104), LinkLegality::Allowed);
        assert_eq!(can_link(&g, 100, 101), LinkLegality::Forbidden(Reason::WrongDirection));
        use crate::graph::{GraphDelta, NodeRecord, PortRecord, Properties, Upsert};
        let midi = PortRecord {
            id: 500,
            node_id: Some(10),
            direction: Direction::Input,
            enum_formats: vec![],
            props: Properties::from([("format.dsp".into(), "8 bit raw midi".into())]),
        };
        let unknown = PortRecord {
            id: 501,
            node_id: Some(900),
            direction: Direction::Input,
            enum_formats: vec![],
            props: Properties::new(),
        };
        g.apply(GraphDelta {
            epoch: 1,
            upserts: vec![
                Upsert::Node(NodeRecord { id: 900, device_id: None, client_id: None, props: Properties::new() }),
                Upsert::Port(midi),
                Upsert::Port(unknown),
            ],
            removals: vec![],
        });
        assert_eq!(can_link(&g, 100, 500), LinkLegality::Forbidden(Reason::IncompatibleKinds));
        assert_eq!(can_link(&g, 100, 501), LinkLegality::Forbidden(Reason::UnknownKind));
        assert_eq!(can_link(&g, 100, 9999), LinkLegality::Forbidden(Reason::MissingPort));
        g.apply(dump::parse(include_str!("../../tests/fixtures/midi.json"), 2).unwrap());
        assert_eq!(can_link(&g, 20, 31), LinkLegality::Allowed);
        assert_eq!(can_link(&g, 20, 41), LinkLegality::NeedsConfirmation(Reason::MidiConversion));
        assert_eq!(can_link(&g, 20, 21), LinkLegality::Forbidden(Reason::WrongDirection));
        g.apply(dump::parse(include_str!("../../tests/fixtures/video.json"), 3).unwrap());
        assert_eq!(can_link(&g, 20, 21), LinkLegality::Allowed);
    }
    #[test]
    fn raw_enumformat_allows_two_real_shape_video_ports_without_dsp() {
        use crate::{
            app::Page,
            graph::{GraphDelta, Upsert},
            matrix::layout::{CellState, Layout, ViewOptions},
        };
        let mut graph = Graph::default();
        graph.apply(dump::parse(include_str!("../../tests/fixtures/private.json"), 1).unwrap());
        let mut node = graph.nodes[&69].clone();
        node.id = 900;
        node.props.insert("node.name".into(), "video-sink-test".into());
        node.props.insert("media.class".into(), "Stream/Input/Video".into());
        let mut port = graph.ports[&70].clone();
        assert!(!port.props.contains_key("format.dsp"));
        assert_eq!(port.props.get("video.enum.family").map(String::as_str), Some("raw"));
        port.id = 901;
        port.node_id = Some(node.id);
        port.direction = Direction::Input;
        port.props.insert("port.direction".into(), "in".into());
        port.props.insert("port.name".into(), "input_1".into());
        graph.apply(GraphDelta { epoch: 1, upserts: vec![Upsert::Node(node), Upsert::Port(port)], removals: vec![] });
        assert_eq!(can_link(&graph, 70, 901), LinkLegality::Allowed);
        let layout = Layout::from_graph(&graph, Page::Video, &ViewOptions::default());
        let cell = layout.cell_for_pair((70, 901)).expect("visible video cell");
        assert_eq!(layout.state(cell), CellState::Empty);
    }
}
