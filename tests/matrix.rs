use std::{rc::Rc, time::Instant};
use aizome_kit::{Appearance, Point, Vec2, sidestep_ui::{Key, NamedKey, Modifiers}, testing::{Harness, assert_golden}, theme::ThemeChoice};
use aizome_musubi::{app::Musubi, matrix::Layout};

fn harness(n: usize, day: bool) -> Harness<Musubi> {
    Harness::with(Musubi::for_matrix_test(Rc::new(Layout::studio(n, n))), Appearance {
        theme: if day { ThemeChoice::Day } else { ThemeChoice::Night },
        reduced_motion: true, ..Appearance::default()
    }, true)
}

fn exact(h: &mut Harness<Musubi>, scale: f64, step: &str) {
    let result = h.check_frame_exact(h.window(), scale);
    assert_eq!(result.differing_pixels, 0, "{step} at {scale}x: {result:?}");
}

#[test]
fn matrix_goldens() {
    for (day, name) in [(false, "night"), (true, "day")] {
        let mut h = harness(30, day);
        assert!(h.theme().has_ui_font());
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/matrix-spike-{name}.png"), 1.);
    }
    let mut h = harness(200, false);
    let w = h.window();
    let r = h.find_id(w, "matrix").unwrap();
    h.scroll(w, Point::new(r.x0 + 450., r.y0 + 400.), Vec2::new(2900., 2900.));
    assert_golden(&mut h, w, "tests/snapshots/matrix-spike-200-night.png", 1.);
}

#[test]
fn matrix_damage_is_exact_at_both_scales() {
    for scale in [1., 2.] {
        let mut h = harness(200, false);
        let w = h.window();
        let r = h.find_id(w, "matrix").unwrap();
        let p = Point::new(r.x0 + 225., r.y0 + 210.);
        exact(&mut h, scale, "initial");
        h.move_to(w, p); exact(&mut h, scale, "hover first");
        h.move_to(w, Point::new(p.x + 48., p.y + 48.)); exact(&mut h, scale, "hover second");
        h.click(w, p); exact(&mut h, scale, "focus click");
        h.key(w, Key::Named(NamedKey::ArrowRight), Modifiers::default()); exact(&mut h, scale, "focus right");
        h.key(w, Key::Named(NamedKey::ArrowDown), Modifiers::default()); exact(&mut h, scale, "focus down");
        h.scroll(w, p, Vec2::new(72., 0.)); exact(&mut h, scale, "horizontal");
        h.scroll(w, p, Vec2::new(0., 72.)); exact(&mut h, scale, "vertical");
        h.scroll(w, p, Vec2::new(144., 120.)); exact(&mut h, scale, "diagonal");
    }
}

fn percentile(mut samples: Vec<f64>) -> (f64, f64) {
    samples.sort_by(f64::total_cmp);
    let p50 = samples[samples.len() / 2];
    let p95 = samples[(samples.len() * 95 / 100).min(samples.len() - 1)];
    (p50, p95)
}

#[test]
#[ignore = "release-only pixel timing, run with --ignored matrix_frames --nocapture"]
fn matrix_frames() {
    for scale in [1., 2.] {
        let mut h = harness(200, false);
        let w = h.window();
        let r = h.find_id(w, "matrix").unwrap();
        let p = Point::new(r.x0 + 270., r.y0 + 230.);
        h.scroll(w, p, Vec2::new(2250., 2200.));
        h.render_frame(w, scale);
        for kind in ["hover", "focus", "scroll", "full"] {
            let mut samples = Vec::new();
            for n in 0..80 {
                match kind {
                    "hover" => h.move_to(w, Point::new(p.x + (n % 2) as f64 * 24., p.y + (n % 2) as f64 * 24.)),
                    "focus" => { h.key(w, Key::Named(if n % 2 == 0 { NamedKey::ArrowLeft } else { NamedKey::ArrowRight }), Modifiers::default()); },
                    "scroll" => h.scroll(w, p, Vec2::new(if n % 2 == 0 { 24. } else { -24. }, if n % 2 == 0 { 24. } else { -24. })),
                    _ => (),
                }
                let start = Instant::now();
                if kind == "full" { h.render_rgba(w, scale); } else { h.render_frame(w, scale); }
                if n >= 10 { samples.push(start.elapsed().as_secs_f64() * 1000.); }
            }
            let (p50, p95) = percentile(samples);
            println!("matrix_frames scale={scale} operation={kind} p50={p50:.3}ms p95={p95:.3}ms");
        }
    }
}
