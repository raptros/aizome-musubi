use crate::{
    backend::{Backend, Command, Connection, Event, Handle, RequestId, fake::FakeBackend, pipewire::PipeWireBackend},
    devices,
    effects::{
        self, Band, BandKind, Chain, Target,
        service::{EffectsService, Status, Systemd, Units, derive_status},
    },
    graph::{
        GlobalId, Graph, LinkState,
        classify::PortKind,
        group::port_label,
        identity::StableId,
        legal::{LinkLegality, Reason, can_link},
    },
    instance,
    matrix::{
        Layout, MatrixMsg, PatchMatrix,
        layout::{Cell, CellState, ViewOptions},
    },
    mixer::{
        self,
        audio::{AudioControl, AudioTask},
        model::{StripControl, StripKind},
    },
    persistence, routes,
    undo::History,
};
use aizome_kit::{
    Rect, column,
    prelude::*,
    row,
    widget::{LayoutCx, Widget},
};
use std::{
    array,
    cell::{Cell as SharedCell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Audio,
    Midi,
    Midi2,
    Video,
    Mixer,
    Effects,
    Devices,
}

impl Page {
    const ALL: [(Page, Icon, &str); 7] = [
        (Page::Audio, Icon::Volume2, "Audio"),
        (Page::Midi, Icon::Music, "MIDI"),
        (Page::Midi2, Icon::Keyboard, "MIDI 2.0"),
        (Page::Video, Icon::Video, "Video"),
        (Page::Mixer, Icon::SlidersHorizontal, "Mixer"),
        (Page::Effects, Icon::Activity, "Effects"),
        (Page::Devices, Icon::Usb, "Devices"),
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Audio => "Audio",
            Page::Midi => "MIDI",
            Page::Midi2 => "MIDI 2.0",
            Page::Video => "Video",
            Page::Mixer => "Mixer",
            Page::Devices => "Devices",
            Page::Effects => "Effects",
        }
    }
    fn accepts(self, kind: PortKind) -> bool {
        matches!(
            (self, kind),
            (Page::Audio, PortKind::Audio)
                | (Page::Midi, PortKind::Midi)
                | (Page::Midi2, PortKind::Ump)
                | (Page::Video, PortKind::Video)
                | (Page::Mixer, PortKind::Audio)
                | (Page::Devices, PortKind::Audio)
                | (Page::Effects, PortKind::Audio)
        )
    }
}

