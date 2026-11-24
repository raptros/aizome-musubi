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


def popover_elements():
    w = window()
    documents = [json.loads(line) for line in machine.succeed(
        f'for f in {INSPECT}/*.json; do if test -f "$f"; then cat "$f"; printf "\\n"; fi; done'
    ).splitlines() if line.strip()]
    return [e for doc in documents if doc.get("kind") == "popover"
            and doc.get("pid") == w["pid"] and doc.get("app_id") == w["app_id"]
            for e in doc["elements"] if e["visible"]]

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
    return g["x"] + x + 12, g["y"] + y + 12


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

def default_volume():
    return float(user("wpctl get-volume @DEFAULT_AUDIO_SINK@").split("Volume: ", 1)[1].split()[0])


def meter_nodes():
    return json.loads(user(
        "pw-dump | jq -c '[.[] | select(.type == \"PipeWire:Interface:Node\") "
        "| select(.info.props[\"media.name\"] == \"Musubi level meter\" "
        "and .info.props[\"application.id\"] == \"org.aizome.Musubi\") | .id]'"))


def sink_meter(fader):
    x, y, width, height = fader["rect"]
    return next((e for e in elements() if e["role"] == "meter" and e["label"] == "Level peak"
                 and e["rect"][1] >= y
                 and e["rect"][1] < y + height and x + width <= e["rect"][0]
                 and e["rect"][0] < x + width + 48), None)


def cards():
    return json.loads(user(
        "pactl -f json list cards | jq -c '[.[] | {index, name, description: .properties[\"device.description\"],"
        " active_profile: (if (.active_profile | type) == \"object\" then .active_profile.name else .active_profile end),"
        " profiles: (.profiles | if type == \"array\" then map({name, available})"
        " else to_entries | map({name: .key, available: .value.available}) end)}]'"))


machine.start()
machine.wait_until_succeeds("pgrep -u alice -x aizome-shell", timeout=120)
default_sink = until("null sink appears", lambda: user(
    "pw-dump | jq -r '[.[] | select(.type == \"PipeWire:Interface:Node\" "
    "and .info.props[\"node.name\"] == \"musubi-vm-default\")] | first | .id // empty'").strip())
