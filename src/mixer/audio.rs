//! Audio control seam: views consume snapshots, never pending widget values.
use aizome_sys::audio::{Audio, AudioState};

use super::model::StripKind;

pub trait AudioControl {
    fn snapshot(&self) -> AudioState;
    fn volume(&self, kind: StripKind, index: u32, value: f32);
    fn mute(&self, kind: StripKind, index: u32, value: bool);
    fn balance(&self, kind: StripKind, index: u32, value: f32);
    fn default_device(&self, kind: StripKind, name: &str);
    fn move_stream(&self, kind: StripKind, index: u32, target: u32);
}

impl AudioControl for Audio {
    fn snapshot(&self) -> AudioState {
        (*Audio::snapshot(self)).clone()
    }
    fn volume(&self, kind: StripKind, index: u32, value: f32) {
        match kind {
            StripKind::Output => {
                self.set_sink_volume(index, value);
            }
            StripKind::Input => {
                self.set_source_volume(index, value);
            }
            StripKind::Playback => {
                self.set_playback_volume(index, value);
            }
            StripKind::Recording => {
                self.set_recording_volume(index, value);
            }
            StripKind::Native => {}
        }
    }
    fn mute(&self, kind: StripKind, index: u32, value: bool) {
        match kind {
            StripKind::Output => {
                self.set_sink_mute(index, value);
            }
            StripKind::Input => {
                self.set_source_mute(index, value);
            }
            StripKind::Playback => {
                self.set_playback_mute(index, value);
            }
            StripKind::Recording => {
                self.set_recording_mute(index, value);
            }
            StripKind::Native => {}
        }
    }
    fn balance(&self, kind: StripKind, index: u32, value: f32) {
        match kind {
            StripKind::Output => {
                self.set_sink_balance(index, value);
            }
            StripKind::Input => {
                self.set_source_balance(index, value);
            }
            _ => {}
        }
    }
    fn default_device(&self, kind: StripKind, name: &str) {
        match kind {
            StripKind::Output => {
                self.set_default_sink(name);
            }
            StripKind::Input => {
                self.set_default_source(name);
            }
            _ => {}
        }
    }
    fn move_stream(&self, kind: StripKind, index: u32, target: u32) {
        match kind {
            StripKind::Playback => {
                self.move_playback(index, target);
            }
            StripKind::Recording => {
                self.move_recording(index, target);
            }
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct FakeAudio {
    pub state: std::cell::RefCell<AudioState>,
    pub commands: std::cell::RefCell<Vec<String>>,
}
impl AudioControl for FakeAudio {
    fn snapshot(&self) -> AudioState {
        self.state.borrow().clone()
    }
    fn volume(&self, kind: StripKind, index: u32, value: f32) {
        self.commands.borrow_mut().push(format!("volume {kind:?} {index} {value}"));
        let mut state = self.state.borrow_mut();
        match kind {
            StripKind::Output => state.sinks.iter_mut().find(|d| d.index == index).map(|d| d.volume = value),
            StripKind::Input => state.sources.iter_mut().find(|d| d.index == index).map(|d| d.volume = value),
            StripKind::Playback => state.playback.iter_mut().find(|s| s.index == index).map(|s| s.volume = value),
            StripKind::Recording => state.recording.iter_mut().find(|s| s.index == index).map(|s| s.volume = value),
            StripKind::Native => None,
        };
    }
    fn mute(&self, kind: StripKind, index: u32, value: bool) {
        self.commands.borrow_mut().push(format!("mute {kind:?} {index} {value}"));
        let mut state = self.state.borrow_mut();
        match kind {
            StripKind::Output => state.sinks.iter_mut().find(|d| d.index == index).map(|d| d.muted = value),
            StripKind::Input => state.sources.iter_mut().find(|d| d.index == index).map(|d| d.muted = value),
            StripKind::Playback => state.playback.iter_mut().find(|s| s.index == index).map(|s| s.muted = value),
            StripKind::Recording => state.recording.iter_mut().find(|s| s.index == index).map(|s| s.muted = value),
            StripKind::Native => None,
        };
    }
    fn balance(&self, kind: StripKind, index: u32, value: f32) {
        self.commands.borrow_mut().push(format!("balance {kind:?} {index} {value}"));
        let mut state = self.state.borrow_mut();
        let devices = if kind == StripKind::Output { &mut state.sinks } else { &mut state.sources };
        if let Some(device) = devices.iter_mut().find(|d| d.index == index) {
            device.balance = value;
        }
    }
    fn default_device(&self, kind: StripKind, name: &str) {
        self.commands.borrow_mut().push(format!("default {kind:?} {name}"));
        let mut state = self.state.borrow_mut();
        let devices = if kind == StripKind::Output {
            state.default_sink = Some(name.into());
            &mut state.sinks
        } else {
            state.default_source = Some(name.into());
            &mut state.sources
        };
        for device in devices {
            device.is_default = device.name == name;
        }
    }
    fn move_stream(&self, kind: StripKind, index: u32, target: u32) {
        self.commands.borrow_mut().push(format!("move {kind:?} {index} {target}"));
        let mut state = self.state.borrow_mut();
        let streams = if kind == StripKind::Playback { &mut state.playback } else { &mut state.recording };
        if let Some(stream) = streams.iter_mut().find(|s| s.index == index) {
            stream.device = target;
        }
    }
}

impl AudioControl for std::rc::Rc<FakeAudio> {
    fn snapshot(&self) -> AudioState {
        self.as_ref().snapshot()
    }
    fn volume(&self, kind: StripKind, index: u32, value: f32) {
        self.as_ref().volume(kind, index, value);
    }
    fn mute(&self, kind: StripKind, index: u32, value: bool) {
        self.as_ref().mute(kind, index, value);
    }
    fn balance(&self, kind: StripKind, index: u32, value: f32) {
        self.as_ref().balance(kind, index, value);
    }
    fn default_device(&self, kind: StripKind, name: &str) {
        self.as_ref().default_device(kind, name);
    }
    fn move_stream(&self, kind: StripKind, index: u32, target: u32) {
        self.as_ref().move_stream(kind, index, target);
    }
}
