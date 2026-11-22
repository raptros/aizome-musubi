# Phase 2 visual review (sekka-visual, anthropic/claude-opus-5-5, 2026-11-21)

Verbatim. Goldens at aizome-musubi `5c0459a`; VM evidence `runs/2026-11-21/patchbay/phase2/s6/evidence/`. No blockers; majors go to P2-S7 polish.

## Images
- `repos/aizome-musubi/docs/design/mockups/mixer-night.png` (opened for reference, 1280×800): the mockup has icon tiles, pop-ups with chevrons, L…R balance, M / ◉ / chain-link buttons, a +6…−60 scale, two-column meters, value boxes and channel labels. The muted M is red and MAIN's ◉ is filled with the accent. Defects: n/a (reference).
- `tests/snapshots/audio-studio-night.png` and `audio-studio-inspector-night.png` (opened for comparison): on the matrix pages the selected sidebar row is an inset rounded blue fill with its count outside the fill. The older inspector LEVEL meter is an empty thin column. Defects: n/a (reference).
- `tests/snapshots/mixer-page-night.png` (1280×824):
  - **Chrome:** the sidebar has PATCHBAY (Audio/MIDI/MIDI 2.0/Video, all 0) and MIXER › Mixer. The Mixer row is an inset rounded blue fill with a sliders icon and the count "8" outside the fill, matching the matrix pages. There is no SYSTEM/Devices row. The toolbar reads "Mixer · 8 strips", [dB|%], an accent-filled "Meters" with a pulse icon, "+ Show…", a greyed "Undo" with icon, and "Filter strips". The status bar reads "8 strips · default output Studio Output".
  - **Device strips** (Speakers, Studio Output, Microphone, Studio Input): each has an icon tile, a name, and the subtitle "Speakers" on all four. The pop-up is filled with bold centred "Speakers ⌄", and all four read "Speakers", including both input devices. Balance has L/R labels. Buttons are M / ◉ / ↔. Studio Output's and Studio Input's ◉ are filled with the accent (white glyph). Speakers' and Microphone's ◉ are unfilled but slightly lighter than the M button.
  - **Stream strips** (Firefox Video/Music, Spotify Playlist) have an outlined pop-up with a thin chevron at its right ("Studio Output ⌄", "Speakers ⌄") and M only.
  - **MAIN** reads "Studio Output" with a pop-up, balance and M.
  - **Scale and meters:** the scale reads +6, 0, −6, −12, −24, −36, −60, −∞. Value boxes read "−5.8 dB" / "−9.3 dB", with "FL·FR" below. Studio Output and MAIN show two-column green/amber meters. RECORDING ("Dis…/Call/St…") is cut by the pinned MAIN strip.
  - Defects:
    - The input devices show the route "Speakers", and every device subtitle is "Speakers".
    - The device pop-up style (filled, bold, centred) differs from the stream pop-up style (outlined, right chevron).
    - "−∞" sits about 4 px above the value box.
    - On stream strips the M row, scale and meter sit about 3–4 px higher than on device strips.
    - No horizontal-scroll indicator is visible.
    - The non-default ◉ has a slightly different fill from M/↔.
