# Musubi 結び — design (v2)

v2 folds in the user's decisions (`decisions.md` D1–D8) and the independent review (`design/review-1.md`; v1 was `design/proposal-v1.md`). It is the product/interaction design; `architecture.md` is the code structure. Pictures in `design/mockups/` are HTML renders with Aizome's palette and fonts, not kit output; kit goldens replace them as each page ships. Research behind it: `research/`.

## 0. Principles
1. **The graph is the truth.** Musubi shows observed PipeWire state. A request is "asked" until the registry shows the result; failures say why. Nothing is optimistic for longer than one round trip.
2. **Aizome first.** Kit widgets and theme only; Night, Day and High Contrast all finished; 24pt targets; everything by keyboard; menus and palette carry every command.
3. **One writer per control, one owner per object.** Pulse-visible volume goes through the same path as Settei; Musubi never fights WirePlumber silently.
4. **Pages only when they work.** A phase ships complete pages; nothing is listed that doesn't do what it says.

## 1. Window

`sidebar_layout` (200pt `sunk` sidebar) + toolbar (30pt `chrome`) + content + status bar (22pt). Default size 1280×800, minimum 900×560.

Sidebar, phase by phase (only shipped pages appear):

| Caption | Page | Phase | Detail (mono, right) |
|---|---|---|---|
| PATCHBAY | Audio | 1 | links on the page |
| | MIDI | 1 | links |
| | MIDI 2.0 | 1 | links |
| | Video | 1 | links |
| MIXER | Mixer | 2 | strips |
| | Effects | 3 | chains |
| SYSTEM | Devices | 2 | devices |
| | Clock & Load | 3 | xruns in the last minute, if any |
| | Setups | 4 | active setup |

Top: app card (tile, "Musubi", "結び · Patchbay"). Bottom: server card ("PipeWire 1.6.9" with a status dot; `48 kHz · 256 · 5.3 ms`). When PipeWire is unreachable, the content of every page is one centred message ("PipeWire isn't answering. Musubi keeps trying.") with the last error in the status bar.

A page whose kind has no ports (no video devices) is a real page with an empty state that says what would appear and why it might not ("No video ports. Cameras appear here when PipeWire's camera support finds them.").

## 2. Patch matrix (phase 1)

