//! Audio control seam: views consume snapshots, never pending widget values.
use aizome_sys::Task;
use aizome_sys::audio::{Audio, AudioState};

pub enum AudioTask {
    Unit(Task<()>),
    Bool(Task<bool>),
}

impl AudioTask {
    pub fn take(&self) -> Option<Result<(), String>> {
        match self {
            Self::Unit(task) => task.take().map(|result| result.map_err(|error| error.to_string())),
            Self::Bool(task) => task.take().map(|result| result.map(|_| ()).map_err(|error| error.to_string())),
        }
    }
    pub fn ok() -> Self {
        Self::Unit(Task::ready(Ok(())))
    }
}

use super::model::StripKind;

pub trait AudioControl {
    fn snapshot(&self) -> AudioState;
    fn volume(&self, kind: StripKind, index: u32, value: f32) -> AudioTask;
    fn mute(&self, kind: StripKind, index: u32, value: bool) -> AudioTask;
    fn balance(&self, kind: StripKind, index: u32, value: f32) -> AudioTask;
    fn default_device(&self, kind: StripKind, name: &str) -> AudioTask;
    fn move_stream(&self, kind: StripKind, index: u32, target: u32) -> AudioTask;
    fn port(&self, kind: StripKind, index: u32, port: &str) -> AudioTask;
    fn profile(&self, card: u32, profile: &str) -> AudioTask;
}

impl AudioControl for Audio {
    fn snapshot(&self) -> AudioState {
        (*Audio::snapshot(self)).clone()
    }
    fn volume(&self, kind: StripKind, index: u32, value: f32) -> AudioTask {
        AudioTask::Unit(match kind {
            StripKind::Output | StripKind::Effect => self.set_sink_volume(index, value),
            StripKind::Input => self.set_source_volume(index, value),
            StripKind::Playback => self.set_playback_volume(index, value),
            StripKind::Recording => self.set_recording_volume(index, value),
            StripKind::Native => return AudioTask::ok(),
        })
    }
    fn mute(&self, kind: StripKind, index: u32, value: bool) -> AudioTask {
        AudioTask::Bool(match kind {
            StripKind::Output | StripKind::Effect => self.set_sink_mute(index, value),
            StripKind::Input => self.set_source_mute(index, value),
            StripKind::Playback => self.set_playback_mute(index, value),
            StripKind::Recording => self.set_recording_mute(index, value),
            StripKind::Native => return AudioTask::ok(),
        })
    }
    fn balance(&self, kind: StripKind, index: u32, value: f32) -> AudioTask {
        AudioTask::Unit(match kind {
            StripKind::Output | StripKind::Effect => self.set_sink_balance(index, value),
            StripKind::Input => self.set_source_balance(index, value),
            _ => return AudioTask::ok(),
        })
    }
    fn default_device(&self, kind: StripKind, name: &str) -> AudioTask {
        AudioTask::Unit(match kind {
            StripKind::Output | StripKind::Effect => self.set_default_sink(name),
            StripKind::Input => self.set_default_source(name),
            _ => return AudioTask::ok(),
        })
    }
    fn port(&self, kind: StripKind, index: u32, port: &str) -> AudioTask {
        AudioTask::Unit(match kind {
            StripKind::Output | StripKind::Effect => self.set_sink_port(index, port),
            StripKind::Input => self.set_source_port(index, port),
            _ => return AudioTask::ok(),
        })
    }
    fn move_stream(&self, kind: StripKind, index: u32, target: u32) -> AudioTask {
        AudioTask::Unit(match kind {
            StripKind::Playback => self.move_playback(index, target),
            StripKind::Recording => self.move_recording(index, target),
            _ => return AudioTask::ok(),
        })
    }
    fn profile(&self, card: u32, profile: &str) -> AudioTask {
        AudioTask::Unit(self.set_card_profile(card, profile))
    }
}

#[derive(Default)]
pub struct FakeAudio {
    pub state: std::cell::RefCell<AudioState>,
    pub commands: std::cell::RefCell<Vec<String>>,
    pub fail_next: std::cell::RefCell<Option<String>>,
}

