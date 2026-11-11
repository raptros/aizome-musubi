use crate::{
    backend::{Backend, Command, Connection, Event, Handle, RequestId, fake::FakeBackend, pipewire::PipeWireBackend},
    graph::{
        GlobalId, Graph,
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
};
use aizome_kit::{Rect, column, prelude::*, row};
use std::{
    array,
    cell::RefCell,
    collections::HashMap,
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
}

impl Page {
    const ALL: [(Page, Icon, &str); 4] = [
        (Page::Audio, Icon::Volume2, "Audio"),
        (Page::Midi, Icon::Music, "MIDI"),
        (Page::Midi2, Icon::Keyboard, "MIDI 2.0"),
        (Page::Video, Icon::Video, "Video"),
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Audio => "Audio",
            Page::Midi => "MIDI",
            Page::Midi2 => "MIDI 2.0",
            Page::Video => "Video",
        }
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
}

#[derive(Clone)]
struct Pending {
    pair: (GlobalId, GlobalId),
}

#[derive(Clone, Debug)]
pub enum Msg {
    Page(Page),
    Activated,
    Matrix(MatrixMsg),
    Backend(Event),
    Confirm(Vec<Cell>, bool),
    ContextAction(ContextAction),
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
    pub graph: Graph,
    connection: Option<Connection>,
    matrix_spike: Option<Rc<Layout>>,
    pub options: [ViewOptions; 4],
    layouts: [Option<Rc<Layout>>; 4],
    hovered: Option<Cell>,
    pub transactions: Vec<Transaction>,
    pending: HashMap<RequestId, Pending>,
    errors: HashMap<(GlobalId, GlobalId), String>,
    next_request: RequestId,
    conversion_confirmed: bool,
    removed: HashMap<(GlobalId, GlobalId), Instant>,
    notice: Option<String>,
    window: WindowKey,
}

impl Musubi {
    /// Synthetic geometry exists only in Harness tests, never in the shipped app.
    pub fn for_matrix_test(layout: Rc<Layout>) -> Self {
        Self { matrix_spike: Some(layout), ..Self::default() }
    }
    pub fn layout(&self, page: Page) -> Option<&Layout> {
        self.layouts[Self::index(page)].as_deref()
    }
    /// The Harness and fake backend use the same command path as the live app.
    pub fn with_fake(fake: Rc<RefCell<FakeBackend>>) -> Self {
        Self { fake: Some(fake), ..Self::default() }
    }
    fn send_command(&mut self, command: Command) {
        if let Some(fake) = &self.fake {
            fake.borrow_mut().command(command);
        } else if let Some(backend) = &mut self.backend {
            backend.command(command);
        }
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
            self.transactions.push(transaction);
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
    fn fold(&mut self, key: crate::graph::group::GroupKey) {
        let set = &mut self.options[Self::index(self.page)].folded;
        if !set.insert(key.clone()) {
            set.remove(&key);
        }
        self.refresh_layouts();
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
            ContextAction::Fold(key) => self.fold(key),
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
            Page::Audio => PortKind::Audio,
            Page::Midi => PortKind::Midi,
            Page::Midi2 => PortKind::Ump,
            Page::Video => PortKind::Video,
        }
    }
    fn index(page: Page) -> usize {
        match page {
            Page::Audio => 0,
            Page::Midi => 1,
            Page::Midi2 => 2,
            Page::Video => 3,
        }
    }
    fn refresh_layouts(&mut self) {
        self.layouts = array::from_fn(|i| {
            let page = [Page::Audio, Page::Midi, Page::Midi2, Page::Video][i];
            let mut layout = Layout::from_graph(&self.graph, page, &self.options[i]);
            for pending in self.pending.values() {
                if let Some(cell) = layout.cell_for_pair(pending.pair) {
                    layout.cells.insert(cell, CellState::Asked);
                }
            }
            for &pair in self.errors.keys() {
                if let Some(cell) = layout.cell_for_pair(pair) {
                    layout.cells.insert(cell, CellState::Error);
                }
            }
            Some(Rc::new(layout))
        });
    }
    fn counts(&self) -> (usize, usize) {
        let kind = self.kind();
        let ports = self.graph.ports_of_kind(kind).count();
        let links = self
            .graph
            .links
            .values()
            .filter(|link| self.graph.kind(link.output) == Some(kind) && self.graph.kind(link.input) == Some(kind))
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
                    self.errors.get(&(source, target)).map(String::as_str).unwrap_or("connection refused")
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
        self.notice.clone().or_else(|| self.hover_sentence())
    }
    fn unavailable(&self) -> Element<Msg> {
        container(
            column![
                text(self.page.title()).size(28.).semibold(),
                text(match &self.connection {
                    Some(Connection::Down(_)) => "PipeWire unavailable.",
                    _ => "Not connected to PipeWire yet.",
                })
                .size(20.)
                .role(Role::Text2),
                text("Musubi keeps trying to reach the server.").role(Role::Text2).wrap(),
            ]
            .spacing(12.),
        )
        .padding(36.)
        .width(Fill)
        .height(Fill)
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
        }
        if let Some(fake) = &self.fake {
            let events = std::iter::from_fn(|| fake.borrow_mut().next_event()).collect::<Vec<_>>();
            for event in events {
                self.update(Msg::Backend(event), cx);
            }
        }
    }

    fn update(&mut self, msg: Msg, cx: &mut Cx<Msg>) {
        match msg {
            Msg::Page(page) => {
                self.page = page;
                self.hovered = None;
            }
            Msg::Activated => cx.activate(self.window),
            Msg::Matrix(MatrixMsg::Hover(cell)) => self.hovered = cell,
            Msg::Matrix(MatrixMsg::Focus(_)) => {}
            Msg::Matrix(MatrixMsg::Toggle(cell)) => {
                let state = self.layouts[Self::index(self.page)].as_ref().map(|l| l.state(cell));
                self.commands_for(vec![cell], state != Some(CellState::Connected), cx);
            }
            Msg::Matrix(MatrixMsg::Line(cells, connect)) => self.commands_for(cells, connect, cx),
            Msg::Matrix(MatrixMsg::Context(cell, column)) => self.context_menu(cell, column, cx),
            Msg::Confirm(cells, connect) => {
                self.conversion_confirmed = true;
                self.commands_for(cells, connect, cx);
            }
            Msg::ContextAction(action) => self.context_action(action, cx),
            Msg::Backend(Event::Connection(connection)) => {
                if matches!(connection, Connection::Down(_)) {
                    self.graph = Graph::default();
                    self.pending.clear();
                    self.errors.clear();
                    self.removed.clear();
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
                            && self.graph.links_by_pair.contains_key(&(from, to)) == intent.connect
                        {
                            intent.observed = true;
                        }
                    }
                }
                self.pending.retain(|&request, pending| {
                    !self.transactions.iter().flat_map(|t| &t.intents).any(|intent| {
                        intent.request == request
                            && intent.result.is_some()
                            && self.graph.links_by_pair.contains_key(&pending.pair) == intent.connect
                    })
                });
                self.refresh_layouts();
            }
            Msg::Backend(Event::CommandResult { request, result }) => {
                for intent in self.transactions.iter_mut().flat_map(|t| &mut t.intents) {
                    if intent.request == request {
                        intent.result = Some(result.clone());
                    }
                }
                if let Err(reason) = result {
                    if let Some(pending) = self.pending.remove(&request) {
                        self.errors.insert(pending.pair, reason.clone());
                        self.notice = Some(reason);
                    }
                } else if let Some(pending) = self.pending.get(&request) {
                    let observed = self.graph.links_by_pair.contains_key(&pending.pair);
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
    }

    fn view(&self, _window: WindowKey) -> Element<Msg> {
        let mut side = column![
            container(
                row![
                    container(aizome_kit::icon(Icon::Music).size(24.)).padding(8.).fill(Tone::Sunk),
                    column![text("Musubi").size(20.).semibold(), text("結び · Patchbay").size(12.).role(Role::Text2),]
                        .spacing(4.),
                ]
                .spacing(10.),
            )
            .padding([9., 4., 12., 4.])
            .width(Fill),
            caption("PATCHBAY"),
        ]
        .spacing(8.)
        .padding([14., 8., 0., 8.]);
        for (page, icon, title) in Page::ALL {
            side = side.push(sidebar_item(icon, title, self.page == page).on_press(Msg::Page(page)).id(title));
        }
        let (ports, links) = self.counts();
        let content: Element<Msg> = if self.matrix_spike.is_none()
            && ports == 0
            && matches!(self.connection, Some(Connection::Up(_)))
        {
            let explanation = match self.page {
                Page::Video => "No video ports. Cameras appear here when PipeWire's camera support finds them.",
                Page::Audio => "No audio ports. Audio devices and applications appear here when PipeWire finds them.",
                Page::Midi => "No MIDI ports. MIDI devices and applications appear here when PipeWire finds them.",
                Page::Midi2 => "No MIDI 2.0 ports. UMP devices and applications appear here when PipeWire finds them.",
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
            PatchMatrix::new(layout.clone(), Msg::Matrix).id("matrix").width(Fill).height(Fill)
        } else {
            self.unavailable()
        };
        let title = format!("{} {ports} ports · {links} links", self.page.title());
        let status = self.status_sentence().unwrap_or_else(|| match &self.connection {
            Some(Connection::Up(_)) => "PipeWire: connected".to_owned(),
            Some(Connection::Down(error)) => error.clone(),
            _ => "PipeWire: not connected".to_owned(),
        });
        let main =
            column![toolbar(vec![text(title).semibold().into()]), content, status_bar(vec![status_text(status)]),]
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
        MenuBar::standard(&self.info()).with_menu(view)
    }
}
