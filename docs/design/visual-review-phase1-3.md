# Phase 1 visual review 3 (sekka-visual, anthropic/claude-opus-5-5, 2026-11-20)

Verbatim. Inputs: goldens at aizome-musubi `ea497e7` (code `9b85fb3`), VM shots `runs/2026-11-11/patchbay/phase1/s8/evidence/` (workspace), metadata `runs/2026-11-11/patchbay/phase1/s8/capture.json`.

## Images
- `repos/aizome-musubi/tests/snapshots/audio-studio-night.png`: This is the full window in Pairs mode with the title "Audio 46 ports · 4 links". The toolbar shows icon+text buttons for Monitors, Swap, Full Names and Undo (dimmed), plus From/To fields. The column groups are BT Headphon…, Built-in…, HDMI Audio, USB Studio "4 ports", EQ, Bitwig "Bitwig Main", Discord and OBS. Rows: Built-in Audio (now whole), USB Studio "4 ports", EQ, Bitwig, Firefox, Spotify and pw-cat. Connected cells are at USB AUX0×Discord and Firefox L·R×Built-in; the half cell is at USB AUX1×OBS. The sidebar counts and the two-line footer are present. Defects:
  - Column names are still cut: "BT Headphon…" and "Built-in…". The second is shorter than before.
  - Subtitle placement:
    - In Firefox and pw-cat the subtitle sits on its own line above its port row.
    - "Firefox Music Player" (y≈634) is 28 px from the L·R row above (606) and 24 px from its own row (658).
    - In Spotify and Bitwig the port row sits on the title line and the subtitle sits below it.
    - Empty grid bands sit beside the title and subtitle lines (y≈555–595, 620–645 and 728–765).
  - The last pw-cat row ends at the status bar (expected for scrolling).
- `…/audio-studio-day.png`: The same layout in Day. The half cell has a dark diagonal on light blue, and the legend swatch matches. Defects: the same name cuts and subtitle placement as Night. The rotated mono port labels are still mid-grey.
- `…/audio-collapsed-count-night.png`: Channels mode with USB Studio collapsed: the row reads "…–AUX3 ▸" and the column "…–AUX3 ▴". The count cells "1" with a blue underline sit at Discord MONO and OBS FL. Header boxes y≈58–170 read BT Headp… "2 ports", Built-in…, HDMI…, USB St…, EQ "EQ Capture…", Bitwig "Bitwig Main", Discord, and OBS "OBS Capture". Defects:
  - A band of about 25–35 px (y≈170–200) sits between the boxes and the port labels.
  - The row and column labels "…–AUX3" lose their start.
  - Names are still cut: "BT Headp…", "Built-in…", "HDMI…", "USB St…" and "EQ Capture…".
- `…/audio-collapsed-count-day.png`: The same in Day, and the "1" count cells read clearly. Defects: as Night.
- `…/audio-tooltip-night.png`: A tooltip box at x≈419–711, y≈596–618 reads "Firefox — Firefox Playback, Firefox Music Player". It is fully legible and uses an em dash and comma, not "·". Defects:
  - It covers the first Firefox L·R row of cells from Built-in to about AUX6, including the connected Firefox×Built-in cell.
  - It has about 50 px of empty width after the text.
