//! PipeWire's non-Send objects stay on one thread; only graph values leave it.
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::mpsc, thread, time::{Duration, Instant}};
use pipewire as pw;
use pw::spa::{self, param::{ParamType, format::{MediaType, MediaSubtype}, format_utils}, utils::dict::DictRef};

use super::{Backend, Command, Connection, Event};
use crate::graph::{ClientRecord, DeviceRecord, Direction, GraphDelta, LinkRecord, NodeRecord, PortRecord, Properties, Upsert, classify::PortKind};

type Sink = Box<dyn Fn(Event) + Send>;
type Objects = Rc<RefCell<HashMap<u32, Bound>>>;
type Pending = Rc<RefCell<Batch>>;

#[derive(Default)]
struct Batch { epoch: u64, changed: HashMap<u32, Upsert>, removed: Vec<u32> }
impl Batch {
    fn upsert(&mut self, value: Upsert) { self.changed.insert(value.id(), value); }
    fn remove(&mut self, id: u32) { self.changed.remove(&id); self.removed.push(id); }
    fn drain(&mut self) -> Option<GraphDelta> {
        if self.changed.is_empty() && self.removed.is_empty() { return None; }
        Some(GraphDelta { epoch: self.epoch, upserts: self.changed.drain().map(|(_, v)| v).collect(), removals: std::mem::take(&mut self.removed) })
    }
}

// Listeners precede their proxies in drop order. The registry callback never moves
// a proxy (or its listener) to the application thread.
#[allow(dead_code)]
enum Bound {
    Node(pw::node::NodeListener, pw::node::Node),
    Port(pw::port::PortListener, pw::port::Port),
    Link(pw::link::LinkListener, pw::link::Link),
    Device(pw::device::DeviceListener, pw::device::Device),
    Client(pw::client::ClientListener, pw::client::Client),
}

fn props(dict: Option<&DictRef>) -> Properties {
    dict.into_iter().flat_map(DictRef::iter).map(|(k, v)| (k.to_owned(), v.to_owned())).collect()
}
fn merge(to: &mut Properties, from: Option<&DictRef>) {
    if let Some(dict) = from { to.extend(dict.iter().map(|(k, v)| (k.to_owned(), v.to_owned()))); }
}
fn prop_id(p: &Properties, key: &str) -> Option<u32> { p.get(key).and_then(|s| s.parse().ok()) }
fn format_kind(pod: &spa::pod::Pod) -> PortKind {
    match format_utils::parse_format(pod) {
        Ok((MediaType::Audio, MediaSubtype::Raw)) => PortKind::Audio,
        Ok((MediaType::Application, MediaSubtype::Control)) => PortKind::Midi,
        Ok((MediaType::Video, _)) => PortKind::Video,
        _ => PortKind::Other,
    }
}

