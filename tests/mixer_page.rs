use aizome_kit::{
    Appearance, Point, Size,
    sidestep_ui::{Modifiers, NamedKey},
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_musubi::{
    app::{Msg, Musubi, Page},
    backend::{Command, fake::FakeBackend},
    mixer::{
        audio::FakeAudio,
        model::{self, StripControl, StripKind},
    },
};
use aizome_sys::audio::{AudioDevice, AudioState, AudioStream};
use std::{cell::RefCell, rc::Rc, time::Duration};

fn setup(day: bool) -> (Harness<Musubi>, Rc<FakeAudio>, Rc<RefCell<FakeBackend>>) {
    let mut dump: serde_json::Value = serde_json::from_str(include_str!("fixtures/studio.json")).unwrap();
    dump.as_array_mut().unwrap().retain(|v| [10, 13, 12, 14, 17, 18, 26, 24].contains(&v["id"].as_u64().unwrap_or(0)));
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(&dump.to_string(), 1).unwrap()));
    let device = |index, name: &str, description: &str, default| AudioDevice {
        index,
        name: name.into(),
        description: description.into(),
        volume: 0.8,
        channel_volumes: vec![0.8, 0.8],
        is_default: default,
        ..AudioDevice::default()
    };
    let stream = |index, app: &str, title: &str, target| AudioStream {
        index,
        application: app.into(),
        media_name: title.into(),
        device: target,
        volume: 0.7,
        volume_writable: true,
        ..AudioStream::default()
    };
    let audio = Rc::new(FakeAudio {
        state: RefCell::new(AudioState {
            available: true,
            default_sink: Some("alsa_output.usb_studio".into()),
            default_source: Some("alsa_input.usb_studio".into()),
            sinks: vec![
                device(10010, "alsa_output.usb_studio", "Studio Output", true),
                device(10013, "alsa_output.analog", "Speakers", false),
            ],
            sources: vec![
                device(10012, "alsa_input.usb_studio", "Studio Input", true),
                device(10014, "alsa_input.analog", "Microphone", false),
            ],
            playback: vec![
                stream(10017, "Firefox", "Video", 10010),
                stream(10026, "Firefox", "Music", 10010),
                stream(10018, "Spotify", "Playlist", 10013),
            ],
            recording: vec![stream(10024, "Discord", "Call", 10012)],
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
    h.send(Msg::Page(Page::Mixer));
    (h, audio, fake)
}

#[test]
fn readback_commands_and_meter_lifecycle() {
    let (mut h, audio, backend) = setup(false);
    let w = h.window();
    let node = aizome_musubi::backend::Handle { epoch: 1, id: 10 };
    assert!(
        backend.borrow().commands.iter().any(|c| matches!(c, Command::StartMeters(nodes) if nodes.contains(&node)))
    );
    backend.borrow().script_meter(node, &[0.7, 0.3]);
    h.advance(Duration::from_millis(35));
    let r = h.find_id(w, "fader-Output-10010").unwrap();
    h.click(w, Point::new(r.x0 + 42., r.y0 + 100.));
    h.named(w, NamedKey::ArrowUp, Modifiers::NONE);
    assert!(audio.commands.borrow().iter().any(|s| s.starts_with("volume Output 10010 ")));
    assert_ne!(audio.state.borrow().sinks[0].volume, 0.8);
    let projected = model::strips(&audio.state.borrow(), &h.program().graph);
    assert!(
        matches!(projected.iter().find(|s| s.kind == StripKind::Output && s.name == "Studio Output").unwrap().control, StripControl::Pulse { volume, .. } if volume == audio.state.borrow().sinks[0].volume)
    );
    h.send(Msg::MixerMute(
        StripKind::Output,
        projected.iter().find(|s| s.name == "Studio Output").unwrap().control.clone(),
        true,
    ));
    assert!(audio.state.borrow().sinks[0].muted);
    h.send(Msg::MixerBalance(StripKind::Output, 10010, -0.4));
    assert_eq!(audio.state.borrow().sinks[0].balance, -0.4);
    h.send(Msg::MixerMove(StripKind::Playback, 10017, 10013));
    assert_eq!(audio.state.borrow().playback[0].device, 10013);
    h.send(Msg::MixerDefault(StripKind::Output, "alsa_output.analog".into()));
    assert!(audio.state.borrow().sinks[1].is_default);
    h.send(Msg::Page(Page::Audio));
    assert!(matches!(backend.borrow().commands.last(), Some(Command::StopMeters)));
}

#[test]
fn ambiguous_identity_disables_strip() {
    let (h, audio, _) = setup(false);
    audio.state.borrow_mut().sinks[0].name = "missing-name".into();
    let strips = model::strips(&audio.state.borrow(), &h.program().graph);
    assert!(
        matches!(&strips.iter().find(|s| s.name == "Studio Output").unwrap().control, StripControl::ReadOnly(reason) if reason.contains("does not match"))
    );
}

#[test]
fn populated_mixer_goldens() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        let (mut h, _, backend) = setup(day);
        backend.borrow().script_meter(aizome_musubi::backend::Handle { epoch: 1, id: 10 }, &[0.8, 0.4]);
        h.send(Msg::MeterTick);
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/mixer-page-{theme}.png"), 1.);
        h.resize(w, Size::new(900., 560.));
        assert_golden(&mut h, w, format!("tests/snapshots/mixer-page-narrow-{theme}.png"), 1.);
    }
}