- `…/audio-asked-night.png`: Channels mode, "5 links". At USB AUX0×AUX0 there is a dashed blue outline with a blue centre square inside the amber focus ring, and the row and column are tinted. The status bar reads "USB Studio · AUX0 → USB Studio · AUX0 · asked — waiting for PipeWire" in full. Defects: none for the state.
- `…/audio-asked-no-focus-night.png`: The same cell shows a dashed blue outline with a blue centre square, with no amber ring and no row/column tint. The status bar is empty. Defects: the "AUX0" row label is still bright and has a thin blue tick at its right edge (x≈409).
- `…/audio-asked-day.png`: The asked cell (dashed blue with centre square) sits inside an amber ring, the column is tinted, and the full status sentence is shown. Defects: none.
- `…/audio-asked-no-focus-day.png`: A dashed blue outline with a centre square on white, with no ring. Defects: the same row-label tick as Night.
- `…/audio-error-night.png`: The AUX0×AUX0 cell has an amber outer ring around a red/coral inner border with two red vertical bars inside. The status bar reads "Permission denied by PipeWire". Defects: none for the state. The amber ring and the red interior read as separate elements.
- `…/audio-error-no-focus-night.png`: A red outlined cell with red bars inside and no amber ring. The "Permission denied by PipeWire" status stays. Defects: the same row-label tick as the asked no-focus image.
- `…/audio-error-day.png`: An amber outer ring with a red inner border and red bars inside; the column is tinted. Defects: none.
- `…/audio-error-no-focus-day.png`: A red outlined cell with bars on white, and no ring. Defects: none beyond the row-label tick.
- `…/audio-studio-inspector-night.png`: The right panel reads INSPECTOR, "AUX0", "Audio · USB Studio". Under GOES TO are three rows: Built-in Audio · FL, Built-in Audio · FR and Discord · MONO, each with an Unlink button. FACTS uses human labels (Node, Port, Alias, Channel, Format, Device, Client, Serial) with mono values. The status bar reads "USB Studio · AUX0 → Built-in Audio · FL · connected — click to disconnect" in full. Defects: the inspected AUX0 row is marked only by a brighter "AUX0" label and a thin blue tick at x≈409. There is no row tint in the grid.
- `…/audio-drag-preview-night.png`: Light-blue preview rings sit at AUX0×AUX0 and AUX1×AUX1, and the AUX1 row and column are tinted. The status bar reads "USB Studio · AUX1 → USB Studio · AUX1 · not connected — click to connect" in full. Defects: none.
- `…/audio-drag-preview-day.png`: The same in Day; the blue rings are clear and the sentence is whole. Defects: none.
- `…/narrow-toolbar-night.png` (1040×800): The title reads "Audio · 4 links", then Matrix|List and Pairs|Channels. The Monitors, Swap, Full Names and Undo buttons are icon-only. The From/To fields are about 140 px wide with readable placeholders, and the status bar shows "46 ports" on the right. Defects:
  - The OBS FR column is cut at the right edge (scroll).
  - "Spotify Playback" is cut at the bottom edge.
  - The icon-only buttons have no visible text label.
- `…/narrow-toolbar-day.png`: The same in Day. Defects: as Night.
- `…/midi-night.png`: "MIDI 15 ports · 0 links".
  - Rows are whole: ALSA MIDI (other), Keystep, Launchkey, Virtual Raw MIDI 0-0 to 0-3, Midi Through, and Ardour "Ardour MIDI".
  - Column groups: Launchkey, "…MIDI 0-0" to "…MIDI 0-3", and Ardour.
  - Ports: MIDI 1, "…0-0" to "…0-3", and midi_in.
  - The legend has a "virtual" swatch.
  - Defects: the matrix uses about 200 px of the roughly 820 px body.
- `…/midi2-show-midi1-night.png`: Show MIDI 1.0 is checked.
  - Column groups: Launchkey, "…MIDI 0-0" to "…MIDI 0-3", Ardour and "MIDI 2 S…", each with a "1.0" badge under the name.
  - Rows are whole, with "1.0" badges in their own column, plus MIDI 2 Synth "MIDI 2 Synth".
  - Defects: "MIDI 2 S…" is cut.
- `…/video-night.png`: "Video 2 ports · 1 link", with one connected cell. Defects: the column name reads "…ra Viewer" (cut at its start) and the row port "…pture_1", although the header band has empty room to the right and above.
- `…/add-route-sheet-night.png`: A sheet with an amber top edge titled "Add Route".
  - The From search shows "USB" with a clear button; the To search shows the placeholder "Search destinations".
  - The first row of each list is whole and highlighted: USB Studio · AUX0 and BT Headphones · FL.
  - The To list runs from BT Headphones · FL to USB Studio · AUX3, with about 14 px left below.
  - Cancel and an enabled Connect button are shown. The status bar reads "From "USB" hides 19 ports".
  - Defects:
    - There is no scroll indicator.
    - The page behind the sheet (Add Route… button, list rows) shows no visible dimming.
