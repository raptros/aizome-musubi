# Visual review 1 — mockups (sekka-visual, anthropic/claude-opus-5-5, 2026-11-11)

Verbatim output of agent MockupVisualReview. Inputs: `design/mockups/*.png` (HTML renders), Aizome kit goldens from Sekka `aebca32e`.

## Images
- `runs/2026-11-11/patchbay/design/mock/v2-audio-phase1-night.png`: Night audio matrix.
  - **Sidebar:** Musubi tile with "結び · Patchbay". The PATCHBAY section lists Audio 14 (selected), MIDI 5, MIDI 2.0 0 and Video 1. The footer reads "PipeWire 1.6.9 / 48 kHz · 256 · 5.3 ms".
  - **Toolbar:** "Audio 31 ports · 14 links", Matrix/List and Pairs/Channels segmented controls, Monitors, Swap, undo, and From/To search fields.
  - **Legend (top left):** Connected, One channel of a pair, Asked not yet made, Hardware (amber), Application (blue), Effects (purple).
  - **Column groups (vertical):** Scarlett 4i4, Built-in Speakers, PG27UCDM (HD…, WH-1000XM5, Headphone EQ, Bitwig Studio, OBS Studio, Discord.
  - **Row groups:** Scarlett 4i4, Built-in Audio, Firefox, Spotify, Bitwig Studio, Headphone EQ.
  - **Cell states:** filled, diagonal-half, dashed-with-dot, and a small "2" badge in the Bitwig "In 1–8" column.
  - **Hover, focus and inspector:** Row "In 1" and the Discord column are highlighted. An amber focus ring sits on Scarlett In 1 → Discord In. The inspector shows SELECTED PORT, GOES TO (3 entries with disconnect glyphs) and PORT key/values.
  - **Status bar:** route text on the left; "48 kHz · 256 · 5.3 ms" in mono on the right.
  - Defects:
    - The "2" badge cell state is not in the legend.
    - The Node value in the inspector wraps to 3 lines and is also ellipsized ("…Scarlett_4i4…").
    - The column name "PG27UCDM (HD…" is truncated.
    - The status bar's right side repeats the sidebar footer text.
    - Most of the canvas is unused: x≈772–1020 to the right of the matrix and y≈612–778 below it.
- `runs/2026-11-11/patchbay/design/mock/v2-audio-phase1-day.png`: Same layout in the Day theme.
  - Light lavender-gray sidebar with a saturated blue "Audio" selection, near-white canvas, and dark-blue connected and half cells.
  - Hardware group bars turn dark gold, applications blue, effects purple.
  - The hover row and column bands are light blue-gray, and the amber focus ring is on the Discord In cell.
  - Defects:
    - Empty cells are light-gray squares with a pale edge on a near-white background. They are low contrast but visible.
    - The amber focus ring is thin against the light background.
    - Same truncation ("PG27UCDM (HD…") and Node wrap+ellipsis as Night.
- `runs/2026-11-11/patchbay/design/mock/v2-list-phase1-night.png`: List view.
  - **Toolbar:** "Audio 31 ports · 16 links", Matrix/List (List active), "+ Add Route...", Disconnect, undo. The From field contains "Scarlett"; the To field is empty.
  - **Table:** columns From▾ / → / To / Kind / State with 5 striped rows. Row 3 (Scarlett 4i4 · In 1 → Discord · In) is selected with a full-width saturated blue fill.
  - **Status bar:** "⌫ disconnects · Return shows the port" on the left; `From: "Scarlett" hides 11 of 16 routes` on the right.
  - Defects:
    - The sidebar shows Audio 16 and the toolbar "16 links", while the matrix mock shows 14.
    - The To column reads "OBS Studio · FL/FR", but the matrix inspector says "OBS Studio · L".
    - There is a wide empty gap between From (x≈212) and the arrow column (x≈538).
    - The Kind column says "Audio" on every row inside the Audio page.
    - The From field shows the text "Scarlett" but has no visible active-filter treatment or clear button.
- `runs/2026-11-11/patchbay/design/mock/midi-night.png`: MIDI matrix.
  - **Sidebar (larger draft):** PATCHBAY (Audio 14, MIDI 7 selected, MIDI 2.0, ALSA Sequencer 2, Video 1), MIXER (Mixer, Effects 1), SYSTEM (Devices 4, Graph, Setups). The footer adds "DSP 11% · 0 xruns" and a green bar.
  - **Toolbar:** "MIDI 16 ports · 7 links", Swap, undo, a "Studio · kept" dropdown and "Filter ports".
  - **Column groups:** KeyLab 61 mk3, Digitakt II, MIDI Through, Bitwig Studio, VCV Rack, Surge XT.
  - **Matrix states:** Row "Seq 1" and the Digitakt column are hover-highlighted with no focus ring. A requested (dashed) cell sits at VCV Out → Surge XT In.
  - **Inspector:** has an ACTIVITY strip of 16 segments.
  - Defects:
    - The matrix corner has no legend.
    - The MIDI Through header bar is a lighter cyan-blue that no legend explains.
    - There is no Matrix/List control (the audio page has one).
    - The "Port 0" vertical label extends higher than its siblings.
    - Counts differ from the audio mock: MIDI 7 vs 5, and MIDI 2.0 has no count vs "0".
- `runs/2026-11-11/patchbay/design/mock/mixer-night.png`: Mixer.
  - **Toolbar:** "Mixer 9 strips", dB/% segmented control, Meters toggle (active), "+ Show...", "Filter strips".
  - **Groups (mono uppercase captions):** OUTPUTS (Scarlett 4i4, Speakers, WH-1000…), INPUTS (Scarlett 4i4, Microphone), PLAYING (Firefox, Spotify, Bitwig Stu…), RECORDING (Discord), and MAIN (Main Output), set apart by a vertical rule.
  - **Each strip:** icon tile, name and subtitle, device dropdown, L–R balance slider, M / monitor / link square buttons, a vertical fader with a dB scale (+6 … −60), a two-column segmented green meter (one amber segment on Bitwig), a mono value box ("-8.0 dB") and a channel caption.
  - The WH-1000XM5 strip has a blue 1pt selection outline, and the Microphone M is a pink fill (muted).
  - Fader thumb positions match the value boxes; I checked −8, −18, −4.5, −6, −3 and −9 against the scale.
  - Defects:
    - Truncated: "WH-1000…", "Bluetooth · …", "Bitwig Stu…", "Tōkyō Jazz …".
    - The dropdowns "Internal Mic", "Scarlett 4i4" (Bitwig) and "Scarlett In 1" (Discord) are clipped at the right edge, so their chevrons are cut off or missing.
    - The "Headphone" dropdowns (Firefox, Spotify) show no chevron, while "Monitors", "Speakers", "A2DP" and "Pro Audio" do.
    - The "-60" scale label sits directly on top of the value box.
    - The strip bottoms run to y≈776, flush against the status bar.
    - The "MONO" caption on Discord looks brighter than the other channel captions.
- `runs/2026-11-11/patchbay/design/mock/effects-night.png`: EQ chain editor.
  - **Toolbar:** "Effects 2 chains" and undo.
  - **Chains pane:** CHAINS (Headphone EQ selected with a green status square; Voice), OTHER EFFECTS (EasyEffects Sink, "Not managed here"), and "+ New Chain..." at the bottom.
  - **Header:** title "Headphone EQ" with a subtitle, "● running" in green mono, and a Bypass button.
  - **Inline fields:** Appears as / Sends to / Preset.
  - **EQ chart:** bordered, with mono axis labels (+12…−12, 20…20k), a filled curve and numbered colored band markers 1–5. Marker 3 has an amber focus ring.
  - **BANDS table:** type dropdowns, mono right-aligned values, toggles, and row 3 selected. Below it are Add Band and Reset buttons and a service footnote.
  - Defects:
    - The sidebar shows "Effects 1" but the toolbar and pane show 2 chains.
    - The bands table leaves a large empty area right of the "On" column (x≈1010–1217).
- `runs/2026-11-11/patchbay/design/mock/graph-night.png`: Clock & Load page.
  - **Toolbar:** "Graph 2 drivers · 9 nodes" and Reset Counters.
  - **CLOCK group:** bordered Settei-style rows: Sample rate with a dropdown, Buffer size with a 64/128/256/512/1024/Auto segmented control (256 active), and Keep after restart with an off toggle.
  - **Charts:** DSP load (green area, "11% · peak 61%") and Xruns (red spike, "1 in the last minute"), each with a caption.
  - **DRIVERS AND WHAT THEY RUN table:** bold driver rows with ▸ markers and indented children; Xruns "1" in red.
  - **Status bar:** "Bitwig Studio missed a cycle at 20:41:07" and "profiler · 1 s interval".
  - Defects:
    - The ▸ (collapsed) glyphs sit on driver rows whose children are shown.
    - The sidebar footer says "0 xruns", but the chart and table show 1 xrun.
    - The Wait and Busy columns show no unit.
- **Kit goldens:** Settei sound Night/Day, Yukimi storage, Tansu system-disk, Myaku cpu, and kit controls all opened. They establish the language I compared against:
  - deep indigo-gray palette with a saturated-blue sidebar selection
  - square 1pt-edged controls and square fader thumbs
  - letter-spaced mono uppercase section captions
  - app tile + name + kanji subtitle in the sidebar header (Yukimi)
  - toolbar with a bold title and a mono count
  - status bar with mono text on the right
  - muted (not saturated) table-row selection (Tansu, Myaku)
  - green segmented level meter (Settei)
  - filled-area mini charts with mono values (Myaku)
- **Arturia references:** `afcc-routing-matrix.png` and `afcc-main-mixer.png` opened. They show charcoal panels, orange and purple numbered tiles, cyan cells, round knobs and round fader caps.

## Checks
1. **Design-language match: pass.**
   - Palette, square 1pt-edged cells and buttons, mono uppercase captions (PATCHBAY, TO →, FROM ↓, SELECTED PORT, OUTPUTS, CLOCK, BANDS), sidebar header tile + "結び · Patchbay", toolbar "Audio 31 ports · 14 links" and the mono right-side status bar all match the Yukimi, Tansu and Settei goldens.
   - Square fader thumbs and green segmented meters match Settei and the kit controls.
   - No Arturia traits appear: no numbered orange tiles, cyan cells, round knobs, round caps, or boxed all-caps group buttons.
   - The one inconsistency is within the mock set: the Phase-1 audio and list sidebars lack the MIXER and SYSTEM sections shown on the other four pages.
2. **Matrix legibility (audio Night and Day): pass.**
   - Group names, port labels (mono "Out 1", "L·R", "In 1–8"), and empty, filled, diagonal-half and dashed cells are all distinguishable at 1x in both themes.
   - The hover row and column bands and the amber focus ring are visible.
   - Weakest states:
     - The "2" count badge in the Bitwig column: tiny digit, and not in the legend.
     - The requested (dashed) cell in Day: thin dashed edge on a pale cell.
     - Empty cells in Day: pale edge on near-white.
     - The Night hover band: a subtle lightening.
     - The amber focus ring in Day: thin on light.
3. **Vertical column-group names: pass.**
   - All read bottom-to-top consistently with icons below.
   - Only truncation: "PG27UCDM (HD…" (Night and Day). The model name stays intact, so the meaning survives.
   - MIDI names (KeyLab 61 mk3, Digitakt II, MIDI Through, Bitwig Studio, VCV Rack, Surge XT) are all complete.
4. **Day theme: pass.**
   - It looks finished, and the sidebar, toolbar and status bar match the Settei Day golden.
   - Connected and half cells are strong dark blue, and labels are dark and legible.
   - Hardware bars shift to dark gold so they stay distinct.
   - Empty-cell edges are low contrast but still readable as a grid, and the focus ring is thin.
5. **List view: pass.**
   - Header row, 24px striped rows, sort marker "From▾", and bold node + regular port text read like the Myaku and Tansu tables.
   - Selection is unmistakable: a full-row saturated blue fill.
   - Filtering is evident through "Scarlett" in the From field and the status text `hides 11 of 16 routes`.
   - Deviations:
     - The selection uses a saturated fill, whereas the Tansu and Myaku tables use a muted row highlight.
     - The filter field has no active state or clear affordance.
     - There is a wide gap before the arrow column, and the Kind column is redundant.
6. **Mixer: pass.**
   - Faders, meters, dB scale and value boxes are legible, and the thumb positions match the values.
   - Square thumbs and green segmented meters are consistent with Settei and the kit.
   - Cramped or misaligned items:
     - Clipped dropdowns with missing chevrons: "Internal Mic", "Scarlett 4i4", "Scarlett In 1", and "Headphone" (×2).
     - Truncated names: "WH-1000…", "Bluetooth · …", "Bitwig Stu…", "Tōkyō Jazz …".
     - The "-60" scale label sits on the value box.
     - The strip bottoms are flush against the status bar.
     - The scale labels are very small.
     - The "MONO" caption is brighter than its siblings.
7. **Effects and Clock & Load pages: pass.**
   - The Graph page's CLOCK group matches Settei grouped rows (label + description, control on the right, segmented control, toggle).
   - The DSP and Xruns charts match the Myaku mini charts (bold title, mono value on the right, filled area, caption).
   - The Effects title and subtitle header matches the Yukimi "The Nix store" layout.
   - The bordered EQ chart and bands table use kit toggles, dropdowns and buttons.
   - Minor: the Effects inline "Appears as / Sends to / Preset" fields depart from the Settei row form, and the driver rows use collapsed ▸ glyphs while expanded.
8. **Concrete visual defects: fail.** Visible defects:
   - **Mixer:** clipped dropdowns and missing chevrons; "-60" on the value box; strips touching the status bar.
   - **Audio inspector:** Node value is both wrapped and ellipsized.
   - **Graph:** ▸ shown on expanded driver rows.
   - **Count mismatches between pages:**
     - Audio 14 vs 16, and 14 vs 16 links
     - MIDI 5 vs 7; MIDI 2.0 "0" vs blank
     - Effects 1 vs "2 chains"
     - "0 xruns" footer vs 1 xrun in the chart and table
   - **Naming mismatch:** OBS "L" (inspector) vs "FL" (list).
   - **Undocumented states:** the "2" badge cell (audio) and the MIDI Through cyan header bar (MIDI, no legend at all).
   - **Sidebar and toolbar differences:** sidebar sections differ between the audio/list pages and the rest; the MIDI page has no Matrix/List control.
   - **Low contrast:** empty cells in Day; tiny dB scale labels in the mixer.
   - **Interactivity cues:** the GOES TO disconnect glyphs in the inspector are frameless gray icons, so they read weakly as buttons. The "Headphone" dropdowns without a chevron look like static boxes.

## Top 5 visual improvements (ranked)
1. **`mixer-night.png`, strip headers and dropdown row (y≈100–170, all strips):** give names and dropdowns enough width that every dropdown shows its chevron. That means "Internal Mic", "Scarlett 4i4", "Scarlett In 1" and "Headphone" ×2. Also avoid "WH-1000…" and "Bitwig Stu…" truncation.
2. **Matrix legends:**
   - In the `v2-audio-phase1-*` legend (top-left, x≈220–345, y≈70–170), add the "2" count-badge cell state.
   - Add an equivalent legend to the empty `midi-night.png` corner (x≈220–440, y≈40–190) covering the MIDI Through cyan bar.
3. **`v2-list-phase1-night.png`:**
   - Selected row (y≈102–126): use the muted table-row selection seen in Tansu and Myaku.
   - From field (x≈968–1118, y≈4–26): give it a visible active-filter state with a clear control.
   - Close the gap between From and → (x≈310–530).
4. **Sidebar consistency across all mockups (x 0–199):** use one section set (PATCHBAY / MIXER / SYSTEM), one footer (with or without DSP/xruns) and matching counts on every page. The audio and list pages differ from midi/mixer/effects/graph, and counts disagree (14/16, 5/7, Effects 1 vs 2 chains, 0 vs 1 xruns).
5. **`v2-audio-phase1-day.png` matrix cells (x≈455–770, y≈230–600):**
   - Strengthen the empty-cell edge and the dashed requested-cell edge.
   - Thicken the amber focus ring on the Discord In cell (x≈748–772, y≈230–250) so it stands out on the light background.

## Unreadable / uncertain
- The meaning of the "2" badge in the Bitwig "In 1–8" cells (`v2-audio-phase1-*`) cannot be told from the image. The legend does not explain it.
- The "MONO" caption brightness difference in `mixer-night.png` is slight, so I'm not certain it's intended rather than a render artifact.
- `mixer-day.png`, `audio-day.png`, `audio-night.png` and `effects-night.png` are listed in `capture.json` alongside these mockups, but only the seven assigned images were reviewed.