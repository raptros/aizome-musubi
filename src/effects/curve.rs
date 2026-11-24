//! Response plot and direct manipulation of the observed EQ bands.
use super::{Band, BandKind};
use aizome_kit::element::MsgMap;
use aizome_kit::hash::PaintHasher;
use aizome_kit::layout::Constraints;
use aizome_kit::sidestep_ui::{Cursor, Key, NamedKey, PointerButton};
use aizome_kit::text::TextSpec;
use aizome_kit::widget::{Event, EventCx, Focusable, LayoutCx, PaintCx, SemanticRole, Semantics, Status, Widget};
use aizome_kit::{Align, Element, FontSpec, Rect, Role, Size};

const RATE: f64 = 48_000.;
const LEFT: f64 = 43.;
const TOP: f64 = 12.;
const RIGHT: f64 = 12.;
const BOTTOM: f64 = 22.;

/// Biquad magnitude at `frequency`, in dB; the same Freq/Q/Gain controls are sent
/// to PipeWire's bq_peaking, bq_lowshelf and bq_highshelf builtins.
pub fn response(bands: &[Band], frequency: f64) -> f64 {
    let w = 2. * std::f64::consts::PI * frequency / RATE;
    let cos = w.cos();
    let sin = w.sin();
    let mut total = 0.;
    for band in bands {
        let w0 = 2. * std::f64::consts::PI * band.freq as f64 / RATE;
        let c = w0.cos();
        let a = 10_f64.powf(band.gain as f64 / 40.);
        let alpha = w0.sin() / (2. * band.q as f64);
        let (b0, b1, b2, a0, a1, a2) = match band.kind {
            BandKind::Peaking => (1. + alpha * a, -2. * c, 1. - alpha * a, 1. + alpha / a, -2. * c, 1. - alpha / a),
            BandKind::LowShelf | BandKind::HighShelf => {
                let root = 2. * a.sqrt() * alpha;
                let ap = a + 1.;
                let am = a - 1.;
                if band.kind == BandKind::LowShelf {
                    (
                        a * (ap - am * c + root),
                        2. * a * (am - ap * c),
                        a * (ap - am * c - root),
                        ap + am * c + root,
                        -2. * (am + ap * c),
                        ap + am * c - root,
                    )
                } else {
                    (
                        a * (ap + am * c + root),
                        -2. * a * (am + ap * c),
                        a * (ap + am * c - root),
                        ap - am * c + root,
                        2. * (am - ap * c),
                        ap - am * c - root,
                    )
                }
            }
        };
        let magnitude2 = |x0: f64, x1: f64, x2: f64| {
            let re = x0 + x1 * cos + x2 * (2. * cos * cos - 1.);
            let im = -x1 * sin - x2 * 2. * sin * cos;
            re * re + im * im
        };
        total += 10. * (magnitude2(b0, b1, b2) / magnitude2(a0, a1, a2)).log10();
    }
    total
}

#[derive(Default)]
struct CurveState {
    selected: usize,
    dragging: bool,
}

