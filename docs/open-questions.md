# Musubi open questions (batched for the user)

Each entry: the question, options (with pictures where they help), what we proceed with meanwhile, and when it must be settled. Settled entries move to `decisions.md`.

| # | Question | Options | Proceeding with | Needed by |
|---|---|---|---|---|
| Q1 | Where the repository is published, and its visibility | `raptros/aizome-musubi` public · private · hand to `bitemyapp` · stay local | local git repo only; nothing pushed | end of phase 1 |
| Q2 | Drag gesture on the matrix | straight line with preview, applied on release (diagonal = 1→1, 2→2, as in AFCC) · paint as you go | built: straight line with preview (`tests/snapshots/audio-drag-preview-night.png`) | phase 1 review |
| Q3 | Accessible alternative to the grid | List view of all routes (filterable table with real controls) · inspector only | built: List view + Add Route sheet (`tests/snapshots/list-filtered-*.png`, `add-route-sheet-night.png`) | phase 1 review |
| Q4 | MIDI activity LEDs in the inspector (needs a MIDI monitor stream on the selected port) | phase 2 · never | left out of phase 1 | phase 2 |
| Q5 | Copyright holder in `LICENSE-MIT` | Aidan Coyne · Chris Allen (if the owner adopts it) · both | Aidan Coyne | before publishing |
| Q6 | Offer Musubi in Sekka's catalog (pinned `flake = false` input + `nix/components.nix` + Genkan catalog entry, like Uta) | later Sekka PR · never | not now; the app's own flake and NixOS module only | after phase 1 |
| Q7 | App icon | an abstract knot (結び) on the 64-grid with Aizome's hard shadow · a patch-grid motif · a cable | built: knot (`data/org.aizome.Musubi.svg`) | phase 1 review |
| Q8 | How an application's streams are named in the matrix | client name (`application.name`, e.g. "pw-cat", "Firefox") with the stream's description as the detail line · the stream's own description ("Musubi_VM_Player") | client name (VM shot `01-audio-before`) | phase 1 review |
| Q9 | Long names in narrow upright column groups ("musubi-vm-…" twice; "Virtual Raw MI…" four times) | keep the end of the name (`…-patch`, `…MIDI 0-0`) · taller column header · show the distinguishing part in the port label | keep the distinguishing end (visual review #1) | before phase 2 |
| Q10 | Merge `phase-1` into `main` now, with the visual polish list (visual review top 8) and three minor code-review items as the first work of the next session · or polish first, then merge | merge after polish | phase 1 review |
