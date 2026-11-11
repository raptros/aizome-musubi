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
    raise AssertionError(description)


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
    # Filters below leave exactly one group on each axis; Channels puts FL
    # first. Derive the widget origin from the inspector, and use the same
    # header/cell pitch as PatchMatrix::view and Axis::new.
    x, y, width, height = until("inspected matrix", lambda: element("matrix"))["rect"]
    assert width > 188 + 24 and height > 180 + 24, (width, height)
    g = window()["geometry"]
    return g["x"] + x + 188 + 12, g["y"] + y + 180 + 12


def ports():
    # The full pw-dump exceeds the test driver's command-output limit.
    dump = json.loads(user(
        "pw-dump | jq -c '[.[] | select(.type == \"PipeWire:Interface:Node\" or .type == \"PipeWire:Interface:Port\")"
        " | {id, type, props: (.info.props | {\"node.id\": .[\"node.id\"],"
        " \"node.name\": .[\"node.name\"], \"node.description\": .[\"node.description\"],"
        " \"port.direction\": .[\"port.direction\"], \"format.dsp\": .[\"format.dsp\"]})}]'"
    ))
    nodes = {str(o["id"]): o["props"] for o in dump if o["type"].endswith(":Node")}
    return [(o["props"], nodes.get(str(o["props"].get("node.id")), {}))
            for o in dump if o["type"].endswith(":Port")]


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
        filter_("from-filter", "pw-play")
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
        midi = [(p, n) for p, n in ports() if "midi" in p.get("format.dsp", "").lower()
                and ("Virtual Raw MIDI" in n.get("node.description", "") or "VirMIDI" in n.get("node.name", ""))]
        assert any(p.get("port.direction") == "out" for p, _ in midi), midi
        assert any(p.get("port.direction") == "in" for p, _ in midi), midi
        filter_("from-filter", "Virtual Raw MIDI")
        filter_("to-filter", "Virtual Raw MIDI")
        until("filtered MIDI matrix", lambda: element("matrix"))
        before = link_list()
        click(*cell())
        until("MIDI link added", lambda: link_list() != before)
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
