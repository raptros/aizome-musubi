use std::rc::Rc;

use aizome_kit::element::MsgFn;
use aizome_kit::hash::PaintHasher;
use aizome_kit::layout::Constraints;
use aizome_kit::sidestep_ui::{Key, NamedKey, PointerButton};
use aizome_kit::text::TextSpec;
use aizome_kit::theme::Theme;
use aizome_kit::widget::{
    Event, EventCx, FocusRing, Focusable, LayoutCx, PaintCx, SemanticRole, Semantics, Status, Widget,
};
use aizome_kit::{Align, Color, Element, FontSpec, Point, Rect, Role, Size, Vec2};

use super::layout::{COLUMN_HEADER, Cell, CellState, Kind, Layout, PITCH, ROW_HEADER};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixMsg {
    Toggle(Cell),
    Line(Vec<Cell>, bool),
    Hover(Option<Cell>),
    Focus(Cell),
    Select(Cell, bool),
    Context(Cell, bool),
}

/// Inclusive raster line, independent of pointer sampling frequency.
pub fn line_cells(start: Cell, end: Cell) -> Vec<Cell> {
    let (mut x, mut y) = (start.col as isize, start.row as isize);
    let (x1, y1) = (end.col as isize, end.row as isize);
    let (dx, dy) = ((x1 - x).abs(), (y1 - y).abs());
    let (sx, sy) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
    let mut error = dx - dy;
    let mut cells = Vec::with_capacity(dx.max(dy) as usize + 1);
    loop {
        cells.push(Cell { row: y as usize, col: x as usize });
        if x == x1 && y == y1 {
            break;
        }
        let twice = 2 * error;
        if twice > -dy {
            error -= dy;
            x += sx;
        }
        if twice < dx {
            error += dx;
            y += sy;
        }
    }
    cells
}

#[derive(Default, Clone, Copy)]
struct State {
    offset: Vec2,
    size: Size,
    hovered: Option<Cell>,
    focused: Option<Cell>,
    keyboard_focus: bool,
    pressed: Option<Cell>,
    line: Option<(Cell, Cell)>,
}

pub struct PatchMatrix {
    layout: Rc<Layout>,
    on_event: aizome_kit::element::MsgMap<MatrixMsg>,
}