fn bind(global: &pw::registry::GlobalObject<&DictRef>, registry: &pw::registry::RegistryRc, objects: &Objects, pending: &Pending) {
    let id = global.id;
    let p = props(global.props);
    let result: Result<Bound, pw::Error> = match global.type_ {
        pw::types::ObjectType::Node => registry.bind::<pw::node::Node, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(NodeRecord { id, device_id: prop_id(&p, "device.id"), client_id: prop_id(&p, "client.id"), props: p.clone() }));
            pending.borrow_mut().upsert(Upsert::Node(state.borrow().clone()));
            let batch = pending.clone();
            let listener = proxy.add_listener_local().info(move |info| {
                let mut record = state.borrow_mut();
                merge(&mut record.props, info.props());
                record.device_id = prop_id(&record.props, "device.id");
                record.client_id = prop_id(&record.props, "client.id");
                batch.borrow_mut().upsert(Upsert::Node(record.clone()));
            }).register();
            Bound::Node(listener, proxy)
        }),
        pw::types::ObjectType::Port => registry.bind::<pw::port::Port, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(PortRecord { id, node_id: prop_id(&p, "node.id"), direction: Direction::Other, enum_formats: Vec::new(), props: p.clone() }));
            pending.borrow_mut().upsert(Upsert::Port(state.borrow().clone()));
            let info_state = state.clone(); let param_state = state;
            let info_batch = pending.clone(); let param_batch = pending.clone();
            let listener = proxy.add_listener_local().info(move |info| {
                let mut record = info_state.borrow_mut();
                merge(&mut record.props, info.props());
                record.node_id = prop_id(&record.props, "node.id");
                record.direction = match info.direction() {
                    spa::utils::Direction::Input => Direction::Input,
                    spa::utils::Direction::Output => Direction::Output,
                    _ => Direction::Other,
                };
                info_batch.borrow_mut().upsert(Upsert::Port(record.clone()));
            }).param(move |_, kind, _, _, pod| {
                if kind != ParamType::EnumFormat { return; }
                if let Some(pod) = pod {
                    let category = format_kind(pod);
                    let mut record = param_state.borrow_mut();
                    if !record.enum_formats.contains(&category) { record.enum_formats.push(category); }
                    param_batch.borrow_mut().upsert(Upsert::Port(record.clone()));
                }
            }).register();
            proxy.enum_params(0, Some(ParamType::EnumFormat), 0, u32::MAX);
            Bound::Port(listener, proxy)
        }),
        pw::types::ObjectType::Link => registry.bind::<pw::link::Link, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(LinkRecord { id, output: prop_id(&p, "link.output.port").unwrap_or(0), input: prop_id(&p, "link.input.port").unwrap_or(0), props: p.clone() }));
            let batch = pending.clone();
            let listener = proxy.add_listener_local().info(move |info| {
                let mut record = state.borrow_mut();
                merge(&mut record.props, info.props());
                record.output = info.output_port_id(); record.input = info.input_port_id();
                record.props.insert("link.state".into(), format!("{:?}", info.state()));
                batch.borrow_mut().upsert(Upsert::Link(record.clone()));
            }).register();
            Bound::Link(listener, proxy)
        }),
        pw::types::ObjectType::Device => registry.bind::<pw::device::Device, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(DeviceRecord { id, props: p.clone() }));
            pending.borrow_mut().upsert(Upsert::Device(state.borrow().clone()));
            let batch = pending.clone();
            let listener = proxy.add_listener_local().info(move |info| {
                let mut record = state.borrow_mut(); merge(&mut record.props, info.props());
                batch.borrow_mut().upsert(Upsert::Device(record.clone()));
            }).register();
            Bound::Device(listener, proxy)
        }),
        pw::types::ObjectType::Client => registry.bind::<pw::client::Client, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(ClientRecord { id, props: p.clone() }));
            pending.borrow_mut().upsert(Upsert::Client(state.borrow().clone()));
            let batch = pending.clone();
            let listener = proxy.add_listener_local().info(move |info| {
                let mut record = state.borrow_mut(); merge(&mut record.props, info.props());
                batch.borrow_mut().upsert(Upsert::Client(record.clone()));
            }).register();
            Bound::Client(listener, proxy)
        }),
        _ => return,
    };
    if let Ok(bound) = result { objects.borrow_mut().insert(id, bound); }
}

struct Session {
    // Order matters: listener hooks and bound proxies are destroyed before registry/core.
    objects: Objects,
    _registry_listener: pw::registry::Listener,
    _core_listener: pw::core::Listener,
    _registry: pw::registry::RegistryRc,
    _core: pw::core::CoreRc,
    _context: pw::context::ContextRc,
}
fn connect(loop_: &pw::main_loop::MainLoopRc, pending: &Pending, ready: &Rc<RefCell<Option<String>>>, failure: &Rc<RefCell<Option<String>>>) -> Result<Session, String> {
    let context = pw::context::ContextRc::new(loop_, None).map_err(|e| e.to_string())?;
    let core = context.connect_rc(Some(pw::properties::properties! {
        "application.name" => "Musubi", "application.id" => "org.aizome.Musubi", "media.category" => "Manager"
    })).map_err(|e| e.to_string())?;
    let registry = core.get_registry_rc().map_err(|e| e.to_string())?;
    let objects: Objects = Default::default();
    let items = objects.clone(); let batch = pending.clone(); let reg = registry.clone();
    let removed = objects.clone(); let removed_batch = pending.clone();
    let registry_listener = registry.add_listener_local().global(move |global| bind(global, &reg, &items, &batch))
        .global_remove(move |id| { removed.borrow_mut().remove(&id); removed_batch.borrow_mut().remove(id); }).register();
    let version = Rc::new(RefCell::new(String::from("PipeWire")));
    let info_version = version.clone(); let done_version = version;
    let up = ready.clone(); let down = failure.clone();
    let core_listener = core.add_listener_local().info(move |info| *info_version.borrow_mut() = info.version().to_owned())
        .done(move |id, _| { if id == pw::core::PW_ID_CORE { *up.borrow_mut() = Some(done_version.borrow().clone()); } })
        .error(move |_, _, code, message| { *down.borrow_mut() = Some(format!("PipeWire error {code}: {message}")); }).register();
    core.sync(0).map_err(|e| e.to_string())?;
    Ok(Session { objects, _registry_listener: registry_listener, _core_listener: core_listener, _registry: registry, _core: core, _context: context })
}

