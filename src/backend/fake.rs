//! Deterministic registry simulation for graph, UI and undo tests.
use std::collections::{HashMap, VecDeque};
use std::{collections::HashSet, sync::Arc};

use super::{Backend, Command, Connection, Event};
use super::{
    Handle,
    meters::{MAX_METERS, MeterReading, MeterStore, Slot},
};
use crate::graph::{
    GlobalId, Graph, GraphDelta, LinkRecord, LinkState, Properties, Upsert, dump,
    legal::{LinkLegality, can_link},
};

pub struct FakeBackend {
    pub graph: Graph,
    events: VecDeque<Event>,
    next_id: GlobalId,
    next_failure: Option<String>,
    defer_next: bool,
    pending_connect: HashMap<GlobalId, u64>,
    pub commands: Vec<Command>,
    meters: MeterStore,
    meter_slots: HashMap<Handle, Arc<Slot>>,
}

impl FakeBackend {
    pub fn from_dump(json: &str, epoch: u64) -> Result<Self, serde_json::Error> {
        let delta = dump::parse(json, epoch)?;
        let next_id = delta.upserts.iter().map(Upsert::id).max().unwrap_or(0).saturating_add(1);
        let mut graph = Graph::default();
        graph.apply(delta.clone());
        Ok(Self {
            graph,
            events: VecDeque::from([
                Event::Connection(Connection::Up("1.6.9".into())),
                Event::ClockDefaults { rate: 48_000, quantum: 1024 },
                Event::Delta(delta),
            ]),
            next_id,
            next_failure: None,
            defer_next: false,
            pending_connect: HashMap::new(),
            meters: MeterStore::default(),
            meter_slots: HashMap::new(),
            commands: Vec::new(),
        })
    }

    /// Make the next mutating command fail, without changing the registry.
    pub fn fail_next(&mut self, reason: impl Into<String>) {
        self.next_failure = Some(reason.into());
    }
    /// Leave the next successful Connect negotiating until `transition_link`.
    pub fn defer_next_connect(&mut self) {
        self.defer_next = true;
    }

    pub fn transition_link(&mut self, id: GlobalId, state: LinkState) {
        let Some(mut link) = self.graph.links.get(&id).cloned() else { return };
        link.state = state.clone();
        self.publish(GraphDelta { epoch: self.graph.epoch, upserts: vec![Upsert::Link(link)], removals: vec![] });
        if let Some(request) = self.pending_connect.remove(&id) {
            match state {
                LinkState::Active => self.events.push_back(Event::CommandResult { request, result: Ok(()) }),
                LinkState::Error(reason) => {
                    self.events.push_back(Event::CommandResult { request, result: Err(reason) })
                }
                LinkState::Negotiating => {
                    self.pending_connect.insert(id, request);
                }
            }
        }
    }

    /// Simulate a registry disappearance and deliver its delta to the Harness.
    pub fn remove(&mut self, id: GlobalId) {
        self.publish(GraphDelta { epoch: self.graph.epoch, upserts: vec![], removals: vec![id] });
        self.meters.remove(Handle { epoch: self.graph.epoch, id });
        self.meter_slots.remove(&Handle { epoch: self.graph.epoch, id });
    }
    /// Simulate a new registry object after a profile switch.
    pub fn upsert(&mut self, object: Upsert) {
        self.publish(GraphDelta { epoch: self.graph.epoch, upserts: vec![object], removals: vec![] });
    }

    fn publish(&mut self, delta: GraphDelta) {
        self.graph.apply(delta.clone());
        self.events.push_back(Event::Delta(delta));
    }
    /// Script deterministic linear peaks without creating a PipeWire client.
    pub fn script_meter(&self, node: Handle, peaks: &[f32]) {
        if !self.meter_slots.contains_key(&node) {
            return;
        }
        let mut bytes = Vec::with_capacity(peaks.len() * 4);
        for peak in peaks {
            bytes.extend_from_slice(&peak.to_ne_bytes());
        }
        // Keep the existing slot: `replace` only occurs on StartMeters.
        self.script_meter_bytes(node, &bytes);
    }
    fn script_meter_bytes(&self, node: Handle, bytes: &[u8]) {
        if let Some(slot) = self.meter_slots.get(&node) {
            slot.process(bytes, bytes.len(), self.meters.elapsed_ns());
        }
    }
    /// Simulate a lost connection and discard all epoch-bound meter values.
    pub fn reset_epoch(&mut self, epoch: u64) {
        self.meters.clear();
        self.meter_slots.clear();
        self.publish(GraphDelta { epoch, ..GraphDelta::default() });
        self.events.push_back(Event::Connection(Connection::Down("fake disconnect".into())));
    }
}

