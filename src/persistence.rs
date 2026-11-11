//! Small TOML settings/state files; runtime PipeWire IDs never reach disk.
use crate::{
    app::Page,
    graph::{Graph, group::GroupKey, identity::StableId},
    matrix::layout::ViewOptions,
};
use serde_json::{Value, json};
use std::{collections::HashSet, fs, path::PathBuf};

fn home(env: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(env)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(fallback)))
}
fn settings_path() -> Option<PathBuf> {
    Some(home("XDG_CONFIG_HOME", ".config")?.join("aizome/musubi.toml"))
}
fn state_path() -> Option<PathBuf> {
    Some(home("XDG_STATE_HOME", ".local/state")?.join("aizome/musubi/state.toml"))
}
fn save(path: Option<PathBuf>, body: String) -> std::io::Result<()> {
    let Some(path) = path else { return Ok(()) };
    fs::create_dir_all(path.parent().expect("settings directory"))?;
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, body)?;
    fs::rename(temporary, path)
}

pub fn load_options() -> [ViewOptions; 4] {
    let mut options = std::array::from_fn(|_| ViewOptions::default());
    let Some(body) = settings_path().and_then(|p| fs::read_to_string(p).ok()) else { return options };
    let mut page = None;
    for line in body.lines() {
        if let Some(section) = line.strip_prefix('[').and_then(|line| line.strip_suffix(']')) {
            page = ["audio", "midi", "midi2", "video"].iter().position(|&name| name == section);
            continue;
        }
        let Some((key, value)) = line.split_once(" = ") else { continue };
        let Some(index) = page else { continue };
        let value = value == "true";
        match key {
            "pairs" => options[index].pairs = value,
            "swap" => options[index].swap = value,
            "monitors" => options[index].monitors = value,
            "show_midi_1" => options[index].show_midi_1 = value,
            "list" => options[index].list = value,
            _ => {}
        }
    }
    options
}

pub fn save_options(options: &[ViewOptions; 4]) -> std::io::Result<()> {
    let mut body = String::new();
    for (name, option) in ["audio", "midi", "midi2", "video"].into_iter().zip(options) {
        body.push_str(&format!(
            "[{name}]\npairs = {}\nswap = {}\nmonitors = {}\nshow_midi_1 = {}\nlist = {}\n\n",
            option.pairs, option.swap, option.monitors, option.show_midi_1, option.list
        ));
    }
    save(settings_path(), body)
}

pub(crate) fn stable_group(graph: &Graph, key: &GroupKey) -> Option<String> {
    let port = graph.ports.values().find(|port| graph.group_key(port).as_ref() == Some(key))?;
    let node = port.node_id?;
    let seq = if matches!(key, GroupKey::MidiClient { .. }) {
        port.props.get("api.alsa.seq.client.name").or_else(|| port.props.get("api.alsa.seq.client")).map(String::as_str)
    } else {
        None
    };
    let StableId::Group { device_name, device_serial_or_bus, client_name, node_name, sequencer_client_name } =
        graph.stable_group_id(node, seq)?
    else {
        return None;
    };
    Some(json!([device_name, device_serial_or_bus, client_name, node_name, sequencer_client_name]).to_string())
}

pub fn load_folds() -> [HashSet<String>; 4] {
    let mut folds = std::array::from_fn(|_| HashSet::new());
    let Some(body) = state_path().and_then(|p| fs::read_to_string(p).ok()) else { return folds };
    let mut page = None;
    for line in body.lines() {
        if let Some(section) = line.strip_prefix('[').and_then(|line| line.strip_suffix(']')) {
            page = ["audio", "midi", "midi2", "video"].iter().position(|&name| name == section);
        } else if let (Some(index), Some(value)) = (page, line.strip_prefix("folded = ")) {
            if let Ok(Value::Array(values)) = serde_json::from_str::<Value>(value) {
                folds[index].extend(values.into_iter().filter_map(|v| v.as_str().map(str::to_owned)));
            }
        }
    }
    folds
}

pub fn restore_folds(graph: &Graph, options: &mut [ViewOptions; 4], stored: &[HashSet<String>; 4]) {
    let keys: Vec<_> = graph.ports.values().filter_map(|port| graph.group_key(port)).collect();
    for (option, saved) in options.iter_mut().zip(stored) {
        option.folded =
            keys.iter().filter(|key| stable_group(graph, key).is_some_and(|id| saved.contains(&id))).cloned().collect();
    }
}

pub fn save_folds(graph: &Graph, options: &[ViewOptions; 4], stored: &[HashSet<String>; 4]) -> std::io::Result<()> {
    let mut body = String::new();
    for (i, name) in ["audio", "midi", "midi2", "video"].into_iter().enumerate() {
        let mut keys = stored[i].clone();
        keys.extend(options[i].folded.iter().filter_map(|key| stable_group(graph, key)));
        let mut keys: Vec<_> = keys.into_iter().collect();
        keys.sort();
        body.push_str(&format!(
            "[{name}]\nfolded = {}\n\n",
            serde_json::to_string(&keys).expect("stable group strings")
        ));
    }
    save(state_path(), body)
}

pub fn page_index(page: Page) -> usize {
    match page {
        Page::Audio => 0,
        Page::Midi => 1,
        Page::Midi2 => 2,
        Page::Video => 3,
    }
}
