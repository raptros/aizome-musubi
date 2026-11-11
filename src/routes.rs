//! Accessible list of physical links and searchable route endpoints.
use std::rc::Rc;

use aizome_kit::{column, prelude::*, row};

use crate::{
    app::Msg,
    graph::{Direction, GlobalId, Graph, classify::PortKind, group::port_label},
    matrix::Layout,
};

#[derive(Clone, Debug)]
pub struct Route {
    pub link: GlobalId,
    pub from: GlobalId,
    pub to: GlobalId,
    pub source: String,
    pub target: String,
    pub state: String,
    pub kind: String,
}

pub fn port_name(graph: &Graph, id: GlobalId) -> String {
    let Some(port) = graph.ports.get(&id) else { return format!("Port {id}") };
    let group = graph.groups(port.direction).into_iter().find(|group| group.ports.contains(&id));
    format!("{} · {}", group.map_or_else(|| "Other".to_owned(), |group| group.label), port_label(port))
}

pub fn routes(
    graph: &Graph,
    layout: &Layout,
    include_kind: bool,
    pending: &[(GlobalId, GlobalId)],
    errors: &[(GlobalId, GlobalId, String)],
) -> Vec<Route> {
    let mut rows: Vec<Route> = graph
        .links
        .values()
        .filter(|link| layout.cell_for_pair((link.output, link.input)).is_some())
        .map(|link| Route {
            link: link.id,
            from: link.output,
            to: link.input,
            source: port_name(graph, link.output),
            target: port_name(graph, link.input),
            state: "Connected".into(),
            kind: kind_label(graph, link.output, link.input, include_kind),
        })
        .collect();
    for &(from, to) in pending {
        if layout.cell_for_pair((from, to)).is_some() && !rows.iter().any(|row| row.from == from && row.to == to) {
            rows.push(Route {
                link: 0,
                from,
                to,
                source: port_name(graph, from),
                target: port_name(graph, to),
                state: "Asked".into(),
                kind: kind_label(graph, from, to, include_kind),
            });
        }
    }
    for &(from, to, ref error) in errors {
        if layout.cell_for_pair((from, to)).is_some() && !rows.iter().any(|row| row.from == from && row.to == to) {
            rows.push(Route {
                link: 0,
                from,
                to,
                source: port_name(graph, from),
                target: port_name(graph, to),
                state: format!("Error: {error}"),
                kind: kind_label(graph, from, to, include_kind),
            });
        }
    }
    rows
}

fn kind_label(graph: &Graph, from: GlobalId, to: GlobalId, show: bool) -> String {
    if !show {
        return String::new();
    }
    let label = |id| match graph.kind(id) {
        Some(PortKind::Midi) => "1.0",
        Some(PortKind::Ump) => "2.0",
        _ => "Other",
    };
    format!("{} → {}", label(from), label(to))
}

pub fn sort(rows: &mut [Route], column: usize, ascending: bool) {
    rows.sort_by(|a, b| {
        let order = match column {
            0 => a.source.cmp(&b.source),
            1 => a.target.cmp(&b.target),
            2 => a.state.cmp(&b.state),
            _ => a.kind.cmp(&b.kind),
        }
        .then_with(|| a.from.cmp(&b.from))
        .then_with(|| a.to.cmp(&b.to));
        if ascending { order } else { order.reverse() }
    });
}

pub fn view(rows: Vec<Route>, selected: Vec<usize>, sorted: (usize, bool), show_kind: bool) -> Element<Msg> {
    let mut columns = vec![
        Column::new("From").width(260.).sortable(),
        Column::new("To").width(260.).sortable(),
        Column::new("State").width(145.).sortable(),
    ];
    if show_kind {
        columns.push(Column::new("Kind").width(110.).sortable());
    }
    let rows = Rc::new(rows);
    let values = rows.clone();
    let names = rows.clone();
    table(columns, rows.len(), move |r, c| {
        let route = &values[r];
        text(match c {
            0 => route.source.as_str(),
            1 => route.target.as_str(),
            2 => route.state.as_str(),
            _ => route.kind.as_str(),
        })
    })
    .selected(selected)
    .on_select(Msg::SelectRoutes)
    .on_activate(Msg::InspectRoute)
    .multiple()
    .type_select(move |i| names[i].source.clone())
    .sorted(sorted.0, sorted.1)
    .on_sort(|column, ascending| Msg::SortRoutes((column, ascending)))
    .id("routes-table")
    .into()
}

pub fn endpoints(graph: &Graph, layout: &Layout, from: bool, query: &str) -> Vec<(GlobalId, String)> {
    let lines = if from == layout.swapped { &layout.col_lines } else { &layout.row_lines };
    let mut result = Vec::new();
    let query = query.to_lowercase();
    for line in lines {
        for &id in &line.ports {
            if graph
                .ports
                .get(&id)
                .is_some_and(|p| p.direction == if from { Direction::Output } else { Direction::Input })
            {
                let name = port_name(graph, id);
                if name.to_lowercase().contains(&query) {
                    result.push((id, name));
                }
            }
        }
    }
    result
}

pub fn sheet(
    from: Vec<(GlobalId, String)>,
    to: Vec<(GlobalId, String)>,
    from_query: &str,
    to_query: &str,
    from_selected: Option<GlobalId>,
    to_selected: Option<GlobalId>,
) -> Element<Msg> {
    let source = Rc::new(from);
    let target = Rc::new(to);
    let sources = source.clone();
    let targets = target.clone();
    container(
        column![
            text("Add Route").size(21.).semibold(),
            row![
                column![
                    text("From").semibold(),
                    search_field(from_query)
                        .placeholder("Search sources")
                        .on_change(Msg::RouteFromFilter)
                        .id("route-from-search"),
                    virtual_list(source.len(), 28., move |i| text(sources[i].1.clone()))
                        .selected(
                            source
                                .iter()
                                .enumerate()
                                .filter_map(|(i, (id, _))| (Some(*id) == from_selected).then_some(i))
                        )
                        .on_select({
                            let source = source.clone();
                            move |rows| Msg::RouteFromSelected(rows.first().map(|&i| source[i].0))
                        })
                        .on_activate({
                            let source = source.clone();
                            move |i| Msg::RouteFromSelected(Some(source[i].0))
                        })
                        .id("route-from-list")
                ]
                .spacing(8.)
                .width(Fill),
                column![
                    text("To").semibold(),
                    search_field(to_query)
                        .placeholder("Search destinations")
                        .on_change(Msg::RouteToFilter)
                        .id("route-to-search"),
                    virtual_list(target.len(), 28., move |i| text(targets[i].1.clone()))
                        .selected(
                            target
                                .iter()
                                .enumerate()
                                .filter_map(|(i, (id, _))| (Some(*id) == to_selected).then_some(i))
                        )
                        .on_select({
                            let target = target.clone();
                            move |rows| Msg::RouteToSelected(rows.first().map(|&i| target[i].0))
                        })
                        .on_activate({
                            let target = target.clone();
                            move |i| Msg::RouteToSelected(Some(target[i].0))
                        })
                        .id("route-to-list")
                ]
                .spacing(8.)
                .width(Fill),
            ]
            .spacing(12.)
            .height(Fill),
            row![
                spacer(),
                button("Cancel").cancel().on_press(Msg::CloseRoute),
                button("Connect")
                    .default()
                    .enabled(from_selected.is_some() && to_selected.is_some())
                    .on_press(Msg::ConnectRoute)
            ]
            .spacing(8.),
        ]
        .spacing(12.)
        .padding(18.),
    )
    .width(640.)
    .height(480.)
    .into()
}
