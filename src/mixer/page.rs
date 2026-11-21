//! Snapshot-driven mixer layout. The MAIN strip is outside the horizontal scroller.
use super::{
    Fader, LevelMeter,
    model::{Strip, StripControl, StripKind},
};
use crate::{app::Msg, backend::meters::MeterReading};
use aizome_kit::{column, prelude::*, row};
use aizome_sys::audio::AudioState;
use std::time::Instant;

fn strip_view(
    strip: &Strip,
    audio: &AudioState,
    readings: &[MeterReading],
    now: Instant,
    main: bool,
    linked: bool,
    percent: bool,
    meters: bool,
) -> Element<Msg> {
    let index = match &strip.control {
        StripControl::Pulse { index, .. } => Some(*index),
        _ => None,
    };
    let reason = match &strip.control {
        StripControl::ReadOnly(reason) => Some(reason.clone()),
        _ => None,
    };
    let (volume, muted, balance) = match &strip.control {
        StripControl::Pulse { volume, muted, balance, .. } => (*volume as f64, *muted, *balance),
        StripControl::Native { channel_volumes, muted, .. } => {
            (channel_volumes.iter().copied().fold(0_f32, f32::max) as f64, muted.unwrap_or(false), None)
        }
        StripControl::ReadOnly(_) => (0., false, None),
    };
    let targets = if main || strip.kind == StripKind::Playback { &audio.sinks } else { &audio.sources };
    let current = if main {
        audio.sinks.iter().find(|d| d.is_default).map(|d| d.index)
    } else {
        index.and_then(|i| match strip.kind {
            StripKind::Playback => audio.playback.iter().find(|s| s.index == i).map(|s| s.device),
            StripKind::Recording => audio.recording.iter().find(|s| s.index == i).map(|s| s.device),
            _ => None,
        })
    };
    let target: Element<Msg> = if main || matches!(strip.kind, StripKind::Playback | StripKind::Recording) {
        let options: Vec<_> = targets.iter().map(|d| d.description.clone()).collect();
        let selected = targets.iter().position(|d| Some(d.index) == current);
        let indices: Vec<_> = targets.iter().map(|d| d.index).collect();
        let names: Vec<_> = targets.iter().map(|d| d.name.clone()).collect();
        let stream_kind = strip.kind;
        dropdown(options, selected)
            .on_select(move |position| {
                if main {
                    Msg::MixerDefault(StripKind::Output, names[position].clone())
                } else {
                    Msg::MixerMove(stream_kind, index.unwrap_or_default(), indices[position])
                }
            })
            .enabled(reason.is_none() && !targets.is_empty())
            .width(112.)
            .id(if main { "main-target" } else { "strip-target" })
            .into()
    } else {
        container(text("Device route").size(11.).role(Role::Text3)).height(24.).into()
    };
    let kind = strip.kind;
    let node = strip.node;
    let control = strip.control.clone();
    let fader = Fader::with_reason_and_mute(
        volume,
        percent,
        reason.clone(),
        move |value| Msg::MixerVolume(kind, control.clone(), value as f32),
        Some(Msg::MixerMute(kind, strip.control.clone(), !muted)),
    )
    .width(75.)
    .height(Fill)
    .id(format!("fader-{kind:?}-{}", index.unwrap_or_else(|| node.map_or(0, |n| n.id))));
    let reading = if meters { node.and_then(|n| readings.iter().find(|r| r.node == n)) } else { None };
    let peaks: &[f32] = reading.map_or(&[], |r| r.peaks.as_slice());
    let paused = !meters || reading.is_some_and(|r| r.paused) || node.is_none();
    let meter = LevelMeter::element::<Msg>(peaks, now, paused).width(20.).height(Fill);
    let title = if main { "MAIN" } else { &strip.name };
    let mut buttons = row![
        button("M")
            .width(32.)
            .selected(muted)
            .enabled(reason.is_none() && !matches!(strip.control, StripControl::Native { muted: None, .. }))
            .on_press(Msg::MixerMute(kind, strip.control.clone(), !muted))
            .tooltip(reason.clone().unwrap_or_else(|| "Mute".into()))
            .id(format!("mute-{kind:?}-{}", index.unwrap_or_default())),
    ]
    .spacing(2.);
    if matches!(kind, StripKind::Output | StripKind::Input) && !main {
        let device = if kind == StripKind::Output { &audio.sinks } else { &audio.sources };
        if let Some(d) = index.and_then(|i| device.iter().find(|d| d.index == i)) {
            buttons = buttons.push(
                button("◉")
                    .width(32.)
                    .selected(d.is_default)
                    .enabled(reason.is_none())
                    .on_press(Msg::MixerDefault(kind, d.name.clone()))
                    .tooltip(if d.is_default { "Default device" } else { "Make default" })
                    .label(if d.is_default { "Default device — on" } else { "Default device — off" }),
            );
        }
    }
    if matches!(kind, StripKind::Output | StripKind::Input | StripKind::Native) && !main {
        if let Some(id) = index.or_else(|| node.map(|n| n.id)) {
            buttons = buttons.push(
                button("↔")
                    .width(32.)
                    .selected(linked)
                    .enabled(reason.is_none())
                    .on_press(Msg::MixerLink(kind, id))
                    .tooltip("Link channels"),
            );
        }
    }
    let balance_control: Element<Msg> = if let (Some(balance), Some(i)) = (balance, index) {
        slider(balance as f64, ValueRange::new(-1., 1.).expect("balance range"))
            .on_change(move |value| Msg::MixerBalance(kind, i, value as f32))
            .enabled(reason.is_none())
            .width(100.)
            .into()
    } else {
        container(text(" ")).height(22.).into()
    };
    let subtitle = if main { &strip.name } else { &strip.subtitle };
    let label = format!("{} · {} · {:?}", strip.name, subtitle, kind);
    mouse_area(
        container(
            column![
                container(
                    text(match kind {
                        StripKind::Output => "OUT",
                        StripKind::Input => "IN",
                        StripKind::Playback => "PLAY",
                        StripKind::Recording => "REC",
                        StripKind::Native => "NODE",
                    })
                    .size(10.)
                    .role(Role::Accent)
                )
                .height(18.),
                text(title).semibold().size(12.).tooltip(title),
                text(subtitle.as_str()).size(10.).role(Role::Text3).tooltip(subtitle.as_str()),
                target,
                balance_control,
                buttons,
                row![fader, meter].spacing(1.).height(Fill),
            ]
            .spacing(5.)
            .width(Fill)
            .height(Fill),
        )
        .padding(5.)
        .fill(Tone::Chrome)
        .border(Edge::Line)
        .width(120.)
        .height(Fill),
    )
    .on_hover(move |hovered| Msg::MixerHover(hovered.then(|| label.clone())))
    .into()
}

