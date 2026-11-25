# Phase 3 visual preview — Effects page (sekka-visual, anthropic/claude-opus-5-5, 2026-11-24)

Verbatim. Goldens at aizome-musubi `911dff0`. Findings feed P3-S5.

## Images
- `repos/aizome-musubi/tests/snapshots/effects-five-night.png`: This is the Night theme with the Effects page selected in the sidebar (count 1). The toolbar reads "Effects · 1 chains" next to a greyed "Undo". The CHAINS column has a single outlined, button-like box with "● Studio EQ" centred in it and a white dot. Under the box, outside it, tiny grey text says "stopped · Follow default". OTHER EFFECTS says "No external effects found", and "+ New EQ" sits at the foot of the column. The editor has the heading "Studio EQ" and the subtitle "A virtual output: audio sent to it is equalised, then passed on." Plain grey "stopped" is at the top right, with an "Effects menu" button above it. The row below has a Name field ("Studio EQ"), "Sends to [Follow default]" and "Preset [Flat / saved]", both plain buttons. Next come an unchecked "Enable" checkbox, a "Bypass" button and "Delete chain…" on the right. The curve plot runs +15 to −15 and 20 to 20k, with five small grey numbered square handles; handle 1 is selected (blue fill, amber edge). BANDS is a table with columns # / Type / Frequency (Hz) / Gain (dB) / Q and five rows (Low shelf 105 / 4 / 0.7; Peaking 220 / −2.5 / 1.2; Peaking 1000 / 0 / 2; Peaking 5600 / 2 / 1.4; High shelf 10000 / −1.5 / 0.7), followed by "+ Add Band". The status bar reads "Studio EQ EQ chain · stopped" on the left and "1 chains" on the right. Defects:
  - "1 chains" appears twice.
  - "Studio EQ EQ chain" repeats "EQ".
  - The chain entry looks like a button, not a list row with a selection.
  - Sends to and Preset have no chevron.
  - Bypass shows no on/off state.
  - The curve line is thin and visibly stepped/beaded, not smooth.
  - Neither axis has a unit label (no dB or Hz).
  - The selected band 1 isn't marked in the table or the status bar.
  - Numbers are left-aligned, Gain has no "+" and decimals are mixed ("4", "−2.5").
  - About 215 px of the table's right side is empty.
- `repos/aizome-musubi/tests/snapshots/effects-five-day.png`: Same layout and content in the Day theme. The chain dot is solid black, input fields are filled grey on a pale lavender panel, the curve is a thin dark-blue line, handles are white with grey edges, and selected handle 1 is navy with an amber edge. Defects:
  - Every Night defect above also applies.
  - The Enable checkbox outline is faint on the light background.
  - I can't tell how readable the "1" on the navy selected handle is.
  - The Name field has the same grey fill as the table cells.
- `repos/aizome-musubi/tests/snapshots/effects-failed-night.png`: Same as five-night, except the top right reads "failed · exit-code" in red. The chain caption is grey "failed · exit-code · Follow default", and the status bar says "Studio EQ EQ chain · failed · exit-code". The chain dot is still white, the same as when stopped, and Enable is unchecked. Defects:
  - The reason is the raw token "exit-code", not something a user can understand.
  - Nothing offers a retry or a log.
  - In the list, the failure is only tiny grey text; the dot doesn't change.
  - An unchecked Enable next to "failed" sends mixed signals.
  - The wording problems from five-night are all here too.
- `repos/aizome-musubi/tests/snapshots/effects-failed-day.png`: Same as failed-night in Day. Red "failed · exit-code" is readable on the light background, and the chain dot is solid black. Defects: the same as failed-night.
- `repos/aizome-musubi/tests/snapshots/effects-empty-night.png`: The toolbar reads "Effects · 0 chains". The CHAINS column says "No EQ chains. Create one to get started." and OTHER EFFECTS says "No external effects found". The editor area shows "No EQ chain selected" and "Select a chain or create a new EQ." The rest of the editor is blank apart from "Effects menu". The status bar says "0 chains" and "+ New EQ" is at the column's foot. Defects: the heading "No EQ chain selected" suggests a choice exists when there are no chains. Minor.
- `repos/aizome-musubi/tests/snapshots/effects-empty-day.png`: Same in Day, readable. Defects: the same as empty-night.
- `repos/aizome-musubi/tests/snapshots/effects-delete-night.png`: The failed-night page, dimmed, with a dialog at the top that has an amber top edge. It is titled "Delete EQ chain" and says: "Remove "Studio EQ", its unit aizome-musubi-effect@eq-1.service, and files /tmp/musubi-effects-page-night/aizome-musubi/effects/eq-1.json and /tmp/musubi-effects-page-night/aizome-musubi/effects/eq-1.conf?" The buttons are [Cancel] and a blue [Delete chain]. Defects:
  - The unit name and the paths break mid-token across lines ("aizome-musubi-" / "effect@eq-1.service", and the /tmp paths).
  - The destructive button uses the normal blue primary style.
  - The title says "Delete EQ chain" but the page button says "Delete chain…".
  - The dialog covers the editor heading.