- `mixer-page-day.png` (1280×824): Day version of the above. The panels are pale lavender. The default ◉ is filled blue with a white glyph, and the other ◉ are white. Meters are dark green and brown-amber. Text and labels are dark and legible. Defects: the same as Night.
- `mixer-page-narrow-night.png` (900×560): the title is gone and Undo shows only its icon. Four device strips are visible. Firefox ("Fire…", "Vide…", "Stu…") is cut by MAIN, which is the allowed scroll-cut. The scale is compressed to about 18 px steps (+6…−60, −∞) and readable. Defects: "−∞" sits about 4–5 px above the value box. The fixture-route issue above also appears here.
- `mixer-page-narrow-day.png` (900×560): Day version of the above. Legible. Defects: as narrow Night.
- `mixer-muted-linked-night.png` (1280×824): Microphone's M is filled pink-red with a dark "M". Speakers' ↔ is accent-filled. Everything else is as in page-night, and the meters are empty. Defects: ↔ is still an arrow glyph, not the mockup's chain-link icon.
- `mixer-muted-linked-day.png` (1280×824): the muted M is filled crimson with a dark "M" (readable). ↔ on is filled blue with a white glyph. Defects: the same glyph note as Night.
- `mixer-device-routes-night.png` (1280×824):
  - The Speakers route pop-up is open below its button with a drop shadow. It lists "Speakers" (dark fill), "Headphones" (lighter fill) and "HDMI · unavailable" (dimmed). Each item is drawn as a bordered box. The pop-up covers Speakers' balance and buttons.
  - Defects:
    - Menu items look like stacked buttons. The current item is marked only by a darker fill, with no check mark, and that fill could read as hover.
    - Available entries carry no availability text.
- `mixer-meters-off-night.png` (1280×824):
  - "Meters" is no longer accent-filled (plain icon and text). Every meter keeps its two columns (no mono-looking collapse) and carries a two-line "Meters / off" label over its top segments.
  - Defects:
    - The label text is tiny (about 6–7 px cap height), overlaps the top segments, and "Meters" spills past the meter's width.
    - Below the label, the meter looks the same as an idle meter.
- `mixer-meter-paused-night.png` (250×414): widget harness. A "paused" label sits at the meter's top-left over the empty segments. The scale runs +6…−60 and −∞, with value box "0.0 dB". Defects:
  - "paused" is about 7 px tall and spills slightly past the meter's right edge.
  - The harness menu is clipped ("Hel").
- `mixer-meter-paused-day.png` (250×414): the same in Day. Defects: the dark "paused" text sits on a segment boundary, and the segment edges cross the text. It is readable only with effort. Harness menu clipped.
- `mixer-percent-night.png` (1280×824): "%" is selected. The fader scale is now in % (150, 100, 50, 25, 10, 0%), and the value boxes read 80% / 70%. Meters are off. Defects: percent mode is only shown with meters off.
- `mixer-toolbar-show-night.png` (1280×824): the filter holds "Firefox" with a clear button. The Show… popover lists Outputs, Inputs, Playing, Recording and Monitors/virtual devices, all checked. Two Firefox strips and MAIN are shown, and MAIN now has a two-column meter. Status reads "6 strips hidden". Defects: no EFFECTS option (phase 3).
- `mixer-widgets-night-rest.png` (250×414): blue fader line, thumb at 0, empty two-column meter, "0.0 dB". The −60/−∞ spacing is now about 25 px. Defects: harness menu clipped ("Hel").
- `mixer-widgets-night-active.png` (250×414): thumb at −6, "−6.0 dB". The meter goes green, then amber, with a red top segment in the right column, all distinguishable. Defects: harness menu clipped.
- `mixer-widgets-night-disabled.png` (250×414): the fader line is grey instead of blue, and the thumb, labels and value text are dimmed. It clearly differs from rest. Defects: harness menu clipped.
- `mixer-widgets-day-rest.png` (250×414): blue line, white thumb, dark labels, lavender empty meter. Defects: harness menu clipped.
- `mixer-widgets-day-active.png` (250×414): dark green, brown-amber and red, all distinguishable. Defects: harness menu clipped.
- `mixer-widgets-day-disabled.png` (250×414): pale line, grey thumb, dimmed text. Clearly differs from rest. Defects: harness menu clipped.
- `devices-page-night.png` (1280×824):
  - **Chrome:** the sidebar has SYSTEM › Devices selected (inset fill, count 3) and Mixer 16. The toolbar reads "Devices · 3 cards" and Undo. Status reads "3 devices".
  - **List pane:** three full-width bordered boxes with centred icon + "●" + name (USB Studio, Built-in Audio, Bluetooth Headset), each with a left-aligned subtitle outside and below the box. USB Studio's box has a slightly lighter border.
  - **Detail pane:** "USB Studio", "usb · alsa_card.usb.studio", then:
    - PROFILE: "Analog Stereo Duplex ⌄".
    - OUTPUT ROUTES: "USB Studio Output · ○ unknown", then Output route "Speakers ⌄".
    - INPUT ROUTES: "USB Studio Input · ○ unknown", then Input route "Mic input ⌄".
    - NODES: "USB Studio Input" and "USB Studio Output", each "4 channels" with [Show in Mixer] [Show in Audio] at the far right.
  - Defects:
    - The list reads as stacked push buttons, not a list.
    - The selection is shown only by a slightly different border.
    - The identical white "●" on every item has no visible meaning.
    - Section captions are indented about 11 px from their content.
    - The node buttons are about 20 px tall and about 430 px from their row names.
    - Title "cards", sidebar count and status "devices" use mixed terms.
