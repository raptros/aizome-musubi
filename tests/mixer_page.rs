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
use aizome_sys::audio::{AudioDevice, AudioState, AudioStream, Port};
use std::{cell::RefCell, rc::Rc, time::Duration};

fn setup(day: bool) -> (Harness<Musubi>, Rc<FakeAudio>, Rc<RefCell<FakeBackend>>) {
    let mut dump: serde_json::Value = serde_json::from_str(include_str!("fixtures/studio.json")).unwrap();
    dump.as_array_mut().unwrap().retain(|v| [10, 13, 12, 14, 17, 18, 26, 24].contains(&v["id"].as_u64().unwrap_or(0)));
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(&dump.to_string(), 1).unwrap()));
    let device = |index, name: &str, description: &str, default, input: bool| AudioDevice {
        index,
        name: name.into(),
        description: description.into(),
        volume: 0.8,
        channel_volumes: vec![0.8, 0.8],
        is_default: default,
        ports: vec![
            Port {
                name: "analog".into(),
                description: if input { "Mic input" } else { "Speakers" }.into(),
                available: Some(true),
                ..Port::default()
            },
            Port {
                name: "headphones".into(),
                description: if input { "Headset mic" } else { "Headphones" }.into(),
                available: Some(true),
                ..Port::default()
            },
            Port { name: "hdmi".into(), description: "HDMI".into(), available: Some(false), ..Port::default() },
        ],
        active_port: Some("analog".into()),
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
                device(10010, "alsa_output.usb_studio", "Studio Output", true, false),
                device(10013, "alsa_output.analog", "Speakers", false, false),
            ],
            sources: vec![
                device(10012, "alsa_input.usb_studio", "Studio Input", true, true),
                device(10014, "alsa_input.analog", "Microphone", false, true),
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
fn device_routes_use_port_setter_and_observed_active_port() {
    let (mut h, audio, _) = setup(false);
    let w = h.window();
    assert!(h.find_id(w, "route-Output-10013").is_some());
    h.send(Msg::MixerRoute(StripKind::Output, 10013, "hdmi".into()));
    assert_eq!(audio.state.borrow().sinks[1].active_port.as_deref(), Some("analog"));
    assert!(!audio.commands.borrow().iter().any(|command| command.starts_with("port ")));
    h.send(Msg::MixerRoute(StripKind::Output, 10013, "headphones".into()));
    assert_eq!(audio.state.borrow().sinks[1].active_port.as_deref(), Some("headphones"));
    assert!(audio.commands.borrow().iter().any(|command| command == "port Output 10013 headphones"));
    h.send(Msg::Undo);
    assert_eq!(audio.state.borrow().sinks[1].active_port.as_deref(), Some("analog"));
    let commands = audio.commands.borrow().len();
    h.send(Msg::MixerDefault(StripKind::Output, "alsa_output.usb_studio".into()));
    assert_eq!(audio.commands.borrow().len(), commands);
    assert!(
        h.semantics(w).iter().any(|(s, _)| { s.label.as_deref() == Some("Default device") && s.checked == Some(true) })
    );
}

#[test]
fn device_route_popover_golden() {
    let (mut h, _, _) = setup(false);
    let w = h.window();
    h.send(Msg::MixerRouteMenu(StripKind::Output, 10013));
    assert_golden(&mut h, w, "tests/snapshots/mixer-device-routes-night.png", 1.);
}

#[test]
fn muted_and_linked_mixer_goldens() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        let (mut h, audio, _) = setup(day);
        audio.state.borrow_mut().sources[1].muted = true;
        h.send(Msg::MixerLink(StripKind::Output, 10013));
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/mixer-muted-linked-{theme}.png"), 1.);
    }
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

#[test]
fn long_target_names_remain_distinguishable_goldens() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        let (mut h, audio, _) = setup(day);
        {
            let mut state = audio.state.borrow_mut();
            state.sinks[0].description = "musubi-vm-default".into();
            state.sinks[1].description = "musubi-vm-patch".into();
            state.sources[0].description = "Built-in Audio Stereo Input".into();
            state.sources[1].description = "Built-in Audio Stereo Microphone".into();
            state.playback[0].device = state.sinks[1].index;
        }
        h.send(Msg::AudioWake);
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/mixer-long-targets-{theme}.png"), 1.);
    }
}