Pictures: `design/mockups/v2-audio-phase1-night.png`, `v2-audio-phase1-day.png`, `v2-list-phase1-night.png`, `midi-night.png` (the MIDI picture's sidebar predates v2).

### 2.1 Layout
- **Rows = sources** (output ports), **columns = destinations** (input ports) (D4). **Swap** (toolbar, ⌥❖S) transposes, remembered per page.
- Four regions: **corner** (axis captions `FROM ↓` / `TO →` and the legend), **column header** (scrolls horizontally only), **row header** (scrolls vertically only), **body** (both). Wheel/touchpad scroll the body; headers follow on their axis.
- **Groups** are interfaces (§2.4): row group = `chrome` box spanning its rows with icon, name and one detail line (detail hidden on one-row groups); column group = the same box upright, name set bottom-to-top, icon at its foot. 8pt gaps between groups. A 3pt stripe in the group's **kind** colour (amber hardware, blue application, violet effect/filter, cyan virtual) + legend.
- **Ports**: short labels (mono 11pt `text3` at Night, `text2` at Day): `L·R`, `FL`, `AUX3`, `In 1`, `MIDI`, vertical in column headers. Long names, aliases and facts in the inspector/status bar.
- **Cells**: 24pt pitch (the hit area), 18pt drawn square. States: empty (`sunk` + `line` edge; Day uses `line2` so the grid holds on near-white); connected (`selection` + `blue` edge + top highlight); half (pairs, diagonal split: upper-left L, lower-right R); asked (dashed `blue` edge, 1.5pt in Day, + 6pt square); error (20% red tint, red inset edge and `!`; focus ring outside it); **not allowed** (no square drawn: e.g. a port with an unknown format, or a UMP↔MIDI 1.0 cell before confirmation — hovering says why).
- Collapsed group (click its box, or ⌥←/⌥→ on it): one line labelled `In 1–8 ▸`, cells show the number of links inside (count + bar) and are **informational**: a click expands the group instead of linking.
- **Legend** in the corner on every matrix page, listing every state and kind colour that page can show (connected, half, asked, error, collapsed count; hardware, application, effect, virtual).
- Long values anywhere (inspector facts, status bar) wrap or ellipsize, never both; a value that wraps is shown whole.
- **Names in headers (D17).** Compact by default. A name that doesn't fit is elided where it says least: when sibling groups (or sibling lines) share a prefix, the shared part goes (`musubi-vm-default` / `musubi-vm-patch` → `…-default` / `…-patch`; four `Virtual Raw MIDI 0-n` → `…MIDI 0-0` … `…MIDI 0-3`); otherwise the end is cut. Every group box and every port label has a tooltip with the full title, subtitle, port name and alias. Both header bands resize: drag the edge between a header and the body (24pt hit strip, resize cursor); double-click the edge to fit its content; **View ▸ Show Full Names** (⌥⌘F, also a toolbar toggle) sizes both bands to show every name in full and back. Sizes are remembered per page; the body keeps at least 240pt each way.
- Compact matrix headers reserve a distinct ≥72pt column group tier before the ≤64pt port-label tier, so long PipeWire aliases cannot erase group names. The row header fits visible group names up to 280pt; MIDI 1.0 tags occupy their own small badge at the column group's foot rather than its title.
- **Status bar** hover sentences omit raw PipeWire port-name suffixes first, then elide the group and port names individually, preserving `From group · port → To group · port · state`. A notice ("Undo applied.") lasts until the next action or page change.

### 2.2 Toolbar
`Audio 31 ports · 14 links` · [Matrix | List] · [Pairs | Channels] · Monitors (Audio only) · Swap · Full Names · Undo · spacer · **From** filter · **To** filter. Action toggles use Lucide icon and text, then drop the text on narrow windows before either filter shrinks below 140pt; at the 900pt minimum the title yields too. Filters are kit search fields (clear button, Escape clears), narrow one axis each, and the status bar's right end says what they hide ("From “Scarlett” hides 9 ports"); with no filter it shows the page's port count, never the clock (the sidebar footer has that).

### 2.3 Interaction
- **Click** a cell: connect / disconnect (pairs: L→L and R→R as two links, one undo step; partial pair cell → completes the pair).
- **Line drag** (AFCC's diagonal tool; Q2): press on a cell and drag; a straight line of cells from the press to the pointer is previewed (outlined); release applies it — connect if the first cell was empty, disconnect if it was connected; cells that already match or aren't allowed are skipped; one undo step. Escape during the drag cancels.
- **Keyboard**: the grid takes the focus (Tab); arrows move the focused cell (2pt `focus` ring) and reveal it; ⌥arrows jump to the next group; Home/End row ends, ❖↑/❖↓ column ends, Page Up/Down a screen; Space toggles; Return opens the inspector; ⇧Space starts a line, arrows extend it, Space applies.
- **Context menu** on a cell/port/group: Disconnect all from this port / to this port, Connect group in order (1→1, 2→2…), Collapse/Expand group, Copy port name.
- **Undo/Redo** (❖Z/⇧❖Z): each user action is one transaction of link creations/removals by stable identity; undo applies the inverse only for links that were actually made/removed; if a port has gone, the undo says so and skips it.
- **Hover**: row and column tinted (`selection` at 16%), labels to `text` semibold; status bar sentence `Scarlett 4i4 · In 1 (capture_AUX0) → Discord · In (input_MONO) · connected — click to disconnect`.
- **Session-manager links** look like any link. If the user removes a link and WirePlumber makes it again within 2 s, the status bar says "WirePlumber connected Firefox to Headphones again: it follows the default output." with **Move Stream…** (phase 2, sets `target.object`) — phase 1 shows the explanation only.

### 2.4 Grouping, labels, pairing (R1 §4; review #3, #11, #13)
1. MIDI bridge first: a node that is the ALSA MIDI bridge has its ports grouped by sequencer client (`api.alsa.seq.*` / `port.group`), labelled by the client name; fallback one group "ALSA MIDI".
2. Else by Device (`device.id`), labelled `device.description` → `device.nick` → `device.name`, disambiguated by bus/serial.
3. Else by client (`client.id`), titled `application.name` → node label.
3a. **Streams (D16).** Inside an application group every node is a stream with a subtitle: `media.name` → `node.description` → `node.name`. A group with one stream shows it as the group's detail line (rows) or second upright line (columns, when the group is wide enough; else in the tooltip). A group with several streams draws a subtitle band (Archivo 11pt `text3`, 16pt) above each stream's lines in the row header and beside them in the column header, so two players of the same app are told apart.
- Multi-stream row titles get a 20pt title line without cells. Each stream subtitle is 4pt above its own first port line and 8pt after the previous stream's last line; single-stream subtitles remain the group's detail line.
4. Else by node. Node label `node.description` → `node.nick` → `node.name` → `media.name` → `Node <id>`; port label `audio.channel` → `port.alias` tail → `port.name` → `Port <id>`.
5. Order: hardware, then effects/virtual, then applications; inside groups by node, direction, `port.group`, channel position (FL FR FC LFE RL RR SL SR, then AUX0…AUXn numerically), `port.id`.
6. **Pairs** only when two ports share node, direction, media kind, `port.group` and are exactly FL+FR; AUXn never pair; a pair row against a mono column offers L and R halves.
7. Kind of a port: `format.dsp` (`32 bit float mono audio`, `8 bit raw midi`, `32 bit raw UMP`, video) → EnumFormat → node `media.class`; unknown ports are listed (greyed) but not linkable.

### 2.5 Pages and legality
- **Audio**: audio ports; Monitors toggle adds sinks' monitor outputs as sources.
- **MIDI**: MIDI 1.0 ports (bridge + JACK/PipeWire apps).
- **MIDI 2.0**: UMP ports. Toolbar toggle **Show MIDI 1.0** adds the MIDI 1.0 ports (their groups tagged `1.0`), so cross-kind cells exist on this page only. The first cross-kind link of a session asks: "PipeWire converts between MIDI 2.0 and MIDI 1.0. Some messages don't survive the conversion." [Connect] [Cancel].
- **Video**: video ports.
- A link is allowed when: output→input, both ports present in this epoch, kinds compatible (audio–audio, MIDI–MIDI, UMP–UMP, UMP↔MIDI with confirmation, video–video with a common format family). The server has the last word: a link that errors or never appears turns the cell red with the reason.

### 2.6 List view (accessibility and overview; Q3)
[Matrix | List]: a kit `table` of every link on the page — From (group · port), To (group · port), State (connected / asked / error), plus Kind only on the MIDI 2.0 page when MIDI 1.0 ports are shown — sortable, filtered by the same From/To fields, with the kit table's own selection look; Delete removes selected links (undoable), Return opens the inspector; **Add Route…** opens a sheet with two searchable lists (From, To) and Connect. Port names match the matrix's labels exactly. Every route is reachable and actionable by keyboard and assistive technology here; the grid also publishes semantics (role Table; focused cell as Cell with row/column labels, state and a Toggle action).

### 2.7 Inspector (⌥❖I; right, 260pt)
Selected port: name, kind, group; **Goes to / Comes from** lists with unlink buttons; facts (node name, port name, alias, channel, `format.dsp`, device, client, object serial) as selectable mono text. Audio level meter for the selected port (one meter stream, only while the inspector shows): phase 2 (with the meter engine). MIDI activity: Q4.

## 3. Mixer (phase 2) — `design/mockups/mixer-night.png`
Sections OUTPUTS · INPUTS · PLAYING · RECORDING · EFFECTS. No MAIN strip: PipeWire has no buses, it duplicated the default output, and system volume lives in Aizome's controls. Strip 84pt: tile + name + detail; pop-up (stream → target; device → route); balance; M (mute), ◉ default (devices), channel link; vertical **Fader** (cubic law, dB scale +6…−60, −∞) and per-channel segmented **LevelMeter** (green, amber > −9 dB, red > 0, 1.5 s peak hold); value box (double-click to type). Toolbar: dB/%, Meters, Sections (which strips), filter. Keys: ↑↓ volume, ←→ balance, M mute, Page/Home/End.
- Control path: Pulse-visible objects through `aizome-sys::audio`-equivalent semantics with a **verified identity bridge** (Pulse proplist `object.serial`/`node.name` ↔ graph node; ambiguous → read-only + reason). Native-only nodes get a fader only with writable `Props.channelVolumes`, read back after writing.
- Meters: one engine on the PipeWire thread, only visible strips (cap 24; strips over the cap show "meter paused"), stop when hidden.
- Strip width follows its content within 84–120pt so names, pop-ups (always with their chevron) and value boxes aren't clipped; dB scale labels at the kit's caption size (10.5pt), clear of the value box; strips stop 8pt above the status bar.
- Playback and recording strips have no balance control until `aizome-sys::audio` exposes stream channel maps and a Pulse balance setter; native Props must not become a second writer for Pulse-visible streams (B11).
- Devices page (Tansu's layout): device list + profile/route pop-ups with availability.

## 4. Effects and Clock & Load (phase 3) — `design/mockups/effects-night.png`, `design/mockups/graph-night.png`
- Effects: stereo **EQ on a virtual output** (D6). Each chain: its own generated config + its own unit `aizome-musubi-effect@<chain>.service` (start at login when enabled; independent restarts). Editor: name, Sends to, preset, response curve (20 Hz–20 kHz, ±15 dB, numbered square handles; drag freq/gain, scroll Q), band table, Bypass. Live edits only if a write/readback test proves `Props.params`; else restart that chain's unit on release (status bar says so).
- Clock & Load: effective rate/quantum shown separately from **forced** values; Automatic clears the runtime force keys **and** Musubi's own default drop-in; "Use at every start" writes only explicit numbers to `pipewire.conf.d/90-aizome-musubi-clock.conf`. DSP load and errors/xruns charts, driver/follower table from the Profiler while the page shows; no global "reset counters".

## 5. Setups (phase 4)
Named sets of routes by composite stable identity (device name + serial/bus path, node name, direction, port name, sequencer client), opt-in **Keep connected** enforced by `aizome-musubi --keeper` (user service, D5) also while Musubi is closed; **Exclusive** with a warning and an explanation of WirePlumber's auto-links; activation, deactivation, pin/unpin and auto-pin semantics written down before implementation; ambiguous/missing matches listed, never guessed.
