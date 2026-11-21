//! Value-only meter snapshots shared with the UI; the process callback touches atomics only.
use super::Handle;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub const MAX_METERS: usize = 24;
pub const MAX_CHANNELS: usize = 64;
const DECAY_PER_SECOND: f32 = 0.8;

#[derive(Clone, Debug)]
pub struct MeterReading {
    pub node: Handle,
    pub peaks: Vec<f32>, // linear amplitude, one entry per channel
    pub last_update: Option<Instant>,
    pub paused: bool,
}

pub struct Slot {
    channels: usize,
    peaks: [AtomicU32; MAX_CHANNELS],
    updated_ns: AtomicU64,
}
impl Slot {
    pub fn new(channels: usize) -> Self {
        Self {
            channels: channels.clamp(1, MAX_CHANNELS),
            peaks: std::array::from_fn(|_| AtomicU32::new(0)),
            updated_ns: AtomicU64::new(0),
        }
    }
    /// F32 interleaved; `stride` is bytes per frame. Never allocates, locks, or wakes the UI.
    pub fn process(&self, bytes: &[u8], stride: usize, now_ns: u64) {
        let frame_bytes = self.channels * 4;
        if stride < frame_bytes {
            return;
        }
        for channel in 0..self.channels {
            let mut peak = 0.0_f32;
            for frame in bytes.chunks_exact(stride) {
                let start = channel * 4;
                let value = f32::from_ne_bytes([frame[start], frame[start + 1], frame[start + 2], frame[start + 3]]);
                if value.is_finite() {
                    peak = peak.max(value.abs());
                }
            }
            self.peaks[channel].store(peak.to_bits(), Ordering::Relaxed);
        }
        self.updated_ns.store(now_ns.max(1), Ordering::Release);
    }
    fn read(&self, origin: Instant, now: Instant) -> (Vec<f32>, Option<Instant>) {
        let stamp = self.updated_ns.load(Ordering::Acquire);
        let last_update = (stamp != 0).then(|| origin + Duration::from_nanos(stamp));
        let decay = last_update
            .map_or(0.0, |time| (1.0 - DECAY_PER_SECOND * now.saturating_duration_since(time).as_secs_f32()).max(0.0));
        (
            self.peaks[..self.channels].iter().map(|v| f32::from_bits(v.load(Ordering::Relaxed)) * decay).collect(),
            last_update,
        )
    }
}

#[derive(Default)]
struct State {
    active: HashMap<Handle, Arc<Slot>>,
    paused: HashSet<Handle>,
}
#[derive(Clone)]
pub struct MeterStore {
    origin: Instant,
    state: Arc<Mutex<State>>,
}
impl Default for MeterStore {
    fn default() -> Self {
        Self { origin: Instant::now(), state: Arc::new(Mutex::new(State::default())) }
    }
}
impl MeterStore {
    pub fn elapsed_ns(&self) -> u64 {
        self.origin.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
    }
    pub fn replace(&self, active: HashMap<Handle, Arc<Slot>>, paused: HashSet<Handle>) {
        *self.state.lock().unwrap() = State { active, paused };
    }
    pub fn clear(&self) {
        *self.state.lock().unwrap() = State::default();
    }
    pub fn remove(&self, handle: Handle) {
        let mut state = self.state.lock().unwrap();
        state.active.remove(&handle);
        state.paused.remove(&handle);
    }
    pub fn readings(&self) -> Vec<MeterReading> {
        let state = self.state.lock().unwrap();
        let now = Instant::now();
        let mut result: Vec<_> = state
            .active
            .iter()
            .map(|(&node, slot)| {
                let (peaks, last_update) = slot.read(self.origin, now);
                MeterReading { node, peaks, last_update, paused: false }
            })
            .collect();
        result.extend(state.paused.iter().map(|&node| MeterReading {
            node,
            peaks: vec![],
            last_update: None,
            paused: true,
        }));
        result.sort_by_key(|v| (v.node.epoch, v.node.id));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interleaved_peak_and_silence() {
        let slot = Slot::new(2);
        let data: Vec<u8> = [0.25_f32, -0.5, -0.75, 0.2].into_iter().flat_map(f32::to_ne_bytes).collect();
        slot.process(&data, 8, 1);
        let (peaks, _) = slot.read(Instant::now(), Instant::now());
        assert!((peaks[0] - 0.75).abs() < 0.01 && (peaks[1] - 0.5).abs() < 0.01);
        slot.process(&[0; 8], 8, 2);
        assert_eq!(slot.read(Instant::now(), Instant::now()).0, vec![0.0, 0.0]);
    }
}
