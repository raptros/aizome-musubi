# Phase 1 visual review 2 (sekka-visual, anthropic/claude-opus-5-5, 2026-11-20)

Verbatim. Inputs: goldens at aizome-musubi `3eb1d67` (code `16913da`), VM shots `runs/2026-11-11/patchbay/phase1/s7/evidence/` (workspace), metadata `runs/2026-11-11/patchbay/phase1/s7/capture.json`.

## Images
- `repos/aizome-musubi/tests/snapshots/audio-studio-night.png`: The full window has a menu bar, sidebar, toolbar, matrix and status bar. The sidebar shows mono counts (Audio 4, MIDI 0, MIDI 2.0 0, Video 0) and a two-line footer: "PipeWire 1.6.9" and "Defaults · 48 kHz · 1024 · 21.3 ms". The toolbar has Matrix|List, Pairs|Channels, Monitors, Swap, Full Names and Undo (disabled), plus From/To fields. Column groups are BT Headphon…, Built-in Au…, HDMI Audio, USB Studio "4 ports", EQ, Bitwig "Bitwig Main", Discord and OBS. The group boxes now run straight into the port labels with no empty band, and the foot icons have clear space above them. Rows have stream subtitles: Firefox has "Firefox Playback" and "Firefox Music Player"; pw-cat has "First pw-cat Player" and "Second pw-cat Player"; Spotify has "Spotify Playback". Connected cells are USB AUX0×Discord and Firefox Playback×Built-in. USB AUX1×OBS is a half cell with a visible diagonal. Defects:
  - Some names are still cut: "BT Headphon…" and "Built-in Au…" (columns) and "Built-in Au…" (row).
  - Groups with several streams (Firefox, pw-cat) put each subtitle on a line above its port row. The gap from the previous port row and the gap to the next one are both about 20 px, so "Firefox Music Player" (y≈622) sits as close to the L·R row above it (602) as to its own row (642).
  - Single-stream groups (Spotify, Bitwig) do the opposite: the subtitle sits under a port row that shares the title line.
  - Empty grid bands sit beside the group-title lines (Firefox y≈555–590, pw-cat y≈710–745).
  - The last pw-cat L·R row is cut by the status bar (expected for scrolling).
- `…/audio-studio-day.png`: The same layout in Day. The half cell at USB AUX1×OBS is light blue with a dark diagonal, and the legend's "half" swatch also shows the diagonal. Stripes are dark amber, violet and blue. Defects: the same name cuts and subtitle placement as Night; the rotated mono port labels are still faint grey.
- `…/audio-streams-night.png`: Identical to `audio-studio-night.png` by eye, and `capture.json` gives the same sha256 (67f3b3…). Defects: as audio-studio-night.
- `…/audio-streams-day.png`: Identical to `audio-studio-day.png` (same sha256, 6f6d7a…). Defects: as audio-studio-day.
- `…/audio-full-names-night.png`: Full Names is checked. The header bands are taller and wider. Names are whole: BT Headphones, Built-in Audio, HDMI Audio, USB Studio, EQ, Bitwig/Bitwig Main, Discord, OBS, and the row "Built-in Audio". Port labels moved right (x≈360). Defects: none.
- `…/audio-tooltip-night.png`: A tooltip at y≈580–614 reads "Firefox · Firefox Playback · Firefox Music Player", fully legible. Defects:
  - The tooltip covers the "Firefox Playback" subtitle, the first Firefox L·R label and the first two cells.
  - Its box is about 110 px wider than the text.
  - The app/stream separator "·" is the same one used between group and port elsewhere ("Built-in Audio · FL"), so the line reads like a group·port path.
