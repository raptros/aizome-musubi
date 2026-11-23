# Phase 2 visual confirmation (sekka-visual, anthropic/claude-opus-5-5, 2026-11-22)

Verbatim. Goldens at aizome-musubi `788e9b9`. All six majors pass; one new blocker (inspector meter scale) sent back to P2-S7b.

## Images
- `repos/aizome-musubi/tests/snapshots/mixer-long-targets-night.png` (1280×824): Mixer with 8 strips.
  - **Device strips:**
    - "musubi-vm-default" and "musubi-vm-patch" (subtitle Speakers, pop-up "Speakers ⌄").
    - "Built-in …eo Input" and "Built-in …crophone" (subtitle Mic input, pop-up "Mic input ⌄"). Their names are cut in the middle, so the endings that tell them apart stay visible.
  - **Stream pop-ups:** Firefox Video "…vm-patch ⌄", Firefox Music "…-default ⌄", Spotify "…vm-patch ⌄". MAIN reads "…-default ⌄" with the subtitle "musubi-vm-default".
  - **Meters:** on, but all empty.
  - **Status bar:** "8 strips · default output musubi-vm-default".
  - Defects:
    - The pop-ups drop the shared "musubi-" prefix, and the cut length varies ("…vm-patch" vs "…-default").
    - The RECORDING strip is cut by the pinned MAIN strip (allowed scroll-cut).
- `…/mixer-long-targets-day.png` (1280×824): Day version of the above. Text is dark and legible, and the cut labels read the same. Defects: as in Night.
- `…/devices-page-night.png` (1280×824):
  - **Chrome:** the title reads "Devices · 3 devices". Sidebar: Devices is selected, count 3. Status bar: "3 devices".
  - **List:** flat rows. The selected "USB Studio" row has a full accent-blue fill with white text. Built-in Audio and Bluetooth Headset have no fill. Each row has a subtitle below it ("usb · Analog Stereo Duplex", …) and a "●" at the right.
  - **Detail pane:** PROFILE, OUTPUT ROUTES, INPUT ROUTES and NODES sections. The node rows read "Input · USB Studio Input" and "Output · USB Studio Output", each "4 channels", with Show in Mixer / Show in Audio at the far right.
  - Defects:
    - The selection fill covers only the title line. The subtitle sits below it without the fill.
    - The "●" still has no visible meaning.
    - Section captions are still indented about 11 px from their content.
- `…/devices-page-day.png` (1280×824): Day version. The selected row is a strong blue fill with white text, and the other rows have no fill. Legible. Defects: as in Night.
- `…/inspector-meter-night.png` (1280×824): Audio matrix in Channels mode. The inspector shows AUX0, "Audio · USB Studio", GOES TO "Discord · MONO" [Unlink], then LEVEL and FACTS. FACTS includes "Format 32 bit float mono audio".
  - **Meter:** two columns, about 48 px wide in total, spanning y≈211–341.
    - Lit green from the bottom up to about y≈236, with amber segments at about y≈224–232 (clearest in the left column). Unlit above that.
    - A scale to its left reads +6, 0, −12, −24, −36, −60, −∞. The labels are evenly spaced about 16 px apart (y≈228, 244, 260, 276, 292, 308, 324).
    - Below it: "Peak −1.9 dB".
  - Defects:
    - The lit top and amber segments sit beside or above the "+6" label, while the readout says −1.9 dB.
    - The labels are evenly spaced although the steps are 6 / 12 / 12 / 12 / 24 dB. They also cover only the middle of the meter's height.
    - The segments are still thin (about 3 px).
    - Two columns are shown for a port listed as mono.
- `…/inspector-meter-day.png` (1280×824): Day version with the same layout. Dark green with brown-amber at the top. "Peak −1.9 dB". Legible. Defects: as in Night.
- `…/mixer-meters-off-night.png` (1280×824):
  - The toolbar "Meters" is plain, not accent-filled.
  - Every strip has a mono "METERS OFF" caption below its value box, above "FL·FR". It doesn't overlap the meter and fits the strip width.
  - Meter segments are visibly darker than the idle (unlit) segments in mixer-page-night.
  - Defects: the caption is small (about 7 px cap height) and grey, but legible.
