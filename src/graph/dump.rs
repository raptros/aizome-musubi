//! Import `pw-dump` snapshots as the same deltas consumed by a live backend.
use super::classify::{PortKind, from_enum_format};
use super::{
    ClientRecord, DeviceRecord, Direction, GlobalId, GraphDelta, LinkRecord, LinkState, MetadataRecord, NativeProps,
    NodeRecord, PortRecord, Properties, Upsert,
};
use serde_json::Value;

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
                if let Some(s) = property_value(value) {
                    result.insert(key.clone(), s);
                }
            }
        }
    }
    result
}
fn id(props: &Properties, key: &str) -> Option<GlobalId> {
    props.get(key)?.parse().ok()
}
fn format_kind(value: &Value) -> PortKind {
    let media = value["mediaType"].as_str().unwrap_or("");
    let subtype = value["mediaSubtype"].as_str().unwrap_or("");
    let kind = from_enum_format(media, subtype);
    if kind != PortKind::Other {
        return kind;
    }
    if media.eq_ignore_ascii_case("application") && subtype.eq_ignore_ascii_case("control") {
        // SPA application/control is shared by MIDI, UMP and unrelated control formats.
        let format = value["format"].as_str().unwrap_or("");
        return match format {
            "Midi" | "MIDI" => PortKind::Midi,
            "UMP" => PortKind::Ump,
            _ => PortKind::Other,
        };
    }
    PortKind::Other
}

pub fn parse(json: &str, epoch: u64) -> Result<GraphDelta, serde_json::Error> {
    let values: Vec<Value> = serde_json::from_str(json)?;
    let mut upserts = Vec::new();
    for value in &values {
        let Some(global_id) = value["id"].as_u64().and_then(|id| GlobalId::try_from(id).ok()) else {
            continue;
        };
        let props = properties(value);
        let record = match value["type"].as_str().unwrap_or("") {
            "PipeWire:Interface:Client" => Upsert::Client(ClientRecord { id: global_id, props }),
            "PipeWire:Interface:Device" => Upsert::Device(DeviceRecord { id: global_id, props }),
            "PipeWire:Interface:Node" => Upsert::Node(NodeRecord {
                id: global_id,
                device_id: id(&props, "device.id"),
                client_id: id(&props, "client.id"),
                props,
                native: NativeProps {
                    channel_volumes: value["info"]["params"]["Props"][0]["channelVolumes"]
                        .as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_f64().map(|x| x as f32)).collect()),
                    muted: value["info"]["params"]["Props"][0]["mute"].as_bool(),
                    writable: value["info"]["params"]["Props"].as_array().is_some_and(|v| !v.is_empty()),
                },
            }),
            "PipeWire:Interface:Port" => {
                let direction = match props
                    .get("port.direction")
                    .map(String::as_str)
                    .or_else(|| value["info"]["direction"].as_str())
                {
                    Some("in" | "input" | "Input") => Direction::Input,
                    Some("out" | "output" | "Output") => Direction::Output,
                    _ => Direction::Other,
                };
                let enum_formats = value["info"]["params"]["EnumFormat"]
                    .as_array()
                    .map(|formats| formats.iter().map(format_kind).collect())
                    .unwrap_or_default();
                let mut props = props;
                if let Some(family) = value["info"]["params"]["EnumFormat"].as_array().and_then(|formats| {
                    formats.iter().find_map(|format| {
                        (format["mediaType"].as_str() == Some("video"))
                            .then(|| format["mediaSubtype"].as_str())
                            .flatten()
                            .filter(|subtype| matches!(*subtype, "raw" | "dsp"))
                    })
                }) {
                    props.insert("video.enum.family".into(), family.into());
                }
                Upsert::Port(PortRecord {
                    id: global_id,
                    node_id: id(&props, "node.id"),
                    direction,
                    enum_formats,
                    props,
                })
            }
            "PipeWire:Interface:Link" => {
                let output = id(&props, "link.output.port")
                    .or_else(|| value["info"]["output-port-id"].as_u64().and_then(|n| GlobalId::try_from(n).ok()));
                let input = id(&props, "link.input.port")
                    .or_else(|| value["info"]["input-port-id"].as_u64().and_then(|n| GlobalId::try_from(n).ok()));
                let (Some(output), Some(input)) = (output, input) else {
                    continue;
                };
                let state = match value["info"]["state"].as_str().unwrap_or("active") {
                    "active" => LinkState::Active,
                    "error" => LinkState::Error(value["info"]["error"].as_str().unwrap_or("link failed").to_owned()),
                    _ => LinkState::Negotiating,
                };
                Upsert::Link(LinkRecord { id: global_id, output, input, state, props })
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
        let mut graph = Graph::default();
        graph.apply(delta);
        assert_eq!(graph.epoch, 7);
        assert_eq!(graph.nodes.len(), 11);
        assert_eq!(graph.ports.len(), 25);
        assert_eq!(
            graph
                .nodes
                .values()
                .filter(|node| node.props.get("factory.name").is_some_and(|name| name == "support.null-audio-sink"))
                .count(),
            4
        );
        assert_eq!(
            graph
                .nodes
                .values()
                .filter(|node| node
                    .props
                    .get("media.class")
                    .is_some_and(|name| name == "Stream/Output/Audio" || name == "Stream/Input/Audio"))
                .count(),
            2
        );
        assert_eq!(graph.ports_of_kind(PortKind::Audio).count(), 22);
        assert_eq!(graph.ports_of_kind(PortKind::Midi).count(), 2);
        assert_eq!(graph.ports_of_kind(PortKind::Video).count(), 1);
        assert!(!graph.ports[&70].props.contains_key("format.dsp"));
        assert_eq!(graph.kind(70), Some(PortKind::Video));
        assert!(graph.ports[&70].enum_formats.contains(&PortKind::Video));
    }
}
