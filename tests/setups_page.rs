use aizome_kit::{
    Appearance, Point,
    sidestep_ui::{Key, Modifiers, NamedKey},
    testing::{Harness, assert_golden},
    theme::ThemeChoice,
};
use aizome_musubi::{
    app::{Msg, Musubi, Page},
    backend::fake::FakeBackend,
    effects::{
        paths::Paths as EffectPaths,
        service::{UnitState, Units},
    },
    graph::{Upsert, identity::StableId},
    setups::{Route, Setup, VERSION, paths::Paths, store::Store},
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
struct UnitsFake(Rc<RefCell<Vec<String>>>);
impl Units for UnitsFake {
    fn enable(&self, name: &str) -> Result<(), String> {
        self.0.borrow_mut().push(format!("enable {name}"));
        Ok(())
    }
    fn start(&self, name: &str) -> Result<(), String> {
        self.0.borrow_mut().push(format!("start {name}"));
        Ok(())
    }
    fn stop(&self, name: &str) -> Result<(), String> {
        self.0.borrow_mut().push(format!("stop {name}"));
        Ok(())
    }
    fn disable(&self, name: &str) -> Result<(), String> {
        self.0.borrow_mut().push(format!("disable {name}"));
        Ok(())
    }
    fn listed(&self) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
    fn state(&self, _: &str) -> Result<UnitState, String> {
        Ok(UnitState::default())
    }
    fn owned(&self, _: &str) -> Result<bool, String> {
        Ok(true)
    }
}
fn fixture(label: &str, day: bool, populated: bool) -> (Harness<Musubi>, Store, UnitsFake) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = if label == "night" || label == "day" {
        std::env::temp_dir().join(format!("musubi-setups-golden-{label}"))
    } else {
        std::env::temp_dir().join(format!(
            "musubi-setups-page-{}-{label}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    };
    if label == "night" || label == "day" {
        let _ = std::fs::remove_dir_all(&root);
    }
    let store = Store { paths: Paths::under(root.clone()) };
    let fake = Rc::new(RefCell::new(FakeBackend::from_dump(include_str!("fixtures/studio.json"), 1).unwrap()));
    if populated {
        let (output, input, waiting, ambiguous, ambiguous_port, second) = {
            let borrowed = fake.borrow();
            let graph = &borrowed.graph;
            let link = graph.links.values().min_by_key(|link| link.id).expect("studio fixture has links");
            let output = graph.stable_port_id(&graph.ports[&link.output]).unwrap();
            let input = graph.stable_port_id(&graph.ports[&link.input]).unwrap();
            let mut waiting = output.clone();
            if let StableId::Port { node_name, .. } = &mut waiting {
                *node_name = Some("Unplugged player".into());
            }
            let mut ambiguous_port = graph.ports[&link.output].clone();
            ambiguous_port.id = 9000;
            ambiguous_port.props.insert("port.name".into(), "ambiguous-port".into());
            let mut second = ambiguous_port.clone();
            second.id = 9001;
            let ambiguous = graph.stable_port_id(&ambiguous_port).unwrap();
            (output, input, waiting, ambiguous, ambiguous_port, second)
        };
        fake.borrow_mut().upsert(Upsert::Port(ambiguous_port));
        fake.borrow_mut().upsert(Upsert::Port(second));
        store
            .save(&Setup {
                version: VERSION,
                id: "studio".into(),
                name: "Studio".into(),
                routes: vec![
                    Route { output: output.clone(), input: input.clone(), confirmed: false },
                    Route { output: waiting, input: input.clone(), confirmed: false },
                    Route { output: ambiguous, input, confirmed: false },
                ],
                keep_connected: false,
                exclusive: false,
            })
            .unwrap();
    }
    let units = UnitsFake::default();
    let appearance = Appearance {
        theme: if day { ThemeChoice::Day } else { ThemeChoice::Night },
        reduced_motion: true,
        ..Appearance::default()
    };
    let mut h = Harness::with(
        Musubi::with_fake_setups_units(fake, store.paths.clone(), EffectPaths::under(root), Box::new(units.clone())),
        appearance,
        true,
    );
    h.send(Msg::Page(Page::Setups));
    (h, store, units)
}
fn click(h: &mut Harness<Musubi>, id: &str) {
    let w = h.window();
    let r = h.find_id(w, id).unwrap_or_else(|| panic!("missing {id}"));
    h.click(w, Point::new((r.x0 + r.x1) / 2., (r.y0 + r.y1) / 2.));
}
#[test]
fn page_actions_persist_and_confirm() {
    let (mut h, store, units) = fixture("actions", false, true);
    click(&mut h, "setup-activate");
    assert_eq!(store.active().unwrap().unwrap().id, "studio");
    click(&mut h, "setup-keep");
    assert!(store.load("studio").unwrap().keep_connected);
    assert!(units.0.borrow().iter().any(|call| call.starts_with("enable ")));
    click(&mut h, "setup-exclusive");
    assert!(!store.load("studio").unwrap().exclusive);
    h.send(Msg::SetupExclusiveConfirmed);
    assert!(store.load("studio").unwrap().exclusive);
    h.send(Msg::SetupNameInput("Renamed".into()));
    h.send(Msg::SetupNameCommit);
    assert_eq!(store.load("studio").unwrap().name, "Renamed");
    click(&mut h, "setup-delete");
    h.key(h.window(), Key::Named(NamedKey::Escape), Modifiers::NONE);
    assert!(store.load("studio").is_ok());
    h.send(Msg::SetupDeleteConfirmed("studio".into()));
    assert!(store.list().unwrap().is_empty());
    assert!(store.active().unwrap().is_none());
    assert!(units.0.borrow().iter().any(|call| call.starts_with("disable ")));
}
#[test]
fn goldens_night_and_day() {
    for (day, theme) in [(false, "night"), (true, "day")] {
        let (mut h, store, _) = fixture(theme, day, true);
        let w = h.window();
        assert_golden(&mut h, w, format!("tests/snapshots/setups-page-{theme}.png"), 1.);
        click(&mut h, "setup-exclusive");
        assert_golden(&mut h, w, format!("tests/snapshots/setups-exclusive-{theme}.png"), 1.);
        h.key(w, Key::Named(NamedKey::Escape), Modifiers::NONE);
        click(&mut h, "setup-delete");
        assert_golden(&mut h, w, format!("tests/snapshots/setups-delete-{theme}.png"), 1.);
        h.key(w, Key::Named(NamedKey::Escape), Modifiers::NONE);
        h.send(Msg::SetupDeleteConfirmed("studio".into()));
        assert!(store.list().unwrap().is_empty());
        assert_golden(&mut h, w, format!("tests/snapshots/setups-empty-{theme}.png"), 1.);
    }
}