- `…/mixer-meter-paused-night.png` (250×414): widget harness.
  - "PAUSED" sits below the "0.0 dB" value box, clear of the meter.
  - The meter segments are grey and dimmer than in `mixer-widgets-night-rest.png` (opened for reference: slate-blue segments).
  - Defects: the caption is small (about 7 px). The harness menu is clipped ("Help" shows as "Hel").
- `…/mixer-meter-paused-day.png` (250×414): "PAUSED" sits below the value box, and no segment edges cross it.
  - The segments are paler and greyer than in `mixer-widgets-day-rest.png` (opened for reference: lavender-blue).
  - Defects: small caption; the harness menu is clipped.
- `…/mixer-page-night.png` (1280×824):
  - **Device strips:** Speakers (subtitle Speakers, route "Speakers"), Studio Output (subtitle Speakers, route "Speakers"), Microphone (subtitle Mic input, route "Mic input"), Studio Input (subtitle Mic input, route "Mic input").
  - **Default ◉:** filled with the accent on Studio Output and Studio Input.
  - **Stream routes:** Firefox ×2 "Studio Output", Spotify "Speakers". MAIN: "Studio Output".
  - **Meters:** lit on Studio Output and MAIN, green with amber from about −6 dB up.
  - **Sidebar:** now includes SYSTEM › Devices (0).
  - Defects (carried over):
    - "−∞" sits about 4 px above the value box.
    - Device and stream pop-ups still use two different styles.
    - The subtitle repeats the route name shown in the pop-up.
- `…/mixer-page-day.png` (1280×824): Day version of the above. Legible, and the meter colours can be told apart. Defects: as in Night.

## Checks
- **Major 1 (long target / device names cut, target hidden): pass.**
  - The same names as the VM case ("musubi-vm-default" / "musubi-vm-patch") are now cut so the end stays visible: "…vm-patch", "…-default". Default and patch can be told apart in both themes.
  - The device names are cut in the middle ("Built-in …eo Input" / "…crophone"), so their distinguishing endings show.
- **Major 2 (Devices list as push-buttons, weak selection): pass.**
  - The rows are flat, with no per-item borders.
  - The selected row has a full accent fill with white text, which is strong in both Night and Day.
- **Major 3 (inspector meter a 10 px sliver, hair-thin segments, no scale or readout): pass.**
  - It is now about 48 px wide, with two columns, a scale and a "Peak −1.9 dB" readout.
  - The segments are still thin (about 3 px).
  - See the new blocker below: the scale and the readout disagree.
- **Major 4 ("Meters off" / "paused" labels tiny, spilling, crossed by segments; off/paused meter looks idle): pass.**
  - The captions "METERS OFF" and "PAUSED" now sit below the value box. They don't overlap or spill, and in Day no segment edges cross them.
  - Off and paused segments are visibly dimmer or greyer than idle ones in both themes.
  - The captions are still small (about 7 px) but legible.
- **Major 5 (input devices showing route "Speakers", every subtitle "Speakers"): pass.**
  - Microphone and Studio Input show the subtitle and route "Mic input". The output devices show "Speakers".
- **Major 6 (identical node rows with no direction): pass.**
  - The node rows now carry a direction prefix: "Input · USB Studio Input" / "Output · USB Studio Output", in both themes.

## New defects (ranked)
**Blocker**
- **Inspector LEVEL (both themes): the scale contradicts the readout.**
  - The lit top and amber segments sit level with or above the "+6" label, and green reaches above "0", while the readout says "Peak −1.9 dB".
  - The scale labels are evenly spaced at about 16 px across unequal dB steps (6 / 12 / 12 / 12 / 24) and span only y≈228–324 of a meter that runs y≈211–341.
  - On the mixer strips, amber starts near −6.
  - As drawn, the scale misleads about the level.

**Major**
- None.

**Minor**
- The inspector meter draws two columns for a port whose FACTS say "mono audio".
- Devices list: the selection fill covers only the title line, and the subtitle sits outside it. The "●" is still unexplained.
- Cut pop-up labels drop the shared "musubi-" prefix, and the cut length varies ("…vm-patch" vs "…-default").
- Device strip subtitles repeat the route already shown in the pop-up ("Speakers" / "Speakers", "Mic input" / "Mic input").
- The "METERS OFF" / "PAUSED" captions are small (about 7 px cap height) and grey.
- Carried over, unchanged:
  - "−∞" sits about 4 px above the value box.
  - Device and stream pop-ups use two styles.
  - The widget harness menu is clipped ("Hel").

