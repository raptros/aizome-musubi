# Phase 3 visual review (sekka-visual, anthropic/claude-opus-5-5, 2026-11-25)

Verbatim. Goldens at aizome-musubi `c99222c`; VM evidence `runs/2026-11-25/patchbay/phase3/s5/evidence/`. No blockers; majors go to P3-S5a with the code review.

## Images
- `repos/aizome-musubi/tests/snapshots/effects-five-night.png`: Night theme. The toolbar reads "Effects · 1 chain" with Undo greyed out.
  - **Chain list:** the CHAINS list has a filled indigo row with an icon and "Studio EQ". A hollow ring sits outside the row on its right, and small grey "stopped · Follow default" sits under it. OTHER EFFECTS reads "No external effects found".
  - **Editor header:** the editor shows a "···" button, the heading "Studio EQ" with its subtitle, and plain "stopped" in the middle of the header. Below are Name [Studio EQ], Sends to [Follow default ⌄], Preset [Custom ⌄], and unchecked Enable and Bypass checkboxes. "Delete EQ…" has a trash icon.
  - **Curve:** a smooth curve with numbered handles 1–5. Handle 1 is selected (blue fill, amber edge). The axis ticks run from +15 to −15 and 20 to 20k, with "dB" at the top left and "Hz" at the top right.
  - **BANDS table:** five rows with # / Type ⌄ / Frequency (Hz) / Gain (dB) / Q, and "+ Add Band" below.
  - **Status bar:** "Studio EQ · stopped" on the left, "1 chain" on the right.

  Defects:
  - The "dB" axis label is drawn on top of the "+15" tick label.
  - Table numbers are left-aligned, Gain has no "+", decimals are mixed ("4", "-2.5", "0", "2"), and the table uses "-" while the axis uses "−".
  - The selected band 1 isn't highlighted in the table or the status bar.
  - About 215 px on the right of the table is empty.
  - There's no On toggle or remove control per band.
  - The status ring sits outside the selected row's fill.
  - The "stopped" label floats mid-header and isn't aligned to anything.
  - "···" has no text label.
- `repos/aizome-musubi/tests/snapshots/effects-five-day.png`: Same content in Day. Fields are grey on a pale lavender panel, the curve is dark blue, the handles are white, and selected handle 1 is navy with an amber edge. Defects:
  - The same "dB"/"+15" overlap and table problems as Night.
  - The "1" on the navy selected handle has very low contrast.
  - The Name field and the table cells have the same grey fill as each other.
  - The checkbox outlines are light but visible.
- `repos/aizome-musubi/tests/snapshots/effects-failed-night.png`: Same as five-night with these changes:
  - **Chain list:** the status dot is solid red, and a red caption wraps over two lines: "failed · PipeWire filter-chain exited with an error ·" / "Follow default".
  - **Editor header:** red "failed · PipeWire filter-chain exited with an error" plus a [Retry] button. Enable is checked.
  - **Status bar:** "Studio EQ · failed · PipeWire filter-chain exited with an error" and "1 chain".

  Defects:
  - The list caption wraps, leaving a "·" dangling at the end of the first line.
  - There's no log action (minor).
  - The "dB"/"+15" overlap and the table problems remain.
- `repos/aizome-musubi/tests/snapshots/effects-failed-day.png`: Same as failed-night in Day. The red text and red dot are readable, and Enable shows as checked. Defects: the same as failed-night, plus the low-contrast "1" on the selected handle.
- `repos/aizome-musubi/tests/snapshots/effects-empty-night.png`: Night. The toolbar reads "Effects · 0 chains". CHAINS says "No EQ chains yet" and the editor says "No EQ chains yet / Create a new EQ to get started." There's a "···" button and "+ New EQ", and the status bar says "0 chains". Defects: none.
- `repos/aizome-musubi/tests/snapshots/effects-empty-day.png`: Same in Day, readable. Defects: none.
- `repos/aizome-musubi/tests/snapshots/effects-delete-night.png`: The failed page, slightly dimmed, with a dialog at the top that has an amber top edge.
  - **Title:** "Delete EQ".
  - **Body:** "Remove "Studio EQ" and its unit?" / "aizome-musubi-effect@eq-1.service" / "Files:" / "/tmp/musubi-effects-page-night/aizome-musubi/effects/" / "eq-1.json" / "/tmp/musubi-effects-page-night/aizome-musubi/effects/" / "eq-1.conf".
  - **Buttons:** [Cancel] and [Delete EQ] with red text and a red outline.

  Defects:
  - Each file path wraps after "effects/", so the file name sits alone on the next line with no indent and reads like a separate entry.
  - The dialog covers the editor heading, Name and Sends to.
