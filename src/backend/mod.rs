//! Backend boundary: commands refer to IDs in one observed connection epoch.
pub mod fake;
pub mod pipewire;

use crate::graph::{GlobalId, GraphDelta};

pub type RequestId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Handle {
    pub epoch: u64,
    pub id: GlobalId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Connect { out: Handle, input: Handle, confirmed: bool, request: RequestId },
    Disconnect { link: Handle, request: RequestId },
    Shutdown,
}

#[derive(Clone, Debug)]
pub enum Event {
    Delta(GraphDelta),
    CommandResult { request: RequestId, result: Result<(), String> },
    ClockDefaults { rate: u32, quantum: u32 },
    Connection(Connection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Connection {
    Up(String),
    Down(String),
}

pub trait Backend {
    fn command(&mut self, command: Command);
    fn next_event(&mut self) -> Option<Event>;
}