## Unreadable / uncertain
- The exact pixel positions of the inspector meter segments against its scale labels are estimated from the original-resolution image. The roughly 15–20 px offset is clear, but not the exact dB the meter is meant to show.
- Majors 1 and 6 were reported on VM captures. No new VM capture was supplied. Both passes come from the goldens:
  - Major 1's golden uses the same musubi-vm-default / patch names as the VM.
  - Major 6's golden node names already differ, so the case of two rows with identical names isn't shown directly.
- Device counts differ between fixtures: the mixer goldens show Devices 0 and Mixer 8, the devices goldens show Devices 3 and Mixer 16. This looks like fixture data and wasn't judged.

## Re-check after `9ffbb69` (verbatim)

I re-checked both inspector-meter images at full size. The blocker is resolved: the scale labels, the lit and amber segments and the "Peak" readout now agree, and the mono port shows as one column. My pixel positions are estimates, so agreement means within a few pixels.

## Images
- `repos/aizome-musubi/tests/snapshots/inspector-meter-night.png` (1280×824): Audio matrix in Channels mode. The inspector shows "AUX0", "Audio · USB Studio", GOES TO "Discord · MONO" [Unlink], LEVEL, then FACTS, which include "Format 32 bit float mono audio".
  - **Meter:** a single column about 24 px wide (x≈1074–1097), running from y≈211 to y≈351.
    - It is unlit grey at y≈211–220, amber at y≈223–236, and green from about y≈237 down to y≈351.
  - **Scale:** labels on the left:

    | Label | Position (y≈) |
    |---|---|
    | +6 | 214 |
    | 0 | 220, moved right to x≈1050 |
    | −12 | 247 |
    | −24 | 273 |
    | −36 | 300 |
    | −60 | 350, level with the meter's bottom |
    | −∞ | 371 |

  - **Readout:** "Peak −1.9 dB" at y≈392.
  - Defects: "+6" and "0" are crowded and offset from each other; "−∞" sits about 20 px below the meter's bottom; the segments are still thin (about 3 px).
- `repos/aizome-musubi/tests/snapshots/inspector-meter-day.png` (1280×824): Day version of the same layout. One column, unlit lavender at the top, then brown-amber, then dark green. The scale and "Peak −1.9 dB" are in the same places and dark and legible. Defects: as in Night.

## Checks
- **Scale labels, lit and amber segments and Peak readout agree: pass.**
  - The labels from 0 down to −60 are evenly spaced per dB: about 26–27 px per 12 dB, and 50 px for the 24 dB from −36 to −60, so about 2.2 px per dB.
  - At that spacing, "Peak −1.9 dB" should be about 4 px below the "0" label (about y≈224). The lit top is at about y≈223, so they match.
  - The amber runs from just below 0 to about −6/−7 dB (y≈223–236), which matches where the mixer strips turn amber.
  - Nothing above the lit level is lit, and the unlit segments above it sit beside the "+6" label.
  - The same holds in Day.
- **Mono port shown as one column: pass.** There is one meter column in both themes, matching "32 bit float mono audio" in FACTS.
- **Blocker from my previous report (inspector scale contradicts readout): pass (resolved).**

## New defects (ranked)
**Blocker**
- None.

**Major**
- None.

**Minor**
1. "+6" (y≈214) and "0" (y≈220) are only about 6 px apart. "0" is pushed right (x≈1050) to avoid "+6", so the two labels are staggered and crowded.
   - The space for +6 to 0 (about 6–9 px) is also squeezed compared with the roughly 2.2 px per dB used below 0.
   - This is readable, but it is the only uneven part of the scale.
2. "−∞" (y≈371) floats about 20 px below the meter's bottom (y≈351), roughly halfway to the "Peak" line, so it reads as detached from the meter.
3. Carried over: the meter segments are still thin, about 3 px with 1 px gaps. They are readable in both themes but harder on Day's light grey.

## Unreadable / uncertain
- Whether the top segments (y≈211–220) use the same dB-per-pixel spacing as below 0, or a squeezed headroom zone. The segments look evenly spaced, but the +6 and 0 labels are closer together than a linear scale would put them. This doesn't affect agreement at the current −1.9 dB peak.