- `repos/aizome-musubi/tests/snapshots/effects-delete-day.png`: The same dialog in Day, with white dialog, the "-page-day" paths and a red outlined [Delete EQ]. Defects: the same path wrapping and covered heading as Night.
- `repos/aizome-musubi/tests/snapshots/clock-automatic-night.png`: Clock & Load, Night, Undo greyed out.
  - **CLOCK card:** "Effective graph values": Rate · 48000 Hz, Quantum · 256 frames, Graph period · 5.33 ms, with a note: "One graph period is not measured round-trip latency."
  - **Forced values · runtime:** "Rate: Automatic", "Quantum: Automatic", two [Automatic ⌄] pop-ups, and a separate [Automatic] button at the far right. Below: a greyed-out "Use at every start" checkbox and the note "Uses only the explicit forced numbers. Takes effect at the next PipeWire start. `aizome-musubi effects remove-all` removes this setting."
  - **DSP LOAD · LAST 60 S:** "11.2% of graph cycle". Labels 100% and 0%, a row of grey dashes, and one small amber bar at the far right.
  - **DRIVERS AND FOLLOWERS table:** Test audio driver / Driver / 48000 / 256 / 0.60 / 4.73 / 0 / 0, and Test playback / Follower / 48000 / 256 / ….
  - **Status bar:** left "Forced clock: Automatic · Automatic · DSP 11.2%", right "Forced Automatic · Automatic frames · DSP 11.2%".

  Defects:
  - The status bar says the same thing twice in two different phrasings, and "Automatic frames" makes no sense.
  - The far-right [Automatic] button has the same label as the pop-up values, so it reads like a status badge.
  - The "Quantum:" label (x≈425) doesn't line up with its pop-up (x≈445).
  - Busy / Wait has no unit.
  - The sidebar footer reads "Defaults · 48 kHz · 1024 · 21.3 ms" while the effective values are 256 / 5.33 ms.
  - The persistence note points to an *effects* command for a clock setting.
- `repos/aizome-musubi/tests/snapshots/clock-automatic-day.png`: Same in Day. Defects: the same as Night, and the DSP zero dashes are very faint on the light panel.
- `repos/aizome-musubi/tests/snapshots/clock-forced-night.png`: Same layout with Undo active.
  - **Forced values:** "Quantum: 256" and the quantum pop-up shows [256 ⌄]. "Use at every start" is enabled and unchecked.
  - **Effective values:** still 256 frames / 5.33 ms, so forced and effective values are identical here.
  - **Table:** Xruns now read 1 / 3.
  - **Status bar:** left "Forced clock: Automatic · 256 frames · DSP 11.2%", right "Forced Automatic · 256 frames · DSP 11.2%".

  Defects:
  - The status bar is duplicated.
  - The footer still reads 1024 / 21.3 ms.
  - The [Automatic] button still reads like a badge.
- `repos/aizome-musubi/tests/snapshots/clock-forced-day.png`: Same in Day. Defects: the same as Night, plus faint DSP dashes.
- `repos/aizome-musubi/tests/snapshots/clock-read-only-night.png`: Forced 256 state.
  - **Disabled controls:** both pop-ups and [Automatic] are greyed out.
  - **Reason:** red "Read-only · PipeWire denies metadata writes" sits under the pop-ups.
  - "Use at every start" is greyed out, and the persistence note is readable. The DSP chart and table are as in the forced shot.
  - **Status bar:** duplicated as in the forced shot.

  Defects:
  - The status bar is duplicated.
  - Undo in the toolbar is still active while the page is read-only.
  - The footer reads 1024.
