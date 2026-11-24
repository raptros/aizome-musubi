use std::collections::{HashMap, HashSet};

use crate::{
    app::Page,
    graph::{
        self, Direction, GlobalId, Graph, LinkState,
        classify::{self, PortKind},
        group::{GroupKey, GroupKind},
    },
};

pub const PITCH: f64 = 24.0;
pub const GAP: f64 = 8.0;
pub const ROW_HEADER: f64 = 188.0;
pub const COLUMN_HEADER: f64 = 180.0;
pub const MIN_ROW_HEADER: f64 = 100.;
pub const MIN_COLUMN_HEADER: f64 = 136.;
type AxisGroup = (String, String, Kind, Option<GroupKey>, bool, bool, Vec<(String, Option<String>)>);

/// Sibling groups that share a name (two identical keyboards, two sequencer
/// clients with one name) get a suffix that tells them apart: the ALSA
/// sequencer client number where there is one, else their order.
fn disambiguate(groups: &mut [AxisGroup]) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for group in groups.iter() {
        *counts.entry(group.0.clone()).or_default() += 1;
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    for group in groups.iter_mut() {
        if counts[&group.0] < 2 {
            continue;
        }
        let ordinal = seen.entry(group.0.clone()).or_default();
        *ordinal += 1;
        let suffix = match &group.3 {
            Some(GroupKey::MidiClient { client, .. }) => format!(" #{client}"),
            _ => format!(" ({ordinal})"),
        };
        group.0.push_str(&suffix);
    }
}

/// Keep distinguishing ends of names when siblings share a prefix or suffix.
pub fn elide(name: &str, siblings: &[String], max_chars: usize) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= max_chars {
        return name.to_owned();
    }
    if max_chars <= 1 {
        return "…".to_owned();
    }
    let shared = siblings
        .iter()
        .filter(|other| other.as_str() != name)
        .map(|other| chars.iter().zip(other.chars()).take_while(|(a, b)| **a == *b).count())
        .max()
        .unwrap_or(0);
    let suffix = siblings
        .iter()
        .filter(|other| other.as_str() != name)
        .map(|other| chars.iter().rev().zip(other.chars().rev()).take_while(|(a, b)| **a == *b).count())
        .max()
        .unwrap_or(0);
    let boundary = (1..=shared.min(chars.len().saturating_sub(3)))
        .filter_map(|n| {
            if chars[n - 1].is_whitespace() {
                Some(n)
            } else if !chars[n - 1].is_alphanumeric() {
                Some(n - 1)
            } else if chars[n].is_whitespace() {
                Some(n + 1)
            } else if !chars[n].is_alphanumeric() {
                Some(n)
            } else {
                None
            }
        })
        .find(|&n| chars.len() - n + 1 <= max_chars);
    if let Some(n) = boundary {
        let tail = &chars[n..];
        if tail.len() >= 3 && tail.len() < chars.len() && tail.len() + 1 <= max_chars {
            return format!("…{}", tail.iter().collect::<String>());
        }
    }
    if suffix >= 2 && chars.len() > suffix + 2 {
        let prefix = &chars[..chars.len() - suffix];
        if prefix.len() + 1 <= max_chars {
            return format!("{}…", prefix.iter().collect::<String>());
        }
    }
    if shared > 0 && chars.len() - shared + 1 <= max_chars {
        return format!("…{}", chars[shared..].iter().collect::<String>());
    }
    if siblings.iter().filter(|other| other.as_str() == name).count() == 1 && siblings.len() == 1 {
        return format!("…{}", chars[chars.len() - max_chars + 1..].iter().collect::<String>());
    }
    format!("{}…", chars[..max_chars - 1].iter().collect::<String>())
}

#[cfg(test)]
mod elision_tests {
    use super::elide;

