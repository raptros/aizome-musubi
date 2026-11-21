//! Mixer controls. Values passed to the fader are the *linear control* values;
//! the audible gain is their cube, as in the PipeWire/Pulse volume convention.
use std::time::{Duration, Instant};

use aizome_kit::element::{MsgFn, MsgMap};
use aizome_kit::hash::PaintHasher;
use aizome_kit::layout::Constraints;
use aizome_kit::sidestep_ui::{Cursor, Ime, Key, NamedKey, PointerButton};
use aizome_kit::text::TextSpec;
use aizome_kit::widget::{
    Event, EventCx, Focusable, LayoutCx, PaintCx, SemanticAction, SemanticRole, Semantics, Status, Widget,
};
use aizome_kit::{Align, Element, FontSpec, Rect, Role, Size};

const MAX_LINEAR: f64 = 1.5;
const MIN_DB: f64 = -60.;
const HOLD: Duration = Duration::from_millis(1500);
const MAX_DB: f64 = 10.565_475_543_7; // 20 log10(1.5³)
/// Linear control value to audible decibels; zero is negative infinity.
pub fn linear_to_db(value: f64) -> f64 {
    if value <= 0. { f64::NEG_INFINITY } else { 60. * value.log10() }
}

/// Audible decibels to a bounded linear control value.
pub fn db_to_linear(db: f64) -> f64 {
    if db.is_infinite() && db.is_sign_negative() { 0. } else { 10_f64.powf(db / 60.).clamp(0., MAX_LINEAR) }
}

fn db_text(db: f64) -> String {
    if db.is_infinite() && db.is_sign_negative() {
        "−∞ dB".into()
    } else {
        format!("{:.1} dB", db).replace('-', "−")
    }
}

fn parse_value(value: &str, percent: bool) -> Option<f64> {
    let trimmed = value.trim().replace('−', "-");
    if trimmed == "-∞" || trimmed.eq_ignore_ascii_case("-inf") || trimmed.eq_ignore_ascii_case("-infinity") {
        return Some(0.);
    }
    let number = trimmed.strip_suffix('%').or_else(|| trimmed.strip_suffix("dB")).unwrap_or(&trimmed).trim();
    let parsed = number.parse::<f64>().ok()?;
    if !parsed.is_finite() {
        return None;
    }
    if percent { Some((parsed / 100.).clamp(0., MAX_LINEAR)) } else { Some(db_to_linear(parsed)) }
}

#[derive(Default)]
struct FaderState {
    dragging: bool,
    editing: Option<String>,
    tooltip: bool,
}

/// Vertical volume control; `.value` is the observed linear value, never an optimistic write.
pub struct Fader {
    value: f64,
    percent: bool,
    read_only_reason: Option<String>,
    on_change: MsgMap<f64>,
}

impl Fader {
    pub fn new<Msg: Send + 'static>(
        value: f64,
        percent: bool,
        on_change: impl Fn(f64) -> Msg + 'static,
    ) -> Element<Msg> {
        Self::with_reason(value, percent, None, on_change)
    }

    pub fn with_reason<Msg: Send + 'static>(
        value: f64,
        percent: bool,
        reason: Option<String>,
        on_change: impl Fn(f64) -> Msg + 'static,
    ) -> Element<Msg> {
        Element::new(
            Self {
                value: value.clamp(0., MAX_LINEAR),
                percent,
                read_only_reason: reason,
                on_change: MsgMap::new(on_change),
            },
            vec![],
        )
    }

    fn track(size: Size) -> Rect {
        Rect::new(34., 12., 58., (size.height - 48.).max(36.))
    }
    fn box_rect(size: Size) -> Rect {
        Rect::new(0., size.height - 28., size.width, size.height - 4.)
    }
    fn at(&self, y: f64, size: Size) -> f64 {
        let track = Self::track(size);
        let ratio = ((track.y1 - y) / track.height()).clamp(0., 1.);
        if ratio == 0. { 0. } else { db_to_linear(MIN_DB + ratio * (MAX_DB - MIN_DB)) }
    }
    fn y(&self, db: f64, size: Size) -> f64 {
        let track = Self::track(size);
        track.y1 - ((db.max(MIN_DB) - MIN_DB) / (MAX_DB - MIN_DB)) * track.height()
    }
    fn send(&mut self, cx: &mut EventCx, value: f64) {
        let value = value.clamp(0., MAX_LINEAR);
        if (value - self.value).abs() > 1e-9 {
            self.value = value;
            let map = self.on_change.clone();
            cx.emit(&MsgFn::from_fn(move || map.make(value)));
            cx.invalidate_rect(cx.bounds());
        }
    }
    fn display(&self) -> String {
        if self.percent { format!("{:.0}%", self.value * 100.) } else { db_text(linear_to_db(self.value)) }
    }
}