impl PatchMatrix {
    pub fn new<Msg: Send + 'static>(layout: Rc<Layout>, f: impl Fn(MatrixMsg) -> Msg + 'static) -> Element<Msg> {
        Element::new(Self { layout, on_event: aizome_kit::element::MsgMap::new(f) }, vec![])
    }

    fn view(size: Size) -> (Rect, Rect, Rect, Rect) {
        let x = ROW_HEADER.min(size.width);
        let y = COLUMN_HEADER.min(size.height);
        (
            Rect::new(0., 0., x, y),
            Rect::new(x, 0., size.width, y),
            Rect::new(0., y, x, size.height),
            Rect::new(x, y, size.width, size.height),
        )
    }

    fn cell(&self, pos: Point, st: &State) -> Option<Cell> {
        let (_, _, _, body) = Self::view(st.size);
        if !body.contains(pos) {
            return None;
        }
        Some(Cell {
            row: self.layout.rows.at(pos.y - body.y0 + st.offset.y)?,
            col: self.layout.cols.at(pos.x - body.x0 + st.offset.x)?,
        })
    }

    fn damage_cell(&self, cx: &mut EventCx, cell: Cell) {
        let st = *cx.state::<State>();
        let (_, col, row, body) = Self::view(st.size);
        let x = body.x0 + self.layout.cols.offsets[cell.col] - st.offset.x;
        let y = body.y0 + self.layout.rows.offsets[cell.row] - st.offset.y;
        // Include glyph antialiasing and the two adjacent cell boundaries;
        // header text can overhang its nominal 24pt label box.
        let margin = 2. * PITCH;
        cx.invalidate_rect(Rect::new(body.x0, y - margin, body.x1, y + PITCH + margin).intersect(body));
        cx.invalidate_rect(Rect::new(x - margin, body.y0, x + PITCH + margin, body.y1).intersect(body));
        cx.invalidate_rect(Rect::new(x - margin, col.y0, x + PITCH + margin, col.y1).intersect(col));
        cx.invalidate_rect(Rect::new(row.x0, y - margin, row.x1, y + PITCH + margin).intersect(row));
    }

    fn notify(&self, cx: &mut EventCx, event: MatrixMsg) {
        let mapper = self.on_event.clone();
        cx.emit(&MsgFn::from_fn(move || mapper.make(event.clone())));
    }

    fn focus_cell(&self, cx: &mut EventCx, next: Cell) {
        let size = cx.size();
        let old = *cx.state::<State>();
        if old.focused == Some(next) {
            return;
        }
        if let Some(cell) = old.focused {
            self.damage_cell(cx, cell);
        }
        let (_, col, row, body) = Self::view(size);
        let x = self.layout.cols.offsets[next.col];
        let y = self.layout.rows.offsets[next.row];
        let after = {
            let st = cx.state::<State>();
            st.focused = Some(next);
            st.offset.x = st
                .offset
                .x
                .max(x + PITCH - body.width())
                .min(x)
                .clamp(0., (self.layout.cols.extent - body.width()).max(0.));
            st.offset.y = st
                .offset
                .y
                .max(y + PITCH - body.height())
                .min(y)
                .clamp(0., (self.layout.rows.extent - body.height()).max(0.));
            st.offset
        };
        let moved = after != old.offset;
        if moved {
            if after.x != old.offset.x {
                cx.invalidate_rect(col);
            }
            if after.y != old.offset.y {
                cx.invalidate_rect(row);
            }
            cx.invalidate_rect(body);
        } else {
            self.damage_cell(cx, next);
        }
        self.notify(cx, MatrixMsg::Focus(next));
    }
}

fn outline(cx: &mut PaintCx, r: Rect, color: Color, width: f64) {
    cx.canvas.fill_rect(Rect::new(r.x0, r.y0, r.x1, r.y0 + width), color);
    cx.canvas.fill_rect(Rect::new(r.x0, r.y1 - width, r.x1, r.y1), color);
    cx.canvas.fill_rect(Rect::new(r.x0, r.y0, r.x0 + width, r.y1), color);
    cx.canvas.fill_rect(Rect::new(r.x1 - width, r.y0, r.x1, r.y1), color);
}

fn kind_color(t: &Theme, kind: Kind) -> Color {
    t.role(match kind {
        Kind::Hardware => Role::Warning,
        Kind::Application => Role::Accent,
        Kind::Effect => Role::Violet,
        Kind::Virtual => Role::Cyan,
    })
}

fn group_icon(kind: Kind, name: &str) -> aizome_kit::Icon {
    let has =
        |needle: &str| name.as_bytes().windows(needle.len()).any(|part| part.eq_ignore_ascii_case(needle.as_bytes()));
    use aizome_kit::Icon;
    if has("usb") || has("scarlett") {
        Icon::Usb
    } else if has("headphone") {
        Icon::Headphones
    } else if has("speaker") || has("monitor") {
        Icon::Speaker
    } else if has("mic") {
        Icon::Mic
    } else if has("midi") || has("keyboard") {
        Icon::Keyboard
    } else if has("video") || has("camera") {
        Icon::Video
    } else {
        match kind {
            Kind::Hardware | Kind::Virtual => Icon::Plug,
            Kind::Application => Icon::AppWindow,
            Kind::Effect => Icon::SlidersHorizontal,
        }
    }
}