- `…/audio-asked-night.png`: Channels mode, "5 links". The USB AUX0 × USB AUX0 cell has a small blue centre square inside a dashed outline, with the amber focus ring around it, and its row and column are tinted. The status bar reads "… · asked — waiting for PipeWire". Defects: the asked state is only shown together with focus, so asked without focus is unseen. "EQ Playback Str…" is cut, and "pw-cat" is cut at the bottom edge.
- `…/audio-asked-day.png`: The same in Day. The dashed blue centre mark and the amber ring are both clear. Defects: as Night.
- `…/audio-error-night.png`: The USB AUX0 × AUX0 cell has a thick coral/salmon ring with a tiny tick in its top-right corner. The status bar reads "Permission denied by PipeWire". Defects: at 1× the error edge and the focus ring merge into one warm ring. Only its hue (coral instead of amber) and a 2–3 px tick set it apart from the plain amber focus ring in VM 02/03.
- `…/audio-error-day.png`: A red inner border with an amber outer ring and a corner tick. Red and amber read as two separate edges. Defects: none for the state itself.
- `…/audio-collapsed-count-night.png`: The USB Studio row is collapsed to "…–AUX3 ▸" and the USB Studio column to "AUX0–AUX3 ▴". Count cells show "1" with a blue underline at Discord MONO and OBS FL. Defects:
  - The column group boxes are shrunk (y≈58–148) with heavy cuts: "BT He…", "Built…", "HDMI …", "USB S…", "EQ Ca…", "Bitwig Bitwig…", "Disco…", "OBS C…".
  - An empty band of about 50 px (y≈150–200) sits between the boxes and the port labels.
  - The row label "…–AUX3" loses its start.
- `…/audio-collapsed-count-day.png`: The same in Day. The count "1" cells read clearly. Defects: the same header cuts, empty band and row label as Night.
- `…/audio-drag-preview-night.png`: Now 1280×824 with the menu bar. The USB AUX0×AUX0 and AUX1×AUX1 cells have a light-blue ring, and the AUX1 row and column are tinted. The status bar reads "USB Studio · AUX1 (capture_AUX1)…SB Studio · AUX1 (playback_AUX1) · not connected — click to connect". Defects: the status sentence is cut in the middle, losing the "→" and the start of the destination name.
- `…/audio-drag-preview-day.png`: The same in Day. The blue preview rings are clear on white. Defects: the same middle-cut status sentence.
- `…/audio-studio-inspector-night.png`: The right panel shows INSPECTOR, AUX0, "Audio · USB Studio". The GOES TO and FACTS captions are now aligned with the content. There are three Unlink rows, and the FACTS label/value pairs have mono values. Defects:
  - The status bar is cut in the middle: "(capture_AUX0)…Built-in Audio".
  - The AUX0 row in the matrix is not marked as the inspected port.
  - Labels mix human names ("Node name") with raw keys ("format.dsp", "object.serial").
- `…/midi-night.png`: "MIDI 15 ports · 0 links". The rows "…MIDI 0-0" to "…MIDI 0-3" with ports "…0-0" to "…0-3" can be told apart. The columns "…0-0" to "…0-3" have full port labels, VirMIDI 0-0 to 0-3. Defects:
  - "Launc…" (column) and "ALSA MIDI (…" (row) are cut.
  - The matrix uses about 200 px of the roughly 890 px body.
- `…/midi-day.png`: The same in Day, clean, with readable stripes. Defects: as Night; the rotated port labels are faint.
- `…/midi2-show-midi1-night.png`: Show MIDI 1.0 is checked. The "1.0" badges now sit in their own column (x≈296–316) and are no longer cramped. Defects:
  - Six column group names read "…· 1.0" and the seventh reads "MIDI…". No column group name can be identified; only the port labels (MIDI 1, VirMIDI 0-0 to 0-3, midi_in, ump_in) differ.
  - Rows are cut harder: "ALSA …", "Keyst…", "Launc…", "…0-0" to "…0-3", "Midi …".
- `…/video-night.png`: "Video 2 ports · 1 link" (the grammar is fixed). There is one connected cell. Defects: "Camera Vi…" (column) and "capture…" (row port) are still cut, although the header band has spare vertical room.
- `…/add-route-sheet-night.png`: A sheet with an amber top edge, titled "Add Route". The From search shows "USB" with a clear button; the To search shows the placeholder "Search destinations". List text is now inset about 8 px. From lists USB Studio · AUX0 to AUX3. To lists BT Headphones · FL/FR, Built-in Audio · FL/FR, HDMI Audio · FL/FR and USB Studio · AUX0 to AUX3. Cancel is shown and Connect is disabled. The status bar reads "From "USB" hides 19 ports". Defects:
  - The first row in both lists sits jammed against or cut by the top edge of the list box (y≈156–168), as if the lists were scrolled by about half a row.
  - No row is selected.
  - No scrollbar or scroll indicator.