- `…/unavailable-night.png`: "Audio" and "PipeWire isn't answering. Musubi keeps trying." are centred. The Pairs/Channels, Monitors, Swap, Full Names, Undo and List controls are dimmed. The server card is back in the footer: a hollow red ring, "PipeWire", "Defaults · unavailable". The status bar reads "private daemon stopped". Defects: the From/To fields still have the normal border and placeholder, so they look enabled.
- `…/midi-vm-bridge-night.png`: "MIDI 19 ports · 0 links". The rows are the same as midi-night.
  - Column groups (left to right): Launchkey, "…MIDI 0-0", "…MIDI 0-0", "…MIDI 0-1", "…MIDI 0-1", "…MIDI 0-2", "…MIDI 0-2", "…MIDI 0-3", "…MIDI 0-3", and Ardour.
  - Ports: MIDI 1, "…0-0", "VirMIDI…", "…0-1", "VirMIDI…", "…0-2", "VirMIDI…", "…0-3", "VirMIDI…", and midi_in.
  - Defects:
    - Each "…MIDI 0-n" group name appears twice side by side, with nothing visible to tell the two groups apart.
    - All four "VirMIDI…" port labels read identically.
- `…/midi-vm-bridge-day.png`: The same in Day. Defects: as Night.
- `runs/2026-11-11/patchbay/phase1/s8/evidence/01-audio-before.png` (VM 1280×800): A real Aizome session (Toolchest menu bar, window tab, shell panel).
  - Header: "Audio · 2 links", with an icon-only toolbar and From/To fields.
  - Columns: "…-vm-default" and "…-vm-patch", each "2 ports".
  - Row: pw-cat "/dev/zero". FL×FL and FR×FR on vm-default are connected.
  - Status bar: "10 ports".
  - Defects: none.
- `…/02-audio-linked.png`:
  - Filters: From "pw-" with a clear button; To "ıusubi-vm-patch" with a clear button.
  - One column, "…subi-vm-patch" "2 ports". The pw-cat FL × FL cell is filled blue with an amber ring, and the pointer sits on its lower right.
  - Counts: the header reads "3 links", the sidebar Audio count is 3, and Undo is enabled.
  - Status bar: "pw-cat · FL → musubi-vm-patch · FL · connected — click to disconnect" in full; on the right, "From "pw-" hides 0 ports".
  - Defects:
    - The first glyph of the To text ("m") is clipped by the search icon.
    - The column name loses "mu".
    - The right status names only the From filter and says it hides 0 ports.
- `…/03-audio-undo.png`: The same filters and the same FL × FL cell, now dark and empty inside the amber ring. "2 links", sidebar 2, Undo disabled, and the status bar reads "Undo applied.". Defects: the same To glyph clip and column cut as 02.
- `…/04-midi-before.png`: "MIDI · 0 links", a 6×6 empty grid, with the amber focus on the top-left cell and the pointer on the MIDI sidebar entry.
  - Column groups: "ALSA MID…", "…MIDI 0-0" to "…MIDI 0-3", and "Midi Thr…".
  - Ports: in, "…0-0" to "…0-3", and "Midi Th…".
  - Rows: ALSA MIDI (other) "out", Virtual Raw MIDI 0-0 to 0-3, and Midi Through "Midi Th…".
  - The status bar shows only "12 ports".
  - Defects: "ALSA MID…", "Midi Thr…" and "Midi Th…" are cut.