/// One gesture's intended operations, retained with stable identities for undo.
#[derive(Clone, Debug)]
pub struct LinkIntent {
    pub from: StableId,
    pub to: StableId,
    pub connect: bool,
    pub request: RequestId,
    pub result: Option<Result<(), String>>,
    pub observed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Transaction {
    pub intents: Vec<LinkIntent>,
    pub mixer: Option<MixerIntent>,
}

impl Transaction {
    pub fn label(&self) -> &'static str {
        if let Some(intent) = &self.mixer {
            return match intent.before {
                MixerValue::Volume(_) => "Volume",
                MixerValue::Mute(_) => "Mute",
                MixerValue::Balance(_) => "Balance",
                MixerValue::Default(_) => "Default Device",
                MixerValue::Target(_) => "Move Stream",
                MixerValue::Port(_) => "Device Route",
                MixerValue::Profile(_) => "Device Profile",
            };
        }
        if self.intents.first().is_some_and(|intent| intent.connect) { "Connect" } else { "Disconnect" }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MixerIdentity {
    Device(StripKind, String),
    Card(String),
    Stream(StripKind, String, String, String),
    Native(String),
}

#[derive(Clone, Debug)]
pub enum MixerValue {
    Volume(f32),
    Mute(bool),
    Balance(f32),
    Default(String),
    Target(String),
    Port(String),
    Profile(String),
}

#[derive(Clone, Debug)]
pub struct MixerIntent {
    pub identity: MixerIdentity,
    pub before: MixerValue,
    pub after: MixerValue,
}

#[derive(Clone)]
struct Pending {
    pair: (GlobalId, GlobalId),
}

struct PendingAudio {
    task: AudioTask,
    transaction: Option<usize>,
    replay: Option<(usize, bool)>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Page(Page),
    AudioWake,
    MeterTick,
    MixerVolume(StripKind, StripControl, f32),
    MixerMute(StripKind, StripControl, bool),
    MixerBalance(StripKind, u32, f32),
    MixerDefault(StripKind, String),
    MixerMove(StripKind, u32, u32),
    MixerRoute(StripKind, u32, String),
    MixerRouteMenu(StripKind, u32),
    DeviceSelect(String),
    DeviceProfileMenu,
    DeviceProfile(String, String),
    DeviceShowMixer(GlobalId),
    DeviceShowAudio(GlobalId),
    MixerLink(StripKind, u32),
    MixerPercent(bool),
    MixerMeters(bool),
    MixerShow,
    MixerShown(usize, bool),
    MixerFilter(String),
    MixerSettle(u64),
    MixerHover(Option<String>),
    EffectSelect(String),
    EffectNew,
    EffectNameInput(String),
    EffectNameCommit,
    EffectTargetMenu,
    EffectTarget(Target),
    EffectPresetMenu,
    EffectPreset(Option<String>),
    EffectPageMenu,
    EffectEnable(bool),
    EffectBypass(bool),
    EffectBand(usize, Band),
    EffectBandEnd,
    EffectSettle(u64),
    EffectBandType(usize, usize),
    EffectValueInput(usize, usize, String),
    EffectValueCommit(usize, usize),
    EffectAddBand,
    EffectDelete,
    EffectDeleteConfirmed(String),
    EffectRemoveAll,
    EffectRemoveAllConfirmed(Vec<String>),
    Activated,
    Matrix(MatrixMsg),
    Backend(Event),
    Confirm(Vec<Cell>, bool),
    ConfirmRoute(GlobalId, GlobalId),
    List(bool),
    ShowMidi1(bool),
    SelectRoutes(Vec<usize>),
    SortRoutes((usize, bool)),
    InspectRoute(usize),
    DeleteRoutes,
    AddRoute,
    CloseRoute,
    RouteFromFilter(String),
    RouteToFilter(String),
    RouteFromFocus,
    RouteToFocus,
    RouteFromActivate(GlobalId),
    RouteToActivate(GlobalId),
    RouteFromSelected(Option<GlobalId>),
    RouteToSelected(Option<GlobalId>),
    ConnectRoute,
    Persist(u64),
    DisconnectSelection,
    ConnectInOrder,
    CollapseAll,
    ContextAction(ContextAction),
    Pairs(bool),
    Monitors(bool),
    Swap,
    FullNames,
    From(String),
    To(String),
    Undo,
    Redo,
    Inspector,
    Unlink(GlobalId, GlobalId),
}

#[derive(Clone, Debug)]
pub enum ContextAction {
    DisconnectFrom(GlobalId),
    DisconnectTo(GlobalId),
    ConnectGroup(Cell),
    Fold(crate::graph::group::GroupKey),
    Copy(GlobalId),
}
#[derive(Default)]
pub struct Musubi {
    pub page: Page,
    pub instance: Option<instance::Instance>,
    pub live: bool,
    backend: Option<PipeWireBackend>,
    fake: Option<Rc<RefCell<FakeBackend>>>,
    audio: Option<Box<dyn AudioControl>>,
    meter_running: bool,
    meter_nodes: Vec<Handle>,
    meter_tick: bool,
    window_suspended: bool,
    linked_channels: HashSet<(StripKind, u32)>,
    mixer_options: persistence::MixerOptions,
    mixer_show: bool,
    mixer_route_menu: Option<(StripKind, u32)>,
    device_selected: Option<String>,
    device_profile_menu: bool,
    mixer_filter: String,
    mixer_hover: Option<String>,
    mixer_pending: Option<usize>,
    audio_pending: Vec<PendingAudio>,
    mixer_succeeded: HashSet<usize>,
    mixer_failed: HashSet<usize>,
    native_pending: HashMap<RequestId, (Option<usize>, Option<(usize, bool)>)>,
    mixer_current: Option<usize>,
    mixer_replaying: Option<(usize, bool)>,
    mixer_generation: u64,
    effects_service: Option<EffectsService<Box<dyn Units>>>,
    effect_chains: Vec<Chain>,
    effect_selected: Option<String>,
    effect_statuses: Vec<(String, Status)>,
    effect_name_input: String,
    effect_values: [String; 3],
    effect_value_band: usize,
    effect_target_menu: bool,
    effect_preset_menu: bool,
    effect_page_menu: bool,
    effect_undo: Vec<(Chain, Chain)>,
    effect_redo: Vec<(Chain, Chain)>,
    effect_pending: Option<(Chain, Chain, usize, RequestId)>,
    effect_observing: Option<(Chain, Chain, bool)>,
    effect_queued: Option<(usize, Band)>,
    effect_burst: Option<(Chain, Chain)>,
    effect_replay: Option<(Chain, Chain, bool)>,
    effect_generation: u64,
    pub graph: Graph,
    connection: Option<Connection>,
    clock_defaults: Option<(u32, u32)>,
    matrix_spike: Option<Rc<Layout>>,
    pub options: [ViewOptions; 4],
    route_selection: Vec<(GlobalId, GlobalId)>,
    route_sort: (usize, bool),
    route_sheet: Option<WindowKey>,
    route_from_filter: String,
    route_to_filter: String,
    route_from: Option<GlobalId>,
    route_to: Option<GlobalId>,
    persisted_folds: [HashSet<String>; 4],
    persist_generation: u64,
    persist_enabled: bool,
    paths: Option<persistence::Paths>,
    layouts: [Option<Rc<Layout>>; 4],
    hovered: Option<Cell>,
    matrix_focus: Rc<SharedCell<Option<Cell>>>,
    selected: Option<StableId>,
    inspector: bool,
    pub transactions: Vec<Transaction>,
    history: History,
    pending: HashMap<RequestId, Pending>,
    errors: HashMap<(GlobalId, GlobalId), String>,
    next_request: RequestId,
    conversion_confirmed: bool,
    removed: HashMap<(GlobalId, GlobalId), Instant>,
    notice: Option<String>,
    window: WindowKey,
}

fn fit_status(sentence: &str) -> String {
    const LIMIT: usize = 100;
    let Some((names, state)) = sentence.rsplit_once(" · ") else {
        return sentence.to_owned();
    };
    let Some((from, to)) = names.split_once(" → ") else {
        return sentence.to_owned();
    };
    let strip_raw = |side: &str| {
        side.split_once(" · ").map_or_else(
            || side.to_owned(),
            |(group, port)| format!("{group} · {}", port.split_once(" (").map_or(port, |(short, _)| short)),
        )
    };
    let mut from = strip_raw(from);
    let mut to = strip_raw(to);
    let suffix = format!(" · {state}");
    let size = |from: &str, to: &str| from.chars().count() + to.chars().count() + 3 + suffix.chars().count();
    if size(&from, &to) <= LIMIT {
        return format!("{from} → {to}{suffix}");
    }
    // Keep both endpoints' group and port, shortening the longest individual name first.
    let mut parts = [
        from.split_once(" · ").map_or(from.clone(), |(group, _)| group.to_owned()),
        from.split_once(" · ").map_or(String::new(), |(_, port)| port.to_owned()),
        to.split_once(" · ").map_or(to.clone(), |(group, _)| group.to_owned()),
        to.split_once(" · ").map_or(String::new(), |(_, port)| port.to_owned()),
    ];
    while size(&from, &to) > LIMIT {
        let Some((index, longest)) = parts.iter().enumerate().max_by_key(|(_, part)| part.chars().count()) else {
            break;
        };
        if longest.chars().count() <= 2 {
            break;
        }
        let chars: Vec<_> = parts[index].chars().collect();
        let keep = (chars.len() - 2) / 2;
        parts[index] = format!(
            "{}…{}",
            chars[..keep].iter().collect::<String>(),
            chars[chars.len() - keep..].iter().collect::<String>()
        );
        from = format!("{} · {}", parts[0], parts[1]);
        to = format!("{} · {}", parts[2], parts[3]);
    }
    format!("{from} → {to}{suffix}")
}

#[cfg(test)]
#[test]
fn long_status_keeps_both_endpoints_and_state() {
    let name = "Extraordinarily long audio stream title".repeat(8);
    let result = fit_status(&format!(
        "{name} · FL (capture_AUX0) → {name} · FR (playback_FR) · connected — click to disconnect"
    ));
    assert!(result.contains(" · FL → ") && result.contains(" · FR · "));
    assert!(!result.contains("capture_AUX0"));
    assert!(result.ends_with(" · connected — click to disconnect"));
    assert!(result.chars().count() <= 100);
}

#[derive(Clone)]
struct ToolbarConfig {
    title: String,
    short_title: String,
    page: Page,
    list: bool,
    pairs: bool,
    monitors: bool,
    swap: bool,
    full_names: bool,
    show_midi_1: bool,
    from: String,
    to: String,
    has_ports: bool,
    can_undo: bool,
    mixer: persistence::MixerOptions,
    mixer_filter: String,
    mixer_show: bool,
    effects_available: bool,
}

struct ResponsiveToolbar(ToolbarConfig);

impl Widget for ResponsiveToolbar {
    fn name(&self) -> &'static str {
        "responsive-toolbar"
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: aizome_kit::Constraints) -> aizome_kit::Size {
        // Reserve both searches before deciding whether there is room for action labels.
        let width = bc.max.width;
        let compact = width < 990.;
        cx.clear_children();
        let child = cx.add_child(toolbar_controls(&self.0, compact, width < 760.).into_raw());
        cx.layout_child(child, bc);
        cx.place_child(child, aizome_kit::Point::ZERO);
        aizome_kit::Size::new(bc.max.width, cx.theme().metrics.toolbar)
    }
}

fn toolbar_controls(config: &ToolbarConfig, compact: bool, very_narrow: bool) -> Element<Msg> {
    let options = config;
    let mut controls: Vec<Element<Msg>> = Vec::new();
    // Narrow windows keep the page and link count, dropping the port count;
    // only very narrow ones drop the title.
    if !very_narrow {
        let title = if compact { config.short_title.clone() } else { config.title.clone() };
        controls.push(text(title).semibold().into());
    }
    if config.page == Page::Mixer {
        controls.push(
            segmented(["dB", "%"], if config.mixer.percent { 1 } else { 0 })
                .on_select(|index| Msg::MixerPercent(index == 1))
                .id("mixer-unit")
                .into(),
        );
        let mut meter_button = button("Meters").icon(Icon::Activity).toolbar().selected(config.mixer.meters);
        if config.mixer.meters {
            meter_button = meter_button.primary();
        }
        controls.push(meter_button.on_press(Msg::MixerMeters(!config.mixer.meters)).id("mixer-meters").into());
        let mut checkboxes: Vec<Element<Msg>> = Vec::new();
        for (index, label) in ["Outputs", "Inputs", "Playing", "Recording", "Monitors/virtual devices", "Effects"]
            .iter()
            .take(if config.effects_available { 6 } else { 5 })
            .enumerate()
        {
            checkboxes.push(
                checkbox(*label, config.mixer.shown[index])
                    .on_toggle(move |on| Msg::MixerShown(index, on))
                    .id(format!("mixer-show-{index}"))
                    .into(),
            );
        }
        controls.push(
            popover(
                button("Sections").icon(Icon::ListFilter).toolbar().on_press(Msg::MixerShow).id("mixer-show"),
                container(column(checkboxes).spacing(8.).padding(12.)).width(220.),
                config.mixer_show,
            )
            .on_dismiss(Msg::MixerShow)
            .into(),
        );
        controls.push(
            if compact { icon_button(Icon::Undo2) } else { button("Undo").icon(Icon::Undo2) }
                .toolbar()
                .on_press(Msg::Undo)
                .enabled(config.can_undo)
                .label("Undo")
                .id("undo")
                .into(),
        );
        controls.push(spacer().into());
        controls.push(
            search_field(&config.mixer_filter)
                .placeholder("Filter strips")
                .on_change(Msg::MixerFilter)
                .width(180.)
                .id("mixer-filter")
                .into(),
        );
        return toolbar(controls).into();
    }
    if config.page == Page::Effects {
        controls.push(
            button("Undo").icon(Icon::Undo2).toolbar().enabled(config.can_undo).on_press(Msg::Undo).id("undo").into(),
        );
        return toolbar(controls).into();
    }
    if config.page == Page::Devices {
        controls.push(
            button("Undo").icon(Icon::Undo2).toolbar().enabled(config.can_undo).on_press(Msg::Undo).id("undo").into(),
        );
        return toolbar(controls).into();
    }
    controls.push(
        segmented(["Matrix", "List"], if options.list { 1 } else { 0 })
            .on_select(|index| Msg::List(index == 1))
            .enabled(config.has_ports)
            .id("matrix-list")
            .into(),
    );
    controls.push(spacer().width(10.).into());
    if config.page == Page::Audio {
        controls.push(
            segmented(["Pairs", "Channels"], if options.pairs { 0 } else { 1 })
                .on_select(|index| Msg::Pairs(index == 0))
                .enabled(config.has_ports)
                .id("pairs-channels")
                .into(),
        );
        controls.push(
            if compact { icon_button(Icon::Headphones) } else { button("Monitors").icon(Icon::Headphones) }
                .toolbar()
                .selected(options.monitors)
                .on_press(Msg::Monitors(!options.monitors))
                .enabled(config.has_ports)
                .tooltip("Show monitor ports")
                .label("Monitors")
                .id("monitors")
                .into(),
        );
    }
    if config.page == Page::Midi2 {
        controls.push(
            checkbox("Show MIDI 1.0", options.show_midi_1)
                .on_toggle(Msg::ShowMidi1)
                .enabled(config.has_ports)
                .id("show-midi-1")
                .into(),
        );
    }
    controls.extend([
        if compact { icon_button(Icon::ArrowLeftRight) } else { button("Swap").icon(Icon::ArrowLeftRight) }
            .toolbar()
            .selected(options.swap)
            .on_press(Msg::Swap)
            .enabled(config.has_ports)
            .tooltip("Swap axes")
            .label("Swap")
            .id("swap")
            .into(),
        if compact { icon_button(Icon::Scan) } else { button("Full Names").icon(Icon::Scan) }
            .toolbar()
            .selected(options.full_names)
            .on_press(Msg::FullNames)
            .enabled(config.has_ports)
            .tooltip("Show full names")
            .label("Full Names")
            .id("full-names")
            .into(),
        if compact { icon_button(Icon::Undo2) } else { button("Undo").icon(Icon::Undo2) }
            .toolbar()
            .on_press(Msg::Undo)
            .enabled(config.can_undo)
            .tooltip("Undo last route change")
            .label("Undo")
            .id("undo")
            .into(),
        spacer().into(),
        search_field(&options.from)
            .placeholder("From")
            .on_change(Msg::From)
            .enabled(config.has_ports)
            .width(140.)
            .id("from-filter")
            .into(),
        search_field(&options.to)
            .placeholder("To")
            .on_change(Msg::To)
            .enabled(config.has_ports)
            .width(140.)
            .id("to-filter")
            .into(),
    ]);
    toolbar(controls)
}

impl Musubi {
    /// Synthetic geometry exists only in Harness tests, never in the shipped app.
    pub fn for_matrix_test(layout: Rc<Layout>) -> Self {
        Self { matrix_spike: Some(layout), ..Self::default() }
    }
    pub fn layout(&self, page: Page) -> Option<&Layout> {
        self.layouts[Self::index(page)].as_deref()
    }
    fn route_rows(&self) -> Vec<routes::Route> {
        let Some(layout) = self.layout(self.page) else { return Vec::new() };
        let pending: Vec<_> = self.pending.values().map(|p| p.pair).collect();
        let errors: Vec<_> = self.errors.iter().map(|(&(from, to), error)| (from, to, error.clone())).collect();
        let mut rows = routes::routes(
            &self.graph,
            layout,
            self.page == Page::Midi2 && self.options[2].show_midi_1,
            &pending,
            &errors,
        );
        routes::sort(&mut rows, self.route_sort.0, self.route_sort.1);
        rows
    }
    fn persist_later(&mut self, cx: &mut Cx<Msg>) {
        if !self.persist_enabled {
            return;
        }
        self.persist_generation += 1;
        cx.after(Duration::from_millis(350), Msg::Persist(self.persist_generation));
    }
    fn connect_route(&mut self, from: GlobalId, to: GlobalId, cx: &mut Cx<Msg>) {
        match can_link(&self.graph, from, to) {
            LinkLegality::Forbidden(reason) => self.notice = Some(format!("Not allowed: {}", reason_text(reason))),
            LinkLegality::NeedsConfirmation(_) if !self.conversion_confirmed => {
                cx.alert(
                    self.window,
                    Alert::new(
                        "MIDI conversion",
                        "PipeWire converts between MIDI 2.0 and MIDI 1.0. Some messages don't survive the conversion.",
                    )
                    .default_button("Connect", Msg::ConfirmRoute(from, to))
                    .cancel_button("Cancel"),
                );
            }
            _ => {
                if let Some(index) = self.submit_pairs(vec![(from, to, true)], cx) {
                    self.history.record(index);
                }
            }
        }
    }
    /// The Harness and fake backend use the same command path as the live app.
    pub fn with_fake(fake: Rc<RefCell<FakeBackend>>) -> Self {
        Self { fake: Some(fake), ..Self::default() }
    }
    pub fn with_fake_audio(fake: Rc<RefCell<FakeBackend>>, audio: Box<dyn AudioControl>) -> Self {
        Self { fake: Some(fake), audio: Some(audio), ..Self::default() }
    }
    /// Keep Harness persistence isolated from the process-wide XDG environment.
    pub fn with_fake_paths(fake: Rc<RefCell<FakeBackend>>, paths: persistence::Paths) -> Self {
        Self { fake: Some(fake), paths: Some(paths), ..Self::default() }
    }
    pub fn with_fake_effects(
        fake: Rc<RefCell<FakeBackend>>,
        paths: effects::paths::Paths,
        units: Box<dyn Units>,
    ) -> Self {
        Self { fake: Some(fake), effects_service: Some(EffectsService { paths, units }), ..Self::default() }
    }
    fn reload_effects(&mut self) {
        let Some(service) = &self.effects_service else { return };
        match service.list() {
            Ok(chains) => {
                self.effect_chains = chains;
                if !self.effect_chains.iter().any(|c| Some(c.id.as_str()) == self.effect_selected.as_deref()) {
                    self.effect_selected = self.effect_chains.first().map(|c| c.id.clone());
                }
                self.effect_name_input = self.selected_effect().map_or_else(String::new, |c| c.name.clone());
                self.refresh_effect_statuses();
            }
            Err(error) => self.notice = Some(format!("Effects: {error}")),
        }
    }
    fn selected_effect(&self) -> Option<&Chain> {
        self.effect_chains.iter().find(|c| Some(c.id.as_str()) == self.effect_selected.as_deref())
    }
    fn refresh_effect_statuses(&mut self) {
        let Some(service) = &self.effects_service else { return };
        self.effect_statuses = self
            .effect_chains
            .iter()
            .map(|chain| {
                let unit = effects::service::unit(&chain.id).expect("validated chain ID");
                let status = match service.units.state(&unit) {
                    Ok(state) => {
                        let capture = format!("musubi_eq.{}", chain.id);
                        let playback = format!("{capture}.playback");
                        let mut nodes = self.graph.nodes.values();
                        let present = nodes.clone().any(|n| n.props.get("node.name") == Some(&capture))
                            && nodes.any(|n| n.props.get("node.name") == Some(&playback));
                        let target = match &chain.target {
                            Target::FollowDefault => true,
                            Target::Output(name) => {
                                self.graph.nodes.values().any(|n| n.props.get("node.name") == Some(name))
                            }
                        };
                        derive_status(&state, present, target)
                    }
                    Err(_) if !chain.enabled => Status::Stopped,
                    Err(error) => Status::Failed(error),
                };
                (chain.id.clone(), status)
            })
            .collect();
        self.finish_effect_save();
    }
    fn finish_effect_save(&mut self) {
        let Some((_, after, _)) = &self.effect_observing else { return };
        let observed = self.effect_chains.iter().any(|chain| chain == after)
            && self.effect_statuses.iter().any(|(id, status)| {
                id == &after.id && matches!((after.enabled, status), (true, Status::Running) | (false, Status::Stopped))
            });
        if observed {
            let (before, after, record) = self.effect_observing.take().unwrap();
            if record {
                self.effect_undo.push((before, after));
                self.effect_redo.clear();
            }
            self.notice = Some("EQ change observed.".into());
            if let Some((first, last, undo)) = self.effect_replay.take() {
                if undo {
                    self.effect_redo.push((first, last));
                } else {
                    self.effect_undo.push((first, last));
                }
            }
        }
    }
    fn effect_save(&mut self, before: Chain, after: Chain, record: bool) {
        let Some(service) = &self.effects_service else { return };
        let result = if before.enabled != after.enabled {
            if after.enabled { service.enable(&after) } else { service.disable(&after.id) }
        } else if after.enabled
            && (before.target != after.target
                || before.bypass != after.bypass
                || before.bands.len() != after.bands.len()
                || before.bands.iter().zip(&after.bands).any(|(a, b)| a.kind != b.kind))
        {
            service.disable(&after.id).and_then(|_| service.save(&after)).and_then(|_| service.enable(&after))
        } else {
            service.save(&after)
        };
        match result {
            Ok(()) => {
                self.reload_effects();
                if self.selected_effect() == Some(&after) {
                    if before.enabled || after.enabled {
                        self.effect_observing = Some((before, after, record));
                        self.notice = Some("Applying… waiting for the EQ chain to report its state.".into());
                        self.finish_effect_save();
                    } else if record {
                        self.effect_undo.push((before, after));
                        self.effect_redo.clear();
                    }
                } else {
                    self.notice = Some("The effect change was not observed".into());
                }
            }
            Err(error) => {
                self.notice = Some(format!("Effects: {error}"));
                self.reload_effects();
            }
        }
    }
    fn replay_effect(&mut self, undo: bool) {
        self.settle_effect_band();
        if self.effect_pending.is_some() || self.effect_observing.is_some() {
            self.notice = Some("EQ undo waiting for the observed change.".into());
            return;
        }
        let transaction = if undo { self.effect_undo.pop() } else { self.effect_redo.pop() };
        let Some((before, after)) = transaction else { return };
        let (current, desired) = if undo { (&after, &before) } else { (&before, &after) };
        if self.selected_effect() != Some(current) {
            self.notice = Some("EQ chain changed since this undo entry.".into());
            if undo {
                self.effect_undo.push((before, after));
            } else {
                self.effect_redo.push((before, after));
            }
            return;
        }
        let changed_bands: Vec<_> = current
            .bands
            .iter()
            .zip(&desired.bands)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(index, _)| index)
            .collect();
        self.effect_replay = Some((before.clone(), after.clone(), undo));
        if current.enabled
            && changed_bands.len() == 1
            && current.bands.len() == desired.bands.len()
            && current.bands[changed_bands[0]].kind == desired.bands[changed_bands[0]].kind
            && current.target == desired.target
            && current.bypass == desired.bypass
        {
            let index = changed_bands[0];
            self.effect_band_queued(index, desired.bands[index].clone());
        } else {
            self.effect_save(current.clone(), desired.clone(), false);
            if self.effect_observing.is_none() && self.selected_effect() == Some(desired) {
                let (first, last, undo) = self.effect_replay.take().unwrap();
                if undo {
                    self.effect_redo.push((first, last));
                } else {
                    self.effect_undo.push((first, last));
                }
            } else if self.effect_observing.is_none() {
                self.effect_replay = None;
                if undo {
                    self.effect_undo.push((before, after));
                } else {
                    self.effect_redo.push((before, after));
                }
            }
        }
    }
    fn effect_band(&mut self, index: usize, band: Band, cx: &mut Cx<Msg>) {
        self.effect_generation += 1;
        cx.after(Duration::from_millis(500), Msg::EffectSettle(self.effect_generation));
        self.effect_band_queued(index, band);
    }
    fn effect_band_queued(&mut self, index: usize, band: Band) {
        if self.effect_pending.is_some() {
            self.effect_queued = Some((index, band));
            return;
        }
        let Some(before) = self.selected_effect().cloned() else { return };
        if before.bands.get(index) == Some(&band) {
            return;
        }
        let mut after = before.clone();
        let Some(slot) = after.bands.get_mut(index) else { return };
        *slot = band;
        if let Err(error) = after.validate() {
            self.notice = Some(format!("Effects: {error}"));
            return;
        }
        let node_name = format!("musubi_eq.{}", before.id);
        let capture = self
            .graph
            .nodes
            .values()
            .find(|n| n.props.get("node.name") == Some(&node_name))
            .map(|n| Handle { epoch: self.graph.epoch, id: n.id });
        if before.enabled
            && let Some(capture) = capture
        {
            let request = self.next_request;
            self.next_request += 1;
            let band = &after.bands[index];
            let gain = if after.bypass { 0. } else { band.gain };
            let mut params = Vec::with_capacity(6);
            for side in ["left", "right"] {
                for (key, value) in [("Freq", band.freq), ("Q", band.q), ("Gain", gain)] {
                    params.push((format!("{side}{index}:{key}"), value));
                }
            }
            self.effect_pending = Some((before, after, index, request));
            self.send_command(Command::SetEffectParams { node: capture, params, request });
            self.pump_effect_fake();
        } else {
            self.effect_save(before.clone(), after.clone(), false);
            if self.selected_effect() == Some(&after) {
                self.record_effect_band(before, after);
            }
        }
    }
    fn pump_effect_fake(&mut self) {
        if let Some(fake) = &self.fake {
            let events = std::iter::from_fn(|| fake.borrow_mut().next_event()).collect::<Vec<_>>();
            for event in events {
                match event {
                    Event::Delta(delta) => {
                        self.graph.apply(delta);
                        self.finish_effect_edit();
                    }
                    Event::CommandResult { request, result: Err(reason) }
                        if self.effect_pending.as_ref().is_some_and(|pending| pending.3 == request) =>
                    {
                        self.effect_pending = None;
                        self.effect_queued = None;
                        self.effect_replay = None;
                        self.notice = Some(format!("EQ edit failed: {reason}"));
                    }
                    _ => {}
                }
            }
        }
    }
    fn record_effect_band(&mut self, before: Chain, after: Chain) {
        match &mut self.effect_burst {
            Some((first, last)) if first.id == before.id => *last = after,
            _ => {
                self.settle_effect_band();
                self.effect_burst = Some((before, after));
            }
        }
    }
    fn settle_effect_band(&mut self) {
        if let Some((before, after)) = self.effect_burst.take() {
            if before != after {
                self.effect_undo.push((before, after));
                self.effect_redo.clear();
            }
        }
    }
    fn finish_effect_edit(&mut self) {
        let Some((_, after, index, _)) = self.effect_pending.as_ref() else { return };
        let name = format!("musubi_eq.{}", after.id);
        let Some(observed) = self
            .graph
            .nodes
            .values()
            .find(|n| n.props.get("node.name") == Some(&name))
            .map(|n| &n.native.effect_params)
        else {
            return;
        };
        let Some(service) = &self.effects_service else { return };
        match service.save_observed_edit(after, *index, observed) {
            Ok(()) => {
                let (before, after, _, _) = self.effect_pending.take().unwrap();
                self.reload_effects();
                if let Some((first, last, undo)) = self.effect_replay.take() {
                    if self.selected_effect() == Some(&after) {
                        if undo {
                            self.effect_redo.push((first, last));
                        } else {
                            self.effect_undo.push((first, last));
                        }
                    } else {
                        self.notice = Some("EQ undo read-back did not match.".into());
                    }
                } else if self.selected_effect() == Some(&after) {
                    self.record_effect_band(before, after);
                }
                if let Some((index, band)) = self.effect_queued.take() {
                    self.effect_band_queued(index, band);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => {
                self.effect_pending = None;
                self.effect_queued = None;
                self.notice = Some(format!("Effects: {error}"));
            }
        }
    }
    fn send_command(&mut self, command: Command) {
        if let Some(fake) = &self.fake {
            fake.borrow_mut().command(command);
        } else if let Some(backend) = &mut self.backend {
            backend.command(command);
        }
    }
    fn inspector_node(&self) -> Option<Handle> {
        if !self.inspector || self.page != Page::Audio {
            return None;
        }
        let id = self.selected.as_ref().and_then(|stable| match self.graph.resolve(stable) {
            crate::graph::identity::Resolution::Unique(id) => Some(id),
            _ => None,
        })?;
        let port = self.graph.ports.get(&id)?;
        if self.graph.kind(id) != Some(PortKind::Audio) {
            return None;
        }
        let node = port.node_id?;
        self.graph.nodes.contains_key(&node).then_some(Handle { epoch: self.graph.epoch, id: node })
    }
    fn sync_meters(&mut self, cx: &mut Cx<Msg>) {
        let nodes: Vec<_> = if self.window_suspended {
            Vec::new()
        } else if self.page == Page::Mixer && self.mixer_options.meters {
            self.audio
                .as_ref()
                .map(|audio| mixer::model::strips(&audio.snapshot(), &self.graph))
                .unwrap_or_default()
                .into_iter()
                .filter(|strip| {
                    let category = match strip.kind {
                        StripKind::Output => 0,
                        StripKind::Effect => 5,
                        StripKind::Input => 1,
                        StripKind::Playback => 2,
                        StripKind::Recording => 3,
                        StripKind::Native => 4,
                    };
                    self.mixer_options.shown[category]
                        && (self.mixer_filter.is_empty()
                            || strip.name.to_lowercase().contains(&self.mixer_filter.to_lowercase())
                            || strip.subtitle.to_lowercase().contains(&self.mixer_filter.to_lowercase()))
                })
                .filter_map(|strip| strip.node)
                .collect()
        } else {
            self.inspector_node().into_iter().collect()
        };
        if nodes.is_empty() {
            if self.meter_running {
                self.send_command(Command::StopMeters);
                self.meter_running = false;
                self.meter_nodes.clear();
            }
            return;
        }
        if self.meter_running && self.meter_nodes == nodes {
            return;
        }
        self.meter_nodes = nodes.clone();
        self.send_command(Command::StartMeters(nodes));
        self.meter_running = true;
        if !self.meter_tick {
            self.meter_tick = true;
            cx.after(Duration::from_millis(33), Msg::MeterTick);
        }
    }
    fn mixer_write(&mut self, kind: StripKind, control: StripControl, volume: Option<f32>, mute: Option<bool>) {
        match control {
            StripControl::Pulse { index, .. } => {
                let tasks = if let Some(audio) = &self.audio {
                    let mut tasks = Vec::new();
                    if let Some(value) = volume {
                        tasks.push(audio.volume(kind, index, value));
                        if self.linked_channels.contains(&(kind, index)) {
                            tasks.push(audio.balance(kind, index, 0.));
                        }
                    }
                    if let Some(value) = mute {
                        tasks.push(audio.mute(kind, index, value));
                    }
                    tasks
                } else {
                    Vec::new()
                };
                let transaction = self.mixer_current.take();
                let replay = self.mixer_replaying.take();
                for (position, task) in tasks.into_iter().enumerate() {
                    self.audio_pending.push(PendingAudio {
                        task,
                        transaction,
                        replay: if position == 0 { replay } else { None },
                    });
                }
            }
            StripControl::Native { node, channel_volumes, muted } => {
                self.next_request += 1;
                self.native_pending.insert(self.next_request, (self.mixer_current.take(), self.mixer_replaying.take()));
                if let Some(value) = volume {
                    let max = channel_volumes.iter().copied().fold(0_f32, f32::max);
                    let gains = if self.linked_channels.contains(&(kind, node.id)) {
                        vec![value; channel_volumes.len()]
                    } else {
                        channel_volumes.into_iter().map(|v| if max == 0. { value } else { v * value / max }).collect()
                    };
                    self.send_command(Command::SetNodeVolume {
                        node,
                        channel_volumes: gains,
                        request: self.next_request,
                    });
                } else if let (Some(value), Some(_)) = (mute, muted) {
                    self.send_command(Command::SetNodeMute { node, muted: value, request: self.next_request });
                }
            }
            StripControl::ReadOnly(reason) => self.notice = Some(reason),
        }
    }
    fn mixer_identity(
        &self,
        kind: StripKind,
        index: u32,
        snapshot: &aizome_sys::audio::AudioState,
    ) -> Option<MixerIdentity> {
        match kind {
            StripKind::Output | StripKind::Effect | StripKind::Input => {
                let devices = if matches!(kind, StripKind::Output | StripKind::Effect) {
                    &snapshot.sinks
                } else {
                    &snapshot.sources
                };
                devices.iter().find(|d| d.index == index).map(|d| MixerIdentity::Device(kind, d.name.clone()))
            }
            StripKind::Playback | StripKind::Recording => {
                let streams = if kind == StripKind::Playback { &snapshot.playback } else { &snapshot.recording };
                let stream = streams.iter().find(|s| s.index == index)?;
                let serial = self
                    .graph
                    .nodes
                    .values()
                    .find(|n| n.props.get("object.serial").is_some_and(|s| s.parse::<u32>().ok() == Some(index)))?
                    .props
                    .get("object.serial")?
                    .clone();
                Some(MixerIdentity::Stream(kind, stream.application.clone(), stream.media_name.clone(), serial))
            }
            StripKind::Native => self
                .graph
                .nodes
                .get(&index)
                .and_then(|node| node.props.get("node.name"))
                .cloned()
                .map(MixerIdentity::Native),
        }
    }

    fn queue_audio(&mut self, task: AudioTask) {
        let transaction = self.mixer_current.take();
        let replay = self.mixer_replaying.take();
        self.audio_pending.push(PendingAudio { task, transaction, replay });
    }
    fn record_success(&mut self, index: usize) {
        if !self.mixer_failed.contains(&index)
            && self.mixer_succeeded.contains(&index)
            && !self.audio_pending.iter().any(|p| p.transaction == Some(index))
            && !self.native_pending.values().any(|pending| pending.0 == Some(index))
        {
            self.mixer_succeeded.remove(&index);
            self.history.record(index);
        }
    }
    fn finish_audio(&mut self) {
        let mut index = 0;
        while index < self.audio_pending.len() {
            let Some(result) = self.audio_pending[index].task.take() else {
                index += 1;
                continue;
            };
            let pending = self.audio_pending.remove(index);
            match result {
                Ok(()) => {
                    if let Some(transaction) = pending.transaction {
                        if !self.mixer_failed.contains(&transaction) {
                            self.mixer_succeeded.insert(transaction);
                            if self.mixer_pending != Some(transaction) {
                                self.record_success(transaction);
                            }
                        }
                    }
                    if let Some((transaction, undo)) = pending.replay {
                        if undo {
                            self.history.after_undo(Some(transaction));
                        } else {
                            self.history.after_redo(Some(transaction));
                        }
                        self.notice = Some(format!("{} applied.", if undo { "Undo" } else { "Redo" }));
                    }
                }
                Err(reason) => {
                    if let Some(transaction) = pending.transaction {
                        self.mixer_succeeded.remove(&transaction);
                        self.mixer_failed.insert(transaction);
                    }
                    if let Some((transaction, undo)) = pending.replay {
                        self.history.defer(transaction, undo);
                    }
                    self.notice = Some(format!("Audio command failed: {reason}"));
                }
            }
        }
    }
    fn record_mixer(&mut self, intent: MixerIntent, burst: bool, cx: &mut Cx<Msg>) {
        if burst {
            if let Some(index) = self.mixer_pending {
                if let Some(previous) = &mut self.transactions[index].mixer
                    && previous.identity == intent.identity
                    && std::mem::discriminant(&previous.after) == std::mem::discriminant(&intent.after)
                {
                    previous.after = intent.after;
                    self.mixer_current = Some(index);
                    self.mixer_generation += 1;
                    cx.after(Duration::from_millis(500), Msg::MixerSettle(self.mixer_generation));
                    return;
                }
                self.mixer_pending = None;
                self.record_success(index);
            }
        } else if let Some(index) = self.mixer_pending.take() {
            self.record_success(index);
        }
        let index = self.transactions.len();
        self.transactions.push(Transaction { intents: Vec::new(), mixer: Some(intent) });
        self.mixer_current = Some(index);
        if burst {
            self.mixer_pending = Some(index);
            self.mixer_generation += 1;
            cx.after(Duration::from_millis(500), Msg::MixerSettle(self.mixer_generation));
        }
    }

    fn replay_mixer(&mut self, transaction_index: usize, undo: bool) {
        let intent = self.transactions[transaction_index].mixer.as_ref().unwrap().clone();
        let snapshot = self.audio.as_ref().map(|a| a.snapshot()).unwrap_or_default();
        let target = if undo { &intent.before } else { &intent.after };
        if let MixerIdentity::Card(name) = &intent.identity {
            let cards: Vec<_> = snapshot.cards.iter().filter(|card| &card.name == name).collect();
            if cards.len() != 1 {
                self.notice =
                    Some(format!("{} skipped: missing or ambiguous card.", if undo { "Undo" } else { "Redo" }));
                return;
            }
            if let MixerValue::Profile(profile) = target {
                if !cards[0].profiles.iter().any(|p| &p.name == profile && p.available) {
                    self.notice = Some(format!("{} skipped: profile unavailable.", if undo { "Undo" } else { "Redo" }));
                    return;
                }
                if let Some(audio) = &self.audio {
                    let task = audio.profile(cards[0].index, profile);
                    self.audio_pending.push(PendingAudio {
                        task,
                        transaction: None,
                        replay: Some((transaction_index, undo)),
                    });
                }
                return;
            }
        }
        let resolution: Vec<_> = mixer::model::strips(&snapshot, &self.graph)
            .into_iter()
            .filter(|strip| {
                let id = match &strip.control {
                    StripControl::Pulse { index, .. } => *index,
                    StripControl::Native { node, .. } => node.id,
                    StripControl::ReadOnly(_) => return false,
                };
                self.mixer_identity(strip.kind, id, &snapshot).as_ref() == Some(&intent.identity)
            })
            .collect();
        if resolution.len() != 1 {
            self.notice = Some(format!(
                "{} skipped: {} object.",
                if undo { "Undo" } else { "Redo" },
                if resolution.is_empty() { "missing" } else { "ambiguous" }
            ));
            return;
        }
        let strip = &resolution[0];
        let index = match &strip.control {
            StripControl::Pulse { index, .. } => *index,
            StripControl::Native { node, .. } => node.id,
            StripControl::ReadOnly(_) => unreachable!(),
        };
        if matches!(target, MixerValue::Volume(_) | MixerValue::Mute(_)) {
            self.mixer_replaying = Some((transaction_index, undo));
        }
        match target {
            MixerValue::Volume(value) => self.mixer_write(strip.kind, strip.control.clone(), Some(*value), None),
            MixerValue::Mute(value) => self.mixer_write(strip.kind, strip.control.clone(), None, Some(*value)),
            MixerValue::Balance(value) => {
                if let Some(audio) = &self.audio {
                    let task = audio.balance(strip.kind, index, *value);
                    self.audio_pending.push(PendingAudio {
                        task,
                        transaction: None,
                        replay: Some((transaction_index, undo)),
                    });
                }
            }
            MixerValue::Default(name) => {
                let devices = if matches!(strip.kind, StripKind::Output | StripKind::Effect) {
                    &snapshot.sinks
                } else {
                    &snapshot.sources
                };
                if devices.iter().filter(|d| &d.name == name).count() != 1 {
                    self.notice =
                        Some(format!("{} skipped: missing default device.", if undo { "Undo" } else { "Redo" }));
                    return;
                }
                if let Some(audio) = &self.audio {
                    let task = audio.default_device(strip.kind, name);
                    self.audio_pending.push(PendingAudio {
                        task,
                        transaction: None,
                        replay: Some((transaction_index, undo)),
                    });
                }
            }
            MixerValue::Target(name) => {
                let devices = if strip.kind == StripKind::Playback { &snapshot.sinks } else { &snapshot.sources };
                let destinations: Vec<_> = devices.iter().filter(|d| &d.name == name).collect();
                if destinations.len() != 1 {
                    self.notice = Some(format!("{} skipped: missing target.", if undo { "Undo" } else { "Redo" }));
                    return;
                }
                if let Some(audio) = &self.audio {
                    let task = audio.move_stream(strip.kind, index, destinations[0].index);
                    self.audio_pending.push(PendingAudio {
                        task,
                        transaction: None,
                        replay: Some((transaction_index, undo)),
                    });
                }
            }
            MixerValue::Port(name) => {
                let devices = if matches!(strip.kind, StripKind::Output | StripKind::Effect) {
                    &snapshot.sinks
                } else {
                    &snapshot.sources
                };
                if !devices
                    .iter()
                    .any(|d| d.index == index && d.ports.iter().any(|p| &p.name == name && p.available != Some(false)))
                {
                    self.notice = Some(format!("{} skipped: route unavailable.", if undo { "Undo" } else { "Redo" }));
                    return;
                }
                if let Some(audio) = &self.audio {
                    let task = audio.port(strip.kind, index, name);
                    self.audio_pending.push(PendingAudio {
                        task,
                        transaction: None,
                        replay: Some((transaction_index, undo)),
                    });
                }
            }
            MixerValue::Profile(_) => unreachable!("card profiles are replayed above"),
        }
        self.mixer_replaying = None;
    }
    fn commands_for(&mut self, cells: Vec<Cell>, connect: bool, cx: &mut Cx<Msg>) {
        self.notice = None;
        let Some(layout) = self.layouts[Self::index(self.page)].as_ref().cloned() else { return };
        let mut transaction = Transaction::default();
        let mut commands = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for cell in cells {
            let (Some(row), Some(col)) = (layout.row_lines.get(cell.row), layout.col_lines.get(cell.col)) else {
                continue;
            };
            if row.collapsed || col.collapsed {
                if let Some(key) = (if row.collapsed {
                    &layout.rows.groups[layout.rows.group_of(cell.row)]
                } else {
                    &layout.cols.groups[layout.cols.group_of(cell.col)]
                })
                .key
                .clone()
                {
                    self.options[Self::index(self.page)].folded.remove(&key);
                    self.refresh_layouts();
                }
                continue;
            }
            let (out, input) = if layout.swapped { (col, row) } else { (row, col) };
            for index in 0..out.ports.len().max(input.ports.len()) {
                let from = out.ports[index.min(out.ports.len() - 1)];
                let to = input.ports[index.min(input.ports.len() - 1)];
                if !seen.insert((from, to)) {
                    continue;
                }
                let existing = self.graph.links_by_pair.get(&(from, to)).and_then(|ids| ids.iter().next()).copied();
                if connect == existing.is_some() {
                    continue;
                }
                match can_link(&self.graph, from, to) {
                    LinkLegality::Forbidden(reason) => {
                        self.notice = Some(format!("Not allowed: {}", reason_text(reason)));
                        continue;
                    }
                    LinkLegality::NeedsConfirmation(_) if !self.conversion_confirmed => {
                        cx.alert(self.window, Alert::new("MIDI conversion", "PipeWire converts between MIDI 2.0 and MIDI 1.0. Some messages don't survive the conversion.")
                            .default_button("Connect", Msg::Confirm(vec![cell], connect)).cancel_button("Cancel"));
                        continue;
                    }
                    _ => {}
                }
                let (Some(from_id), Some(to_id)) = (
                    self.graph.ports.get(&from).and_then(|p| self.graph.stable_port_id(p)),
                    self.graph.ports.get(&to).and_then(|p| self.graph.stable_port_id(p)),
                ) else {
                    self.notice = Some("Port has no stable identity.".into());
                    continue;
                };
                self.next_request += 1;
                let request = self.next_request;
                let epoch = self.graph.epoch;
                let command = if connect {
                    Command::Connect {
                        out: Handle { epoch, id: from },
                        input: Handle { epoch, id: to },
                        confirmed: self.conversion_confirmed,
                        request,
                    }
                } else {
                    Command::Disconnect { link: Handle { epoch, id: existing.expect("checked above") }, request }
                };
                self.pending.insert(request, Pending { pair: (from, to) });
                self.errors.remove(&(from, to));
                transaction.intents.push(LinkIntent {
                    from: from_id,
                    to: to_id,
                    connect,
                    request,
                    result: None,
                    observed: false,
                });
                commands.push(command);
                if connect {
                    self.removed.remove(&(from, to));
                } else {
                    self.removed.insert((from, to), Instant::now());
                }
            }
        }
        if !transaction.intents.is_empty() {
            let index = self.transactions.len();
            self.transactions.push(transaction);
            self.history.record(index);
            self.refresh_layouts();
            for command in commands {
                self.send_command(command);
            }
            if let Some(fake) = &self.fake {
                let events = std::iter::from_fn(|| fake.borrow_mut().next_event()).collect::<Vec<_>>();
                for event in events {
                    self.update(Msg::Backend(event), cx);
                }
            }
        }
    }
    fn pump_fake(&mut self, cx: &mut Cx<Msg>) {
        if let Some(fake) = &self.fake {
            let events = std::iter::from_fn(|| fake.borrow_mut().next_event()).collect::<Vec<_>>();
            for event in events {
                self.update(Msg::Backend(event), cx);
            }
        }
    }

    /// Execute explicit physical links (inspector and history need not be visible in the matrix).
    fn submit_pairs(&mut self, pairs: Vec<(GlobalId, GlobalId, bool)>, cx: &mut Cx<Msg>) -> Option<usize> {
        let mut transaction = Transaction::default();
        let mut commands = Vec::new();
        for (from, to, connect) in pairs {
            let existing = self.graph.links_by_pair.get(&(from, to)).and_then(|ids| ids.first()).copied();
            if existing.is_some() == connect {
                continue;
            }
            let (Some(from_id), Some(to_id)) = (
                self.graph.ports.get(&from).and_then(|p| self.graph.stable_port_id(p)),
                self.graph.ports.get(&to).and_then(|p| self.graph.stable_port_id(p)),
            ) else {
                continue;
            };
            self.next_request += 1;
            let request = self.next_request;
            let epoch = self.graph.epoch;
            let command = if connect {
                Command::Connect {
                    out: Handle { epoch, id: from },
                    input: Handle { epoch, id: to },
                    confirmed: self.conversion_confirmed,
                    request,
                }
            } else {
                Command::Disconnect { link: Handle { epoch, id: existing.expect("present") }, request }
            };
            self.pending.insert(request, Pending { pair: (from, to) });
            self.errors.remove(&(from, to));
            transaction.intents.push(LinkIntent {
                from: from_id,
                to: to_id,
                connect,
                request,
                result: None,
                observed: false,
            });
            commands.push(command);
            if connect {
                self.removed.remove(&(from, to));
            } else {
                self.removed.insert((from, to), Instant::now());
            }
        }
        if transaction.intents.is_empty() {
            return None;
        }
        let index = self.transactions.len();
        self.transactions.push(transaction);
        self.refresh_layouts();
        for command in commands {
            self.send_command(command);
        }
        self.pump_fake(cx);
        Some(index)
    }

    fn replay(&mut self, undo: bool, cx: &mut Cx<Msg>) {
        let index = if undo { self.history.undo() } else { self.history.redo() };
        let Some(index) = index else { return };
        if self.transactions[index].mixer.is_some() {
            self.replay_mixer(index, undo);
            if !self.audio_pending.iter().any(|pending| pending.replay == Some((index, undo)))
                && !self.native_pending.values().any(|pending| pending.1 == Some((index, undo)))
            {
                self.history.defer(index, undo);
            }
            return;
        }
        if self.transactions[index]
            .intents
            .iter()
            .any(|intent| intent.result.is_none() || (matches!(intent.result, Some(Ok(()))) && !intent.observed))
        {
            self.history.defer(index, undo);
            self.notice = Some(format!("{} waiting for PipeWire.", if undo { "Undo" } else { "Redo" }));
            return;
        }
        let mut pairs = Vec::new();
        let mut skipped = Vec::new();
        for intent in &self.transactions[index].intents {
            if !intent.observed || !matches!(intent.result, Some(Ok(()))) {
                continue;
            }
            let from = self.graph.resolve(&intent.from);
            let to = self.graph.resolve(&intent.to);
            match (from, to) {
                (crate::graph::identity::Resolution::Unique(from), crate::graph::identity::Resolution::Unique(to)) => {
                    pairs.push((from, to, !intent.connect));
                }
                (from, to) => {
                    let reason = if matches!(from, crate::graph::identity::Resolution::Ambiguous(_))
                        || matches!(to, crate::graph::identity::Resolution::Ambiguous(_))
                    {
                        "ambiguous"
                    } else {
                        "missing"
                    };
                    skipped.push(format!("{reason} port"));
                }
            }
        }
        let replay = self.submit_pairs(pairs, cx);
        if undo {
            self.history.after_undo(replay);
        } else {
            self.history.after_redo(replay);
        }
        self.notice = Some(if skipped.is_empty() {
            if undo { "Undo applied.".into() } else { "Redo applied.".into() }
        } else {
            format!("{} skipped: {}.", if undo { "Undo" } else { "Redo" }, skipped.join(", "))
        });
    }
    fn fold(&mut self, key: crate::graph::group::GroupKey, cx: &mut Cx<Msg>) {
        let index = Self::index(self.page);
        let stable = persistence::stable_group(&self.graph, &key);
        let set = &mut self.options[index].folded;
        if !set.insert(key.clone()) {
            set.remove(&key);
            if let Some(stable) = stable {
                self.persisted_folds[index].remove(&stable);
            }
        } else if let Some(stable) = stable {
            self.persisted_folds[index].insert(stable);
        }
        self.refresh_layouts();
        self.persist_later(cx);
    }
    fn context_menu(&self, cell: Cell, column: bool, cx: &mut Cx<Msg>) {
        let Some(layout) = &self.layouts[Self::index(self.page)] else { return };
        let (Some(row), Some(col)) = (layout.row_lines.get(cell.row), layout.col_lines.get(cell.col)) else { return };
        let (out, input) = if layout.swapped { (col, row) } else { (row, col) };
        let (Some(&from), Some(&to)) = (out.ports.first(), input.ports.first()) else { return };
        let mut menu = Menu::new("Patch");
        menu = menu.item(MenuItem::new(
            "Disconnect all from this port",
            Msg::ContextAction(ContextAction::DisconnectFrom(from)),
        ));
        menu = menu
            .item(MenuItem::new("Disconnect all to this port", Msg::ContextAction(ContextAction::DisconnectTo(to))));
        menu =
            menu.item(MenuItem::new("Connect group in order", Msg::ContextAction(ContextAction::ConnectGroup(cell))));
        let group = if column {
            &layout.cols.groups[layout.cols.group_of(cell.col)]
        } else {
            &layout.rows.groups[layout.rows.group_of(cell.row)]
        };
        if let Some(key) = &group.key {
            menu = menu.item(MenuItem::new(
                if group.collapsed { "Expand group" } else { "Collapse group" },
                Msg::ContextAction(ContextAction::Fold(key.clone())),
            ));
        }
        menu = menu.item(MenuItem::new(
            "Copy port name",
            Msg::ContextAction(ContextAction::Copy(if column { to } else { from })),
        ));
        cx.show_menu(self.window, Rect::new(200., 200., 201., 201.), menu, MenuStyle::at_point());
    }
    fn context_action(&mut self, action: ContextAction, cx: &mut Cx<Msg>) {
        match action {
            ContextAction::Fold(key) => self.fold(key, cx),
            ContextAction::Copy(id) => {
                if let Some(name) = self.graph.ports.get(&id).and_then(|port| port.props.get("port.name")) {
                    cx.set_clipboard_text(name);
                }
            }
            ContextAction::ConnectGroup(cell) => {
                if let Some(layout) = &self.layouts[Self::index(self.page)] {
                    let row = layout.rows.groups[layout.rows.group_of(cell.row)].clone();
                    let col = layout.cols.groups[layout.cols.group_of(cell.col)].clone();
                    let cells = (0..(row.end - row.first).min(col.end - col.first))
                        .map(|i| Cell { row: row.first + i, col: col.first + i })
                        .collect();
                    self.commands_for(cells, true, cx);
                }
            }
            ContextAction::DisconnectFrom(id) => self.disconnect_port(id, true, cx),
            ContextAction::DisconnectTo(id) => self.disconnect_port(id, false, cx),
        }
    }
    fn disconnect_port(&mut self, id: GlobalId, from: bool, cx: &mut Cx<Msg>) {
        if let Some(layout) = &self.layouts[Self::index(self.page)] {
            let cells: Vec<_> = self
                .graph
                .links
                .values()
                .filter(|link| if from { link.output == id } else { link.input == id })
                .filter_map(|link| layout.cell_for_pair((link.output, link.input)))
                .collect();
            self.commands_for(cells, false, cx);
        }
    }
    fn kind(&self) -> PortKind {
        match self.page {
            Page::Audio | Page::Mixer | Page::Effects | Page::Devices => PortKind::Audio,
            Page::Midi => PortKind::Midi,
            Page::Midi2 => PortKind::Ump,
            Page::Video => PortKind::Video,
        }
    }
    fn index(page: Page) -> usize {
        match page {
            Page::Audio | Page::Mixer | Page::Effects | Page::Devices => 0,
            Page::Midi => 1,
            Page::Midi2 => 2,
            Page::Video => 3,
        }
    }
    fn refresh_layouts(&mut self) {
        self.layouts = array::from_fn(|i| {
            let page = [Page::Audio, Page::Midi, Page::Midi2, Page::Video][i];
            let mut layout = Layout::from_graph(&self.graph, page, &self.options[i]);
            for link in self.graph.links.values() {
                if let Some(cell) = layout.cell_for_pair((link.output, link.input)) {
                    match &link.state {
                        LinkState::Active => {}
                        LinkState::Negotiating => {
                            layout.cells.insert(cell, CellState::Asked);
                        }
                        LinkState::Error(reason) => {
                            layout.cells.insert(cell, CellState::Error);
                            layout.error_reasons.insert(cell, reason.clone());
                        }
                    }
                }
            }
            for pending in self.pending.values() {
                if let Some(cell) = layout.cell_for_pair(pending.pair) {
                    layout.cells.entry(cell).or_insert(CellState::Asked);
                }
            }
            for (&pair, reason) in &self.errors {
                if let Some(cell) = layout.cell_for_pair(pair) {
                    layout.cells.insert(cell, CellState::Error);
                    layout.error_reasons.insert(cell, reason.clone());
                }
            }
            Some(Rc::new(layout))
        });
    }
    fn counts(&self) -> (usize, usize) {
        let visible = |kind| {
            kind == Some(self.kind())
                || self.page == Page::Midi2 && self.options[2].show_midi_1 && kind == Some(PortKind::Midi)
        };
        let ports = self.graph.ports.values().filter(|port| visible(self.graph.kind(port.id))).count();
        let links = self
            .graph
            .links
            .values()
            .filter(|link| visible(self.graph.kind(link.output)) && visible(self.graph.kind(link.input)))
            .count();
        (ports, links)
    }
    fn hover_sentence(&self) -> Option<String> {
        let cell = self.hovered?;
        let layout = self.layouts[Self::index(self.page)].as_ref()?;
        let (row, col) = (layout.row_lines.get(cell.row)?, layout.col_lines.get(cell.col)?);
        let (out, input) = if layout.swapped { (col, row) } else { (row, col) };
        let (&source, &target) = (out.ports.first()?, input.ports.first()?);
        let (from_name, to_name) = if layout.swapped {
            (
                &layout.cols.groups[layout.cols.group_of(cell.col)].name,
                &layout.rows.groups[layout.rows.group_of(cell.row)].name,
            )
        } else {
            (
                &layout.rows.groups[layout.rows.group_of(cell.row)].name,
                &layout.cols.groups[layout.cols.group_of(cell.col)].name,
            )
        };
        let describe = |id| {
            let port = &self.graph.ports[&id];
            let short = port_label(port);
            let full = port.props.get("port.name").map(String::as_str).unwrap_or("");
            format!("{short} ({full})")
        };
        let state = match layout.state(cell) {
            CellState::Connected => "connected — click to disconnect",
            CellState::Half => "partly connected — click to complete",
            CellState::Asked => "asked — waiting for PipeWire",
            CellState::Error => {
                return Some(format!(
                    "{from_name} · {} → {to_name} · {} · error — {}",
                    describe(source),
                    describe(target),
                    layout.error_reasons.get(&cell).map(String::as_str).unwrap_or("connection refused")
                ));
            }
            CellState::NotAllowed => {
                return Some(format!(
                    "{from_name} · {} → {to_name} · {} · not allowed — {}",
                    describe(source),
                    describe(target),
                    reason_text(match can_link(&self.graph, source, target) {
                        LinkLegality::Forbidden(reason) | LinkLegality::NeedsConfirmation(reason) => reason,
                        LinkLegality::Allowed => Reason::UnknownKind,
                    })
                ));
            }
            CellState::Count(_) => "collapsed — click to expand",
            CellState::Empty => "not connected — click to connect",
        };
        Some(format!("{from_name} · {} → {to_name} · {} · {state}", describe(source), describe(target)))
    }
    pub fn status_sentence(&self) -> Option<String> {
        let chain_status =
            self.effect_selected.as_ref().and_then(|id| self.effect_statuses.iter().find(|(name, _)| name == id)).map(
                |(_, status)| {
                    format!(
                        "{} · {}",
                        self.selected_effect().map_or("EQ chain", |c| c.name.as_str()),
                        effects::page::status_text(status)
                    )
                },
            );
        let error = self
            .effect_selected
            .as_ref()
            .and_then(|id| self.effect_statuses.iter().find(|(name, _)| name == id))
            .is_some_and(|(_, status)| {
                matches!(status, Status::Failed(_) | Status::StartLimit | Status::TargetMissing | Status::NodesMissing)
            });
        let sentence = if self.page == Page::Effects && error {
            chain_status
        } else {
            self.notice.clone().or_else(|| {
                if self.page == Page::Effects {
                    chain_status
                } else if self.page == Page::Mixer {
                    self.mixer_hover.clone()
                } else {
                    self.hover_sentence()
                }
            })
        };
        sentence.map(|sentence| fit_status(&sentence))
    }

    fn unavailable(&self) -> Element<Msg> {
        container(
            column![
                text(self.page.title()).size(28.).semibold(),
                text("PipeWire isn't answering. Musubi keeps trying.").size(20.).role(Role::Text2).wrap(),
            ]
            .spacing(12.),
        )
        .align_x(Align::Center)
        .align_y(Align::Center)
        .padding(36.)
        .width(Fill)
        .height(Fill)
        .into()
    }
    fn select_cell(&mut self, cell: Cell, column: bool) {
        let Some(layout) = self.layout(self.page) else { return };
        let line = if column { layout.col_lines.get(cell.col) } else { layout.row_lines.get(cell.row) };
        self.selected = line
            .and_then(|line| line.ports.first())
            .and_then(|id| self.graph.ports.get(id))
            .and_then(|port| self.graph.stable_port_id(port));
    }

    fn inspector_view(&self) -> Element<Msg> {
        let mut body: Vec<Element<Msg>> = vec![text("INSPECTOR").size(11.).semibold().role(Role::Text2).into()];
        let selected = self.selected.as_ref().and_then(|id| match self.graph.resolve(id) {
            crate::graph::identity::Resolution::Unique(id) => self.graph.ports.get(&id),
            _ => None,
        });
        if let Some(port) = selected {
            let group = self
                .graph
                .groups(port.direction)
                .into_iter()
                .find(|group| group.ports.contains(&port.id))
                .map(|group| group.label)
                .unwrap_or_default();
            body.push(text(port_label(port)).size(19.).semibold().wrap().into());
            body.push(
                text(format!("{:?} · {group}", self.graph.kind(port.id).unwrap_or(PortKind::Other)))
                    .size(12.)
                    .role(Role::Text2)
                    .wrap()
                    .into(),
            );
            for (heading, outgoing) in [("Goes to", true), ("Comes from", false)] {
                let mut links: Vec<_> = self
                    .graph
                    .links
                    .values()
                    .filter(|link| if outgoing { link.output == port.id } else { link.input == port.id })
                    .collect();
                links.sort_by_key(|link| (link.output, link.input));
                if links.is_empty() {
                    continue;
                }
                body.push(text(heading).mono().size(10.5).letter_spacing(1.2).caps().role(Role::Text3).into());
                for link in links {
                    let other = if outgoing { link.input } else { link.output };
                    let name = routes::port_name(&self.graph, other);
                    body.push(
                        row![
                            text(name).size(12.).wrap().width(Fill),
                            button("Unlink")
                                .compact()
                                .on_press(Msg::Unlink(link.output, link.input))
                                .id(format!("unlink:{}:{}", link.output, link.input))
                                .height(24.),
                        ]
                        .spacing(4.)
                        .into(),
                    );
                }
            }
            if self.graph.kind(port.id) == Some(PortKind::Audio) {
                let reading = self.inspector_node().and_then(|node| {
                    if let Some(fake) = &self.fake {
                        fake.borrow().meters().into_iter().find(|r| r.node == node)
                    } else {
                        self.backend.as_ref().and_then(|backend| backend.meters().into_iter().find(|r| r.node == node))
                    }
                });
                let channel = port
                    .props
                    .get("port.id")
                    .and_then(|id| id.parse::<usize>().ok())
                    .or_else(|| {
                        port.props.get("audio.channel").and_then(|name| {
                            name.strip_prefix("AUX").and_then(|index| index.parse::<usize>().ok()).or_else(
                                || match name.as_str() {
                                    "FL" | "MONO" => Some(0),
                                    "FR" => Some(1),
                                    _ => None,
                                },
                            )
                        })
                    })
                    .unwrap_or(0);
                let peaks: &[f32] =
                    reading.as_ref().and_then(|r| r.peaks.get(channel)).map(std::slice::from_ref).unwrap_or(&[]);
                let peak = peaks.first().copied().unwrap_or(0.);
                let peak_text =
                    if peak > 0. { format!("{:.1} dB", 20. * peak.log10()) } else { "−∞ dB".to_owned() };
                body.push(text("LEVEL").mono().size(10.5).role(Role::Text3).into());
                body.push(
                    row![
                        Element::new(mixer::MeterScale, vec![]).width(32.).height(150.),
                        mixer::LevelMeter::with_state::<Msg>(
                            peaks,
                            1,
                            Instant::now(),
                            reading.as_ref().is_some_and(|r| r.paused).then_some("paused"),
                        )
                        .id("inspector-meter")
                        .width(32.)
                        .height(150.),
                    ]
                    .spacing(8.)
                    .into(),
                );
                body.push(text("−∞").mono().size(10.5).role(Role::Text3).into());
                body.push(text(format!("Peak {peak_text}")).mono().size(11.).role(Role::Text2).into());
            }
            body.push(text("FACTS").mono().size(10.5).letter_spacing(1.2).role(Role::Text3).into());
            let node = port.node_id.and_then(|id| self.graph.nodes.get(&id));
            let device = node.and_then(|node| node.device_id).and_then(|id| self.graph.devices.get(&id));
            let client = node.and_then(|node| node.client_id).and_then(|id| self.graph.clients.get(&id));
            let facts = [
                ("Node", node.and_then(|n| n.props.get("node.name"))),
                ("Port", port.props.get("port.name")),
                ("Alias", port.props.get("port.alias")),
                ("Channel", port.props.get("audio.channel")),
                ("Format", port.props.get("format.dsp")),
                (
                    "Device",
                    device.and_then(|d| d.props.get("device.description").or_else(|| d.props.get("device.name"))),
                ),
                ("Client", client.and_then(|c| c.props.get("application.name").or_else(|| c.props.get("client.name")))),
                ("Serial", port.props.get("object.serial")),
            ];
            for (name, value) in facts {
                body.push(text(name).size(11.).role(Role::Text3).into());
                body.push(
                    text(value.map(String::as_str).unwrap_or("—"))
                        .size(11.)
                        .mono()
                        .wrap()
                        .selectable()
                        .width(Fill)
                        .into(),
                );
            }
        } else {
            body.push(text("Select a port in the matrix.").role(Role::Text2).wrap().into());
        }
        container(scroll(column(body).spacing(7.).padding(12.).width(Fill)).width(Fill).height(Fill))
            .fill(Tone::Surface)
            .width(260.)
            .height(Fill)
            .id("inspector")
            .into()
    }
}

fn reason_text(reason: Reason) -> &'static str {
    match reason {
        Reason::MissingPort => "port no longer present",
        Reason::WrongDirection => "both ports must run output → input",
        Reason::UnknownKind => "port format is unknown",
        Reason::IncompatibleKinds => "media kinds are incompatible",
        Reason::MidiConversion => "MIDI conversion needs confirmation",
        Reason::IncompatibleVideoFormat => "video format families differ",
    }
}

impl Program for Musubi {
    type Msg = Msg;

