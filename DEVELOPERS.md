# Developing Musubi

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