- `…/05-midi-linked.png`: Both filters read "/irtual Raw MIDI" with clear buttons. A 4×4 grid; the top-left cell (Virtual Raw MIDI 0-0 × "…MIDI 0-0") is filled with an amber ring, and the pointer is on its lower right. "MIDI · 1 link" and sidebar MIDI 1.
  - Status bar: "Virtual …MIDI 0-0 · VirMIDI 0-0 → Virtual …MIDI 0-0 · VirMIDI 0-0 · connected — click to disconnect"; on the right, "From "Virtual Raw MI…".
  - Defects:
    - The "V" of both filter texts is clipped by the search icon.
    - The right status is cut.
    - The pointer covers part of the cell.

## Checks
1. **VM sequence: pass.**
   - 02: the only remaining cell column's first cell (pw-cat FL × musubi-vm-patch FL) is filled. The count goes from 2 to 3 links (sidebar 2 → 3) and the status bar says "connected".
   - 03: the same cell is empty, with 2 links, sidebar 2, and "Undo applied.".
   - 05: the first VirMIDI row × first VirMIDI column cell is filled, with "1 link" and sidebar MIDI 1.
   - The pointer overlaps the filled cells, but the fill is visible around it.
2. **All cell states distinguishable at 1× in Night and Day: pass.**
   - Each state reads differently from the others in both themes:
     - empty: outline.
     - connected: solid fill.
     - half: diagonal, visible on Night and Day.
     - asked: dashed outline with a centre square.
     - error: red border with inner bars.
     - count: digit with an underline.
     - drag preview: light-blue ring.
     - focus: amber outer ring.
   - Error vs focus: the amber focus ring around an error cell is a separate outer edge around a red interior with bars. Plain focus on an empty cell (VM 03/04, Night) has a dark interior.
   - The asked and error no-focus goldens exist in both themes, and the states read without the ring.
   - Caveat: plain Day focus on an empty cell was not among my images (see Uncertain).
3. **Names: fail.**
   - No group name vanishes any more (VM 04, midi2-show-midi1 and collapsed-count all show names).
   - Siblings are not always distinguishable: `midi-vm-bridge-{night,day}` shows each "…MIDI 0-n" group name twice side by side, and the four "VirMIDI…" ports read identically.
   - Subtitles are still placed inconsistently: above the port row in Firefox and pw-cat, below a title-line port row in Spotify, Bitwig and Ardour. "Firefox Music Player" sits about midway between two rows.
   - Tooltip: sized better (about 50 px slack) and the separator changed, but it sits over the Firefox row's cells and hides a connected cell.