impl Backend for FakeBackend {
    fn command(&mut self, command: Command) {
        self.commands.push(command.clone());
        let (request, result) = match command {
            Command::Connect { out, input, confirmed, request } => {
                let result = if out.epoch != self.graph.epoch || input.epoch != self.graph.epoch {
                    Err("stale connection epoch".into())
                } else if !self.graph.ports.contains_key(&out.id) || !self.graph.ports.contains_key(&input.id) {
                    Err("port not found".into())
                } else {
                    match can_link(&self.graph, out.id, input.id) {
                        LinkLegality::Allowed => Ok(()),
                        LinkLegality::NeedsConfirmation(_) if confirmed => Ok(()),
                        legality => Err(format!("illegal link: {legality:?}")),
                    }
                }
                .and_then(|()| self.next_failure.take().map_or(Ok(()), Err));
                if result.is_ok() {
                    let id = self.next_id;
                    self.next_id = self.next_id.saturating_add(1);
                    let props = Properties::from([
                        ("link.output.port".into(), out.id.to_string()),
                        ("link.input.port".into(), input.id.to_string()),
                        ("object.linger".into(), "true".into()),
                    ]);
                    let state = if self.defer_next { LinkState::Negotiating } else { LinkState::Active };
                    self.publish(GraphDelta {
                        epoch: self.graph.epoch,
                        upserts: vec![Upsert::Link(LinkRecord { id, output: out.id, input: input.id, state, props })],
                        removals: vec![],
                    });
                    if self.defer_next {
                        self.defer_next = false;
                        self.pending_connect.insert(id, request);
                        return;
                    }
                }
                (request, result)
            }
            Command::Disconnect { link, request } => {
                let result = if link.epoch != self.graph.epoch {
                    Err("stale connection epoch".into())
                } else if !self.graph.links.contains_key(&link.id) {
                    Err(format!("link {} not found", link.id))
                } else if let Some(reason) = self.next_failure.take() {
                    Err(reason)
                } else {
                    self.publish(GraphDelta { epoch: self.graph.epoch, upserts: vec![], removals: vec![link.id] });
                    Ok(())
                };
                (request, result)
            }
            Command::SetNodeVolume { node, channel_volumes, request } => {
                let result = if node.epoch != self.graph.epoch {
                    Err("stale connection epoch".into())
                } else if let Some(current) = self.graph.nodes.get(&node.id) {
                    if !current.native.writable
                        || current.native.channel_volumes.as_ref().is_none_or(|v| v.len() != channel_volumes.len())
                        || channel_volumes.iter().any(|v| !v.is_finite() || *v < 0.0)
                    {
                        Err("node has no writable channelVolumes Props".into())
                    } else if let Some(error) = self.next_failure.take() {
                        Err(error)
                    } else {
                        let mut changed = current.clone();
                        changed.native.channel_volumes = Some(channel_volumes);
                        self.publish(GraphDelta {
                            epoch: node.epoch,
                            upserts: vec![Upsert::Node(changed)],
                            removals: vec![],
                        });
                        Ok(())
                    }
                } else {
                    Err("node not found".into())
                };
                (request, result)
            }
            Command::SetNodeMute { node, muted, request } => {
                let result = if node.epoch != self.graph.epoch {
                    Err("stale connection epoch".into())
                } else if let Some(current) = self.graph.nodes.get(&node.id) {
                    if !current.native.writable || current.native.muted.is_none() {
                        Err("node has no writable mute".into())
                    } else if let Some(error) = self.next_failure.take() {
                        Err(error)
                    } else {
                        let mut changed = current.clone();
                        changed.native.muted = Some(muted);
                        self.publish(GraphDelta {
                            epoch: node.epoch,
                            upserts: vec![Upsert::Node(changed)],
                            removals: vec![],
                        });
                        Ok(())
                    }
                } else {
                    Err("node not found".into())
                };
                (request, result)
            }
            Command::SetEffectGain { request, .. } | Command::SetEffectParams { request, .. } => {
                (request, Err("effect DSP is not simulated by the fake backend".into()))
            }
            Command::StartMeters(nodes) => {
                self.meters.clear();
                self.meter_slots.clear();
                let mut paused = HashSet::new();
                for node in nodes {
                    if node.epoch != self.graph.epoch || !self.graph.nodes.contains_key(&node.id) {
                        continue;
                    }
                    if self.meter_slots.contains_key(&node) || paused.contains(&node) {
                        continue;
                    }
                    if self.meter_slots.len() >= MAX_METERS {
                        paused.insert(node);
                        continue;
                    }
                    let channels = self.graph.nodes[&node.id]
                        .props
                        .get("audio.channels")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(2);
                    self.meter_slots.insert(node, Arc::new(Slot::new(channels)));
                }
                self.meters.replace(self.meter_slots.clone(), paused);
                return;
            }
            Command::StopMeters => {
                self.meter_slots.clear();
                self.meters.clear();
                return;
            }
            Command::Shutdown => {
                self.events.push_back(Event::Connection(Connection::Down("shutdown".into())));
                return;
            }
        };
        self.events.push_back(Event::CommandResult { request, result });
    }