#[test]
fn mixer_toolbar_filter_show_meters_and_percent_goldens() {
    let (mut h, _audio, backend) = setup(false);
    let w = h.window();
    h.send(Msg::MixerFilter("Firefox".into()));
    h.send(Msg::MixerShow);
    assert_golden(&mut h, w, "tests/snapshots/mixer-toolbar-show-night.png", 1.);
    h.send(Msg::MixerShow);
    h.send(Msg::MixerFilter(String::new()));
    h.send(Msg::MixerMeters(false));
    assert!(matches!(backend.borrow().commands.last(), Some(Command::StopMeters)));
    assert_golden(&mut h, w, "tests/snapshots/mixer-meters-off-night.png", 1.);
    h.send(Msg::MixerPercent(true));
    assert!(
        h.semantics(w)
            .iter()
            .any(|(s, _)| s.label.as_deref() == Some("Volume") && s.value.as_deref().is_some_and(|v| v.contains('%')))
    );
    assert_golden(&mut h, w, "tests/snapshots/mixer-percent-night.png", 1.);
}

#[test]
fn mixer_undo_volume_mute_balance_default_and_move() {
    let (mut h, audio, _) = setup(false);
    let control = model::strips(&audio.state.borrow(), &h.program().graph)
        .into_iter()
        .find(|s| s.name == "Studio Output")
        .unwrap()
        .control;
    h.send(Msg::MixerVolume(StripKind::Output, control, 0.5));
    h.advance(Duration::from_millis(510));
    let control = model::strips(&audio.state.borrow(), &h.program().graph)
        .into_iter()
        .find(|s| s.name == "Studio Output")
        .unwrap()
        .control;
    h.send(Msg::MixerMute(StripKind::Output, control, true));
    h.send(Msg::MixerBalance(StripKind::Output, 10010, 0.4));
    h.advance(Duration::from_millis(510));
    h.send(Msg::MixerMove(StripKind::Playback, 10017, 10013));
    h.send(Msg::MixerDefault(StripKind::Output, "alsa_output.analog".into()));
    for _ in 0..5 {
        h.send(Msg::Undo);
    }
    let state = audio.state.borrow();
    assert_eq!(state.sinks[0].volume, 0.8);
    assert!(!state.sinks[0].muted);
    assert_eq!(state.sinks[0].balance, 0.);
    assert!(state.sinks[0].is_default);
    assert_eq!(state.playback[0].device, 10010);
}

#[test]
fn audio_wake_starts_meter_for_newly_visible_pulse_strip() {
    let (mut h, audio, backend) = setup(false);
    let playback = audio.state.borrow_mut().playback.remove(0);
    h.send(Msg::MixerFilter("Video".into()));
    backend.borrow_mut().commands.clear();
    audio.state.borrow_mut().playback.push(playback);
    h.send(Msg::AudioWake);
    assert!(backend.borrow().commands.iter().any(|command| matches!(
        command,
        Command::StartMeters(nodes) if nodes.contains(&aizome_musubi::backend::Handle { epoch: 1, id: 17 })
    )));
}

#[test]
fn failed_audio_done_never_commits_history_and_failed_replay_can_retry() {
    let (mut h, audio, _) = setup(false);
    let control = model::strips(&audio.state.borrow(), &h.program().graph)
        .into_iter()
        .find(|s| s.name == "Studio Output")
        .unwrap()
        .control;
    *audio.fail_next.borrow_mut() = Some("refused".into());
    h.send(Msg::MixerVolume(StripKind::Output, control, 0.5));
    assert!(h.program().status_sentence().unwrap().contains("refused"));
    h.advance(Duration::from_millis(510));
    let commands = audio.commands.borrow().len();
    h.send(Msg::Undo);
    assert_eq!(audio.commands.borrow().len(), commands);

    let control = model::strips(&audio.state.borrow(), &h.program().graph)
        .into_iter()
        .find(|s| s.name == "Studio Output")
        .unwrap()
        .control;
    h.send(Msg::MixerVolume(StripKind::Output, control, 0.5));
    h.advance(Duration::from_millis(510));
    *audio.fail_next.borrow_mut() = Some("undo refused".into());
    h.send(Msg::Undo);
    assert!(h.program().status_sentence().unwrap().contains("undo refused"));
    h.send(Msg::Undo);
    assert_eq!(audio.state.borrow().sinks[0].volume, 0.8);
}
