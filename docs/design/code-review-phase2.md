# Phase 2 code review (reviewer, openai-codex/gpt-6-sol high, 2026-11-22)

Verbatim; diff `main..phase-2` at `5ef964a`. Blocker 1 is settled by decision A7 plus its fixes (a)–(c); the rest go to P2-S7.

**Verdict: incorrect.** The patch has release-blocking control-state and one-writer failures, plus several mixer and test defects. I found no provable allocation, lock, logging, panic, or buffer overrun in the meter process callback itself.

### Blockers

- **Render only observed Pulse state** — `src/app.rs:1518-1544`. Mixer volume, mute, balance, and default writes use `Audio::snapshot()` as read-back, but the pinned `aizome-sys` service updates that snapshot *before* Pulse confirms a write (`repos/sekka/desktop/aizome/crates/aizome-sys/src/audio.rs:587-611,684-725,1242-1253,1260-1279`). A refused write can therefore appear successful in the UI, contrary to the acceptance requirement. **Fix:** change the service/seam to expose server-observed values separately from pending commands, and render the former.

- **Do not grant native control when the Pulse bridge fails** — `src/mixer/model.rs:134-158`. Native eligibility excludes Pulse nodes only when their serial equals a Pulse index or a strip resolved to them. If a writable Pulse playback node has a different serial from its Pulse index, its Pulse strip is read-only but the same graph node gains a writable native strip. That turns a failed identity bridge into a second writer. **Fix:** require positive evidence that a node is native-only; leave uncertain nodes read-only rather than treating an unmatched index as proof.

### Majors

- **Reject a conflicting serial during name fallback** — `src/mixer/model.rs:49-58`. When no serial equals a device’s Pulse index, `resolve` accepts a name match even if that node *has a different serial*. This contradicts the stated absent-serial-only fallback and enables a Pulse control despite conflicting identities. **Fix:** restrict name fallback to uniquely matching nodes with no serial; reject conflicting serials.

- **Wait for a mixer write to settle before committing undo** — `src/app.rs:1529-1544,752-775,850-855`. Mixer transactions enter history on submission, and replay advances history and reports “applied” immediately after another asynchronous request. Unlike link transactions, neither the original write nor its undo must succeed or be observed. A refused profile, route, or volume write can thus consume undo/redo while leaving the server unchanged. **Fix:** retain pending mixer transactions, observe command outcomes and read-back, and advance history only when the requested state settles.

- **Identify MAIN by the default device’s stable name** — `src/mixer/page.rs:360-362`. MAIN chooses the first output strip whose *description* equals that of any default sink. Two sinks with the same description make MAIN’s fader control whichever strip sorts first, even when the other sink is default. **Fix:** carry the Pulse device name or index in `Strip` and match MAIN to the actual default device.

- **Reconcile meters when Audio wakes the UI** — `src/app.rs:1483`. `AudioWake` redraws the snapshot but never calls `sync_meters`. If a graph node already exists and a later Pulse snapshot makes its strip visible—for example with Native hidden and Playing shown—the strip appears without starting its meter until an unrelated graph or UI change. **Fix:** call `sync_meters(cx)` on Audio wake, retaining the existing unchanged-set check.

### Minors

- **Discard channel-link state with its runtime identity** — `src/app.rs:1970-1979`. Disconnect clears the graph but retains `linked_channels`, which is keyed by Pulse indices and native global IDs. After a server restart reuses an index, a different object inherits the linked indicator and a subsequent volume write may centre its balance. **Fix:** clear link state on disconnect and prune it when its object disappears, or key it by stable identity.

- **Assert the exact VM audio link** — `nix/vm-test.py:200-209`. The audio check searches `pw-link -l` for the source and destination names separately, unlike the exact-endpoint `linked()` check used for MIDI. Two unrelated links containing those names can satisfy “created” without linking the selected ports. **Fix:** resolve the intended FL port IDs and assert `linked(source, destination)` before the click, after it, and after Undo.

**Confidence:** 0.93.