pub struct PipeWireBackend { tx: pw::channel::Sender<Command>, thread: Option<thread::JoinHandle<()>>, events: Option<mpsc::Receiver<Event>> }
impl PipeWireBackend {
    pub fn start(sink: impl Fn(Event) + Send + 'static) -> Result<Self, String> {
        let (tx, rx) = pw::channel::channel();
        let thread = thread::Builder::new().name("musubi-pipewire".into()).spawn(move || run(rx, Box::new(sink))).map_err(|e| e.to_string())?;
        Ok(Self { tx, thread: Some(thread), events: None })
    }
    pub fn channel() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let mut backend = Self::start(move |event| { let _ = tx.send(event); })?;
        backend.events = Some(rx);
        Ok(backend)
    }
    pub fn recv_timeout(&self, duration: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
        self.events.as_ref().expect("channel backend required").recv_timeout(duration)
    }
    pub fn shutdown(&mut self) {
        if let Some(thread) = self.thread.take() { let _ = self.tx.send(Command::Shutdown); let _ = thread.join(); }
    }
}
impl Backend for PipeWireBackend {
    fn command(&mut self, command: Command) { let _ = self.tx.send(command); }
    fn next_event(&mut self) -> Option<Event> { self.events.as_ref().and_then(|events| events.try_recv().ok()) }
}
impl Drop for PipeWireBackend { fn drop(&mut self) { self.shutdown(); } }

fn run(rx: pw::channel::Receiver<Command>, sink: Sink) {
    pw::init();
    let Ok(mainloop) = pw::main_loop::MainLoopRc::new(None) else { sink(Event::Connection(Connection::Down("Cannot create PipeWire loop".into()))); return; };
    let pending: Pending = Default::default();
    let ready = Rc::new(RefCell::new(None)); let failure = Rc::new(RefCell::new(None));
    let session: Rc<RefCell<Option<Session>>> = Default::default();
    let next_try = Rc::new(RefCell::new(Instant::now()));
    let backoff = Rc::new(RefCell::new(Duration::from_millis(500)));
    let pending_tick = pending.clone(); let ready_tick = ready.clone(); let failure_tick = failure.clone();
    let session_tick = session.clone(); let next_tick = next_try.clone(); let backoff_tick = backoff.clone();
    let sink = Rc::new(sink);
    let tick_sink = sink.clone();
    let tick_loop = mainloop.clone();
    let timer = mainloop.loop_().add_timer(move |_| {
        if let Some(reason) = failure_tick.borrow_mut().take() {
            session_tick.borrow_mut().take();
            let mut batch = pending_tick.borrow_mut();
            batch.changed.clear(); batch.removed.clear(); batch.epoch += 1;
            tick_sink(Event::Connection(Connection::Down(reason)));
            *ready_tick.borrow_mut() = None;
            *next_tick.borrow_mut() = Instant::now() + *backoff_tick.borrow();
            let doubled = (*backoff_tick.borrow() * 2).min(Duration::from_secs(5));
            *backoff_tick.borrow_mut() = doubled;
        }
        if session_tick.borrow().is_none() && Instant::now() >= *next_tick.borrow() {
            match connect(&tick_loop, &pending_tick, &ready_tick, &failure_tick) {
                Ok(connected) => { *session_tick.borrow_mut() = Some(connected); }
                Err(reason) => { *failure_tick.borrow_mut() = Some(reason); }
            }
        }
        if let Some(version) = ready_tick.borrow_mut().take() {
            *backoff_tick.borrow_mut() = Duration::from_millis(500);
            tick_sink(Event::Connection(Connection::Up(version)));
        }
        if let Some(delta) = pending_tick.borrow_mut().drain() { tick_sink(Event::Delta(delta)); }
    });
    let _ = timer.update_timer(Some(Duration::from_millis(1)), Some(Duration::from_millis(16)));
    let stop = mainloop.clone();
    let _attached = rx.attach(mainloop.loop_(), move |cmd| match cmd {
        Command::Shutdown => stop.quit(),
        Command::Connect { request, .. } | Command::Disconnect { request, .. } =>
            sink(Event::CommandResult { request, result: Err("NotYet: link mutations are not implemented".into()) }),
    });
    mainloop.run();
    drop(timer);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coalesces_repeated_changes_and_removal() {
        let mut batch = Batch::default(); batch.epoch = 8;
        batch.upsert(Upsert::Client(ClientRecord { id: 4, props: Properties::new() }));
        batch.upsert(Upsert::Client(ClientRecord { id: 4, props: Properties::from([("name".into(), "updated".into())]) }));
        let delta = batch.drain().unwrap();
        assert_eq!(delta.epoch, 8); assert_eq!(delta.upserts.len(), 1);
        assert!(batch.drain().is_none());
        batch.remove(4); let delta = batch.drain().unwrap();
        assert_eq!(delta.removals, vec![4]);
    }
}