- `repos/aizome-musubi/tests/snapshots/clock-read-only-day.png`: Same in Day. The red reason is readable, and the disabled pop-up text is faint but legible. Defects: the same as Night.
- `runs/2026-11-25/patchbay/phase3/s5/evidence/00-effects-created.png`: Real Aizome session at 20:12.
  - **Desktop:** Toolchest menu bar, workspaces 1–4, a right panel (Annai / Welcome to Aizome / Musubi / Musubi — Patchbay / SHELF "Drop to park"), and a window tab "Musubi — Patchbay" with an amber edge.
  - **Toolbar:** "Effects · 1" with Undo greyed out.
  - **Chain list:** "EQ 1", with a hollow ring and "stopped · Follow default".
  - **Editor:** Preset [Flat ⌄], Enable and Bypass unchecked, a flat 0 dB line, and selected handle 1 at 1k.
  - **Table:** 1 / Peaking / 1000 / 0 / 1.
  - **Status bar:** "EQ 1 · stopped" and "1 chain". The pointer is on "New EQ".

  Defects:
  - The toolbar leaves out the noun ("Effects · 1", where the golden says "Effects · 1 chain").
  - The "dB"/"+15" overlap.
  - Undo is greyed out right after the create action. [INFERENCE] This may be intended.
- `runs/2026-11-25/patchbay/phase3/s5/evidence/00-effects-boosted.png`: 20:13, Undo active.
  - **State:** a green dot, green "running · Follow default", and green "running" in the header. Enable is checked, Bypass unchecked, and Preset is [Custom ⌄].
  - **Curve:** a smooth peak at 1k rising to about +12 dB, with handle 1 at the peak.
  - **Table:** Gain reads "12".
  - **Counts:** Audio 4, Mixer 7. The status bar reads "EQ 1 · running" and "1 chain". The pointer is on the chart.

  Defects:
  - The toolbar reads "Effects · 1".
  - The "dB"/"+15" overlap.
  - Gain "12" has no "+".
- `runs/2026-11-25/patchbay/phase3/s5/evidence/00-effects-deleted.png`: 20:13.
  - **Toolbar:** "Effects · 0".
  - **Page:** "No EQ chains yet" in both the list and the editor.
  - **Counts:** the sidebar shows Effects 0, Audio 2 and Mixer 5, and the status bar says "0 chains".

  Defect: the toolbar reads "Effects · 0" (no noun).
- `runs/2026-11-25/patchbay/phase3/s5/evidence/00-clock-forced.png`: 20:13, Undo active.
  - **Effective values:** Rate · 48000 Hz, Quantum · 2048 frames, Graph period · 42.67 ms.
  - **Forced values:** "Quantum: 2048", with the quantum pop-up showing [2048 ⌄]. "Use at every start" is enabled and unchecked.
  - **DSP chart:** "0.0% of graph cycle", amber dashes along 0%.
  - **Table:** musubi-vm-default / Driver / 48000 / **2048**, and musubi_vm_player / Follower / 48000 / **2048**. The last column header is cut to "Xruns / err…".
  - **Status bar:** left "Forced clock: Automatic · 2048 frames · DSP 0.0%", right "Forced Automatic · 2048 frames · DSP 0.0%".
  - **Sidebar:** Effects 1, and the footer reads "Defaults · 48 kHz · 1024 · 21.3 ms".

  Defects:
  - The status bar is duplicated.
  - The table header is cut off.
  - The footer reads 1024 / 21.3 ms while the page shows 2048 / 42.67 ms.
  - The sidebar shows Effects 1, while the deletion shot (also 20:13) shows 0.
- `runs/2026-11-25/patchbay/phase3/s5/evidence/00-clock-automatic.png`: 20:13.
  - **Effective values:** Quantum · 4096 frames, Graph period · 85.33 ms.
  - **Forced values:** "Rate: Automatic", "Quantum: Automatic", both pop-ups [Automatic ⌄]. The pointer is on the [Automatic] button, and "Use at every start" is greyed out.
  - **Table:** both nodes show quantum **4096**.
  - **Status bar:** left "Forced clock: Automatic · Automatic · DSP 0.0%", right "Forced Automatic · Automatic frames · DSP 0.0%".
  - **Sidebar:** Effects 1, and the footer reads 1024 / 21.3 ms.

  Defects:
  - The status bar is duplicated and says "Automatic frames".
  - The footer reads 1024 / 21.3 ms while the effective values are 4096 / 85.33 ms.
  - The table header is cut to "Xruns / err…".
  - The sidebar shows Effects 1 after the deletion shot.

