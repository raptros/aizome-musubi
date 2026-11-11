# Phase 1 code review (reviewer, openai-codex/gpt-6-sol high, 2026-11-11)

Verbatim; findings Major 1–6 fixed in `e3efc5a`…`a5284ec` (see `docs/phases/phase-1-log.md`); Minor 1–3 open.

# Musubi phase 1 — independent code review

Scope: `git diff 724cc94..HEAD -- src tests nix flake.nix Cargo.toml scripts data`; design, architecture, plan, fixtures and existing evidence. Read-only review: no tests/builds run. Verdict: **not ready to merge** while major findings remain.

## Blocker
- None established.

## Major
1. **Confirm active links, not just observed Link globals.** `src/backend/pipewire.rs:195-208` records `link.state` then calls `link_seen` regardless of state; `src/matrix/layout.rs:167-191` treats every Link as connected. When PipeWire reports a Link in `Error` or negotiation, the request succeeds and the cell shows connected although the route cannot carry media. Fix: distinguish active, negotiating and failed Link states, surface error reason, and complete a Connect only for an active observed link; test an Error state transition.
2. **Accept real video port format evidence.** `src/graph/legal.rs:20-21,40-46` and `src/matrix/layout.rs:176-184,341-349` require equal `video.format.family` or `format.dsp` strings. The captured video port (`tests/fixtures/private.json:5255-5287`) has `EnumFormat = video/raw` but neither property; two such real ports are visible yet no Video cell can be clicked. The hand-built video fixture supplies artificial properties (`video.json:109-111,145-147`). Fix: preserve comparable capabilities from real EnumFormat PODs, compare compatible families, and test two real-shape video ports.
3. **Do not discard an undo entry before a request settles.** `src/app.rs:370-413` pops history and skips intents without observed results, then `after_undo(None)` loses the entry. A quick Undo while Connect is pending leaves the eventual link connected with no way to undo that transaction. Fix: retain pending entries or queue their inverse until the result is observed; test delayed fake-backend results.
4. **Reconcile matrix focus on layout shrink.** `src/matrix/widget.rs:109-126,134-141,606-626` indexes old focused row/column offsets without checking the new layout length. A filter, collapse, pair toggle or port disappearance while a later cell is focused makes the next arrow-key event panic. Fix: clear/re-resolve focus, hover, press and line indices on layout replacement, ideally by stable port identity; regress focused port removal.
5. **Classify live UMP formats separately from MIDI 1.0.** `src/backend/pipewire.rs:106-112,172-188` calls every Application/Control EnumFormat MIDI; `src/graph/classify.rs:28-47` lets this override media.class when the optional `format.dsp` property is missing. A UMP-only control port without that property appears on the MIDI 1.0 page and can bypass the MIDI-conversion confirmation. `src/graph/dump.rs:25-45` already distinguishes control formats in fixtures. Fix: decode MIDI/UMP control format from live PODs or keep indistinguishable formats unknown; test both.
6. **Assert the intended MIDI route in the VM.** `nix/vm-test.py:169-184` compares the complete `pw-link -l` output before/after clicking. Any unrelated WirePlumber link changing in that interval satisfies the check even when no virtual MIDI source/destination was connected. Fix: identify the selected virmidi output and input global IDs and assert an observed Link global with exactly those endpoints.

## Minor
1. **Persist device folds with device identity.** `src/persistence.rs:66-86` selects an arbitrary HashMap port in a Device group and encodes its node name using `src/graph/identity.rs:65-86`. For a multi-node device, a changed iteration order or profile node replacement prevents its saved fold from restoring. Fix: derive group identity from Device name and bus/serial; use node name only for per-node groups; test a multi-node device across relaunch.
2. **Publish usable matrix Cell semantics.** `src/matrix/widget.rs:699-732` publishes only a Table with a Click action; no Cell node or Toggle action exists. The focused text is useful, but assistive technology cannot discover/activate each cell as required by `docs/design.md` §2.6 and acceptance row 8. Fix: expose focused Cell labels/state and a toggle action through the normal command path; test activation rather than only presence of Table.
3. **Remove test-wide XDG environment races.** `tests/app.rs:493-522` mutates three process-wide env variables while other Harness tests may run concurrently. `nix/package.nix:29-32` serializes Nix checkPhase, not ordinary parallel `cargo test --locked`; the phase log records the earlier race. Fix: isolate the persistence test in a process or inject a settings path without env mutation; maintain documented serialized test invocation until fixed.
4. **Correct the VM video-device claim.** `nix/vm-test.nix:42` requests `vivid`, but passing `runs/2026-11-11/patchbay/phase1/s5b/vm-build-8.log:1882-1884` explicitly says `Failed to find module vivid`; the driver checks no video port. Fix: package an available module and assert it loaded, or remove the claimed device from `DEVELOPERS.md` and the phase plan.

## Nit / documentation accuracy
- `docs/architecture.md` Layers/Threads says a Metadata proxy is bound and monitored; `src/backend/pipewire.rs:116-251` binds only Node/Port/Link/Device/Client, so live `Graph::metadata` remains empty. Describe the actual phase-1 scope instead. `docs/phases/phase-1.md` also names gesture/inspector files that were implemented inside widget/app instead.

## Phase-1 acceptance table
| # | Result | Evidence / gap |
|---|---|---|
| 1 | Met | S1a flake/dev-shell build and version logs; package also built in S5b. |
| 2 | Partly | Fixture tests and page goldens; real video legality and optional-DSP UMP classification incomplete. |
| 3 | Partly | Harness click/drag/Space/pairs, live click and two backend links, VM click; no live drag/keyboard/pair gesture evidence. |
| 4 | Partly | Fake error, illegal-cell and live output→output preflight; live link-state Error incorrectly shown connected. |
| 5 | Partly | Harness undo/redo and missing port, VM audio Undo; pending-result Undo loses history, no live vanished-port undo. |
| 6 | Met | `tests/live.rs:216-279` restarts the private daemon, checks new epoch and refuses stale ID; Harness unavailable state. |
| 7 | Partly | Harness layout, filters, options and fold relaunch; multi-node device folds unstable. |
| 8 | Partly | Keyboard Add Route Harness and List Table semantics; matrix Cell/Toggle semantics absent. |
| 9 | Met at measured level | Exact damage tests 1×/2× and release timing in `docs/performance.md`; numbers cover rasterization rather than complete compositor latency. |
| 10 | Partly | Night/Day S4 goldens and VM images; no sekka-visual approval in reviewed phase log/evidence. |
| 11 | Partly | S5b VM reports both routes and images; MIDI proof not endpoint-specific, `vivid` unavailable. |
| 12 | Partly | README, DEVELOPERS, architecture and decisions present; metadata and VM video statements inaccurate. |

## Risks for phase 2
- Mixer must bridge Pulse identities to current PipeWire nodes without reusing runtime IDs or display names as persistent identities.
- Meter/parameter listeners increase one-thread object lifetime and reconnect demands; test core loss with meters active.
- WirePlumber policy links can race stream moves; distinguish observed policy state from user intent before adding persistent routing.
