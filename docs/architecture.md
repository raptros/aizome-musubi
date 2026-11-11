# Musubi architecture

Living document: every change must fit here or change this first. Product behaviour is in `design.md`; decisions in `decisions.md`.

## Repository and dependencies

One Cargo package, `aizome-musubi` (binary `aizome-musubi`, app id `org.aizome.Musubi`), shaped like Tsutsumi (`research/external-apps.md` §5, decision A4):

| Path | What |
|---|---|
| `src/` | the app (modules below) |
| `tests/` | Harness tests (fake backend), goldens in `tests/snapshots/`, live tests against a private PipeWire |
| `data/` | `org.aizome.Musubi.desktop`, `org.aizome.Musubi.svg` (+ its generator) |
| `nix/package.nix`, `flake.nix` | package, dev shell, checks, NixOS module `programs.musubi.enable`, VM check |
| `docs/` | this, `design.md`, `decisions.md`, `open-questions.md`, `research/`, `design/` (mockups, reviews) |

Dependencies: `aizome-kit` by path from a sibling Sekka checkout (`../sekka/desktop/aizome/crates/aizome-kit`), pinned in `flake.nix` to Sekka `aebca32e…`; `pipewire` 0.10.1 (`v1_2_0`), as Sekka's own lock has it. Later phases add `aizome-sys` (Pulse service) the same way.

## Layers

```mermaid
flowchart LR
  subgraph PW[PipeWire thread — owns every proxy]
    REG[registry + bound Node/Port/Link/Device/Client/Metadata] --> MODEL_PW[graph store]
    CMD[command queue pw::channel] --> ACT[link-factory create / registry destroy / metadata / params]
  end
  MODEL_PW -- GraphDelta, coalesced per frame --> STREAM[Cx::stream]
  STREAM --> APP[Program::update]
  APP --> GRAPH[graph::Graph — plain data]
  GRAPH --> DERIVE[matrix::Layout per page]
  DERIVE --> VIEW[view: PatchMatrix / List / Inspector]
  VIEW -- Msg --> APP
  APP -- Command --> CMD
```

| Module | Owns | Never |
|---|---|---|
| `backend::pipewire` | the PipeWire MainLoop/Context/Core/Registry on one thread; proxies, listeners; command execution; epoch; reconnect | sends proxies across threads; blocks on the UI |
| `backend::fake` | a scriptable graph for tests (same `Backend` trait) | — |
| `graph` | plain-data store: clients, devices, nodes, ports, links, metadata; sparse `links_by_pair`; pending (out-of-order) children; apply `GraphDelta` | knows about pages or pixels |
| `graph::classify` | port kind (format.dsp → EnumFormat → media.class), channel position, pairing predicate | — |
| `graph::identity` | live id (`GlobalId` + epoch) and stable identity (device name + serial/bus, node name, direction, port name, seq client); ambiguity detection | persists runtime ids |
| `matrix::layout` | for (page, view options, filters, folded groups): ordered row/column groups and lines, offsets, cell state lookup | allocates the cross-product |
| `matrix::widget` (`PatchMatrix`) | fixed corner, x-scrolling column header, y-scrolling row header, xy-scrolling body; clamped offsets in widget state, pitch-based hit testing, clipped drawing, targeted hover/focus damage and redraw-on-scroll, table semantics | owns graph data (gets an `Rc<Layout>`) |
| `routes` (List view) | kit `table` over the page's links; Add Route sheet | — |
| `undo` | transactions of link intents by stable identity; inverse on observed results only | replays runtime ids from another epoch |
| `app` | `Program`: model, `Msg`, `update`, `view`, menus, settings persistence | does I/O in `view` |

Later phases add: `mixer` (strips, `Fader`, `LevelMeter`; Pulse bridge), `meters` (peak streams on the PipeWire thread), `devices`, `effects` (config generation + unit control), `clock` (settings metadata + profiler), `setups` + `keeper` (user service binary mode). Each gets a row here when it lands.

## Matrix regions and scrolling

The corner stays fixed; column labels follow x only, row labels follow y only, and the body follows both. Wheel deltas clamp the widget's stored offsets to the content extents. The spike invalidates the affected header regions and the body on scroll (plus the corner in the wheel path), then redraws them; it does not ask the compositor to pixel-scroll overlapping regions. This keeps boundary pixels and clipped rotated labels correct for horizontal, vertical and diagonal movement without coordinating three disjoint scroll commands. Hover and focus instead invalidate old/new row and column bands, including their header labels; keyboard focus that reveals an offscreen cell uses the scroll redraw path. Measured release costs for both scales are in `performance.md`.

## Threads and messages
- UI thread: kit event loop, `Program`.
- PipeWire thread: everything PipeWire. Commands in: `Connect{out, in, request}`, `Disconnect{link, request}`, `Shutdown` (+ later phases). Events out: `GraphDelta{epoch, upserts, removals}`, `CommandResult{request, Ok|Err(reason)}`, `Connection{Up(version)|Down(reason)}`. Deltas are batched on the PipeWire thread and flushed at most every 16 ms, so a burst (profile switch, app start) is one UI message.

## Identity
- Live actions: `(epoch, GlobalId)`. An epoch ends at disconnect; every pending command fails and the store is cleared.
- Persisted (Swap per page, folded groups, undo across reconnects, setups): stable identity; two live objects with the same stable identity are *ambiguous* and are never acted on by persisted state.

## Files
- Settings `$XDG_CONFIG_HOME/aizome/musubi.toml` (view options per page).
- State `$XDG_STATE_HOME/aizome/musubi/` (folded groups, window).
- Data `$XDG_DATA_HOME/aizome/musubi/` (setups, effect chains; later phases).
- One instance: `$XDG_RUNTIME_DIR/aizome-app.musubi.sock`.

## Testing seams
- `Backend` trait → `fake` for Harness tests and goldens (deterministic graphs: studio, laptop, MIDI rig, empty, disconnected).
- `scripts/private-pipewire` runs a private daemon without WirePlumber or host-device monitors. Static null adapters expose Test Speakers (`Audio/Sink`, stereo FL/FR), Test Interface Output (`Audio/Sink`, AUX0–AUX3), Test Interface Input (`Audio/Source`, AUX0–AUX3), and Test Mic (`Audio/Source`, MONO); their playback and source-monitor ports have `format.dsp = "32 bit float mono audio"`. Clients provide stereo playback (`Stream/Output/Audio`), mono capture (`Stream/Input/Audio`), MIDI playback (`Stream/Output/Midi`) and capture (`Stream/Input/Midi`) with `format.dsp = "8 bit raw midi"` on both MIDI ports, and a software-generated `Video/Source` with a `capture_1` port (no `format.dsp` property). Unlinked audio clients have no ports without a session manager; UMP is not represented.
- Live evidence is `tests/fixtures/private.json`, with link-create/remove and isolation transcript in `runs/2026-11-11/patchbay/phase1/s1c/`. This proves only the private PipeWire instance, not hardware.
- VM check: NixOS VM with `snd-virmidi` (MIDI bridge) and `vivid` (video), driving the packaged app with real input; screenshots for `sekka-visual`.
