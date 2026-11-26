//! Snapshot-only Setups page; all writes flow through the app's setup messages.
use super::{End, RouteStatus, Setup, plan};
use crate::{
    app::{Msg, reason_text},
    graph::{Graph, identity::StableId},
    routes,
};
use aizome_kit::{Element, column, prelude::*, row};

fn endpoint(id: &StableId) -> String {
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

pub fn view(
    graph: &Graph,
    entries: &[(String, Result<Setup, String>)],
    selected: Option<&str>,
    active: Option<&Setup>,
    name_input: &str,
    keeper_error: Option<&str>,
) -> Element<Msg> {
    let mut list = column![caption("SETUPS")].spacing(8.).width(225.);
    for (id, result) in entries {
        match result {
            Ok(setup) => {
                let active = active.is_some_and(|a| a.id == *id);
                let state = [
                    active.then_some("Active"),
                    setup.keep_connected.then_some("kept"),
                    setup.exclusive.then_some("exclusive"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
                list = list.push(
                    column![
                        sidebar_item(Icon::ListFilter, setup.name.as_str(), selected == Some(id.as_str()))
                            .on_press(Msg::SetupSelect(id.clone()))
                            .id(format!("setup-{id}"))
                            .width(Fill),
                        text(if state.is_empty() { "Inactive" } else { &state }).size(10.).role(if active {
                            Role::Accent
                        } else {
                            Role::Text3
                        })
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
    list = list
        .push(spacer())
        .push(button("New setup from this page…").icon(Icon::Plus).on_press(Msg::SetupNew).id("setup-new").width(Fill));
    let mut editor = column![].spacing(12.).width(Fill).height(Fill);
    if let Some(setup) =
        entries.iter().find(|(id, _)| Some(id.as_str()) == selected).and_then(|(_, result)| result.as_ref().ok())
    {
        let is_active = active.is_some_and(|a| a.id == setup.id);
        editor = editor.push(
            row![
                text(setup.name.as_str()).size(18.).semibold(),
                spacer(),
                button(if is_active { "Deactivate" } else { "Activate" })
                    .on_press(if is_active { Msg::DeactivateSetup } else { Msg::ActivateSetup(setup.id.clone()) })
                    .id("setup-activate")
            ]
            .spacing(8.),
        );
        if !is_active && active.is_some() {
            editor = editor.push(
                text(format!(
                    "Activating this setup replaces the active setup “{}”; its links stay connected.",
                    active.unwrap().name
                ))
                .role(Role::Text2)
                .wrap(),
            );
        }
        editor = editor.push(
            row![
                text("Name").role(Role::Text2),
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
            ]
            .spacing(18.),
        );
        editor = editor.push(
            row![
                text(if let Some(error) = keeper_error {
                    format!("Keeper failed: {error}")
                } else if is_active && (setup.keep_connected || setup.exclusive) {
                    "Kept while Musubi is closed".into()
                } else {
                    "Not needed".into()
                })
                .role(if keeper_error.is_some() { Role::Danger } else { Role::Text2 }),
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
        let mut table = column![
            row![text("Out → in").width(Fill), text("Status").width(190.), text("Action").width(65.)].spacing(8.)
        ]
        .spacing(6.);
        for (index, (route, state)) in setup.routes.iter().zip(states.routes.iter()).enumerate() {
            let (label, role, tooltip) = status(graph, state);
            let mut state_text = text(label).role(role).size(11.);
            if let Some(candidates) = tooltip {
                state_text = state_text.tooltip(candidates);
            }
            table = table.push(
                row![
                    text(format!("{} → {}", endpoint(&route.output), endpoint(&route.input)))
                        .size(11.)
                        .wrap()
                        .width(Fill),
                    state_text.width(190.),
                    button("Unpin")
                        .compact()
                        .on_press(Msg::UnpinRoute(setup.id.clone(), index))
                        .id(format!("setup-unpin-{index}"))
                        .width(65.)
                ]
                .spacing(8.)
                .padding(4.),
            );
        }
        if setup.routes.is_empty() {
            table = table.push(text("No routes pinned yet").role(Role::Text3));
        }
        editor = editor.push(container(table).fill(Tone::Chrome).border(Edge::Line).padding(8.).width(Fill));
        editor = editor.push(spacer()).push(
            button("Delete setup…").icon(Icon::Trash).on_press(Msg::SetupDelete(setup.id.clone())).id("setup-delete"),
        );
    } else {
        editor = editor.push(
            column![
                text("No setup selected").size(18.),
                text("Save the current page’s links to make a setup.").role(Role::Text3)
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

pub fn new_sheet(name: &str) -> Element<Msg> {
    container(
        column![
            text("Save Setup").size(21.).semibold(),
            text("Save the current page’s connected links as routes.").role(Role::Text2),
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
