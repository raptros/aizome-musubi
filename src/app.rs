use crate::instance;
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
}

#[derive(Default)]
pub struct Musubi {
    pub page: Page,
    pub instance: Option<instance::Instance>,
    window: WindowKey,
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
    }

    fn update(&mut self, msg: Msg, cx: &mut Cx<Msg>) {
        match msg {
            Msg::Page(page) => self.page = page,
            Msg::Activated => cx.activate(self.window),
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
        let main = column![
            toolbar(vec![text(self.page.title()).semibold().into()]),
            container(
                column![
                    text(self.page.title()).size(28.).semibold(),
                    text("Not connected to PipeWire yet.").size(20.).role(Role::Text2),
                    text("This page cannot display ports or change connections in this build.")
                        .role(Role::Text2)
                        .wrap(),
                ]
                .spacing(12.),
            )
            .padding(36.)
            .width(Fill)
            .height(Fill),
            status_bar(vec![status_text("PipeWire: not connected")]),
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