pub fn view(
    strips: &[Strip],
    audio: &AudioState,
    readings: &[MeterReading],
    now: Instant,
    linked: &std::collections::HashSet<(StripKind, u32)>,
    percent: bool,
    meters: bool,
    shown: &[bool; 5],
    filter: &str,
) -> Element<Msg> {
    let filter = filter.to_lowercase();
    let visible = |s: &&Strip| {
        let category = match s.kind {
            StripKind::Output => 0,
            StripKind::Input => 1,
            StripKind::Playback => 2,
            StripKind::Recording => 3,
            StripKind::Native => 4,
        };
        shown[category]
            && (filter.is_empty()
                || s.name.to_lowercase().contains(&filter)
                || s.subtitle.to_lowercase().contains(&filter))
    };
    let main = strips
        .iter()
        .find(|s| s.kind == StripKind::Output && audio.sinks.iter().any(|d| d.is_default && s.name == d.description));
    let mut content = row![].spacing(12.).height(Fill);
    for (kind, heading) in [
        (StripKind::Output, "OUTPUTS"),
        (StripKind::Input, "INPUTS"),
        (StripKind::Playback, "PLAYING"),
        (StripKind::Recording, "RECORDING"),
        (StripKind::Native, "NATIVE"),
    ] {
        let group: Vec<_> = strips.iter().filter(|s| s.kind == kind).filter(&visible).collect();
        if group.is_empty() {
            continue;
        }
        let mut strips_row = row![].spacing(6.).height(Fill);
        for strip in group {
            let id = match &strip.control {
                StripControl::Pulse { index, .. } => *index,
                _ => strip.node.map_or(0, |n| n.id),
            };
            strips_row = strips_row.push(strip_view(
                strip,
                audio,
                readings,
                now,
                false,
                linked.contains(&(strip.kind, id)),
                percent,
                meters,
            ));
        }
        content = content.push(column![text(heading).size(10.).role(Role::Text3), strips_row].spacing(5.).height(Fill));
    }
    let scrolling = scroll(content).horizontal().height(Fill);
    let main_strip: Element<Msg> = main.map_or_else(
        || container(text("No default output")).width(120.).height(Fill).into(),
        |s| strip_view(s, audio, readings, now, true, false, percent, meters),
    );
    row![
        scrolling,
        container(column![text("MAIN").size(10.).role(Role::Text3), main_strip].spacing(5.))
            .padding([0., 8.])
            .width(136.)
            .height(Fill)
    ]
    .spacing(4.)
    .padding([8., 8., 8., 8.])
    .height(Fill)
    .width(Fill)
    .into()
}
