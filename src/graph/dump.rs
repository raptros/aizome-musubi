//! Import `pw-dump` snapshots as the same deltas consumed by a live backend.
use serde_json::Value;
use super::{ClientRecord, DeviceRecord, Direction, GlobalId, GraphDelta, LinkRecord, MetadataRecord, NodeRecord, PortRecord, Properties, Upsert};
use super::classify::{PortKind, from_enum_format};

fn property_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}
fn properties(value: &Value) -> Properties {
    let mut result = Properties::new();
    // Registry properties can precede the info callback; info values take precedence.
    for source in [&value["props"], &value["info"]["props"]] {
        if let Some(map) = source.as_object() {
            for (key, value) in map {
                if let Some(s) = property_value(value) { result.insert(key.clone(), s); }
            }
        }
    }
    result
}
fn id(props: &Properties, key: &str) -> Option<GlobalId> { props.get(key)?.parse().ok() }
fn format_kind(value: &Value) -> PortKind {
    let media = value["mediaType"].as_str().unwrap_or("");
    let subtype = value["mediaSubtype"].as_str().unwrap_or("");
    let kind = from_enum_format(media, subtype);
    if kind != PortKind::Other { return kind; }
    if media.eq_ignore_ascii_case("application") && subtype.eq_ignore_ascii_case("control") {
        // SPA application/control is shared by MIDI, UMP and unrelated control formats.
        let format = value["format"].as_str().unwrap_or("");
        return match format { "Midi" | "MIDI" => PortKind::Midi, "UMP" => PortKind::Ump, _ => PortKind::Other };
    }
    PortKind::Other
}

pub fn parse(json: &str, epoch: u64) -> Result<GraphDelta, serde_json::Error> {
    let values: Vec<Value> = serde_json::from_str(json)?;
    let mut upserts = Vec::new();
    for value in &values {
        let Some(global_id) = value["id"].as_u64().and_then(|id| GlobalId::try_from(id).ok()) else { continue; };
        let props = properties(value);
        let record = match value["type"].as_str().unwrap_or("") {
            "PipeWire:Interface:Client" => Upsert::Client(ClientRecord { id: global_id, props }),
            "PipeWire:Interface:Device" => Upsert::Device(DeviceRecord { id: global_id, props }),
            "PipeWire:Interface:Node" => Upsert::Node(NodeRecord { id: global_id, device_id: id(&props, "device.id"), client_id: id(&props, "client.id"), props }),
            "PipeWire:Interface:Port" => {
                let direction = match props.get("port.direction").map(String::as_str).or_else(|| value["info"]["direction"].as_str()) {
                    Some("in" | "input" | "Input") => Direction::Input,
                    Some("out" | "output" | "Output") => Direction::Output,
                    _ => Direction::Other,
                };
                let enum_formats = value["info"]["params"]["EnumFormat"].as_array().map(|formats| formats.iter().map(format_kind).collect()).unwrap_or_default();
                Upsert::Port(PortRecord { id: global_id, node_id: id(&props, "node.id"), direction, enum_formats, props })
            }
            "PipeWire:Interface:Link" => {
                let output = id(&props, "link.output.port").or_else(|| value["info"]["output-port-id"].as_u64().and_then(|n| GlobalId::try_from(n).ok()));
                let input = id(&props, "link.input.port").or_else(|| value["info"]["input-port-id"].as_u64().and_then(|n| GlobalId::try_from(n).ok()));
                let (Some(output), Some(input)) = (output, input) else { continue; };
                Upsert::Link(LinkRecord { id: global_id, output, input, props })
            }
            "PipeWire:Interface:Metadata" => Upsert::Metadata(MetadataRecord { id: global_id, props }),
            _ => continue,
        };
        upserts.push(record);
    }
    Ok(GraphDelta { epoch, upserts, removals: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, classify::PortKind};
    #[test]
    fn private_dump_has_all_real_ports_including_video_without_dsp() {
        let delta = parse(include_str!("../../tests/fixtures/private.json"), 7).unwrap();
        let mut graph = Graph::default(); graph.apply(delta);
        assert_eq!(graph.epoch, 7);
        assert_eq!(graph.nodes.len(), 11);
        assert_eq!(graph.ports.len(), 25);
        assert_eq!(graph.nodes.values().filter(|node| node.props.get("factory.name").is_some_and(|name| name == "support.null-audio-sink")).count(), 4);
        assert_eq!(graph.nodes.values().filter(|node| node.props.get("media.class").is_some_and(|name| name == "Stream/Output/Audio" || name == "Stream/Input/Audio")).count(), 2);
        assert_eq!(graph.ports_of_kind(PortKind::Audio).count(), 22);
        assert_eq!(graph.ports_of_kind(PortKind::Midi).count(), 2);
        assert_eq!(graph.ports_of_kind(PortKind::Video).count(), 1);
        assert!(!graph.ports[&70].props.contains_key("format.dsp"));
        assert_eq!(graph.kind(70), Some(PortKind::Video));
        assert!(graph.ports[&70].enum_formats.contains(&PortKind::Video));
    }
}