    fn info(&self) -> AppInfo {
        AppInfo::new("org.aizome.Musubi", "Musubi", "Patchbay").icon(Icon::Music)
    }

    fn launched(&mut self, cx: &mut Cx<Msg>) {
        self.persist_enabled = self.live || self.paths.is_some();
        if self.live {
            self.paths = persistence::Paths::from_environment();
        }
        if self.persist_enabled {
            self.options = persistence::load_options(self.paths.as_ref());
            self.mixer_options = persistence::load_mixer(self.paths.as_ref());
            self.persisted_folds = persistence::load_folds(self.paths.as_ref());
        }
        self.window = cx.open_window(WindowSpec::new("Musubi — Patchbay").size(1280., 800.).min_size(900., 560.));
        if let Some(instance) = &self.instance {
            let sender = cx.stream();
            if let Ok(listener) = instance.listener.try_clone() {
                instance::serve(listener, move || sender.send(Msg::Activated));
            }
        }
        if self.live {
            let sender = cx.stream();
            match PipeWireBackend::start(move |event| {
                sender.send(Msg::Backend(event));
            }) {
                Ok(backend) => self.backend = Some(backend),
                Err(reason) => self.connection = Some(Connection::Down(reason)),
            }
            self.audio = Some(Box::new(aizome_sys::audio::Audio::new(aizome_sys::Wake::new({
                let sender = cx.stream();
                move || {
                    sender.send(Msg::AudioWake);
                }
            }))));
        }
        if self.live {
            match (effects::paths::Paths::from_environment(), Systemd::user()) {
                (Ok(paths), Ok(units)) => self.effects_service = Some(EffectsService { paths, units: Box::new(units) }),
                (Err(error), _) => self.notice = Some(format!("Effects: {error}")),
                (_, Err(error)) => self.notice = Some(format!("Effects: {error}")),
            }
        }
        self.reload_effects();
        if let Some(fake) = &self.fake {
            let events = std::iter::from_fn(|| fake.borrow_mut().next_event()).collect::<Vec<_>>();
            for event in events {
                self.update(Msg::Backend(event), cx);
            }
        }
    }

