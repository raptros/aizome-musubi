//! Pulse-visible controls use `Audio`; native Props belong only to graph nodes
//! without a matching Pulse object. An uncertain bridge never grants a control.
use std::collections::HashSet;

use aizome_sys::audio::{AudioDevice, AudioState, AudioStream};

use crate::backend::Handle;
use crate::graph::{Graph, NodeRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StripKind {
    Effect,
    Output,
    Input,
    Playback,
    Recording,
    Native,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StripControl {
    Pulse { index: u32, volume: f32, muted: bool, balance: Option<f32> },
    Native { node: Handle, channel_volumes: Vec<f32>, muted: Option<bool> },
    ReadOnly(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Strip {
    pub kind: StripKind,
    pub name: String,
    pub pulse_name: Option<String>,
    pub subtitle: String,
    pub node: Option<Handle>,
    pub control: StripControl,
}

fn serial(node: &NodeRecord) -> Option<u64> {
    node.props.get("object.serial")?.parse().ok()
}

/// pipewire-pulse exposes the PipeWire object serial as its Pulse index.
/// A serial match takes precedence; only an absent serial permits name fallback.
/// Multiple candidates, mismatched names, and missing nodes are read-only.
fn resolve<'a>(graph: &'a Graph, index: u32, name: Option<&str>) -> Result<&'a NodeRecord, String> {
    let candidates: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| !crate::graph::is_musubi_meter(&n.props) && serial(n) == Some(u64::from(index)))
        .collect();
    let match_name = |n: &&NodeRecord| n.props.get("node.name").is_some_and(|value| Some(value.as_str()) == name);
    let nodes = if candidates.is_empty() {
        name.filter(|n| !n.is_empty())
            .map(|_| {
                graph
                    .nodes
                    .values()
                    .filter(|n| !crate::graph::is_musubi_meter(&n.props) && serial(n).is_none())
                    .filter(match_name)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        candidates
    };
    match nodes.as_slice() {
        [] => Err(format!("No graph node for Pulse index {index}")),
        [node] if name.is_some_and(|expected| node.props.get("node.name").is_none_or(|actual| actual != expected)) => {
            Err(format!("Pulse name does not match graph node for index {index}"))
        }
        [node] => Ok(node),
        _ => Err(format!("Several graph nodes match Pulse index {index}")),
    }
}

fn device_strip(graph: &Graph, device: &AudioDevice, kind: StripKind) -> Strip {
    let resolution = resolve(graph, device.index, Some(&device.name));
    let node = resolution.as_ref().ok().map(|n| Handle { epoch: graph.epoch, id: n.id });
    let control = match resolution {
        Ok(_) => StripControl::Pulse {
            index: device.index,
            volume: device.volume,
            muted: device.muted,
            balance: Some(device.balance),
        },
        Err(reason) => StripControl::ReadOnly(reason),
    };
    let subtitle = device
        .active_port
        .as_deref()
        .and_then(|active| device.ports.iter().find(|port| port.name == active))
        .map(|port| port.description.clone())
        .or_else(|| (!device.bus.is_empty()).then(|| device.bus.clone()))
        .unwrap_or_else(|| device.description.clone());
    Strip { kind, name: device.description.clone(), pulse_name: Some(device.name.clone()), subtitle, node, control }
}

fn stream_strip(graph: &Graph, stream: &AudioStream, kind: StripKind) -> Strip {
    let resolution = resolve(graph, stream.index, None);
    let node = resolution.as_ref().ok().map(|n| Handle { epoch: graph.epoch, id: n.id });
    let control = match resolution {
        Ok(_) if stream.volume_writable => {
            StripControl::Pulse { index: stream.index, volume: stream.volume, muted: stream.muted, balance: None }
        }
        Ok(_) => StripControl::ReadOnly("Pulse stream volume is not writable".into()),
        Err(reason) => StripControl::ReadOnly(reason),
    };
    Strip {
        kind,
        name: if stream.application.is_empty() { stream.media_name.clone() } else { stream.application.clone() },
        pulse_name: None,
        subtitle: stream.media_name.clone(),
        node,
        control,
    }
}

/// Immutable mixer projection; no proxy, command or UI state is stored here.
pub fn strips(audio: &AudioState, graph: &Graph) -> Vec<Strip> {
    let mut result = Vec::new();
    result.extend(audio.sinks.iter().map(|d| {
        device_strip(graph, d, if d.name.starts_with("musubi_eq.") { StripKind::Effect } else { StripKind::Output })
    }));
    result.extend(audio.sources.iter().map(|d| device_strip(graph, d, StripKind::Input)));
    result.extend(
        audio
            .playback
            .iter()
            .filter(|s| s.app_id != "org.aizome.Musubi" || s.media_name != "Musubi level meter")
            .map(|s| stream_strip(graph, s, StripKind::Playback)),
    );
    result.extend(
        audio
            .recording
            .iter()
            .filter(|s| s.app_id != "org.aizome.Musubi" || s.media_name != "Musubi level meter")
            .map(|s| stream_strip(graph, s, StripKind::Recording)),
    );
    let pulse_nodes: HashSet<_> = result.iter().filter_map(|s| s.node.map(|n| n.id)).collect();
    let pulse_names: HashSet<_> = audio.sinks.iter().chain(&audio.sources).map(|d| d.name.as_str()).collect();
    let pulse_stream_serials: HashSet<_> =
        audio.playback.iter().chain(&audio.recording).map(|s| u64::from(s.index)).collect();
    for node in graph.nodes.values() {
        if crate::graph::is_musubi_meter(&node.props) {
            continue;
        }
        if pulse_nodes.contains(&node.id)
            || node.props.get("node.name").is_some_and(|name| pulse_names.contains(name.as_str()))
            || serial(node).is_some_and(|value| pulse_stream_serials.contains(&value))
            || !node.props.get("media.class").is_some_and(|class| {
                class.starts_with("Audio/")
                    || class.starts_with("Stream/Output/Audio")
                    || class.starts_with("Stream/Input/Audio")
            })
        {
            continue;
        }
        // A filter is not a Pulse sink/source or playback/recording stream.
        let native_only = node.props.get("media.class").is_some_and(|class| class == "Audio/Filter");
        let control = match (&node.native.channel_volumes, node.native.writable, native_only) {
            (Some(gains), true, true) if !gains.is_empty() => StripControl::Native {
                node: Handle { epoch: graph.epoch, id: node.id },
                channel_volumes: gains.clone(),
                muted: node.native.muted,
            },
            _ => StripControl::ReadOnly("Native-only control not established".into()),
        };
        result.push(Strip {
            kind: StripKind::Native,
            name: node
                .props
                .get("node.description")
                .or_else(|| node.props.get("node.name"))
                .cloned()
                .unwrap_or_else(|| format!("Node {}", node.id)),
            pulse_name: None,
            subtitle: node.props.get("node.name").cloned().unwrap_or_default(),
            node: Some(Handle { epoch: graph.epoch, id: node.id }),
            control,
        });
    }
    result.sort_by_key(|s| (s.kind as u8, s.name.clone()));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphDelta, Properties, Upsert};

    #[test]
    fn recorded_private_proplists_bridge_serials_and_fail_closed() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/private-pulse.json")).unwrap();
        let mut audio = AudioState::default();
        let mut graph = Graph::default();
        let mut records = Vec::new();
        for (list, kind) in [
            ("sinks", StripKind::Output),
            ("sources", StripKind::Input),
            ("sink-inputs", StripKind::Playback),
            ("source-outputs", StripKind::Recording),
        ] {
            for entry in fixture[list].as_array().unwrap() {
                let index = entry["index"].as_u64().unwrap() as u32;
                let serial = entry["properties"]["object.serial"].as_str().unwrap();
                assert_eq!(serial.parse::<u32>().unwrap(), index);
                let name = entry["properties"]["node.name"].as_str().unwrap();
                let node = NodeRecord {
                    id: index + 100,
                    client_id: None,
                    device_id: None,
                    props: Properties::from([
                        ("object.serial".into(), serial.into()),
                        ("node.name".into(), name.into()),
                        ("media.class".into(), "Audio/Sink".into()),
                    ]),
                    native: Default::default(),
                };
                records.push(Upsert::Node(node));
                match kind {
                    StripKind::Output => {
                        audio.sinks.push(AudioDevice { index, name: name.into(), ..Default::default() })
                    }
                    StripKind::Input => {
                        audio.sources.push(AudioDevice { index, name: name.into(), ..Default::default() })
                    }
                    StripKind::Playback => {
                        audio.playback.push(AudioStream { index, volume_writable: true, ..Default::default() })
                    }
                    StripKind::Recording => {
                        audio.recording.push(AudioStream { index, volume_writable: true, ..Default::default() })
                    }
                    StripKind::Native | StripKind::Effect => unreachable!(),
                }
            }
        }
        graph.apply(GraphDelta { epoch: 7, upserts: records, removals: vec![] });
        assert_eq!(
            strips(&audio, &graph).iter().filter(|s| matches!(s.control, StripControl::Pulse { .. })).count(),
            6
        );
        graph.nodes.get_mut(&133).unwrap().props.insert("node.name".into(), "wrong".into());
        assert!(matches!(device_strip(&graph, &audio.sinks[0], StripKind::Output).control, StripControl::ReadOnly(_)));
        let duplicate = graph.nodes[&134].clone();
        graph.nodes.insert(999, NodeRecord { id: 999, ..duplicate });
        assert!(
            matches!(resolve(&graph, 34, Some("musubi_test_interface_out")), Err(reason) if reason.contains("Several"))
        );
        graph.nodes.get_mut(&135).unwrap().props.remove("object.serial");
        assert_eq!(resolve(&graph, 35, Some("musubi_test_interface_in")).unwrap().id, 135);
        assert!(resolve(&graph, 9999, None).is_err());
    }

    #[test]
    fn conflicting_serial_never_grants_pulse_or_native_control() {
        let mut graph = Graph::default();
        graph.nodes.insert(
            4,
            NodeRecord {
                id: 4,
                client_id: None,
                device_id: None,
                props: Properties::from([
                    ("object.serial".into(), "99".into()),
                    ("node.name".into(), "sink.one".into()),
                    ("media.class".into(), "Audio/Sink".into()),
                ]),
                native: crate::graph::NativeProps {
                    channel_volumes: Some(vec![0.5]),
                    writable: true,
                    ..Default::default()
                },
            },
        );
        let mut audio = AudioState::default();
        audio.sinks.push(AudioDevice { index: 7, name: "sink.one".into(), ..Default::default() });
        assert!(resolve(&graph, 7, Some("sink.one")).is_err());
        assert!(strips(&audio, &graph).iter().all(|strip| matches!(strip.control, StripControl::ReadOnly(_))));
    }
}
