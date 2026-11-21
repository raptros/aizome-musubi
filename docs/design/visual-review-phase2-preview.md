# Phase 2 visual preview — Mixer page (sekka-visual, anthropic/claude-opus-5-5, 2026-11-20)

Verbatim. Images at aizome-musubi `8c639e2`. Findings feed P2-S5 (see `docs/phases/phase-2.md`).

## Images
- `repos/aizome-musubi/docs/design/mockups/mixer-night.png` (mockup, 1280×800): the Night window has no menu bar. The sidebar has PATCHBAY, MIXER (Mixer, Effects) and SYSTEM (Devices, Graph, Setups) sections, and the selected "Mixer" row is an inset rounded fill. The toolbar shows "Mixer 9 strips", dB|%, an accent-filled "Meters" with an icon, "+ Show…" and a filter field. Each strip has an icon tile, name and subtitle, a pop-up with chevron, a balance slider labelled L…R, buttons M / ◉ / link, a fader with a +6…−60 mono scale, a two-column meter (one column for the MONO strip), a value box and a channel label (FL·FR, AUX0, MONO). The muted Microphone M is red. MAIN's ◉ is accent-filled. The focused strip has an accent outline. The status bar has a key hint on the left and "meters at 30 Hz · 9 streams" on the right. Defects: none (reference).
- `tests/snapshots/mixer-page-night.png` (1280×824):
  - **Chrome:** menu bar, sidebar with Audio/MIDI/MIDI 2.0/Video all showing 0, and Mixer under MIXER. The toolbar reads "Mixer", [dB|%] with dB selected, "Meters" with a thin outline, "Show…", a greyed "Undo" and "Filter strips".
  - **Sections and strips:** OUTPUTS (Speakers, Studio Output), INPUTS (Microphone, Studio Input), PLAYING (Firefox/Video, Firefox/Music, Spotify/Playlist), and RECORDING ("Dis…/Cal…", cut by the pinned MAIN strip). Each strip starts with a blue kind tag (OUT/IN/PLAY) instead of an icon tile.
  - **Device strips:** the subtitle is the raw node name (`alsa_output.analog`, `alsa_output.usb_studio`). Below it is plain text "Device route" with no box or chevron. There is a balance slider with no L/R labels, then [M][●][↔].
  - **Stream strips:** a pop-up with chevron ("Studio Output ⌄", "Speakers ⌄") and M only.
  - **MAIN:** "MAIN / Studio Output", pop-up "Studio Output ⌄", balance, M.
  - **Fader and meters:** fader scale +6, 0, −6, −12, −24, −36, −∞ (no −60 label). Meters have two columns with green and amber.
  - **Status bar:** "8 strips · default output Studio Output".
  - Defects:
    - Device strips have no route pop-up, only the static placeholder "Device route".
    - No strip icon tiles.
    - The default "●" is shown only as a darker button fill (Studio Output and Studio Input dark; Speakers and Microphone grey).
    - Balance has no L/R labels.
    - Sidebar "Mixer" selection spans the full sidebar width, unlike the inset selection on the matrix pages. The Mixer row has no strip count, and it uses the same speaker icon as "Audio".
    - No horizontal scroll indicator is visible.
    - No channel labels under the value boxes.
    - Stream strips' M row and scale sit about 2 px higher than on device strips.
- `tests/snapshots/mixer-page-day.png` (1280×824): same layout in Day. Strips are light lavender panels on a pale background. Text is dark and kind tags are blue. Meters are dark green and brown-amber, with empty segments in light lavender. The selected "dB" is blue-filled. The status bar text matches Night. Defects:
  - Same structural defects as Night.
  - Default "●" differs only by a light-grey fill (Studio Output, Studio Input) versus white (Speakers, Microphone), which is very weak.
- `tests/snapshots/mixer-page-narrow-night.png` (900×560): the toolbar title "Mixer" is gone and Undo is an icon only. Visible strips are Speakers, Studio Output, Microphone and Studio Input. Firefox is cut by MAIN ("Fire", "Video", "Stu" pop-up; section header "PLAYI"). The scale is compressed to 20 px steps, readable and not overlapping. "−∞" sits about 8 px above the value box. Status: "8 strips · default output Studio Output". Defects: same default-state and "Device route" issues. The cut strip is deliberate.
- `tests/snapshots/mixer-page-narrow-day.png` (900×560): Day version of the above. Readable. Defects: same as above; the Day default state is weak.
- `tests/snapshots/mixer-toolbar-show-night.png` (1280×824):
  - **Filter and popover:** the filter field holds "Firefox" with a clear (⊗) button. The Show… popover sits below the button with a drop shadow and square corners. It has blue checked boxes for Outputs, Inputs, Playing, Recording and "Monitors/virtual devices". It covers part of the second Firefox strip, which is expected.
  - **Strips:** two Firefox strips plus MAIN. The meters are empty. MAIN's meter shows a single column with no "off" label, while the Firefox meters show two columns.
  - **Status bar:** "6 strips hidden" (8 − 2 Firefox).
  - Defects:
    - The popover has no option for an EFFECTS section (design §3 lists EFFECTS among the sections).
    - MAIN's meter shape changes without any label.