    fn update(&mut self, msg: Msg, cx: &mut Cx<Msg>) {
        if !matches!(
            &msg,
            Msg::AudioWake | Msg::Backend(_) | Msg::Persist(_) | Msg::Matrix(MatrixMsg::Hover(_)) | Msg::MixerHover(_)
        ) {
            self.notice = None;
        }
        if self.effect_observing.is_some() {
            self.refresh_effect_statuses();
        }
        match msg {
            Msg::EffectSelect(id) => {
                self.settle_effect_band();
                self.effect_selected = Some(id);
                self.effect_name_input = self.selected_effect().map_or_else(String::new, |c| c.name.clone());
                self.effect_values = Default::default();
            }
            Msg::EffectNew => {
                let Some(service) = &self.effects_service else { return };
                let mut number = 1;
                let id = loop {
                    let id = format!("eq-{number}");
                    if !self.effect_chains.iter().any(|c| c.id == id) {
                        break id;
                    }
                    number += 1;
                };
                let chain = Chain {
                    id: id.clone(),
                    name: format!("EQ {number}"),
                    target: Target::FollowDefault,
                    bands: vec![Band { kind: BandKind::Peaking, freq: 1000., gain: 0., q: 1. }],
                    bypass: false,
                    enabled: false,
                };
                match service.save(&chain) {
                    Ok(()) => {
                        self.effect_selected = Some(id);
                        self.reload_effects();
                    }
                    Err(error) => self.notice = Some(format!("Effects: {error}")),
                }
            }
            Msg::EffectNameInput(value) => self.effect_name_input = value,
            Msg::EffectNameCommit => {
                if let Some(before) = self.selected_effect().cloned() {
                    if before.name != self.effect_name_input {
                        let mut after = before.clone();
                        after.name = self.effect_name_input.clone();
                        self.effect_save(before, after, true);
                    }
                }
            }
            Msg::EffectTargetMenu => self.effect_target_menu = !self.effect_target_menu,
            Msg::EffectPresetMenu => self.effect_preset_menu = !self.effect_preset_menu,
            Msg::EffectPageMenu => self.effect_page_menu = !self.effect_page_menu,
            Msg::EffectTarget(target) => {
                self.effect_target_menu = false;
                if let Some(before) = self.selected_effect().cloned() {
                    if before.target != target {
                        let mut after = before.clone();
                        after.target = target;
                        self.effect_save(before, after, true);
                    }
                }
            }
            Msg::EffectPreset(preset) => {
                self.effect_preset_menu = false;
                if let Some(before) = self.selected_effect().cloned() {
                    let mut after = before.clone();
                    after.bands =
                        preset.as_ref().and_then(|id| self.effect_chains.iter().find(|c| &c.id == id)).map_or_else(
                            || vec![Band { kind: BandKind::Peaking, freq: 1000., gain: 0., q: 1. }],
                            |c| c.bands.clone(),
                        );
                    if before != after {
                        self.effect_save(before, after, true);
                    }
                }
            }
            Msg::EffectEnable(enabled) => {
                if let Some(before) = self.selected_effect().cloned() {
                    let mut after = before.clone();
                    after.enabled = enabled;
                    self.effect_save(before, after, true);
                }
            }
            Msg::EffectBypass(bypass) => {
                if let Some(before) = self.selected_effect().cloned() {
                    let mut after = before.clone();
                    after.bypass = bypass;
                    self.effect_save(before, after, true);
                }
            }
            Msg::EffectBand(index, band) => self.effect_band(index, band, cx),
            Msg::EffectBandEnd => {
                self.effect_generation += 1;
                cx.after(Duration::from_millis(50), Msg::EffectSettle(self.effect_generation));
            }
            Msg::EffectSettle(generation) if generation == self.effect_generation => {
                if self.effect_pending.is_some() || self.effect_queued.is_some() {
                    cx.after(Duration::from_millis(50), Msg::EffectSettle(generation));
                } else {
                    self.settle_effect_band();
                }
            }
            Msg::EffectSettle(_) => {}
            Msg::EffectBandType(index, kind) => {
                if let Some(band) = self.selected_effect().and_then(|c| c.bands.get(index)).cloned() {
                    let mut band = band;
                    band.kind = match kind {
                        1 => BandKind::LowShelf,
                        2 => BandKind::HighShelf,
                        _ => BandKind::Peaking,
                    };
                    self.effect_band(index, band, cx);
                }
            }
            Msg::EffectValueInput(index, field, value) => {
                if field < 3 {
                    self.effect_value_band = index;
                    self.effect_values[field] = value;
                }
            }
            Msg::EffectValueCommit(index, field) => {
                if let Some(mut band) = self.selected_effect().and_then(|c| c.bands.get(index)).cloned() {
                    let text = self.effect_values.get(field).map(String::as_str).unwrap_or("");
                    if let Ok(value) = text.parse::<f32>() {
                        match field {
                            0 => band.freq = value,
                            1 => band.gain = value,
                            2 => band.q = value,
                            _ => {}
                        }
                        self.effect_band(index, band, cx);
                    } else if !text.is_empty() {
                        self.notice = Some("Enter a numeric EQ value".into());
                    }
                    if field < 3 {
                        self.effect_values[field].clear();
                    }
                }
            }
            Msg::EffectAddBand => {
                if let Some(before) = self.selected_effect().cloned() {
                    if before.bands.len() < 8 {
                        let mut after = before.clone();
                        after.bands.push(Band { kind: BandKind::Peaking, freq: 1000., gain: 0., q: 1. });
                        self.effect_save(before, after, true);
                    }
                }
            }
            Msg::EffectDelete => {
                if let (Some(chain), Some(service)) = (self.selected_effect(), &self.effects_service) {
                    cx.alert(
                        self.window,
                        Alert::new(
                            "Delete EQ chain",
                            format!(
                                "Remove “{}”, its unit {}, and files {} and {}?",
                                chain.name,
                                effects::service::unit(&chain.id).unwrap_or_default(),
                                service.paths.json(&chain.id).unwrap().display(),
                                service.paths.conf(&chain.id).unwrap().display()
                            ),
                        )
                        .default_button("Delete chain", Msg::EffectDeleteConfirmed(chain.id.clone()))
                        .cancel_button("Cancel"),
                    );
                }
            }
            Msg::EffectDeleteConfirmed(id) => {
                if let Some(service) = &self.effects_service {
                    match service.delete(&id) {
                        Ok(()) => {
                            if self.effect_selected.as_deref() == Some(&id) {
                                self.effect_selected = None;
                            }
                            self.effect_undo.clear();
                            self.effect_redo.clear();
                            self.effect_burst = None;
                            self.reload_effects();
                        }
                        Err(error) => self.notice = Some(format!("Effects: {error}")),
                    }
                }
            }
            Msg::EffectRemoveAll => {
                self.effect_page_menu = false;
                if let Some(service) = &self.effects_service {
                    match service.removal_manifest() {
                        Ok(resources) => {
                            let description = if resources.is_empty() {
                                "No Musubi effects or clock settings found.".to_owned()
                            } else {
                                format!(
                                    "Stop and disable these units and remove these paths:\n{}\nOther applications' files are not touched.",
                                    resources.join("\n")
                                )
                            };
                            cx.alert(
                                self.window,
                                Alert::new("Remove Musubi effects and clock settings", description)
                                    .default_button("Remove all", Msg::EffectRemoveAllConfirmed(resources))
                                    .cancel_button("Cancel"),
                            );
                        }
                        Err(error) => self.notice = Some(format!("Effects: {error}")),
                    }
                }
            }
            Msg::EffectRemoveAllConfirmed(resources) => {
                if let Some(service) = &self.effects_service {
                    if !matches!(service.removal_manifest(), Ok(current) if current == resources) {
                        self.notice = Some("Effects changed since confirmation; review the list again.".into());
                        return;
                    }
                    match service.remove_all() {
                        Ok(()) => {
                            self.effect_selected = None;
                            self.effect_undo.clear();
                            self.effect_redo.clear();
                            self.reload_effects();
                        }
                        Err(error) => self.notice = Some(format!("Effects: {error}")),
                    }
                }
            }
            Msg::AudioWake => {
                self.finish_audio();
                self.sync_meters(cx);
            }
            Msg::MeterTick => {
                self.meter_tick = false;
                if self.meter_running && !self.window_suspended {
                    cx.after(Duration::from_millis(33), Msg::MeterTick);
                    self.meter_tick = true;
                }
            }
            Msg::MixerPercent(value) => {
                self.mixer_options.percent = value;
                self.persist_later(cx);
            }
            Msg::MixerMeters(value) => {
                self.mixer_options.meters = value;
                self.sync_meters(cx);
                self.persist_later(cx);
            }
            Msg::MixerShow => self.mixer_show = !self.mixer_show,
            Msg::MixerShown(index, shown) => {
                self.mixer_options.shown[index] = shown;
                self.sync_meters(cx);
                self.persist_later(cx);
            }
            Msg::MixerFilter(filter) => {
                self.mixer_filter = filter;
                self.sync_meters(cx);
            }
            Msg::MixerHover(text) => self.mixer_hover = text,
            Msg::MixerSettle(generation) => {
                if generation == self.mixer_generation {
                    if let Some(index) = self.mixer_pending.take() {
                        self.record_success(index);
                    }
                }
            }
            Msg::MixerVolume(kind, control, value) => {
                let snapshot = self.audio.as_ref().map(|a| a.snapshot()).unwrap_or_default();
                let index = match &control {
                    StripControl::Pulse { index, .. } => *index,
                    StripControl::Native { node, .. } => node.id,
                    StripControl::ReadOnly(_) => 0,
                };
                let before = match &control {
                    StripControl::Pulse { volume, .. } => *volume,
                    StripControl::Native { channel_volumes, .. } => {
                        channel_volumes.iter().copied().fold(0_f32, f32::max)
                    }
                    StripControl::ReadOnly(_) => value,
                };
                if (before - value).abs() > 1e-6 {
                    if let Some(identity) = self.mixer_identity(kind, index, &snapshot) {
                        self.record_mixer(
                            MixerIntent {
                                identity,
                                before: MixerValue::Volume(before),
                                after: MixerValue::Volume(value),
                            },
                            true,
                            cx,
                        );
                    }
                    self.mixer_write(kind, control, Some(value), None);
                }
            }
            Msg::MixerMute(kind, control, value) => {
                let snapshot = self.audio.as_ref().map(|a| a.snapshot()).unwrap_or_default();
                let (index, before) = match &control {
                    StripControl::Pulse { index, muted, .. } => (*index, Some(*muted)),
                    StripControl::Native { node, muted, .. } => (node.id, *muted),
                    StripControl::ReadOnly(_) => (0, None),
                };
                if let Some(before) = before.filter(|before| *before != value) {
                    if let Some(identity) = self.mixer_identity(kind, index, &snapshot) {
                        self.record_mixer(
                            MixerIntent { identity, before: MixerValue::Mute(before), after: MixerValue::Mute(value) },
                            false,
                            cx,
                        );
                    }
                    self.mixer_write(kind, control, None, Some(value));
                }
            }
            Msg::MixerBalance(kind, index, value) => {
                let snapshot = self.audio.as_ref().map(|a| a.snapshot()).unwrap_or_default();
                let devices = if matches!(kind, StripKind::Output | StripKind::Effect) {
                    &snapshot.sinks
                } else {
                    &snapshot.sources
                };
                if let Some(device) = devices.iter().find(|d| d.index == index) {
                    if (device.balance - value).abs() > 1e-6 {
                        self.record_mixer(
                            MixerIntent {
                                identity: MixerIdentity::Device(kind, device.name.clone()),
                                before: MixerValue::Balance(device.balance),
                                after: MixerValue::Balance(value),
                            },
                            true,
                            cx,
                        );
                        if let Some(audio) = &self.audio {
                            let task = audio.balance(kind, index, value);
                            self.queue_audio(task);
                        }
                    }
                }
            }
            Msg::MixerDefault(kind, name) => {
                let snapshot = self.audio.as_ref().map(|a| a.snapshot()).unwrap_or_default();
                let devices = if matches!(kind, StripKind::Output | StripKind::Effect) {
                    &snapshot.sinks
                } else {
                    &snapshot.sources
                };
                let previous = devices.iter().find(|d| d.is_default);
                if let Some(previous) = previous.filter(|d| d.name != name) {
                    if devices.iter().any(|d| d.name == name) {
                        self.record_mixer(
                            MixerIntent {
                                identity: MixerIdentity::Device(kind, name.clone()),
                                before: MixerValue::Default(previous.name.clone()),
                                after: MixerValue::Default(name.clone()),
                            },
                            false,
                            cx,
                        );
                        if let Some(audio) = &self.audio {
                            let task = audio.default_device(kind, &name);
                            self.queue_audio(task);
                        }
                    }
                }
            }
            Msg::MixerRouteMenu(kind, index) => {
                self.mixer_route_menu =
                    if self.mixer_route_menu == Some((kind, index)) { None } else { Some((kind, index)) };
            }
            Msg::DeviceSelect(name) => {
                self.device_selected = Some(name);
                self.device_profile_menu = false;
                self.mixer_route_menu = None;
            }
            Msg::DeviceProfileMenu => self.device_profile_menu = !self.device_profile_menu,
            Msg::DeviceProfile(name, profile) => {
                self.device_profile_menu = false;
                let snapshot = self.audio.as_ref().map(|audio| audio.snapshot()).unwrap_or_default();
                let cards: Vec<_> = snapshot.cards.iter().filter(|card| card.name == name).collect();
                if cards.len() == 1
                    && cards[0].profiles.iter().any(|choice| choice.name == profile && choice.available)
                    && cards[0].active_profile.as_deref() != Some(&profile)
                {
                    if let Some(previous) = &cards[0].active_profile {
                        self.record_mixer(
                            MixerIntent {
                                identity: MixerIdentity::Card(name),
                                before: MixerValue::Profile(previous.clone()),
                                after: MixerValue::Profile(profile.clone()),
                            },
                            false,
                            cx,
                        );
                    }
                    if let Some(audio) = &self.audio {
                        let task = audio.profile(cards[0].index, &profile);
                        self.queue_audio(task);
                    }
                }
            }
            Msg::DeviceShowMixer(id) => {
                if self.graph.nodes.contains_key(&id) {
                    self.page = Page::Mixer;
                    self.mixer_filter = self.graph.nodes[&id]
                        .props
                        .get("node.description")
                        .or_else(|| self.graph.nodes[&id].props.get("node.name"))
                        .cloned()
                        .unwrap_or_default();
                    self.sync_meters(cx);
                }
            }
            Msg::DeviceShowAudio(id) => {
                if let Some(port) = self
                    .graph
                    .ports
                    .values()
                    .filter(|port| port.node_id == Some(id) && self.graph.kind(port.id) == Some(PortKind::Audio))
                    .min_by_key(|port| port.id)
                {
                    self.selected = self.graph.stable_port_id(port);
                    self.page = Page::Audio;
                    self.inspector = true;
                    self.sync_meters(cx);
                }
            }
            Msg::MixerRoute(kind, index, port) => {
                self.mixer_route_menu = None;
                let snapshot = self.audio.as_ref().map(|a| a.snapshot()).unwrap_or_default();
                let devices = if matches!(kind, StripKind::Output | StripKind::Effect) {
                    &snapshot.sinks
                } else {
                    &snapshot.sources
                };
                if let Some(device) = devices.iter().find(|d| d.index == index)
                    && let Some(route) =
                        device.ports.iter().find(|route| route.name == port && route.available != Some(false))
                    && device.active_port.as_deref() != Some(route.name.as_str())
                {
                    if let Some(previous) = &device.active_port {
                        self.record_mixer(
                            MixerIntent {
                                identity: MixerIdentity::Device(kind, device.name.clone()),
                                before: MixerValue::Port(previous.clone()),
                                after: MixerValue::Port(port.clone()),
                            },
                            false,
                            cx,
                        );
                    }
                    if let Some(audio) = &self.audio {
                        let task = audio.port(kind, index, &port);
                        self.queue_audio(task);
                    }
                }
            }
            Msg::MixerLink(kind, index) => {
                if !self.linked_channels.remove(&(kind, index)) {
                    self.linked_channels.insert((kind, index));
                    if let Some(audio) = &self.audio {
                        if matches!(kind, StripKind::Output | StripKind::Effect | StripKind::Input) {
                            let task = audio.balance(kind, index, 0.);
                            self.queue_audio(task);
                        }
                    }
                }
            }
            Msg::MixerMove(kind, index, target) => {
                let snapshot = self.audio.as_ref().map(|a| a.snapshot()).unwrap_or_default();
                let streams = if kind == StripKind::Playback { &snapshot.playback } else { &snapshot.recording };
                let devices = if kind == StripKind::Playback { &snapshot.sinks } else { &snapshot.sources };
                if let Some(stream) = streams.iter().find(|s| s.index == index && s.device != target) {
                    if let (Some(old), Some(new), Some(identity)) = (
                        devices.iter().find(|d| d.index == stream.device),
                        devices.iter().find(|d| d.index == target),
                        self.mixer_identity(kind, index, &snapshot),
                    ) {
                        self.record_mixer(
                            MixerIntent {
                                identity,
                                before: MixerValue::Target(old.name.clone()),
                                after: MixerValue::Target(new.name.clone()),
                            },
                            false,
                            cx,
                        );
                        if let Some(audio) = &self.audio {
                            let task = audio.move_stream(kind, index, target);
                            self.queue_audio(task);
                        }
                    }
                }
            }
            Msg::Page(page) => {
                self.page = page;
                self.hovered = None;
                self.mixer_hover = None;
                self.matrix_focus.set(None);
                self.sync_meters(cx);
            }
            Msg::Activated => cx.activate(self.window),
            Msg::Matrix(MatrixMsg::Hover(cell)) => self.hovered = cell,
            Msg::Matrix(MatrixMsg::HeaderSize(row, size)) => {
                let options = &mut self.options[Self::index(self.page)];
                options.full_names = false;
                if row {
                    options.row_header = size
                } else {
                    options.column_header = size
                };
                self.refresh_layouts();
                self.persist_later(cx);
            }
            Msg::Matrix(MatrixMsg::FitHeader(row)) => {
                if let Some(layout) = self.layout(self.page) {
                    let (width, height) = layout.fitted_headers();
                    let options = &mut self.options[Self::index(self.page)];
                    if row {
                        options.row_header = width
                    } else {
                        options.column_header = height
                    };
                    options.full_names = false;
                    self.refresh_layouts();
                    self.persist_later(cx);
                }
            }
            Msg::Matrix(MatrixMsg::Focus(cell)) => {
                self.select_cell(cell, false);
                self.sync_meters(cx);
            }
            Msg::Matrix(MatrixMsg::Inspect(cell)) => {
                self.select_cell(cell, false);
                self.inspector = true;
                self.sync_meters(cx);
            }
            Msg::Matrix(MatrixMsg::Select(cell, column)) => {
                self.select_cell(cell, column);
                self.sync_meters(cx);
            }
            Msg::Matrix(MatrixMsg::Toggle(cell)) => {
                let state = self.layouts[Self::index(self.page)].as_ref().map(|l| l.state(cell));
                self.commands_for(vec![cell], state != Some(CellState::Connected), cx);
            }
            Msg::Matrix(MatrixMsg::Line(cells, connect)) => self.commands_for(cells, connect, cx),
            Msg::ConfirmRoute(from, to) => {
                self.conversion_confirmed = true;
                self.connect_route(from, to, cx);
            }
            Msg::List(list) => {
                self.options[Self::index(self.page)].list = list;
                self.persist_later(cx);
            }
            Msg::ShowMidi1(show) => {
                self.options[2].show_midi_1 = show;
                self.refresh_layouts();
                self.persist_later(cx);
            }
            Msg::SelectRoutes(indices) => {
                let rows = self.route_rows();
                self.route_selection =
                    indices.into_iter().filter_map(|i| rows.get(i).map(|r| (r.from, r.to))).collect();
            }
            Msg::SortRoutes(sort) => self.route_sort = sort,
            Msg::InspectRoute(index) => {
                if let Some(route) = self.route_rows().get(index) {
                    self.selected = self.graph.ports.get(&route.from).and_then(|p| self.graph.stable_port_id(p));
                    self.inspector = true;
                }
            }
            Msg::DeleteRoutes => {
                let pairs: Vec<_> = self.route_selection.iter().map(|&(a, b)| (a, b, false)).collect();
                if let Some(index) = self.submit_pairs(pairs, cx) {
                    self.history.record(index);
                }
                self.route_selection.clear();
            }
            Msg::AddRoute => {
                if self.route_sheet.is_none() {
                    self.route_sheet = Some(cx.show_sheet(self.window));
                    self.route_from_filter = self.options[Self::index(self.page)].from.clone();
                    self.route_to_filter = self.options[Self::index(self.page)].to.clone();
                    self.route_from = self.layout(self.page).and_then(|layout| {
                        routes::endpoints(&self.graph, layout, true, &self.route_from_filter).first().map(|v| v.0)
                    });
                    self.route_to = self.layout(self.page).and_then(|layout| {
                        routes::endpoints(&self.graph, layout, false, &self.route_to_filter).first().map(|v| v.0)
                    });
                    cx.focus("route-from-search");
                }
            }
            Msg::CloseRoute => {
                if let Some(sheet) = self.route_sheet.take() {
                    cx.close_window(sheet);
                }
            }
            Msg::RouteFromFilter(value) => {
                self.route_from_filter = value;
                self.route_from = None;
            }
            Msg::RouteToFilter(value) => {
                self.route_to_filter = value;
                self.route_to = None;
            }
            Msg::RouteFromFocus => {
                if self.route_from.is_none() {
                    self.route_from = self.layout(self.page).and_then(|layout| {
                        routes::endpoints(&self.graph, layout, true, &self.route_from_filter).first().map(|v| v.0)
                    });
                }
            }
            Msg::RouteToFocus => {
                if self.route_to.is_none() {
                    self.route_to = self.layout(self.page).and_then(|layout| {
                        routes::endpoints(&self.graph, layout, false, &self.route_to_filter).first().map(|v| v.0)
                    });
                }
            }
            Msg::RouteFromActivate(id) => {
                self.route_from = Some(id);
                self.update(Msg::ConnectRoute, cx);
            }
            Msg::RouteToActivate(id) => {
                self.route_to = Some(id);
                self.update(Msg::ConnectRoute, cx);
            }
            Msg::RouteFromSelected(value) => self.route_from = value,
            Msg::RouteToSelected(value) => self.route_to = value,
            Msg::ConnectRoute => {
                if let (Some(from), Some(to)) = (self.route_from, self.route_to) {
                    self.update(Msg::CloseRoute, cx);
                    self.connect_route(from, to, cx);
                }
            }
            Msg::Persist(generation) if generation == self.persist_generation => {
                if let Err(error) = persistence::save_options(self.paths.as_ref(), &self.options, &self.mixer_options)
                    .and_then(|_| {
                        persistence::save_folds(self.paths.as_ref(), &self.graph, &self.options, &self.persisted_folds)
                    })
                {
                    self.notice = Some(format!("Could not save view settings: {error}"));
                }
            }
            Msg::Persist(_) => {}
            Msg::DisconnectSelection => {
                let Some(id) = self.selected.as_ref().and_then(|id| match self.graph.resolve(id) {
                    crate::graph::identity::Resolution::Unique(id) => Some(id),
                    _ => None,
                }) else {
                    return;
                };
                let pairs = self
                    .graph
                    .links
                    .values()
                    .filter(|l| l.output == id || l.input == id)
                    .map(|l| (l.output, l.input, false))
                    .collect();
                if let Some(index) = self.submit_pairs(pairs, cx) {
                    self.history.record(index);
                }
            }
            Msg::ConnectInOrder => {
                if let Some(layout) = self.layout(self.page) {
                    let cells = (0..layout.row_lines.len().min(layout.col_lines.len()))
                        .map(|i| Cell { row: i, col: i })
                        .collect();
                    self.commands_for(cells, true, cx);
                }
            }
            Msg::CollapseAll => {
                let keys: Vec<_> = self
                    .graph
                    .groups(crate::graph::Direction::Output)
                    .into_iter()
                    .chain(self.graph.groups(crate::graph::Direction::Input))
                    .map(|g| g.key)
                    .collect();
                for key in keys {
                    self.options[Self::index(self.page)].folded.insert(key.clone());
                    if let Some(stable) = persistence::stable_group(&self.graph, &key) {
                        self.persisted_folds[Self::index(self.page)].insert(stable);
                    }
                }
                self.refresh_layouts();
                self.persist_later(cx);
            }
            Msg::Matrix(MatrixMsg::Context(cell, column)) => self.context_menu(cell, column, cx),
            Msg::Confirm(cells, connect) => {
                self.conversion_confirmed = true;
                self.commands_for(cells, connect, cx);
            }
            Msg::ContextAction(action) => self.context_action(action, cx),
            Msg::Pairs(pairs) => {
                self.options[Self::index(self.page)].pairs = pairs;
                self.refresh_layouts();
                self.persist_later(cx);
            }
            Msg::Monitors(monitors) => {
                self.options[Self::index(self.page)].monitors = monitors;
                self.refresh_layouts();
                self.persist_later(cx);
            }
            Msg::Swap => {
                let options = &mut self.options[Self::index(self.page)];
                options.swap = !options.swap;
                self.refresh_layouts();
                self.persist_later(cx);
            }
            Msg::FullNames => {
                let options = &mut self.options[Self::index(self.page)];
                options.full_names = !options.full_names;
                self.refresh_layouts();
                self.persist_later(cx);
            }
            Msg::From(from) => {
                self.options[Self::index(self.page)].from = from;
                self.refresh_layouts();
            }
            Msg::To(to) => {
                self.options[Self::index(self.page)].to = to;
                self.refresh_layouts();
            }
            Msg::Undo if self.page == Page::Effects => self.replay_effect(true),
            Msg::Redo if self.page == Page::Effects => self.replay_effect(false),
            Msg::Undo => self.replay(true, cx),
            Msg::Redo => self.replay(false, cx),
            Msg::Inspector => {
                self.inspector = !self.inspector;
                self.sync_meters(cx);
            }
            Msg::Unlink(from, to) => {
                if let Some(index) = self.submit_pairs(vec![(from, to, false)], cx) {
                    self.history.record(index);
                }
            }
            Msg::Backend(Event::ClockDefaults { rate, quantum }) => {
                self.clock_defaults = Some((rate, quantum));
            }
            Msg::Backend(Event::Connection(connection)) => {
                if matches!(connection, Connection::Down(_)) {
                    self.graph = Graph::default();
                    self.linked_channels.clear();
                    self.pending.clear();
                    self.errors.clear();
                    self.removed.clear();
                    self.clock_defaults = None;
                    self.refresh_layouts();
                }
                self.connection = Some(connection);
            }
            Msg::Backend(Event::Delta(delta)) => {
                for change in &delta.upserts {
                    if let crate::graph::Upsert::Link(link) = change {
                        let pair = (link.output, link.input);
                        if let Some(removed) = self.removed.remove(&pair) {
                            if removed.elapsed() < Duration::from_secs(2) {
                                let name = |id| {
                                    self.graph.ports.get(&id).map(port_label).unwrap_or_else(|| format!("port {id}"))
                                };
                                self.notice = Some(format!(
                                    "WirePlumber connected {} to {} again: it follows the default output.",
                                    name(pair.0),
                                    name(pair.1)
                                ));
                            }
                        }
                    }
                }
                self.graph.apply(delta);
                self.refresh_effect_statuses();
                self.finish_effect_edit();
                let snapshot = self.audio.as_ref().map(|audio| audio.snapshot()).unwrap_or_default();
                let existing: HashSet<_> = mixer::model::strips(&snapshot, &self.graph)
                    .iter()
                    .filter_map(|strip| {
                        let id = match strip.control {
                            StripControl::Pulse { index, .. } => index,
                            _ => strip.node?.id,
                        };
                        Some((strip.kind, id))
                    })
                    .collect();
                self.linked_channels.retain(|key| existing.contains(key));
                if self.persist_enabled {
                    persistence::restore_folds(&self.graph, &mut self.options, &self.persisted_folds);
                }
                for transaction in &mut self.transactions {
                    for intent in &mut transaction.intents {
                        let (
                            crate::graph::identity::Resolution::Unique(from),
                            crate::graph::identity::Resolution::Unique(to),
                        ) = (self.graph.resolve(&intent.from), self.graph.resolve(&intent.to))
                        else {
                            continue;
                        };
                        if intent.result.is_some()
                            && (if intent.connect {
                                self.graph.has_active_link((from, to))
                            } else {
                                !self.graph.links_by_pair.contains_key(&(from, to))
                            })
                        {
                            intent.observed = true;
                        }
                    }
                }
                self.pending.retain(|&request, pending| {
                    !self.transactions.iter().flat_map(|t| &t.intents).any(|intent| {
                        intent.request == request
                            && intent.result.is_some()
                            && (if intent.connect {
                                self.graph.has_active_link(pending.pair)
                            } else {
                                !self.graph.links_by_pair.contains_key(&pending.pair)
                            })
                    })
                });
                self.refresh_layouts();
                self.sync_meters(cx);
            }
            Msg::Backend(Event::CommandResult { request, result }) => {
                if self.effect_pending.as_ref().is_some_and(|pending| pending.3 == request) {
                    if let Err(reason) = &result {
                        self.effect_pending = None;
                        self.effect_queued = None;
                        self.effect_replay = None;
                        self.notice = Some(format!("EQ edit failed: {reason}"));
                    }
                }
                for intent in self.transactions.iter_mut().flat_map(|t| &mut t.intents) {
                    if intent.request == request {
                        intent.result = Some(result.clone());
                    }
                }
                if let Some((transaction, replay)) = self.native_pending.remove(&request) {
                    match &result {
                        Ok(()) => {
                            if let Some(transaction) = transaction {
                                self.mixer_succeeded.insert(transaction);
                                if self.mixer_pending != Some(transaction) {
                                    self.record_success(transaction);
                                }
                            }
                            if let Some((transaction, undo)) = replay {
                                if undo {
                                    self.history.after_undo(Some(transaction));
                                } else {
                                    self.history.after_redo(Some(transaction));
                                }
                                self.notice = Some(format!("{} applied.", if undo { "Undo" } else { "Redo" }));
                            }
                        }
                        Err(reason) => {
                            if let Some(transaction) = transaction {
                                self.mixer_succeeded.remove(&transaction);
                                self.mixer_failed.insert(transaction);
                            }
                            if let Some((transaction, undo)) = replay {
                                self.history.defer(transaction, undo);
                            }
                            self.notice = Some(format!("Native control failed: {reason}"));
                        }
                    }
                }
                if let Err(reason) = result {
                    if let Some(pending) = self.pending.remove(&request) {
                        self.errors.insert(pending.pair, reason.clone());
                        self.notice = Some(reason);
                    }
                } else if let Some(pending) = self.pending.get(&request) {
                    let observed = self.graph.has_active_link(pending.pair);
                    if let Some(intent) =
                        self.transactions.iter_mut().flat_map(|t| &mut t.intents).find(|i| i.request == request)
                    {
                        intent.observed = observed == intent.connect;
                        if intent.observed {
                            self.pending.remove(&request);
                        }
                    }
                }
                self.refresh_layouts();
            }
        }
        self.finish_audio();
    }

