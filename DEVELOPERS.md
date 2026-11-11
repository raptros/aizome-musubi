# Developing Musubi

## Testing against a private PipeWire

From the workspace root, with `repos/sekka` as the sibling checkout:

```sh
tools/heavy musubi-private -- tools/nix-env nix shell --inputs-from ./repos/sekka nixpkgs#pipewire nixpkgs#jq -c repos/aizome-musubi/scripts/private-pipewire -- sh -c 'pw-link -o; pw-link -i; pw-dump'
```

`private-pipewire [--keep] -- command…` starts its own daemon with private HOME, XDG runtime/config/state/data and PipeWire socket, and an unreachable D-Bus address. It does not start WirePlumber, access host devices or use the user's PipeWire connection. It installs four null audio nodes (Test Speakers stereo, Test Interface Output and Input four-channel AUX, Test Mic mono), sets DSP port configurations explicitly because there is no session manager, and launches playback/capture clients plus a MIDI capture client. All links must be made manually with `pw-link`. On exit, its child processes are reaped and its temporary directory removed; `--keep` retains only files/logs for diagnosis, not processes. The command's exit status is preserved. Do not use `pw-cli -r pipewire-0` outside this script or point it at the host runtime directory.

Evidence and fixture generation: while inside the wrapper, run `pw-dump > repos/aizome-musubi/tests/fixtures/private.json` and perform a manual `pw-link` create/list/delete/list. Retain transcripts under `runs/`, not inside the repository. This is a private PipeWire component check, not a VM or physical-hardware test.
