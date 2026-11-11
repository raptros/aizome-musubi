# Musubi 結び — design proposal (draft 1)

Working name **Musubi** (結び, "tying, joining"; also the knot that binds), generic name **Patchbay**; binary `aizome-musubi`, app id `org.aizome.Musubi`, crate `desktop/aizome/apps/musubi`. Name and generic name are user decisions (alternatives at the end).

Sources: R1 `research/r1-graph.md`, R2 `research/r2-mixer-effects.md`, Aizome look from `crates/aizome-theme/src/lib.rs` (palette), `crates/aizome-kit/src/theme.rs` (metrics), `docs/kit.md`, goldens of Settei Sound, Yukimi Storage, Tansu, Myaku, Kura. Layout reference: Arturia AudioFuse Control Center (16Rig manual 2.0.3 §8.4–8.6; `design/reference/afcc-*.png`). Mockups are HTML renders with the exact palette, Archivo and IBM Plex Mono (`design/mock/*.png`); they are pictures for decisions, not kit output.

## 1. Window and navigation

Yukimi/Settei model: `sidebar_layout` (200pt `sunk` sidebar) + toolbar + content + status bar.

| Sidebar | Page | Content |
|---|---|---|
| app card | — | tile + "Musubi" + "結び · Patchbay" (Yukimi's card) |
| PATCHBAY | Audio · MIDI · MIDI 2.0 · Video (· ALSA Sequencer, decision D3) | one patch matrix per media kind; detail = number of links |
| MIXER | Mixer · Effects | strips; effect chains |
| SYSTEM | Devices · Graph · Setups | profiles/routes; rate, buffer, load, xruns; saved routings |
| footer card | — | "PipeWire 1.6.9", `48 kHz · 256 · 5.3 ms`, `DSP 11% · 0 xruns`, load bar (Yukimi's machine card) |

Pages with nothing in them (no MIDI 2.0 ports) stay listed with an empty state that says why ("No MIDI 2.0 ports. Devices and apps that speak UMP appear here.").

Menus: kit standard + **Edit** Undo/Redo of routing changes (❖Z / ⇧❖Z), **View** (pages ❖1…❖9, Pairs/Channels, Show Monitors, Swap Axes, Collapse All Groups, Inspector ⌥❖I), **Routing** (Disconnect All from Selection, Connect in Order, Save Setup…, Keep Connected). Every command reachable from the palette.

## 2. Patch matrix (the crucial part)

Pictures: `mock/audio-night.png`, `mock/audio-day.png`, `mock/midi-night.png`.

- **Axes:** rows = where sound comes **from** (output ports), columns = where it goes **to** (input ports). Toolbar **Swap** flips them, remembered per page. Corner labels `FROM ↓` / `TO →` (AFCC's "sources/destinations, not inputs/outputs" advice).
- **Groups = interfaces.** Grouping key (R1 §4): PipeWire Device (`device.id`) → ALSA sequencer client on the MIDI bridge → client (`client.id`) → node. A row group is a `chrome` box spanning its rows (icon, name, one-line detail: "USB · Pro Audio", "JACK · 6 outputs", the stream's media title); a column group is the same box upright, name set vertically (bottom-to-top), icon at its foot. 8pt gaps between groups on both axes. A 3pt stripe in the group's kind colour: amber hardware, blue application, violet effects, cyan virtual (MIDI Through, loopbacks); legend in the corner.
- **Ports** labelled short (mono 11pt `text3`): channel names `L·R`, `In 1`, `AUX3`, `Out 5`; vertical in the column header. Full names, aliases, node name, format in the inspector and the status bar.
- **Cells:** 24pt pitch (kit minimum target), 18pt square. Empty = `sunk` with a `line` edge. Connected = `selection` (ai indigo) with a `blue` edge and a 1pt top highlight. Asked-but-not-yet-seen = dashed `blue` edge + small square; link error = `red` edge + the error in the status bar. Pairs: a stereo row/column is one line; its cell is split on the diagonal (upper-left = L, lower-right = R), so L only / R only / both are visible; a click on a pair cell links L→L and R→R as two links, one undo step.
- **Pairs / Channels** segmented in the toolbar. Pairs only joins ports that really are FL/FR of one node (R1 "Stereo collapse"); pro-audio `AUXn` channels are never paired.
- **Collapsed groups** (AFCC): click a group's box to fold it to one line labelled `In 1–8 ▸`; its cells show how many links the group holds (count + bar). Folded state remembered per group identity.
- **Hover:** crosshair: the row and column tinted `selection` at 16%, their labels go `text` semibold; the status bar reads `Scarlett 4i4 · In 1 capture_AUX0 → Discord · In input_MONO · connected — click to disconnect`.
- **Pointer:** click toggles; drag paints: starting on an empty cell connects every cell dragged over, starting on a connected one disconnects; a diagonal drag connects 1→1, 2→2 (AFCC's diagonal tool). Right-click: Disconnect all from this port / to this port, Connect group in order, Fold group, Show in Mixer.
- **Keyboard:** arrows move the focused cell (2pt `focus` ring), Space toggles, ⌥arrows jump a group, Return opens the inspector for the port, type-to-filter. Undo/Redo.
- **Filter field** narrows both axes to groups/ports whose names match. **Monitors** toggle shows sinks' monitor outputs (hidden by default, R1 §5).
- **Inspector** (right, 260pt, ⌥❖I): selected port's level meter (audio) or activity LEDs per MIDI channel (MIDI), "Goes to" list with unlink buttons, and the port's PipeWire facts (node name, port name, channel, `format.dsp`, device).
- **Session-manager links** (WirePlumber's) look the same as the user's: a link is a link. A link the session manager re-makes after the user removed it gets a one-line notice with "Keep it disconnected" (adds it to the active setup as excluded) — decision D5.
- Implementation: one custom kit `Widget` (`PatchMatrix`) drawing on the canvas, headers fixed while the grid scrolls both ways (`cx.scrolled` with the header bands as `fixed`), only visible cells built, hover/focus as `parts` so moving the pointer redraws two bands, not the grid. Vertical text needs `Canvas::rotate` with text — first spike of the implementation; fallback is a 45° header or short labels plus the status bar.

## 3. MIDI pages

R1 §2/§5: ALSA hardware MIDI reaches PipeWire through one `Midi-Bridge` node whose ports are split into groups by ALSA client (`KeyLab 61 mk3`, `Digitakt II`, `MIDI Through`); JACK/PipeWire apps are their own groups. All of them are PipeWire MIDI ports and link to each other, so they share one **MIDI** page. **MIDI 2.0** (UMP, `32 bit raw UMP`) gets its own page; links between MIDI 1.0 and UMP ports are allowed only after the formats are checked, with a note that PipeWire converts. Inspector shows activity LEDs for channels 1–16 (decision D3 covers ALSA-level `aconnect` routing).

## 4. Mixer

Picture: `mock/mixer-night.png`. AFCC's strip model, Aizome's look.

- Sections with mono captions over the strips: **OUTPUTS** (hardware sinks), **INPUTS** (hardware sources), **PLAYING** (app playback streams), **RECORDING** (app capture streams), **EFFECTS** (chains' virtual devices). **MAIN** strip pinned right: the default output; its pop-up chooses the default.
- Strip (84pt, `chrome`, `line` edge, bevel): tile icon + name + detail; pop-up = where it goes (streams) or which port/route (devices: "Headphones", "Line Out"); balance (L–R); buttons M (mute, `red` when on), ◉ default (devices), link channels; vertical fader with mono dB scale (+6 … −60, −∞) and a cubic law like PipeWire's own sliders; segmented meter per channel (`green`, `amber` above −9 dB, `red` above 0, peak hold); value box `−4.5 dB` (double-click to type). Unlinked channels split the fader per channel.
- Toolbar: dB/% segmented, Meters toggle, Show… popover (AFCC's "+/- Channels": which strips show), filter.
- Selected strip: `selection` edge; arrows: ↑↓ volume, ←→ balance, M mute.
- **Control path (R2 §1–3):** devices and Pulse-visible streams go through the existing `aizome-sys::audio` service (pipewire-pulse): the same cubic volume, balance, port, profile, default and move-stream semantics as Settei and the volume keys, and WirePlumber keeps restoring routes. Nodes Pulse can't see (JACK clients, filter streams) get a fader only when their node exposes writable `Props.channelVolumes`; otherwise the strip shows its meter and says "No volume of its own". Never two writers for one control.
- **Meters (R2 §5):** native peak streams owned by the Musubi PipeWire thread (pwvucontrol's model: `resample.peaks`, `stream.monitor`, `node.passive`, `node.dont-reconnect`, honest `media.name = "Musubi level meter"`), one per *visible* strip, capped (24), ~30 Hz to the UI, all stopped when the page or window is hidden. Peak hold 1.5 s; no clip claims from `pw-top` errors.

## 5. Effects

Picture: `mock/effects-night.png`. v1 = **equaliser chains** on PipeWire's filter-chain builtins (`bq_lowshelf`, `bq_peaking`, `bq_highshelf`, per channel): a chain appears as a virtual output (or input) named by the user and sends to a chosen device.
- Page: chains list (status dot: running/failed/target missing) + "Other effects" (filter nodes Musubi didn't make, e.g. EasyEffects, read-only, routable in the matrix); editor: name, "Sends to", preset; response curve (canvas, 20 Hz–20 kHz log, ±15 dB, numbered square handles in band colours; drag = frequency/gain, scroll = Q); band table (type pop-up, frequency, gain, Q, on); Add Band, Reset, Bypass.
- Runtime (R2 §7): chains live in **one dedicated helper** `pipewire -c $XDG_CONFIG_HOME/aizome/musubi/effects.conf` run by a user unit `aizome-musubi-effects.service` (shipped by the Nix module, started/stopped by Musubi), so effects outlive the app and need no daemon restart. Config written atomically from Musubi's own model; Musubi never edits other drop-ins. Band edits: first spike whether filter-chain's `Props.params` (`"<node>:<control>"`) can be set live; if yes, live edits; if not, the helper restarts on release (brief gap, told in the status bar).
- Later (not v1): LV2/LADSPA hosting, convolver, per-chain meters.

## 6. System pages

- **Devices** (Tansu's layout): devices list (icon, name, bus, profile, status dot) and details: profile pop-up (incl. Pro Audio where offered), routes per direction with availability, the device's nodes with channel counts. Profile/route writes through `aizome-sys::audio` (`set_card_profile`, port) so WirePlumber's restore holds.
- **Graph** (`mock/graph-night.png`): sample rate (Automatic / forced) and buffer size (64…1024 / Auto) via the `settings` metadata (`clock.force-rate`, `clock.force-quantum`; 0 = automatic), labelled as one graph period (`256 / 48 kHz = 5.3 ms`), not round-trip latency; "Keep after restart" writes `pipewire.conf.d/90-aizome-musubi-clock.conf` (`default.clock.rate` / `default.clock.quantum`). DSP load and xrun charts (Myaku's style) and a driver/follower table from the Profiler (`pw-top`'s numbers), sampled only while the page shows.
- **Setups** (qpwgraph's patchbay): named sets of links matched by stable names (device/node/port names; never runtime ids or serials; ambiguous matches reported, not guessed). **Keep connected** re-makes them when their ports appear; **Exclusive** removes other links on the setup's ports. Active setup shown in the matrix toolbar (`Studio · kept`). Works while Musubi runs (it stays running without a window, like a menu-bar extra) — decision D5.

## 7. Architecture

- `apps/musubi`: Program (Elm model) + views + `PatchMatrix`, `Fader`, `LevelMeter`, `EqCurve` widgets (app-local until another app needs them).
- **Graph thread** (pattern of `aizome-media/src/pipewire.rs` and Helvum; R1 §6): owns MainLoop/Context/Core/Registry, all proxies and listeners, meter streams, `settings`/`default` metadata, profiler; commands in through `pw::channel`; deltas out through `Cx::stream()`, coalesced to at most one UI message per frame; reconnect with epoch reset after core errors.
- Graph model and grouping/classification per R1's implementation summary; port identity for setups/undo by stable names.
- `aizome-sys::audio::Audio` for Pulse-visible volume/mute/balance/route/profile/default/move (R2 §1).
- Effects helper: generated config + `systemd --user` unit via Nix module (`desktop/aizome/nix/module.nix`).
- A `Backend` trait with a fake implementation for headless Harness tests and goldens.
- Settings `$XDG_CONFIG_HOME/aizome/musubi.toml`; setups in `$XDG_DATA_HOME/aizome/musubi/setups/`.

## 8. Verification plan (draft)

Headless Harness tests with the fake backend (matrix toggles, pairs, folding, undo, filter; mixer keys) + goldens Night/Day of every page; a live test against a private `pipewire` + `wireplumber` with null sinks and loopback nodes in an isolated runtime dir (component level); a NixOS VM check (`nix/vm-*-test.nix` pattern, test sinks already used by `vm-settings-test.nix`, `snd-virmidi` for MIDI) driving the packaged app with real input and screenshots reviewed by `sekka-visual`.

## Decisions for the user
D1 name · D2 scope of v1 · D3 ALSA-sequencer page · D4 default axes · D5 setups/keep-connected behaviour.
