use crate::{
    backend::{Connection, Event, pipewire::PipeWireBackend},
    graph::{Graph, classify::PortKind, group::port_label},
    instance,
    matrix::{
        Layout, MatrixMsg, PatchMatrix,
        layout::{Cell, CellState, ViewOptions},
    },
};
use aizome_kit::{column, prelude::*, row};
use std::{array, rc::Rc};

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

#[derive(Clone, Debug)]
pub enum Msg {
    Page(Page),
    Activated,
    Matrix(MatrixMsg),
    Backend(Event),
}

#[derive(Default)]
pub struct Musubi {
    pub page: Page,
    pub instance: Option<instance::Instance>,
    pub live: bool,
    backend: Option<PipeWireBackend>,
    pub graph: Graph,
    connection: Option<Connection>,
    matrix_spike: Option<Rc<Layout>>,
    pub options: [ViewOptions; 4],
    layouts: [Option<Rc<Layout>>; 4],
    hovered: Option<Cell>,
    window: WindowKey,
}

impl Musubi {
    /// Synthetic geometry exists only in Harness tests, never in the shipped app.
    pub fn for_matrix_test(layout: Rc<Layout>) -> Self {
        Self { matrix_spike: Some(layout), ..Self::default() }
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
            Some(Rc::new(Layout::from_graph(&self.graph, page, &self.options[i])))
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
            CellState::Error => "error — connection refused",
            CellState::NotAllowed => "not allowed — incompatible ports",
            CellState::Count(_) => "collapsed — click to expand",
            CellState::Empty => "not connected — click to connect",
        };
        Some(format!("{from_name} · {} → {to_name} · {} · {state}", describe(source), describe(target)))
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
    }

    fn update(&mut self, msg: Msg, cx: &mut Cx<Msg>) {
        match msg {
            Msg::Page(page) => {
                self.page = page;
                self.hovered = None;
            }
            Msg::Activated => cx.activate(self.window),
            Msg::Matrix(MatrixMsg::Hover(cell)) => self.hovered = cell,
            Msg::Matrix(_) => {}
            Msg::Backend(Event::Connection(connection)) => {
                if matches!(connection, Connection::Down(_)) {
                    self.graph = Graph::default();
                    self.refresh_layouts();
                }
                self.connection = Some(connection);
            }
            Msg::Backend(Event::Delta(delta)) => {
                self.graph.apply(delta);
                self.refresh_layouts();
            }
            Msg::Backend(Event::CommandResult { .. }) => {}
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
        let status = self.hover_sentence().unwrap_or_else(|| match &self.connection {
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
