# Phase 1 — the patch matrix

Scope: `design.md` §1–2 (window, Audio/MIDI/MIDI 2.0/Video matrices, List view, inspector without meter, undo) on a live PipeWire backend, packaged, tested, documented. Out of scope: mixer, meters, devices, effects, clock, setups (later phases). Architecture: `architecture.md`. Decisions: `decisions.md`.

Done means every row of the acceptance table below is shown by the evidence named there, the code review has no open blocker/major finding, and `sekka-visual` passed the screenshots.

## Slices (one worker at a time, in order)

### S1 — skeleton, build, spikes
1. `Cargo.toml` (decision A4; `pipewire = { version = "0.10.1", features = ["v1_2_0"] }`; profiles as Tsutsumi), `src/main.rs` (`--version`, one instance via the kit's mechanism used by Tsutsumi/Sekka apps), `src/lib.rs`, `src/app.rs` (Program with `AppInfo::new("org.aizome.Musubi", "Musubi", "Patchbay")`, sidebar with the four Patchbay pages, toolbar, status bar, empty content).
2. `flake.nix` + `nix/package.nix` modelled on sekka-edu's selective `aizomeFor`/`fontsFor` and Tsutsumi's single package (`packages.musubi`/`default`, `apps.default`, `devShells.default`, `checks.musubi`, `nixosModules.default` with `programs.musubi.enable`), PipeWire as a build/runtime input. `flake.lock` pinned to Sekka `aebca32e5ef6245a5ec68d2c3c8953ca957c3375`.
3. **Spike A — `PatchMatrix` geometry** (`src/matrix/widget.rs`): a custom `Widget` with four regions (corner, column header x-only, row header y-only, body x+y), scroll offsets in widget state, wheel/touchpad scrolling both axes, hit testing over the full 24pt pitch, vertical (rotated −90°) group names and port labels in the column header, hover crosshair and focus ring updated through `invalidate_rect` of the old/new bands (not a view rebuild). Synthetic 200 × 200-port layout. Prove with Harness: goldens Night/Day, `check_frame_exact` after hover moves and after diagonal scrolling, and a timing test (ignored by default) measuring hover/scroll/toggle frames at 1× and 2× against the 4 ms budget; write the numbers in `docs/performance.md`.
4. **Spike B — private PipeWire** (`scripts/private-pipewire`): starts `pipewire` (and `wireplumber` with ALSA, BlueZ, libcamera and V4L2 monitors disabled) with a test config in a private runtime directory (`PIPEWIRE_RUNTIME_DIR`, `XDG_RUNTIME_DIR`, private `XDG_CONFIG_HOME`), creating: two null audio sinks (one stereo, one 4-channel AUX), one null source, a stereo playback stream (`pw-cat`/`pw-play` on a generated file or `pw-loopback`), MIDI 1.0 producer and consumer nodes, and a video source if the toolchain allows (`pw-v4l2`-free: a `videotestsrc` GStreamer pipeline into `pipewiresink` if available, else record the gap). Runs a command inside it, then tears everything down. Never touches the session's PipeWire. Capture `pw-dump` of it as `tests/fixtures/private.json`.
Acceptance S1: package builds (`nix build .#musubi` and `cargo build --locked` in the dev shell), app opens in the Harness, spike goldens and frame tests pass, frame numbers recorded, private PipeWire script prints `pw-link -l` showing a link it made and leaves no process behind.

### S2 — graph model (pure)
`src/graph/{mod,classify,group,identity}.rs`: the store and algorithms of `design.md` §2.4–2.5 and `architecture.md` (out-of-order children kept pending; sparse links by pair; port kind from `format.dsp` → EnumFormat → `media.class`; MIDI bridge first, device, client, node grouping; labels; channel order; strict pairing predicate; stable identity + ambiguity). `src/backend/fake.rs` loads `pw-dump`-shaped JSON fixtures. Fixtures (`tests/fixtures/`): `private.json` (S1), `studio.json` (hand-made: USB pro-audio 4-in/4-out, built-in analog, HDMI, Bluetooth A2DP, Firefox, Spotify, a JACK DAW with 6 outputs/8 inputs, a filter-chain EQ, Discord mono input, OBS stereo input), `midi.json` (MIDI bridge with two USB keyboards and Midi Through, a JACK MIDI app, one UMP app), `video.json`, `empty.json`. Unit tests for every predicate, label fallback, ordering (AUX10 after AUX9) and ambiguity case.

### S3 — PipeWire backend
`src/backend/{mod,pipewire}.rs`: the thread, registry, binds (Node, Port, Link, Device, Client), Port EnumFormat, `Connect`/`Disconnect` with `link-factory` (`object.linger=true`) and `registry.destroy_global`, per-request results (error text from core errors), deltas coalesced ≤ 16 ms, connection up/down, reconnect with backoff and epoch. Live tests (`tests/live.rs`, run inside `scripts/private-pipewire`, ignored unless `MUSUBI_LIVE=1`): enumerate, connect, observe link, disconnect, reject output→output, survive a PipeWire restart (epoch increments, stale ids rejected).

### S4 — the pages
Matrix page (`src/matrix/{layout,widget,gesture}.rs`, `src/inspector.rs`, `src/routes.rs` for List view and Add Route sheet, `src/undo.rs`), all of `design.md` §2: states (empty/connected/half/asked/error/not allowed), pairs/channels, collapse (informational cells), swap, From/To filters with hidden counts, monitors toggle, click, line drag with preview, keyboard model, context menus, undo/redo, WirePlumber re-link notice, MIDI 2.0 "Show MIDI 1.0" + conversion confirmation, empty and unavailable states, inspector, List view, semantics (Table/Cell, labels, Toggle action). Menus and palette commands (`design.md` §1 menu list, phase 1 items only). View settings persisted (`$XDG_CONFIG_HOME/aizome/musubi.toml`, folded groups in state) by stable identity. Harness tests driving real input against the fake backend; goldens Night/Day: audio (studio), audio swapped, audio channels mode, MIDI, MIDI 2.0 with conversion sheet, video, List view filtered, Add Route sheet, empty, PipeWire unavailable.

### S5 — packaging, docs, VM
`data/org.aizome.Musubi.desktop` (`Name=Musubi`, `GenericName=Patchbay`, Keywords: patchbay;pipewire;jack;midi;routing;connections;qpwgraph;helvum;audio, `Categories=AudioVideo;Audio;Midi;`), `data/org.aizome.Musubi.svg` from a generator in the style of Sekka's `icons/genicons.py` (Q7 default: a knot on the 64-grid), installed by `nix/package.nix`. `README.md` (experience-first, honest limits, Night/Day screenshots from goldens), `DEVELOPERS.md` (sibling checkout, pins, tests and levels, private PipeWire, VM check). `nix/vm-test.nix` exported as `checks.x86_64-linux.vm`: NixOS VM with Sekka's Aizome module (from the pinned input) + `programs.musubi.enable`, PipeWire + WirePlumber, `snd-virmidi` (bridge MIDI) and `vivid` (video), test sinks like Sekka's `vm-settings-test.nix`; drives the packaged app with `aizomectl input` and `AIZOME_KIT_INSPECT` (pattern: Sekka `nix/vm-tokei-reorder-test.{nix,py}`): open Audio, click a cell, verify `pw-link -l` shows the link, undo, verify it's gone, open MIDI and link two virmidi ports, screenshot each step; copy screenshots and a screen recording out.

## Acceptance

| # | Behaviour | Evidence |
|---|---|---|
| 1 | Builds from its flake and from the dev shell; `--version` | build logs |
| 2 | Audio/MIDI/MIDI 2.0/Video pages show the right ports, grouped and labelled per §2.4 | unit tests on fixtures + goldens |
| 3 | Click, line drag, keyboard Space make/remove real links; pairs make two | live tests + VM |
| 4 | Illegal cells can't be acted on; server errors show on the cell and status bar | unit + live (output→output, unknown kind) |
| 5 | Undo/redo of a transaction, including after a port vanished | Harness + live |
| 6 | PipeWire restart: unavailable state, recovery, no stale ids | live |
| 7 | Filters, collapse, swap, pairs/channels, monitors behave per §2.2–2.3 and persist | Harness |
| 8 | List view and Add Route reach every route by keyboard; semantics exposed | Harness (`semantics()`, keyboard only) |
| 9 | Hover/focus/scroll damage is exact; frames within 4 ms at 200 × 200 (1×, 2×) or documented | `check_frame_exact`, timing test, `docs/performance.md` |
| 10 | Night/Day goldens for every page/state listed in S4 | `tests/snapshots/` + sekka-visual |
| 11 | Packaged app in a NixOS VM links real virmidi ports and a test sink from the UI | VM check, screenshots, recording |
| 12 | Docs: README, DEVELOPERS, architecture (updated), decisions (updated) | review |
