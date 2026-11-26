# Visual review — phase 4 (Setups)

Reviewer: `sekka-visual` (OMP, anthropic/claude-opus-5-5), 2026-11-25, goldens at `8ecc904` and VM screenshots from `runs/2026-11-25/patchbay/phase4/s4/evidence/` (Sekka contribution workspace). Recorded verbatim below.

I opened all 8 Setups goldens, the house-style references (`effects-five-night`, `effects-empty-night`, `effects-delete-night/day`, `effects-failed-day`, `devices-page-night`, `devices-routes-day`, `clock-forced-night`), and VM shots `09`, `10` and `02`. All images loaded. The pages look like they're built from the Aizome kit: same tokens, Night/Day, sidebar, panes and dialog style. Nothing looks like a pro-audio skin. The problems are labels, wording, type size and table layout.

## Images
- `setups-page-night.png` / `-day.png`: The header says "Setups · 1". The list has the "Studio" row selected with the caption "Inactive". The detail pane has the title, an Activate button, a Name field, Keep connected and Exclusive checkboxes, the line "Not needed", a ROUTES label and a 3-row table (Out → in / Status / Action) with Unpin buttons. "Delete setup…" is at the bottom left of the pane and "+ New setup from this page…" at the bottom of the list. The status bar says "1 setups". Defects: rows 1 and 2 are identical; route text is about 11px; wide empty gap in the table.
- `setups-empty-night/-day.png`: The list says "No setups yet". The detail heading says "No setup selected", with "Save the current page's links to make a setup." The status bar says "Setup deleted." and "0 setups". Header "Setups · 0". Defects: the heading doesn't fit the state; the guidance is unclear.
- `setups-delete-night/-day.png`: A "Delete setup" dialog at the top centre (x 441–839) with an amber top edge and dimmed background. Body: "Remove this setup and its file?" followed by a full /tmp/nix-shell… path broken over two lines. Buttons: Cancel and a red-outlined "Delete setup". Defects: the dialog doesn't name the setup; the path wraps in the middle.
- `setups-exclusive-night/-day.png`: An "Exclusive setup" dialog with the same placement. Body: "Exclusive removes other links on this setup's ports. WirePlumber may re-create its automatic links." Buttons: Cancel and a filled "Turn on Exclusive". Defects: the dialog's bottom edge (y≈155) cuts through the checkbox row, leaving 2–3px of both checkboxes visible at x≈466 and x≈601.
- `09-setup-kept.png`: Real session, 1044px-wide window inside the Aizome compositor. The header says just "Setups" with no count. The list row "VM keeper" has the caption "Active · kept" in accent blue. Deactivate button; Keep connected is checked. Keeper line: "Kept while Musubi is closed". 3 routes, all Connected. Status bar: "Setup saved." and "1 setups". Defects: route text ends at x≈748 and the Status column starts at x≈757; the mouse pointer sits on "Keep connected" (still readable).
- `10-setup-deleted.png`: Same empty state as the golden. The header says "Setups" with no count; the sidebar shows Setups 0; the status bar says "Setup deleted." and "0 setups". Defects: same wording problems as the golden.
- `02-audio-linked.png`: Audio matrix. Toolbar: "Audio · 3 links", Matrix/List, Pairs/Channels, 4 icons, 2 search fields. **No setup chip is visible**, and the sidebar shows Setups 0. Defects (not Setups): the second search field shows "ıusubi-vm-patch" with its first letter cut off at the magnifier icon; the column header is shortened to "…subi-vm-patch".
- References (house style): Effects headers name the noun ("Effects · 1 chain"). Effects and Devices show an Undo link in the header. List rows in both have a status dot. The Effects detail has a one-line explanation under the title and puts "Delete EQ…" at the top right. The Effects empty state says "No EQ chains yet / Create a new EQ to get started." The Effects delete dialog names the item ("Remove "Studio EQ" and its unit?") and lists the files in a block. Table cells and node rows use 13px body text.

## Blockers
1. **Two routes look the same but have different states.** `setups-page-*`, table rows 1 and 2 (y≈284 and y≈322) both read "Firefox · capture_FL → alsa_card.pci.analog · playback_FL". One says Connected, the other Waiting for output, and each has its own Unpin. You can't tell which one you're removing. This confirms the known issue. Raw node names are also in the real session (`09`: "musubi-vm-default", "musubi-vm-patch").
   - **Fix:** show the node description in 13px, with the port name muted after it.
   - **Fix:** if two routes still have the same label, add what tells them apart (device serial/ID or "(2)").
   - **Fix:** put the full raw names in a tooltip.

