# Phase 2 — mixer, meters, devices

Scope: `design.md` §3 (Mixer page, Devices page) and §2.7's port meter in the inspector; decisions A1 (one writer per control: Pulse-visible objects through `aizome-sys::audio` semantics), A2 (one PipeWire thread owns meter streams), A3 (stable identity). Research: `research/mixer-effects.md` §1–5, §8. Out of scope: effects, clock & load (phase 3), setups (phase 4), MIDI activity (D12). Backlog items B1–B8 (`backlog.md`) are fixed when a slice touches their code.

Done means every acceptance row below is shown by its evidence, the code review has no open blocker/major, and `sekka-visual` passed the Mixer and Devices screenshots.

## Slices (one worker at a time)

### P2-S1 — control-path spike (decides A1's shape)
1. **Pulse service.** Check `aizome-sys`'s dependency closure (`repos/sekka/desktop/aizome/crates/aizome-sys/Cargo.toml`, features). If `aizome_sys::audio::Audio` can be depended on by path with only the audio service compiled (feature-gated or small enough), use it; extend `flake.nix`'s `aizomeFor` copy list accordingly. If its closure drags in unrelated services (BlueZ, NetworkManager, CUPS…), write `src/pulse.rs` on `libpulse-binding` with the **same semantics** (cubic volume, balance via channel map, mute, ports, profiles, default sink/source, move stream) — the exact versions Sekka's lock uses — and record the choice as decision A6 with the measured closure.
2. **Identity bridge.** Map Pulse objects to graph nodes by `object.serial` (pipewire-pulse puts it in the proplist) and fall back to `node.name`; ambiguous or missing → strip is read-only with the reason. Unit tests on recorded proplists.
3. **Native-only nodes.** For nodes Pulse can't see (JACK clients, filter streams): read `Props` (`enum_params`), offer a fader only when `channelVolumes` is present and writable; write with `set_param(Props)` and treat the read-back value as the truth.
4. Live proof in `scripts/private-pipewire` extended with `pipewire-pulse` (private socket, never the session's): read and set a null sink's volume/mute/balance through the chosen path, observe the change in `pw-dump`. Decision A6 and `architecture.md` rows written.

### P2-S2 — meter engine
On the PipeWire thread: one capture stream per metered node (`stream.monitor=true` for sinks / `stream.capture.sink`, `node.passive=true`, `node.dont-reconnect=true`, `resample.peaks=true`, low rate, `media.name="Musubi level meter"`, honest `application.id`), peaks per channel written to an atomic snapshot in the RT callback (no allocation, no locks); the UI samples at 30 Hz only while a meter is visible; `StartMeters(nodes)`/`StopMeters` commands; cap 24 (over the cap: "meter paused"); everything stops when the page or window hides and on disconnect. Musubi's own meter streams are hidden from every page and the mixer. Tests: fake meter source in the fake backend; live: a private `pw-cat` tone shows a non-zero peak, silence reads −∞, the meter stream disappears on stop.

### P2-S3 — widgets
App-local `Fader` (vertical, cubic law, dB scale +6…−60/−∞, value box with double-click to type, keyboard ↑↓/Page/Home/End, 24pt target, semantics as Slider with value text) and `LevelMeter` (segmented per channel, green / amber > −9 dB / red > 0 dB, 1.5 s peak hold, semantics as a meter). Harness tests for pointer/keys mapping and value text; goldens Night/Day.

### P2-S4 — Mixer page
Sidebar gains MIXER ▸ Mixer. Sections OUTPUTS · INPUTS · PLAYING · RECORDING (· EFFECTS stays empty until phase 3: the section is omitted, not shown empty), MAIN strip pinned right (default output; its pop-up sets the default). Strip per design §3 (tile, name, subtitle per D16, route/target pop-up, balance, M, ◉ default, channel link, Fader, LevelMeter, value box), widths 84–120pt so nothing clips. Toolbar: dB/% , Meters toggle, Show… popover (which strips), filter. Moving a stream through its pop-up uses the Pulse path (`move_sink_input`/`move_source_output`); WirePlumber's own re-targeting is shown as observed. Undo covers volume/mute/route changes as transactions (A3). Tests: Harness on the fake backend (volume/mute/balance/move commands, read-only strips with reasons, keyboard), goldens Night/Day (populated, filtered, narrow window), live: change a private null sink's volume from the UI and read it back.

### P2-S5 — Devices page + inspector meter
Sidebar SYSTEM ▸ Devices (Tansu's layout): device list (icon, name, bus, active profile, status dot) and details (profile pop-up incl. Pro Audio when offered, routes per direction with availability, the device's nodes with channel counts); writes through the Pulse path (`set_card_profile`, port) so WirePlumber's restore keeps them. Profile switches that replace nodes keep the matrix and mixer consistent (epoch-safe, stable identity). Inspector (§2.7) gains the selected audio port's meter (one stream while the inspector shows). Tests + goldens; live where the private PipeWire allows (null devices have no profiles: say so; the VM covers it).

### P2-S6 — VM, docs, reviews
VM check extended: Mixer page lowers the test sink's volume with real input and `wpctl get-volume` confirms; the player's strip meter moves; Devices page lists the virtual devices. README/DEVELOPERS/architecture updated; code review (reviewer) and visual review (sekka-visual); backlog items fixed on the way are struck through.

## Acceptance

| # | Behaviour | Evidence |
|---|---|---|
| 1 | Volume, mute, balance of devices and streams change PipeWire state through one writer, and the UI shows the read-back value | unit + private PipeWire (+pulse) live + VM |
| 2 | Pulse ↔ graph identity is verified; ambiguous strips are read-only with a reason | unit (recorded proplists) |
| 3 | Native-only nodes get a fader only when writable `Props` exist | unit + live |
| 4 | Meters show real peaks, cost nothing when hidden, never appear as recording apps | live + Harness + `pw-dump` after hide |
| 5 | Default output/input and stream moves work and survive WirePlumber's policy | live + VM |
| 6 | Profiles/routes change through the Pulse path; node replacement keeps pages consistent | VM |
| 7 | Mixer and Devices pages: keyboard reachable, semantics, Night/Day goldens | Harness |
| 8 | Docs, code review, visual review | files under `docs/` |