fn swatch(cx: &mut PaintCx, t: &Theme, state: CellState, x: f64, y: f64, size: f64) {
    if state == CellState::NotAllowed {
        return;
    }
    let square = Rect::new(x, y, x + size, y + size);
    let edge = match state {
        CellState::Empty | CellState::Count(_) => {
            if t.is_dark() {
                t.line()
            } else {
                t.line2()
            }
        }
        CellState::Connected | CellState::Half | CellState::Asked => t.role(Role::Accent),
        CellState::Error => t.role(Role::Danger),
        CellState::NotAllowed => unreachable!(),
    };
    cx.canvas.fill_rect(
        square,
        if matches!(state, CellState::Connected | CellState::Half) { t.selection() } else { t.sunk() },
    );
    if state == CellState::Asked {
        let width = if t.is_dark() { 1. } else { 1.5 };
        for step in 0..4 {
            let a = step as f64 * size / 4.;
            cx.canvas.fill_rect(Rect::new(x + a, y, x + a + size / 8., y + width), edge);
            cx.canvas.fill_rect(Rect::new(x + a, y + size - width, x + a + size / 8., y + size), edge);
            cx.canvas.fill_rect(Rect::new(x, y + a, x + width, y + a + size / 8.), edge);
            cx.canvas.fill_rect(Rect::new(x + size - width, y + a, x + size, y + a + size / 8.), edge);
        }
    } else {
        outline(cx, square, edge, 1.);
    }
    match state {
        CellState::Half => {
            for step in 2..(size as usize - 2) {
                let yy = y + step as f64;
                cx.canvas.fill_rect(Rect::new(x + 2., yy, x + size - step as f64, yy + 1.), edge);
            }
        }
        CellState::Asked => cx.canvas.fill_rect(
            Rect::new(x + (size - 6.) / 2., y + (size - 6.) / 2., x + (size + 6.) / 2., y + (size + 6.) / 2.),
            edge,
        ),
        CellState::Connected if t.is_dark() => {
            cx.canvas.fill_rect(Rect::new(x + 2., y + 2., x + size - 2., y + 4.), t.bevel_hi())
        }
        CellState::Count(n) if n > 0 => {
            cx.canvas.fill_rect(Rect::new(x + 2., y + size - 3., x + size - 2., y + size - 1.), t.role(Role::Accent));
            let spec = TextSpec::new(FontSpec::mono(if size < 15. { 9. } else { 11. }), t.text());
            cx.draw_text_in(&n.to_string(), &spec, Rect::new(x, y - 2., x + size, y + size - 2.), Align::Center);
        }
        _ => (),
    }
}