## Majors
1. **Route text is too small.** `setups-page-*`, `09`, table rows: route labels and status words are about 11px, smaller than the 13px table header above them. Effects band cells and Devices node rows use 13px. In Day this makes the coloured statuses weak.
   - **Fix:** use body size (13px) for route and status cells, as in the Effects BANDS table.
2. **Red and green status text in Day is weak.** `setups-page-day`, Status column, x≈993: "Ambiguous: 2 matches" is thin red on the lavender table fill, and "Connected" is thin dark green on the same fill. Both read noticeably weaker than the red in `effects-failed-day`, which is 13px on the page background. [INFERENCE: these look close to the 4.5:1 limit; I didn't measure pixels.] Night is fine: green, grey and pink-red all read well.
   - **Fix:** use 13px with the kit's danger/success text tokens, checked at ≥4.5:1 against the *table fill*.
   - **Fix:** optionally add a small status glyph, as in the Audio legend.
3. **Table columns don't follow content.**
   - In `setups-page-*` route text ends at x≈775 and Status starts at x≈993: about 220px of empty space.
   - In `09` (1044px wide) route text ends at x≈748 and Status starts at x≈757: 9px apart, so any longer label will collide or clip.
   - **Fix:** let the route column take the free space and shorten long labels with "…" (full text in a tooltip).
   - **Fix:** size Status and Action to their content and keep them at the right edge.
   - **Fix:** or split the route into separate Out and In columns so the table uses its width.
4. **Unpin has no safety net.** In `setups-page-*` each Unpin removes a saved route in one click. There's no confirmation and no Undo in the header, but Effects, Devices and Clock & Load all have "↶ Undo" next to the page title.
   - **Fix:** add the same header Undo to Setups (and mention it in the status bar, e.g. "Route unpinned · Undo").
5. **Delete dialog doesn't say which setup, and the path wraps badly.** `setups-delete-*`, dialog body: "Remove this setup and its file?" doesn't name the setup, and the dialog covers the title. The path wraps in the middle of a folder name ("…/aizome/" then "musubi/setups/studio.json").
   - **Fix:** follow the Effects wording: `Remove "Studio" and its file?`, then a blank line, `File in <dir>:` and `studio.json` on its own line.
   - Placement, the amber top edge, the dimmed background and the red-outlined destructive button already match Effects.
6. **Empty state heading and guidance are wrong.**
   - In `setups-empty-*` and `10` (detail pane, y≈76–100) the heading "No setup selected" suggests there are setups to choose from. "Save the current page's links" (and the button "New setup from this page…") is unclear when the current page *is* Setups, which has no links.
   - **Fix:** follow Effects: "No setups yet" / "A setup remembers links so Musubi can restore them. Connect things on Audio, MIDI or Video, then save them as a setup."
   - **Fix:** if the button uses the last patchbay page, name it in the label ("New setup from Audio…").
7. **Keeper line has no context.** `setups-page-*` at y≈180: "Not needed" stands alone under the checkboxes. This confirms the known issue. In `09`, "Kept while Musubi is closed" shows that the active state works.
   - **Fix:** prefix it ("Keeper: …") and give the off state a reason, e.g. "Keeper: off — turn on Keep connected to hold these links while Musubi is closed."

## Minors
1. **Wrong plural in the status bar.** "1 setups" appears in `setups-page-*`, `-delete-*`, `-exclusive-*` and `09` (status bar, right). Confirmed. Fix: "1 setup".
2. **Header count is missing or incomplete.** The Harness header reads "Setups · 1" / "Setups · 0" with no noun. The real session (`09`, `10`) shows just "Setups" with no count, while `02` shows "Audio · 3 links". Fix: show "Setups · 1 setup" in both builds, like "Effects · 1 chain" and "Devices · 3 devices".
3. **List rows lack the status dot and full state.** `setups-page-*` list row (x 215–432) has no status dot; Effects (x≈425) and Devices (x≈479) both have one. The caption shows only "Inactive" or "Active · kept". No image shows how an exclusive or route-count caption looks.
   - **Fix:** add the same filled/hollow dot for Active/Inactive.
   - **Fix:** use a caption like "Active · kept · exclusive · 3 routes".
   - The accent-blue caption in `09` differs from Effects' muted grey (Effects only colours the caption red for failures); use the muted tone or the dot instead.
4. **No explanation under the title.** `setups-page-*` has nothing under "Studio"; Effects has "A virtual output: …". Fix: add one line, e.g. "Links Musubi restores when this setup is active."
5. **"Name" label is styled differently.** `setups-page-*` at x=458 uses body size; the Effects "Name" label is small and muted. Fix: use the Effects label style.
6. **Table headers don't line up with cells.**
   - "Out → in" header is at x=466, row text at x=470.
   - "Status" header is at x=997, cells at x=993.
   - "Action" header is at x=1195, buttons at x=1191.
   - Fix: share one cell padding. The "Action" header could also be dropped, since every row has the same single button.
7. **Rows aren't sorted.** `09` lists output_FR before output_FL. Fix: sort by node, then channel.
8. **Bottom buttons are 2px apart.** The "Delete setup…" button (y 766–789) sits 2px higher than "New setup from this page…" (y 768–791), both in the goldens and in `09`. Its position also differs from Effects, where "Delete EQ…" is at the top right next to the checkboxes. Fix: line up the button edges, or move Delete to the Effects position.
9. **Exclusive dialog cuts through the checkbox row.** In `setups-exclusive-*` the dialog's bottom edge leaves checkbox slivers showing. The wording opens with jargon and has no warning cue.
   - **Fix:** make the dialog clear the row (or cover it fully).
   - **Fix:** rephrase: `Turn on Exclusive for "Studio"? While it's active, Musubi disconnects any other links on these ports. WirePlumber may reconnect its automatic links.`
   - Keep "Turn on Exclusive" as the filled primary button; it reads clearly.
10. **Clipped search text** (Audio page, outside Setups scope). In `02` the second search field cuts off its first letter under the icon.

## Checks
- **List shows Active / kept / exclusive:** uncertain. Inactive (goldens) and "Active · kept" (`09`) are visible. No image shows an exclusive or active state in the Harness.
- **Detail pane has name, Activate/Deactivate, Keep connected, Exclusive and keeper line:** pass. All are present; Activate is in the goldens and Deactivate in `09`. The keeper line wording is a major issue (#7).
- **Exclusive warning dialog:** pass. Visible in Night and Day; minor issues only.
- **Route table, out → in, status in words, Unpin:** fail. Connected, Waiting for output and "Ambiguous: 2 matches" all appear in words, but two rows can't be told apart (Blocker 1).
- **Delete with destructive confirmation:** pass for the mechanism (red-outlined confirm, same as Effects); the wording issues are Major 5.
- **"New setup from this page…" present:** pass. Its wording is unclear (Major 6).
- **Hierarchy and alignment:** fail. Route text is smaller than the header, and headers and cells are offset by 4px.
- **Matches Effects and Devices:** fail. No Undo, no status dot, header count and noun differ, Delete is in a different place.
- **Night contrast:** pass.
- **Day contrast:** fail (red/green status text, see Major 2; [INFERENCE] on exact ratios).
- **Dialog placement:** pass. Same as Effects.
- **Dialog wording:** fail for Delete; pass with minors for Exclusive.
- **Empty-state guidance:** fail.
- **Table column use:** fail. Wide empty gap at 1280px, near collision at 1044px.
- **Selected list row and caption match other pages:** pass for geometry and fill (x 215 start, same blue fill and caption position); the missing status dot is Minor 3.
- **Truncation:** pass in the Setups images. `09` is at the edge, and `02` has the clipped search field.
- **Anything un-Aizome:** pass. Nothing found.
- **Setup chip in the matrix toolbar (`02`):** uncertain. No chip is visible, but the sidebar shows Setups 0 at that moment, so its absence may be correct.

## Unreadable / uncertain
- Contrast ratios are judged by eye only; I didn't measure pixels.
- Keyboard focus: no focus ring is visible in any image, so I can't tell which dialog button has default focus.
- Exclusive-on state and the Activate → Active change aren't captured in any Harness golden.
- The setup chip in the matrix toolbar doesn't appear in any provided image.

## Per-image verdict
| Image | Verdict | Reason |
|---|---|---|
| `setups-page-night.png` | fail | Blocker 1; Majors 1, 3, 4, 7 |
| `setups-page-day.png` | fail | Blocker 1; Majors 1–4, 7 |
| `setups-empty-night.png` | fail | Major 6 |
| `setups-empty-day.png` | fail | Major 6 |
| `setups-delete-night.png` | fail | Major 5 |
| `setups-delete-day.png` | fail | Major 5 |
| `setups-exclusive-night.png` | pass | The dialog is right; the background shows the page issues already counted above; Minor 9 |
| `setups-exclusive-day.png` | pass | Same as Night; Minor 9 |
| `09-setup-kept.png` | fail | Major 3 (near collision); Blocker 1 (raw names); Minors 1, 2, 7 |
| `10-setup-deleted.png` | fail | Major 6; Minor 2 |
| `02-audio-linked.png` | uncertain | No setup chip visible while there are 0 setups; Minor 10 |
| `effects-*`, `devices-*`, `clock-forced-night.png` | pass | Used as house-style references |