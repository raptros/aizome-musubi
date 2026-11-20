# Backlog

Known, non-blocking issues carried past phase 1. Sources: `docs/design/visual-review-phase1-3.md` §7 (ranked), code review minors (all fixed). Take them up alongside phase 2 work that touches the same code.

| # | Issue | Where | Source |
|---|---|---|---|
| B1 | Stream subtitle placement: in multi-stream groups a subtitle sits about midway between the previous stream's lines and its own; single-stream groups put it under the title-line port row. Want: subtitle 4pt above its own lines, 8pt+ below the previous stream; no empty grid band beside title-only lines | `matrix::layout` row/column offsets | review 3 #2 |
| B2 | Tooltip covers the hovered row's cells (incl. a connected one); ~50px slack | widget tooltip placement | review 3 #3 |
| B3 | Search fields clip the first glyph under the magnifier when scrolled to the start; right-hand status text cut in narrow windows; "From … hides" ignores the To filter | toolbar fields (kit `search_field` in a narrow toolbar), status bar | review 3 #4, #7 |
| B4 | Names still cut with spare header room: video "…ra Viewer"/"…pture_1", "BT Headphon…", "Built-in…" (columns), "Midi Thr…", "MIDI 2 S…"; collapsed ranges lose their start ("…–AUX3") | header fit heuristics | review 3 #5, #6 |
| B5 | Unavailable page: From/To fields look enabled | app toolbar | review 3 #8 |
| B6 | Inspected port marked only by a thin tick; add a faint row/column tint | widget | review 3 #9 |
| B7 | Add Route sheet: no scroll indicator; background not visibly dimmed | `routes` sheet / kit | review 3 #10 |
| B8 | MIDI matrices use a small part of the body; Day rotated port labels still mid-grey; narrow toolbar icon-only | layout, theme roles | review 3 #11 |
| B9 | Paint-as-you-go drag as an optional setting, if users ask (D10) | gesture | D10 |