pub struct Curve {
    bands: Vec<Band>,
    change: MsgMap<(usize, Band)>,
    end: MsgMap<()>,
}
impl Curve {
    pub fn new<Msg: Clone + Send + 'static>(
        bands: Vec<Band>,
        change: impl Fn((usize, Band)) -> Msg + 'static,
        end: impl Fn(()) -> Msg + 'static,
    ) -> Element<Msg> {
        Element::new(Self { bands, change: MsgMap::new(change), end: MsgMap::new(end) }, vec![])
    }
    fn plot(size: Size) -> Rect {
        Rect::new(LEFT, TOP, (size.width - RIGHT).max(LEFT + 1.), (size.height - BOTTOM).max(TOP + 1.))
    }
    fn x(freq: f64, r: Rect) -> f64 {
        r.x0 + (freq / 20.).log10() / 3. * r.width()
    }
    fn y(gain: f64, r: Rect) -> f64 {
        r.y0 + (15. - gain) / 30. * r.height()
    }
    fn hit_band(&self, pos: aizome_kit::Point, r: Rect) -> Option<usize> {
        self.bands.iter().enumerate().find_map(|(i, band)| {
            let x = Self::x(band.freq as f64, r);
            let y = Self::y(band.gain as f64, r);
            ((pos.x - x).abs() <= 13. && (pos.y - y).abs() <= 13.).then_some(i)
        })
    }
    fn send(&mut self, cx: &mut EventCx, index: usize, band: Band) {
        if self.bands.get(index) != Some(&band) {
            self.bands[index] = band.clone();
            let change = self.change.clone();
            cx.emit(&aizome_kit::element::MsgFn::from_fn(move || change.make((index, band.clone()))));
            cx.invalidate_rect(cx.bounds());
        }
    }
}
impl Widget for Curve {
    fn name(&self) -> &'static str {
        "eq-response"
    }
    fn layout(&mut self, _cx: &mut LayoutCx, bc: Constraints) -> Size {
        bc.constrain(Size::new(600., 220.))
    }
    fn paint(&self, cx: &mut PaintCx) {
        let t = cx.theme();
        let r = Self::plot(cx.size());
        cx.canvas.fill_rect(cx.bounds(), t.sunk());
        let caption = TextSpec::new(FontSpec::mono(t.metrics.caption), t.text3());
        for (gain, label) in
            [(15., "+15"), (10., "+10"), (5., "+5"), (0., "0"), (-5., "−5"), (-10., "−10"), (-15., "−15")]
        {
            let y = Self::y(gain, r);
            cx.canvas.fill_rect(Rect::new(r.x0, y, r.x1, y + 1.), t.line());
            cx.draw_text_in(label, &caption, Rect::new(0., y - 9., r.x0 - 5., y + 9.), Align::End);
        }
        for (freq, label) in [
            (20., "20"),
            (50., "50"),
            (100., "100"),
            (200., "200"),
            (500., "500"),
            (1000., "1k"),
            (2000., "2k"),
            (5000., "5k"),
            (10000., "10k"),
            (20000., "20k"),
        ] {
            let x = Self::x(freq, r);
            cx.canvas.fill_rect(Rect::new(x, r.y0, x + 1., r.y1), t.line());
            cx.draw_text_in(label, &caption, Rect::new(x - 16., r.y1 + 3., x + 16., r.y1 + 20.), Align::Center);
        }
        let mut last: Option<(f64, f64)> = None;
        for pixel in 0..=r.width() as usize {
            let x = r.x0 + pixel as f64;
            let freq = 20. * 1000_f64.powf(pixel as f64 / r.width());
            let y = Self::y(response(&self.bands, freq).clamp(-15., 15.), r);
            if let Some((prev_x, prev_y)) = last {
                // Short axis-aligned spans connect successive samples without a new palette.
                cx.canvas.fill_rect(Rect::new(prev_x, prev_y.min(y), x + 1., prev_y.max(y) + 1.), t.role(Role::Accent));
            }
            last = Some((x, y));
        }
        let selected = cx.state::<CurveState>().map_or(0, |s| s.selected);
        for (index, band) in self.bands.iter().enumerate() {
            let x = Self::x(band.freq as f64, r);
            let y = Self::y(band.gain as f64, r);
            let box_ = Rect::new(x - 9., y - 9., x + 9., y + 9.);
            cx.canvas.fill_rect(box_, if index == selected { t.selection() } else { t.raised() });
            aizome_kit::draw::border(cx.canvas, box_, if index == selected { t.focus() } else { t.line2() }, 1.);
            cx.draw_text_in(
                &(index + 1).to_string(),
                &TextSpec::new(FontSpec::mono(t.metrics.small), t.text()),
                box_,
                Align::Center,
            );
        }
    }
    fn hash(&self, h: &mut PaintHasher) {
        h.add(&self.bands.len());
        for b in &self.bands {
            h.f64(b.freq as f64).f64(b.gain as f64).f64(b.q as f64);
        }
    }
    fn hit(&self) -> bool {
        true
    }
    fn focusable(&self) -> Focusable {
        Focusable::Tab
    }
    fn cursor(&self) -> Option<Cursor> {
        Some(Cursor::Pointer)
    }
    fn event(&mut self, cx: &mut EventCx, event: &Event) -> Status {
        let r = Self::plot(cx.size());
        match event {
            Event::PointerDown { pos, button: PointerButton::Left, .. } => {
                if let Some(index) = self.hit_band(*pos, r) {
                    cx.state::<CurveState>().selected = index;
                    cx.state::<CurveState>().dragging = true;
                    cx.request_focus();
                    cx.capture_pointer();
                    cx.invalidate_rect(cx.bounds());
                    Status::Handled
                } else {
                    Status::Ignored
                }
            }
            Event::PointerMove { pos, .. } if cx.state::<CurveState>().dragging => {
                let index = cx.state::<CurveState>().selected;
                let mut band = self.bands[index].clone();
                band.freq = (20. * 1000_f64.powf(((pos.x - r.x0) / r.width()).clamp(0., 1.))) as f32;
                band.gain = (15. - 30. * ((pos.y - r.y0) / r.height()).clamp(0., 1.)) as f32;
                self.send(cx, index, band);
                Status::Handled
            }
            Event::PointerUp { button: PointerButton::Left, .. } if cx.state::<CurveState>().dragging => {
                cx.state::<CurveState>().dragging = false;
                cx.release_pointer();
                let end = self.end.clone();
                cx.emit(&aizome_kit::element::MsgFn::from_fn(move || end.make(())));
                Status::Handled
            }
            Event::Scroll { delta, .. } if !self.bands.is_empty() => {
                let index = cx.state::<CurveState>().selected.min(self.bands.len() - 1);
                let mut band = self.bands[index].clone();
                band.q = (band.q + (delta.y.signum() * -0.1) as f32).clamp(0.1, 20.);
                self.send(cx, index, band);
                Status::Handled
            }
            Event::Key(k) if k.pressed && !self.bands.is_empty() => {
                let index = cx.state::<CurveState>().selected.min(self.bands.len() - 1);
                let mut band = self.bands[index].clone();
                let step = if k.modifiers.shift() { 5. } else { 1. };
                match &k.key {
                    Key::Named(NamedKey::ArrowUp) => band.gain = (band.gain + step).min(15.),
                    Key::Named(NamedKey::ArrowDown) => band.gain = (band.gain - step).max(-15.),
                    Key::Named(NamedKey::ArrowRight) => {
                        band.freq = (band.freq as f64 * if step > 1. { 1.25 } else { 1.05 }).min(20_000.) as f32
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        band.freq = (band.freq as f64 / if step > 1. { 1.25 } else { 1.05 }).max(20.) as f32
                    }
                    Key::Character(c) if c == "[" => band.q = (band.q - 0.1 * step).max(0.1),
                    Key::Character(c) if c == "]" => band.q = (band.q + 0.1 * step).min(20.),
                    _ => return Status::Ignored,
                }
                self.send(cx, index, band);
                Status::Handled
            }
            _ => Status::Ignored,
        }
    }
    fn semantics(&self) -> Option<Semantics> {
        Some(Semantics {
            role: SemanticRole::Slider,
            label: Some(
                "EQ response; select a numbered band and use arrows for frequency and gain, brackets for Q".into(),
            ),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peaking_centre_gain() {
        for gain in [-15., -7., 0., 6., 12., 15.] {
            let band = Band { kind: BandKind::Peaking, freq: 1000., gain, q: 1.2 };
            assert!((response(&[band], 1000.) - gain as f64).abs() < 0.1);
        }
    }
}
