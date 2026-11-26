//! Persisted setup routes and read-only matching against the observed graph.
pub mod keeper;
pub mod page;
pub mod paths;
pub mod reconcile;
pub mod store;

use crate::graph::{
    GlobalId, Graph,
    identity::{Resolution, StableId},
    legal::{self, LinkLegality, Reason},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    pub output: StableId,
    pub input: StableId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Setup {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub routes: Vec<Route>,
    pub keep_connected: bool,
    pub exclusive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Active {
    pub id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Output,
    Input,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteStatus {
    Linked(GlobalId),
    Linkable(GlobalId, GlobalId),
    Missing { which: End },
    Ambiguous { which: End, candidates: Vec<GlobalId> },
    Illegal { reason: Reason },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupPlan {
    pub routes: Vec<RouteStatus>,
    pub foreign_links: Vec<GlobalId>,
}

/// Only uniquely resolved setup endpoints enter Exclusive's scope. A duplicate
/// identity never authorizes removal of links from either candidate port.
pub fn plan(graph: &Graph, setup: &Setup) -> SetupPlan {
    let mut routes = Vec::with_capacity(setup.routes.len());
    let mut named = HashSet::new();
    let mut expected = HashSet::new();
    let mut ambiguous = HashSet::new();
    for route in &setup.routes {
        let out = graph.resolve(&route.output);
        let input = graph.resolve(&route.input);
        for resolved in [&out, &input] {
            if let Resolution::Ambiguous(candidates) = resolved {
                ambiguous.extend(candidates.iter().copied());
            }
        }
        let status = match (out, input) {
            (Resolution::Ambiguous(candidates), _) => RouteStatus::Ambiguous { which: End::Output, candidates },
            (_, Resolution::Ambiguous(candidates)) => RouteStatus::Ambiguous { which: End::Input, candidates },
            (Resolution::Missing, _) => RouteStatus::Missing { which: End::Output },
            (_, Resolution::Missing) => RouteStatus::Missing { which: End::Input },
            (Resolution::Unique(out), Resolution::Unique(input)) => {
                expected.insert((out, input));
                let linked = graph.links_by_pair.get(&(out, input)).and_then(|ids| {
                    ids.iter()
                        .copied()
                        .filter(|id| {
                            graph
                                .links
                                .get(id)
                                .is_some_and(|link| !matches!(link.state, crate::graph::LinkState::Error(_)))
                        })
                        .min()
                });
                if let Some(id) = linked {
                    RouteStatus::Linked(id)
                } else {
                    match legal::can_link(graph, out, input) {
                        LinkLegality::Allowed => RouteStatus::Linkable(out, input),
                        LinkLegality::Forbidden(reason) | LinkLegality::NeedsConfirmation(reason) => {
                            RouteStatus::Illegal { reason }
                        }
                    }
                }
            }
        };
        match &status {
            RouteStatus::Linked(id) => {
                if let Some(link) = graph.links.get(id) {
                    named.extend([link.output, link.input]);
                }
            }
            RouteStatus::Linkable(out, input) => {
                named.extend([*out, *input]);
            }
            _ => {}
        }
        routes.push(status);
    }
    let mut foreign_links: Vec<_> = graph
        .links
        .values()
        .filter(|link| {
            (named.contains(&link.output) || named.contains(&link.input))
                && !ambiguous.contains(&link.output)
                && !ambiguous.contains(&link.input)
                && !expected.contains(&(link.output, link.input))
        })
        .map(|link| link.id)
        .collect();
    foreign_links.sort_unstable();
    SetupPlan { routes, foreign_links }
}
