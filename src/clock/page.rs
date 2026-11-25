//! Clock controls and profiler diagnostics, rendered only from observed state.
use super::{Effective, Load, Settings};
use crate::app::Msg;
use aizome_kit::{Element, column, prelude::*, row};

pub fn view(settings: &Settings, load: &Load, persistent: bool) -> Element<Msg> {
    let read_only = settings.read_only.is_some();
    let value = |n: Effective<u32>, suffix: &str| match n {
        Effective::Unavailable => "Unavailable".into(),
        Effective::Varies => "varies".into(),
        Effective::Value(n) => format!("{n} {suffix}"),
    };
    let (effective_rate, effective_quantum) = load.effective();
    let period = match (effective_rate, effective_quantum) {
        (Effective::Value(rate), Effective::Value(quantum)) if rate > 0 => {
            format!("Graph period · {:.2} ms", quantum as f64 * 1000. / rate as f64)
        }
        (Effective::Varies, _) | (_, Effective::Varies) => "Graph period · varies".into(),
        _ => "Graph period · unavailable".into(),
    };
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
            text(format!("Rate · {}", value(effective_rate, "Hz"))).width(175.),
            text(format!("Quantum · {}", value(effective_quantum, "frames"))).width(210.),
            text(period)
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

    let current = load.current_percent();
    let mut bars = row![].spacing(2.).height(110.).align(Align::End).width(Fill);
    let now = load.samples.back().map(|(instant, _)| *instant);
    for second in (0..60).rev() {
        let sample = now
            .and_then(|now| {
                load.samples.iter().rev().find(|(at, _)| now.saturating_duration_since(*at).as_secs() == second)
            })
            .map_or(0., |(_, value)| *value);
        bars = bars.push(container(spacer()).fill(Tone::Focus).width(Fill).height((sample / 100. * 106.).max(1.)));
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
            column![
                caption("DSP LOAD · LAST 60 S"),
                text(format!("{current:.1}% of graph cycle")),
                row![
                    column![text("100%").size(10.).mono(), spacer(), text("0%").size(10.).mono()]
                        .height(110.)
                        .width(38.),
                    bars
                ]
                .spacing(8.)
                .width(Fill)
            ]
            .spacing(8.)
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