- `…/list-filtered-day.png`: List mode with From "USB" (and its clear button). Add Route… and Delete Selected (disabled). A From/To/State table with a sort chevron and two Connected rows. Defects: none. The Pairs|Channels control is still shown in List mode.
- `…/empty-page-night.png`: The Video page reads "0 ports · 0 links" with a centred line, "No video ports. Cameras appear here when PipeWire's camera support finds them." Swap, Full Names, Undo and List are dimmed. Defects: none.
- `…/unavailable-night.png`: The heading "Audio" and the line "PipeWire isn't answering. Musubi keeps trying." are centred as a block. Pairs, Monitors, Swap, Full Names and Undo are dimmed. The status bar reads "private daemon stopped". Defects:
  - The PipeWire server card (hollow red ring) has moved up under the navigation (y≈308) instead of sitting in the bottom footer, and it has no second line.
  - The From/To fields and the selected Matrix/Channels segments still look active.
- `runs/2026-11-11/patchbay/phase1/s7/evidence/01-audio-before.png` (VM, 1280×800): A real Aizome session with the shell panel on the right. "Audio 10 ports · 2 links". The columns "…-vm-default" and "…-vm-patch" (2 ports each) can be told apart. The row is pw-cat with the subtitle "/dev/zero". FL×FL and FR×FR are connected on the vm-default group. Defects: none.
- `…/02-audio-linked.png`: From "pw-", and the To field shows only "h" (the field is about 55 px wide). One column, "musubi-vm-pat…". The cell pw-cat FL × FL is filled blue with the amber focus ring and the pointer on it. The header reads "3 links", the sidebar Audio count is 3 and Undo is enabled. The status bar reads "pw-cat · FL (output_FL) → musubi-vm-patch · FL (playback_FL) · connected — click to disconnect". Defects:
  - The To filter text can't be read.
  - The column name is cut, losing "patch".
  - The pointer covers part of the cell.
- `…/03-audio-undo.png`: The same filters. The FL × FL cell is dark and empty, with the amber ring. "2 links", sidebar count 2, Undo disabled, and the status bar reads "Undo applied.". Defects: the same To field and column cut as 02.
- `…/04-midi-before.png`: "MIDI 12 ports · 0 links", a 6×6 grid, all empty, with focus on the top-left cell. The status bar is empty, so the stale message is gone. Rows: "ALSA MIDI (…" out, "…MIDI 0-0" to "…MIDI 0-3" (ports "…0-0" to "…0-3"), and Midi Through "Midi Th…". Defects:
  - The **column group boxes show only icons, with no names at all** (x≈392–572, y≈84–122). The port labels ("in", VirMIDI 0-0 to 0-3, "Midi Through Port-0") take the whole header height.
  - The "in" column can't be identified.
- `…/05-midi-linked.png`: Both filters show "ual Raw MIDI" with their starts hidden. A 4×4 grid. The top-left cell ("…MIDI 0-0" × "…0-0") is filled with the amber ring and the pointer on it. The header reads "1 link" and the sidebar MIDI count is 1. The status bar reads "Virtual Raw MIDI 0-0 · VirMIDI 0… MIDI 0-0:VirMIDI 0-0 (playback)) · connected — click to disconnect" and "From "Virtual Raw M…". Defects:
  - The filter starts are still hidden.
  - The status sentence is cut in the middle, losing the "→" and the destination group.
  - The pointer covers part of the cell.
- References: Settei `sound-night.png` and Tansu `system-disk-night.png` both opened.

## Checks
1. **VM sequence state changes: pass.**
   - 02: the first row × first column cell (pw-cat FL × musubi-vm-patch FL) is filled. The count goes from 2 to 3 links (sidebar too), Undo is enabled, and the status bar says "connected".
   - 03: the same cell is empty, the count is 2 links, and the status bar says "Undo applied.".
   - 05: the first VirMIDI row ("…MIDI 0-0") × first column ("…0-0") is filled, with "1 link" and sidebar MIDI 1.
   - Caveat: the pointer covers part of the filled cell in 02 and 05, but the fill is visible around it.
