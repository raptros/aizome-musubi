use crate::{backend::{Connection, Event, pipewire::PipeWireBackend}, graph::{Graph, classify::PortKind}, instance, matrix::{Layout, MatrixMsg, PatchMatrix}};
use std::rc::Rc;
use aizome_kit::{column, row, prelude::*};

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
    window: WindowKey,
}

impl Musubi {
    /// Synthetic geometry exists only in Harness tests, never in the shipped app.
    pub fn for_matrix_test(layout: Rc<Layout>) -> Self {
        Self { matrix_spike: Some(layout), ..Self::default() }
    }
    fn kind(&self) -> PortKind {
        match self.page {
            Page::Audio => PortKind::Audio, Page::Midi => PortKind::Midi,
            Page::Midi2 => PortKind::Ump, Page::Video => PortKind::Video,
        }
    }
    fn counts(&self) -> (usize, usize) {
        let kind = self.kind();
        let ports = self.graph.ports_of_kind(kind).count();
        let links = self.graph.links.values().filter(|link|
            self.graph.kind(link.output) == Some(kind) && self.graph.kind(link.input) == Some(kind)).count();
        (ports, links)
    }
    fn unavailable(&self) -> Element<Msg> {
        container(
            column![
                text(self.page.title()).size(28.).semibold(),
                text(match &self.connection {
                    Some(Connection::Down(_)) => "PipeWire unavailable.",
                    _ => "Not connected to PipeWire yet.",
                }).size(20.).role(Role::Text2),
                text("This page cannot display ports or change connections in this build.")
                    .role(Role::Text2)
                    .wrap(),
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
        self.window = cx.open_window(
            WindowSpec::new("Musubi — Patchbay")
                .size(1280., 800.)
                .min_size(900., 560.),
        );
        if let Some(instance) = &self.instance {
            let sender = cx.stream();
            if let Ok(listener) = instance.listener.try_clone() {
                instance::serve(listener, move || sender.send(Msg::Activated));
            }
        }
        if self.live {
            let sender = cx.stream();
            match PipeWireBackend::start(move |event| { sender.send(Msg::Backend(event)); }) {
                Ok(backend) => self.backend = Some(backend),
                Err(reason) => self.connection = Some(Connection::Down(reason)),
            }
        }
    }

    fn update(&mut self, msg: Msg, cx: &mut Cx<Msg>) {
        match msg {
            Msg::Page(page) => self.page = page,
            Msg::Activated => cx.activate(self.window),
            Msg::Matrix(_) => {},
            Msg::Backend(Event::Connection(connection)) => {
                if matches!(connection, Connection::Down(_)) {
                    self.graph = Graph::default();
                }
                self.connection = Some(connection);
            }
            Msg::Backend(Event::Delta(delta)) => self.graph.apply(delta),
            Msg::Backend(Event::CommandResult { .. }) => {}
        }
    }

    fn view(&self, _window: WindowKey) -> Element<Msg> {
        let mut side = column![
            container(
                row![
                    container(aizome_kit::icon(Icon::Music).size(24.))
                        .padding(8.)
                        .fill(Tone::Sunk),
                    column![
                        text("Musubi").size(20.).semibold(),
                        text("結び · Patchbay").size(12.).role(Role::Text2),
                    ]
                    .spacing(4.),
                ]
                .spacing(10.),
            )
            .padding(14.)
            .width(Fill)
            .fill(Tone::Chrome),
            caption("PATCHBAY"),
        ]
        .spacing(8.)
        .padding([14., 8., 0., 8.]);
        for (page, icon, title) in Page::ALL {
            side = side.push(
                sidebar_item(icon, title, self.page == page)
                    .on_press(Msg::Page(page))
                    .id(title),
            );
        }
        let content: Element<Msg> = if self.page == Page::Audio && self.matrix_spike.is_some() {
            PatchMatrix::new(self.matrix_spike.as_ref().unwrap().clone(), Msg::Matrix).id("matrix").width(Fill).height(Fill)
        } else if matches!(self.connection, Some(Connection::Up(_))) {
            let (ports, links) = self.counts();
            container(column![
                text(self.page.title()).size(28.).semibold(),
                text(format!("{ports} ports · {links} links")).size(20.),
            ].spacing(12.)).padding(36.).width(Fill).height(Fill).into()
        } else {
            self.unavailable()
        };
        let main = column![
            toolbar(vec![text(self.page.title()).semibold().into()]),
            content,
            status_bar(vec![status_text(match &self.connection {
                Some(Connection::Up(_)) => "PipeWire: connected",
                _ => "PipeWire: not connected",
            })]),
        ]
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
