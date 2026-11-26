//! Stateless decision step with explicit, caller-owned in-flight and retry state.
use super::{Route, RouteStatus, Setup, plan};
use crate::graph::{GlobalId, Graph, LinkState, identity::StableId};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    CreateLink(GlobalId, GlobalId),
    RemoveLink(GlobalId),
}

#[derive(Default)]
pub struct State {
    creates: HashMap<RouteKey, (GlobalId, GlobalId)>,
    removes: HashMap<GlobalId, PairKey>,
    attempts: HashMap<PairKey, Vec<Instant>>,
    conflicted: HashSet<PairKey>,
    failures: HashMap<RouteKey, (u32, Instant)>,
}

type RouteKey = (StableId, StableId);
type PairKey = (StableId, StableId);

pub struct Step {
    pub state: State,
    pub actions: Vec<Action>,
    pub notices: Vec<String>,
}

/// A failed create is retried after 1, 5, then 30 seconds (30 thereafter).
pub fn create_failed(state: &mut State, route: &Route, now: Instant) {
    let failure = state.failures.entry((route.output.clone(), route.input.clone())).or_insert((0, now));
    failure.0 = failure.0.saturating_add(1);
    failure.1 = now
        + Duration::from_secs(match failure.0 {
            1 => 1,
            2 => 5,
            _ => 30,
        });
    state.creates.remove(&(route.output.clone(), route.input.clone()));
}

pub fn create_pending(state: &mut State, route: &Route, pair: (GlobalId, GlobalId)) {
    state.creates.insert((route.output.clone(), route.input.clone()), pair);
}
pub fn next_retry(state: &State, now: Instant) -> Option<Duration> {
    state
        .failures
        .iter()
        .filter(|(key, (_, until))| !state.creates.contains_key(*key) && *until > now)
        .map(|(_, (_, until))| until.saturating_duration_since(now))
        .min()
}

pub fn remove_failed(state: &mut State, id: GlobalId) {
    state.removes.remove(&id);
}

/// Pure with respect to the graph and setup: the returned state is the only memory of requests.
pub fn step(graph: &Graph, setup: &Setup, mut state: State, now: Instant) -> Step {
    let plan = plan(graph, setup);
    let mut actions = Vec::new();
    let mut notices = Vec::new();
    state.creates.retain(|key, pair| {
        graph.resolve(&key.0) == crate::graph::identity::Resolution::Unique(pair.0)
            && graph.resolve(&key.1) == crate::graph::identity::Resolution::Unique(pair.1)
            && !graph.links_by_pair.get(pair).is_some_and(|ids| {
                ids.iter().any(|id| graph.links.get(id).is_some_and(|link| !matches!(link.state, LinkState::Error(_))))
            })
    });
    state.removes.retain(|id, _| graph.links.contains_key(id));
    for (route, status) in setup.routes.iter().zip(plan.routes) {
        let key = (route.output.clone(), route.input.clone());
        match status {
            RouteStatus::Linked(_) => {
                state.failures.remove(&key);
            }
            RouteStatus::Linkable(out, input) if setup.keep_connected => {
                if !state.creates.contains_key(&key) && !state.failures.get(&key).is_some_and(|(_, until)| now < *until)
                {
                    state.creates.insert(key, (out, input));
                    actions.push(Action::CreateLink(out, input));
                }
            }
            _ => {}
        }
    }
    if setup.exclusive {
        for id in plan.foreign_links {
            if state.removes.contains_key(&id) {
                continue;
            }
            let Some(link) = graph.links.get(&id) else { continue };
            let (Some(out), Some(input)) = (
                graph.ports.get(&link.output).and_then(|p| graph.stable_port_id(p)),
                graph.ports.get(&link.input).and_then(|p| graph.stable_port_id(p)),
            ) else {
                continue;
            };
            let pair = (out, input);
            if state.conflicted.contains(&pair) {
                continue;
            }
            let attempts = state.attempts.entry(pair.clone()).or_default();
            attempts.retain(|time| now.saturating_duration_since(*time) < Duration::from_secs(60));
            if attempts.len() >= 3 {
                notices.push("WirePlumber's policy keeps re-linking this setup port; Exclusive stopped removing that link until re-activation or toggling Exclusive.".into());
                state.conflicted.insert(pair);
            } else {
                attempts.push(now);
                state.removes.insert(id, pair);
                actions.push(Action::RemoveLink(id));
            }
        }
    }
    Step { state, actions, notices }
}
