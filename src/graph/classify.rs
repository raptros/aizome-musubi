//! Per-port classification and channel ordering.
use super::{Direction, NodeRecord, PortRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortKind {
    Audio,
    Midi,
    Ump,
    Video,
    Other,
}

pub fn from_dsp(dsp: &str) -> PortKind {
    match dsp {
        "32 bit float mono audio" => PortKind::Audio,
        "8 bit raw midi" => PortKind::Midi,
        "32 bit raw UMP" => PortKind::Ump,
        "32 bit float RGBA video" => PortKind::Video,
        _ => PortKind::Other,
    }
}

pub fn from_enum_format(media_type: &str, subtype: &str) -> PortKind {
    match (media_type.to_ascii_lowercase().as_str(), subtype.to_ascii_lowercase().as_str()) {
        ("audio", "raw" | "dsp") => PortKind::Audio,
        ("video", "raw" | "dsp" | "mjpg" | "h264" | "h265") => PortKind::Video,
        ("application", "control") => PortKind::Other, // Control may be MIDI, UMP, or something else.
        _ => PortKind::Other,
    }
}

pub fn classify(port: &PortRecord, node: Option<&NodeRecord>) -> PortKind {
    if let Some(kind) = port.props.get("format.dsp").map(|s| from_dsp(s)) {
        if kind != PortKind::Other {
            return kind;
        }
    }
    // Alternatives with different kinds do not establish a safe category.
    if let Some((&first, rest)) = port.enum_formats.split_first() {
        if first != PortKind::Other && rest.iter().all(|&kind| kind == first) {
            return first;
        }
        if rest.iter().any(|&kind| kind != first) {
            return PortKind::Other;
        }
        // A control EnumFormat with no distinguishable MIDI/UMP type cannot
        // inherit MIDI 1.0 from a broad node media.class.
        return PortKind::Other;
    }
    let class = node.and_then(|node| node.props.get("media.class")).map(String::as_str);
    match class {
        Some("Audio/Sink" | "Audio/Source" | "Audio/Duplex" | "Stream/Output/Audio" | "Stream/Input/Audio") => {
            PortKind::Audio
        }
        Some("Midi/Bridge" | "Stream/Output/Midi" | "Stream/Input/Midi") => PortKind::Midi,
        Some("Video/Source" | "Video/Sink" | "Stream/Output/Video" | "Stream/Input/Video") => PortKind::Video,
        _ => PortKind::Other,
    }
}

/// Semantic channel ordering; unknown names retain a deterministic lexical fallback.
pub fn channel_sort_key(channel: Option<&str>) -> (u8, u32, &str) {
    match channel {
        Some(name) => {
            let rank = match name {
                "FL" => Some(0),
                "FR" => Some(1),
                "FC" => Some(2),
                "LFE" => Some(3),
                "RL" => Some(4),
                "RR" => Some(5),
                "SL" => Some(6),
                "SR" => Some(7),
                _ => None,
            };
            if let Some(rank) = rank {
                return (0, rank, "");
            }
            if let Some(n) = name.strip_prefix("AUX").and_then(|n| n.parse::<u32>().ok()) {
                return (1, n, "");
            }
            if name == "MONO" {
                return (2, 0, "");
            }
            (3, 0, name)
        }
        None => (4, 0, ""),
    }
}

pub fn is_stereo_pair(left: &PortRecord, right: &PortRecord, left_kind: PortKind, right_kind: PortKind) -> bool {
    left.id != right.id
        && left.node_id.is_some()
        && left.node_id == right.node_id
        && left.direction != Direction::Other
        && left.direction == right.direction
        && left_kind == right_kind
        && left_kind != PortKind::Other
        && left.props.get("port.group") == right.props.get("port.group")
        && left.props.get("audio.channel").map(String::as_str) == Some("FL")
        && right.props.get("audio.channel").map(String::as_str) == Some("FR")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GlobalId, Properties};
    fn port(id: GlobalId, channel: &str) -> PortRecord {
        PortRecord {
            id,
            node_id: Some(1),
            direction: Direction::Output,
            enum_formats: vec![],
            props: Properties::from([("audio.channel".into(), channel.into())]),
        }
    }
    #[test]
    fn precedence_and_unknown_control() {
        let mut p = port(1, "FL");
        let n = NodeRecord {
            id: 1,
            client_id: None,
            device_id: None,
            props: Properties::from([("media.class".into(), "Video/Source".into())]),
        };
        assert_eq!(classify(&p, Some(&n)), PortKind::Video);
        p.enum_formats = vec![PortKind::Audio];
        assert_eq!(classify(&p, Some(&n)), PortKind::Audio);
        p.props.insert("format.dsp".into(), "8 bit raw midi".into());
        assert_eq!(classify(&p, Some(&n)), PortKind::Midi);
        p.props.insert("format.dsp".into(), "32 bit raw UMP".into());
        assert_eq!(classify(&p, Some(&n)), PortKind::Ump);
        p.props.insert("format.dsp".into(), "32 bit float RGBA video".into());
        assert_eq!(classify(&p, Some(&n)), PortKind::Video);
        p.props.insert("format.dsp".into(), "32 bit float mono audio".into());
        assert_eq!(classify(&p, Some(&n)), PortKind::Audio);
        p.props.clear();
        p.enum_formats = vec![PortKind::Other];
        assert_eq!(from_enum_format("application", "control"), PortKind::Other);
        assert_eq!(classify(&p, None), PortKind::Other);
        let midi_node = NodeRecord { props: Properties::from([("media.class".into(), "Stream/Input/Midi".into())]), ..n.clone() };
        assert_eq!(classify(&p, Some(&midi_node)), PortKind::Other);
        p.enum_formats = vec![PortKind::Midi];
        assert_eq!(classify(&p, Some(&midi_node)), PortKind::Midi);
        p.enum_formats = vec![PortKind::Ump];
        assert_eq!(classify(&p, Some(&midi_node)), PortKind::Ump);
        p.enum_formats = vec![PortKind::Audio, PortKind::Video];
        assert_eq!(classify(&p, Some(&n)), PortKind::Other);
    }
    #[test]
    fn channel_and_pair() {
        let mut channels = ["AUX10", "MONO", "FR", "AUX9", "FL", "FC", "RL"];
        channels.sort_by_key(|name| channel_sort_key(Some(name)));
        assert_eq!(channels, ["FL", "FR", "FC", "RL", "AUX9", "AUX10", "MONO"]);
        let left = port(1, "FL");
        let mut right = port(2, "FR");
        assert!(is_stereo_pair(&left, &right, PortKind::Audio, PortKind::Audio));
        right.props.insert("port.group".into(), "other".into());
        assert!(!is_stereo_pair(&left, &right, PortKind::Audio, PortKind::Audio));
        right.props.remove("port.group");
        assert!(!is_stereo_pair(&left, &right, PortKind::Audio, PortKind::Video));
        right.node_id = Some(3);
        assert!(!is_stereo_pair(&left, &right, PortKind::Audio, PortKind::Audio));
        right.node_id = Some(1);
        right.direction = Direction::Input;
        assert!(!is_stereo_pair(&left, &right, PortKind::Audio, PortKind::Audio));
        right.direction = Direction::Output;
        right.props.insert("audio.channel".into(), "AUX1".into());
        assert!(!is_stereo_pair(&left, &right, PortKind::Audio, PortKind::Audio));
        right.props.insert("audio.channel".into(), "MONO".into());
        assert!(!is_stereo_pair(&left, &right, PortKind::Audio, PortKind::Audio));
        let aux0 = port(3, "AUX0");
        let aux1 = port(4, "AUX1");
        assert!(!is_stereo_pair(&aux0, &aux1, PortKind::Audio, PortKind::Audio));
    }
}
