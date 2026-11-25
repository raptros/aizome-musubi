//! Clock controls and profiler diagnostics, rendered only from observed state.
use super::{Load, Settings};
use crate::app::Msg;
use aizome_kit::{Element, column, prelude::*, row};

pub fn view(settings: &Settings, load: &Load, persistent: bool) -> Element<Msg> {
    let read_only = settings.read_only.is_some();
    let value = |n: Option<u32>, suffix: &str| n.map_or_else(|| "Unavailable".into(), |n| format!("{n} {suffix}"));
    let rates = settings.rates();
    let quanta = settings.quanta();
    let rate_index = settings.force_rate.and_then(|n| rates.iter().position(|&r| r == n)).map(|i| i + 1).unwrap_or(0);
    let quantum_index =
        settings.force_quantum.and_then(|n| quanta.iter().position(|&q| q == n)).map(|i| i + 1).unwrap_or(0);
    let rate_choices: Vec<String> =
        std::iter::once("Automatic".into()).chain(rates.iter().map(|n| format!("{n} Hz"))).collect();
    let quantum_choices: Vec<String> =
        std::iter::once("Automatic".into()).chain(quanta.iter().map(u32::to_string)).collect();
    let rate = settings.force_rate.map_or_else(|| "Automatic".into(), |n| format!("{n} Hz"));
    let quantum = settings.force_quantum.map_or_else(|| "Automatic".into(), |n| n.to_string());
    let mut clock = column![
        caption("CLOCK"),
        text("Effective graph values").semibold(),
        row![
            text(format!("Rate · {}", value(settings.rate, "Hz"))).width(175.),
            text(format!("Quantum · {}", value(settings.quantum, "frames"))).width(210.),
            text(
                settings
                    .period_ms()
                    .map_or_else(|| "Graph period · unavailable".into(), |ms| format!("Graph period · {ms:.2} ms"))
            )
        ]
        .spacing(12.),
        text("One graph period is not measured round-trip latency.").size(11.).role(Role::Text3),
        divider().edge(Edge::Line),
        text("Forced values · runtime").semibold(),
        row![
            text(format!("Rate: {rate}")).width(180.),
            text(format!("Quantum: {quantum}")).width(180.),
            spacer(),
            button("Automatic").on_press(Msg::ClockAutomatic).enabled(!read_only).id("clock-automatic")
        ]
        .spacing(10.),
        row![
            dropdown(rate_choices, Some(rate_index))
                .on_select(move |index| if index == 0 { Msg::ClockClearRate } else { Msg::ClockRate(rates[index - 1]) })
                .enabled(!read_only)
                .width(200.)
                .id("clock-rate"),
            dropdown(quantum_choices, Some(quantum_index))
                .on_select(move |index| if index == 0 {
                    Msg::ClockClearQuantum
                } else {
                    Msg::ClockQuantum(quanta[index - 1])
                })
                .enabled(!read_only)
                .width(200.)
                .id("clock-quantum")
        ]
        .spacing(10.),
    ]
    .spacing(10.);
    if let Some(reason) = &settings.read_only {
        clock = clock.push(text(format!("Read-only · {reason}")).role(Role::Danger).wrap());
    }
    clock = clock.push(divider().edge(Edge::Line)).push(
        checkbox("Use at every start", persistent).on_toggle(Msg::ClockPersist).enabled(!read_only && ((settings.force_rate.is_some() || settings.force_quantum.is_some()) || persistent)).id("clock-persist")
    ).push(text("Uses only the explicit forced numbers. Takes effect at the next PipeWire start. `aizome-musubi effects remove-all` removes this setting.").size(11.).role(Role::Text3).wrap());

    let current = load.samples.back().map_or(0., |(_, sample)| *sample);
    let mut bars = row![].spacing(2.).height(72.);
    for (_, sample) in load.samples.iter().rev().take(60).rev() {
        bars = bars.push(container(spacer()).fill(Tone::Chrome).width(5.).height((sample / 100. * 68.).max(2.)));
    }
    let mut table = column![
        row![
            text("Node").width(260.),
            text("Role").width(90.),
            text("Rate").width(90.),
            text("Quantum").width(90.),
            text("Busy / Wait").width(135.),
            text("Xruns / errors since opened")
        ]
        .spacing(8.)
    ]
    .spacing(6.);
    for driver in &load.rows {
        let (xruns, errors) = load.delta(driver);
        table = table.push(
            row![
                text(driver.node.as_str()).width(260.),
                text(if driver.driver { "Driver" } else { "Follower" }).width(90.),
                text(driver.rate.to_string()).mono().width(90.),
                text(driver.quantum.to_string()).mono().width(90.),
                text(format!("{:.2} / {:.2}", driver.busy, driver.wait)).mono().width(135.),
                text(format!("{xruns} / {errors}")).mono()
            ]
            .spacing(8.),
        );
    }
    if load.rows.is_empty() {
        table = table.push(text("Waiting for profiler rows…").role(Role::Text3));
    }
    column![
        container(clock).fill(Tone::Chrome).border(Edge::Line).padding(16.).width(Fill),
        container(
            column![caption("DSP LOAD · LAST 60 S"), text(format!("{current:.1}% of graph cycle")), bars].spacing(8.)
        )
        .fill(Tone::Chrome)
        .border(Edge::Line)
        .padding(16.)
        .width(Fill),
        caption("DRIVERS AND FOLLOWERS · COUNTERS SINCE OPENED"),
        container(table).fill(Tone::Chrome).border(Edge::Line).padding(12.).width(Fill),
    ]
    .spacing(14.)
    .padding(18.)
    .width(Fill)
    .height(Fill)
    .into()
}
