//! PipeWire's non-Send objects stay on one thread; only graph values leave it.
use pipewire as pw;
use pw::spa::{
    self,
    param::{
        ParamType,
        format::{MediaSubtype, MediaType},
        format_utils,
    },
    utils::dict::DictRef,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use super::{Backend, Command, Connection, Event, RequestId};
use crate::graph::{
    ClientRecord, DeviceRecord, Direction, Graph, GraphDelta, LinkRecord, LinkState, NodeRecord, PortRecord, Properties, Upsert,
    classify::PortKind,
    legal::{LinkLegality, can_link},
};

type Sink = Box<dyn Fn(Event) + Send>;
type Objects = Rc<RefCell<HashMap<u32, Bound>>>;
type Pending = Rc<RefCell<Batch>>;

type Actions = Rc<RefCell<HashMap<RequestId, Action>>>;
enum Action {
    Connect { output: u32, input: u32, proxy: pw::link::Link, started: Instant },
    Disconnect { link: u32, sequence: Option<i32>, started: Instant },
}
fn complete(
    actions: &Actions,
    results: &Rc<RefCell<Vec<(RequestId, Result<(), String>)>>>,
    request: RequestId,
    result: Result<(), String>,
) {
    if actions.borrow_mut().remove(&request).is_some() {
        results.borrow_mut().push((request, result));
    }
}
fn link_seen(
    actions: &Actions,
    results: &Rc<RefCell<Vec<(RequestId, Result<(), String>)>>>,
    output: u32,
    input: u32,
    state: &LinkState,
) {
    let request = actions
        .borrow()
        .iter()
        .filter_map(|(&id, action)| match action {
            Action::Connect { output: a, input: b, .. } if *a == output && *b == input => Some(id),
            _ => None,
        })
        .min();
    if let Some(id) = request {
        match state {
            LinkState::Active => complete(actions, results, id, Ok(())),
            LinkState::Error(reason) => complete(actions, results, id, Err(reason.clone())),
            LinkState::Negotiating => {}
        }
    }
}

#[derive(Default)]
struct Batch {
    epoch: u64,
    changed: HashMap<u32, Upsert>,
    removed: Vec<u32>,
}
impl Batch {
    fn upsert(&mut self, value: Upsert) {
        self.changed.insert(value.id(), value);
    }
    fn remove(&mut self, id: u32) {
        self.changed.remove(&id);
        self.removed.push(id);
    }
    fn drain(&mut self) -> Option<GraphDelta> {
        if self.changed.is_empty() && self.removed.is_empty() {
            return None;
        }
        Some(GraphDelta {
            epoch: self.epoch,
            upserts: self.changed.drain().map(|(_, v)| v).collect(),
            removals: std::mem::take(&mut self.removed),
        })
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
    if let Some(dict) = from {
        to.extend(dict.iter().map(|(k, v)| (k.to_owned(), v.to_owned())));
    }
}
fn prop_id(p: &Properties, key: &str) -> Option<u32> {
    p.get(key).and_then(|s| s.parse().ok())
}
fn format_kind(pod: &spa::pod::Pod) -> PortKind {
    match format_utils::parse_format(pod) {
        Ok((MediaType::Audio, MediaSubtype::Raw)) => PortKind::Audio,
        Ok((MediaType::Application, MediaSubtype::Control)) => PortKind::Midi,
        Ok((MediaType::Video, _)) => PortKind::Video,
        _ => PortKind::Other,
    }
}

fn bind(
    global: &pw::registry::GlobalObject<&DictRef>,
    registry: &pw::registry::RegistryRc,
    objects: &Objects,
    pending: &Pending,
    actions: &Actions,
    results: &Rc<RefCell<Vec<(RequestId, Result<(), String>)>>>,
) {
    let id = global.id;
    let p = props(global.props);
    let result: Result<Bound, pw::Error> = match global.type_ {
        pw::types::ObjectType::Node => registry.bind::<pw::node::Node, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(NodeRecord {
                id,
                device_id: prop_id(&p, "device.id"),
                client_id: prop_id(&p, "client.id"),
                props: p.clone(),
            }));
            pending.borrow_mut().upsert(Upsert::Node(state.borrow().clone()));
            let batch = pending.clone();
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    let mut record = state.borrow_mut();
                    merge(&mut record.props, info.props());
                    record.device_id = prop_id(&record.props, "device.id");
                    record.client_id = prop_id(&record.props, "client.id");
                    batch.borrow_mut().upsert(Upsert::Node(record.clone()));
                })
                .register();
            Bound::Node(listener, proxy)
        }),
        pw::types::ObjectType::Port => registry.bind::<pw::port::Port, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(PortRecord {
                id,
                node_id: prop_id(&p, "node.id"),
                direction: Direction::Other,
                enum_formats: Vec::new(),
                props: p.clone(),
            }));
            pending.borrow_mut().upsert(Upsert::Port(state.borrow().clone()));
            let info_state = state.clone();
            let param_state = state;
            let info_batch = pending.clone();
            let param_batch = pending.clone();
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    let mut record = info_state.borrow_mut();
                    merge(&mut record.props, info.props());
                    record.node_id = prop_id(&record.props, "node.id");
                    record.direction = match info.direction() {
                        spa::utils::Direction::Input => Direction::Input,
                        spa::utils::Direction::Output => Direction::Output,
                        _ => Direction::Other,
                    };
                    info_batch.borrow_mut().upsert(Upsert::Port(record.clone()));
                })
                .param(move |_, kind, _, _, pod| {
                    if kind != ParamType::EnumFormat {
                        return;
                    }
                    if let Some(pod) = pod {
                        let category = format_kind(pod);
                        let mut record = param_state.borrow_mut();
                        if !record.enum_formats.contains(&category) {
                            record.enum_formats.push(category);
                        }
                        param_batch.borrow_mut().upsert(Upsert::Port(record.clone()));
                    }
                })
                .register();
            proxy.enum_params(0, Some(ParamType::EnumFormat), 0, u32::MAX);
            Bound::Port(listener, proxy)
        }),
        pw::types::ObjectType::Link => registry.bind::<pw::link::Link, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(LinkRecord {
                id,
                output: prop_id(&p, "link.output.port").unwrap_or(0),
                input: prop_id(&p, "link.input.port").unwrap_or(0),
                state: LinkState::Negotiating,
                props: p.clone(),
            }));
            let batch = pending.clone();
            let link_actions = actions.clone();
            let link_results = results.clone();
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    let mut record = state.borrow_mut();
                    merge(&mut record.props, info.props());
                    record.output = info.output_port_id();
                    record.input = info.input_port_id();
                    record.state = match info.state() {
                        pw::link::LinkState::Active => LinkState::Active,
                        pw::link::LinkState::Error(reason) => LinkState::Error(reason.to_owned()),
                        _ => LinkState::Negotiating,
                    };
                    batch.borrow_mut().upsert(Upsert::Link(record.clone()));
                    link_seen(&link_actions, &link_results, record.output, record.input, &record.state);
                })
                .register();
            Bound::Link(listener, proxy)
        }),
        pw::types::ObjectType::Device => registry.bind::<pw::device::Device, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(DeviceRecord { id, props: p.clone() }));
            pending.borrow_mut().upsert(Upsert::Device(state.borrow().clone()));
            let batch = pending.clone();
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    let mut record = state.borrow_mut();
                    merge(&mut record.props, info.props());
                    batch.borrow_mut().upsert(Upsert::Device(record.clone()));
                })
                .register();
            Bound::Device(listener, proxy)
        }),
        pw::types::ObjectType::Client => registry.bind::<pw::client::Client, _>(global).map(|proxy| {
            let state = Rc::new(RefCell::new(ClientRecord { id, props: p.clone() }));
            pending.borrow_mut().upsert(Upsert::Client(state.borrow().clone()));
            let batch = pending.clone();
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    let mut record = state.borrow_mut();
                    merge(&mut record.props, info.props());
                    batch.borrow_mut().upsert(Upsert::Client(record.clone()));
                })
                .register();
            Bound::Client(listener, proxy)
        }),
        _ => return,
    };
    if let Ok(bound) = result {
        objects.borrow_mut().insert(id, bound);
    }
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
fn connect(
    loop_: &pw::main_loop::MainLoopRc,
    pending: &Pending,
    ready: &Rc<RefCell<Option<String>>>,
    failure: &Rc<RefCell<Option<String>>>,
    actions: &Actions,
    results: &Rc<RefCell<Vec<(RequestId, Result<(), String>)>>>,
) -> Result<Session, String> {
    let context = pw::context::ContextRc::new(loop_, None).map_err(|e| e.to_string())?;
    let core = context
        .connect_rc(Some(pw::properties::properties! {
            "application.name" => "Musubi", "application.id" => "org.aizome.Musubi", "media.category" => "Manager"
        }))
        .map_err(|e| e.to_string())?;
    let registry = core.get_registry_rc().map_err(|e| e.to_string())?;
    let objects: Objects = Default::default();
    let items = objects.clone();
    let batch = pending.clone();
    let reg = registry.clone();
    let seen_actions = actions.clone();
    let seen_results = results.clone();
    let removed = objects.clone();
    let removed_batch = pending.clone();
    let removed_actions = actions.clone();
    let removed_results = results.clone();
    let registry_listener = registry
        .add_listener_local()
        .global(move |global| bind(global, &reg, &items, &batch, &seen_actions, &seen_results))
        .global_remove(move |id| {
            removed.borrow_mut().remove(&id);
            removed_batch.borrow_mut().remove(id);
            let requests: Vec<_> = removed_actions
                .borrow()
                .iter()
                .filter_map(|(&request, action)| match action {
                    Action::Disconnect { link, .. } if *link == id => Some(request),
                    _ => None,
                })
                .collect();
            for request in requests {
                complete(&removed_actions, &removed_results, request, Ok(()));
            }
        })
        .register();
    let version = Rc::new(RefCell::new(String::from("PipeWire")));
    let info_version = version.clone();
    let done_version = version;
    let up = ready.clone();
    let down = failure.clone();
    let error_actions = actions.clone();
    let error_results = results.clone();
    let core_listener = core
        .add_listener_local()
        .info(move |info| *info_version.borrow_mut() = info.version().to_owned())
        .done(move |id, _| {
            if id == pw::core::PW_ID_CORE {
                *up.borrow_mut() = Some(done_version.borrow().clone());
            }
        })
        .error(move |id, seq, code, message| {
            use pw::proxy::ProxyT;
            let requests: Vec<_> = error_actions
                .borrow()
                .iter()
                .filter_map(|(&request, action)| match action {
                    Action::Connect { proxy, .. } if proxy.upcast_ref().id() == id => Some(request),
                    Action::Disconnect { sequence: Some(s), .. } if id != pw::core::PW_ID_CORE && *s == seq => {
                        Some(request)
                    }
                    _ => None,
                })
                .collect();
            for request in &requests {
                complete(&error_actions, &error_results, *request, Err(format!("PipeWire error {code}: {message}")));
            }
            // The server also reports per-object protocol errors on the core
            // ("unknown resource N", -ENOENT, for a proxy whose global just went
            // away). Only a broken connection (-EPIPE) takes the session down.
            if requests.is_empty() && id == pw::core::PW_ID_CORE && code == -libc::EPIPE {
                *down.borrow_mut() = Some(format!("PipeWire error {code}: {message}"));
            }
        })
        .register();
    core.sync(0).map_err(|e| e.to_string())?;
    Ok(Session {
        objects,
        _registry_listener: registry_listener,
        _core_listener: core_listener,
        _registry: registry,
        _core: core,
        _context: context,
    })
}

