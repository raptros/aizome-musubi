# Phase 3 — effects (EQ chains), clock & load

Scope: `design.md` §4 (mockups `design/mockups/effects-night.png`, `graph-night.png`); decisions D6 (stereo EQ on a virtual output; one chain per user service; a short dropout of that chain only, on release, if live edits are impossible) and A1–A7. Research: `research/mixer-effects.md` §6–8. Out of scope: input/voice EQ, LADSPA/LV2 hosting, convolution, setups (phase 4).

Done means every acceptance row is shown by its evidence, the code review has no open blocker or major, and `sekka-visual` has passed the Effects and Clock & Load screenshots. Starts after the user's review of phase 2.

## Slices (one worker at a time)

### P3-S1 — spike: filter-chain control and helper lifetime (decides two open points)
In the private PipeWire, with a scratch chain built from two `bq_peaking` builtins per channel:
1. **Live control.** Run `pw-cli enum-params <node> Props` on the chain's nodes. Find the exact `params` POD (`"<node>:<control>"` keys) and the node that accepts it. Write it with pipewire-rs 0.10.1 `set_param(Props)` and read it back. Live edits are proven only when the read-back changes **and** a meter on the chain's output shows the level change for a test tone at the band's frequency. Otherwise D6's restart-on-release path applies. Record the result as decision A8.
2. **Helper lifetime.** Run `pipewire -c <generated filter-chain conf>` as a separate process, the way the user unit will. Show its nodes appear, survive Musubi quitting, and go away when the helper stops. Show that restarting the helper drops only that chain. Measure the dropout length.
3. **Supervision path.** Decide how Musubi starts, stops and enables a per-chain unit:
   - systemd's user manager over D-Bus (`org.freedesktop.systemd1`, `StartUnit`/`StopUnit`/`EnableUnitFiles`, via `zbus`, already in the closure through `aizome-sys`),
   - or a `systemctl --user` subprocess.

   Prefer D-Bus and record the choice as decision A9. The NixOS module ships the template `aizome-musubi-effect@.service`, which runs `pipewire -c %h/.config/aizome-musubi/effects/%i.conf` and has no restart loop (`Restart=on-failure`, `StartLimitBurst=3`).

### P3-S2 — effects model, config, service
- **Config.** `effects::Chain` holds a name, a target (output by stable identity, or "follow default"), up to 8 bands (peaking, low and high shelf: freq, gain, Q), bypass and enabled. Store it under `~/.config/aizome-musubi/effects/` as `<chain>.json`, the source of truth, next to the generated `<chain>.conf`. Both files are written atomically. The conf is generated only from the JSON, with nothing else of the user's in it.
- **Virtual output.** The chain exposes an `Audio/Sink` named `musubi_eq.<chain>` (description "<name> EQ"), and its playback stream targets the chosen output (`target.object`). It never becomes the default output on its own. Choosing it as default goes through the Mixer, with EasyEffects' warning reasoning shown once.
- **Service control** through the A9 path:
  - enable means write files, enable the unit and start it;
  - disable means stop and disable it;
  - edit means a live write (A8) or, on release, a regenerate followed by restarting that unit only.
- **Status** is observed and never assumed: running, the expected nodes present, the target missing, failed (with the unit's last log line), or the start limit hit.
- **Tests:** unit tests for config generation (golden text), JSON round trip and status derivation. A live test in the private PipeWire runs the helper directly, without systemd, and checks the chain's nodes and EQ effect by meter.

### P3-S3 — Effects page
- **Sidebar.** MIXER ▸ Effects. The Mixer's Show… menu gets EFFECTS, and chain sinks appear as an EFFECTS section in the Mixer.
- **Page layout:**
  - **Chain list:** name, status dot and target.
  - **Editor:** name, "Sends to" pop-up, preset (Flat, plus the user's saved presets), and the response curve. The curve spans 20 Hz–20 kHz and ±15 dB, with numbered square handles: drag for frequency and gain, scroll for Q, keyboard for everything.
  - **Band table:** type, frequency, gain and Q, editable as values.
  - **Bypass** and Enable.
  - **Read-only list of external effect nodes**, e.g. EasyEffects, labelled as not managed by Musubi.
- **Status bar** says when an edit restarts the chain ("Applying… the EQ restarts on release").
- **Undo** covers band edits, bypass, target and enable, entered only after the observed result.
- **Tests:** Harness tests on a fake effects service, plus Night/Day goldens for a chain with 5 bands, a failed chain and the empty state.

### P3-S4 — Clock & Load page
- **Sidebar.** SYSTEM ▸ Clock & Load.
- **Clock settings:**
  - **Effective values** (rate, quantum, latency in ms) are shown separately from **forced** values (`clock.force-rate`, `clock.force-quantum` in the `settings` metadata, subject 0).
  - **Writes** go through native metadata: allowed rates and quantum choices come from `clock.allowed-rates` and min/max quantum.
  - **Automatic** clears both force keys **and** Musubi's own drop-in.
  - **"Use at every start"** writes only explicit numbers to `~/.config/pipewire/pipewire.conf.d/90-aizome-musubi-clock.conf`. The page says it takes effect at the next PipeWire start.
  - **Permission errors** show as read-only controls with the reason.
- **Load:** the Profiler is subscribed only while the page shows.
  - DSP load chart (last 60 s).
  - Xrun/error deltas per driver since the page opened, labelled "since opened", never "reset".
  - Driver/follower table: node, rate, quantum, busy/wait, errors.
- **Tests:** unit tests for metadata parse/write, the drop-in text and profiler delta maths. Live: force quantum 256 and back to Automatic, read back from the `settings` metadata; profiler rows appear. Night/Day goldens.

### P3-S5 — VM, docs, reviews
- **VM check:**
  1. Create an EQ chain from the Effects page with real input. The unit runs and `musubi_eq.<chain>` appears.
  2. Route the test player to it.
  3. A band boost at the tone's frequency raises the chain output's meter level.
  4. Disable it: the unit is stopped and the nodes are gone.
  5. Force the quantum to 256 from Clock & Load: `pw-metadata -n settings` shows it. Automatic clears it.
- README, DEVELOPERS and architecture updated. Code review (reviewer), visual review (sekka-visual) and a phase 3 log.

## Acceptance

| # | Behaviour | Evidence |
|---|---|---|
| 1 | A chain is a user service independent of Musubi; it survives quitting Musubi and stops when disabled | private (helper) + VM (systemd) |
| 2 | EQ bands change the audio (meter at the band frequency), live per A8 or by restarting that chain only on release | private + VM |
| 3 | Musubi writes only its own files (`effects/*.json`, `*.conf`, the clock drop-in) and never edits other configuration | unit (paths) + review |
| 4 | Status is observed: running / missing target / failed with reason / start limit | unit + private + VM |
| 5 | Clock: effective vs forced shown separately; force and Automatic round-trip through `settings` metadata; the drop-in contains explicit numbers only | unit + private + VM |
| 6 | Load: profiler only while visible; per-driver deltas since opened | unit + private |
| 7 | Effects and Clock & Load pages: keyboard reachable, semantics, Night/Day goldens | Harness |
| 8 | Docs, code review, visual review | files under `docs/` |
