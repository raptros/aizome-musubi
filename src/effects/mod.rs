//! Persistent EQ definition and deterministic PipeWire filter-chain configuration.
pub mod paths;
pub mod service;

use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandKind {
    Peaking,
    LowShelf,
    HighShelf,
}

impl BandKind {
    fn label(&self) -> &'static str {
        match self {
            Self::Peaking => "bq_peaking",
            Self::LowShelf => "bq_lowshelf",
            Self::HighShelf => "bq_highshelf",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub kind: BandKind,
    pub freq: f32,
    pub gain: f32,
    pub q: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "identity", rename_all = "snake_case")]
pub enum Target {
    FollowDefault,
    Output(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chain {
    pub id: String,
    pub name: String,
    pub target: Target,
    pub bands: Vec<Band>,
    pub bypass: bool,
    pub enabled: bool,
}

impl Chain {
    pub fn validate(&self) -> io::Result<()> {
        paths::checked_id(&self.id)?;
        if self.name.is_empty()
            || self.name.len() > 128
            || self.name.chars().any(char::is_control)
            || self.bands.len() > 8
        {
            return Err(invalid("invalid chain name or band count"));
        }
        if let Target::Output(target) = &self.target {
            if target.is_empty() || target.len() > 256 || target.chars().any(char::is_control) {
                return Err(invalid("invalid output identity"));
            }
        }
        for band in &self.bands {
            if !band.freq.is_finite()
                || !(20.0..=20_000.0).contains(&band.freq)
                || !band.gain.is_finite()
                || !(-15.0..=15.0).contains(&band.gain)
                || !band.q.is_finite()
                || !(0.1..=20.0).contains(&band.q)
            {
                return Err(invalid("invalid EQ band"));
            }
        }
        Ok(())
    }

    pub fn config(&self) -> io::Result<String> {
        self.validate()?;
        let quote = |text: &str| serde_json::to_string(text).expect("string serialization");
        let mut out = format!(
            "context.properties = {{ application.name = \"Musubi EQ helper\" }}\ncontext.spa-libs = {{ audio.convert.* = audioconvert/libspa-audioconvert }}\ncontext.modules = [\n  {{ name = libpipewire-module-rt args = {{ nice.level = 0 }} flags = [ ifexists nofail ] }}\n  {{ name = libpipewire-module-protocol-native }}\n  {{ name = libpipewire-module-client-node }}\n  {{ name = libpipewire-module-adapter }}\n  {{ name = libpipewire-module-filter-chain\n    args = {{\n      node.description = {}\n      filter.graph = {{\n        nodes = [\n",
            quote(&format!("{} EQ", self.name))
        );
        let count = self.bands.len().max(1);
        for (index, band) in self.bands.iter().enumerate() {
            for side in ["left", "right"] {
                out.push_str(&format!("          {{ type = builtin name = {side}{index} label = {} control = {{ \"Freq\" = {} \"Q\" = {} \"Gain\" = {} }} }}\n", band.kind.label(), band.freq, band.q, if self.bypass { 0.0 } else { band.gain }));
            }
        }
        if self.bands.is_empty() {
            for side in ["left", "right"] {
                out.push_str(&format!("          {{ type = builtin name = {side}0 label = bq_peaking control = {{ \"Freq\" = 1000 \"Q\" = 1 \"Gain\" = 0 }} }}\n"));
            }
        }
        out.push_str("        ]\n");
        if count > 1 {
            out.push_str("        links = [\n");
            for index in 1..count {
                for side in ["left", "right"] {
                    out.push_str(&format!(
                        "          {{ output = \"{side}{}:Out\" input = \"{side}{index}:In\" }}\n",
                        index - 1
                    ));
                }
            }
            out.push_str("        ]\n");
        }
        out.push_str("        inputs = [ \"left0:In\" \"right0:In\" ]\n");
        out.push_str(&format!("        outputs = [ \"left{}:Out\" \"right{}:Out\" ]\n", count - 1, count - 1));
        out.push_str("      }\n");
        out.push_str(&format!("      capture.props = {{ node.name = {} media.class = Audio/Sink audio.channels = 2 audio.position = [ FL FR ] node.always-process = true }}\n", quote(&format!("musubi_eq.{}", self.id))));
        out.push_str(&format!("      playback.props = {{ node.name = {} node.passive = true audio.channels = 2 audio.position = [ FL FR ]", quote(&format!("musubi_eq.{}.playback", self.id))));
        if let Target::Output(target) = &self.target {
            out.push_str(&format!(" target.object = {}", quote(target)));
        }
        out.push_str(" }\n    }\n  }\n]\n");
        Ok(out)
    }
}

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_golden() {
        let chain = Chain {
            id: "studio".into(),
            name: "Studio".into(),
            target: Target::Output("alsa_output.test".into()),
            bands: vec![
                Band { kind: BandKind::Peaking, freq: 1000., gain: 12., q: 1. },
                Band { kind: BandKind::HighShelf, freq: 6000., gain: -2., q: 0.7 },
            ],
            bypass: false,
            enabled: true,
        };
        assert_eq!(serde_json::from_str::<Chain>(&serde_json::to_string(&chain).unwrap()).unwrap(), chain);
        assert_eq!(chain.config().unwrap(), include_str!("../../tests/fixtures/effects-chain.conf"));
        let mut bypass = chain.clone();
        bypass.bypass = true;
        bypass.target = Target::FollowDefault;
        assert!(!bypass.config().unwrap().contains("target.object"));
        assert!(!bypass.config().unwrap().contains("\"Gain\" = 12"));
        assert!(bypass.config().unwrap().contains("\"Gain\" = 0"));
    }
}
