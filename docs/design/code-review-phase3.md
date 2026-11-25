# Phase 3 code review (reviewer, openai-codex/gpt-6-sol high, 2026-11-25)

Verbatim; diff `main..phase-3` at `bada5e4`. Fixes go to P3-S5a; D22 (cleanup) blockers first.

**Verdict: incorrect.** The patch does not yet meet D22’s cleanup condition, and several live-control paths can report success or retain pending work without observing the result.

### Blockers

- **Keep writes and cleanup inside owned paths** — `src/effects/paths.rs:36-42`. `ensure_effects` accepts a symlink because it canonicalizes the directory and checks only `is_dir()`. If `effects` points elsewhere, `save` writes there and `remove_all` can delete matching files there before `remove_dir` fails. **Fix:** reject symlinked managed directories before writing or deleting, and apply the same path-boundary rule to the clock drop-in’s parent.

- **Finish cleanup when a chain definition is corrupt** — `src/effects/service.rs:236-255`. `delete` stops and disables the unit, then calls `load` and `save` before removing either file. An unreadable or malformed JSON file makes deletion fail with both files still present; `remove_all` then abandons later chains and the clock drop-in. **Fix:** make deletion independent of parsing or rewriting JSON; remove the two managed files after unit cleanup, and continue processing other resources while reporting errors.

- **Do not abort all recovery on the first cleanup error** — `src/effects/service.rs:312-328`. An unexpected directory entry aborts `remove_all` *before any unit is stopped*. A stop, disable, or file-removal error in `delete` likewise prevents later units and the clock drop-in from being processed. **Fix:** attempt cleanup of every discovered managed resource, preserve unrelated entries, and return an aggregate failure afterward.

### Majors

- **Wait for stop completion before claiming deletion** — `src/effects/service.rs:75-82,247-255`. `StopUnit` returns a queued job path, not confirmation that the service stopped. `delete` immediately disables the unit and removes its files, so it can return successfully while the helper is still running or its stop job subsequently fails. **Fix:** observe the stop job’s result and inactive unit state before declaring cleanup complete.

- **Verify ownership before removing name-matched units** — `src/effects/service.rs:291-299`. `remove_all` stops and disables every listed unit with the prefix and a valid-looking instance ID, including a user-provided unit of that name with no Musubi chain files. **Fix:** verify the unit’s managed template or an ownership marker while retaining a way to identify genuine orphaned Musubi instances.

- **Use the same config location in the unit and writer** — `flake.nix:64-72`. `Paths::from_environment` writes under `XDG_CONFIG_HOME`, but `ExecStart` always reads `%h/.config/aizome-musubi/effects/%i.conf`. With a nondefault XDG config home, enabling a chain starts a helper that cannot read its generated config. **Fix:** give the unit the same resolved config path as the writer.

- **Move systemd queries off the UI thread** — `src/app.rs:719-746`. `refresh_effect_statuses` makes blocking D-Bus property calls for every chain and is invoked on graph deltas. A slow user manager therefore stalls rendering and input, with cost increasing per chain. **Fix:** perform unit queries asynchronously and deliver their observations to `update`.

- **Observe restarted EQ nodes before committing undo** — `src/app.rs:749-761`. For bypass, target, and topology edits, the history gate checks that the new JSON is saved, the unit is active, and nodes *with the expected names* exist. Those can all describe the old helper while its stop/start jobs are pending; the edit enters undo history without observing the replacement chain or its target. **Fix:** wait for the stop/start jobs and identify the replacement nodes and their applied configuration before settling the edit.

- **Fail effect writes that receive no read-back** — `src/backend/pipewire.rs:883-901`. `SetEffectParams` emits `CommandResult(Ok)` as soon as `set_param` is submitted. Unlike clock writes, it has no timeout or correlated server-error handling. If PipeWire rejects or does not report the parameter, `effect_pending` remains indefinitely and subsequent band edits only queue. **Fix:** track the request through a matching read-back or a bounded failure path.

- **Discard pending effect edits on disconnect** — `src/app.rs:2909-2921`. The connection-down branch clears the graph but not `effect_pending` or `effect_queued`. After reconnect, the old request may never produce a read-back, leaving new edits queued indefinitely; a later same-named node can also be compared against the stale request. **Fix:** fail and clear pending effect work on epoch loss, then resolve a fresh handle for a new edit.

- **Clear the drop-in only after Automatic succeeds** — `src/app.rs:2122-2131`. `ClockAutomatic` removes the persistent file even when `apply_clock` returns early because another write is pending, or when the metadata command later fails. The UI has then changed persistence without clearing the forced runtime keys. **Fix:** associate drop-in removal with successful read-back of both Automatic values; report a failure without deleting it.

- **Reset a start-limited unit when Retry is pressed** — `src/app.rs:2222-2231`. Retry calls only `StartUnit`. For a unit shown as `StartLimit`, systemd continues rejecting starts until its failed/start-limit state is reset. **Fix:** call `ResetFailedUnit` for that state before starting, then observe the resulting job.

### Minors

- **Restrict permissions on generated files** — `src/effects/paths.rs:62-68`. `create_new` uses the process umask, so JSON containing chain names and output identities, the helper config, and the clock drop-in can be created world-readable under a permissive home/config directory. **Fix:** create managed files with mode `0600` and managed directories with restrictive permissions.

- **Make manual recovery preserve unrelated files** — `README.md:69-75`. The documented `rm -rf` deletes every entry under the effects directory, whereas `remove_all` refuses unexpected entries; the commands therefore do not perform the “same recovery” and can delete files Musubi did not create. **Fix:** document removal of managed chain files and temporary files only, then remove the directory if empty. The quoted unit glob and `xargs -r` already avoid shell expansion and an empty-list invocation.

- **Describe the shipped pages accurately** — `README.md:61`. The Limits section says there is no Effects page or clock controls, but this patch adds both. **Fix:** update that status text.

The private live-test launcher isolates its PipeWire runtime and config, and the VM exercises guest resources rather than the host. I did not run checks; this was a read-only review. **Confidence: 0.91.**