- `devices-page-day.png` (1280×824): Day version. The list pane is darker lavender and the detail pane is near-white. Text is dark and legible. Defects:
  - As in Night. USB Studio's selection is only a darker border, which is very weak.
- `devices-profiles-night.png` (1280×824): the profile pop-up is open. It lists "Analog Stereo Duplex · 1 out / 1 in" (dark fill), "Pro Audio · 1 out / 1 in" and "HDMI (unplugged) · 1 out / 0 in · unavailable" (dimmed). Defects:
  - The items are stacked button boxes, and the current item is marked only by fill, with no check mark.
  - Only the unavailable entry states availability.
- `devices-profiles-day.png` (1280×824): Day version. The current item has a light-grey fill and the unavailable entry is grey and readable. Defects: as in Night.
- `devices-routes-night.png` (1280×824): the Output route pop-up is open with "Speakers" (dark fill), "Headphones" and "HDMI unplugged · unplugged" (dimmed). It covers the "USB Studio Input · ○ u…" line and the Input route button, which is expected. Defects:
  - The same item styling as the profile pop-up, with no check mark.
  - "unplugged · unplugged" is redundant.
- `devices-routes-day.png` (1280×824): Day version. Legible. Defects: as in Night.
- `inspector-meter-night.png` (1280×824):
  - **Page:** the Audio matrix in Channels mode. The inspector shows "AUX0", "Audio · USB Studio", GOES TO "Discord · MONO" [Unlink], then a LEVEL meter and FACTS (Node … Serial 10100).
  - **Meter:** one column about 10 px wide and 100 px tall, made of about 2 px segments. It is green up to near the top, with amber and red at the top.
  - Defects:
    - The meter is a sliver in a 260 pt pane, with no scale, no dB readout and no peak value. The segments are hair-thin.
- `inspector-meter-day.png` (1280×824): Day version. The meter is the same sliver, dark green with amber at the top on light grey. Defects: as in Night, and the thin segments are harder to read on light grey.
- `runs/2026-11-21/patchbay/phase2/s6/evidence/06-mixer-live-meter.png` (1280×800, Aizome session):
  - **Session:** the Toolchest menu bar shows "Musubi" menus, workspaces 1–4, a tray and 02:51. The right side panel shows Annai, Musubi — Patchbay and SHELF. Musubi is a tiled window about 1044 px wide with an orange title tab.
  - **Toolbar:** "Mixer · 5", [dB|%], accent-filled Meters, Show…, and an Undo icon.
  - **Strips:**
    - "Built-in Audio Ste…" (Analog Output, pop-up "Analog Output ⌄").
    - "musubi-vm-default" (pop-up "No route ⌄", dimmed, ◉ accent).
    - "musubi-vm-patch" ("No route ⌄").
    - "Built-in Audio Ste…" (Microphone, "Microphone ⌄", ◉ accent).
    - "pw-play" (subtitle "-", pop-up "musubi-vm…").
    - MAIN ("musubi-vm-default", pop-up "musubi-vm…").
  - **Live meters:** musubi-vm-default, pw-play and MAIN show lit green two-column meters with peak-hold segments around −12. A "−12.0 dB" drag readout floats above MAIN's fader, and the cursor covers MAIN's value box.
  - **Status bar:** "MAIN · volume — adjust with arrows or drag" and "5 strips · default output musubi-vm-default".
  - Defects:
    - The pop-ups on pw-play and MAIN are cut to "musubi-vm…", so the distinguishing part (-default vs -patch) is hidden.
    - Both device names are cut to "Built-in Audio Ste…".
    - pw-play's subtitle is a bare "-".
    - The title drops its noun ("Mixer · 5").
    - The cursor partly covers a value box.