    fn next_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }
    fn meters(&self) -> Vec<MeterReading> {
        self.meters.readings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Handle;
    #[test]
    fn connects_disconnects_and_reports_failure_without_mutation() {
        let mut fake = FakeBackend::from_dump(include_str!("../../tests/fixtures/studio.json"), 4).unwrap();
        assert!(matches!(fake.next_event(), Some(Event::Connection(Connection::Up(_)))));
        assert!(matches!(fake.next_event(), Some(Event::ClockDefaults { rate: 48_000, quantum: 1024 })));
        assert!(matches!(fake.next_event(), Some(Event::Delta(_))));
        let out = Handle { epoch: 4, id: 100 };
        let input = Handle { epoch: 4, id: 104 };
        fake.command(Command::Connect { out, input, confirmed: false, request: 1 });
        let id = match fake.next_event().unwrap() {
            Event::Delta(delta) => match &delta.upserts[0] {
                Upsert::Link(link) => link.id,
                _ => panic!("not link"),
            },
            _ => panic!("missing delta"),
        };
        assert!(matches!(fake.next_event(), Some(Event::CommandResult { request: 1, result: Ok(()) })));
        assert_eq!(fake.graph.links_by_pair[&(100, 104)], vec![id]);
        fake.fail_next("factory denied");
        fake.command(Command::Disconnect { link: Handle { epoch: 4, id }, request: 2 });
        assert!(
            matches!(fake.next_event(), Some(Event::CommandResult { request: 2, result: Err(e) }) if e == "factory denied")
        );
        assert!(fake.graph.links.contains_key(&id));
        fake.command(Command::Disconnect { link: Handle { epoch: 4, id }, request: 3 });
        assert!(matches!(fake.next_event(), Some(Event::Delta(delta)) if delta.removals == vec![id]));
        assert!(matches!(fake.next_event(), Some(Event::CommandResult { request: 3, result: Ok(()) })));
        assert!(!fake.graph.links.contains_key(&id));
    }
    #[test]
    fn rejects_invalid_handles_and_requires_midi_confirmation() {
        let mut fake = FakeBackend::from_dump(include_str!("../../tests/fixtures/midi.json"), 9).unwrap();
        fake.next_event();
        fake.next_event();
        fake.next_event();
        let port = |id| Handle { epoch: 9, id };
        fake.command(Command::Connect { out: port(20), input: port(41), confirmed: false, request: 1 });
        assert!(
            matches!(fake.next_event(), Some(Event::CommandResult { result: Err(e), .. }) if e.contains("NeedsConfirmation"))
        );
        fake.command(Command::Connect { out: port(20), input: port(41), confirmed: true, request: 2 });
        let id = match fake.next_event() {
            Some(Event::Delta(delta)) => match &delta.upserts[0] {
                Upsert::Link(link) => link.id,
                _ => panic!("not a link"),
            },
            _ => panic!("no link"),
        };
        assert!(matches!(fake.next_event(), Some(Event::CommandResult { request: 2, result: Ok(()) })));
        for (out, input, reason) in [
            (port(20), port(21), "WrongDirection"),
            (port(20), port(999), "port not found"),
            (Handle { epoch: 8, id: 20 }, port(41), "stale connection epoch"),
        ] {
            fake.command(Command::Connect { out, input, confirmed: true, request: 3 });
            assert!(
                matches!(fake.next_event(), Some(Event::CommandResult { result: Err(e), .. }) if e.contains(reason))
            );
        }
        fake.command(Command::Disconnect { link: Handle { epoch: 8, id }, request: 4 });
        assert!(
            matches!(fake.next_event(), Some(Event::CommandResult { result: Err(e), .. }) if e.contains("stale connection epoch"))
        );
        fake.command(Command::Disconnect { link: port(9999), request: 5 });
        assert!(
            matches!(fake.next_event(), Some(Event::CommandResult { result: Err(e), .. }) if e.contains("not found"))
        );
        assert!(fake.graph.links.contains_key(&id));
    }
    #[test]
    fn meter_cap_script_stop_and_epoch_reset() {
        let mut fake = FakeBackend::from_dump(include_str!("../../tests/fixtures/studio.json"), 4).unwrap();
        let sample = fake.graph.nodes.values().next().unwrap().clone();
        let nodes: Vec<_> = (1000..1026)
            .map(|id| {
                let node = crate::graph::NodeRecord { id, ..sample.clone() };
                fake.graph.apply(GraphDelta { epoch: 4, upserts: vec![Upsert::Node(node)], removals: vec![] });
                Handle { epoch: 4, id }
            })
            .collect();
        fake.command(Command::StartMeters(nodes.clone()));
        let readings = fake.meters();
        assert_eq!(readings.iter().filter(|m| !m.paused).count(), 24);
        assert_eq!(readings.iter().filter(|m| m.paused).count(), 2);
        let active = readings.iter().find(|m| !m.paused).unwrap().node;
        fake.script_meter(active, &[0.8, 0.4]);
        assert!(fake.meters().iter().find(|m| m.node == active).unwrap().peaks[0] > 0.7);
        fake.command(Command::StopMeters);
        assert!(fake.meters().is_empty());
        fake.command(Command::StartMeters(vec![active]));
        fake.reset_epoch(5);
        assert!(fake.meters().is_empty());
        fake.command(Command::StartMeters(vec![active]));
        assert!(fake.meters().is_empty());
    }
}
