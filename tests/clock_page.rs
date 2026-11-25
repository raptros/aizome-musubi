use aizome_kit::{
    Appearance, Point,
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_musubi::{
    app::{Msg, Musubi, Page},
    backend::{Backend, Command, Event, fake::FakeBackend},
    clock::DriverRow,
    effects::{
        paths::Paths,
        service::{UnitState, Units},
    },
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

struct NoUnits;
impl Units for NoUnits {
    fn enable(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn start(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn stop(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn disable(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn listed(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
    fn state(&self, _: &str) -> Result<UnitState, String> {
        Ok(UnitState::default())
    }
    fn owned(&self, _: &str) -> Result<bool, String> {
        Ok(true)
    }
}

fn setup(label: &str, day: bool) -> (Harness<Musubi>, Rc<RefCell<FakeBackend>>, Paths) {
    let root = PathBuf::from(format!("/tmp/musubi-clock-page-{label}"));
    let _ = std::fs::remove_dir_all(&root);
    let paths = Paths::under(root);
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
    let appearance = Appearance {
        theme: if day { ThemeChoice::Day } else { ThemeChoice::Night },
        reduced_motion: true,
        ..Appearance::default()
    };
    let mut h =
        Harness::with(Musubi::with_fake_effects(fake.clone(), paths.clone(), Box::new(NoUnits)), appearance, true);
    h.send(Msg::Page(Page::Clock));
    flush(&mut h, &fake);
    (h, fake, paths)
}

fn flush(h: &mut Harness<Musubi>, fake: &Rc<RefCell<FakeBackend>>) {
    while let Some(event) = { fake.borrow_mut().next_event() } {
        h.send(Msg::Backend(event));
    }
}
fn click_id(h: &mut Harness<Musubi>, id: &str) {
    let window = h.window();
    let rect = h.find_id(window, id).unwrap();
    h.click(window, Point::new((rect.x0 + rect.x1) / 2., (rect.y0 + rect.y1) / 2.));
}
fn rows() -> Vec<DriverRow> {
    vec![
        DriverRow {
            id: 42,
            node: "Test audio driver".into(),
            driver: true,
            rate: 48000,
            quantum: 256,
            busy: 0.60,
            wait: 4.73,
            errors: 4,
            xruns: 2,
        },
        DriverRow {
            id: 43,
            node: "Test playback".into(),
            driver: false,
            rate: 48000,
            quantum: 256,
            busy: 0.18,
            wait: 0.08,
            errors: 1,
            xruns: 0,
        },
    ]
}

#[test]
fn clock_persistence_and_subscription() {
    let (mut h, fake, paths) = setup("behavior", false);
    assert!(fake.borrow().commands.iter().any(|c| matches!(c, Command::StartProfiler)));
    fake.borrow_mut().publish_profiler(rows());
    flush(&mut h, &fake);
    h.send(Msg::ClockQuantum(256));
    flush(&mut h, &fake);
    assert_eq!(h.program().status_sentence().as_deref(), Some("Forced clock: rate automatic · quantum 256 frames"));
    assert_eq!(fake.borrow().clock.force_quantum, Some(256));
    click_id(&mut h, "clock-persist");
    assert_eq!(
        std::fs::read_to_string(&paths.clock).unwrap(),
        "# Managed by Musubi. Applies at the next PipeWire start.\ncontext.properties = {\n    default.clock.quantum = 256\n}\n"
    );
    click_id(&mut h, "clock-automatic");
    flush(&mut h, &fake);
    assert_eq!(fake.borrow().clock.force_quantum, None);
    assert!(!paths.clock.exists());
    h.send(Msg::Page(Page::Audio));
    assert!(fake.borrow().commands.iter().any(|c| matches!(c, Command::StopProfiler)));
    fake.borrow_mut().publish_profiler(rows());
    assert!(fake.borrow_mut().next_event().is_none());
}

#[test]
fn golden_clock_states() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        let (mut h, fake, _) = setup(theme, day);
        let window = h.window();
        fake.borrow_mut().publish_profiler(rows());
        flush(&mut h, &fake);
        assert_golden(&mut h, window, format!("tests/snapshots/clock-automatic-{theme}.png"), 1.);
        h.send(Msg::ClockQuantum(256));
        flush(&mut h, &fake);
        fake.borrow_mut().publish_profiler(
            rows()
                .into_iter()
                .map(|mut row| {
                    row.errors += 3;
                    row.xruns += 1;
                    row
                })
                .collect(),
        );
        flush(&mut h, &fake);
        assert_golden(&mut h, window, format!("tests/snapshots/clock-forced-{theme}.png"), 1.);
        let mut readonly = fake.borrow().clock.clone();
        readonly.read_only = Some("PipeWire denies metadata writes".into());
        h.send(Msg::Backend(Event::ClockSettings(readonly)));
        assert_golden(&mut h, window, format!("tests/snapshots/clock-read-only-{theme}.png"), 1.);
    }
}
