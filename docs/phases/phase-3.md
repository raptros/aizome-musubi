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

**P3-S1 spike result (2026-11-22).** The private helper's capture node enumerates a
second `Props` object with `params` as a struct of alternating string control names
and float values. Writing `left:Gain` and `right:Gain` to +12 through pipewire-rs
reads back +12 and raises the 1000 Hz output meter 11.95 dB, without a restart
(A8). The helper survives backend disconnect/reconnect; killing it removes its
nodes, and kill-to-reappearance measured 158 ms. A9 selects the user systemd
D-Bus API; the VM subtest exercises the template and `StartUnit`/`StopUnit`.
Evidence: `runs/2026-11-22/patchbay/phase3/s1/{live,vm}.log`, with the
reproducible config in `tests/fixtures/effects-spike.conf`.

### P3-S2 — effects model, config, service
- **Config.** `effects::Chain` holds a name, a target (output by stable identity, or "follow default"), up to 8 bands (peaking, low and high shelf: freq, gain, Q), bypass and enabled. Store it under `~/.config/aizome-musubi/effects/` as `<chain>.json`, the source of truth, next to the generated `<chain>.conf`. Both files are written atomically. The conf is generated only from the JSON, with nothing else of the user's in it.
- **Virtual output.** The chain exposes an `Audio/Sink` named `musubi_eq.<chain>` (description "<name> EQ"), and its playback stream targets the chosen output (`target.object`). It never becomes the default output on its own. Choosing it as default goes through the Mixer, with EasyEffects' warning reasoning shown once.
- **Service control** through the A9 path:
  - enable means write files, enable the unit and start it;
  - disable means stop and disable it;
  - edit means a live write (A8) or, on release, a regenerate followed by restarting that unit only.
- **Status** is observed and never assumed: running, the expected nodes present, the target missing, failed (with the unit's last log line), or the start limit hit.
- **Tests:** unit tests for config generation (golden text), JSON round trip and status derivation. A live test in the private PipeWire runs the helper directly, without systemd, and checks the chain's nodes and EQ effect by meter.

**P3-S2 implementation (2026-11-24).** `effects::Chain` stores up to eight stereo
peaking/shelf bands in atomic JSON, and generates the filter-chain conf in the
same directory. `effects::service` uses A9's user-manager D-Bus API; its
`edit_band` sends keyed A8 `Props.params` through the existing backend and
`save_observed_edit` persists only after the capture node's Props read-back.
The headless VM exercises the app's code path; its 440 Hz output meter rose
0.0800 → 0.3184 (+12.00 dB) after a live Props edit. The template is
enableable (`WantedBy=default.target`), and the VM checks both unit states,
nodes and file removal. The Effects page remains P3-S3. D22 cleanup: `remove`
stops/disables before removing both files; `remove-all` discovers enabled units
and local chain files, removes the effects directory and only Musubi's clock
drop-in. The documented `effects list | remove | remove-all` recovery commands
work without a GUI; README also includes the three-command manual recovery.
Evidence: `runs/2026-11-24/patchbay/phase3/s2/{unit-full,live,vm-recovery}.log`.

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

### P3-S4b — Effects polish (from `docs/design/visual-review-phase3-effects-preview.md`)
The blocker (Bypass state) and the wording ("1 chains", "Studio EQ EQ chain") are fixed in `03585b9`. Majors:
- **Chain list rows** as on Devices: icon, name, subtitle inside a filled selection; a status dot whose colour *and* text differ by state.
- **Failed chains:**
  - a human reason (from the unit's `Result`, e.g. "exit-code" → "PipeWire filter-chain exited with an error"), plus the last journal line if cheap;
  - a Retry button.
- **Enable vs state:** Enable shows the user's intent; the state text shows what's running. A failed chain stays enabled.
- **Pop-ups:** Sends to and Preset get chevrons. Preset reads "Flat" or the preset's name, not "Flat / saved".
- **Response curve:** a smooth line (path through the samples, 2 px, anti-aliased) with axis units (dB, Hz).

Minors:
- **Band table:**
  - right-aligned numbers, consistent decimals and "+" gains;
  - the selected band highlighted in the table and named in the status bar;
  - a per-band On toggle and remove control.
- **Labels:** clearer wording ("Effects menu" → "⋯" with a tooltip). Consistent naming: New EQ / Delete EQ… / Delete EQ.
- **Empty state:** "No EQ chains yet".
- **Delete dialog:** paths on their own lines, never broken mid-token; a destructive-styled button.
- **Day:** a stronger checkbox outline.

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
