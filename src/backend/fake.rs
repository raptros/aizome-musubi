//! Deterministic registry simulation for graph, UI and undo tests.
use std::collections::VecDeque;

use super::{Backend, Command, Connection, Event};
use crate::graph::{dump, legal::{can_link, LinkLegality}, GlobalId, Graph, GraphDelta, LinkRecord, Properties, Upsert};

pub struct FakeBackend {
    pub graph: Graph,
    events: VecDeque<Event>,
    next_id: GlobalId,
    next_failure: Option<String>,
}

impl FakeBackend {
    pub fn from_dump(json: &str, epoch: u64) -> Result<Self, serde_json::Error> {
        let delta = dump::parse(json, epoch)?;
        let next_id = delta.upserts.iter().map(Upsert::id).max().unwrap_or(0).saturating_add(1);
        let mut graph = Graph::default();
        graph.apply(delta.clone());
        Ok(Self { graph, events: VecDeque::from([Event::Connection(Connection::Up("fake PipeWire".into())), Event::Delta(delta)]), next_id, next_failure: None })
    }

    /// Make the next mutating command fail, without changing the registry.
    pub fn fail_next(&mut self, reason: impl Into<String>) { self.next_failure = Some(reason.into()); }

    fn publish(&mut self, delta: GraphDelta) {
        self.graph.apply(delta.clone());
        self.events.push_back(Event::Delta(delta));
    }
}

impl Backend for FakeBackend {
    fn command(&mut self, command: Command) {
        let (request, result) = match command {
            Command::Connect { out, input, request } => {
                let result = if out.epoch != self.graph.epoch || input.epoch != self.graph.epoch {
                    Err("stale connection epoch".into())
                } else if let Some(reason) = self.next_failure.take() { Err(reason) }
                else if !matches!(can_link(&self.graph, out.id, input.id), LinkLegality::Allowed | LinkLegality::NeedsConfirmation(_)) {
                    Err(format!("illegal link: {:?}", can_link(&self.graph, out.id, input.id)))
                } else {
                    let id = self.next_id;
                    self.next_id = self.next_id.saturating_add(1);
                    let props = Properties::from([
                        ("link.output.port".into(), out.id.to_string()),
                        ("link.input.port".into(), input.id.to_string()),
                        ("object.linger".into(), "true".into()),
                    ]);
                    self.publish(GraphDelta { epoch: self.graph.epoch, upserts: vec![Upsert::Link(LinkRecord { id, output: out.id, input: input.id, props })], removals: vec![] });
                    Ok(())
                };
                (request, result)
            }
            Command::Disconnect { link, request } => {
                let result = if link.epoch != self.graph.epoch { Err("stale connection epoch".into()) }
                else if let Some(reason) = self.next_failure.take() { Err(reason) }
                else if !self.graph.links.contains_key(&link.id) { Err(format!("link {} not found", link.id)) }
                else {
                    self.publish(GraphDelta { epoch: self.graph.epoch, upserts: vec![], removals: vec![link.id] });
                    Ok(())
                };
                (request, result)
            }
            Command::Shutdown => {
                self.events.push_back(Event::Connection(Connection::Down("shutdown".into())));
                return;
            }
        };
        self.events.push_back(Event::CommandResult { request, result });
    }

    fn next_event(&mut self) -> Option<Event> { self.events.pop_front() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Handle;
    #[test]
    fn connects_disconnects_and_reports_failure_without_mutation() {
        let mut fake = FakeBackend::from_dump(include_str!("../../tests/fixtures/studio.json"), 4).unwrap();
        assert!(matches!(fake.next_event(), Some(Event::Connection(Connection::Up(_)))));
        assert!(matches!(fake.next_event(), Some(Event::Delta(_))));
        let out = Handle { epoch: 4, id: 100 }; let input = Handle { epoch: 4, id: 104 };
        fake.command(Command::Connect { out, input, request: 1 });
        let id = match fake.next_event().unwrap() { Event::Delta(delta) => match &delta.upserts[0] { Upsert::Link(link) => link.id, _ => panic!("not link") }, _ => panic!("missing delta") };
        assert!(matches!(fake.next_event(), Some(Event::CommandResult { request: 1, result: Ok(()) })));
        assert_eq!(fake.graph.links_by_pair[&(100, 104)], vec![id]);
        fake.fail_next("factory denied");
        fake.command(Command::Disconnect { link: Handle { epoch: 4, id }, request: 2 });
        assert!(matches!(fake.next_event(), Some(Event::CommandResult { request: 2, result: Err(e) }) if e == "factory denied"));
        assert!(fake.graph.links.contains_key(&id));
        fake.command(Command::Disconnect { link: Handle { epoch: 4, id }, request: 3 });
        assert!(matches!(fake.next_event(), Some(Event::Delta(delta)) if delta.removals == vec![id]));
        assert!(matches!(fake.next_event(), Some(Event::CommandResult { request: 3, result: Ok(()) })));
        assert!(!fake.graph.links.contains_key(&id));
    }
}