2. **Every cell state distinguishable at 1× in Night and Day: uncertain.**
   - Clear in both themes: empty (outline), connected (solid fill), half (diagonal, now visible in Day too), asked (small blue centre square with a dashed outline), collapsed count (digit with a blue underline) and drag preview (light-blue ring with row/column tint).
   - Error in Day: a red inner border inside the amber focus ring, plus a corner tick. This is distinct.
   - Error in Night: the error edge and the focus ring merge into one thick coral ring at 1×. It differs from the amber focus ring in VM 02/03 only by hue and a 2–3 px tick. I can't say with confidence that a user would tell them apart without comparing side by side.
   - Asked and error are only shown with the focus ring on, so neither has been seen without focus.
3. **Names: fail.**
   - Working:
     - The VM sinks can be told apart ("…-vm-default" / "…-vm-patch", VM 01).
     - The MIDI VirMIDI rows can be told apart ("…MIDI 0-0" to "…MIDI 0-3", VM 04/05 and midi goldens), and so can the columns in the midi goldens and VM 05 ("…0-0" to "…0-3").
     - Stream subtitles are readable and don't overlap.
     - Full Names shows whole names.
     - The tooltip shows the full text.
   - Failing:
     - VM 04's column group boxes are blank (icons only).
     - `midi2-show-midi1-night` columns all read "…· 1.0".
     - In VM 02, with one sibling left, the name falls back to "musubi-vm-pat…".
     - Subtitle placement is inconsistent and ambiguous: a subtitle sits above its port row in groups with several streams and below it in single-stream groups.
     - Still cut: "Camera Vi…" and "capture…", "Built-in Au…", "BT Headphon…", "ALSA MIDI (…", "Launc…", "EQ Playback Str…", and the collapsed-count headers ("Disco…", "OBS C…").
4. **Looks like an Aizome app: pass.**
   - The palette, square 1pt-edged controls, mono captions (PATCHBAY, FROM/TO), app card, sidebar with mono counts, two-line footer (version, then rate · quantum · latency), and mono status bar match Settei and Tansu.
   - In the VM it sits correctly in a real Aizome session (menu bar, window tab, shell panel).
   - Remaining gap: toolbar actions are text only, where Tansu uses icon+text.
   - On the unavailable page the server card moves out of the footer.
5. **Add Route sheet, List view, empty and unavailable pages finished and native: fail.**
   - List view (Day): pass.
   - Empty page: pass.
   - Unavailable page: mostly finished. It is centred and the controls are dimmed, but the server card jumps up under the navigation and the From/To fields still look active.
   - Add Route sheet: improved (inset text, From search shows "USB"), but the first row of both lists is jammed against or cut by the list top edge, nothing is selected, and there is no scroll indicator.
6. **First review's top 8:**
   1. **MIDI rows / VM columns identifiable: partly.**
      - Resolved: VM 01 columns ("…-vm-default", "…-vm-patch"), VM 04/05 rows ("…MIDI 0-0" to "…MIDI 0-3"), and midi goldens.
      - New gaps: VM 04 column group boxes are blank; midi2-show-midi1 columns all read "…· 1.0"; VM 02 single column "musubi-vm-pat…".
   2. **Day half diagonal: resolved.** The `audio-studio-day` AUX1×OBS cell and the legend swatch both show a dark diagonal on light blue.
   3. **Error vs focus, plus goldens for asked, count and error in both themes: partly.**
      - All six goldens exist and read clearly.
      - Day error (red inside amber) is distinct.
      - Night error is a single coral ring, close to the focus ring.
   4. **Finish the Add Route sheet: partly.**
      - Done: the inset, and the From search showing "USB".
      - Not done: no selected row, no scroll indicator, and the cut row moved from the bottom to the top of both lists.
   5. **Status bar and filter fields: partly.**
      - Resolved: the stale "Undo applied." (VM 04 status is empty), and VM 05 now ends with the state ("connected — click to disconnect").
      - Not resolved:
        - Hover sentences are now cut in the middle, losing "→" and the destination (VM 05, drag-preview, inspector).
        - The filter starts are still hidden (VM 05 "ual Raw MIDI").
        - The VM 02/03 To field shows only "h", which is worse than before.
   6. **Tighten the column header: partly.**
      - Resolved: in the audio-studio goldens the empty band is gone and the foot icons have space.
      - Not resolved: the collapsed-count goldens still have about 50 px of empty band with heavily cut names, and VM 04 has no room left for group names.
   7. **Sidebar counts and footer second line: resolved.** Mono counts are shown per page, and the footer reads "PipeWire 1.6.9 / Defaults · 48 kHz · 1024 · 21.3 ms" (VM: 1.6.11).
   8. **Unavailable page consistent with empty page: resolved.** The message is centred and the toolbar controls are dimmed. Leftovers: the From/To fields still look active, and the server card has moved (see 7).