- `repos/aizome-musubi/tests/snapshots/effects-delete-day.png`: The same dialog in Day, with the -day paths and a navy "Delete chain". Defects: the same as delete-night.
- `repos/aizome-musubi/docs/design/mockups/effects-night.png` (reference): The chain list has filled rows with an icon, name and subtitle, a filled indigo selection and a status square. The top right has "● running" plus a [Bypass] button. Sends to and Preset are pop-ups with chevrons. The curve is a smooth filled line with handles in per-band colours. The table has Hz, dB and Q right-aligned, a highlighted selected row and an On toggle per band. The status bar shows the selected band's values and a hint.
- `repos/aizome-musubi/tests/snapshots/devices-page-night.png` (reference): The list rows have an icon, a name and a dot on the right, with a subtitle below. The selected row is filled indigo. Pop-ups have chevrons ("Analog Stereo Duplex ⌄", "Speakers ⌄").
- `repos/aizome-musubi/tests/snapshots/mixer-page-night.png` (reference): Every target pop-up has a chevron and values are in mono boxes ("−5.8 dB"). The status bar reads "8 strips · default output Studio Output".
- `runs/2026-11-24/patchbay/phase3/s3/`: There is no real-VM screenshot evidence. The folder has no PNG files, only `vm.log`, `full-test.log`, `effects-harness.log`, `behaviour-test.log` and `unit-scaffold.log`. The directory listing also showed a 0 B `vm` entry, but reading it failed with "Path … not found".

## Checks
1. **Chain list: fail.** The entry is an outlined box with its text centred, so it looks like a push button. It has no icon and no indigo fill. On Devices, the same kind of list uses icon rows and a filled selection. I can only infer the selection because the editor shows Studio EQ. The status ("stopped" / "failed · exit-code") does appear as text, which is good, but it's tiny grey text outside the box. The dot is the same in both states (white at Night, black at Day), so it says nothing about state.
2. **Editor controls: fail.** The Name field is clear. "Sends to [Follow default]" and "Preset [Flat / saved]" have no chevron, unlike the Devices, Mixer and mockup pop-ups, so they look like action buttons. Enable is a clear unchecked checkbox. Bypass is a plain button with no pressed or on/off look, so you can't tell whether the EQ is bypassed. In the failed shots Enable is unchecked while the state says failed, which is contradictory.
3. **Response curve: fail.**
   - The bands match the table: 1 is at about 105 Hz, +4; 2 at 220, −2.5; 3 at 1k, 0; 4 at about 5.6k, +2; 5 at 10k, −1.5.
   - The handles are numbered and readable at Night.
   - The tick values are there (+15 to −15; 20 to 20k).
   - However, the line is a 1 px stepped/beaded trace, not a smooth curve, and neither axis has a unit (dB or Hz).
4. **Band table: pass, with minor issues.** The columns line up, the cells are bordered input fields with Type drop-downs, and the units are in the headers ("Frequency (Hz)", "Gain (dB)"). Minor issues:
   - Numbers are left-aligned.
   - Decimals are mixed ("4", "2", "−2.5") and gains have no "+".
   - The selected band isn't highlighted.
   - There is no per-band On toggle or remove control (the mockup has an On toggle).
   - The right side of the table is empty.
5. **Failed chain: fail.** The state is clear in the editor (red "failed" at the top right and in the status bar). The reason is just "exit-code", which doesn't tell a user what went wrong, and there is no retry or log action. In the chain list, the failure is tiny grey text with the same neutral dot as a stopped chain.
6. **Delete confirmation: pass.** It names the chain "Studio EQ", the unit `aizome-musubi-effect@eq-1.service` and both files (`eq-1.json`, `eq-1.conf`). Minor issues: the unit name and paths break mid-token, and the destructive button isn't styled as destructive.
7. **Wording: fail.**
   - Pluralisation: "Effects · 1 chains" in the toolbar and "1 chains" in the status bar, in the five, failed and delete shots.
   - Duplicated word: "Studio EQ EQ chain".
   - Unclear labels: "Flat / saved", "Effects menu" and "exit-code".
   - Inconsistent naming: "New EQ", "Delete chain…" and "Delete EQ chain".
   - Empty state: "No EQ chain selected" when there are no chains.
8. **Day contrast: pass, with minor issues.** Text, tick labels, red "failed · exit-code", table cells and handle numbers on white are all readable. The Enable checkbox outline is faint and the grey field fills are close to the panel colour. I can't confirm how readable the "1" on the navy selected handle is (see Unreadable / uncertain).

## Unreadable / uncertain
- **Real-VM evidence:** there are no PNGs under `runs/2026-11-24/patchbay/phase3/s3/`, and reading the `vm` entry failed with "Path … not found". Nothing on a real VM can be passed visually.
- **Day selected handle:** I can't confirm how readable the "1" on the navy handle is in `effects-five-day.png` at its original size.
- **Running state:** none of the shots shows a running chain, so I can't check how a running state or an engaged Bypass looks.

## Ranked defects
**Blocker**
- Bypass is a plain button with no on/off state in every shot. Nothing on screen tells you whether the EQ is bypassed.

**Major**
- In the failed shots, Enable is unchecked next to "failed", which sends mixed signals about the chain's state.
- The chain list reads as a button, not a selectable list. It doesn't match the Devices list or the mockup.
- In the list, a failed chain looks the same as a stopped one (same neutral dot). The failure is only tiny grey text.
- The failure reason is the raw "exit-code" token, and no retry or log action is shown.
- "Sends to" and "Preset" have no chevron, so they don't look like pop-ups (unlike Devices, Mixer and the mockup).
- The response curve is stepped/beaded, not smooth, and the axes have no units (dB, Hz).
- "1 chains" pluralisation error in the toolbar and status bar.
- "Studio EQ EQ chain" in the status bar.

**Minor**
- The selected band isn't shown in the table or the status bar, which the mockup does.
- Table numbers are left-aligned, decimals are mixed, gains have no "+", and the right side of the table is empty.
- Unclear labels: "Flat / saved" and "Effects menu".
- Naming varies between "New EQ", "Delete chain…" and "Delete EQ chain".
- The empty state says "No EQ chain selected" when no chains exist.
- The delete dialog breaks paths mid-token, and its destructive button isn't styled as destructive.
- Day: the Enable checkbox outline is faint and the grey fields are close to the panel colour.
- There is more space above CHAINS than above the Devices list.