- `tests/snapshots/mixer-meters-off-night.png` (1280×824): "Meters" has no outline. Every strip's meter collapses to one grey column with a small "off" chip at its top-right. Everything else is as in page-night. Defects:
  - The "off" chip is tiny (about 13×10 px) and low contrast, though readable.
  - A one-column collapsed meter could read as a mono meter.
  - The only toolbar cue for on/off is the presence or absence of a hairline outline.
- `tests/snapshots/mixer-percent-night.png` (1280×824): "%" is selected. Value boxes read 80%, 80%, 80%, 80%, 70%, 70%, 70% and 80% (MAIN). The fader scale still shows dB labels (+6 … −∞). Meters are off (the "off" chips are visible). Defects:
  - The % value boxes are shown against a dB scale, which mixes units.
  - The capture also has meters off, so percent mode is never seen with live meters.
- `tests/snapshots/mixer-widgets-night-rest.png` (250×414): the harness menu bar is clipped ("Help" cut at the right edge). Header "OUTPUT · TEST STEREO". Fader scale +6, 0, −6, −12, −24, −36, −60, −∞. The thumb at 0 hides the 0 tick dot. Empty two-column meter. Value box "0.0 dB". Defects: "−60" and "−∞" are stacked about 12 px apart, which is crowded. "−∞" sits about 6 px above the value box. Harness menu clipped.
- `tests/snapshots/mixer-widgets-night-active.png` (250×414): thumb at −6, value box "−6.0 dB". The meter shows green, then amber from about −6 up, and one red segment at the top of the right column. Colours are distinguishable. Defects: same −60/−∞ crowding.
- `tests/snapshots/mixer-widgets-night-disabled.png` (250×414): visually almost identical to rest. The value text is at most slightly dimmer, and the fader line and thumb look unchanged. Defects: the disabled state is barely distinguishable from rest in Night.
- `tests/snapshots/mixer-widgets-day-rest.png` (250×414): Day. White thumb at 0, dark labels, lavender empty meter, value box "0.0 dB". Defects: −60/−∞ crowding; the "0·" label touches the thumb edge.
- `tests/snapshots/mixer-widgets-day-active.png` (250×414): thumb at −6. Dark green, brown-amber and a red top segment, all distinguishable. Defects: crowding as above.
- `tests/snapshots/mixer-widgets-day-disabled.png` (250×414): grey-blue filled thumb, grey filled value box, dimmed text. Clearly different from rest. Defects: none beyond the crowding.
- Matrix pages opened for comparison (`audio-studio-night.png`, `audio-studio-day.png`): same menu bar, app card, server card, toolbar height and mono status bar. The selected sidebar row is an inset rounded fill with its count ("4") outside the fill. Toolbar toggles carry Lucide icons (Monitors, Swap, Full Names) and the title carries counts ("Audio 46 ports · 4 links").

## Checks
- **1. Name, subtitle, pop-up with chevron and value box readable and unclipped (except scroll-cut strips): fail.**
  - Names, subtitles and value boxes are readable and unclipped on every fully visible strip, in both themes and at narrow width.
  - Stream and MAIN pop-ups show their chevron and fit ("Studio Output ⌄", "Speakers ⌄").
  - But every device strip (Speakers, Studio Output, Microphone, Studio Input) has no pop-up at all, only static text "Device route" with no box or chevron.
  - The only clipping is the RECORDING strip and the Firefox strip in narrow, both cut by horizontal scroll as allowed.
- **2. Fader scale labels readable, not colliding (including −∞), clear of the value box: pass.**
  - On the pages, +6/0/−6/−12/−24/−36/−∞ are readable and separate, also in the compressed narrow layout. "−∞" sits about 8 px above the value box, not touching. Thumbs never cover a label's text.
  - Minor: in the widget captures "−60" and "−∞" are crowded (about 12 px pitch) and "−∞" is about 6 px from the box. The page scale omits −60 although the design says +6…−60. The thumb hides the 0 tick dot.
