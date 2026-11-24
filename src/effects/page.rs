//! Snapshot-only Effects editor. Actions are sent back to the app for read-back.
use super::{BandKind, Chain, Target, curve::Curve, service::Status};
use crate::app::Msg;
use aizome_kit::{Element, column, prelude::*, row};

pub fn status_text(status: &Status) -> String {
    match status {
        Status::Running => "running".into(),
        Status::NodesMissing => "nodes missing".into(),
        Status::TargetMissing => "target missing".into(),
        Status::Failed(reason) => format!("failed · {reason}"),
        Status::StartLimit => "failed · start limit hit".into(),
        Status::Stopped => "stopped".into(),
    }
}

pub fn view(
    chains: &[Chain],
    selected: Option<&str>,
    statuses: &[(String, Status)],
    outputs: &[(String, String)],
    external: &[String],
    target_menu: bool,
    preset_menu: bool,
    page_menu: bool,
    name_input: &str,
    edits: &[String; 3],
) -> Element<Msg> {
    let current = selected.and_then(|id| chains.iter().find(|c| c.id == id));
    let mut list = column![caption("CHAINS")].spacing(8.).width(225.);
    for chain in chains {
        let status = statuses.iter().find(|(id, _)| id == &chain.id).map_or(&Status::Stopped, |(_, status)| status);
        let destination = match &chain.target {
            Target::FollowDefault => "Follow default".into(),
            Target::Output(name) => {
                outputs.iter().find(|(id, _)| id == name).map_or_else(|| name.clone(), |(_, label)| label.clone())
            }
        };
        let id = chain.id.clone();
        list = list.push(
            button(format!("● {} · {} · {destination}", chain.name, status_text(status)))
                .selected(selected == Some(chain.id.as_str()))
                .on_press(Msg::EffectSelect(id))
                .id(format!("chain-{}", chain.id))
                .width(Fill),
        );
    }
    if chains.is_empty() {
        list = list.push(text("No EQ chains. Create one to get started.").size(12.).role(Role::Text3));
    }
    list = list.push(caption("OTHER EFFECTS"));
    for name in external {
        list = list.push(
            column![text(name.as_str()).size(12.), text("Not managed by Musubi").size(10.).role(Role::Text3)]
                .spacing(2.),
        );
    }
    if external.is_empty() {
        list = list.push(text("No external effects found").size(11.).role(Role::Text3));
    }
    list = list
        .push(spacer())
        .push(button("New EQ").icon(Icon::Plus).on_press(Msg::EffectNew).id("effect-new").width(Fill));

    let mut editor = column![].spacing(10.).width(Fill).height(Fill);
    if let Some(chain) = current {
        let status = statuses.iter().find(|(id, _)| id == &chain.id).map_or(&Status::Stopped, |(_, status)| status);
        let id = chain.id.clone();
        editor = editor.push(
            row![
                column![
                    text(chain.name.as_str()).size(18.).semibold(),
                    text("A virtual output: audio sent to it is equalised, then passed on.")
                        .size(11.)
                        .role(Role::Text3)
                ]
                .spacing(3.),
                spacer(),
                text(status_text(status)).role(match status {
                    Status::Running => Role::Success,
                    Status::Failed(_) | Status::StartLimit => Role::Danger,
                    _ => Role::Text3,
                })
            ]
            .spacing(8.),
        );
        let choices: Vec<Element<Msg>> = std::iter::once((Target::FollowDefault, "Follow default".to_owned()))
            .chain(
                outputs
                    .iter()
                    .filter(|(name, _)| !name.starts_with("musubi_eq."))
                    .map(|(name, label)| (Target::Output(name.clone()), label.clone())),
            )
            .map(|(target, label)| button(label).on_press(Msg::EffectTarget(target)).width(Fill).into())
            .collect();
        let target_label = match &chain.target {
            Target::FollowDefault => "Follow default".into(),
            Target::Output(name) => {
                outputs.iter().find(|(id, _)| id == name).map_or_else(|| name.clone(), |(_, label)| label.clone())
            }
        };
        let preset_choices: Vec<Element<Msg>> =
            std::iter::once(button("Flat").on_press(Msg::EffectPreset(None)).width(Fill).into())
                .chain(
                    chains.iter().filter(|c| c.id != chain.id).map(|c| {
                        button(c.name.clone()).on_press(Msg::EffectPreset(Some(c.id.clone()))).width(Fill).into()
                    }),
                )
                .collect();
        editor = editor.push(
            row![
                text("Name").size(11.).role(Role::Text2),
                text_field(name_input)
                    .on_change(Msg::EffectNameInput)
                    .on_submit(Msg::EffectNameCommit)
                    .on_blur(Msg::EffectNameCommit)
                    .width(170.)
                    .id("effect-name"),
                text("Sends to").size(11.).role(Role::Text2),
                popover(
                    button(target_label).on_press(Msg::EffectTargetMenu).id("effect-target"),
                    container(column(choices).spacing(3.).padding(8.)).width(210.),
                    target_menu
                )
                .on_dismiss(Msg::EffectTargetMenu),
                text("Preset").size(11.).role(Role::Text2),
                popover(
                    button("Flat / saved").on_press(Msg::EffectPresetMenu).id("effect-preset"),
                    container(column(preset_choices).spacing(3.).padding(8.)).width(210.),
                    preset_menu
                )
                .on_dismiss(Msg::EffectPresetMenu),
            ]
            .spacing(8.),
        );
        editor = editor.push(
            row![
                checkbox("Enable", chain.enabled).on_toggle(Msg::EffectEnable).id("effect-enable"),
                button("Bypass").selected(chain.bypass).on_press(Msg::EffectBypass(!chain.bypass)).id("effect-bypass"),
                spacer(),
                button("Delete chain…").icon(Icon::Trash).on_press(Msg::EffectDelete).id("effect-delete")
            ]
            .spacing(8.),
        );
        editor = editor.push(
            Curve::new(chain.bands.clone(), |(index, band)| Msg::EffectBand(index, band))
                .id("effect-curve")
                .height(250.)
                .width(Fill),
        );
        editor = editor.push(caption("BANDS"));
        let mut table = column![
            row![
                text("#").width(30.),
                text("Type").width(150.),
                text("Frequency (Hz)").width(145.),
                text("Gain (dB)").width(125.),
                text("Q").width(105.)
            ]
            .spacing(8.)
        ]
        .spacing(5.);
        for (index, band) in chain.bands.iter().enumerate() {
            let kinds = ["Peaking".to_owned(), "Low shelf".to_owned(), "High shelf".to_owned()];
            let kind = match band.kind {
                BandKind::Peaking => 0,
                BandKind::LowShelf => 1,
                BandKind::HighShelf => 2,
            };
            let freq = if edits[0].is_empty() { band.freq.to_string() } else { edits[0].clone() };
            let gain = if edits[1].is_empty() { band.gain.to_string() } else { edits[1].clone() };
            let q = if edits[2].is_empty() { band.q.to_string() } else { edits[2].clone() };
            table = table.push(
                row![
                    text(format!("{}", index + 1)).mono().width(30.),
                    dropdown(kinds, Some(kind)).on_select(move |kind| Msg::EffectBandType(index, kind)).width(150.),
                    text_field(&freq)
                        .on_change(move |value| Msg::EffectValueInput(index, 0, value))
                        .on_submit(Msg::EffectValueCommit(index, 0))
                        .on_blur(Msg::EffectValueCommit(index, 0))
                        .mono()
                        .width(145.)
                        .id(format!("effect-freq-{index}")),
                    text_field(&gain)
                        .on_change(move |value| Msg::EffectValueInput(index, 1, value))
                        .on_submit(Msg::EffectValueCommit(index, 1))
                        .on_blur(Msg::EffectValueCommit(index, 1))
                        .mono()
                        .width(125.)
                        .id(format!("effect-gain-{index}")),
                    text_field(&q)
                        .on_change(move |value| Msg::EffectValueInput(index, 2, value))
                        .on_submit(Msg::EffectValueCommit(index, 2))
                        .on_blur(Msg::EffectValueCommit(index, 2))
                        .mono()
                        .width(105.)
                        .id(format!("effect-q-{index}")),
                ]
                .spacing(8.),
            );
        }
        editor = editor.push(container(table).fill(Tone::Chrome).border(Edge::Line).padding(8.).width(Fill));
        editor = editor.push(
            button("+ Add Band").on_press(Msg::EffectAddBand).enabled(chain.bands.len() < 8).id("effect-add-band"),
        );
        let _ = id;
    } else {
        editor = editor.push(
            container(
                column![
                    text("No EQ chain selected").size(18.),
                    text("Select a chain or create a new EQ.").role(Role::Text3)
                ]
                .spacing(8.),
            )
            .width(Fill)
            .height(Fill),
        );
    }
    let menu: Vec<Element<Msg>> =
        vec![button("Remove all Musubi effects and clock settings…").on_press(Msg::EffectRemoveAll).width(Fill).into()];
    row![
        container(list).padding(10.).fill(Tone::Sunk).width(245.).height(Fill),
        container(
            column![
                row![
                    spacer(),
                    popover(
                        button("Effects menu").icon(Icon::Ellipsis).on_press(Msg::EffectPageMenu).id("effects-menu"),
                        container(column(menu).padding(8.)).width(360.),
                        page_menu
                    )
                    .on_dismiss(Msg::EffectPageMenu)
                ],
                editor
            ]
            .spacing(8.)
        )
        .padding(12.)
        .width(Fill)
        .height(Fill)
    ]
    .spacing(0.)
    .height(Fill)
    .into()
}
