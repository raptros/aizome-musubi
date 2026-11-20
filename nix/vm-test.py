"""Packaged Musubi: compositor input only, live PipeWire observations only."""
import json
import shlex
import time
from pathlib import Path

INSPECT = "/run/user/1000/musubi-inspect"
inputs = []
observations = []


def user(command):
    return machine.succeed("su - alice -c " + shlex.quote(
        "export XDG_RUNTIME_DIR=/run/user/1000; "
        "export WAYLAND_DISPLAY=$(ls /run/user/1000 | grep -m1 '^wayland-[0-9]*$'); "
        "export AIZOME_SOCKET=/run/user/1000/aizome.$WAYLAND_DISPLAY.sock; "
        "export AIZOME_SHELL_SOCKET=/run/user/1000/aizome-shell.$WAYLAND_DISPLAY.sock; "
        + command))


def until(description, check, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        try:
            result = check()
            if result:
                return result
        except (AssertionError, KeyError, StopIteration):
            pass
        time.sleep(0.3)
    diagnose(description)
    raise AssertionError(description)


def diagnose(description):
    # A failed check discards $out, so the evidence for the failure goes to the log.
    try:
        print(f"--- diagnosis for {description!r}")
        print("window:", json.dumps(window()))
        for e in elements():
            print("element:", json.dumps({k: e.get(k) for k in ("name", "text", "role", "label", "value", "rect")}))
        print("inputs:", json.dumps(inputs[-12:]))
        print("links:", user("pw-link -l"))
        print("ports:", json.dumps([(p.get("port.direction"), n.get("node.name"), n.get("node.description"))
                                    for p, n in ports()]))
    except Exception as e:  # diagnosis must never hide the original failure
        print("diagnosis failed:", e)


def window():
    return next((w for w in json.loads(user("aizomectl --json windows"))["windows"]
                 if w["app_id"] == "org.aizome.Musubi"), None)


def elements():
    w = window()
    if w is None:
        return []
    documents = [json.loads(line) for line in machine.succeed(
        f'for f in {INSPECT}/*.json; do if test -f "$f"; then cat "$f"; printf "\\n"; fi; done'
    ).splitlines() if line.strip()]
    return [e for doc in documents if doc.get("kind") == "window"
            and doc.get("pid") == w["pid"] and doc.get("app_id") == w["app_id"]
            for e in doc["elements"] if e["visible"]]


def element(id_):
    return next((e for e in elements() if e.get("name") == id_), None)


def input_(command):
    inputs.append(command)
    user("aizomectl input " + command)
    time.sleep(0.25)


def click(x, y):
    input_(f"motion {x} {y}")
    input_("click left")


def click_element(id_, part=None):
    e = until("inspect element " + id_, lambda: element(id_))
    rect = e["rect"]
    if part is not None:
        p = next(p for p in e["parts"] if p["name"] == part)
        rect = p["rect"]
    x, y, width, height = rect
    g = window()["geometry"]
    click(g["x"] + x + width / 2, g["y"] + y + height / 2)


def filter_(id_, value):
    click_element(id_)
    input_("type " + shlex.quote(value))


def cell():
    # Parts are window-relative rectangles in the inspect JSON.
    e = until("inspected matrix", lambda: element("matrix"))
    body = next(part for part in e["parts"] if part["name"] == "body")
    x, y, width, height = body["rect"]
    assert width > 24 and height > 24, (width, height)
    g = window()["geometry"]
    # Keep clear of the 12pt header-resize hit strip at each body edge.
    return g["x"] + x + 16, g["y"] + y + 16


def ports():
    # The full pw-dump exceeds the test driver's command-output limit.
    dump = json.loads(user(
        "pw-dump | jq -c '[.[] | select(.type == \"PipeWire:Interface:Node\" or .type == \"PipeWire:Interface:Port\")"
        " | {id, type, props: (.info.props | {\"node.id\": .[\"node.id\"],"
        " \"node.name\": .[\"node.name\"], \"node.description\": .[\"node.description\"],"
        " \"port.direction\": .[\"port.direction\"], \"format.dsp\": .[\"format.dsp\"],"
        " \"port.name\": .[\"port.name\"], \"port.alias\": .[\"port.alias\"]})}]'"
    ))
    # jq writes missing keys as null: make every value a string.
    clean = lambda props: {k: (v if isinstance(v, str) else "" if v is None else str(v)) for k, v in props.items()}
    nodes = {str(o["id"]): clean(o["props"]) for o in dump if o["type"].endswith(":Node")}
    return [(dict(clean(o["props"]), id=str(o["id"])), nodes.get(str(o["props"].get("node.id")), {}))
            for o in dump if o["type"].endswith(":Port")]


def linked(output_port, input_port):
    # A Link global with exactly these endpoints, as PipeWire reports it.
    found = user(
        "pw-dump | jq -c '[.[] | select(.type == \"PipeWire:Interface:Link\")"
        f" | select(.info[\"output-port-id\"] == {output_port} and .info[\"input-port-id\"] == {input_port})] | length'")
    return int(found.strip()) > 0


def link_list():
    value = user("pw-link -l")
    observations.append({"links": value})
    return value


def capture(name):
    machine.screenshot(name)
    observations.append({"step": name, "window": window(), "elements": elements()})


machine.start()
machine.wait_until_succeeds("pgrep -u alice -x aizome-shell", timeout=120)
try:
    with subtest("stream FL to second sink FL, then Undo"):
        until("player has managed output ports", lambda: any(
            p.get("port.direction") == "out" and "musubi_vm_player" in n.get("node.name", "")
            for p, n in ports()), seconds=60)
        user(f"aizomectl spawn env AIZOME_KIT_INSPECT={INSPECT} aizome-musubi")
        until("Musubi window", window)
        until("Audio matrix", lambda: element("matrix"))
        capture("01-audio-before")
        click_element("pairs-channels", "Channels")
        # The player's group is its client, which pw-play names "pw-cat"
        # (application.name); "pw-" matches either spelling.
        filter_("from-filter", "pw-")
        filter_("to-filter", "musubi-vm-patch")
        until("filtered Audio matrix", lambda: element("matrix"))
        before = link_list()
        assert "musubi-vm-patch:playback_FL" not in before or "musubi_vm_player:output_FL" not in before, before
        click(*cell())
        until("FL audio patch created", lambda: (
            "musubi_vm_player:output_FL" in link_list() and
            "musubi-vm-patch:playback_FL" in link_list()))
        capture("02-audio-linked")
        input_("chord super+z")
        until("FL audio patch removed on Undo", lambda: (
            "musubi-vm-patch:playback_FL" not in link_list()))
        capture("03-audio-undo")

    with subtest("virtual MIDI output to virtual MIDI input"):
        click_element("MIDI")
        until("MIDI matrix", lambda: element("matrix"))
        capture("04-midi-before")
        # ALSA MIDI reaches PipeWire through the bridge node; snd-virmidi's
        # clients show up in the bridge's port names and aliases.
        midi = [(p, n) for p, n in ports() if "midi" in p.get("format.dsp", "").lower()
                and any("VirMIDI" in p.get(k, "") or "Virtual Raw MIDI" in p.get(k, "")
                        for k in ("port.name", "port.alias"))]
        assert any(p.get("port.direction") == "out" for p, _ in midi), midi
        assert any(p.get("port.direction") == "in" for p, _ in midi), midi
        # Filtered to the virtual clients, the first row is client 0-0's output
        # and the first column client 0-0's input.
        first = lambda direction: next(p["id"] for p, _ in midi if p.get("port.direction") == direction
                                       and any("0-0" in p.get(k, "") for k in ("port.name", "port.alias")))
        source, destination = first("out"), first("in")
        assert not linked(source, destination), (source, destination)
        filter_("from-filter", "Virtual Raw MIDI")
        filter_("to-filter", "Virtual Raw MIDI")
        until("filtered MIDI matrix", lambda: element("matrix"))
        click(*cell())
        until("virmidi 0-0 output linked to 0-0 input", lambda: linked(source, destination))
        capture("05-midi-linked")
    machine.fail("coredumpctl --no-pager list aizome-musubi")
finally:
    machine.screenshot("final")
    Path(machine.out_dir, "inputs.json").write_text(json.dumps(inputs, indent=2))
    Path(machine.out_dir, "observations.json").write_text(json.dumps(observations, indent=2))
    machine.succeed("journalctl --no-pager > /tmp/musubi-vm-journal.log")
    machine.copy_from_machine("/tmp/musubi-vm-journal.log")
    machine.copy_from_machine(INSPECT)
    machine.shutdown()