## Checks
1. **Prior preview findings resolved: pass.** Each item from the 2026-11-24 preview:
   - **Bypass state: pass.** Bypass is now a checkbox with a visible unchecked state in every effects shot. No shot shows Bypass turned on.
   - **Chain list rows and selection: pass.** Each chain is a filled indigo row with an icon, like the Devices list, and the status dot changes: hollow ring when stopped, red when failed, green when running. Minor: the dot sits outside the row's fill, and the failed caption wraps with a dangling "·".
   - **Failed state, reason and Retry: pass.** The reason is now readable: "PipeWire filter-chain exited with an error". It appears in red in the header, the list caption and the status bar, and there is a [Retry] button. No log action is shown.
   - **Enable vs state: pass.** Enable is checked in the failed shots and in the VM running shot, and unchecked when stopped.
   - **Dropdown chevrons: pass.** Sends to, Preset, the Type cells and the clock pop-ups all show "⌄".
   - **Smooth curve with axis units: pass.** The curve is smooth with no stepping, and "dB" and "Hz" labels are present. The "dB" label overlaps "+15" (see check 2).
   - **Wording: pass in the goldens, with minor issues.**
     - Fixed: "1 chain" / "0 chains", the status bar reads "Studio EQ · stopped" with no repeated "EQ", "No EQ chains yet", and the button and dialog names now match ("New EQ", "Delete EQ…", "Delete EQ"). Preset now reads "Flat" / "Custom".
     - Remaining minor issues: the VM toolbar drops the noun ("Effects · 1", "Effects · 0"), and "···" has no text label.
2. **Effects editor legibility: fail.**
   - **Curve handles and band table: pass.** All handles sit where the table says:
     - Golden: 105 Hz at +4, 220 at −2.5, 1k at 0, 5.6k at +2, 10k at −1.5.
     - VM: 1k at 0, then 1k at +12.
   - **Table alignment and units: pass, with minor issues.** The columns line up and the units are in the headers. Numbers are still left-aligned, with no "+" on gains, mixed decimals, a hyphen instead of a minus sign, and no selected-row highlight.
   - **Axis label collision: fail.** "dB" is drawn on top of "+15" in every effects golden (Night and Day) and in both VM effects shots.
3. **Delete confirmation: fail.**
   - **Names the unit and files: pass.** "Studio EQ", `aizome-musubi-effect@eq-1.service`, `…/effects/eq-1.json` and `…/effects/eq-1.conf` all appear.
   - **Unit name unbroken: pass.** It stays on one line.
   - **Paths unbroken: fail.** Each path wraps after "effects/", so "eq-1.json" and "eq-1.conf" sit alone on the next line with no indent, in both Night and Day.
   - **Destructive styling: pass.** [Delete EQ] has red text and a red outline, clearly different from [Cancel].
4. **Clock & Load: fail.**
   - **Effective vs forced values: pass.**
     - Goldens: the effective quantum (256) matches the table.
     - VM forced: effective 2048 frames / 42.67 ms, table 2048, forced 2048.
     - VM automatic: effective 4096 frames / 85.33 ms, table 4096.
     - In the forced golden, forced and effective are both 256, so the golden never shows them differing.
     - Outside this section, the sidebar footer "Defaults · 48 kHz · 1024 · 21.3 ms" never changes and disagrees with every effective value shown.
   - **Automatic and "Use at every start": pass, with issues.**
     - The persistence note is readable in Night and Day. "Use at every start" is enabled when a value is forced and greyed out when everything is Automatic; the note says why ("Uses only the explicit forced numbers").
     - The far-right [Automatic] button has the same label as the values beside it and reads like a badge. It's only identifiable as a button because the VM pointer is on it.
     - The note names `aizome-musubi effects remove-all` for a clock setting.
   - **Read-only state shows its reason: pass.** The controls are disabled and red "Read-only · PipeWire denies metadata writes" is shown. Undo stays active (minor).
   - **DSP chart legible with an axis: pass, with minor issues.** The y labels 100% and 0% are legible, the title gives "LAST 60 S", and the current value appears as text. There's no time axis or gridlines, and the grey zero-dashes in Day are very faint.
   - **Status bar not duplicated: fail.** Every clock golden and VM shot shows the forced state twice, worded differently on the left and right, and the right side says "Automatic frames".
