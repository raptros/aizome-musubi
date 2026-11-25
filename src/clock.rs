//! Observed PipeWire settings and per-page diagnostics.
pub mod page;
use crate::effects::paths::{self, Paths};
use std::{
    collections::{HashMap, VecDeque},
    fs, io,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub rate: Option<u32>,
    pub allowed_rates: Vec<u32>,
    pub quantum: Option<u32>,
    pub min_quantum: Option<u32>,
    pub max_quantum: Option<u32>,
    pub force_rate: Option<u32>,
    pub force_quantum: Option<u32>,
    pub read_only: Option<String>,
}
impl Settings {
    pub fn observe(&mut self, subject: u32, key: Option<&str>, value: Option<&str>) -> bool {
        if subject != 0 {
            return false;
        }
        let Some(key) = key else {
            *self = Self::default();
            return true;
        };
        let number = value.and_then(|s| s.parse::<u32>().ok()).filter(|&n| n > 0);
        match key {
            "clock.rate" => self.rate = number,
            "clock.allowed-rates" => {
                self.allowed_rates = value.map_or_else(Vec::new, |s| {
                    s.trim_matches(|c| c == '[' || c == ']')
                        .split(|c: char| c.is_whitespace() || c == ',')
                        .filter_map(|s| s.parse::<u32>().ok())
                        .filter(|&n| n > 0)
                        .collect()
                })
            }
            "clock.quantum" => self.quantum = number,
            "clock.min-quantum" => self.min_quantum = number,
            "clock.max-quantum" => self.max_quantum = number,
            "clock.force-rate" => self.force_rate = number,
            "clock.force-quantum" => self.force_quantum = number,
            _ => return false,
        }
        true
    }
    pub fn rates(&self) -> Vec<u32> {
        let mut rates = self.allowed_rates.clone();
        rates.sort_unstable();
        rates.dedup();
        rates
    }
    pub fn quanta(&self) -> Vec<u32> {
        let (Some(min), Some(max)) = (self.min_quantum, self.max_quantum) else {
            return Vec::new();
        };
        if min > max {
            return Vec::new();
        }
        (6..=13).map(|bit| 1_u32 << bit).filter(|&n| n >= min && n <= max).collect()
    }
    pub fn period_ms(&self) -> Option<f64> {
        Some(self.quantum? as f64 * 1000. / self.rate? as f64)
    }
    pub fn valid_rate(&self, rate: u32) -> bool {
        self.rates().contains(&rate)
    }
    pub fn valid_quantum(&self, quantum: u32) -> bool {
        self.quanta().contains(&quantum)
    }
}

pub fn force_value(value: Option<u32>) -> String {
    value.unwrap_or(0).to_string()
}

pub fn drop_in(rate: Option<u32>, quantum: Option<u32>) -> io::Result<String> {
    if rate.is_none() && quantum.is_none() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "choose an explicit rate or quantum"));
    }
    if rate.is_some_and(|n| !(8_000..=384_000).contains(&n))
        || quantum.is_some_and(|n| n < 64 || n > 8192 || !n.is_power_of_two())
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid explicit clock value"));
    }
    let mut text = String::from("# Managed by Musubi. Applies at the next PipeWire start.\ncontext.properties = {\n");
    if let Some(rate) = rate {
        text.push_str(&format!("    default.clock.rate = {rate}\n"));
    }
    if let Some(quantum) = quantum {
        text.push_str(&format!("    default.clock.quantum = {quantum}\n"));
    }
    text.push_str("}\n");
    Ok(text)
}
pub fn persist(paths: &Paths, rate: Option<u32>, quantum: Option<u32>) -> io::Result<()> {
    paths::atomic_write(&paths.clock, drop_in(rate, quantum)?.as_bytes())
}
pub fn clear(paths: &Paths) -> io::Result<()> {
    match fs::remove_file(&paths.clock) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DriverRow {
    pub id: u32,
    pub node: String,
    pub driver: bool,
    pub rate: u32,
    pub quantum: u32,
    pub busy: f64,
    pub wait: f64,
    pub errors: u64,
    pub xruns: u64,
}
#[derive(Default)]
pub struct Load {
    pub rows: Vec<DriverRow>,
    pub samples: VecDeque<(Instant, f64)>,
    baseline: HashMap<u32, (u64, u64)>,
}
impl Load {
    pub fn observe(&mut self, rows: Vec<DriverRow>, now: Instant) {
        for row in &rows {
            self.baseline.entry(row.id).or_insert((row.xruns, row.errors));
        }
        let load = rows
            .iter()
            .filter(|r| r.driver && r.quantum > 0)
            .map(|r| (r.busy / (r.busy + r.wait).max(f64::EPSILON) * 100.).clamp(0., 100.))
            .fold(0_f64, f64::max);
        self.samples.push_back((now, load));
        while self
            .samples
            .front()
            .is_some_and(|(time, _)| now.saturating_duration_since(*time) > Duration::from_secs(60))
        {
            self.samples.pop_front();
        }
        self.rows = rows;
    }
    pub fn delta(&self, row: &DriverRow) -> (u64, u64) {
        self.baseline.get(&row.id).map_or((0, 0), |&(x, e)| (row.xruns.saturating_sub(x), row.errors.saturating_sub(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_and_choices() {
        let mut settings = Settings::default();
        for (key, value) in [
            ("clock.rate", "48000"),
            ("clock.quantum", "256"),
            ("clock.allowed-rates", "[ 44100 48000 96000 ]"),
            ("clock.min-quantum", "128"),
            ("clock.max-quantum", "1024"),
            ("clock.force-quantum", "256"),
        ] {
            assert!(settings.observe(0, Some(key), Some(value)));
        }
        assert_eq!(settings.period_ms(), Some(256. / 48.));
        assert_eq!(settings.quanta(), vec![128, 256, 512, 1024]);
        assert_eq!(settings.rates(), vec![44100, 48000, 96000]);
        settings.observe(0, Some("clock.force-quantum"), Some("0"));
        assert_eq!(settings.force_quantum, None);
        assert!(!settings.observe(1, Some("clock.rate"), Some("100")));
    }
    #[test]
    fn explicit_drop_in_only() {
        assert!(drop_in(None, None).is_err());
        assert!(drop_in(Some(0), None).is_err());
        assert!(drop_in(None, Some(255)).is_err());
        assert_eq!(
            drop_in(Some(48000), Some(256)).unwrap(),
            "# Managed by Musubi. Applies at the next PipeWire start.\ncontext.properties = {\n    default.clock.rate = 48000\n    default.clock.quantum = 256\n}\n"
        );
        assert!(!drop_in(None, Some(256)).unwrap().contains("rate ="));
        assert_eq!(force_value(None), "0");
    }
    #[test]
    fn deltas_since_opened_and_rolling_window() {
        let mut load = Load::default();
        let now = Instant::now();
        let row = DriverRow {
            id: 42,
            node: "test".into(),
            driver: true,
            rate: 48000,
            quantum: 256,
            busy: 1.,
            wait: 3.,
            errors: 7,
            xruns: 5,
        };
        load.observe(vec![row.clone()], now);
        let mut later = row.clone();
        later.errors = 11;
        later.xruns = 8;
        load.observe(vec![later.clone()], now + Duration::from_secs(61));
        assert_eq!(load.delta(&later), (3, 4));
        assert_eq!(load.samples.len(), 1);
        assert_eq!(load.samples[0].1, 25.);
    }
}
