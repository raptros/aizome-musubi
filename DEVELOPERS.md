# Developing Musubi

## Checkout and pins

Keep `aizome-musubi/` next to `sekka/`; Cargo resolves `aizome-kit` by
`../sekka/desktop/aizome/crates/aizome-kit`, including Sekka's local Sukumo
crates. This is a separate Cargo package/workspace, not a member of Sekka.
The flake pins a Sekka revision and copies only the kit's needed Aizome and
Sukumo paths into the Nix build. Develop with the sibling checkout on that
same revision, inside Sekka's `nix develop .#aizome` shell. Never edit the
Sekka checkout as part of a Musubi package update.

When advancing the kit, coordinate three things in one change:

1. Select the new `inputs.sekka.url` revision in `flake.nix` and use an
   adjacent Sekka checkout at exactly that revision.
2. In Sekka's Aizome dev shell, run `cargo update --workspace` in Musubi,
   then `cargo test --locked`; inspect and commit
   Musubi's `Cargo.lock`. Cargo does not inherit Sekka's workspace patches
   across the package boundary. If the kit requires another local crate,
   add its path to `aizomeFor` in `flake.nix`.
3. Update Musubi's `flake.lock`, build `.#musubi` with
   `--no-update-lock-file`, and commit source, Cargo lock and flake lock
   together. Do not advertise a new pin based on a mismatched local kit.

In the Sekka contribution workspace, wrap **all** Cargo tests/builds and
Nix work with `tools/heavy` and use `tools/nix-env` for Nix; build/test
artifacts belong under `runs/`, never in source control.

## Tests and evidence

With `../sekka` beside this checkout, from `../sekka`:

```sh
nix build ../aizome-musubi#test-fonts --out-link /tmp/aizome-fonts
nix build ../aizome-musubi#test-fonts-conf --out-link /tmp/aizome-fonts.conf
nix develop .#aizome -c bash -c 'cd ../aizome-musubi && cargo fmt && AIZOME_FONT_DIR=/tmp/aizome-fonts FONTCONFIG_FILE=/tmp/aizome-fonts.conf cargo test --locked'
```

(In the Sekka contribution workspace the same command runs under `tools/heavy … tools/nix-env`.) Use Sekka's pinned Aizome fonts rather than
an arbitrary system font. `tests/app.rs` exercises the app in the kit's
headless Harness with fixture graph data; `tests/matrix.rs` checks layout,
input and exact redraws; graph/backend unit tests use `tests/fixtures/`.
`tests/snapshots/` holds the Night and Day Harness goldens. Run the tests
without `AIZOME_UPDATE_GOLDENS` normally; when deliberately changing visuals,
set `AIZOME_UPDATE_GOLDENS=1`, inspect both themes at original resolution,
and commit the intended golden changes. `docs/screenshots/` is a copy of
selected fixture goldens for the README, not live-session evidence.

The ignored release timing test is
`cargo test --release --locked --test matrix matrix_frames -- --ignored --nocapture --test-threads=1`;
record host/load and frame measurements in `docs/performance.md`. The
measurement times Harness rasterization, not end-to-end compositor latency.
Unit/Harness tests prove fixture/UI behaviour; private PipeWire tests prove
live daemon links on isolated ports; a VM check proves packaged app and
system-session interaction; neither of these certifies physical hardware.
Do not infer VM or hardware behaviour from a green Harness test.

## Testing against a private PipeWire

From the workspace root, with `repos/sekka` as the sibling checkout:

```sh
tools/heavy musubi-private -- sh -c '
  base=$(tools/nix-env nix eval --raw --inputs-from ./repos/sekka nixpkgs#gst_all_1.gst-plugins-base.outPath) || exit
  GST_PLUGIN_PATH="$base/lib/gstreamer-1.0" tools/nix-env nix shell --inputs-from ./repos/sekka \
    nixpkgs#pipewire nixpkgs#gst_all_1.gstreamer nixpkgs#gst_all_1.gst-plugins-base nixpkgs#jq \
    -c repos/aizome-musubi/scripts/private-pipewire -- sh -c "pw-link -o; pw-link -i; pw-dump"
'
```