    fn window_state(&mut self, window: WindowKey, state: aizome_kit::WindowState, cx: &mut Cx<Msg>) {
        if window == self.window {
            self.window_suspended = state.suspended;
            self.sync_meters(cx);
        }
    }

    fn view(&self, window: WindowKey) -> Element<Msg> {
        if self.route_sheet == Some(window) {
            let (from, to) = self
                .layout(self.page)
                .map(|layout| {
                    (
                        routes::endpoints(&self.graph, layout, true, &self.route_from_filter),
                        routes::endpoints(&self.graph, layout, false, &self.route_to_filter),
                    )
                })
                .unwrap_or_default();
            return routes::sheet(
                from,
                to,
                &self.route_from_filter,
                &self.route_to_filter,
                self.route_from,
                self.route_to,
            );
        }
        let mut side = column![
            row![
                container(aizome_kit::icon(Icon::Music).size(18.).role(Role::Accent))
                    .padding(5.)
                    .fill(Tone::Chrome)
                    .border(Edge::Line),
                column![text("Musubi").size(15.).weight(700.), text("結び · Patchbay").size(11.).role(Role::Text3)]
                    .spacing(1.),
            ]
            .spacing(10.)
            .padding([14., 12., 12., 12.]),
            divider().edge(Edge::Line),
            caption("PATCHBAY"),
        ]
        .spacing(8.)
        .width(Fill);
        for (page, icon, title) in
            Page::ALL.into_iter().filter(|(page, _, _)| !matches!(page, Page::Mixer | Page::Effects | Page::Devices))
        {
            let count = self
                .graph
                .links
                .values()
                .filter(|link| {
                    self.graph.kind(link.output).is_some_and(|kind| page.accepts(kind))
                        && self.graph.kind(link.input).is_some_and(|kind| page.accepts(kind))
                })
                .count();
            side = side.push(
                row![
                    sidebar_item(icon, title, self.page == page).on_press(Msg::Page(page)).id(title).width(Fill),
                    text(count.to_string()).mono().size(11.).role(Role::Text3),
                ]
                .spacing(4.)
                .padding([0., 10.])
                .width(Fill),
            );
        }
        let strip_count =
            self.audio.as_ref().map_or(0, |audio| mixer::model::strips(&audio.snapshot(), &self.graph).len());
        side = side.push(caption("MIXER")).push(
            row![
                sidebar_item(Icon::SlidersHorizontal, "Mixer", self.page == Page::Mixer)
                    .on_press(Msg::Page(Page::Mixer))
                    .id("Mixer")
                    .width(Fill),
                text(strip_count.to_string()).mono().size(11.).role(Role::Text3),
            ]
            .spacing(4.)
            .padding([0., 10.])
            .width(Fill),
        );
        if self.effects_service.is_some() {
            side = side.push(
                row![
                    sidebar_item(Icon::Activity, "Effects", self.page == Page::Effects)
                        .on_press(Msg::Page(Page::Effects))
                        .id("Effects")
                        .width(Fill),
                    text(self.effect_chains.len().to_string()).mono().size(11.).role(Role::Text3),
                ]
                .spacing(4.)
                .padding([0., 10.])
                .width(Fill),
            );
        }
        let cards = self.audio.as_ref().map_or(0, |audio| audio.snapshot().cards.len());
        side = side.push(caption("SYSTEM")).push(
            row![
                sidebar_item(Icon::Usb, "Devices", self.page == Page::Devices)
                    .on_press(Msg::Page(Page::Devices))
                    .id("Devices")
                    .width(Fill),
                text(cards.to_string()).mono().size(11.).role(Role::Text3),
            ]
            .spacing(4.)
            .padding([0., 10.])
            .width(Fill),
        );
        let version = match &self.connection {
            Some(Connection::Up(version)) => format!("PipeWire {version}"),
            _ => "PipeWire".to_owned(),
        };
        let status = if matches!(self.connection, Some(Connection::Up(_))) { "●" } else { "○" };
        let clock = self.clock_defaults.map(|(rate, quantum)| {
            format!(
                "Defaults · {} kHz · {quantum} · {:.1} ms",
                rate as f64 / 1000.,
                quantum as f64 * 1000. / rate as f64
            )
        });
        side = side.push(spacer()).push(
            container(
                column![
                    row![
                        text(status).role(if status == "●" { Role::Success } else { Role::Danger }),
                        text(version).size(12.)
                    ]
                    .spacing(7.),
                    text(clock.unwrap_or_else(|| "Defaults · unavailable".to_owned())).size(10.).role(Role::Text3),
                ]
                .spacing(3.),
            )
            .padding(12.)
            .edges(Edge::Line, Sides::TOP)
            .width(Fill),
        );
        let (ports, links) = self.counts();
        let content: Element<Msg> = if self.page == Page::Mixer {
            if let Some(audio) = &self.audio {
                let snapshot = audio.snapshot();
                let strips = mixer::model::strips(&snapshot, &self.graph);
                let readings = if self.meter_running {
                    if let Some(fake) = &self.fake {
                        fake.borrow().meters()
                    } else {
                        self.backend.as_ref().map_or_else(Vec::new, Backend::meters)
                    }
                } else {
                    Vec::new()
                };
                mixer::page::view(
                    &strips,
                    &snapshot,
                    &readings,
                    Instant::now(),
                    &self.linked_channels,
                    self.mixer_route_menu,
                    self.mixer_options.percent,
                    self.mixer_options.meters,
                    &self.mixer_options.shown,
                    &self.mixer_filter,
                )
            } else {
                self.unavailable()
            }
        } else if self.page == Page::Effects {
            let outputs = self
                .audio
                .as_ref()
                .map(|a| a.snapshot().sinks.iter().map(|d| (d.name.clone(), d.description.clone())).collect::<Vec<_>>())
                .unwrap_or_default();
            let external = self
                .graph
                .nodes
                .values()
                .filter(|node| {
                    node.props.get("media.class").is_some_and(|class| class == "Audio/Filter")
                        && !node.props.get("node.name").is_some_and(|name| name.starts_with("musubi_eq."))
                })
                .map(|node| {
                    node.props
                        .get("node.description")
                        .or_else(|| node.props.get("node.name"))
                        .cloned()
                        .unwrap_or_else(|| format!("Node {}", node.id))
                })
                .collect::<Vec<_>>();
            effects::page::view(
                &self.effect_chains,
                self.effect_selected.as_deref(),
                &self.effect_statuses,
                &outputs,
                &external,
                self.effect_target_menu,
                self.effect_preset_menu,
                self.effect_page_menu,
                &self.effect_name_input,
                &self.effect_values,
            )
        } else if self.page == Page::Devices {
            self.audio.as_ref().map_or_else(
                || self.unavailable(),
                |audio| {
                    devices::view(
                        &audio.snapshot(),
                        &self.graph,
                        self.device_selected.as_deref(),
                        self.device_profile_menu,
                        self.mixer_route_menu,
                    )
                },
            )
        } else if self.matrix_spike.is_none() && ports == 0 && matches!(self.connection, Some(Connection::Up(_))) {
            let explanation = match self.page {
                Page::Video => "No video ports. Cameras appear here when PipeWire's camera support finds them.",
                Page::Audio => "No audio ports. Audio devices and applications appear here when PipeWire finds them.",
                Page::Midi => "No MIDI ports. MIDI devices and applications appear here when PipeWire finds them.",
                Page::Midi2 => "No MIDI 2.0 ports. UMP devices and applications appear here when PipeWire finds them.",
                Page::Mixer | Page::Effects | Page::Devices => unreachable!("content handled above"),
            };
            container(text(explanation).size(20.).role(Role::Text2).wrap())
                .align_x(Align::Center)
                .align_y(Align::Center)
                .padding(36.)
                .width(Fill)
                .height(Fill)
                .into()
        } else if let Some(layout) = self.matrix_spike.as_ref().filter(|_| self.page == Page::Audio).or_else(|| {
            self.layouts[Self::index(self.page)].as_ref().filter(|_| matches!(self.connection, Some(Connection::Up(_))))
        }) {
            if self.options[Self::index(self.page)].list {
                let rows = self.route_rows();
                let selected = rows
                    .iter()
                    .enumerate()
                    .filter_map(|(i, row)| self.route_selection.contains(&(row.from, row.to)).then_some(i))
                    .collect();
                column![
                    row![
                        button("Add Route…").on_press(Msg::AddRoute).id("add-route"),
                        button("Delete Selected").on_press(Msg::DeleteRoutes).enabled(!self.route_selection.is_empty())
                    ]
                    .spacing(8.)
                    .padding(10.),
                    routes::view(
                        rows,
                        selected,
                        self.route_sort,
                        self.page == Page::Midi2 && self.options[2].show_midi_1
                    ),
                ]
                .width(Fill)
                .height(Fill)
                .into()
            } else {
                PatchMatrix::new(
                    layout.clone(),
                    self.matrix_focus.clone(),
                    self.selected.as_ref().and_then(|stable| match self.graph.resolve(stable) {
                        crate::graph::identity::Resolution::Unique(id) => layout
                            .row_lines
                            .iter()
                            .position(|line| line.ports.contains(&id))
                            .map(|index| (false, index))
                            .or_else(|| {
                                layout
                                    .col_lines
                                    .iter()
                                    .position(|line| line.ports.contains(&id))
                                    .map(|index| (true, index))
                            }),
                        _ => None,
                    }),
                    Msg::Matrix,
                )
                .id("matrix")
                .width(Fill)
                .height(Fill)
            }
        } else {
            self.unavailable()
        };
        let links_text = format!("{links} {}", if links == 1 { "link" } else { "links" });
        let title = format!("{} {ports} ports · {links_text}", self.page.title());
        let short_title = format!("{} · {links_text}", self.page.title());
        let options = &self.options[Self::index(self.page)];
        let available = matches!(self.connection, Some(Connection::Up(_))) || self.matrix_spike.is_some();
        let has_ports = available && ports > 0;
        let controls: Element<Msg> = Element::new(
            ResponsiveToolbar(ToolbarConfig {
                title: if self.page == Page::Mixer {
                    format!("Mixer · {strip_count} strips")
                } else if self.page == Page::Effects {
                    format!("Effects · {}", chains_label(self.effect_chains.len()))
                } else if self.page == Page::Devices {
                    format!("Devices · {cards} {}", if cards == 1 { "device" } else { "devices" })
                } else {
                    title
                },
                short_title: if self.page == Page::Mixer {
                    format!("Mixer · {strip_count}")
                } else if self.page == Page::Effects {
                    format!("Effects · {}", self.effect_chains.len())
                } else if self.page == Page::Devices {
                    format!("Devices · {cards}")
                } else {
                    short_title
                },
                page: self.page,
                list: options.list,
                pairs: options.pairs,
                monitors: options.monitors,
                swap: options.swap,
                full_names: options.full_names,
                show_midi_1: options.show_midi_1,
                from: options.from.clone(),
                to: options.to.clone(),
                has_ports,
                can_undo: if self.page == Page::Effects {
                    !self.effect_undo.is_empty()
                } else {
                    self.history.undo_label(&self.transactions).is_some()
                },
                mixer: self.mixer_options.clone(),
                mixer_filter: self.mixer_filter.clone(),
                mixer_show: self.mixer_show,
                effects_available: self.effects_service.is_some(),
            }),
            vec![],
        )
        .width(Fill);
        let status = if let Some(Connection::Down(error)) = &self.connection {
            error.clone()
        } else {
            self.status_sentence().unwrap_or_default()
        };
        let right = if self.page == Page::Mixer {
            let snapshot = self.audio.as_ref().map(|audio| audio.snapshot()).unwrap_or_default();
            let strips = mixer::model::strips(&snapshot, &self.graph);
            let hidden = strips
                .iter()
                .filter(|strip| {
                    let category = match strip.kind {
                        StripKind::Output => 0,
                        StripKind::Effect => 5,
                        StripKind::Input => 1,
                        StripKind::Playback => 2,
                        StripKind::Recording => 3,
                        StripKind::Native => 4,
                    };
                    !self.mixer_options.shown[category]
                        || (!self.mixer_filter.is_empty()
                            && !strip.name.to_lowercase().contains(&self.mixer_filter.to_lowercase())
                            && !strip.subtitle.to_lowercase().contains(&self.mixer_filter.to_lowercase()))
                })
                .count();
            if !self.mixer_filter.is_empty() || hidden > 0 {
                format!("{hidden} strips hidden")
            } else {
                let default =
                    snapshot.sinks.iter().find(|d| d.is_default).map(|d| d.description.as_str()).unwrap_or("none");
                format!("{} strips · default output {default}", strips.len())
            }
        } else if self.page == Page::Devices {
            format!("{cards} {}", if cards == 1 { "device" } else { "devices" })
        } else if self.page == Page::Effects {
            chains_label(self.effect_chains.len())
        } else if !options.from.is_empty() {
            format!("From “{}” hides {} ports", options.from, self.layout(self.page).map_or(0, |l| l.hidden_from))
        } else if !options.to.is_empty() {
            format!("To “{}” hides {} ports", options.to, self.layout(self.page).map_or(0, |l| l.hidden_to))
        } else {
            format!("{ports} ports")
        };
        let body: Element<Msg> = if self.inspector && !matches!(self.page, Page::Mixer | Page::Effects | Page::Devices)
        {
            row![content, self.inspector_view()].height(Fill).into()
        } else {
            content
        };
        let main = column![controls, body, status_bar(vec![status_text(status), spacer().into(), status_text(right)]),]
            .height(Fill);
        sidebar_layout(side, main).into()
    }

