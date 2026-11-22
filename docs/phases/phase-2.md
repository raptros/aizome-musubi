# Phase 2 — mixer, meters, devices

Scope: `design.md` §3 (Mixer page, Devices page) and §2.7's port meter in the inspector; decisions A1 (one writer per control: Pulse-visible objects through `aizome-sys::audio` semantics), A2 (one PipeWire thread owns meter streams), A3 (stable identity). Research: `research/mixer-effects.md` §1–5, §8. Out of scope: effects, clock & load (phase 3), setups (phase 4), MIDI activity (D12). Backlog items B1–B8 (`backlog.md`) are fixed when a slice touches their code.

Done means every acceptance row below is shown by its evidence, the code review has no open blocker/major, and `sekka-visual` passed the Mixer and Devices screenshots.

## Slices (one worker at a time)

### P2-S1 — control-path spike (decides A1's shape)
1. **Pulse service (decided, A6).** Depend on `aizome-sys` by path and use `aizome_sys::audio::Audio`; add `aizome-sys` and `aizome-apps` to `flake.nix`'s `aizomeFor` copy list and `libpulseaudio` to the package. Verify `cargo build --locked` and `nix build .#musubi`.
2. **Identity bridge.** Map Pulse objects to graph nodes by `object.serial` (pipewire-pulse puts it in the proplist) and fall back to `node.name`; ambiguous or missing → strip is read-only with the reason. Unit tests on recorded proplists.
3. **Native-only nodes.** For nodes Pulse can't see (JACK clients, filter streams): read `Props` (`enum_params`), offer a fader only when `channelVolumes` is present and writable; write with `set_param(Props)` and treat the read-back value as the truth.
4. Live proof in `scripts/private-pipewire` extended with `pipewire-pulse` (private socket, never the session's): read and set a null sink's volume/mute/balance through the chosen path, observe the change in `pw-dump`. `architecture.md` rows written for the Pulse service and the identity bridge.

### P2-S2 — meter engine
On the PipeWire thread: one capture stream per metered node (`stream.monitor=true` for sinks / `stream.capture.sink`, `node.passive=true`, `node.dont-reconnect=true`, `resample.peaks=true`, low rate, `media.name="Musubi level meter"`, honest `application.id`), peaks per channel written to an atomic snapshot in the RT callback (no allocation, no locks); the UI samples at 30 Hz only while a meter is visible; `StartMeters(nodes)`/`StopMeters` commands; cap 24 (over the cap: "meter paused"); everything stops when the page or window hides and on disconnect. Musubi's own meter streams are hidden from every page and the mixer. Tests: fake meter source in the fake backend; live: a private `pw-cat` tone shows a non-zero peak, silence reads −∞, the meter stream disappears on stop.

### P2-S3 — widgets
App-local `Fader` (vertical, cubic law, dB scale +6…−60/−∞, value box with double-click to type, keyboard ↑↓/Page/Home/End, 24pt target, semantics as Slider with value text) and `LevelMeter` (segmented per channel, green / amber > −9 dB / red > 0 dB, 1.5 s peak hold, semantics as a meter). Harness tests for pointer/keys mapping and value text; goldens Night/Day.

### P2-S4 — Mixer page
Sidebar gains MIXER ▸ Mixer. Sections OUTPUTS · INPUTS · PLAYING · RECORDING (· EFFECTS stays empty until phase 3: the section is omitted, not shown empty), MAIN strip pinned right (default output; its pop-up sets the default). Strip per design §3 (tile, name, subtitle per D16, route/target pop-up, balance, M, ◉ default, channel link, Fader, LevelMeter, value box), widths 84–120pt so nothing clips. Toolbar: dB/% , Meters toggle, Show… popover (which strips), filter. Moving a stream through its pop-up uses the Pulse path (`move_sink_input`/`move_source_output`); WirePlumber's own re-targeting is shown as observed. Undo covers volume/mute/route changes as transactions (A3). Tests: Harness on the fake backend (volume/mute/balance/move commands, read-only strips with reasons, keyboard), goldens Night/Day (populated, filtered, narrow window), live: change a private null sink's volume from the UI and read it back.

### P2-S5 — Devices page + inspector meter
Sidebar SYSTEM ▸ Devices (Tansu's layout): device list (icon, name, bus, active profile, status dot) and details (profile pop-up incl. Pro Audio when offered, routes per direction with availability, the device's nodes with channel counts); writes through the Pulse path (`set_card_profile`, port) so WirePlumber's restore keeps them. Profile switches that replace nodes keep the matrix and mixer consistent (epoch-safe, stable identity). Inspector (§2.7) gains the selected audio port's meter (one stream while the inspector shows). Tests + goldens; live where the private PipeWire allows (null devices have no profiles: say so; the VM covers it).

Also in P2-S5, from `docs/design/visual-review-phase2-preview.md`:
- **Blockers:**
  - Device strips get the route pop-up (ports/routes with availability, through the Pulse path) instead of the "Device route" placeholder.
  - Default device: ◉ accent-filled when default (both themes), semantics Toggle checked. The same goes for M on (red as in the mockup) and channel link on.
- **Majors:**
  - Mute and link on-states captured.
  - Meters off: keep the channel columns, dim them and add a readable "Meters off" overlay; label paused meters; MAIN's meter keeps its channel count.
  - Sidebar Mixer row: inset rounded selection like the matrix rows, a strip count, its own icon.
  - Night disabled fader visibly dimmed.
- **Minors:**
  - Icon tiles instead of OUT/IN/PLAY tags.
  - Device subtitles: description/route, not raw node names.
  - Balance L/R labels.
  - Channel labels under the value boxes.
  - Meters/Show… icons and a title count; the Show… EFFECTS option waits for phase 3.
  - Scale: show −60 with −∞ on its own row.
  - % mode uses a % scale.
  - Align stream and device rows.
  - Scroll indicator.
  - Per-control status hover.

### P2-S6 — VM, docs, reviews
VM check extended: Mixer page lowers the test sink's volume with real input and `wpctl get-volume` confirms; the player's strip meter moves; Devices page lists the virtual devices. README/DEVELOPERS/architecture updated; code review (reviewer) and visual review (sekka-visual); backlog items fixed on the way are struck through.

## Acceptance

| # | Behaviour | Evidence |
|---|---|---|
| 1 | Volume, mute, balance of devices and streams change PipeWire state through one writer, and the UI shows the read-back value | unit + private PipeWire (+pulse) live; VM `runs/2026-11-21/patchbay/phase2/s6/evidence/{observations.json,inputs.json,06-mixer-live-meter.png}`: two real Page Down presses on the default output fader changed `wpctl get-volume @DEFAULT_AUDIO_SINK@` from 1.00 → 0.79 → 0.63 (−6 dB each); mute/balance and streams retain their private-live/Harness evidence, not an additional VM assertion |
| 2 | Pulse ↔ graph identity is verified; ambiguous strips are read-only with a reason | unit (recorded proplists) |
| 3 | Native-only nodes get a fader only when writable `Props` exist | unit + live |
| 4 | Meters show real peaks, cost nothing when hidden, never appear as recording apps | live + Harness + `pw-dump` after hide |
| 5 | Default output/input and stream moves work and survive WirePlumber's policy | live + VM `runs/2026-11-21/patchbay/phase2/s6/evidence/observations.json`: the VM explicitly selected the null sink as default with `wpctl`, then proved that the Mixer fader writes and reads back through WirePlumber; default-changing and stream-moving UI actions are **not** asserted by this VM check |
| 6 | Profiles/routes change through the Pulse path; node replacement keeps pages consistent | VM `runs/2026-11-21/patchbay/phase2/s6/evidence/{07-devices-before.png,08-devices-profile-switched.png,observations.json,inputs.json}`: the `snd-dummy` card lists five profiles; Tab/Return on its real popover selected `output:stereo-fallback` from `output:stereo-fallback+input:stereo-fallback`; `pactl list cards` confirmed Active Profile and the Devices button reflected read-back. Route writes and node replacement retain earlier live/Harness evidence, not a VM assertion |
| 7 | Mixer and Devices pages: keyboard reachable, semantics, Night/Day goldens | Harness |
| 8 | Docs, code review, visual review | files under `docs/` |