`private-pipewire [--keep] -- command…` starts its own daemon with private HOME, XDG runtime/config/state/data and PipeWire socket, and an unreachable D-Bus address. It does not start WirePlumber, access host devices or use the user's PipeWire connection. Four null audio nodes provide Test Speakers (`Audio/Sink`, FL/FR), Test Interface Output (`Audio/Sink`, AUX0–AUX3), Test Interface Input (`Audio/Source`, AUX0–AUX3), and Test Mic (`Audio/Source`, MONO); all their playback and monitor ports have `format.dsp = "32 bit float mono audio"`. It configures these ports explicitly without a session manager. Client nodes supply stereo playback (`Stream/Output/Audio`), mono capture (`Stream/Input/Audio`), MIDI playback (`Stream/Output/Midi`) and capture (`Stream/Input/Midi`), both MIDI ports with `format.dsp = "8 bit raw midi"`, and software-generated video (`Video/Source`, `capture_1`, no `format.dsp` property). Unlinked audio clients have no ports without a session manager; UMP is not represented. The GStreamer base plugins must be on `GST_PLUGIN_PATH` for `videotestsrc`; the script adds the pinned PipeWire plugin for `pipewiresink`. All links must be made manually with `pw-link`. On exit, its child processes are reaped and its temporary directory removed; `--keep` retains only files/logs for diagnosis, not processes. The command's exit status is preserved. Do not use `pw-cli -r pipewire-0` outside this script or point it at the host runtime directory.

Live tests (`tests/live.rs`, gated by `MUSUBI_LIVE=1`) link the null sinks' monitor outputs, since unlinked streams have no ports here; streams and WirePlumber's own links are exercised in the VM check. From the Sekka contribution workspace: `tools/heavy musubi-live -- bash runs/2026-11-11/patchbay/phase1/live.sh` (Sekka's `.#aizome` dev shell, the pinned PipeWire/GStreamer tools on `PATH`, this script around `cargo test --test live`).

Evidence and fixture generation: while inside the wrapper, run `pw-dump > repos/aizome-musubi/tests/fixtures/private.json` and perform a manual `pw-link` create/list/delete/list. Retain transcripts under `runs/`, not inside the repository. This is a private PipeWire component check, not a VM or physical-hardware test.

## VM check

From the contribution workspace root, commit source first (Nix flakes build
the committed tree), then run:

```sh
tools/heavy musubi-vm -- tools/nix-env nix build ./repos/aizome-musubi#checks.x86_64-linux.vm -L --max-jobs 1 --cores 4 --out-link runs/2026-11-11/patchbay/phase1/s7/vm
```

`nix/vm-test.nix` imports the pinned Sekka Aizome NixOS module and this
repository's Musubi module. The disposable 1280×800 guest autologs in to
`aizome-session` with WirePlumber, two virtual audio sinks, a continuously
playing `pw-play` stream and `snd-virmidi`. The Python driver
spawns the packaged app, obtains widget rectangles through
`AIZOME_KIT_INSPECT`, and performs pointer and keyboard gestures with
`aizomectl input`. It selects Channels and filters to the player and
second sink, clicks the FL cell, checks the live `pw-link -l` state,
presses Super+Z and checks removal. It then clicks MIDI, filters to the
virtual clients, clicks the first cell and checks that a Link global joins
exactly virmidi 0-0's output port to its input port. The test copies its
screenshots, input/observation JSON, inspector JSON and guest journal to the
Nix result; copy the images to `runs/` for visual review. A VM check does not
certify hardware MIDI, camera or physical audio devices, and it has no video
device: the test kernel lacks `vivid`, so Video routing is covered only by
fixtures and the private PipeWire's software video source.

A failing run keeps no result, so the driver prints its evidence (window,
inspected elements, recent inputs, links, ports) to the build log before it
fails. Under portable Nix the result lives inside its namespace; copy it out
with `tools/nix-env nix shell --inputs-from ./repos/sekka nixpkgs#bashInteractive
-c cp -rL <out-link>/. <dir>/`. The S7 evidence directory is
`runs/2026-11-11/patchbay/phase1/s7/evidence/` (five scenario screenshots
plus a final screenshot, inputs, observations, inspect JSON, and guest journal).
