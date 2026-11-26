//! Snapshot-only Setups page; all writes flow through the app's setup messages.
use super::{End, Route, RouteStatus, Setup, plan};
use crate::{
    app::{Msg, Page, reason_text},
    graph::{
        Graph,
        identity::{Resolution, StableId},
    },
    routes,
};
use aizome_kit::{Element, column, prelude::*, row};

/// The stored identity in words, for ports that were never seen with a saved label.
fn identity_text(id: &StableId) -> String {
    match id {
        StableId::Port { device_name, application_name, sequencer_client_name, node_name, port_name, .. } => format!(
            "{} · {}",
            device_name
                .as_ref()
                .or(application_name.as_ref())
                .or(sequencer_client_name.as_ref())
                .or(node_name.as_ref())
                .map(String::as_str)
                .unwrap_or("Unknown"),
            port_name.as_deref().unwrap_or("port")
        ),
        StableId::Group { .. } => "Unknown endpoint".into(),
    }
}

/// The live port's name when it resolves, else the label saved with the route.
fn end_label(graph: &Graph, id: &StableId, saved: Option<&String>) -> String {
    match graph.resolve(id) {
        Resolution::Unique(port) => routes::port_name(graph, port),
        _ => saved.cloned().unwrap_or_else(|| identity_text(id)),
    }
}

