// The private PipeWire fixture has null devices but no sound cards or profiles;
// these Harness cases simulate cards, and P2-S6's VM covers real profile switching.
use aizome_kit::{
    Appearance,
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_musubi::{
    app::{Msg, Musubi, Page},
    backend::{Backend, Command, Handle, fake::FakeBackend},
    graph::Upsert,
    matrix::layout::Cell,
    mixer::{
        audio::FakeAudio,
        model::{self, StripKind},
    },
};
use aizome_sys::audio::{AudioDevice, AudioState, Card, CardProfile, Port};
use std::{cell::RefCell, rc::Rc};

fn setup(day: bool) -> (Harness<Musubi>, Rc<FakeAudio>, Rc<RefCell<FakeBackend>>) {
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
    let profile = |name: &str, description: &str, available, sinks, sources| CardProfile {
        name: name.into(),
        description: description.into(),
        available,
        sinks,
        sources,
        ..CardProfile::default()
    };
    let card = |index, name: &str, description: &str, bus: &str, active: &str, profiles| Card {
        index,
        name: name.into(),
        description: description.into(),
        bus: bus.into(),
        active_profile: Some(active.into()),
        profiles,
        ..Card::default()
    };
    let device = |index, name: &str, description: &str, bus: &str| AudioDevice {
        index,
        name: name.into(),
        description: description.into(),
        bus: bus.into(),
        channel_volumes: vec![0.8, 0.8],
        ports: vec![
            Port {
                name: "analog".into(),
                description: if name.starts_with("alsa_input") { "Mic input" } else { "Speakers" }.into(),
                available: Some(true),
                ..Port::default()
            },
            Port {
                name: "headphones".into(),
                description: if name.starts_with("alsa_input") { "Headset mic" } else { "Headphones" }.into(),
                available: Some(true),
                ..Port::default()
            },
            Port {
                name: "hdmi".into(),
                description: "HDMI unplugged".into(),
                available: Some(false),
                ..Port::default()
            },
        ],
        active_port: Some("analog".into()),
        ..AudioDevice::default()
    };
    let audio = Rc::new(FakeAudio {
        state: RefCell::new(AudioState {
            available: true,
            cards: vec![
                card(
                    101,
                    "alsa_card.usb.studio",
                    "USB Studio",
                    "usb",
                    "output:analog-stereo+input:analog-stereo",
                    vec![
                        profile("output:analog-stereo+input:analog-stereo", "Analog Stereo Duplex", true, 1, 1),
                        profile("pro-audio", "Pro Audio", true, 1, 1),
                        profile("hdmi", "HDMI (unplugged)", false, 1, 0),
                    ],
                ),
                card(
                    102,
                    "alsa_card.pci.analog",
                    "Built-in Audio",
                    "pci",
                    "analog",
                    vec![
                        profile("analog", "Analog Stereo", true, 1, 1),
                        profile("surround", "Surround 5.1", true, 1, 0),
                        profile("hdmi", "HDMI", false, 1, 0),
                    ],
                ),
                card(
                    103,
                    "bluez_card.headphones",
                    "Bluetooth Headset",
                    "bluetooth",
                    "a2dp",
                    vec![
                        profile("a2dp", "High Fidelity Playback", true, 1, 0),
                        profile("hsp", "Headset Head Unit", true, 1, 1),
                    ],
                ),
            ],
            sinks: vec![
                device(10010, "alsa_output.usb_studio", "USB Studio Output", "usb"),
                device(10013, "alsa_output.analog", "Speakers", "pci"),
            ],
            sources: vec![device(10012, "alsa_input.usb_studio", "USB Studio Input", "usb")],
            ..AudioState::default()
        }),
        ..FakeAudio::default()
    });
    let appearance = Appearance {
        theme: if day { ThemeChoice::Day } else { ThemeChoice::Night },
        reduced_motion: true,
        ..Appearance::default()
    };
    let mut h = Harness::with(Musubi::with_fake_audio(fake.clone(), Box::new(audio.clone())), appearance, true);
    h.send(Msg::Page(Page::Devices));
    (h, audio, fake)
}

fn drain(h: &mut Harness<Musubi>, fake: &Rc<RefCell<FakeBackend>>) {
    loop {
        let Some(event) = fake.borrow_mut().next_event() else { break };
        h.send(Msg::Backend(event));
    }
}

#[test]
fn profile_and_route_are_read_back_and_undoable() {
    let (mut h, audio, _) = setup(false);
    h.send(Msg::DeviceProfile("alsa_card.usb.studio".into(), "hdmi".into()));
    assert!(audio.commands.borrow().is_empty());
    h.send(Msg::DeviceProfile("alsa_card.usb.studio".into(), "pro-audio".into()));
    assert_eq!(audio.commands.borrow().last().unwrap(), "profile 101 pro-audio");
    assert_eq!(audio.state.borrow().cards[0].active_profile.as_deref(), Some("pro-audio"));
    h.send(Msg::Undo);
    assert_eq!(
        audio.state.borrow().cards[0].active_profile.as_deref(),
        Some("output:analog-stereo+input:analog-stereo")
    );
    h.send(Msg::MixerRoute(StripKind::Output, 10010, "hdmi".into()));
    assert_eq!(audio.state.borrow().sinks[0].active_port.as_deref(), Some("analog"));
    h.send(Msg::MixerRoute(StripKind::Output, 10010, "headphones".into()));
    assert_eq!(audio.commands.borrow().last().unwrap(), "port Output 10010 headphones");
    assert_eq!(audio.state.borrow().sinks[0].active_port.as_deref(), Some("headphones"));
}

#[test]
fn replacing_profile_nodes_rebinds_mixer_and_inspector() {
    let (mut h, audio, fake) = setup(false);
    let original = fake.borrow().graph.nodes[&12].clone();
    let port = fake.borrow().graph.ports[&100].clone();
    h.send(Msg::Page(Page::Audio));
    let layout = h.program().layout(Page::Audio).unwrap();
    let cell = layout.row_lines.iter().position(|line| line.ports.contains(&100)).unwrap();
    h.send(Msg::Matrix(aizome_musubi::matrix::MatrixMsg::Select(Cell { row: cell, col: 0 }, false)));
    h.send(Msg::Inspector);
    assert!(
        fake.borrow()
            .commands
            .iter()
            .any(|cmd| matches!(cmd, Command::StartMeters(nodes) if nodes == &[Handle { epoch: 1, id: 12 }]))
    );
    h.send(Msg::DeviceProfile("alsa_card.usb.studio".into(), "pro-audio".into()));
    fake.borrow_mut().remove(100);
    fake.borrow_mut().remove(12);
    drain(&mut h, &fake);
    assert!(h.find_id(h.window(), "inspector-meter").is_none());
    assert!(matches!(fake.borrow().commands.last(), Some(Command::StopMeters)));
    let mut replacement = original;
    replacement.id = 210;
    replacement.props.insert("object.serial".into(), "10210".into());
    let mut new_port = port;
    new_port.id = 300;
    new_port.node_id = Some(210);
    fake.borrow_mut().upsert(Upsert::Node(replacement));
    fake.borrow_mut().upsert(Upsert::Port(new_port));
    audio.state.borrow_mut().sources[0].index = 10210;
    drain(&mut h, &fake);
    assert!(h.find_id(h.window(), "inspector-meter").is_some());
    h.send(Msg::Page(Page::Devices));
    h.send(Msg::DeviceShowMixer(210));
    assert_eq!(h.program().page, Page::Mixer);
    assert!(h.find_id(h.window(), "fader-Input-10210").is_some());
    h.send(Msg::Page(Page::Devices));
    h.send(Msg::DeviceShowAudio(210));
    assert_eq!(h.program().page, Page::Audio);
    assert!(h.find_id(h.window(), "inspector-meter").is_some());
    assert!(
        fake.borrow()
            .commands
            .iter()
            .any(|cmd| matches!(cmd, Command::StartMeters(nodes) if nodes == &[Handle { epoch: 1, id: 210 }]))
    );
    let projected = model::strips(&audio.state.borrow(), &h.program().graph);
    assert_eq!(
        projected.iter().find(|strip| strip.name == "USB Studio Input").unwrap().node,
        Some(Handle { epoch: 1, id: 210 })
    );
    h.send(Msg::Inspector);
    assert!(matches!(fake.borrow().commands.last(), Some(Command::StopMeters)));
}

#[test]
fn devices_and_inspector_goldens() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        let (mut h, _, fake) = setup(day);
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/devices-page-{theme}.png"), 1.);
        h.send(Msg::DeviceProfileMenu);
        assert_golden(&mut h, w, format!("tests/snapshots/devices-profiles-{theme}.png"), 1.);
        h.send(Msg::DeviceProfileMenu);
        h.send(Msg::MixerRouteMenu(StripKind::Output, 10010));
        assert_golden(&mut h, w, format!("tests/snapshots/devices-routes-{theme}.png"), 1.);
        h.send(Msg::Page(Page::Audio));
        let layout = h.program().layout(Page::Audio).unwrap();
        let row = layout.row_lines.iter().position(|line| line.ports.contains(&100)).unwrap();
        h.send(Msg::Matrix(aizome_musubi::matrix::MatrixMsg::Select(Cell { row, col: 0 }, false)));
        h.send(Msg::Inspector);
        fake.borrow().script_meter(Handle { epoch: 1, id: 12 }, &[0.8, 0.4]);
        h.send(Msg::MeterTick);
        assert_golden(&mut h, w, format!("tests/snapshots/inspector-meter-{theme}.png"), 1.);
    }
}