    fn menus(&self) -> MenuBar<Msg> {
        let mut view = Menu::new("View");
        for (i, (page, _, title)) in Page::ALL.iter().enumerate() {
            view = view.item(
                MenuItem::new(*title, Msg::Page(*page))
                    .shortcut(Shortcut::cmd(char::from(b'1' + i as u8)))
                    .checked(self.page == *page),
            );
        }
        let options = &self.options[Self::index(self.page)];
        view = view
            .separator()
            .item(MenuItem::new("Matrix", Msg::List(false)).checked(!options.list))
            .item(MenuItem::new("List", Msg::List(true)).checked(options.list))
            .item(MenuItem::new("Pairs", Msg::Pairs(true)).checked(options.pairs).enabled(self.page == Page::Audio))
            .item(
                MenuItem::new("Channels", Msg::Pairs(false)).checked(!options.pairs).enabled(self.page == Page::Audio),
            )
            .item(
                MenuItem::new("Show Monitors", Msg::Monitors(!options.monitors))
                    .checked(options.monitors)
                    .enabled(self.page == Page::Audio),
            )
            .item(
                MenuItem::new("Show MIDI 1.0", Msg::ShowMidi1(!options.show_midi_1))
                    .checked(options.show_midi_1)
                    .enabled(self.page == Page::Midi2),
            )
            .item(MenuItem::new("Swap Axes", Msg::Swap).shortcut(Shortcut::cmd('S').alt()).checked(options.swap))
            .item(
                MenuItem::new("Show Full Names", Msg::FullNames)
                    .shortcut(Shortcut::cmd('F').alt())
                    .checked(options.full_names),
            )
            .item(MenuItem::new("Collapse All Groups", Msg::CollapseAll));
        let edit = Menu::new("Edit")
            .item(
                MenuItem::new(
                    format!("Undo {}", self.history.undo_label(&self.transactions).unwrap_or("Connect")),
                    Msg::Undo,
                )
                .role(MenuRole::Undo)
                .shortcut(Shortcut::cmd('Z'))
                .enabled(self.history.undo_label(&self.transactions).is_some()),
            )
            .item(
                MenuItem::new(
                    format!("Redo {}", self.history.redo_label(&self.transactions).unwrap_or("Connect")),
                    Msg::Redo,
                )
                .role(MenuRole::Redo)
                .shortcut(Shortcut::cmd('Z').shift())
                .enabled(self.history.redo_label(&self.transactions).is_some()),
            );
        let edit = edit
            .item(
                MenuItem::new("Delete Selected Routes", Msg::DeleteRoutes)
                    .shortcut(Shortcut::key(aizome_kit::menu::ShortcutKey::Delete))
                    .enabled(options.list && !self.route_selection.is_empty()),
            )
            .item(
                MenuItem::new("Remove Selected Routes", Msg::DeleteRoutes)
                    .shortcut(Shortcut::key(aizome_kit::menu::ShortcutKey::Backspace))
                    .enabled(options.list && !self.route_selection.is_empty()),
            );
        view = view.item(
            MenuItem::new("Inspector", Msg::Inspector).shortcut(Shortcut::cmd('I').alt()).checked(self.inspector),
        );
        let routing = Menu::new("Routing")
            .item(
                MenuItem::new("Disconnect All from Selection", Msg::DisconnectSelection)
                    .enabled(self.selected.is_some()),
            )
            .item(MenuItem::new("Connect in Order", Msg::ConnectInOrder))
            .item(MenuItem::new("Add Route…", Msg::AddRoute));
        MenuBar::standard(&self.info()).replace("Edit", edit).with_menu(view).with_menu(routing)
    }
}

fn chains_label(count: usize) -> String {
    format!("{count} {}", if count == 1 { "chain" } else { "chains" })
}