impl Widget for Fader {
    fn name(&self) -> &'static str {
        "mixer-fader"
    }
    fn layout(&mut self, _cx: &mut LayoutCx, bc: Constraints) -> Size {
        bc.constrain(Size::new(76., 300.))
    }
    fn paint(&self, cx: &mut PaintCx) {
        let t = cx.theme();
        let size = cx.size();
        let track = Self::track(size);
        let disabled = self.read_only_reason.is_some();
        let caption = TextSpec::new(FontSpec::mono(t.metrics.caption), t.text2());
        for (db, text) in
            [(6., "+6"), (0., "0"), (-6., "−6"), (-12., "−12"), (-24., "−24"), (-36., "−36"), (-60., "−60")].into_iter()
        {
            let y = self.y(db, size);
            cx.draw_text_in(text, &caption, Rect::new(0., y - 10., 31., y + 10.), Align::End);
            cx.canvas.fill_rect(Rect::new(30., y, 33., y + 1.), t.line2());
        }
        cx.draw_text_in("−∞", &caption, Rect::new(0., track.y1 + 2., 32., track.y1 + 22.), Align::End);
        cx.canvas.fill_rect(Rect::new(44., track.y0, 46., track.y1), t.line2());
        let y = if self.value == 0. { track.y1 } else { self.y(linear_to_db(self.value), size) };
        if !disabled {
            cx.canvas.fill_rect(Rect::new(44., y, 46., track.y1), t.selection());
        }
        let thumb = Rect::new(33., y - 7., 57., y + 7.);
        cx.canvas.fill_rect(thumb, if disabled { t.line() } else { t.raised() });
        aizome_kit::draw::border(cx.canvas, thumb, t.line2(), 1.);
        let box_rect = Self::box_rect(size);
        cx.canvas.fill_rect(box_rect, t.sunk());
        aizome_kit::draw::border(cx.canvas, box_rect, t.line2(), 1.);
        let value =
            cx.state::<FaderState>().and_then(|s| s.editing.as_deref()).map_or_else(|| self.display(), str::to_owned);
        cx.draw_text_in(
            &value,
            &TextSpec::new(FontSpec::mono(t.metrics.small), if disabled { t.text3() } else { t.text() }),
            box_rect.inset(3.),
            Align::Center,
        );
        if cx.state::<FaderState>().is_some_and(|s| s.editing.is_some()) {
            cx.canvas.fill_rect(
                Rect::new(box_rect.x1 - 5., box_rect.y0 + 4., box_rect.x1 - 4., box_rect.y1 - 4.),
                t.focus(),
            );
        }
        if disabled && cx.state::<FaderState>().is_some_and(|s| s.tooltip) {
            if let Some(reason) = &self.read_only_reason {
                let tip = Rect::new(2., 2., size.width - 2., 29.);
                cx.canvas.fill_rect(tip, t.raised());
                cx.draw_text_in(
                    reason,
                    &TextSpec::new(FontSpec::ui(t.metrics.small), t.text()),
                    tip.inset(3.),
                    Align::Center,
                );
            }
        }
    }
    fn hash(&self, h: &mut PaintHasher) {
        h.f64(self.value).add(&(self.percent, &self.read_only_reason));
    }
    fn hit(&self) -> bool {
        true
    }
    fn focusable(&self) -> Focusable {
        Focusable::Tab
    }
    fn cursor(&self) -> Option<Cursor> {
        self.read_only_reason.is_none().then_some(Cursor::Pointer)
    }
    fn hover_paints(&self) -> bool {
        false
    }
    fn event(&mut self, cx: &mut EventCx, event: &Event) -> Status {
        if self.read_only_reason.is_some() {
            match event {
                Event::PointerMove { .. } => {
                    cx.set_timer(1, Duration::from_millis(650));
                    Status::Handled
                }
                Event::Timer(1) => {
                    cx.state::<FaderState>().tooltip = true;
                    cx.invalidate_rect(cx.bounds());
                    Status::Handled
                }
                Event::PointerLeave => {
                    cx.cancel_timer(1);
                    cx.state::<FaderState>().tooltip = false;
                    cx.invalidate_rect(cx.bounds());
                    Status::Handled
                }
                _ => Status::Ignored,
            }
        } else {
            match event {
                Event::PointerDown { pos, button: PointerButton::Left, clicks, .. } => {
                    cx.request_focus();
                    if *clicks >= 2 && Self::box_rect(cx.size()).contains(*pos) {
                        cx.state::<FaderState>().editing = Some(self.display());
                        cx.invalidate_rect(Self::box_rect(cx.size()));
                    } else if !Self::box_rect(cx.size()).contains(*pos) {
                        cx.state::<FaderState>().dragging = true;
                        cx.capture_pointer();
                        self.send(cx, self.at(pos.y, cx.size()));
                    }
                    Status::Handled
                }
                Event::PointerMove { pos, .. } if cx.state::<FaderState>().dragging => {
                    self.send(cx, self.at(pos.y, cx.size()));
                    Status::Handled
                }
                Event::PointerUp { button: PointerButton::Left, .. } => {
                    cx.state::<FaderState>().dragging = false;
                    cx.release_pointer();
                    Status::Handled
                }
                Event::Scroll { delta, .. } => {
                    let db = if self.value == 0. { MIN_DB } else { linear_to_db(self.value) };
                    self.send(cx, db_to_linear(db - delta.y.signum()));
                    Status::Handled
                }
                Event::Key(k) if k.pressed => {
                    if let Some(buffer) = cx.state::<FaderState>().editing.as_mut() {
                        match &k.key {
                            Key::Named(NamedKey::Escape) => {
                                cx.state::<FaderState>().editing = None;
                            }
                            Key::Named(NamedKey::Enter) => {
                                if let Some(value) = parse_value(buffer, self.percent) {
                                    cx.state::<FaderState>().editing = None;
                                    self.send(cx, value);
                                }
                            }
                            Key::Named(NamedKey::Backspace) => {
                                buffer.pop();
                            }
                            Key::Character(_) => {
                                if let Some(text) = &k.text {
                                    buffer.push_str(text);
                                }
                            }
                            _ => return Status::Ignored,
                        }
                        cx.invalidate_rect(Self::box_rect(cx.size()));
                        return Status::Handled;
                    }
                    let db = if self.value == 0. { MIN_DB } else { linear_to_db(self.value) };
                    let new = match k.key {
                        Key::Named(NamedKey::ArrowUp) => db_to_linear(db + 1.),
                        Key::Named(NamedKey::ArrowDown) => db_to_linear(db - 1.),
                        Key::Named(NamedKey::PageUp) => db_to_linear(db + 6.),
                        Key::Named(NamedKey::PageDown) => db_to_linear(db - 6.),
                        Key::Named(NamedKey::Home) => 1.,
                        Key::Named(NamedKey::End) => 0.,
                        _ => return Status::Ignored,
                    };
                    self.send(cx, new);
                    Status::Handled
                }
                Event::Ime(Ime::Commit(text)) if cx.state::<FaderState>().editing.is_some() => {
                    cx.state::<FaderState>().editing.as_mut().unwrap().push_str(text);
                    cx.invalidate_rect(Self::box_rect(cx.size()));
                    Status::Handled
                }
                _ => Status::Ignored,
            }
        }
    }
    fn semantics(&self) -> Option<Semantics> {
        Some(Semantics {
            role: SemanticRole::Slider,
            label: Some(self.read_only_reason.as_deref().map_or_else(|| "Volume".into(), |r| format!("Volume — {r}"))),
            value: Some(db_text(linear_to_db(self.value))),
            range: Some((0., MAX_LINEAR, self.value)),
            disabled: self.read_only_reason.is_some(),
            actions: if self.read_only_reason.is_some() {
                vec![]
            } else {
                vec![SemanticAction::Increment, SemanticAction::Decrement, SemanticAction::SetValue]
            },
            ..Default::default()
        })
    }
}

