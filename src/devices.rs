//! Card details are projections of the Pulse snapshot and current graph, never pending writes.
use crate::{
    app::Msg,
    graph::{Graph, NodeRecord, classify::PortKind},
    mixer::model::StripKind,
};
use aizome_kit::{Element, column, prelude::*, row};
use aizome_sys::audio::{AudioDevice, AudioState, Card, DeviceState};

pub fn card_nodes<'a>(graph: &'a Graph, card: &Card) -> Vec<&'a NodeRecord> {
    let mut nodes: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| {
            n.device_id.is_some_and(|id| {
                graph.devices.get(&id).is_some_and(|device| device.props.get("device.name") == Some(&card.name))
            })
        })
        .collect();
    nodes.sort_by(|a, b| {
        a.props
            .get("node.description")
            .or_else(|| a.props.get("node.name"))
            .cmp(&b.props.get("node.description").or_else(|| b.props.get("node.name")))
    });
    nodes
}

fn icon(card: &Card) -> Icon {
    match card.bus.as_str() {
        "usb" => Icon::Usb,
        "bluetooth" => Icon::Headphones,
        _ => Icon::Speaker,
    }
}

fn profile_name(card: &Card) -> &str {
    card.active_profile
        .as_deref()
        .and_then(|name| card.profiles.iter().find(|p| p.name == name))
        .map_or("No active profile", |profile| profile.description.as_str())
}

fn route(device: &AudioDevice, kind: StripKind, open: bool) -> Element<Msg> {
    let active = device.active_port.as_deref().and_then(|name| device.ports.iter().find(|p| p.name == name));
    let entries: Vec<Element<Msg>> = device
        .ports
        .iter()
        .map(|port| {
            let unavailable = port.available == Some(false);
            button(if unavailable { format!("{} · unplugged", port.description) } else { port.description.clone() })
                .enabled(!unavailable)
                .tooltip(if unavailable { "Route unavailable: not plugged in" } else { "Select device route" })
                .selected(device.active_port.as_deref() == Some(&port.name))
                .on_press(Msg::MixerRoute(kind, device.index, port.name.clone()))
                .width(Fill)
                .into()
        })
        .collect();
    row![
        text(if kind == StripKind::Output { "Output route" } else { "Input route" }).width(110.),
        popover(
            button(format!("{} ⌄", active.map_or("No route", |p| p.description.as_str())))
                .on_press(Msg::MixerRouteMenu(kind, device.index))
                .enabled(!entries.is_empty())
                .id(format!("device-route-{kind:?}-{}", device.index)),
            container(column(entries).spacing(4.).padding(8.)).width(210.),
            open,
        )
        .on_dismiss(Msg::MixerRouteMenu(kind, device.index)),
    ]
    .spacing(8.)
    .into()
}

