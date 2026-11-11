use aizome_kit::{
    Appearance, Point, Rect, Vec2,
    sidestep_ui::{Key, Modifiers, NamedKey},
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_musubi::{
    app::{Musubi, Page},
    graph::{Graph, dump, group::GroupKey},
    matrix::{
        Layout,
        layout::{Cell, CellState, ViewOptions},
    },
};
use std::{rc::Rc, time::Instant};

fn studio_graph() -> Graph {
    let mut graph = Graph::default();
    graph.apply(dump::parse(include_str!("fixtures/studio.json"), 1).unwrap());
    graph
}

#[test]
fn graph_layout_groups_pairs_channels_collapse_swap_filters_and_monitors() {
    let mut graph = studio_graph();
    let mut options = ViewOptions { pairs: true, ..ViewOptions::default() };
    let pairs = Layout::from_graph(&graph, Page::Audio, &options);
    assert!(!pairs.rows.groups.is_empty() && !pairs.cols.groups.is_empty());
    assert_eq!(pairs.rows.groups[0].kind, aizome_musubi::matrix::layout::Kind::Hardware);
    assert!(pairs.rows.labels.iter().chain(&pairs.cols.labels).any(|s| s == "L·R"));
    let source_count = pairs.row_lines.iter().map(|line| line.ports.len()).sum::<usize>();
    options.pairs = false;
    let channels = Layout::from_graph(&graph, Page::Audio, &options);
    assert_eq!(channels.rows.labels.len(), source_count);
    assert!(
        channels.rows.labels.len() > pairs.rows.labels.len() || channels.cols.labels.len() > pairs.cols.labels.len()
    );
    let key: GroupKey = channels.rows.groups[0].key.clone().unwrap();
    options.folded.insert(key);
    let folded = Layout::from_graph(&graph, Page::Audio, &options);
    assert!(folded.rows.groups[0].collapsed);
    assert!(matches!(folded.state(Cell { row: 0, col: 0 }), CellState::Count(_)));
    options.swap = true;
    let swapped = Layout::from_graph(&graph, Page::Audio, &options);
    assert_eq!(swapped.rows.labels, folded.cols.labels);
    assert_eq!(swapped.cols.labels, folded.rows.labels);
    assert_eq!(swapped.state(Cell { row: 0, col: 0 }), folded.state(Cell { row: 0, col: 0 }));
    options.from = "scarlett".into();
    let filtered = Layout::from_graph(&graph, Page::Audio, &options);
    assert!(filtered.hidden_from > 0);
    assert_eq!(filtered.hidden_to, 0);
    assert!(filtered.cols.groups.iter().all(|g| g.name.to_lowercase().contains("scarlett")));
    options.from = "AUX0".into();
    let port_filtered = Layout::from_graph(&graph, Page::Audio, &options);
    assert!(port_filtered.hidden_from > 0);
    let mut monitor = graph.ports[&100].clone();
    monitor.id = 99999;
    monitor.props.insert("port.monitor".into(), "true".into());
    graph.apply(aizome_musubi::graph::GraphDelta {
        epoch: 1,
        upserts: vec![aizome_musubi::graph::Upsert::Port(monitor)],
        removals: vec![],
    });
    options.monitors = true;
    let with_monitors = Layout::from_graph(&graph, Page::Audio, &options);
    assert_eq!(
        with_monitors.col_lines.iter().map(|l| l.ports.len()).sum::<usize>(),
        port_filtered.col_lines.iter().map(|l| l.ports.len()).sum::<usize>() + 1
    );
}

fn harness(n: usize, day: bool) -> Harness<Musubi> {
    Harness::with(
        Musubi::for_matrix_test(Rc::new(Layout::studio(n, n))),
        Appearance {
            theme: if day { ThemeChoice::Day } else { ThemeChoice::Night },
            reduced_motion: true,
            ..Appearance::default()
        },
        true,
    )
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
        h.move_to(w, p);
        exact(&mut h, scale, "hover first");
        h.move_to(w, Point::new(p.x + 48., p.y + 48.));
        exact(&mut h, scale, "hover second");
        h.click(w, p);
        exact(&mut h, scale, "focus click");
        h.key(w, Key::Named(NamedKey::ArrowRight), Modifiers::default());
        exact(&mut h, scale, "focus right");
        h.key(w, Key::Named(NamedKey::ArrowDown), Modifiers::default());
        exact(&mut h, scale, "focus down");
        h.scroll(w, p, Vec2::new(72., 0.));
        exact(&mut h, scale, "horizontal");
        h.scroll(w, p, Vec2::new(0., 72.));
        exact(&mut h, scale, "vertical");
        h.scroll(w, p, Vec2::new(144., 120.));
        exact(&mut h, scale, "diagonal");
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
        let whole = Rect::from_origin_size(Point::ZERO, h.size(w));
        for kind in ["hover", "focus", "scroll", "full"] {
            let mut samples = Vec::new();
            for n in 0..80 {
                match kind {
                    "hover" => h.move_to(w, Point::new(p.x + (n % 2) as f64 * 24., p.y + (n % 2) as f64 * 24.)),
                    "focus" => {
                        h.key(
                            w,
                            Key::Named(if n % 2 == 0 { NamedKey::ArrowLeft } else { NamedKey::ArrowRight }),
                            Modifiers::default(),
                        );
                    }
                    "scroll" => h.scroll(
                        w,
                        p,
                        Vec2::new(if n % 2 == 0 { 24. } else { -24. }, if n % 2 == 0 { 24. } else { -24. }),
                    ),
                    _ => (),
                }
                let damage = if kind == "full" {
                    h.take_damage(w);
                    vec![whole]
                } else {
                    h.take_damage(w)
                };
                assert!(!damage.is_empty(), "{kind} produced no damage at sample {n}");
                let start = Instant::now();
                h.render_damage(w, &damage, scale).expect("nonempty damage");
                if n >= 10 {
                    samples.push(start.elapsed().as_secs_f64() * 1000.);
                }
            }
            let (p50, p95) = percentile(samples);
            println!("matrix_frames scale={scale} operation={kind} p50={p50:.3}ms p95={p95:.3}ms");
        }
    }
}
