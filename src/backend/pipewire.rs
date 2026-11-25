//! PipeWire's non-Send objects stay on one thread; only graph values leave it.
use pipewire as pw;
use pw::spa::{
    self,
    param::{
        ParamType,
        format::{MediaSubtype, MediaType},
        format_utils,
    },
    pod::{Object, Pod, Property, Value, ValueArray, deserialize::PodDeserializer, serialize::PodSerializer},
    utils::dict::DictRef,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    collections::HashSet,
    io::Cursor,
    rc::Rc,
    sync::Arc,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use super::ClockKey;
use super::{Backend, Command, Connection, Event, RequestId};
use super::{
    Handle,
    meters::{MAX_METERS, MeterReading, MeterStore, Slot},
};
use crate::clock::{self, Settings};
use crate::graph::{
    ClientRecord, DeviceRecord, Direction, Graph, GraphDelta, LinkRecord, LinkState, NativeProps, NodeRecord,
    PortRecord, Properties, Upsert,
    classify::PortKind,
    legal::{LinkLegality, can_link},
};

type Sink = Box<dyn Fn(Event) + Send>;
type Objects = Rc<RefCell<HashMap<u32, Bound>>>;
type Pending = Rc<RefCell<Batch>>;

type ClockState = Rc<RefCell<Settings>>;
type ClockEvents = Rc<RefCell<Vec<Event>>>;
type ClockWrites = Rc<RefCell<HashMap<RequestId, (ClockKey, Option<u32>, Instant)>>>;
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
    Node(pw::node::NodeListener, Rc<pw::node::Node>),
    Port(pw::port::PortListener, pw::port::Port),
    Link(pw::link::LinkListener, pw::link::Link),
    Device(pw::device::DeviceListener, pw::device::Device),
    Client(pw::client::ClientListener, pw::client::Client),
    Metadata(pw::metadata::MetadataListener, pw::metadata::Metadata, bool),
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
fn control_kind(types: u32) -> PortKind {
    let midi = 1_u32 << spa::sys::SPA_CONTROL_Midi;
    let ump = 1_u32 << spa::sys::SPA_CONTROL_UMP;
    match types {
        value if value == midi => PortKind::Midi,
        value if value == ump => PortKind::Ump,
        _ => PortKind::Other,
    }
}
fn format_kind(pod: &spa::pod::Pod) -> PortKind {
    match format_utils::parse_format(pod) {
        Ok((MediaType::Audio, MediaSubtype::Raw)) => PortKind::Audio,
        Ok((MediaType::Application, MediaSubtype::Control)) => pod
            .as_object()
            .ok()
            .and_then(|object| object.find_prop(spa::utils::Id(spa::sys::SPA_FORMAT_CONTROL_types)))
            .and_then(|property| property.value().get_int().ok())
            .map_or(PortKind::Other, |types| control_kind(types as u32)),
        Ok((MediaType::Video, _)) => PortKind::Video,
        _ => PortKind::Other,
    }
}

fn observed_props(pod: &Pod, previous: &NativeProps) -> NativeProps {
    // The parameter callback is the sole source of gain/mute values; command submission
    // never changes a snapshot.
    let Ok(value) = (unsafe {
        PodDeserializer::deserialize_ptr::<Value>(std::ptr::NonNull::new(pod.as_raw_ptr()).expect("pod pointer"))
    }) else {
        return previous.clone();
    };
    let Value::Object(object) = value else { return previous.clone() };
    let mut observed = NativeProps { writable: previous.writable, ..NativeProps::default() };
    for prop in object.properties {
        match (prop.key, prop.value) {
            (spa::sys::SPA_PROP_channelVolumes, Value::ValueArray(ValueArray::Float(v))) => {
                observed.channel_volumes = Some(v);
            }
            (spa::sys::SPA_PROP_mute, Value::Bool(v)) => observed.muted = Some(v),
            (spa::sys::SPA_PROP_params, Value::Struct(values)) => {
                let mut values = values.into_iter();
                while let (Some(key), Some(value)) = (values.next(), values.next()) {
                    if let (Value::String(key), Value::Float(value)) = (key, value) {
                        if key.starts_with("left") || key.starts_with("right") {
                            observed.effect_params.push((key, value));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    observed
}

fn write_props(proxy: &pw::node::Node, property: Property) -> Result<(), String> {
    let object =
        Object { type_: spa::sys::SPA_TYPE_OBJECT_Props, id: spa::sys::SPA_PARAM_Props, properties: vec![property] };
    let bytes = PodSerializer::serialize(Cursor::new(Vec::new()), &Value::Object(object))
        .map_err(|e| format!("Cannot encode Props: {e:?}"))?
        .0
        .into_inner();
    let pod = Pod::from_bytes(&bytes).ok_or("Cannot decode encoded Props")?;
    proxy.set_param(ParamType::Props, 0, pod);
    Ok(())
}

fn state_value(settings: &Settings, key: ClockKey) -> Option<u32> {
    match key {
        ClockKey::Rate => settings.force_rate,
        ClockKey::Quantum => settings.force_quantum,
    }
}
fn bind(
    global: &pw::registry::GlobalObject<&DictRef>,
    registry: &pw::registry::RegistryRc,
    objects: &Objects,
    pending: &Pending,
    actions: &Actions,
    results: &Rc<RefCell<Vec<(RequestId, Result<(), String>)>>>,
    clock: &ClockState,
    clock_events: &ClockEvents,
    clock_writes: &ClockWrites,
) {
    let id = global.id;
    let p = props(global.props);
    let result: Result<Bound, pw::Error> = match global.type_ {
        pw::types::ObjectType::Metadata if p.get("metadata.name").is_some_and(|name| name == "settings") => {
            registry.bind::<pw::metadata::Metadata, _>(global).map(|proxy| {
                let writable = global
                    .permissions
                    .contains(pw::permissions::PermissionFlags::W | pw::permissions::PermissionFlags::X);
                let observed = clock.clone();
                let events = clock_events.clone();
                let writes = clock_writes.clone();
                let done = results.clone();
                {
                    let mut state = observed.borrow_mut();
                    state.read_only = (!writable)
                        .then(|| "PipeWire settings metadata is read-only (missing write permission)".into());
                    events.borrow_mut().push(Event::ClockSettings(state.clone()));
                }
                let listener = proxy
                    .add_listener_local()
                    .property(move |subject, key, _, value| {
                        if observed.borrow_mut().observe(subject, key, value) {
                            let state = observed.borrow().clone();
                            events.borrow_mut().push(Event::ClockSettings(state.clone()));
                            let completed: Vec<_> = writes
                                .borrow()
                                .iter()
                                .filter_map(|(&request, (force, expected, _))| {
                                    (key == Some(force.name()) && state_value(&state, *force) == *expected)
                                        .then_some(request)
                                })
                                .collect();
                            for request in completed {
                                writes.borrow_mut().remove(&request);
                                done.borrow_mut().push((request, Ok(())));
                            }
                        }
                        0
                    })
                    .register();
                Bound::Metadata(listener, proxy, writable)
            })
        }
        pw::types::ObjectType::Node => registry.bind::<pw::node::Node, _>(global).map(|proxy| {
            let proxy = Rc::new(proxy);
            let can_write =
                global.permissions.contains(pw::permissions::PermissionFlags::W | pw::permissions::PermissionFlags::X);
            let state = Rc::new(RefCell::new(NodeRecord {
                id,
                device_id: prop_id(&p, "device.id"),
                client_id: prop_id(&p, "client.id"),
                props: p.clone(),
                native: NativeProps::default(),
            }));
            pending.borrow_mut().upsert(Upsert::Node(state.borrow().clone()));
            let info_state = state.clone();
            let param_state = state;
            let info_batch = pending.clone();
            let param_batch = pending.clone();
            let info_proxy = proxy.clone();
            let listener = proxy
                .add_listener_local()
                .info(move |info| {
                    let mut record = info_state.borrow_mut();
                    merge(&mut record.props, info.props());
                    record.device_id = prop_id(&record.props, "device.id");
                    record.client_id = prop_id(&record.props, "client.id");
                    let props_param = info.params().iter().find(|param| param.id() == ParamType::Props);
                    record.native.writable = can_write
                        && props_param.is_some_and(|param| param.flags().contains(spa::param::ParamInfoFlags::WRITE));
                    info_batch.borrow_mut().upsert(Upsert::Node(record.clone()));
                    if props_param.is_some() {
                        info_proxy.enum_params(0, Some(ParamType::Props), 0, u32::MAX);
                    }
                })
                .param(move |_, kind, _, _, pod| {
                    if kind == ParamType::Props {
                        if let Some(pod) = pod {
                            let mut record = param_state.borrow_mut();
                            record.native = observed_props(pod, &record.native);
                            param_batch.borrow_mut().upsert(Upsert::Node(record.clone()));
                        }
                    }
                })
                .register();
            proxy.subscribe_params(&[ParamType::Props]);
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
                        if let Ok((MediaType::Video, subtype)) = format_utils::parse_format(pod) {
                            if let Some(family) = match subtype {
                                MediaSubtype::Raw => Some("raw"),
                                MediaSubtype::Dsp => Some("dsp"),
                                _ => None,
                            } {
                                record.props.insert("video.enum.family".into(), family.into());
                            }
                        }
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

struct MeterStream {
    // Listener must be destroyed before its stream.
    _listener: pw::stream::StreamListener<()>,
    _stream: pw::stream::StreamRc,
}

fn start_meter(
    core: &pw::core::CoreRc,
    node: &NodeRecord,
    store: &MeterStore,
) -> Result<(MeterStream, Arc<Slot>), String> {
    let channels = node.props.get("audio.channels").and_then(|v| v.parse::<usize>().ok()).unwrap_or(2).clamp(1, 64);
    let serial = node.props.get("object.serial").ok_or("meter target has no object.serial")?;
    let slot = Arc::new(Slot::new(channels));
    let props = pw::properties::properties! {
        "media.type" => "Audio", "media.category" => "Capture", "media.role" => "Measurement",
        "media.name" => "Musubi level meter",
        "application.name" => "Musubi", "application.id" => "org.aizome.Musubi",
        "musubi.meter" => "true",
        "target.object" => serial.as_str(),
        "node.passive" => "true", "node.dont-reconnect" => "true",
        "stream.monitor" => "true",
        "resample.peaks" => "true", "node.rate" => "1/25", "node.latency" => "1/25",
        "audio.channels" => channels.to_string().as_str(),
        "stream.capture.sink" => if node.props.get("media.class").is_some_and(|v| v == "Audio/Sink") { "true" } else { "false" },
    };
    let stream = pw::stream::StreamRc::new(core.clone(), "Musubi level meter", props).map_err(|e| e.to_string())?;
    let process_slot = slot.clone();
    let origin = store.clone();
    let listener = stream
        .add_local_listener::<()>()
        .process(move |stream, _| unsafe {
            let buffer = stream.dequeue_raw_buffer();
            if buffer.is_null() {
                return;
            }
            let spa_buffer = (*buffer).buffer;
            if !spa_buffer.is_null() && (*spa_buffer).n_datas > 0 {
                let data = &*(*spa_buffer).datas;
                if !data.data.is_null() && !data.chunk.is_null() {
                    let chunk = &*data.chunk;
                    let offset = chunk.offset as usize;
                    let size = chunk.size as usize;
                    if offset <= data.maxsize as usize && size <= data.maxsize as usize - offset {
                        let bytes = std::slice::from_raw_parts((data.data as *const u8).add(offset), size);
                        let stride = (chunk.stride as usize).max(channels * 4);
                        process_slot.process(bytes, stride, origin.elapsed_ns());
                    }
                }
            }
            stream.queue_raw_buffer(buffer);
        })
        .register()
        .map_err(|e| e.to_string())?;
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(144);
    info.set_channels(channels as u32);
    let mut position = [spa::sys::SPA_AUDIO_CHANNEL_UNKNOWN; 64];
    if channels == 1 {
        position[0] = spa::sys::SPA_AUDIO_CHANNEL_MONO;
    } else {
        position[0] = spa::sys::SPA_AUDIO_CHANNEL_FL;
        position[1] = spa::sys::SPA_AUDIO_CHANNEL_FR;
        for (i, channel) in position.iter_mut().enumerate().take(channels).skip(2) {
            *channel = spa::sys::SPA_AUDIO_CHANNEL_AUX0 + (i as u32 - 2);
        }
    }
    info.set_position(position);
    let bytes = PodSerializer::serialize(
        Cursor::new(Vec::new()),
        &Value::Object(Object {
            type_: spa::sys::SPA_TYPE_OBJECT_Format,
            id: spa::sys::SPA_PARAM_EnumFormat,
            properties: info.into(),
        }),
    )
    .map_err(|e| format!("{e:?}"))?
    .0
    .into_inner();
    let mut params = [Pod::from_bytes(&bytes).ok_or("invalid meter format")?];
    stream
        .connect(
            spa::utils::Direction::Input,
            Some(node.id),
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS
                | pw::stream::StreamFlags::DONT_RECONNECT,
            &mut params,
        )
        .map_err(|e| e.to_string())?;
    Ok((MeterStream { _listener: listener, _stream: stream }, slot))
}

struct Session {
    // Order matters: listener hooks and bound proxies are destroyed before registry/core.
    meters: HashMap<Handle, MeterStream>,
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
    clock_defaults: &Rc<RefCell<Option<(u32, u32)>>>,
    actions: &Actions,
    results: &Rc<RefCell<Vec<(RequestId, Result<(), String>)>>>,
    clock: &ClockState,
    clock_events: &ClockEvents,
    clock_writes: &ClockWrites,
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
    let seen_clock = clock.clone();
    let seen_clock_events = clock_events.clone();
    let seen_clock_writes = clock_writes.clone();
    let removed = objects.clone();
    let removed_batch = pending.clone();
    let removed_actions = actions.clone();
    let removed_results = results.clone();
    let registry_listener = registry
        .add_listener_local()
        .global(move |global| {
            bind(
                global,
                &reg,
                &items,
                &batch,
                &seen_actions,
                &seen_results,
                &seen_clock,
                &seen_clock_events,
                &seen_clock_writes,
            )
        })
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
    let info_clock = clock_defaults.clone();
    let done_version = version;
    let up = ready.clone();
    let down = failure.clone();
    let error_actions = actions.clone();
    let error_results = results.clone();
    let error_clock = clock.clone();
    let error_clock_events = clock_events.clone();
    let error_writes = clock_writes.clone();
    let error_objects = objects.clone();
    let core_listener = core
        .add_listener_local()
        .info(move |info| {
            *info_version.borrow_mut() = info.version().to_owned();
            if let Some(props) = info.props() {
                if let (Some(rate), Some(quantum)) = (
                    props.get("default.clock.rate").and_then(|v| v.parse::<u32>().ok()).filter(|&v| v > 0),
                    props.get("default.clock.quantum").and_then(|v| v.parse::<u32>().ok()).filter(|&v| v > 0),
                ) {
                    *info_clock.borrow_mut() = Some((rate, quantum));
                }
            }
        })
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
            let metadata_error = error_objects
                .borrow()
                .values()
                .any(|bound| matches!(bound, Bound::Metadata(_, proxy, _) if proxy.upcast_ref().id() == id));
            if metadata_error {
                let reason = format!("PipeWire settings metadata is read-only: {message} ({code})");
                error_clock.borrow_mut().read_only = Some(reason.clone());
                error_clock_events.borrow_mut().push(Event::ClockSettings(error_clock.borrow().clone()));
                for request in error_writes.borrow().keys().copied().collect::<Vec<_>>() {
                    error_writes.borrow_mut().remove(&request);
                    error_results.borrow_mut().push((request, Err(reason.clone())));
                }
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
        meters: HashMap::new(),
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
    meter_store: &MeterStore,
    clock_writes: &ClockWrites,
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
        Command::SetEffectGain { node, left, right, request } => {
            let result = if node.epoch != pending.borrow().epoch {
                Err("stale connection epoch".into())
            } else if !left.is_finite() || !right.is_finite() {
                Err("non-finite filter gain".into())
            } else {
                let borrowed = session.borrow();
                let active = borrowed.as_ref().ok_or("PipeWire unavailable".to_owned());
                active.and_then(|active| {
                    let objects = active.objects.borrow();
                    let Some(Bound::Node(_, proxy)) = objects.get(&node.id) else {
                        return Err("effect node proxy not found".into());
                    };
                    write_props(
                        proxy,
                        Property::new(
                            spa::sys::SPA_PROP_params,
                            Value::Struct(vec![
                                Value::String("left:Gain".into()),
                                Value::Float(left),
                                Value::String("right:Gain".into()),
                                Value::Float(right),
                            ]),
                        ),
                    )
                })
            };
            if result.is_ok() {
                results.borrow_mut().push((request, Ok(())));
            }
            (request, result)
        }
        Command::SetEffectParams { node, params, request } => {
            let result = if node.epoch != pending.borrow().epoch {
                Err("stale connection epoch".into())
            } else if params.is_empty()
                || params.iter().any(|(key, value)| {
                    !value.is_finite()
                        || !["Freq", "Q", "Gain"].iter().any(|suffix| {
                            key.strip_suffix(suffix).is_some_and(|prefix| {
                                let Some(indexed) = prefix.strip_suffix(':') else { return false };
                                ["left", "right"].iter().any(|side| {
                                    indexed.strip_prefix(side).is_some_and(|index| {
                                        !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit())
                                    })
                                })
                            })
                        })
                })
            {
                Err("invalid effect control".into())
            } else {
                let borrowed = session.borrow();
                borrowed.as_ref().ok_or("PipeWire unavailable".to_owned()).and_then(|active| {
                    let objects = active.objects.borrow();
                    let Some(Bound::Node(_, proxy)) = objects.get(&node.id) else {
                        return Err("effect node proxy not found".into());
                    };
                    let mut controls = Vec::with_capacity(params.len() * 2);
                    for (key, value) in params {
                        controls.push(Value::String(key));
                        controls.push(Value::Float(value));
                    }
                    write_props(proxy, Property::new(spa::sys::SPA_PROP_params, Value::Struct(controls)))?;
                    proxy.enum_params(0, Some(ParamType::Props), 0, u32::MAX);
                    Ok(())
                })
            };
            if result.is_ok() {
                results.borrow_mut().push((request, Ok(())));
            }
            (request, result)
        }
        Command::SetNodeVolume { node, channel_volumes, request } => {
            let result = if node.epoch != pending.borrow().epoch {
                Err("stale connection epoch".into())
            } else {
                let batch = pending.borrow();
                let observed = match batch.changed.get(&node.id) {
                    Some(Upsert::Node(n)) => Some(n.clone()),
                    _ => graph.borrow().nodes.get(&node.id).cloned(),
                };
                let current = observed.ok_or_else(|| "node not found".to_owned());
                current.and_then(|n| {
                    let channels = n.native.channel_volumes.as_ref().ok_or("node has no channelVolumes".to_owned())?;
                    if !n.native.writable {
                        return Err("node Props are not writable".into());
                    }
                    if channel_volumes.len() != channels.len()
                        || channel_volumes.is_empty()
                        || channel_volumes.iter().any(|v| !v.is_finite() || *v < 0.0)
                    {
                        return Err("invalid channel volumes".into());
                    }
                    let borrowed = session.borrow();
                    let active = borrowed.as_ref().ok_or("PipeWire unavailable".to_owned())?;
                    let objects = active.objects.borrow();
                    let Some(Bound::Node(_, proxy)) = objects.get(&node.id) else {
                        return Err("node proxy not found".into());
                    };
                    write_props(
                        proxy,
                        Property::new(
                            spa::sys::SPA_PROP_channelVolumes,
                            Value::ValueArray(ValueArray::Float(channel_volumes)),
                        ),
                    )?;
                    proxy.enum_params(0, Some(ParamType::Props), 0, 1);
                    Ok(())
                })
            };
            if result.is_ok() {
                results.borrow_mut().push((request, Ok(())));
            }
            (request, result)
        }
        Command::SetNodeMute { node, muted, request } => {
            let result = if node.epoch != pending.borrow().epoch {
                Err("stale connection epoch".into())
            } else {
                let batch = pending.borrow();
                let observed = match batch.changed.get(&node.id) {
                    Some(Upsert::Node(n)) => Some(n.clone()),
                    _ => graph.borrow().nodes.get(&node.id).cloned(),
                };
                let current = observed.ok_or_else(|| "node not found".to_owned());
                current.and_then(|n| {
                    if !n.native.writable || n.native.muted.is_none() {
                        return Err("node has no writable mute".into());
                    }
                    let borrowed = session.borrow();
                    let active = borrowed.as_ref().ok_or("PipeWire unavailable".to_owned())?;
                    let objects = active.objects.borrow();
                    let Some(Bound::Node(_, proxy)) = objects.get(&node.id) else {
                        return Err("node proxy not found".into());
                    };
                    write_props(proxy, Property::new(spa::sys::SPA_PROP_mute, Value::Bool(muted)))?;
                    proxy.enum_params(0, Some(ParamType::Props), 0, 1);
                    Ok(())
                })
            };
            if result.is_ok() {
                results.borrow_mut().push((request, Ok(())));
            }
            (request, result)
        }
        Command::SetClock { key, value, request } => {
            let result = (|| {
                if value.is_some_and(|n| n == 0) {
                    return Err("clock value must be explicit".to_owned());
                }
                let borrowed = session.borrow();
                let active = borrowed.as_ref().ok_or("PipeWire unavailable")?;
                let objects = active.objects.borrow();
                let Some((proxy, writable)) = objects.values().find_map(|bound| match bound {
                    Bound::Metadata(_, proxy, writable) => Some((proxy, *writable)),
                    _ => None,
                }) else {
                    return Err("settings metadata unavailable".to_owned());
                };
                if !writable {
                    return Err("settings metadata is read-only (missing write permission)".to_owned());
                }
                clock_writes.borrow_mut().insert(request, (key, value, Instant::now()));
                let number = clock::force_value(value);
                proxy.set_property(0, key.name(), Some("Spa:String"), Some(&number));
                Ok(())
            })();
            (request, result)
        }
        Command::StartProfiler | Command::StopProfiler => return,
        Command::StartMeters(nodes) => {
            let mut borrowed = session.borrow_mut();
            if let Some(active) = borrowed.as_mut() {
                active.meters.clear();
                let batch = pending.borrow();
                let mut slots = HashMap::new();
                let mut paused = HashSet::new();
                for handle in nodes {
                    if handle.epoch != batch.epoch || slots.contains_key(&handle) || paused.contains(&handle) {
                        continue;
                    }
                    let node = match batch.changed.get(&handle.id) {
                        Some(Upsert::Node(node)) => Some(node.clone()),
                        _ => graph.borrow().nodes.get(&handle.id).cloned(),
                    };
                    let Some(node) = node else {
                        continue;
                    };
                    if crate::graph::is_musubi_meter(&node.props) {
                        continue;
                    }
                    if slots.len() >= MAX_METERS {
                        paused.insert(handle);
                        continue;
                    }
                    match start_meter(&active._core, &node, meter_store) {
                        Ok((meter, slot)) => {
                            active.meters.insert(handle, meter);
                            slots.insert(handle, slot);
                        }
                        Err(reason) => {
                            eprintln!("Musubi meter for node {} could not start: {reason}", handle.id);
                            paused.insert(handle);
                        }
                    }
                }
                meter_store.replace(slots, paused);
            } else {
                meter_store.clear();
            }
            return;
        }
        Command::StopMeters => {
            if let Some(active) = session.borrow_mut().as_mut() {
                active.meters.clear();
            }
            meter_store.clear();
            return;
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
    meters: MeterStore,
}
impl PipeWireBackend {
    pub fn start(sink: impl Fn(Event) + Send + 'static) -> Result<Self, String> {
        let (tx, rx) = pw::channel::channel();
        let meters = MeterStore::default();
        let thread = thread::Builder::new()
            .name("musubi-pipewire".into())
            .spawn({
                let meters = meters.clone();
                move || run(rx, Box::new(sink), meters)
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { tx, thread: Some(thread), events: None, meters })
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
    fn meters(&self) -> Vec<MeterReading> {
        self.meters.readings()
    }
}
impl Drop for PipeWireBackend {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run(rx: pw::channel::Receiver<Command>, sink: Sink, meters: MeterStore) {
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
    let clock_defaults: Rc<RefCell<Option<(u32, u32)>>> = Default::default();
    let clock: ClockState = Default::default();
    let clock_events: ClockEvents = Default::default();
    let clock_writes: ClockWrites = Default::default();
    let failure = Rc::new(RefCell::new(None));
    let session: Rc<RefCell<Option<Session>>> = Default::default();
    let next_try = Rc::new(RefCell::new(Instant::now()));
    let backoff = Rc::new(RefCell::new(Duration::from_millis(500)));
    let pending_tick = pending.clone();
    let ready_tick = ready.clone();
    let failure_tick = failure.clone();
    let clock_tick = clock_defaults.clone();
    let clock_state_tick = clock.clone();
    let clock_events_tick = clock_events.clone();
    let clock_writes_tick = clock_writes.clone();
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
    let meters_tick = meters.clone();
    let timer = mainloop.loop_().add_timer(move |_| {
        if let Some(reason) = failure_tick.borrow_mut().take() {
            for request in actions_tick.borrow().keys().copied().collect::<Vec<_>>() {
                complete(&actions_tick, &results_tick, request, Err(format!("PipeWire down: {reason}")));
            }
            for request in clock_writes_tick.borrow().keys().copied().collect::<Vec<_>>() {
                clock_writes_tick.borrow_mut().remove(&request);
                results_tick.borrow_mut().push((request, Err(format!("PipeWire down: {reason}"))));
            }
            session_tick.borrow_mut().take();
            meters_tick.clear();
            let mut batch = pending_tick.borrow_mut();
            batch.changed.clear();
            batch.removed.clear();
            batch.epoch += 1;
            graph_tick.borrow_mut().apply(GraphDelta { epoch: batch.epoch, ..GraphDelta::default() });
            tick_sink(Event::Connection(Connection::Down(reason)));
            *clock_state_tick.borrow_mut() = Settings::default();
            tick_sink(Event::ClockSettings(clock_state_tick.borrow().clone()));
            *clock_tick.borrow_mut() = None;
            *ready_tick.borrow_mut() = None;
            *next_tick.borrow_mut() = Instant::now() + *backoff_tick.borrow();
            let doubled = (*backoff_tick.borrow() * 2).min(Duration::from_secs(5));
            *backoff_tick.borrow_mut() = doubled;
        }
        if session_tick.borrow().is_none() && Instant::now() >= *next_tick.borrow() {
            match connect(
                &tick_loop,
                &pending_tick,
                &ready_tick,
                &failure_tick,
                &clock_tick,
                &actions_tick,
                &results_tick,
                &clock_state_tick,
                &clock_events_tick,
                &clock_writes_tick,
            ) {
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
            if let Some((rate, quantum)) = clock_tick.borrow_mut().take() {
                tick_sink(Event::ClockDefaults { rate, quantum });
            }
        }
        // Registry removals are observed in the pending batch before it drains.
        if let Some(active) = session_tick.borrow_mut().as_mut() {
            for id in &pending_tick.borrow().removed {
                let handle = Handle { epoch: pending_tick.borrow().epoch, id: *id };
                active.meters.remove(&handle);
                meters_tick.remove(handle);
            }
        }
        if let Some(delta) = pending_tick.borrow_mut().drain() {
            graph_tick.borrow_mut().apply(delta.clone());
            tick_sink(Event::Delta(delta));
        }
        for event in clock_events_tick.borrow_mut().drain(..) {
            tick_sink(event);
        }
        for request in clock_writes_tick
            .borrow()
            .iter()
            .filter_map(|(&id, (_, _, started))| (started.elapsed() >= Duration::from_secs(3)).then_some(id))
            .collect::<Vec<_>>()
        {
            clock_writes_tick.borrow_mut().remove(&request);
            results_tick.borrow_mut().push((request, Err("settings metadata did not confirm the change".into())));
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
    let command_clock_writes = clock_writes.clone();
    let _attached = rx.attach(mainloop.loop_(), move |cmd| match cmd {
        Command::Shutdown => stop.quit(),
        command => execute(
            command,
            &command_session,
            &command_graph,
            &pending,
            &command_actions,
            &command_results,
            &meters,
            &command_clock_writes,
        ),
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
    #[test]
    fn distinguishes_control_midi_and_ump_from_enumformat_mask() {
        assert_eq!(control_kind(1 << spa::sys::SPA_CONTROL_Midi), PortKind::Midi);
        assert_eq!(control_kind(1 << spa::sys::SPA_CONTROL_UMP), PortKind::Ump);
        assert_eq!(control_kind((1 << spa::sys::SPA_CONTROL_Midi) | (1 << spa::sys::SPA_CONTROL_UMP)), PortKind::Other);
        assert_eq!(control_kind(0), PortKind::Other);
    }
}