fn execute(
    command: Command,
    session: &Rc<RefCell<Option<Session>>>,
    graph: &Rc<RefCell<Graph>>,
    pending: &Pending,
    actions: &Actions,
    results: &Rc<RefCell<Vec<(RequestId, Result<(), String>)>>>,
) {
    let (request, outcome) = match command {
        Command::Connect { out, input, confirmed, request } => {
            let graph = graph.borrow();
            let mut current = Graph::default();
            // Include registry updates still waiting for the next 16 ms frame.
            current.apply(GraphDelta {
                epoch: graph.epoch,
                upserts: graph
                    .nodes
                    .values()
                    .cloned()
                    .map(Upsert::Node)
                    .chain(graph.ports.values().cloned().map(Upsert::Port))
                    .collect(),
                removals: vec![],
            });
            let batch = pending.borrow();
            current.apply(GraphDelta {
                epoch: batch.epoch,
                upserts: batch.changed.values().cloned().collect(),
                removals: batch.removed.clone(),
            });
            let result = if out.epoch != current.epoch || input.epoch != current.epoch {
                Err("stale connection epoch".into())
            } else if !current.ports.contains_key(&out.id) || !current.ports.contains_key(&input.id) {
                Err("port not found".into())
            } else {
                match can_link(&current, out.id, input.id) {
                    LinkLegality::Allowed => Ok(()),
                    LinkLegality::NeedsConfirmation(_) if confirmed => Ok(()),
                    legality => Err(format!("illegal link: {legality:?}")),
                }
            };
            let result = result.and_then(|()| {
                let source = current.ports[&out.id].node_id.ok_or("output node not found")?;
                let target = current.ports[&input.id].node_id.ok_or("input node not found")?;
                let borrowed = session.borrow();
                let active = borrowed.as_ref().ok_or("PipeWire unavailable")?;
                let proxy = active
                    ._core
                    .create_object::<pw::link::Link>(
                        "link-factory",
                        &pw::properties::properties! {
                            "link.output.node" => source.to_string(), "link.output.port" => out.id.to_string(),
                            "link.input.node" => target.to_string(), "link.input.port" => input.id.to_string(),
                            "object.linger" => "true"
                        },
                    )
                    .map_err(|e| e.to_string())?;
                actions.borrow_mut().insert(
                    request,
                    Action::Connect { output: out.id, input: input.id, proxy, started: Instant::now() },
                );
                Ok(())
            });
            (request, result)
        }
        Command::Disconnect { link, request } => {
            let graph = graph.borrow();
            let batch = pending.borrow();
            let result = if link.epoch != batch.epoch {
                Err("stale connection epoch".into())
            } else if (!graph.links.contains_key(&link.id)
                && !matches!(batch.changed.get(&link.id), Some(Upsert::Link(_))))
                || batch.removed.contains(&link.id)
            {
                Err(format!("link {} not found", link.id))
            } else {
                let borrowed = session.borrow();
                let active = borrowed.as_ref().ok_or_else(|| "PipeWire unavailable".to_owned());
                active.and_then(|s| s._registry.destroy_global(link.id).into_result().map_err(|e| e.to_string())).map(
                    |success| {
                        let sequence = match success {
                            spa::utils::result::SpaSuccess::Async(seq) => Some(seq.seq()),
                            _ => None,
                        };
                        actions
                            .borrow_mut()
                            .insert(request, Action::Disconnect { link: link.id, sequence, started: Instant::now() });
                    },
                )
            };
            (request, result)
        }
        Command::Shutdown => return,
    };
    if let Err(reason) = outcome {
        results.borrow_mut().push((request, Err(reason)));
    }
}

