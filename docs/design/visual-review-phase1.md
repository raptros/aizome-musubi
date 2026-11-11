# Phase 1 visual review (sekka-visual, anthropic/claude-opus-5-5, 2026-11-11)

Verbatim. Inputs: goldens at aizome-musubi `a5284ec`, VM screenshots `runs/2026-11-11/patchbay/phase1/s5b/evidence/` (workspace), metadata `runs/2026-11-11/patchbay/phase1/capture.json`.

## Images
- `runs/2026-11-11/patchbay/phase1/s5b/evidence/01-audio-before.png` (1280×800, VM): The Audio page shows "Audio 10 ports · 2 links". There is one row group, pw-cat (FL, FR), and two column groups that are both labelled "musubi-vm-…", each with FL and FR. Two cells are filled: pw-cat FL × group 1 FL and pw-cat FR × group 1 FR. Group 2's cells are empty. The filters are empty, Undo is disabled, the status bar reads "10 ports" and the Aizome shell panel is on the right. Defects: the two column group names are identical after truncation, so they can't be told apart.
- `…/02-audio-linked.png`: The From filter is "pw-" and the To filter shows "ubi-vm-patch" with its start scrolled out of view. One column group "musubi-vm-…" (FL, FR) remains. The cell pw-cat FL × FL is filled blue with an amber focus ring, and the pointer sits over it. The header reads "3 links" and Undo is enabled. The status bar reads "pw-cat · FL (output_FL) → musubi-vm-patch · FL (playback_FL) · connected — click to disconnect" and "From "pw-" hides 0 ports". Defects: the column name is cut; the To field hides the start of its text; the pointer covers part of the cell.
- `…/03-audio-undo.png`: Same filters as 02. Cell FL × FL is dark and empty with the amber focus ring still on it. The header reads "2 links", Undo is disabled and the status bar says "Undo applied.". Defects: same truncation as 02.
- `…/04-midi-before.png`: The MIDI page shows "MIDI 12 ports · 0 links" in a 6×6 grid, all empty, with the focus ring on the top-left cell. Rows: "ALSA MIDI (ot…" out, four rows "Virtual Raw MI…" / "VirMIDI…", and "Midi Through" / "Midi Th…". Columns: "ALSA MIDI (…" in, four "Virtual Raw…" columns with ports VirMIDI 0-0 to 0-3, and "Midi Through" Port-0. The legend includes hardware and virtual. Defects: the four Virtual Raw MIDI rows can't be told apart (name and port are both truncated), and the status bar still says "Undo applied." from the Audio page.
- `…/05-midi-linked.png`: Both filters show "ual Raw MIDI" with their starts scrolled out of view. The grid is 4×4. The top-left cell (first row × VirMIDI 0-0) is filled blue with the focus ring and the pointer on it. The header reads "1 links". The status bar reads "Virtual Raw MIDI 0-0 · VirMIDI 0-0 (Virtual Raw MIDI 0-0:VirMIDI 0-0 (capture)) → Virtual Raw MIDI 0-0 · VirMIDI 0-0 (V…  …". Defects: the status sentence is cut before the destination port and the state; the row labels are truncated; "1 links" is ungrammatical.
- `repos/aizome-musubi/tests/snapshots/audio-studio-night.png` (1280×824): Menu bar, sidebar, toolbar (Matrix|List, Pairs|Channels, Monitors, Swap, Undo, From/To). Column groups: BT Headpho…, Built-in Audio, HDMI Audio, USB Studio, EQ, Bitwig, Discord, OBS, with amber/violet/blue stripes. Row groups: Built-in Audio, USB Studio, EQ, Bitwig, Firefox, Spotify. The legend sits in the corner. Cells: connected at USB AUX0×Discord and Firefox×Built-in; half at USB AUX1×OBS, with the diagonal visible. Defects: "BT Headpho…" is truncated; the icons at the foot of the column labels touch the text (EQ, Built-in, HDMI); there is an empty band of about 30 px between the group boxes and the port labels.
- `…/audio-studio-day.png`: Same layout in Day. The half cell at USB AUX1×OBS looks like a solid blue square at 1×, and the legend's "half" swatch also looks solid. Defects: half can't be told from connected; the rotated port labels are faint grey on light grey.
- `…/audio-studio-channels-night.png`: Channels mode with FL/FR columns. Connected cells: Firefox FL→Built-in FL, FR→FR, USB AUX0→Discord, AUX1→OBS FL. Defects: "BT Headpho…" truncated; icon/text contact on the column labels.
- `…/audio-studio-swapped-night.png`: The axis captions read "TO ↓ / FROM →". Sinks are rows and app sources are columns. Connected cells: Built-in L·R × Firefox and Discord × USB AUX0; half: OBS × USB AUX1. Defects: none beyond the shared header ones.
- `…/audio-studio-filtered-night.png`: From = "USB" with a clear button. Only the USB Studio rows remain. The status bar reads "From "USB" hides 13 ports". Defects: none.
- `…/audio-studio-inspector-night.png`: A right panel shows INSPECTOR, "AUX0", "Audio · USB Studio". Under GOES TO there are three rows (Built-in Audio · FL, Built-in Audio · FR, Discord · MONO), each with an Unlink button. FACTS lists label/value pairs with mono values: alsa_input.usb_studio, capture_AUX0, 12:capture_AUX0, AUX0, 32 bit float mono audio, USB Studio, —, 10100. The status bar holds the hover sentence. Defects: the GOES TO and FACTS captions are indented about 11 px from INSPECTOR and from the list rows; the AUX0 row in the matrix isn't marked as the inspected port.
- `…/audio-drag-preview-night.png` (1280×800, **no menu bar**): Channels mode. Two diagonal cells (USB AUX0×USB AUX0 and AUX1×AUX1) have a light-blue ring that is larger than a cell, and the row and column are tinted. The status bar says "…not connected — click to connect". Defects: the menu bar is missing, which doesn't match the 1280×824 metadata.
- `…/audio-error-night.png` (1280×800, **no menu bar**): Cell USB AUX0×USB AUX0 has an orange-red edge, and its row and column are tinted. The status bar says "Permission denied by PipeWire". Defects: the menu bar is missing; the error edge looks close to the amber focus ring seen in the VM shots.
- `…/midi-night.png`: "MIDI 7 ports · 0 links". Columns: Launchkey (MIDI 1), Ardour (midi_in). Rows: ALSA MIDI (ot…, Keystep, Launchkey, Midi Through, Ardour. All cells are empty. Defects: "ALSA MIDI (ot…" is truncated; the matrix uses only about 50 px of a roughly 890 px body.
- `…/midi-day.png`: Same in Day. Empty-cell edges are visible on near-white and the stripes are dark amber, cyan and blue. Defects: "ALSA MIDI (ot…" truncated; the rotated port labels are faint.
- `…/midi2-show-midi1-night.png`: Show MIDI 1.0 is checked. Columns: "Launchkey · …", "Ardour · 1.0", "MIDI 2 Synth". Rows carry a small "1.0" badge: ALSA MID…, Keystep, Launchkey, Midi Thro…, Ardour, MIDI 2 Synth (no badge). Defects: "Launchkey · …" hides the 1.0 tag; "ALSA MID…" and "Midi Thro…" are truncated; the badges are squeezed against the port labels.
- `…/midi2-confirm-night.png`: A sheet drops from the toolbar with an amber top edge and the background dimmed. Title "MIDI conversion", text "PipeWire converts between MIDI 2.0 and MIDI 1.0. Some messages don't survive the conversion.", buttons Cancel and Connect (default, blue). Defects: none visible. The sheet covers the List segment and the column headers.
- `…/video-night.png`: "Video 2 ports · 1 links". Column "Camera Vie…" (input_1), row USB Camera "capture…", one connected cell. Defects: "Camera Vie…" and "capture…" are truncated; "1 links".
- `…/list-filtered-night.png`: List mode. Add Route… and Delete Selected (disabled) buttons, and a table From / To / State with a sort chevron on From. Two rows: USB Studio · AUX1 → OBS · FL and USB Studio · AUX0 → Discord · MONO, both Connected. Defects: none; no row is selected.
- `…/list-filtered-day.png`: Same in Day, clean and readable. Defects: none.
- `…/add-route-sheet-night.png`: Sheet titled "Add Route" with From and To columns, each with a search field and a list. From shows USB Studio · AUX0 to AUX3; To shows BT Headphones · FL through EQ · FL, with "EQ · FR" cut off at the bottom. Cancel; Connect is disabled. Defects: list text sits flush on the left border of both list boxes with no inset; the last row is half cut and no scrollbar is visible; the From list is filtered to USB while its search field shows only the "Search sources" placeholder; nothing is selected.
- `…/empty-page-night.png`: Video page, "0 ports · 0 links". One centred line: "No video ports. Cameras appear here when PipeWire's camera support finds them." Defects: none. Swap and Matrix/List look active with nothing to act on.
- `…/unavailable-night.png`: Audio page with the heading "Audio" and the line "PipeWire isn't answering. Musubi keeps trying." placed top-left. The footer shows "PipeWire" with a hollow red ring. The status bar says "private daemon stopped". Defects: the message is top-left, while the empty page centres its message; toolbar controls (Pairs/Channels, Monitors, Swap) look enabled.
- References I opened: Settei `sound-night.png`, Yukimi `storage-night.png`, Tansu `system-disk-night.png`, and the mockups `v2-audio-phase1-night.png` and `v2-list-phase1-night.png`. The design doc has §2.

## Checks
1. **VM state changes: pass.** In 02 the pw-cat FL × FL cell is filled, the count goes to 3 links and the status bar names "musubi-vm-patch · FL (playback_FL) · connected". In 03 the same cell is dark and empty, the count is 2 links and the status bar says "Undo applied.". In 05 the top-left cell is filled, the count is 1 links and the status bar names Virtual Raw MIDI 0-0 VirMIDI 0-0 → Virtual Raw MIDI 0-0 VirMIDI 0-0. Two caveats:
   - 02, 03 and 05 also have filters typed in, which the expected sequence doesn't mention.
   - In 05 the row's own label doesn't show "0-0"; only the status bar identifies it.
2. **Cell states distinguishable at 1× in Night and Day: fail.**
   - Day half state: in `audio-studio-day.png` the USB AUX1×OBS cell and the legend's "half" swatch both read as solid blue. I can't see the diagonal; only a slightly different blue separates it from connected (Discord cell).
   - Asked and collapsed count: no golden shows them in a cell. They appear only as legend swatches, and the Night "count" swatch is a tiny "2" with a bar.
   - Error: the orange-red edge in `audio-error-night.png` is close to the amber focus ring in VM 02/03.
   - Night-only states: the drag-preview and error goldens are Night only, so Day is unshown for those states.
   - What does work: Night empty, connected and half (diagonal visible) are clear, and the drag preview's light-blue ring is distinct from empty.
3. **Group names and port labels readable: fail.** The port labels themselves are readable. Truncations that lose meaning:
   - VM 01: two column groups are both "musubi-vm-…".
   - VM 04/05: four rows read "Virtual Raw MI…" / "VirMIDI…", so they can't be told apart.
   - VM 04: "Midi Th…" port label.
   - VM 05: the status bar sentence ends "(V…  …", losing the destination port and state.
   - VM 02/03/05: the filter fields show "ubi-vm-patch" and "ual Raw MIDI" with their starts hidden.
   - `midi2-show-midi1-night`: "Launchkey · …" loses the 1.0 tag; "ALSA MID…" and "Midi Thro…".
   - `video-night`: "Camera Vie…" and "capture…".
   - Less serious: "ALSA MIDI (ot…" (midi-night/day, VM 04) and "BT Headpho…" (audio goldens).
4. **Looks like an Aizome app: pass.**
   - Matches the references: the palette (navy surfaces, blue selection, amber accents), square 1pt-edged controls, the mono caption "PATCHBAY" and mono status bar, the app card (tile, name, 結び · Patchbay) like Yukimi's, and a footer server card with a status dot.
   - Gaps against the mockup and references: the sidebar has no mono link counts; the footer has no second line (rate · quantum · latency) and the goldens read "PipeWire fake PipeWire"; toolbar actions are text-only where Tansu uses icon+text.
5. **List view, Add Route sheet, MIDI 2.0 alert, empty and unavailable pages look finished and native: fail.**
   - List view (Night and Day): pass. A proper table with header, sort chevron and buttons.
   - MIDI 2.0 alert: pass. Dimmed sheet, clear text, default button.
   - Empty page: pass.
   - Add Route sheet: fail. List text has no inset from the border, the last row is cut with no scrollbar, and the From list is filtered while its search field looks empty.
   - Unavailable page: uncertain. The message is top-left rather than centred like the empty page, and the toolbar controls still look enabled.
6. **Inspector: pass.** Labels and mono values are readable, and no text both wraps and ellipsizes. The Unlink buttons are bordered, filled and look like buttons. One minor flaw: the GOES TO and FACTS captions are indented about 11 px from the content.
7. **Day theme finished, enough contrast: pass.** The Day goldens (`audio-studio-day`, `midi-day`, `list-filtered-day`) look complete: empty-cell edges hold on near-white, the stripes are darkened, and the selected segment and sidebar item are strong. Weak spots: the rotated mono port labels and legend text are faint grey on light grey, and the half state reads as solid blue (check 2).
8. **Concrete visual defects: fail.** Defects found:
   - `audio-drag-preview-night`, `audio-error-night`: 1280×800 with no menu bar, unlike the other goldens.
   - Audio goldens, column header: icons at the label foot touch the text (EQ, Built-in Audio, HDMI Audio); an empty band at about y 170–200 sits between the group boxes and the port labels.
   - `add-route-sheet-night`, list boxes at x≈338 and x≈646: text has no inset; "EQ · FR" is clipped at y≈450; the empty From search field doesn't match the filtered list.
   - VM 04, status bar: stale "Undo applied." on the MIDI page.
   - VM 05, status bar: sentence cut off with "…".
   - `midi2-show-midi1-night`, rows x≈300–322: "1.0" badges are cramped against the names and port labels.
   - `unavailable-night` and `empty-page-night`: the two messages are placed inconsistently (top-left heading vs centred line).
   - Toolbar in all matrix goldens: the Matrix|List and Pairs|Channels segmented controls are only about 4 px apart; in Night, Swap's filled look reads like a pressed or selected state next to the outlined Undo.
   - Inspector golden: section captions are misaligned with the content.

## Top 8 visual improvements (ranked)
1. **Make MIDI rows and VM columns identifiable** (VM 04/05 row header, x≈205–388; VM 01 column headers "musubi-vm-…"). Keep the distinguishing part of the name visible ("…MIDI 0-0", "…-patch"), or show it in the port column.
2. **Draw the half diagonal visibly in Day** (`audio-studio-day`: USB AUX1×OBS cell and the legend's "half" swatch).
3. **Separate the error edge from the amber focus ring** (`audio-error-night` USB AUX0×AUX0 cell vs VM 02/03 focus ring). Add goldens showing asked, collapsed-count and error cells in both themes.
4. **Finish the Add Route sheet** (`add-route-sheet-night` lists): inset the list text, show a selected row and a scroll indicator, avoid the half-cut last row, and show in the From search field that the list is filtered.
5. **Fix the status bar and filter fields in the VM shots**: don't cut the hover sentence before the destination and state (VM 05), clear the stale "Undo applied." when the page changes (VM 04), and widen the filter fields or keep the start of the text visible (VM 02/03/05).
6. **Tighten the column header** (audio goldens, y≈170–228): remove the empty band between the group boxes and the port labels, and leave a gap between the foot icon and the group name.
7. **Bring the sidebar up to the mockup** (all goldens, left column): add mono link counts per page, and a second footer line with rate · quantum · latency instead of "PipeWire fake PipeWire".
8. **Make the unavailable page consistent with the empty page** (`unavailable-night` content area): centre the message, and show the toolbar controls as disabled when PipeWire is unreachable.

## Unreadable / uncertain
- Day variants of drag preview, error, inspector, channels, swapped, filtered, MIDI 2.0, video, Add Route, empty and unavailable are listed in `capture.json` but were not among the images given to me, so I did not judge them.
- I could not see a diagonal in the Day half cell at 1×; I'm reporting what is visible, not measured colours.
- Whether the error edge colour differs measurably from the focus ring: I can't tell by eye at 1×.