- **3. One meter bar per channel, green/amber/red distinguishable, off/paused state clear: uncertain.**
  - Stereo strips show two columns. Green, amber and red are distinguishable in both themes (red seen only in the widget active captures).
  - Meters-off shows an "off" chip, but it is tiny, and the meter collapses to a single column that resembles a mono meter.
  - No mono or multichannel strip is fully visible, and no "meter paused" state is captured.
  - MAIN's meter in the Show… capture is a single unlabelled column.
- **4. M, ◉ and balance states unmistakable in both themes; default device identifiable: fail.**
  - Default is shown only by a darker button fill: dark versus grey in Night, light grey versus white in Day. There is no accent fill and the glyph is identical. The default can only be identified by cross-reading the status bar.
  - No muted strip appears in any capture, so the M on-state cannot be judged.
  - Balance sliders have no L/R labels.
  - The "↔" channel-link button shows the same look on every strip, so its on/off state can't be determined.
- **5. Toolbar, Show… popover, filter and status bar read as Aizome, not clipped, status matches page: pass.**
  - Archivo text, mono status, kit segmented control, search field with clear button, checkbox popover. Nothing is clipped. The title correctly yields at 900 px.
  - Status text matches: "8 strips · default output Studio Output" equals the 8 non-MAIN strips and MAIN's "Studio Output". "6 strips hidden" equals 8 − 2 Firefox strips.
  - Minor:
    - The Meters on-state is only a hairline outline.
    - Meters and Show… lack the Lucide icons the matrix toggles use.
    - The title has no count.
    - The popover has no EFFECTS option.
- **6. Consistent with matrix-page chrome; Day contrast adequate: fail.**
  - Menu bar, app and server cards, toolbar height and status bar match the matrix pages.
  - The sidebar selection does not match: "Mixer" is a full-width square fill, while matrix pages use an inset rounded fill. The Mixer row has no count and reuses the Audio speaker icon.
  - Day text, scale labels and meter colours are adequately legible.
  - The Day default-device state (light grey versus white) is too weak.

## Unreadable / uncertain
- Whether the Night disabled widget differs from rest at all: they look near-identical in the captures I opened.
- M (mute) on-state: not captured.
- The "↔" channel-link on-state, and what the button means: not captured or labelled.
- The "meter paused" state above the 24-strip meter cap: not captured.
- Mono or multichannel meters: no such strip is fully visible.
- Horizontal scrollbar: none is visible in any capture, so I can't tell if one exists (it may be an overlay).
- Sidebar counts all read 0 on the mixer captures but "Audio 4" on the matrix capture. This may be a fixture difference; I can't tell from the images.
- `mixer-page-narrow-night.actual.png` is also in the folder (not assigned). I did not review it.

## Ranked defects
**Blocker**
1. Device strips have no route pop-up. A static placeholder "Device route" stands in for the pop-up with chevron (check 1).
2. The default device is not identifiable from the ◉ control in either theme: only a dark or grey fill, no accent, and weakest in Day (check 4).

**Major**
3. No muted strip is captured, so the M on-state is unverified. The ↔ channel-link on-state is unverified too, and its glyph doesn't read as "link".
4. Meters-off state: a tiny "off" chip, and the meter collapses to a mono-looking single column. MAIN's meter is an unlabelled single column in the Show… capture. The paused state is not captured.
5. The sidebar selection style differs from the matrix pages (full-width square versus inset rounded). The Mixer row has no strip count and reuses the Audio icon.
6. In Night, the disabled widget is barely distinguishable from rest.

**Minor**
7. Strip headers use OUT/IN/PLAY text tags instead of the mockup and design icon tiles. Subtitles are raw node names (`alsa_output.usb_studio`).
8. Balance has no L/R labels. There are no channel labels (FL·FR, MONO) under the value boxes.
9. The Meters on-state is only a hairline outline. Meters and Show… have no icons. The toolbar title has no count. The Show… popover has no EFFECTS option.
10. The page scale has no −60 label. In the widgets, −60 and −∞ are crowded and −∞ sits close to the value box. The thumb covers the 0 tick dot.
11. Percent mode shows % values against a dB scale.
12. Stream strips' controls and scale sit about 2 px higher than on device strips.
13. No visible horizontal-scroll affordance.
14. The widget harness menu bar is clipped ("Help").