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
    port.props.get("video.format.family").or_else(|| port.props.get("format.dsp")).map(String::as_str)
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
}
