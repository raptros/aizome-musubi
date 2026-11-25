//! Backend boundary: commands refer to IDs in one observed connection epoch.
pub mod fake;
pub mod meters;
pub mod pipewire;
mod profiler;

use crate::clock::{DriverRow, Settings};
use crate::graph::{GlobalId, GraphDelta};
use meters::MeterReading;

pub type RequestId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Handle {
    pub epoch: u64,
    pub id: GlobalId,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Connect {
        out: Handle,
        input: Handle,
        confirmed: bool,
        request: RequestId,
    },
    Disconnect {
        link: Handle,
        request: RequestId,
    },
    SetNodeVolume {
        node: Handle,
        channel_volumes: Vec<f32>,
        request: RequestId,
    },
    SetNodeMute {
        node: Handle,
        muted: bool,
        request: RequestId,
    },
    /// Filter-chain's keyed controls, addressed on its capture node.
    SetEffectGain {
        node: Handle,
        left: f32,
        right: f32,
        request: RequestId,
    },
    /// A8: keyed filter-chain controls on the capture node, in a single Props.params write.
    SetEffectParams {
        node: Handle,
        params: Vec<(String, f32)>,
        request: RequestId,
    },
    SetClock {
        key: ClockKey,
        value: Option<u32>,
        request: RequestId,
    },
    StartProfiler,
    StopProfiler,
    StartMeters(Vec<Handle>),
    StopMeters,
    Shutdown,
}

#[derive(Clone, Debug)]
pub enum Event {
    Delta(GraphDelta),
    CommandResult { request: RequestId, result: Result<(), String> },
    ClockDefaults { rate: u32, quantum: u32 },
    ClockSettings(Settings),
    ProfilerRows(Vec<DriverRow>),
    Connection(Connection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Connection {
    Up(String),
    Down(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockKey {
    Rate,
    Quantum,
}
impl ClockKey {
    pub fn name(self) -> &'static str {
        match self {
            Self::Rate => "clock.force-rate",
            Self::Quantum => "clock.force-quantum",
        }
    }
}

pub trait Backend {
    fn command(&mut self, command: Command);
    fn next_event(&mut self) -> Option<Event>;
    /// Returns only snapshots; implementations never poll PipeWire from the UI.
    fn meters(&self) -> Vec<MeterReading>;
}
