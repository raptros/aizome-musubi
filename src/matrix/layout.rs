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
}

impl Axis {
    pub fn new(groups: Vec<(String, String, Kind, Vec<String>)>) -> Self {
        Self::from_groups(
            groups
                .into_iter()
                .map(|(name, detail, kind, labels)| (name, detail, kind, None, false, false, labels))
                .collect(),
        )
    }

    fn from_groups(groups: Vec<(String, String, Kind, Option<GroupKey>, bool, bool, Vec<String>)>) -> Self {
        let (mut entries, mut labels, mut offsets) = (Vec::new(), Vec::new(), Vec::new());
        let mut at = 0.0;
        for (name, detail, kind, key, collapsed, midi_1, ports) in groups {
            if ports.is_empty() {
                continue;
            }
            if !entries.is_empty() {
                at += GAP;
            }
            let first = labels.len();
            for port in ports {
                offsets.push(at);
                labels.push(port);
                at += PITCH;
            }
            entries.push(Group { name, detail, kind, first, end: labels.len(), key, collapsed, midi_1 });
        }
        Self { groups: entries, labels, offsets, extent: at }
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
}

#[derive(Clone, Debug, Default)]
pub struct ViewOptions {
    pub pairs: bool,
    pub swap: bool,
    pub monitors: bool,
    pub show_midi_1: bool,
    pub list: bool,
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
}

impl Layout {
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
            Page::Audio => &[PortKind::Audio],
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
        Self {
            rows,
            cols,
            row_lines,
            col_lines,
            links: graph.links.values().filter(|link| link.state == LinkState::Active).map(|link| (link.output, link.input)).collect(),
            cells: HashMap::new(),
            error_reasons: HashMap::new(),
            hidden_from,
            hidden_to,
            revision: graph.epoch,
            swapped: options.swap,
            page,
        }
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
                labels.push(if filtered.len() == 1 { format!("{first} ▸") } else { format!("{first}–{last} ▸") });
                lines.push(Self::line(graph, filtered, true));
            } else {
                let mut index = 0;
                while index < filtered.len() {
                    let a = filtered[index];
                    if options.pairs
                        && index + 1 < filtered.len()
                        && classify::is_stereo_pair(
                            &graph.ports[&a],
                            &graph.ports[&filtered[index + 1]],
                            graph.kind(a).unwrap_or(PortKind::Other),
                            graph.kind(filtered[index + 1]).unwrap_or(PortKind::Other),
                        )
                    {
                        labels.push("L·R".to_owned());
                        lines.push(Self::line(graph, vec![a, filtered[index + 1]], false));
                        index += 2;
                    } else {
                        labels.push(graph::group::port_label(&graph.ports[&a]));
                        lines.push(Self::line(graph, vec![a], false));
                        index += 1;
                    }
                }
            }
            groups.push((
                group.label,
                format!("{port_count} ports"),
                group.kind.into(),
                Some(group.key),
                collapsed,
                midi_group,
                labels,
            ));
        }
        (Axis::from_groups(groups), lines, hidden)
    }

    fn line(graph: &Graph, ports: Vec<GlobalId>, collapsed: bool) -> Line {
        let first = &graph.ports[&ports[0]];
        Line {
            kind: graph.kind(first.id).unwrap_or(PortKind::Other),
            video_family: first.props.get("video.format.family").or_else(|| first.props.get("format.dsp")).or_else(|| first.props.get("video.enum.family")).cloned(),
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
        }
    }
}