5. **VM screenshots: pass.** They match the goldens' look (Night tokens, Archivo UI text, mono numbers, same layout) inside a real Aizome session. Differences are the toolbar dropping its noun and the narrower window cutting the "Xruns / err…" header.
   - **Created chain: pass.** "EQ 1" is stopped with a flat curve.
   - **Boosted chain: pass.** The chain is running (green), Enable is checked, and the curve is raised to about +12 dB at 1k, matching Gain 12.
   - **Deletion result: pass.** The page shows 0 chains with the empty state and Effects 0 in the sidebar.
   - **Forced and automatic clock states: pass.** Both show 2048 then Automatic, with effective values matching the table.
6. **Day contrast: pass, with minor issues.** Text, red failure and read-only messages, the curve, ticks, the table and the dialog are readable. Weak spots:
   - the "1" on the navy selected handle is hard to read;
   - the DSP zero-dashes are very faint;
   - the Name field and table cells share the same grey;
   - disabled clock pop-ups are faint (expected for disabled controls).

## Unreadable / uncertain
- I can't confirm that the "1" on the navy selected handle in `effects-five-day.png` / `effects-failed-day.png` is readable at original resolution; it has very low contrast.
- No shot shows Bypass turned on, so I can't check how the engaged state looks.
- The order of the VM shots isn't clear. The clock shots and `00-effects-deleted.png` are all stamped 20:13, but the clock shots show Effects 1 (Audio 2, Mixer 5) while the deletion shot shows Effects 0. I can't tell whether the clock shots were taken before the deletion or the count is stale.
- I can't tell whether the greyed-out Undo right after creating the chain in `00-effects-created.png` is intended.
- I can't tell whether the footer's "Defaults · … · 1024 · 21.3 ms" is meant as the configured default rather than the current clock.
- The closing quote character around `aizome-musubi effects remove-all` is too small to identify.

## Ranked defects
**Blocker**
- None found. No control is hidden and no state indicator is wrong on the page itself. The footer item under Major could mislead if it is meant to show the live clock.

**Major**
- The Clock & Load status bar shows the forced state twice, in different words, on the left and right, and says "Automatic frames". This is in every clock golden and VM shot.
- The sidebar footer "Defaults · 48 kHz · 1024 · 21.3 ms" never changes and disagrees with the effective values on screen (256 / 5.33 ms, 2048 / 42.67 ms, 4096 / 85.33 ms).
- On the response curve, the "dB" axis label is drawn over the "+15" tick in every effects shot, Night, Day and VM.
- In the delete dialog, the file paths wrap after "effects/", leaving the file names on separate unindented lines.
- The [Automatic] reset button has the same label as the values beside it and reads like a status badge rather than an action.

**Minor**
- The VM toolbar drops its noun ("Effects · 1", "Effects · 0"), unlike the goldens' "1 chain" / "0 chains".
- Band table: numbers left-aligned, no "+" on gains, mixed decimals, a hyphen instead of a minus sign, no highlight on the selected band in the table or status bar, about 215 px empty on the right, and no On or remove control per band.
- The failed list caption wraps with a "·" dangling at the line end.
- The status dot sits outside the selected chain row's fill.
- The "stopped" / "running" label floats in the middle of the editor header.
- The persistence note cites an *effects* command for a clock setting.
- The "Quantum:" label doesn't line up with its pop-up, and Busy / Wait has no unit.
- The DSP chart has no time axis or gridlines, and its Day zero-dashes are very faint.
- In the narrow VM window, the "Xruns / errors since opened" header is cut to "Xruns / err…".
- Undo stays active in the read-only clock state.
- The delete dialog still covers the editor heading.
- "···" has no text label.
- Day: the selected handle's number has low contrast, and the Name field and table cells share the same grey.