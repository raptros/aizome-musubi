# Phase 1 visual check 3b — bridge blocker (sekka-visual, anthropic/claude-opus-5-5, 2026-11-20)

Verbatim. Images at aizome-musubi `8d29896`. Review 3's blocker #1 resolved; the residual duplicate port label under #50/#60 is told apart by its group header.

## Images
- `repos/aizome-musubi/tests/snapshots/midi-vm-bridge-night.png`: Dark theme, 1280×824. Musubi window with the MIDI tab selected. The toolbar reads "MIDI 16 ports · 0 links", with Matrix active and List, Swap, Full Names, Undo (greyed), From and To controls. The legend shows connected, error, half, count, asked, hardware, app and virtual.
  - Column group headers, rotated, left to right: "Launchkey", "…0-0 #50", "…0-0 #60", "…MIDI 0-1", "…MIDI 0-2", "…MIDI 0-3", "Ardour". Each has a client icon and a coloured top bar: orange for hardware, blue for Ardour.
  - Column port labels, rotated, left to right: "MIDI 1", "…0-0", "…0-0", "…0-1", "…0-2", "…0-3", "midi_in".
  - Rows, group / port: ALSA MIDI (other) / MIDI 1; Keystep / Keys; Launchkey / Keyboard; Virtual Raw MIDI 0-0 / …0-0; Virtual Raw MIDI 0-1 / …0-1; Virtual Raw MIDI 0-2 / …0-2; Virtual Raw MIDI 0-3 / …0-3; Midi Through / Port-0; Ardour (subtitle "Ardour MIDI") / midi_out.
  - The grid is 9×7 and every cell is empty. The status bar reads "16 ports". The sidebar shows Audio 0, MIDI 0, MIDI 2.0 0, Video 0, and "PipeWire 1.6.9 / Defaults · 48 kHz · 1024 · 21.3 ms".
  - Defects: no clipping, black or empty surfaces, or wrong window. The two adjacent column port labels "…0-0" are identical (see check 2).
- `repos/aizome-musubi/tests/snapshots/midi-vm-bridge-day.png`: Light theme, 1280×824. The layout and every text string match the night image exactly: same headers ("…0-0 #50", "…0-0 #60", "…MIDI 0-1/0-2/0-3"), same column port labels (including two "…0-0"), same 9 rows, same 9×7 empty grid, "16 ports". Header boxes are white with darker orange, teal and blue bars. All text is legible against the light background.
  - Defects: same as night. No clipping or rendering defects seen. The two column port labels "…0-0" are identical.

## Checks
- 1. Every column group can be told apart from its siblings: **pass**. In both images the seven headers are all different. The twin "Virtual Raw MIDI 0-0" clients show as "…0-0 #50" and "…0-0 #60". The client number and "#" are readable at this size. The other three show as "…MIDI 0-1", "…MIDI 0-2" and "…MIDI 0-3".
- 2. Every column port label can be told apart: **fail** (strictly on the label text). Columns 2 and 3 are both labelled "…0-0", with the same text and position, in both themes. The other five labels are different from each other: "MIDI 1", "…0-1", "…0-2", "…0-3", "midi_in". The "(playback)" tail is gone and the old four identical "VirMIDI…" labels are now four different suffixes. The two "…0-0" columns can be told apart only from the group header directly above them (#50 or #60), not from the port label itself.
- 3. Every row can be told apart: **pass**. All 9 rows have different group/port pairs in both themes. The Virtual Raw MIDI rows are labelled 0-0, 0-1, 0-2 and 0-3 in both the group name and the "…0-x" port label.
- 4. Is review 3's blocker #1 resolved: **pass**. The pairs of identical column group names are now disambiguated by "#50" and "#60". The four identical "VirMIDI…" port labels are now "…0-0", "…0-1", "…0-2" and "…0-3". Every column can be identified uniquely from what is on screen (header plus port label). One duplicate label pair remains, described under check 2 and the new defects below.
  - New or residual defects:
    - (a) The column port label "…0-0" appears twice (under #50 and under #60), so the label alone does not identify the port.
    - (b) There is only one "Virtual Raw MIDI 0-0" row and it has no "#50"/"#60" qualifier. On screen you cannot tell which of the two column clients (#50 or #60) it belongs to. I can't tell from the image whether this is a defect or just how the fixture is set up.
    - (c) The headers are cut short from the left ("…0-0 #50" loses "Virtual Raw MIDI", "…MIDI 0-1" loses "Virtual Raw"). It stays readable, but the two forms are inconsistent across sibling headers.

## Unreadable / uncertain
- None. All text in both images is legible. Check 2 is a strict fail only for the "…0-0"/"…0-0" pair; whether that pair is acceptable because the headers above tell them apart is a judgement for the ticket owner. Item (b) under check 4 also can't be settled from the images alone.