- `07-devices-before.png` (1280×800): same session. The toolbar shows "Devices · 1" and Undo. The list has one item, "Built-in Audio", with the subtitle "device · Stereo Output + Stereo Input". The detail pane shows:
  - Profile: "Stereo Output + Stereo Input ⌄".
  - Output routes: "Built-in Audio Stereo · ○ suspended", Output route "Analog Output ⌄".
  - Input routes: "Built-in Audio Stereo · ○ suspended", Input route "Microphone ⌄".
  - Nodes: two rows, both "Built-in Audio Stereo · 2 channels", each with Show in Mixer / Show in Audio.

  The sidebar shows Mixer 5 and Devices 1, and the status bar reads "1 devices". Defects:
  - The two node rows are indistinguishable, with no direction shown.
  - "1 devices".
- `08-devices-profile-switched.png` (1280×800): the profile now reads "Stereo Output ⌄", with the cursor on its chevron. The list subtitle reads "device · Stereo Output". The INPUT ROUTES section is gone, one node row remains, and the sidebar shows Mixer 4. Defects:
  - The cursor partly covers the chevron.
  - "1 devices".

## Checks
- **1. Preview blockers/majors resolved: fail** (one item only partly resolved).
  - **Device route pop-ups: pass.** Every device strip has a pop-up with a chevron, and the open menu lists routes with "HDMI · unavailable" dimmed.
  - **Default identifiable from ◉: pass.** The default ◉ is filled with the accent and has a white glyph in both themes. The others are unfilled. The status bar agrees.
  - **M on-state: pass.** It is red-filled in both themes.
  - **Link on-state: pass.** It is accent-filled, though the glyph is still ↔.
  - **Meters-off and paused: fail (partly resolved).**
    - The toolbar Meters on/off cue is now clear, and meters keep their per-channel columns.
    - But the "Meters off" and "paused" labels are still about 6–7 px tall and spill past the meter.
    - In Day, the segment edges cross the "paused" text.
  - **Sidebar row style, count and icon: pass.** The Mixer row has an inset rounded fill like the matrix pages, the count is outside the fill, and it has its own sliders icon.
  - **Night disabled fader: pass.** The line is grey instead of blue and the text is dimmed, clearly different from rest.
- **2. Strip text, pop-ups with chevrons, value boxes and scale labels readable and unclipped: fail.**
  - Goldens:
    - All fully visible strips are readable and unclipped in both themes and at narrow width. Both pop-up styles show a chevron.
    - Scale labels are separate and clear of the value box ("−∞" about 4 px above it).
  - In the VM capture:
    - The pw-play and MAIN pop-ups are cut to "musubi-vm…", so the target can't be told apart (default vs patch).
    - Both device names are cut to "Built-in Audio Ste…".
- **3. Devices page: fail.**
  - **List:** the list items are bordered, centred push-button boxes with subtitles outside them, so it doesn't read as a list. The selected item differs only by its border colour, which is very weak in both themes.
  - **Pop-ups:** they show unavailability ("unavailable", "unplugged") on dimmed entries. Available entries carry no state, and the current entry has no check mark.
  - **Node rows:** they are clear in the goldens, but in the VM capture both rows read "Built-in Audio Stereo · 2 channels".
  - **Look:** palette and fonts are Aizome, not Arturia, but the list and menu styling don't match the kit's sidebar and table selection look.