#[derive(Default)]
struct MeterState {
    held: [f32; 8],
    until: [Option<Instant>; 8],
    shown: [f32; 8],
}

/// One segmented bar per channel. `peaks` are linear audio amplitudes (not fader values).
pub struct LevelMeter {
    peaks: [f32; 8],
    channels: usize,
    now: Instant,
    paused: bool,
}

impl LevelMeter {
    pub fn new(peaks: &[f32], now: Instant, paused: bool) -> Element<()> {
        Self::element(peaks, now, paused)
    }
    pub fn element<Msg>(peaks: &[f32], now: Instant, paused: bool) -> Element<Msg> {
        let mut values = [0.; 8];
        let channels = peaks.len().clamp(1, 8);
        for (dest, peak) in values.iter_mut().zip(peaks) {
            *dest = if peak.is_finite() { peak.max(0.) } else { 0. };
        }
        Element::new(Self { peaks: values, channels, now, paused }, vec![])
    }
    fn db(peak: f32) -> f64 {
        if peak <= 0. { f64::NEG_INFINITY } else { 20. * f64::from(peak).log10() }
    }
    fn segment(db: f64) -> usize {
        (((db.max(-60.) + 60.) / 66.) * 32.).ceil().clamp(0., 32.) as usize
    }
}

impl Widget for LevelMeter {
    fn name(&self) -> &'static str {
        "mixer-level-meter"
    }
    fn layout(&mut self, cx: &mut LayoutCx, bc: Constraints) -> Size {
        let st = cx.state::<MeterState>();
        if !self.paused {
            for i in 0..self.channels {
                let peak = self.peaks[i];
                st.shown[i] = peak;
                if peak >= st.held[i] || st.until[i].is_none_or(|deadline| self.now >= deadline) {
                    st.held[i] = peak;
                    st.until[i] = Some(self.now + HOLD);
                }
            }
        }
        if let Some(next) = st.until[..self.channels].iter().flatten().filter(|&&time| time > self.now).min().copied() {
            cx.set_timer(1, next.duration_since(self.now));
        }
        bc.constrain(Size::new((self.channels as f64 * 9. + 3.).max(24.), 260.))
    }
    fn paint(&self, cx: &mut PaintCx) {
        let t = cx.theme();
        let st = cx.state::<MeterState>();
        let size = cx.size();
        let step = (size.height - 8.) / 32.;
        let width = ((size.width - 3.) / self.channels as f64).min(12.);
        for channel in 0..self.channels {
            let count = if self.paused { 0 } else { Self::segment(Self::db(st.map_or(0., |s| s.shown[channel]))) };
            let held = if self.paused { 0 } else { Self::segment(Self::db(st.map_or(0., |s| s.held[channel]))) };
            for index in 0..32 {
                let db = -60. + 66. * index as f64 / 32.;
                let role = if db > 0. {
                    Role::Danger
                } else if db > -9. {
                    Role::Warning
                } else {
                    Role::Success
                };
                let active = index < count || index + 1 == held;
                let color = if active { t.role(role) } else { t.line2() };
                let y = size.height - 4. - (index + 1) as f64 * step;
                cx.canvas.fill_rect(
                    Rect::new(
                        2. + channel as f64 * width,
                        y,
                        1. + (channel + 1) as f64 * width,
                        y + (step - 2.).max(1.),
                    ),
                    color,
                );
            }
        }
        if self.paused {
            cx.draw_text_in(
                "paused",
                &TextSpec::new(FontSpec::ui(t.metrics.caption), t.text2()),
                Rect::new(0., size.height / 2. - 10., size.width, size.height / 2. + 10.),
                Align::Center,
            );
        }
    }
    fn hash(&self, h: &mut PaintHasher) {
        h.add(&(self.channels, self.paused));
        for v in &self.peaks[..self.channels] {
            h.add(&v.to_bits());
        }
    }
    fn event(&mut self, cx: &mut EventCx, event: &Event) -> Status {
        if matches!(event, Event::Timer(1)) {
            let st = cx.state::<MeterState>();
            if let Some(first) = st.until[..self.channels].iter().flatten().min().copied() {
                for i in 0..self.channels {
                    if st.until[i].is_some_and(|deadline| deadline <= first) {
                        st.held[i] = st.shown[i];
                        st.until[i] = None;
                    }
                }
                if let Some(next) = st.until[..self.channels].iter().flatten().min().copied() {
                    cx.set_timer(1, next.duration_since(first));
                }
            }
            cx.invalidate_rect(cx.bounds());
            return Status::Handled;
        }
        Status::Ignored
    }
    fn semantics(&self) -> Option<Semantics> {
        let peak = self.peaks[..self.channels].iter().copied().fold(0_f32, f32::max);
        Some(Semantics {
            role: SemanticRole::Meter,
            label: Some("Level peak".into()),
            value: Some(if self.paused { "paused".into() } else { db_text(Self::db(peak)) }),
            range: Some((-60., 6., Self::db(peak).max(-60.))),
            disabled: self.paused,
            ..Default::default()
        })
    }
}