fn status(graph: &Graph, state: &RouteStatus) -> (String, Role, Option<String>) {
    match state {
        RouteStatus::Linked(_) => ("Connected".into(), Role::Success, None),
        RouteStatus::Linkable(_, _) => ("Ready".into(), Role::Text2, None),
        RouteStatus::Missing { which } => {
            (format!("Waiting for {}", if *which == End::Output { "output" } else { "input" }), Role::Text3, None)
        }
        RouteStatus::Ambiguous { candidates, .. } => (
            format!("Ambiguous: {} matches", candidates.len()),
            Role::Danger,
            Some(
                candidates
                    .iter()
                    .map(|&id| format!("{} (port {id})", routes::port_name(graph, id)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ),
        RouteStatus::Illegal { reason } => (format!("Not allowed: {}", reason_text(*reason)), Role::Danger, None),
    }
}

pub fn routes_label(count: usize) -> String {
    format!("{count} {}", if count == 1 { "route" } else { "routes" })
}

/// Rows sorted by label, keeping each route's stored index for Unpin.
fn route_rows<'a>(graph: &Graph, setup: &'a Setup) -> Vec<(usize, &'a Route, String, String)> {
    let mut rows: Vec<_> = setup
        .routes
        .iter()
        .enumerate()
        .map(|(index, route)| {
            (
                index,
                route,
                end_label(graph, &route.output, route.output_label.as_ref()),
                end_label(graph, &route.input, route.input_label.as_ref()),
            )
        })
        .collect();
    rows.sort_by(|a, b| (&a.2, &a.3).cmp(&(&b.2, &b.3)));
    // Different identities can share a name (two missing devices): number them.
    let mut seen = std::collections::HashMap::<(String, String), usize>::new();
    for row in &mut rows {
        let n = seen.entry((row.2.clone(), row.3.clone())).or_insert(0);
        *n += 1;
        if *n > 1 {
            row.2 = format!("{} ({n})", row.2);
        }
    }
    rows
}

pub struct View<'a> {
    pub graph: &'a Graph,
    pub entries: &'a [(String, Result<Setup, String>)],
    pub selected: Option<&'a str>,
    pub active: Option<&'a Setup>,
    pub name_input: &'a str,
    pub keeper_error: Option<&'a str>,
    /// The patchbay page whose links a new setup saves.
    pub new_page: Page,
}

pub fn view(v: View) -> Element<Msg> {
    let View { graph, entries, selected, active, name_input, keeper_error, new_page } = v;
    let mut list = column![caption("SETUPS")].spacing(8.).width(225.);
    for (id, result) in entries {
        match result {
            Ok(setup) => {
                let active = active.is_some_and(|a| a.id == *id);
                let state = [
                    Some(if active { "Active".to_owned() } else { "Inactive".to_owned() }),
                    setup.keep_connected.then(|| "kept".to_owned()),
                    setup.exclusive.then(|| "exclusive".to_owned()),
                    Some(routes_label(setup.routes.len())),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
                let (dot, role) = if active { ("●", Role::Success) } else { ("○", Role::Text3) };
                list = list.push(
                    column![
                        row![
                            sidebar_item(Icon::ListFilter, setup.name.as_str(), selected == Some(id.as_str()))
                                .on_press(Msg::SetupSelect(id.clone()))
                                .id(format!("setup-{id}"))
                                .width(Fill),
                            text(dot).role(role)
                        ]
                        .spacing(3.)
                        .width(Fill),
                        text(state).size(10.).role(Role::Text3).wrap()
                    ]
                    .spacing(3.)
                    .padding([3., 4.])
                    .width(Fill),
                );
            }
            Err(error) => {
                list = list.push(
                    column![
                        text(format!("{id} · Damaged")).role(Role::Danger).wrap(),
                        text(error.as_str()).size(10.).role(Role::Danger).wrap(),
                        button("Remove…").on_press(Msg::SetupDelete(id.clone())).id(format!("setup-remove-{id}"))
                    ]
                    .spacing(4.)
                    .padding(4.),
                )
            }
        }
    }
    if entries.is_empty() {
        list = list.push(text("No setups yet").size(12.).role(Role::Text3));
    }
    list = list.push(spacer()).push(
        button(format!("New setup from {}…", new_page.title()))
            .icon(Icon::Plus)
            .on_press(Msg::SetupNew)
            .id("setup-new")
            .width(Fill),
    );
    let mut editor = column![].spacing(10.).width(Fill).height(Fill);
    if let Some(setup) =
        entries.iter().find(|(id, _)| Some(id.as_str()) == selected).and_then(|(_, result)| result.as_ref().ok())
    {
        let is_active = active.is_some_and(|a| a.id == setup.id);
        editor = editor.push(
            row![
                column![
                    text(setup.name.as_str()).size(18.).semibold(),
                    text("Links Musubi restores when this setup is active.").size(11.).role(Role::Text3)
                ]
                .spacing(3.),
                spacer(),
                button(if is_active { "Deactivate" } else { "Activate" })
                    .on_press(if is_active { Msg::DeactivateSetup } else { Msg::ActivateSetup(setup.id.clone()) })
                    .id("setup-activate")
            ]
            .spacing(8.),
        );
        if let Some(current) = active.filter(|_| !is_active) {
            editor = editor.push(
                text(format!(
                    "Activating this setup replaces the active setup “{}”; its links stay connected.",
                    current.name
                ))
                .role(Role::Text2)
                .wrap(),
            );
        }
        editor = editor.push(
            row![
                text("Name").size(11.).role(Role::Text2),
                text_field(name_input)
                    .on_change(Msg::SetupNameInput)
                    .on_submit(Msg::SetupNameCommit)
                    .on_blur(Msg::SetupNameCommit)
                    .id("setup-name")
                    .width(240.)
            ]
            .spacing(8.),
        );
        editor = editor.push(
            row![
                checkbox("Keep connected", setup.keep_connected).on_toggle(Msg::SetupKeep).id("setup-keep"),
                checkbox("Exclusive", setup.exclusive).on_toggle(Msg::SetupExclusive).id("setup-exclusive"),
                spacer(),
                button("Delete setup…")
                    .icon(Icon::Trash)
                    .on_press(Msg::SetupDelete(setup.id.clone()))
                    .id("setup-delete")
            ]
            .spacing(18.),
        );
        let enforced = setup.keep_connected || setup.exclusive;
        editor = editor.push(
            row![
                text(if let Some(error) = keeper_error {
                    format!("Keeper failed: {error}")
                } else if is_active && enforced {
                    "Keeper: on — holds these links while Musubi is closed.".into()
                } else if enforced {
                    "Keeper: starts when this setup is active.".into()
                } else {
                    "Keeper: off — turn on Keep connected to hold these links while Musubi is closed.".into()
                })
                .role(if keeper_error.is_some() { Role::Danger } else { Role::Text2 })
                .wrap(),
                if keeper_error.is_some() {
                    Element::from(button("Retry").on_press(Msg::SetupKeeperRetry).id("setup-keeper-retry"))
                } else {
                    spacer().width(0.).into()
                }
            ]
            .spacing(8.),
        );
        editor = editor.push(caption("ROUTES"));
        let states = plan(graph, setup);
        const STATUS: f32 = 170.;
        const ACTION: f32 = 64.;
        let mut table = column![
            row![text("Out").width(Fill), text("In").width(Fill), text("Status").width(STATUS), text("").width(ACTION)]
                .spacing(8.)
                .padding([0., 4.])
        ]
        .spacing(4.);
        for (index, route, out, input) in route_rows(graph, setup) {
            let (label, role, candidates) = status(graph, &states.routes[index]);
            let mut state_text = text(label).role(role).wrap().width(STATUS);
            if let Some(candidates) = candidates {
                state_text = state_text.tooltip(candidates);
            }
            table = table.push(
                row![
                    text(out).wrap().tooltip(identity_text(&route.output)).width(Fill),
                    text(input).wrap().tooltip(identity_text(&route.input)).width(Fill),
                    state_text,
                    button("Unpin")
                        .compact()
                        .on_press(Msg::UnpinRoute(setup.id.clone(), index))
                        .id(format!("setup-unpin-{index}"))
                        .width(ACTION)
                ]
                .spacing(8.)
                .padding(4.),
            );
        }
        if setup.routes.is_empty() {
            table = table.push(
                container(
                    text("No routes yet. Add a link from the matrix’s context menu, or save a new setup.")
                        .role(Role::Text3)
                        .wrap(),
                )
                .padding(4.),
            );
        }
        editor = editor.push(container(table).fill(Tone::Chrome).border(Edge::Line).padding(8.).width(Fill));
    } else {
        editor = editor.push(
            column![
                text(if entries.is_empty() { "No setups yet" } else { "No setup selected" }).size(18.),
                text(if entries.is_empty() {
                    "A setup remembers links so Musubi can restore them. Connect things on Audio, MIDI or Video, \
                     then save them as a setup."
                } else {
                    "Select a setup, or save a new one."
                })
                .role(Role::Text3)
                .wrap()
            ]
            .spacing(8.),
        );
    }
    row![
        container(list).padding(10.).fill(Tone::Sunk).width(245.).height(Fill),
        container(editor).padding(12.).width(Fill).height(Fill)
    ]
    .spacing(0.)
    .height(Fill)
    .into()
}

pub fn new_sheet(name: &str, page: Page) -> Element<Msg> {
    container(
        column![
            text("Save Setup").size(21.).semibold(),
            text(format!("Save the connected {} links as routes.", page.title())).role(Role::Text2),
            text("Name"),
            text_field(name)
                .on_change(Msg::SetupNewName)
                .on_submit(Msg::SetupNewCommit)
                .id("setup-new-name")
                .width(Fill),
            row![
                spacer(),
                button("Cancel").cancel().on_press(Msg::SetupNewCancel),
                button("Save Setup")
                    .default()
                    .enabled(!name.trim().is_empty())
                    .on_press(Msg::SetupNewCommit)
                    .id("setup-new-save")
            ]
            .spacing(8.)
        ]
        .spacing(12.)
        .padding(18.),
    )
    .width(440.)
    .height(210.)
    .into()
}
