use aizome_kit::sidestep_ui::{Modifiers, NamedKey, PointerButton};
use aizome_kit::testing::{Harness, assert_golden};
use aizome_kit::theme::ThemeChoice;
use aizome_kit::widget::SemanticRole;
use aizome_kit::{Appearance, Cx, Element, Layoutable, Point, Program, Role, WindowKey, WindowSpec, row, text};
use aizome_musubi::mixer::{Fader, LevelMeter, db_to_linear, linear_to_db};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
enum Msg {
    Volume(f64),
    Peaks([f32; 2], Instant),
}
struct Demo {
    value: f64,
    peaks: [f32; 2],
    reason: Option<String>,
    now: Instant,
    changes: usize,
}
impl Demo {
    fn new(value: f64, peaks: [f32; 2], disabled: bool) -> Self {
        Self {
            value,
            peaks,
            reason: disabled.then(|| "Ambiguous PipeWire identity".into()),
            now: Instant::now(),
            changes: 0,
        }
    }
}
impl Program for Demo {
    type Msg = Msg;
    fn info(&self) -> aizome_kit::AppInfo {
        aizome_kit::AppInfo::new("org.aizome.Musubi.Widgets", "Mixer widgets", "Musubi")
    }
    fn launched(&mut self, cx: &mut Cx<Msg>) {
        cx.open_window(WindowSpec::new("Mixer widgets").size(250., 390.));
    }
    fn update(&mut self, msg: Msg, _cx: &mut Cx<Msg>) {
        match msg {
            Msg::Volume(value) => {
                self.value = value;
                self.changes += 1;
            }
            Msg::Peaks(peaks, now) => {
                self.peaks = peaks;
                self.now = now;
            }
        }
    }
    fn view(&self, _window: WindowKey) -> Element<Msg> {
        aizome_kit::column![
            text("OUTPUT · TEST STEREO").role(Role::Accent),
            row![
                Fader::with_reason(self.value, false, self.reason.clone(), Msg::Volume).id("fader"),
                LevelMeter::element(&self.peaks, self.now, false).id("meter"),
            ],
        ]
        .padding(20.)
        .into()
    }
}
fn harness(value: f64, peaks: [f32; 2], disabled: bool, day: bool) -> Harness<Demo> {
    Harness::with(
        Demo::new(value, peaks, disabled),
        Appearance {
            theme: if day { ThemeChoice::Day } else { ThemeChoice::Night },
            reduced_motion: true,
            ..Appearance::default()
        },
        true,
    )
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-3, "{a} != {b}");
}

#[test]
fn cubic_law_pointer_drag_and_keyboard_steps() {
    close(linear_to_db(1.), 0.);
    close(linear_to_db(db_to_linear(-12.)), -12.);
    assert!(linear_to_db(0.).is_infinite());
    let mut h = harness(1., [0., 0.], false, false);
    let w = h.window();
    let r = h.find_id(w, "fader").unwrap();
    h.drag(w, Point::new(r.x0 + 42., r.y0 + 12.), Point::new(r.x0 + 42., r.y1 - 49.), 5);
    assert!(h.program().value < 0.12, "drag to bottom follows dB range");
    h.click(w, Point::new(r.x0 + 42., r.y0 + 100.));
    let before = linear_to_db(h.program().value);
    h.named(w, NamedKey::ArrowUp, Modifiers::NONE);
    close(linear_to_db(h.program().value), before + 1.);
    h.named(w, NamedKey::PageDown, Modifiers::NONE);
    close(linear_to_db(h.program().value), before - 5.);
    h.named(w, NamedKey::Home, Modifiers::NONE);
    close(h.program().value, 1.);
    h.named(w, NamedKey::End, Modifiers::NONE);
    close(h.program().value, 0.);
}

#[test]
fn value_entry_cancel_semantics_and_disabled() {
    let mut h = harness(db_to_linear(-12.), [0., 0.], false, false);
    let w = h.window();
    let r = h.find_id(w, "fader").unwrap();
    let box_at = Point::new(r.center().x, r.y1 - 16.);
    assert!(
        h.semantics(w).iter().any(|(s, _)| s.role == SemanticRole::Slider && s.value.as_deref() == Some("−12.0 dB"))
    );
    h.press(w, box_at, PointerButton::Left, 2);
    h.release(w, box_at, PointerButton::Left);
    for _ in 0..12 {
        h.named(w, NamedKey::Backspace, Modifiers::NONE);
    }
    h.type_text(w, "-6");
    h.named(w, NamedKey::Escape, Modifiers::NONE);
    close(linear_to_db(h.program().value), -12.);
    h.press(w, box_at, PointerButton::Left, 2);
    h.release(w, box_at, PointerButton::Left);
    for _ in 0..12 {
        h.named(w, NamedKey::Backspace, Modifiers::NONE);
    }
    h.type_text(w, "-6 dB\n");
    close(linear_to_db(h.program().value), -6.);
    let mut disabled = harness(1., [0., 0.], true, false);
    let w = disabled.window();
    disabled.named(w, NamedKey::End, Modifiers::NONE);
    assert_eq!(disabled.program().changes, 0);
    assert!(disabled.semantics(w).iter().any(|(s, _)| s.role == SemanticRole::Slider
        && s.disabled
        && s.label.as_deref().unwrap().contains("Ambiguous")));
}

#[test]
fn peak_hold_and_meter_damage() {
    let mut h = harness(1., [0.9, 1.1], false, false);
    let w = h.window();
    let meter = h.find_id(w, "meter").unwrap();
    assert!(h.semantics(w).iter().any(|(s, _)| s.role == SemanticRole::Meter && s.actions.is_empty()));
    h.take_damage(w);
    let at = h.program().now + Duration::from_millis(500);
    h.send(Msg::Peaks([0., 0.], at));
    let damage = h.take_damage(w);
    assert!(!damage.is_empty());
    assert!(
        damage
            .iter()
            .all(|rect| rect.x0 >= meter.x0 && rect.x1 <= meter.x1 && rect.y0 >= meter.y0 && rect.y1 <= meter.y1),
        "{damage:?}, meter {meter:?}"
    );
    let (_, _, held_pixels) = h.render_rgba(w, 1.);
    h.advance(Duration::from_millis(1600));
    let (_, _, expired_pixels) = h.render_rgba(w, 1.);
    assert!(held_pixels != expired_pixels, "peak marker expires after 1.5 s");
    let damage = h.take_damage(w);
    assert!(damage.iter().all(|rect| rect.intersect(meter) == *rect));
}

#[test]
fn mixer_widget_goldens() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        for (state, value, peaks, disabled) in [
            ("rest", 1., [0., 0.], false),
            ("active", db_to_linear(-6.), [0.8, 1.3], false),
            ("disabled", 1., [0., 0.], true),
        ] {
            let mut h = harness(value, peaks, disabled, day);
            let w = h.window();
            assert_golden(&mut h, w, format!("tests/snapshots/mixer-widgets-{theme}-{state}.png"), 1.);
        }
    }
}
