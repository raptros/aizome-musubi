# Phase 4 — Setups (keep connected, exclusive)

Scope: `design.md` §5; decisions D5 (opt-in per setup; a small user service keeps them while Musubi is closed; Exclusive with a warning), D22 (cleanup is guaranteed: the same rules apply to setup files and the keeper unit), D24–D26 (below), A1–A9. Research: `research/pipewire-graph.md` §3, §5 (identity, WirePlumber policy).

## Semantics

**Setup.** A named list of routes. Each route is a pair of endpoints by *stable identity*, never by global id:
- **hardware:** device name, plus bus path or serial when present, node name, direction, port name or channel;
- **app:** `application.name`, plus `media.name` or `node.name`, plus port or channel;
- **MIDI:** sequencer client name plus port name.

Stored as `$XDG_DATA_HOME/aizome/musubi/setups/<id>.json`, written atomically with 0600 permissions, never through a symlink.

**Matching.** A route endpoint matches exactly one live port, or it is:
- *missing*: nothing matches; the route waits;
- *ambiguous*: several match; the route is listed and never acted on, and the UI says which candidates.

**One active setup (D24).** At most one setup is active. Activating a setup deactivates the previous one first, then creates every missing matched link of the new one in one undoable transaction. The previous setup's links stay; only its enforcement stops. Deactivating does nothing to the graph: it only stops Keep connected and Exclusive. Undo still reverses the activation's links. The active setup's id is stored in `$XDG_DATA_HOME/aizome/musubi/active.json`, so it survives a restart and the keeper reads it.

**Keep connected (opt-in per setup).** While a setup is active with Keep connected, a matched route whose link disappears is re-created when both endpoints are present. That covers a device replugged, an app restarted, or WirePlumber moving a stream.
- Musubi does this while it runs, and `aizome-musubi --keeper` does it while Musubi is closed (Keep connected and Exclusive alike). The keeper is a user unit, `aizome-musubi-keeper.service`, enabled only while the active setup has Keep connected or Exclusive on, through A9's D-Bus path.
- The keeper and the app never act on the same setup at once: the app takes over while it runs, through a lock in `$XDG_RUNTIME_DIR`.

**Exclusive (D25, opt-in, with a warning).** While the setup is active, links that touch a port the setup names but aren't in the setup are removed. Links on other ports are never touched. WirePlumber may re-create its automatic links. Musubi removes them again at most 3 times per minute per link, then stops, and the status bar explains WirePlumber's policy rather than fighting it.

**Pin / unpin (D26).** Changing routes while a setup is active doesn't change the setup. "Pin" adds the selected live link to a setup; "unpin" removes a route from it. There is no recording mode.

**Cleanup (D22).**
- Deleting a setup removes its file (and `active.json` if it was active), and disables the keeper unit if no longer needed.
- `aizome-musubi setups list|remove <id>|remove-all` follows the `effects` CLI: remove-all deletes every setup file and `active.json` and disables the keeper, continuing past failures and keeping foreign files.
- README recovery commands are updated to match.

## Slices
1. **P4-S1: model and matching.** Identity extraction for every endpoint kind; a match engine with missing/ambiguous results; setup JSON and paths. Unit tests on fixture graphs: replug changes global ids, two identical devices make a route ambiguous, an app restart gives new ids.
2. **P4-S2: activation, keep connected, exclusive in the app.** The backend transaction path, the rate limit, undo. Private live tests: kill and restart a `pw-cat` stream and see the route re-linked; under Exclusive, add a stray link and see it removed.
3. **P4-S3: the keeper.** `--keeper` mode (no UI; the same backend thread and match engine); the unit template in the NixOS module; handover lock; enable/disable through the effects thread. VM: with Musubi closed, the keeper restores a link after the player restarts.
4. **P4-S4: Setups page.**
   - Sidebar PATCHBAY ▸ Setups. A setup list with active/kept/exclusive state, and route rows with matched/missing/ambiguous status and candidates.
   - New from the current page's links; pin/unpin from the matrix's context menu and the inspector.
   - Night/Day goldens.
5. **P4-S5: VM, docs, code review, visual review, merge.**

## Acceptance

| # | Behaviour | Evidence |
|---|---|---|
| 1 | Routes match by stable identity across replug and restart; ambiguous routes are never acted on | unit |
| 2 | Activation is one undoable transaction | Harness + live |
| 3 | Keep connected restores links while Musubi runs and while it's closed (keeper) | live + VM |
| 4 | Exclusive removes foreign links within scope, rate-limited against WirePlumber | live + VM |
| 5 | Cleanup leaves no setup files or keeper enablement | unit + VM |
| 6 | Setups page: keyboard, semantics, goldens | Harness |
| 7 | Docs, code review, visual review | `docs/` |

## User answers (2026-11-25)
Q1 several active setups → one at a time (D24); Q2 Exclusive scope → only the setup's ports (D25); Q3 changes while active → explicit pin/unpin only (D26).

