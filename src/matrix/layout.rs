use std::collections::HashMap;

pub const PITCH: f64 = 24.0;
pub const GAP: f64 = 8.0;
pub const ROW_HEADER: f64 = 188.0;
pub const COLUMN_HEADER: f64 = 180.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cell { pub row: usize, pub col: usize }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellState { Empty, Connected, Half, Asked, Error, NotAllowed }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Hardware, Application, Effect, Virtual }

#[derive(Clone, Debug)]
pub struct Group {
    pub name: String,
    pub detail: String,
    pub kind: Kind,
    pub first: usize,
    pub end: usize,
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
        let (mut entries, mut labels, mut offsets) = (Vec::new(), Vec::new(), Vec::new());
        let mut at = 0.0;
        for (name, detail, kind, ports) in groups {
            if ports.is_empty() { continue; }
            if !entries.is_empty() { at += GAP; }
            let first = labels.len();
            for port in ports { offsets.push(at); labels.push(port); at += PITCH; }
            entries.push(Group { name, detail, kind, first, end: labels.len() });
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
        if forward { self.groups.get(g + 1).map_or(i, |next| next.first) }
        else { self.groups.get(g.wrapping_sub(1)).map_or(i, |prev| prev.first) }
    }
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub rows: Axis,
    pub cols: Axis,
    pub cells: HashMap<Cell, CellState>,
    pub revision: u64,
}

impl Layout {
    pub fn state(&self, cell: Cell) -> CellState {
        self.cells.get(&cell).copied().unwrap_or(CellState::Empty)
    }

    /// Synthetic data is for headless geometry tests; no runtime backend consumes it.
    pub fn studio(rows: usize, cols: usize) -> Self {
        fn axis(n: usize, source: bool) -> Axis {
            let names = if source { ["Scarlett 4i4", "JACK Studio", "Firefox", "EQ Chain", "Monitor Bus"] }
                else { ["Studio Monitors", "JACK DAW", "Discord", "OBS Studio", "Virtual Sink"] };
            let kinds = [Kind::Hardware, Kind::Application, Kind::Application, Kind::Effect, Kind::Virtual];
            let mut groups = Vec::new();
            for first in (0..n).step_by(6) {
                let g = (first / 6) % names.len();
                let labels = (first..(first + 6).min(n))
                    .map(|i| match i % 6 { 0 => "FL".into(), 1 => "FR".into(), 2 => "AUX1".into(), 3 => "AUX2".into(), _ => format!("In {}", i % 6 + 1) })
                    .collect();
                groups.push((format!("{} {}", names[g], first / 6 + 1), format!("{} ports", (first + 6).min(n) - first), kinds[g], labels));
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
        Self { rows: axis(rows, true), cols: axis(cols, false), cells, revision: 1 }
    }
}