impl AudioControl for FakeAudio {
    fn snapshot(&self) -> AudioState {
        self.state.borrow().clone()
    }
    fn volume(&self, kind: StripKind, index: u32, value: f32) -> AudioTask {
        self.commands.borrow_mut().push(format!("volume {kind:?} {index} {value}"));
        if let Some(reason) = self.fail_next.borrow_mut().take() {
            return AudioTask::Unit(Task::ready(Err(aizome_sys::Error::Failed(reason))));
        }
        let mut state = self.state.borrow_mut();
        match kind {
            StripKind::Output | StripKind::Effect => {
                state.sinks.iter_mut().find(|d| d.index == index).map(|d| d.volume = value)
            }
            StripKind::Input => state.sources.iter_mut().find(|d| d.index == index).map(|d| d.volume = value),
            StripKind::Playback => state.playback.iter_mut().find(|s| s.index == index).map(|s| s.volume = value),
            StripKind::Recording => state.recording.iter_mut().find(|s| s.index == index).map(|s| s.volume = value),
            StripKind::Native => None,
        };
        AudioTask::ok()
    }
    fn mute(&self, kind: StripKind, index: u32, value: bool) -> AudioTask {
        self.commands.borrow_mut().push(format!("mute {kind:?} {index} {value}"));
        let mut state = self.state.borrow_mut();
        match kind {
            StripKind::Output | StripKind::Effect => {
                state.sinks.iter_mut().find(|d| d.index == index).map(|d| d.muted = value)
            }
            StripKind::Input => state.sources.iter_mut().find(|d| d.index == index).map(|d| d.muted = value),
            StripKind::Playback => state.playback.iter_mut().find(|s| s.index == index).map(|s| s.muted = value),
            StripKind::Recording => state.recording.iter_mut().find(|s| s.index == index).map(|s| s.muted = value),
            StripKind::Native => None,
        };
        AudioTask::ok()
    }
    fn balance(&self, kind: StripKind, index: u32, value: f32) -> AudioTask {
        self.commands.borrow_mut().push(format!("balance {kind:?} {index} {value}"));
        let mut state = self.state.borrow_mut();
        let devices =
            if matches!(kind, StripKind::Output | StripKind::Effect) { &mut state.sinks } else { &mut state.sources };
        if let Some(device) = devices.iter_mut().find(|d| d.index == index) {
            device.balance = value;
        }
        AudioTask::ok()
    }
    fn default_device(&self, kind: StripKind, name: &str) -> AudioTask {
        self.commands.borrow_mut().push(format!("default {kind:?} {name}"));
        let mut state = self.state.borrow_mut();
        let devices = if matches!(kind, StripKind::Output | StripKind::Effect) {
            state.default_sink = Some(name.into());
            &mut state.sinks
        } else {
            state.default_source = Some(name.into());
            &mut state.sources
        };
        for device in devices {
            device.is_default = device.name == name;
        }
        AudioTask::ok()
    }
    fn port(&self, kind: StripKind, index: u32, port: &str) -> AudioTask {
        self.commands.borrow_mut().push(format!("port {kind:?} {index} {port}"));
        let mut state = self.state.borrow_mut();
        let devices =
            if matches!(kind, StripKind::Output | StripKind::Effect) { &mut state.sinks } else { &mut state.sources };
        if let Some(device) = devices.iter_mut().find(|device| device.index == index)
            && device.ports.iter().any(|candidate| candidate.name == port && candidate.available != Some(false))
        {
            device.active_port = Some(port.into());
        }
        AudioTask::ok()
    }
    fn move_stream(&self, kind: StripKind, index: u32, target: u32) -> AudioTask {
        self.commands.borrow_mut().push(format!("move {kind:?} {index} {target}"));
        let mut state = self.state.borrow_mut();
        let streams = if kind == StripKind::Playback { &mut state.playback } else { &mut state.recording };
        if let Some(stream) = streams.iter_mut().find(|s| s.index == index) {
            stream.device = target;
        }
        AudioTask::ok()
    }
    fn profile(&self, card: u32, profile: &str) -> AudioTask {
        self.commands.borrow_mut().push(format!("profile {card} {profile}"));
        if let Some(entry) = self.state.borrow_mut().cards.iter_mut().find(|entry| entry.index == card) {
            entry.active_profile = Some(profile.into());
        }
        AudioTask::ok()
    }
}

impl AudioControl for std::rc::Rc<FakeAudio> {
    fn snapshot(&self) -> AudioState {
        self.as_ref().snapshot()
    }
    fn volume(&self, kind: StripKind, index: u32, value: f32) -> AudioTask {
        self.as_ref().volume(kind, index, value)
    }
    fn mute(&self, kind: StripKind, index: u32, value: bool) -> AudioTask {
        self.as_ref().mute(kind, index, value)
    }
    fn balance(&self, kind: StripKind, index: u32, value: f32) -> AudioTask {
        self.as_ref().balance(kind, index, value)
    }
    fn default_device(&self, kind: StripKind, name: &str) -> AudioTask {
        self.as_ref().default_device(kind, name)
    }
    fn port(&self, kind: StripKind, index: u32, port: &str) -> AudioTask {
        self.as_ref().port(kind, index, port)
    }
    fn move_stream(&self, kind: StripKind, index: u32, target: u32) -> AudioTask {
        self.as_ref().move_stream(kind, index, target)
    }
    fn profile(&self, card: u32, profile: &str) -> AudioTask {
        self.as_ref().profile(card, profile)
    }
}
