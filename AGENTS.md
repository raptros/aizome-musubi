# Working on Musubi

Musubi 結び is Aizome's PipeWire patchbay: a patch matrix, then a mixer, effects and setups (`docs/design.md`). Read `docs/architecture.md` before changing code and keep it true: a change that doesn't fit the architecture changes the document first, in the same commit. Record product and architecture choices in `docs/decisions.md`; questions for the user go in `docs/open-questions.md`.

- **Kit:** Musubi builds against Aizome's kit from a sibling Sekka checkout (`../sekka`, at the revision `flake.nix` pins). Move the flake pin, the sibling checkout and `Cargo.lock` together. Use only the kit's widgets and the theme's colours; Night and Day must both look finished; every target 24pt or more; everything reachable by keyboard and from the menus.
- **PipeWire:** one thread owns every PipeWire object (`src/backend/pipewire.rs`); the UI gets plain data. Never act on a runtime id from an earlier connection; persist only stable identities.
- **Tests:** headless Harness tests against the fake backend, with Day/Night goldens in `tests/snapshots/` (inspect both before blessing them with `AIZOME_UPDATE_GOLDENS=1`). Live tests start their own private PipeWire in a private runtime directory and never touch the session's. Say which level a result is: headless, private PipeWire, VM, real hardware. A lower level never stands in for a higher one.
- **Git:** topic branches, one subject per commit. Commits made by an agent end with `Agent: OMP (<model>) on behalf of Aidan Coyne (@raptros)`.