- **4. Inspector meter legible and fits: fail.**
  - It fits the inspector with no clipping.
  - It is a single column about 10×100 px with 2 px segments and no scale or readout. Only "high, amber/red near the top" can be read, in both themes.
- **5. VM screenshots match the goldens and show a live meter and the profile switch: pass.**
  - The VM captures match the goldens: Night theme, Archivo UI text, Plex Mono numerals and status bar, the same strip anatomy, scale, sidebar and toolbar.
  - Lit meters with peak-hold segments show on musubi-vm-default, pw-play and MAIN.
  - In 07→08 the profile changes from "Stereo Output + Stereo Input" to "Stereo Output". The input routes and one node disappear, the list subtitle updates, and the Mixer count goes from 5 to 4.
- **6. Day contrast adequate everywhere: pass.**
  - Text, scale labels, value boxes, ◉/↔ accent fills, the crimson muted M, meter colours, dimmed unavailable entries and pop-ups are all legible.
  - Exceptions, ranked minor below: the Day "paused" label crossed by segment edges, the weak Day device-list selection, and the thin Day inspector meter.

## Unreadable / uncertain
- Whether all four device strips in the mixer goldens showing subtitle/route "Speakers" (input devices included) is fixture data or wrong rendering. The VM shows correct per-device routes.
- The mixer goldens' sidebar has no SYSTEM/Devices row, while the devices goldens and the VM have one. This may be a fixture difference.
- Whether meter segments line up with the fader's dB ticks. The meter has no scale of its own and extends above +6 and below −∞, so the amber threshold position can't be checked.
- Horizontal scrollbar: still none visible in any capture.
- Percent mode with live meters: not captured.

## Ranked defects
**Blocker**
- None.

**Major**
1. The VM stream and MAIN target pop-ups are cut to "musubi-vm…", hiding which output they target (default vs patch). The device names are cut too.
2. The Devices list is stacked push-button boxes, with the selection shown only by a border change, which is very weak in Day.
3. The inspector LEVEL meter is a 10 px sliver with hair-thin segments and no scale or readout.
4. The "Meters off" and "paused" labels are about 6–7 px tall and spill past the meter. In Day the segment edges cross "paused". Below the label, an off or paused meter looks like an idle one.
5. In the mixer goldens, input devices (Microphone, Studio Input) show the route "Speakers", and every device subtitle is "Speakers". The golden misleads about state; verify the fixture.
6. VM Devices: the two node rows are identical ("Built-in Audio Stereo · 2 channels") with no direction.

**Minor**
7. Pop-up menus (routes and profiles) draw items as bordered buttons. The current item is marked only by fill, with no check mark, and available items carry no state. "HDMI unplugged · unplugged" is redundant.
8. Two pop-up styles on one page: device/profile pop-ups are filled, bold and centred; stream pop-ups are outlined with a thin chevron at the right.
9. The link button uses ↔ instead of the mockup's chain-link icon. The non-default ◉ has a different fill/border from M and ↔.
10. "−∞" sits about 4–5 px above the value box. On stream strips the M row, scale and meter sit about 3–4 px higher than on device strips.
11. No horizontal-scroll affordance is visible.
12. Devices page:
    - Section captions are indented about 11 px from their content.
    - "Show in…" buttons are about 20 px tall and far from the node names.
    - The meaning of the "●" in list items is unclear.
    - "○ unknown/suspended" sits next to node names without explanation.
13. Wording:
    - Titles drop their noun at the VM width ("Mixer · 5", "Devices · 1").
    - The status bar reads "1 devices".
    - The title says "cards", the status bar says "devices".
    - pw-play's subtitle is a bare "-".
14. The mixer goldens' sidebar has no Devices row (fixture?). Percent mode is only shown with meters off. The Show… popover has no EFFECTS option (phase 3).
15. The widget harness menu is clipped ("Hel"). In the VM captures the cursor covers MAIN's value box and the profile chevron.