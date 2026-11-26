# Code review — phase 4 (Setups)

Reviewer: `reviewer` agent (OMP, openai-codex/gpt-6-sol, high), static review of `main..phase-4` at `8ecc904`, 2026-11-25. Recorded by the coordinator (the reviewer is read-only). Coordinator additions are marked (C).

**Verdict: incorrect: 2 blockers, 8 majors, 4 minors.**

## Blockers
1. **Foreign keeper units can be stopped.** `src/effects/service.rs:158-174`, `src/main.rs:54-65`: disable calls `stop`/`disable` without the ownership check used when enabling. Launching the GUI with no active setup, or running cleanup, can stop a same-named unit Musubi doesn't own. Fix: check `owned()` before either operation.
2. **Activation undo is undone by Keep connected.** `src/app.rs:3897-3900`: Undo disconnects activation-created links, then the graph delta runs `enforce_setup`, which recreates them. Fix (C, decision A11): undoing an activation also restores the previous active setup (or none), so enforcement of the undone setup stops before its links go.


## Majors
3. **Deleting stops at a keeper failure.** `src/app.rs:153-160, 2815-2826`: deleting the active setup uses `?` on keeper shutdown, so a systemd failure prevents `store.delete`. Fix: attempt both and aggregate the errors (D22).
Items 1–3 fixed in `99ac44724f` (keeper ownership, activation undo/redo, independent deletion).
4. **Exclusive fights forever at 3 per minute.** `src/setups/reconcile.rs:104-115`: attempts expire after 60 s. Fix: after 3 removals in 60 s, the pair stays conflicted (no more removals, one notice) until the setup is re-activated or Exclusive is toggled; the plan says "then stops".
5. **Exclusive misses half-resolved routes.** `src/setups/mod.rs:105-115`: `named` gains ports only for routes that resolve at both ends, so with one end missing or the pair illegal, the unique end's foreign links stay. Fix: add every uniquely resolved endpoint to scope independently; still exclude ambiguous candidates.
6. **Activation stranded behind an errored link.** `src/app.rs:1073-1082, 2050-2052`: `plan` treats an error-only pair as linkable, but `submit_pairs` sees the link id and skips creation after `create_pending` marked it in flight; nothing clears the mark. Fix: the same non-error-link test in submission; mark pending only for commands actually sent.
7. **CLI remove blocked by a damaged `active.json`.** `src/main.rs:29`: `setups remove <id>` aborts on `store.active()?`. Fix: delete by filename first and report the marker error separately.

Items 4–7 fixed in `fb9e74c4e6` (Exclusive conflict and endpoint scope, errored-link submission, damaged-marker CLI cleanup).
8. **Overlapping activations lose undo.** `src/app.rs:1006-1015, 1082`: activating B before A settles overwrites the single `activation_pending`. Fix: track every pending activation, or refuse a second activation until the first settles (status bar says why).
9. **Confirmed MIDI conversions can't be restored.** `src/setups/mod.rs:97-100`: `NeedsConfirmation` is grouped with `Forbidden`, so a pinned, user-approved MIDI↔MIDI 2.0 route is always Illegal once absent. Fix: persist the approval on the route (`confirmed: bool`, serde default false) and allow it when set.
10. **Retry timer not rescheduled.** `src/app.rs:1023-1027`: an outstanding 30 s timer keeps `setup_retry_scheduled` set after a newer 1 s deadline appears. Fix: generation-tag scheduled retries, or reschedule when the earliest deadline moves earlier.

Items 8–10 fixed by the coordinator (commit after this note): a second activation is refused while one is settling, with a status-bar notice; routes persist `confirmed` from the live link's legality (`Route::for_link`), and `plan` restores approved conversions; `reconcile::retry_timer` replaces a later timer when an earlier deadline appears. Regression tests: `second_activation_waits_for_the_first_to_settle`, `approved_midi_conversion_is_restored_and_unapproved_is_not`, `earlier_retry_deadline_replaces_a_later_timer`.

## Minors
11. **Lock failure exits 0.** `src/main.rs:134-139`: the GUI doesn't open, but `main` returns success. Fix: exit non-zero after the message.
Item 11 fixed in `99ac447`.
12. (C) **The Harness hides Setups from the sidebar to keep old goldens** (`8ecc904`, `src/app.rs` sidebar filter on `setup_store.is_some()`). The real app always shows it, so every other page's golden stops matching the product. Fix: always show it; regenerate the affected goldens.
13. (C) **Route labels are raw identities.** The setups page shows `alsa_card.pci.analog · playback_FL`, while every other page shows `node.description`. Two different routes in the fixture render identically. Fix: store a display label per endpoint when pinning or saving (description plus port), shown when the port is missing; use the live description when present; disambiguate duplicates.
14. (C) Status bar says "1 setups"; the keeper line "Not needed" needs context ("Keeper not needed: Keep connected and Exclusive are off").

Tests cover ordinary activation, a short Exclusive burst and keeper restoration. They don't exercise these failure and boundary paths: each fix needs a regression test.
