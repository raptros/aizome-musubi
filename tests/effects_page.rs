use aizome_kit::{
    Appearance, Point,
    sidestep_ui::{Key, Modifiers, NamedKey},
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_musubi::{
    app::{Msg, Musubi, Page},
    backend::{Backend, Event, fake::FakeBackend},
    effects::{
        Band, BandKind, Chain, Target,
        paths::Paths,
        service::{EffectsService, UnitState, Units},
    },
    graph::Upsert,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

#[derive(Default)]
struct FakeUnits {
    state: RefCell<UnitState>,
    listed: RefCell<Vec<String>>,
    calls: RefCell<Vec<String>>,
}
#[derive(Clone)]
struct TestUnits(Rc<FakeUnits>);
impl Units for TestUnits {
    fn enable(&self, name: &str) -> Result<(), String> {
        self.0.listed.borrow_mut().push(name.into());
        self.0.calls.borrow_mut().push(format!("enable {name}"));
        Ok(())
    }
    fn start(&self, name: &str) -> Result<(), String> {
        self.0.state.borrow_mut().active = "active".into();
        self.0.calls.borrow_mut().push(format!("start {name}"));
        Ok(())
    }
    fn stop(&self, name: &str) -> Result<(), String> {
        self.0.state.borrow_mut().active = "inactive".into();
        self.0.calls.borrow_mut().push(format!("stop {name}"));
        Ok(())
    }
    fn disable(&self, name: &str) -> Result<(), String> {
        self.0.listed.borrow_mut().retain(|n| n != name);
        self.0.calls.borrow_mut().push(format!("disable {name}"));
        Ok(())
    }
    fn listed(&self) -> Result<Vec<String>, String> {
        Ok(self.0.listed.borrow().clone())
    }
    fn state(&self, _: &str) -> Result<UnitState, String> {
        Ok(self.0.state.borrow().clone())
    }
}
fn root(label: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/musubi-effects-page-{label}"))
}
fn chain() -> Chain {
    Chain {
        id: "eq-1".into(),
        name: "Studio EQ".into(),
        target: Target::FollowDefault,
        bands: vec![
            Band { kind: BandKind::LowShelf, freq: 105., gain: 4., q: 0.7 },
            Band { kind: BandKind::Peaking, freq: 220., gain: -2.5, q: 1.2 },
            Band { kind: BandKind::Peaking, freq: 1000., gain: 0., q: 2. },
            Band { kind: BandKind::Peaking, freq: 5600., gain: 2., q: 1.4 },
            Band { kind: BandKind::HighShelf, freq: 10000., gain: -1.5, q: 0.7 },
        ],
        bypass: false,
        enabled: false,
    }
}
fn setup(label: &str, day: bool, populated: bool) -> (Harness<Musubi>, Rc<RefCell<FakeBackend>>, Rc<FakeUnits>, Paths) {
    let paths = Paths::under(root(label));
    let _ = std::fs::remove_dir_all(root(label));
    if populated {
        EffectsService { paths: paths.clone(), units: TestUnits(Rc::new(FakeUnits::default())) }
            .save(&chain())
            .unwrap();
    }
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
    let units = Rc::new(FakeUnits::default());
    let appearance = Appearance {
        theme: if day { ThemeChoice::Day } else { ThemeChoice::Night },
        reduced_motion: true,
        ..Appearance::default()
    };
    let mut h = Harness::with(
        Musubi::with_fake_effects(fake.clone(), paths.clone(), Box::new(TestUnits(units.clone()))),
        appearance,
        true,
    );
    h.send(Msg::Page(Page::Effects));
    (h, fake, units, paths)
}
fn flush(h: &mut Harness<Musubi>, fake: &Rc<RefCell<FakeBackend>>) {
    loop {
        let event = { fake.borrow_mut().next_event() };
        if let Some(event) = event {
            h.send(Msg::Backend(event));
        } else {
            break;
        }
    }
}
fn nodes(h: &mut Harness<Musubi>, fake: &Rc<RefCell<FakeBackend>>) {
    let mut node = fake.borrow().graph.nodes.values().next().unwrap().clone();
    node.id = 450;
    node.props.insert("node.name".into(), "musubi_eq.eq-1".into());
    fake.borrow_mut().upsert(Upsert::Node(node.clone()));
    node.id = 451;
    node.props.insert("node.name".into(), "musubi_eq.eq-1.playback".into());
    fake.borrow_mut().upsert(Upsert::Node(node));
    flush(h, fake);
}
fn click_id(h: &mut Harness<Musubi>, id: &str) {
    let w = h.window();
    let r = h.find_id(w, id).unwrap();
    h.click(w, Point::new((r.x0 + r.x1) / 2., (r.y0 + r.y1) / 2.));
}
#[test]
fn create_edit_keys_pointer_undo_redo_and_failed_status() {
    let (mut h, fake, units, paths) = setup("behavior", false, false);
    click_id(&mut h, "effect-new");
    assert!(paths.json("eq-1").unwrap().exists());
    nodes(&mut h, &fake);
    click_id(&mut h, "effect-enable");
    assert_eq!(units.state.borrow().active, "active");
    let w = h.window();
    let r = h.find_id(w, "effect-curve").unwrap();
    let x = r.x0 + 43. + (1000_f64 / 20.).log10() / 3. * (r.width() - 55.);
    let y = r.y0 + 12. + (r.height() - 34.) / 2.;
    h.click(w, Point::new(x, y));
    for _ in 0..12 {
        h.key(w, Key::Named(NamedKey::ArrowUp), Modifiers::NONE);
    }
    h.advance(Duration::from_millis(550));
    assert_eq!(
        EffectsService { paths: paths.clone(), units: TestUnits(units.clone()) }.load("eq-1").unwrap().bands[0].gain,
        12.
    );
    h.send(Msg::Undo);
    assert_eq!(
        EffectsService { paths: paths.clone(), units: TestUnits(units.clone()) }.load("eq-1").unwrap().bands[0].gain,
        0.
    );
    h.send(Msg::Redo);
    assert_eq!(
        EffectsService { paths: paths.clone(), units: TestUnits(units.clone()) }.load("eq-1").unwrap().bands[0].gain,
        12.
    );
    let handle_y = y - 12. / 30. * (r.height() - 34.);
    h.drag(w, Point::new(x, handle_y), Point::new(x + 32., handle_y + 35.), 4);
    let dragged = EffectsService { paths: paths.clone(), units: TestUnits(units.clone()) }.load("eq-1").unwrap();
    assert!(dragged.bands[0].gain < 12. && dragged.bands[0].freq > 1000.);
    h.send(Msg::Undo);
    assert_eq!(
        EffectsService { paths: paths.clone(), units: TestUnits(units.clone()) }.load("eq-1").unwrap().bands[0].gain,
        12.
    );
    units.state.borrow_mut().active = "failed".into();
    units.state.borrow_mut().result = "exit-code".into();
    h.send(Msg::Backend(Event::Delta(Default::default())));
    assert!(h.program().status_sentence().unwrap().contains("PipeWire filter-chain exited with an error"));
    assert!(EffectsService { paths: paths.clone(), units: TestUnits(units.clone()) }.load("eq-1").unwrap().enabled);
    let calls = units.calls.borrow().len();
    click_id(&mut h, "effect-retry");
    assert_eq!(units.calls.borrow().len(), calls + 1);
    assert!(units.calls.borrow().last().unwrap().starts_with("start "));
}
#[test]
fn goldens_and_confirmation() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        let (mut h, _, units, paths) = setup(theme, day, true);
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/effects-five-{theme}.png"), 1.);
        click_id(&mut h, "effect-enable");
        assert!(EffectsService { paths: paths.clone(), units: TestUnits(units.clone()) }.load("eq-1").unwrap().enabled);
        units.state.borrow_mut().active = "failed".into();
        units.state.borrow_mut().result = "exit-code".into();
        h.send(Msg::Backend(Event::Delta(Default::default())));
        assert_golden(&mut h, w, format!("tests/snapshots/effects-failed-{theme}.png"), 1.);
        click_id(&mut h, "effect-delete");
        assert_golden(&mut h, w, format!("tests/snapshots/effects-delete-{theme}.png"), 1.);
        h.key(w, Key::Named(NamedKey::Escape), Modifiers::NONE);
        assert!(paths.json("eq-1").unwrap().exists());
        h.send(Msg::EffectDeleteConfirmed("eq-1".into()));
        assert!(!paths.json("eq-1").unwrap().exists());
        assert_golden(&mut h, w, format!("tests/snapshots/effects-empty-{theme}.png"), 1.);
    }
}