pub fn view(
    audio: &AudioState,
    graph: &Graph,
    selected: Option<&str>,
    profile_menu: bool,
    route_menu: Option<(StripKind, u32)>,
) -> Element<Msg> {
    if audio.cards.is_empty() {
        return container(
            text("No sound cards. Devices appear here when PipeWire finds them.").size(20.).role(Role::Text2).wrap(),
        )
        .align_x(Align::Center)
        .align_y(Align::Center)
        .padding(36.)
        .width(Fill)
        .height(Fill)
        .into();
    }
    let card = selected.and_then(|name| audio.cards.iter().find(|c| c.name == name)).unwrap_or(&audio.cards[0]);
    let list: Vec<Element<Msg>> = audio
        .cards
        .iter()
        .map(|entry| {
            let active = entry.active_profile.as_deref() != Some("off") && entry.active_profile.is_some();
            let label = if entry.description.is_empty() { &entry.name } else { &entry.description };
            column![
                button(format!("{}  {}", if active { "●" } else { "○" }, label))
                    .icon(icon(entry))
                    .selected(entry.name == card.name)
                    .width(Fill)
                    .on_press(Msg::DeviceSelect(entry.name.clone()))
                    .id(format!("device-{}", entry.index)),
                text(format!("{} · {}", if entry.bus.is_empty() { "device" } else { &entry.bus }, profile_name(entry)))
                    .size(11.)
                    .role(Role::Text2)
                    .wrap(),
            ]
            .spacing(3.)
            .padding([3., 4.])
            .width(Fill)
            .into()
        })
        .collect();
    let mut detail: Vec<Element<Msg>> = vec![
        text(card.description.as_str()).size(24.).semibold().wrap().into(),
        text(format!("{} · {}", if card.bus.is_empty() { "Device" } else { &card.bus }, card.name))
            .size(11.)
            .role(Role::Text3)
            .wrap()
            .into(),
        caption("PROFILE").into(),
    ];
    let profiles: Vec<Element<Msg>> = card
        .profiles
        .iter()
        .map(|profile| {
            let unavailable = !profile.available;
            button(format!(
                "{} · {} out / {} in{}",
                profile.description,
                profile.sinks,
                profile.sources,
                if unavailable { " · unavailable" } else { "" }
            ))
            .selected(card.active_profile.as_deref() == Some(&profile.name))
            .enabled(!unavailable)
            .tooltip(if unavailable {
                "Profile unavailable: hardware or connection is absent"
            } else {
                "Select card profile"
            })
            .on_press(Msg::DeviceProfile(card.name.clone(), profile.name.clone()))
            .id(format!("profile-{}", profile.name))
            .width(Fill)
            .into()
        })
        .collect();
    detail.push(
        popover(
            button(format!("{} ⌄", profile_name(card))).on_press(Msg::DeviceProfileMenu).id("device-profile"),
            container(column(profiles).spacing(4.).padding(8.)).width(300.),
            profile_menu,
        )
        .on_dismiss(Msg::DeviceProfileMenu)
        .into(),
    );
    let nodes = card_nodes(graph, card);
    let outputs: Vec<_> =
        audio.sinks.iter().filter(|d| nodes.iter().any(|n| n.props.get("node.name") == Some(&d.name))).collect();
    let inputs: Vec<_> =
        audio.sources.iter().filter(|d| nodes.iter().any(|n| n.props.get("node.name") == Some(&d.name))).collect();
    for (heading, devices, kind) in
        [("OUTPUT ROUTES", outputs, StripKind::Output), ("INPUT ROUTES", inputs, StripKind::Input)]
    {
        if !devices.is_empty() {
            detail.push(caption(heading).into());
            for device in devices {
                let status = match device.state {
                    DeviceState::Running => "● running",
                    DeviceState::Idle => "● idle",
                    DeviceState::Suspended => "○ suspended",
                    DeviceState::Unknown => "○ unknown",
                };
                detail.push(text(format!("{} · {status}", device.description)).size(12.).into());
                detail.push(route(device, kind, route_menu == Some((kind, device.index))));
            }
        }
    }
    detail.push(caption("NODES").into());
    if nodes.is_empty() {
        detail.push(text("No audio nodes in this profile.").role(Role::Text2).into());
    }
    for node in nodes {
        let name = node
            .props
            .get("node.description")
            .or_else(|| node.props.get("node.name"))
            .map_or("Audio node", String::as_str);
        let channels = graph
            .ports
            .values()
            .filter(|p| p.node_id == Some(node.id) && graph.kind(p.id) == Some(PortKind::Audio))
            .count();
        let channels = node.props.get("audio.channels").and_then(|v| v.parse::<usize>().ok()).unwrap_or(channels);
        detail.push(
            row![
                column![
                    text(name).semibold().size(13.),
                    text(format!("{channels} channels")).size(11.).role(Role::Text3)
                ]
                .spacing(2.)
                .width(Fill),
                button("Show in Mixer").compact().on_press(Msg::DeviceShowMixer(node.id)),
                button("Show in Audio").compact().on_press(Msg::DeviceShowAudio(node.id)),
            ]
            .spacing(8.)
            .into(),
        );
    }
    row![
        container(scroll(column(list).spacing(6.).padding(12.).width(Fill)).width(Fill).height(Fill))
            .fill(Tone::Chrome)
            .width(310.)
            .height(Fill),
        container(scroll(column(detail).spacing(12.).padding(20.).width(Fill)).width(Fill).height(Fill))
            .fill(Tone::Surface)
            .width(Fill)
            .height(Fill),
    ]
    .height(Fill)
    .width(Fill)
    .into()
}