pub struct PipeWireBackend {
    tx: pw::channel::Sender<Command>,
    thread: Option<thread::JoinHandle<()>>,
    events: Option<mpsc::Receiver<Event>>,
}
impl PipeWireBackend {
    pub fn start(sink: impl Fn(Event) + Send + 'static) -> Result<Self, String> {
        let (tx, rx) = pw::channel::channel();
        let thread = thread::Builder::new()
            .name("musubi-pipewire".into())
            .spawn(move || run(rx, Box::new(sink)))
            .map_err(|e| e.to_string())?;
        Ok(Self { tx, thread: Some(thread), events: None })
    }
    pub fn channel() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let mut backend = Self::start(move |event| {
            let _ = tx.send(event);
        })?;
        backend.events = Some(rx);
        Ok(backend)
    }
    pub fn recv_timeout(&self, duration: Duration) -> Result<Event, mpsc::RecvTimeoutError> {
        self.events.as_ref().expect("channel backend required").recv_timeout(duration)
    }
    pub fn shutdown(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = self.tx.send(Command::Shutdown);
            let _ = thread.join();
        }
    }
}
impl Backend for PipeWireBackend {
    fn command(&mut self, command: Command) {
        let _ = self.tx.send(command);
    }
    fn next_event(&mut self) -> Option<Event> {
        self.events.as_ref().and_then(|events| events.try_recv().ok())
    }
}
impl Drop for PipeWireBackend {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run(rx: pw::channel::Receiver<Command>, sink: Sink) {
    pw::init();
    let Ok(mainloop) = pw::main_loop::MainLoopRc::new(None) else {
        sink(Event::Connection(Connection::Down("Cannot create PipeWire loop".into())));
        return;
    };
    let pending: Pending = Default::default();
    let actions: Actions = Default::default();
    let results: Rc<RefCell<Vec<(RequestId, Result<(), String>)>>> = Default::default();
    let graph = Rc::new(RefCell::new(Graph::default()));
    let last_probe = Rc::new(RefCell::new(Instant::now()));
    let ready = Rc::new(RefCell::new(None));
    let failure = Rc::new(RefCell::new(None));
    let session: Rc<RefCell<Option<Session>>> = Default::default();
    let next_try = Rc::new(RefCell::new(Instant::now()));
    let backoff = Rc::new(RefCell::new(Duration::from_millis(500)));
    let pending_tick = pending.clone();
    let ready_tick = ready.clone();
    let failure_tick = failure.clone();
    let session_tick = session.clone();
    let next_tick = next_try.clone();
    let backoff_tick = backoff.clone();
    let actions_tick = actions.clone();
    let results_tick = results.clone();
    let graph_tick = graph.clone();
    let probe_tick = last_probe.clone();
    let sink = Rc::new(sink);
    let tick_sink = sink.clone();
    let tick_loop = mainloop.clone();
    let timer = mainloop.loop_().add_timer(move |_| {
        if let Some(reason) = failure_tick.borrow_mut().take() {
            for request in actions_tick.borrow().keys().copied().collect::<Vec<_>>() {
                complete(&actions_tick, &results_tick, request, Err(format!("PipeWire down: {reason}")));
            }
            session_tick.borrow_mut().take();
            let mut batch = pending_tick.borrow_mut();
            batch.changed.clear();
            batch.removed.clear();
            batch.epoch += 1;
            graph_tick.borrow_mut().apply(GraphDelta { epoch: batch.epoch, ..GraphDelta::default() });
            tick_sink(Event::Connection(Connection::Down(reason)));
            *ready_tick.borrow_mut() = None;
            *next_tick.borrow_mut() = Instant::now() + *backoff_tick.borrow();
            let doubled = (*backoff_tick.borrow() * 2).min(Duration::from_secs(5));
            *backoff_tick.borrow_mut() = doubled;
        }
        if session_tick.borrow().is_none() && Instant::now() >= *next_tick.borrow() {
            match connect(&tick_loop, &pending_tick, &ready_tick, &failure_tick, &actions_tick, &results_tick) {
                Ok(connected) => {
                    *session_tick.borrow_mut() = Some(connected);
                }
                Err(reason) => {
                    *failure_tick.borrow_mut() = Some(reason);
                }
            }
        }
        if let Some(version) = ready_tick.borrow_mut().take() {
            *backoff_tick.borrow_mut() = Duration::from_millis(500);
            tick_sink(Event::Connection(Connection::Up(version)));
        }
        if let Some(delta) = pending_tick.borrow_mut().drain() {
            graph_tick.borrow_mut().apply(delta.clone());
            tick_sink(Event::Delta(delta));
        }
        let expired: Vec<_> = actions_tick
            .borrow()
            .iter()
            .filter_map(|(&id, action)| {
                let started = match action {
                    Action::Connect { started, .. } | Action::Disconnect { started, .. } => started,
                };
                (started.elapsed() >= Duration::from_secs(3)).then_some(id)
            })
            .collect();
        for request in expired {
            complete(&actions_tick, &results_tick, request, Err("PipeWire request timed out after 3 s".into()));
        }
        for (request, result) in results_tick.borrow_mut().drain(..) {
            tick_sink(Event::CommandResult { request, result });
        }
        if probe_tick.borrow().elapsed() >= Duration::from_secs(1) {
            *probe_tick.borrow_mut() = Instant::now();
            if let Some(active) = session_tick.borrow().as_ref() {
                if let Err(error) = active._core.sync(0) {
                    *failure_tick.borrow_mut() = Some(format!("PipeWire disconnected: {error}"));
                }
            }
        }
    });
    let _ = timer.update_timer(Some(Duration::from_millis(1)), Some(Duration::from_millis(16)));
    let stop = mainloop.clone();
    let command_session = session.clone();
    let command_graph = graph.clone();
    let command_actions = actions.clone();
    let command_results = results.clone();
    let _attached = rx.attach(mainloop.loop_(), move |cmd| match cmd {
        Command::Shutdown => stop.quit(),
        command => execute(command, &command_session, &command_graph, &pending, &command_actions, &command_results),
    });
    mainloop.run();
    drop(timer);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coalesces_repeated_changes_and_removal() {
        let mut batch = Batch::default();
        batch.epoch = 8;
        batch.upsert(Upsert::Client(ClientRecord { id: 4, props: Properties::new() }));
        batch.upsert(Upsert::Client(ClientRecord {
            id: 4,
            props: Properties::from([("name".into(), "updated".into())]),
        }));
        let delta = batch.drain().unwrap();
        assert_eq!(delta.epoch, 8);
        assert_eq!(delta.upserts.len(), 1);
        assert!(batch.drain().is_none());
        batch.remove(4);
        let delta = batch.drain().unwrap();
        assert_eq!(delta.removals, vec![4]);
    }
}