    #[test]
    fn sibling_prefix_and_fallback() {
        let siblings: Vec<String> = vec!["musubi-vm-default".into(), "musubi-vm-patch".into()];
        assert_eq!(elide(&siblings[1], &siblings, 9), "…-patch");
        let midi: Vec<String> = vec!["Virtual Raw MIDI 0-0".into(), "Virtual Raw MIDI 0-1".into()];
        assert_eq!(elide(&midi[0], &midi, 10), "…MIDI 0-0");
        assert_eq!(elide("Long unique label", &siblings, 8), "Long un…");
        assert_eq!(elide("abc", &siblings, 2), "a…");
        let suffix: Vec<String> = vec!["Foo · x".into(), "Bar · x".into()];
        assert_eq!(elide(&suffix[0], &suffix, 5), "Foo…");
        assert_eq!(elide(&suffix[1], &suffix, 5), "Bar…");
        assert_eq!(elide("musubi-vm-patch", &["musubi-vm-patch".into()], 12), "…bi-vm-patch");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellState {
    Empty,
    Connected,
    Half,
    Asked,
    Error,
    NotAllowed,
    Count(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Hardware,
    Application,
    Effect,
    Virtual,
}

impl From<GroupKind> for Kind {
    fn from(value: GroupKind) -> Self {
        match value {
            GroupKind::Hardware => Self::Hardware,
            GroupKind::Application => Self::Application,
            GroupKind::Effect => Self::Effect,
            GroupKind::Virtual => Self::Virtual,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    pub name: String,
    pub detail: String,
    pub kind: Kind,
    pub first: usize,
    pub start: f64,
    pub finish: f64,
    pub end: usize,
    pub key: Option<GroupKey>,
    pub collapsed: bool,
    pub midi_1: bool,
}

#[derive(Clone, Debug)]
pub struct Axis {
    pub groups: Vec<Group>,
    pub labels: Vec<String>,
    pub offsets: Vec<f64>,
    pub extent: f64,
    pub subtitles: Vec<(f64, String)>,
}

impl Axis {
    pub fn new(groups: Vec<(String, String, Kind, Vec<String>)>) -> Self {
        Self::from_groups(
            groups
                .into_iter()
                .map(|(name, detail, kind, labels)| {
                    (name, detail, kind, None, false, false, labels.into_iter().map(|label| (label, None)).collect())
                })
                .collect(),
        )
    }

    fn from_groups(
        groups: Vec<(String, String, Kind, Option<GroupKey>, bool, bool, Vec<(String, Option<String>)>)>,
    ) -> Self {
        let (mut entries, mut labels, mut offsets, mut subtitles) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut at = 0.0;
        for (name, detail, kind, key, collapsed, midi_1, ports) in groups {
            if ports.is_empty() {
                continue;
            }
            if !entries.is_empty() {
                at += GAP;
            }
            let first = labels.len();
            let start = at;
            let multi_stream = ports.iter().filter(|(_, subtitle)| subtitle.is_some()).count() > 1;
            if multi_stream {
                at += 20.;
            }
            for (port, subtitle) in ports {
                if let Some(subtitle) = subtitle {
                    if at > start + 20. {
                        at += 8.;
                    }
                    subtitles.push((at, subtitle));
                    at += 20.;
                }
                offsets.push(at);
                labels.push(port);
                at += PITCH;
            }
            if kind == Kind::Application && !detail.is_empty() && labels.len() == first + 1 {
                at += 16.;
            }
            entries.push(Group {
                name,
                detail,
                kind,
                first,
                end: labels.len(),
                key,
                collapsed,
                midi_1,
                start,
                finish: at,
            });
        }
        Self { groups: entries, labels, offsets, extent: at, subtitles }
    }

    pub fn at(&self, p: f64) -> Option<usize> {
        let i = self.offsets.partition_point(|&start| start <= p).checked_sub(1)?;
        (p < self.offsets[i] + PITCH).then_some(i)
    }

    pub fn visible(&self, start: f64, end: f64) -> std::ops::Range<usize> {
        let first = self.offsets.partition_point(|&p| p + PITCH <= start);
        let last = self.offsets.partition_point(|&p| p < end);
        first..last
    }

    pub fn group_of(&self, i: usize) -> usize {
        self.groups.partition_point(|g| g.end <= i)
    }

    pub fn next_group(&self, i: usize, forward: bool) -> usize {
        let g = self.group_of(i);
        if forward {
            self.groups.get(g + 1).map_or(i, |next| next.first)
        } else {
            self.groups.get(g.wrapping_sub(1)).map_or(i, |prev| prev.first)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub ports: Vec<GlobalId>,
    pub kind: PortKind,
    pub video_family: Option<String>,
    pub collapsed: bool,
    pub tooltip: String,
}

#[derive(Clone, Debug, Default)]
pub struct ViewOptions {
    pub pairs: bool,
    pub swap: bool,
    pub monitors: bool,
    pub show_midi_1: bool,
    pub list: bool,
    pub row_header: f64,
    pub column_header: f64,
    pub full_names: bool,
    pub from: String,
    pub to: String,
    pub folded: HashSet<GroupKey>,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub rows: Axis,
    pub cols: Axis,
    /// Sparse request-state overrides, keyed by displayed cell. S4b owns and fills this map.
    pub error_reasons: HashMap<Cell, String>,
    pub cells: HashMap<Cell, CellState>,
    pub row_lines: Vec<Line>,
    pub col_lines: Vec<Line>,
    pub links: HashSet<(GlobalId, GlobalId)>,
    pub hidden_from: usize,
    pub hidden_to: usize,
    pub revision: u64,
    pub page: Page,
    pub swapped: bool,
    pub row_header: f64,
    pub column_header: f64,
}

impl Layout {
    pub fn fitted_headers(&self) -> (f64, f64) {
        let row = self.rows.groups.iter().map(|g| g.name.chars().count()).max().unwrap_or(0);
        let row_detail = self.rows.groups.iter().map(|g| g.detail.chars().count()).max().unwrap_or(0);
        let row_label = self.rows.labels.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        let row_subtitle = self.rows.subtitles.iter().map(|(_, s)| s.chars().count()).max().unwrap_or(0);
        let col = self.cols.groups.iter().map(|g| g.name.chars().count()).max().unwrap_or(0);
        let col_detail = self.cols.groups.iter().map(|g| g.detail.chars().count()).max().unwrap_or(0);
        let col_subtitle = self.cols.subtitles.iter().map(|(_, s)| s.chars().count()).max().unwrap_or(0);
        let col_label = self.cols.labels.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        (
            (25. + row as f64 * 7.5 + row_label as f64 * 7. + 20.)
                .max(97. + row as f64 * 7.5)
                .max(95. + row_detail as f64 * 7.)
                .max(33. + row_subtitle as f64 * 7.)
                .max(MIN_ROW_HEADER),
            (48. + col.max(col_detail).max(col_subtitle) as f64 * 7.5 + col_label as f64 * 7. + 26.)
                .max(MIN_COLUMN_HEADER),
        )
    }

    pub fn state(&self, cell: Cell) -> CellState {
        if let Some(&state) = self.cells.get(&cell) {
            return state;
        }
        let (Some(row), Some(col)) = (self.row_lines.get(cell.row), self.col_lines.get(cell.col)) else {
            return CellState::Empty;
        };
        let (out, input) = if self.swapped { (col, row) } else { (row, col) };
        let count =
            out.ports.iter().map(|&a| input.ports.iter().filter(|&&b| self.links.contains(&(a, b))).count()).sum();
        if out.collapsed || input.collapsed {
            return CellState::Count(count);
        }
        if out.kind == PortKind::Other
            || input.kind == PortKind::Other
            || !(out.kind == input.kind
                || matches!((out.kind, input.kind), (PortKind::Midi, PortKind::Ump) | (PortKind::Ump, PortKind::Midi)))
            || (out.kind == PortKind::Video && (out.video_family.is_none() || out.video_family != input.video_family))
        {
            return CellState::NotAllowed;
        }
        if count == 0 {
            CellState::Empty
        } else if count < out.ports.len().max(input.ports.len()) {
            CellState::Half
        } else {
            CellState::Connected
        }
    }

    /// Locate a visible cell without constructing the grid's cross product.
    pub fn cell_for_pair(&self, pair: (GlobalId, GlobalId)) -> Option<Cell> {
        let (from, to) = if self.swapped { (pair.1, pair.0) } else { pair };
        Some(Cell {
            row: self.row_lines.iter().position(|line| line.ports.contains(&from))?,
            col: self.col_lines.iter().position(|line| line.ports.contains(&to))?,
        })
    }

    pub fn from_graph(graph: &Graph, page: Page, options: &ViewOptions) -> Self {
        let kinds: &[PortKind] = match page {
            Page::Audio | Page::Mixer | Page::Effects | Page::Devices => &[PortKind::Audio],
            Page::Midi => &[PortKind::Midi],
            Page::Midi2 if options.show_midi_1 => &[PortKind::Ump, PortKind::Midi],
            Page::Midi2 => &[PortKind::Ump],
            Page::Video => &[PortKind::Video],
        };
        let (from_groups, from_lines, hidden_from) = Self::axis_from_graph(
            graph,
            Direction::Output,
            kinds,
            options,
            &options.from,
            page == Page::Midi2 && options.show_midi_1,
        );
        let (to_groups, to_lines, hidden_to) = Self::axis_from_graph(
            graph,
            Direction::Input,
            kinds,
            options,
            &options.to,
            page == Page::Midi2 && options.show_midi_1,
        );
        let (rows, row_lines, cols, col_lines) = if options.swap {
            (to_groups, to_lines, from_groups, from_lines)
        } else {
            (from_groups, from_lines, to_groups, to_lines)
        };
        let row_name_chars = rows.groups.iter().map(|g| g.name.chars().count()).max().unwrap_or(0);
        let mut layout = Self {
            rows,
            cols,
            row_lines,
            col_lines,
            links: graph
                .links
                .values()
                .filter(|link| link.state == LinkState::Active)
                .map(|link| (link.output, link.input))
                .collect(),
            cells: HashMap::new(),
            error_reasons: HashMap::new(),
            hidden_from,
            hidden_to,
            revision: graph.epoch,
            swapped: options.swap,
            page,
            row_header: if options.row_header == 0. {
                (105. + row_name_chars as f64 * 7.5).clamp(ROW_HEADER, 280.)
            } else {
                options.row_header.max(MIN_ROW_HEADER)
            },
            column_header: if options.column_header == 0. {
                COLUMN_HEADER
            } else {
                options.column_header.max(MIN_COLUMN_HEADER)
            },
        };
        if options.full_names {
            (layout.row_header, layout.column_header) = layout.fitted_headers();
        }
        layout
    }

    fn axis_from_graph(
        graph: &Graph,
        direction: Direction,
        kinds: &[PortKind],
        options: &ViewOptions,
        filter: &str,
        tag_midi: bool,
    ) -> (Axis, Vec<Line>, usize) {
        let mut groups = Vec::new();
        let mut lines = Vec::new();
        let mut hidden = 0;
        let query = filter.to_lowercase();
        for group in graph.groups(direction) {
            let ports: Vec<_> = group
                .ports
                .iter()
                .copied()
                .filter(|&id| {
                    let kind = graph.kind(id).unwrap_or(PortKind::Other);
                    if !kinds.contains(&kind) {
                        return false;
                    }
                    if direction == Direction::Output
                        && !options.monitors
                        && graph.ports[&id].props.get("port.monitor").is_some_and(|v| v == "true")
                    {
                        return false;
                    }
                    true
                })
                .collect();
            if ports.is_empty() {
                continue;
            }
            let group_match = !query.is_empty() && group.label.to_lowercase().contains(&query);
            let filtered: Vec<_> = ports
                .iter()
                .copied()
                .filter(|&id| {
                    query.is_empty()
                        || group_match
                        || graph::group::port_label(&graph.ports[&id]).to_lowercase().contains(&query)
                })
                .collect();
            hidden += ports.len() - filtered.len();
            if filtered.is_empty() {
                continue;
            }
            let port_count = filtered.len();
            let collapsed = options.folded.contains(&group.key);
            let midi_group = tag_midi && filtered.iter().all(|id| graph.kind(*id) == Some(PortKind::Midi));
            let mut labels = Vec::new();
            if collapsed {
                let first = graph::group::port_label(&graph.ports[&filtered[0]]);
                let last = graph::group::port_label(&graph.ports[filtered.last().expect("nonempty")]);
                labels
                    .push((if filtered.len() == 1 { format!("{first} ▸") } else { format!("{first}–{last} ▸") }, None));
                lines.push(Self::line(graph, filtered.clone(), true));
            } else {
                let mut index = 0;
                let first_node = graph.ports[&filtered[0]].node_id;
                let multi_stream = matches!(group.key, GroupKey::Client(_))
                    && filtered.iter().any(|id| graph.ports[id].node_id != first_node);
                let mut previous_stream = None;
                while index < filtered.len() {
                    let a = filtered[index];
                    let node = graph.ports[&a].node_id;
                    let subtitle = if multi_stream && node != previous_stream {
                        node.map(|id| graph::group::stream_subtitle(graph, id))
                    } else {
                        None
                    };
                    previous_stream = node;
                    if options.pairs
                        && index + 1 < filtered.len()
                        && classify::is_stereo_pair(
                            &graph.ports[&a],
                            &graph.ports[&filtered[index + 1]],
                            graph.kind(a).unwrap_or(PortKind::Other),
                            graph.kind(filtered[index + 1]).unwrap_or(PortKind::Other),
                        )
                    {
                        labels.push(("L·R".to_owned(), subtitle));
                        lines.push(Self::line(graph, vec![a, filtered[index + 1]], false));
                        index += 2;
                    } else {
                        labels.push((graph::group::port_label(&graph.ports[&a]), subtitle));
                        lines.push(Self::line(graph, vec![a], false));
                        index += 1;
                    }
                }
            }
            groups.push((
                group.label,
                if matches!(group.key, GroupKey::Client(_))
                    && filtered.iter().all(|id| graph.ports[id].node_id == graph.ports[&filtered[0]].node_id)
                {
                    graph.ports[&filtered[0]]
                        .node_id
                        .map(|id| graph::group::stream_subtitle(graph, id))
                        .unwrap_or_default()
                } else {
                    format!("{port_count} ports")
                },
                group.kind.into(),
                Some(group.key),
                collapsed,
                midi_group,
                labels,
            ));
        }
        disambiguate(&mut groups);
        (Axis::from_groups(groups), lines, hidden)
    }

    fn line(graph: &Graph, ports: Vec<GlobalId>, collapsed: bool) -> Line {
        let first = &graph.ports[&ports[0]];
        Line {
            tooltip: ports
                .iter()
                .map(|id| {
                    let port = &graph.ports[id];
                    let node = port.node_id.map(|id| graph::group::stream_subtitle(graph, id)).unwrap_or_default();
                    format!(
                        "{node} · {} · {}",
                        port.props.get("port.name").map(String::as_str).unwrap_or(""),
                        port.props.get("port.alias").map(String::as_str).unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join(" / "),
            kind: graph.kind(first.id).unwrap_or(PortKind::Other),
            video_family: first
                .props
                .get("video.format.family")
                .or_else(|| first.props.get("format.dsp"))
                .or_else(|| first.props.get("video.enum.family"))
                .cloned(),
            ports,
            collapsed,
        }
    }

    /// Synthetic data is for headless geometry tests; no runtime backend consumes it.
    pub fn studio(rows: usize, cols: usize) -> Self {
        fn axis(n: usize, source: bool) -> Axis {
            let names = if source {
                ["Scarlett 4i4", "JACK Studio", "Firefox", "EQ Chain", "Monitor Bus"]
            } else {
                ["Studio Monitors", "JACK DAW", "Discord", "OBS Studio", "Virtual Sink"]
            };
            let kinds = [Kind::Hardware, Kind::Application, Kind::Application, Kind::Effect, Kind::Virtual];
            let mut groups = Vec::new();
            for first in (0..n).step_by(6) {
                let g = (first / 6) % names.len();
                let labels = (first..(first + 6).min(n))
                    .map(|i| match i % 6 {
                        0 => "FL".into(),
                        1 => "FR".into(),
                        2 => "AUX1".into(),
                        3 => "AUX2".into(),
                        _ => format!("In {}", i % 6 + 1),
                    })
                    .collect();
                groups.push((
                    format!("{} {}", names[g], first / 6 + 1),
                    format!("{} ports", (first + 6).min(n) - first),
                    kinds[g],
                    labels,
                ));
            }
            Axis::new(groups)
        }
        let mut cells = HashMap::new();
        for row in 0..rows {
            for col in 0..cols {
                let state = match (row * 17 + col * 13) % 79 {
                    0..=2 => CellState::Connected,
                    3 => CellState::Half,
                    4 => CellState::Asked,
                    5 => CellState::Error,
                    6 => CellState::NotAllowed,
                    _ => continue,
                };
                cells.insert(Cell { row, col }, state);
            }
        }
        Self {
            rows: axis(rows, true),
            cols: axis(cols, false),
            cells,
            error_reasons: HashMap::new(),
            row_lines: vec![],
            col_lines: vec![],
            links: HashSet::new(),
            hidden_from: 0,
            hidden_to: 0,
            revision: 1,
            swapped: false,
            page: Page::Audio,
            row_header: ROW_HEADER,
            column_header: COLUMN_HEADER,
        }
    }
}