impl Widget for PatchMatrix {
    fn name(&self) -> &'static str {
        "patch-matrix"
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: Constraints) -> Size {
        let size = bc.constrain(Size::new(
            if bc.max.width.is_finite() { bc.max.width } else { 800. },
            if bc.max.height.is_finite() { bc.max.height } else { 600. },
        ));
        let st = cx.state::<State>();
        st.size = size;
        st.offset.x = st.offset.x.clamp(0., (self.layout.cols.extent - (size.width - ROW_HEADER).max(0.)).max(0.));
        st.offset.y = st.offset.y.clamp(0., (self.layout.rows.extent - (size.height - COLUMN_HEADER).max(0.)).max(0.));
        size
    }

    fn paint(&self, cx: &mut PaintCx) {
        let size = cx.size();
        let (corner, cols, rows, body) = Self::view(size);
        let (offset, hover, focus, keyboard, preview) =
            cx.state::<State>().map_or((Vec2::ZERO, None, None, false, None), |s| {
                (s.offset, s.hovered, s.focused, s.keyboard_focus, s.line)
            });
        let t = cx.theme().clone();
        cx.canvas.fill_rect(cx.bounds(), t.surface());
        cx.canvas.fill_rect(corner, t.chrome());
        cx.canvas.fill_rect(cols, t.chrome());
        cx.canvas.fill_rect(rows, t.chrome());
        cx.canvas.fill_rect(body, t.sunk());
        let caption = TextSpec::new(FontSpec::mono(11.), t.text2());
        let label = TextSpec::new(FontSpec::mono(11.), t.text3());
        let strong = TextSpec::new(FontSpec::mono(11.), t.text());
        let group_name = TextSpec::new(FontSpec { weight: 600., ..FontSpec::ui(12.5) }, t.text());
        cx.draw_text_in(
            if self.layout.swapped { "TO ↓" } else { "FROM ↓" },
            &caption,
            Rect::new(12., 10., corner.x1 - 4., 32.),
            Align::Start,
        );
        cx.draw_text_in(
            if self.layout.swapped { "FROM →" } else { "TO →" },
            &caption,
            Rect::new(12., 34., corner.x1 - 4., 56.),
            Align::Start,
        );
        for (i, (name, state)) in [
            ("connected", CellState::Connected),
            ("half", CellState::Half),
            ("asked", CellState::Asked),
            ("error", CellState::Error),
            ("count", CellState::Count(2)),
        ]
        .into_iter()
        .enumerate()
        {
            let x = 8. + (i / 3) as f64 * 92.;
            let y = 66. + (i % 3) as f64 * 20.;
            swatch(cx, &t, state, x, y + 2., 11.);
            cx.draw_text_in(
                name,
                &TextSpec::new(FontSpec::mono(9.), t.text2()),
                Rect::new(x + 15., y, x + 90., y + 16.),
                Align::Start,
            );
        }
        let mut kinds = Vec::new();
        for group in self.layout.rows.groups.iter().chain(&self.layout.cols.groups) {
            if !kinds.contains(&group.kind) {
                kinds.push(group.kind);
            }
        }
        for (i, kind) in kinds.into_iter().enumerate() {
            let x = 8. + (i / 2) as f64 * 92.;
            let y = 128. + (i % 2) as f64 * 20.;
            cx.canvas.fill_rect(Rect::new(x, y + 3., x + 11., y + 14.), kind_color(&t, kind));
            let name = match kind {
                Kind::Hardware => "hardware",
                Kind::Application => "app",
                Kind::Effect => "effect",
                Kind::Virtual => "virtual",
            };
            cx.draw_text_in(
                name,
                &TextSpec::new(FontSpec::mono(9.), t.text2()),
                Rect::new(x + 15., y, x + 90., y + 16.),
                Align::Start,
            );
        }
        cx.canvas.save();
        cx.canvas.clip_rect(cols);
        let visible_cols = self.layout.cols.visible(offset.x, offset.x + body.width());
        for group in &self.layout.cols.groups {
            let x = body.x0 + self.layout.cols.offsets[group.first] - offset.x;
            let width = self.layout.cols.offsets[group.end - 1] + PITCH - self.layout.cols.offsets[group.first];
            if x + width <= cols.x0 || x >= cols.x1 {
                continue;
            }
            cx.canvas.fill_rect(Rect::new(x + 2., 4., x + width - 2., 117.), t.raised());
            cx.canvas.fill_rect(Rect::new(x + 2., 4., x + width - 2., 7.), kind_color(&t, group.kind));
            cx.canvas.save();
            cx.canvas.clip_rect(Rect::new(x + 3., 8., x + width - 3., 116.));
            cx.canvas.translate(Vec2::new(x + width / 2. - 4., 99.));
            cx.canvas.rotate(-std::f64::consts::FRAC_PI_2);
            let text = cx.layout_text(&group.name, &group_name, Some(86.));
            cx.canvas.draw_text(&text, Point::ZERO);
            cx.canvas.restore();
            group_icon(group.kind, &group.name).draw(
                &mut cx.canvas,
                Point::new(x + width / 2. - 7., 98.),
                14.,
                t.text2(),
            );
        }
        for col in visible_cols.clone() {
            let x = body.x0 + self.layout.cols.offsets[col] - offset.x;
            cx.canvas.save();
            cx.canvas.translate(Vec2::new(x + 20., 174.));
            cx.canvas.rotate(-std::f64::consts::FRAC_PI_2);
            let spec = if hover.is_some_and(|h| h.col == col) { &strong } else { &label };
            let text = cx.layout_text(&self.layout.cols.labels[col], spec, None);
            cx.canvas.draw_text(&text, Point::ZERO);
            cx.canvas.restore();
        }
        cx.canvas.restore();
        cx.canvas.save();
        cx.canvas.clip_rect(rows);
        let visible_rows = self.layout.rows.visible(offset.y, offset.y + body.height());
        for group in &self.layout.rows.groups {
            let y = body.y0 + self.layout.rows.offsets[group.first] - offset.y;
            let height = self.layout.rows.offsets[group.end - 1] + PITCH - self.layout.rows.offsets[group.first];
            if y + height <= rows.y0 || y >= rows.y1 {
                continue;
            }
            cx.canvas.fill_rect(Rect::new(3., y, rows.x1 - 2., y + height), t.raised());
            cx.canvas.fill_rect(Rect::new(3., y, 6., y + height), kind_color(&t, group.kind));
            group_icon(group.kind, &group.name).draw(&mut cx.canvas, Point::new(9., y + 3.), 14., t.text2());
            cx.draw_text_in(&group.name, &group_name, Rect::new(25., y, rows.x1 - 59., y + 20.), Align::Start);
            if group.end > group.first + 1 {
                cx.draw_text_in(&group.detail, &label, Rect::new(25., y + 19., rows.x1 - 59., y + 38.), Align::Start);
            }
        }
        for row in visible_rows.clone() {
            let y = body.y0 + self.layout.rows.offsets[row] - offset.y;
            let spec = if hover.is_some_and(|h| h.row == row) { &strong } else { &label };
            cx.draw_text_in(
                &self.layout.rows.labels[row],
                spec,
                Rect::new(rows.x1 - 64., y, rows.x1 - 8., y + PITCH),
                Align::Start,
            );
        }
        cx.canvas.restore();
        cx.canvas.save();
        cx.canvas.clip_rect(body);
        if let Some(h) = hover {
            let x = body.x0 + self.layout.cols.offsets[h.col] - offset.x;
            let y = body.y0 + self.layout.rows.offsets[h.row] - offset.y;
            let tint = t.selection().with_alpha(0.16);
            cx.canvas.fill_rect(Rect::new(body.x0, y, body.x1, y + PITCH), tint);
            cx.canvas.fill_rect(Rect::new(x, body.y0, x + PITCH, body.y1), tint);
        }
        let preview_cells = preview.map(|(start, end)| line_cells(start, end));
        for row in visible_rows {
            let y = body.y0 + self.layout.rows.offsets[row] - offset.y;
            for col in visible_cols.clone() {
                let x = body.x0 + self.layout.cols.offsets[col] - offset.x;
                let cell = Cell { row, col };
                let state = self.layout.state(cell);
                swatch(cx, &t, state, x + 3., y + 3., 18.);
                if keyboard && focus == Some(cell) {
                    outline(cx, Rect::new(x + 1., y + 1., x + 23., y + 23.), t.focus(), 2.);
                }
                if preview_cells.as_ref().is_some_and(|cells| cells.contains(&cell)) {
                    outline(cx, Rect::new(x + 1., y + 1., x + 23., y + 23.), t.role(Role::Accent), 2.);
                }
            }
        }
        cx.canvas.restore();
        cx.canvas.fill_rect(Rect::new(ROW_HEADER - 1., 0., ROW_HEADER, size.height), t.line());
        cx.canvas.fill_rect(Rect::new(0., COLUMN_HEADER - 1., size.width, COLUMN_HEADER), t.line());
    }

    fn hash(&self, h: &mut PaintHasher) {
        // Layout is immutable per Rc: replacing any labels or sparse cell states changes identity.
        h.add(&(Rc::as_ptr(&self.layout) as usize, self.layout.revision));
    }

    fn hit(&self) -> bool {
        true
    }
    fn focusable(&self) -> Focusable {
        Focusable::Click
    }
    fn focus_ring(&self) -> FocusRing {
        FocusRing::Never
    }
    fn hover_paints(&self) -> bool {
        false
    }
    fn solid(&self) -> bool {
        true
    }

    fn event(&mut self, cx: &mut EventCx, event: &Event) -> Status {
        match event {
            Event::PointerMove { pos, .. } => {
                let pressed = cx.state::<State>().pressed;
                if let Some(start) = pressed {
                    if let Some(end) = self.cell(*pos, cx.state::<State>()) {
                        if start != end {
                            cx.state::<State>().line = Some((start, end));
                            cx.invalidate_rect(Self::view(cx.size()).3);
                        }
                    }
                }
                let next = self.cell(*pos, cx.state::<State>());
                let old = cx.state::<State>().hovered;
                if next != old {
                    if let Some(c) = old {
                        self.damage_cell(cx, c);
                    }
                    if let Some(c) = next {
                        self.damage_cell(cx, c);
                    }
                    cx.state::<State>().hovered = next;
                    self.notify(cx, MatrixMsg::Hover(next));
                }
                Status::Handled
            }
            Event::PointerLeave => {
                if let Some(old) = cx.state::<State>().hovered.take() {
                    self.damage_cell(cx, old);
                    self.notify(cx, MatrixMsg::Hover(None));
                }
                Status::Handled
            }
            Event::PointerDown { pos, button: PointerButton::Left, .. } => {
                if let Some(c) = self.cell(*pos, cx.state::<State>()) {
                    cx.request_focus();
                    cx.capture_pointer();
                    self.focus_cell(cx, c);
                    cx.state::<State>().pressed = Some(c);
                } else {
                    let st = cx.state::<State>();
                    let (_, columns, rows, _) = Self::view(st.size);
                    if rows.contains(*pos) {
                        if let Some(row) = self.layout.rows.at(pos.y - rows.y0 + st.offset.y) {
                            self.notify(cx, MatrixMsg::Select(Cell { row, col: 0 }, false));
                        }
                    } else if columns.contains(*pos) {
                        if let Some(col) = self.layout.cols.at(pos.x - columns.x0 + st.offset.x) {
                            self.notify(cx, MatrixMsg::Select(Cell { row: 0, col }, true));
                        }
                    }
                }
                Status::Handled
            }
            Event::PointerUp { pos, button: PointerButton::Left, .. } => {
                let start = cx.state::<State>().pressed.take();
                cx.release_pointer();
                if let Some(start) = start {
                    let end = self.cell(*pos, cx.state::<State>()).unwrap_or(start);
                    cx.state::<State>().line = None;
                    cx.invalidate_rect(Self::view(cx.size()).3);
                    if start == end {
                        self.notify(cx, MatrixMsg::Toggle(start));
                    } else {
                        self.notify(
                            cx,
                            MatrixMsg::Line(line_cells(start, end), self.layout.state(start) != CellState::Connected),
                        );
                    }
                }
                Status::Handled
            }
            Event::PointerDown { pos, button: PointerButton::Right, .. } => {
                let st = cx.state::<State>();
                let (_, columns, rows, _) = Self::view(st.size);
                let cell = self
                    .cell(*pos, st)
                    .map(|cell| (cell, false))
                    .or_else(|| {
                        rows.contains(*pos)
                            .then(|| {
                                self.layout
                                    .rows
                                    .at(pos.y - rows.y0 + st.offset.y)
                                    .map(|row| (Cell { row, col: 0 }, false))
                            })
                            .flatten()
                    })
                    .or_else(|| {
                        columns
                            .contains(*pos)
                            .then(|| {
                                self.layout
                                    .cols
                                    .at(pos.x - columns.x0 + st.offset.x)
                                    .map(|col| (Cell { row: 0, col }, true))
                            })
                            .flatten()
                    });
                if let Some((cell, column)) = cell {
                    self.notify(cx, MatrixMsg::Context(cell, column));
                }
                Status::Handled
            }
            Event::Scroll { delta, .. } => {
                let (corner, col, row, body) = Self::view(cx.size());
                let (before, after) = {
                    let st = cx.state::<State>();
                    let before = st.offset;
                    st.offset.x = (st.offset.x + delta.x).clamp(0., (self.layout.cols.extent - body.width()).max(0.));
                    st.offset.y = (st.offset.y + delta.y).clamp(0., (self.layout.rows.extent - body.height()).max(0.));
                    (before, st.offset)
                };
                if after == before {
                    return Status::Ignored;
                }
                cx.invalidate_rect(col);
                if after.y != before.y {
                    cx.invalidate_rect(row);
                }
                cx.invalidate_rect(body);
                cx.invalidate_rect(corner);
                Status::Handled
            }
            Event::Focus(active) => {
                cx.state::<State>().keyboard_focus = *active;
                if let Some(c) = cx.state::<State>().focused {
                    self.damage_cell(cx, c);
                }
                Status::Handled
            }
            Event::Key(k)
                if k.pressed && !self.layout.rows.labels.is_empty() && !self.layout.cols.labels.is_empty() =>
            {
                let current = cx.state::<State>().focused.unwrap_or(Cell { row: 0, col: 0 });
                let mut next = current;
                let row_count = self.layout.rows.labels.len();
                let col_count = self.layout.cols.labels.len();
                let page = ((cx.size().height - COLUMN_HEADER) / PITCH).floor().max(1.) as usize;
                match &k.key {
                    Key::Named(NamedKey::ArrowLeft) => {
                        next.col = if k.modifiers.alt() {
                            self.layout.cols.next_group(next.col, false)
                        } else {
                            next.col.saturating_sub(1)
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        next.col = if k.modifiers.alt() {
                            self.layout.cols.next_group(next.col, true)
                        } else {
                            (next.col + 1).min(col_count - 1)
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        next.row = if k.modifiers.alt() {
                            self.layout.rows.next_group(next.row, false)
                        } else {
                            next.row.saturating_sub(1)
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        next.row = if k.modifiers.alt() {
                            self.layout.rows.next_group(next.row, true)
                        } else {
                            (next.row + 1).min(row_count - 1)
                        }
                    }
                    Key::Named(NamedKey::Home) => next.col = 0,
                    Key::Named(NamedKey::End) => next.col = col_count - 1,
                    Key::Named(NamedKey::PageUp) => next.row = next.row.saturating_sub(page),
                    Key::Named(NamedKey::PageDown) => next.row = (next.row + page).min(row_count - 1),
                    Key::Named(NamedKey::Escape)
                        if cx.state::<State>().line.is_some() || cx.state::<State>().pressed.is_some() =>
                    {
                        cx.state::<State>().line = None;
                        cx.state::<State>().pressed = None;
                        cx.invalidate_rect(Self::view(cx.size()).3);
                        return Status::Handled;
                    }
                    Key::Character(s) if s == " " => {
                        if let Some((start, end)) = cx.state::<State>().line.take() {
                            cx.invalidate_rect(Self::view(cx.size()).3);
                            self.notify(
                                cx,
                                MatrixMsg::Line(
                                    line_cells(start, end),
                                    self.layout.state(start) != CellState::Connected,
                                ),
                            );
                        } else if k.modifiers.shift() {
                            cx.state::<State>().line = Some((current, current));
                            cx.invalidate_rect(Self::view(cx.size()).3);
                        } else {
                            self.notify(cx, MatrixMsg::Toggle(current));
                        }
                        return Status::Handled;
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.notify(cx, MatrixMsg::Focus(current));
                        return Status::Handled;
                    }
                    _ => return Status::Ignored,
                }
                self.focus_cell(cx, next);
                if let Some((start, _)) = cx.state::<State>().line {
                    cx.state::<State>().line = Some((start, next));
                    cx.invalidate_rect(Self::view(cx.size()).3);
                }
                Status::Handled
            }
            _ => Status::Ignored,
        }
    }

    fn semantics(&self) -> Option<Semantics> {
        // The full enumerable link table is a distinct view in S4. This widget
        // exposes its current cell without allocating O(rows × columns) nodes.
        Some(Semantics {
            role: SemanticRole::Table,
            label: Some("Patch matrix, source rows and destination columns".into()),
            ..Default::default()
        })
    }

    fn parts(&self) -> Vec<(String, Rect)> {
        Vec::new()
    }
}