7. **Remaining defects (ranked by how much a user would notice):**
   1. `04-midi-before.png`, column header x≈392–572, y≈84–122: the group boxes have no names, only icons, and the "in" column can't be identified.
   2. `midi2-show-midi1-night.png`, column header x≈392–640: every group name is "…· 1.0" or "MIDI…". Row names are cut to 5–6 characters ("Keyst…", "Launc…", "Midi …").
   3. `02-audio-linked.png` / `03-audio-undo.png`, To field x≈981–1035: only "h" is visible. In `05-midi-linked.png` both fields show "ual Raw MIDI", and the right status text is "From "Virtual Raw M…".
   4. Hover sentences cut in the middle, losing the arrow and the destination: `05-midi-linked.png`, `audio-drag-preview-{night,day}.png` and `audio-studio-inspector-night.png`, status bar left.
   5. Stream subtitles: in the audio goldens (Firefox y≈582–642, pw-cat y≈738–798) each subtitle sits evenly between two port rows. Single-stream groups (Spotify, Bitwig, EQ in channels) put the subtitle below the port row instead. Empty grid bands sit beside the title lines.
   6. `audio-collapsed-count-{night,day}.png` header: group names cut to "BT He…", "Built…", "Disco…", "OBS C…"; an empty band at y≈150–200; the row label "…–AUX3 ▸".
   7. `add-route-sheet-night.png` lists at y≈156–168: the first row is cut by the top edge, with no selection and no scrollbar.
   8. `audio-error-night.png` cell at x≈558–580, y≈267–289: the coral error ring is hard to tell from the amber focus ring at 1×.
   9. Remaining cut names: `video-night.png` "Camera Vi…" and "capture…"; audio goldens "BT Headphon…" and "Built-in Au…" (row and column); `02-audio-linked.png` "musubi-vm-pat…"; "EQ Playback Str…"; "ALSA MIDI (…" and "Launc…".
   10. `unavailable-night.png` sidebar y≈290–320: the server card moves from the footer to under the navigation and loses its second line. The From/To fields look enabled.
   11. `audio-tooltip-night.png` at y≈580–614: the tooltip covers the subtitle, port label and cells. It has about 110 px of empty width, and it uses the same "·" separator as group·port.
   12. `audio-studio-inspector-night.png`: the inspected AUX0 row isn't marked in the matrix, and the FACTS labels mix human names with raw keys.
   13. All matrix goldens: toolbar actions are text only, and "Swap" reads like a label rather than a button. In Day, the rotated mono port labels are faint.

## Unreadable / uncertain
- `audio-studio-night.png` and `audio-streams-night.png` are the same image (same sha256, 67f3b3…), and so are `audio-studio-day.png` and `audio-streams-day.png` (6f6d7a…). No golden shows the pre-streams layout separately.
- I can't tell by eye at 1× whether Night's error ring holds two colours (amber focus plus red error) or one coral colour.
- Asked and error are only shown together with focus, so I haven't seen either without the focus ring.
- I can't tell from the image whether the Add Route To list has more rows below "USB Studio · AUX3". There is no scroll indicator, and I'm not inferring the content from the 46-port count.
- Day variants of full names, tooltip, inspector, MIDI 2.0, video, Add Route, empty and unavailable weren't in my list, so I didn't judge them.