4. **Toolbar and status bar: fail.**
   - Title present: "Audio · 2 links" and "MIDI · 0 links" in the VM, "Audio · 4 links" in narrow-toolbar.
   - Hover sentences keep both ends with the "→": VM 02 and 05, drag-preview, inspector and asked.
   - The filters at VM width are mostly readable, but the first glyph is clipped by the search icon ("ıusubi-vm-patch" in 02/03, "/irtual Raw MIDI" in 05).
   - The right status is still cut in 05 ("From "Virtual Raw MI…").
5. **Add Route sheet, unavailable page and inspector look finished: pass.**
   - Add Route: first rows whole, first row of each list selected, Connect enabled.
   - Unavailable: centred, the card is back in the footer with a second line, and the controls are dimmed.
   - Inspector: human labels and a full status sentence.
   - Small leftovers: the From/To fields look enabled on the unavailable page, the sheet has no scroll indicator or dimmed background, and the inspected row has only a faint marker.
6. **Review 2 §7 defects:**
   1. VM 04 group boxes with no names: **resolved.** The names show: "ALSA MID…", "…MIDI 0-0" to "…MIDI 0-3", "Midi Thr…".
   2. midi2-show-midi1 "…· 1.0" columns and cut rows: **resolved.** Names and "1.0" badges are separate, and rows are whole. Only "MIDI 2 S…" is cut.
   3. Filter field text: **partly.** 02/03 To now reads almost whole ("ıusubi-vm-patch"), and 05 shows from "irtual" onward, but the first glyph is clipped. The right status in 05 is still cut.
   4. Hover sentences cut in the middle: **resolved.** The arrow and destination are present in 05, drag-preview and inspector.
   5. Stream subtitle placement and empty bands: **not resolved.**
   6. Collapsed-count header: **partly.** The names are longer (Discord, OBS, Bitwig whole) and the band shrinks to about 25–35 px, but "BT Headp…", "USB St…" and "HDMI…" are still cut, and "…–AUX3" still loses its start.
   7. Add Route first row, selection, scroll: **resolved for the cut row and selection.** There is still no scroll indicator.
   8. Night error vs focus: **resolved.**
   9. Remaining cut names: **partly.**
      - Now whole: "Built-in Audio" (row), "ALSA MIDI (other)", "Launchkey" and "EQ Playback Stream".
      - Still cut: "BT Headphon…", "Built-in…" (column), video "…ra Viewer" and "…pture_1", VM "…subi-vm-patch", "Midi Thr…" and "MIDI 2 S…".
   10. Unavailable card and fields: **partly.** The card is back in the footer with a second line, but the From/To fields still look enabled.
   11. Tooltip: **partly.** The separator is fixed, the slack is smaller, and it no longer covers the subtitle or label, but it still covers the row's cells, including a connected one.
   12. Inspector: **partly.** The labels are now human. The inspected row is marked only by a thin tick and a brighter label.
   13. Text-only toolbar, Swap, faint Day labels: **partly.**
       - Fixed: the wide toolbar is now icon+text.
       - Still open: the narrow toolbar is icon-only, and the Day rotated port labels are still mid-grey.
7. **Remaining defects, ranked by user impact:**
   1. `midi-vm-bridge-{night,day}.png` column header x≈490–740: pairs of identical group names ("…MIDI 0-0" twice, and so on) and four identical "VirMIDI…" ports. A user can't tell which bridge group is which. **Blocks** calling phase 1 visually done, since it is the same class of problem as review 2's #1/#2.
   2. Stream subtitle placement (audio goldens, Firefox y≈582–658, Spotify y≈690–707): the layout is inconsistent and the subtitle sits about midway between rows. Not blocking.
   3. `audio-tooltip-night.png` y≈596–618: covers the Firefox row's cells, including the connected Built-in cell. Not blocking.
   4. Filter first glyph clipped by the search icon (`02`/`03` To, `05` both fields), and the right status cut in `05`. Not blocking.
   5. Cut names: `video-night.png` "…ra Viewer" and "…pture_1" with spare header room; "BT Headphon…" and "Built-in…" (audio columns); "…subi-vm-patch"; "Midi Thr…"; "MIDI 2 S…". Not blocking.
   6. `audio-collapsed-count-*` "…–AUX3" losing its start, residual band and cut group names. Not blocking.
   7. `02`/`03` right status "From "pw-" hides 0 ports" while the To filter visibly hides the vm-default column. Not blocking.
   8. `unavailable-night.png` From/To fields look enabled. Not blocking.
   9. `audio-studio-inspector-night.png`: the inspected row is marked only by a thin tick at x≈409. The asked/error no-focus goldens carry the same tick. Not blocking.
   10. `add-route-sheet-night.png`: no scroll indicator and no visible background dimming. Not blocking.
   11. The MIDI matrices use a small part of the body width; the Day rotated port labels are faint; the narrow toolbar is icon-only. Not blocking.

## Unreadable / uncertain
- Plain Day focus on an empty cell isn't shown in any image I opened. Day focus was seen only around asked and error cells.
- I can't tell from the image whether the Add Route To list has more rows below "USB Studio · AUX3", because there is no scroll indicator.
- `final.png` was not opened. `capture.json` gives it the same sha256 as `05-midi-linked.png`.
- No `*nofocus*` or `*vm-like*` files exist in the snapshots folder. I reviewed the `*no-focus*` goldens (asked/error, Night/Day) and `midi-vm-bridge-{night,day}` in their place.
- Day variants of the tooltip, inspector, MIDI 2.0, video, Add Route and unavailable images weren't in my list, so I didn't open or judge them.