user("wpctl set-default " + default_sink)
observations.append({"default_sink": default_sink, "wpctl": user("wpctl status")})
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
        audio_ports = until("FL audio endpoint IDs", lambda: (
            pair if len(pair[0]) == 1 and len(pair[1]) == 1 else None
        ) if (pair := (
            [p["id"] for p, n in ports() if n.get("node.name") == "musubi_vm_player"
             and p.get("port.direction") == "out" and p.get("port.name") == "output_FL"],
            [p["id"] for p, n in ports() if n.get("node.name") == "musubi-vm-patch"
             and p.get("port.direction") == "in" and p.get("port.name") == "playback_FL"],
        )) else None)
        source, destination = audio_ports[0][0], audio_ports[1][0]
        assert not linked(source, destination), (source, destination)
        click(*cell())
        until("FL audio patch created", lambda: linked(source, destination))
        capture("02-audio-linked")
        input_("chord super+z")
        until("FL audio patch removed on Undo", lambda: not linked(source, destination))
        capture("03-audio-undo")

    with subtest("virtual MIDI output to virtual MIDI input"):
        click_element("MIDI")
        until("MIDI matrix", lambda: element("matrix"))
        capture("04-midi-before")
        # ALSA MIDI reaches PipeWire through the bridge node; wait for the
        # 0-0 client's input and output before fixing their PipeWire IDs.
        midi = until("VirMIDI 0-0 input and output ports", lambda: (
            found if all(any(
                p.get("port.direction") == direction
                and any("0-0" in p.get(k, "") for k in ("port.name", "port.alias"))
                for p, _ in found) for direction in ("out", "in")) else None
        ) if (found := [(p, n) for p, n in ports() if "midi" in p.get("format.dsp", "").lower()
                       and any("VirMIDI" in p.get(k, "") or "Virtual Raw MIDI" in p.get(k, "")
                               for k in ("port.name", "port.alias"))]) else None, seconds=60)
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

    with subtest("Mixer fader writes via WirePlumber and live sink meter"):
        click_element("Mixer")
        # The default output's strip: its fader id carries the Pulse index,
        # which pipewire-pulse sets to the node's object.serial.
        dump = json.loads(user("pw-dump"))
        default_name = user("pactl get-default-sink").strip()
        default_serial = next(
            o["info"]["props"]["object.serial"] for o in dump
            if o.get("type") == "PipeWire:Interface:Node"
            and o["info"]["props"].get("node.name") == default_name)
        fader = until("default output fader", lambda: element(f"fader-Output-{default_serial}"))
        assert not fader.get("disabled"), fader
        initial = default_volume()
        x, y, width, height = fader["rect"]
        geometry = window()["geometry"]
        click(geometry["x"] + x + width / 2, geometry["y"] + y + height - 16)
        input_("key 109")  # KEY_PAGEDOWN (evdev; the compositor's named keys omit it)
        first = until("first 6 dB fader decrement", lambda: (
            value if 0.75 < value / initial < 0.83 else None
        ) if (value := default_volume()) else None)
        input_("key 109")
        second = until("second 6 dB fader decrement", lambda: (
            value if 0.59 < value / initial < 0.67 else None
        ) if (value := default_volume()) else None)
        assert second < first < initial, (initial, first, second)
        observations.append({"wpctl_volume": [initial, first, second], "fader": fader})
        until("meter capture nodes active", meter_nodes)
        meter = until("default sink meter has a nonzero peak", lambda: (
            e if e and e["value"] not in ("−∞ dB", "paused", "Meters off") else None
        ) if (e := sink_meter(fader)) else None, seconds=45)
        observations.append({"sink_meter": meter, "meter_nodes": meter_nodes()})
        capture("06-mixer-live-meter")
        click_element("Devices")
        until("meter nodes removed on hide", lambda: not meter_nodes())
        observations.append({"meter_nodes_after_hide": meter_nodes()})

    with subtest("Devices lists ALSA card and switches a real offered profile"):
        available_cards = until("Pulse sees VM ALSA card", cards, seconds=60)
        listed = until("VM card in Devices page", lambda: [
            card for card in available_cards if element(f"device-{card['index']}")
        ])
        observations.append({"cards_before": available_cards, "pactl_before": user("pactl list cards")})
        card = listed[0]
        capture("07-devices-before")
        profiles = [profile for profile in card["profiles"]
                    if isinstance(profile["name"], str) and profile["name"]
                    and profile["name"] != card["active_profile"] and profile["name"] != "off"
                    and profile["available"] not in ("no", False)]
        if profiles:
            before_label = element("device-profile")["text"]
            click_element("device-profile")
            target = "profile-" + profiles[0]["name"]
            choices = until("profile popover has offered entry", lambda: (
                entries if any(e.get("name") == target for e in entries) else None
            ) if (entries := popover_elements()) else None)
            # The kit renders popovers in a separate Wayland popup; use its
            # own keyboard focus rather than coordinates from the main window.
            for _ in range(len(choices) + 2):
                if any(e.get("name") == target and e.get("focused") for e in popover_elements()):
                    break
                input_("key tab")
            else:
                raise AssertionError(("profile not keyboard reachable", target, popover_elements()))
            input_("key return")
            switched = until("Pulse and WirePlumber observe selected profile", lambda: next(
                (c for c in cards() if c["name"] == card["name"] and c["active_profile"] == profiles[0]["name"]),
                None), seconds=45)
            assert f"Active Profile: {profiles[0]['name']}" in user("pactl list cards")
            until("Devices profile button reflects read-back", lambda: (
                button if button.get("text") != before_label else None
            ) if (button := element("device-profile")) else None)
            observations.append({"profile_selected": profiles[0]["name"], "cards_after": switched,
                                 "pactl_after": user("pactl list cards"), "wpctl_after": user("wpctl status")})
            capture("08-devices-profile-switched")
        else:
            observations.append({"profile_switch": "No alternate available non-off profile on VM card"})
    machine.fail("coredumpctl --no-pager list aizome-musubi")
finally:
    machine.screenshot("final")
    Path(machine.out_dir, "inputs.json").write_text(json.dumps(inputs, indent=2))
    Path(machine.out_dir, "observations.json").write_text(json.dumps(observations, indent=2))
    machine.succeed("journalctl --no-pager > /tmp/musubi-vm-journal.log")
    machine.copy_from_machine("/tmp/musubi-vm-journal.log")
    machine.copy_from_machine(INSPECT)
    machine.shutdown()
