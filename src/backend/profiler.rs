//! PipeWire's Profiler extension has C bindings but no pipewire-rs proxy wrapper.
//! This proxy and its hook stay on the backend's PipeWire thread.
use crate::clock::DriverRow;
use pipewire as pw;
use pw::spa::{self, pod::Pod};
use std::{cell::RefCell, ffi::CString, pin::Pin, rc::Rc};

pub struct Subscription {
    hook: Pin<Box<spa::sys::spa_hook>>,
    _events: Pin<Box<pw::sys::pw_profiler_events>>,
    _callback: Box<Rc<RefCell<Vec<Vec<DriverRow>>>>>,
    proxy: *mut pw::sys::pw_proxy,
}

impl Subscription {
    pub fn bind(
        registry: &pw::registry::RegistryRc,
        id: u32,
        rows: Rc<RefCell<Vec<Vec<DriverRow>>>>,
    ) -> Result<Self, String> {
        let type_ = CString::new("PipeWire:Interface:Profiler").unwrap();
        // SAFETY: `registry` and the returned proxy are owned by this same MainLoop thread;
        // the extension version is the version in pipewire/extensions/profiler.h.
        let proxy = unsafe {
            spa::spa_interface_call_method!(
                registry.as_raw_ptr(),
                pw::sys::pw_registry_methods,
                bind,
                id,
                type_.as_ptr(),
                pw::sys::PW_VERSION_PROFILER,
                0
            )
        } as *mut pw::sys::pw_proxy;
        if proxy.is_null() {
            return Err("could not bind PipeWire Profiler".into());
        }
        let mut hook = Box::pin(unsafe { std::mem::zeroed::<spa::sys::spa_hook>() });
        let events = Box::pin(pw::sys::pw_profiler_events {
            version: pw::sys::PW_VERSION_PROFILER_EVENTS,
            profile: Some(on_profile),
        });
        let mut callback = Box::new(rows);
        // SAFETY: pinned hook/events and boxed callback outlive registration; Drop removes the
        // hook before destroying the proxy and releasing the callback.
        unsafe {
            pw::sys::pw_proxy_add_object_listener(
                proxy,
                hook.as_mut().get_unchecked_mut(),
                events.as_ref().get_ref() as *const _ as *const _,
                (&mut *callback as *mut Rc<RefCell<Vec<Vec<DriverRow>>>>).cast(),
            );
        }
        Ok(Self { hook, _events: events, _callback: callback, proxy })
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        spa::utils::hook::remove(*self.hook);
        unsafe {
            pw::sys::pw_proxy_destroy(self.proxy);
        }
    }
}
unsafe extern "C" fn on_profile(data: *mut std::ffi::c_void, raw: *const spa::sys::spa_pod) {
    if raw.is_null() {
        return;
    }
    // SAFETY: PipeWire invokes this callback synchronously with the pod and callback alive.
    let rows = unsafe { &*(data as *const Rc<RefCell<Vec<Vec<DriverRow>>>>) };
    let pod = unsafe { Pod::from_raw(raw) };
    let parsed = parse(pod);
    if !parsed.is_empty() {
        rows.borrow_mut().push(parsed);
    }
}

fn integer(pod: &Pod) -> Option<u64> {
    pod.get_int().ok().map(|n| n.max(0) as u64).or_else(|| pod.get_long().ok().map(|n| n.max(0) as u64))
}
fn stamp(pod: &Pod) -> Option<i64> {
    pod.get_long().ok()
}
fn block(pod: &Pod, driver: bool, rate: u32, quantum: u32, fallback_errors: u64) -> Option<DriverRow> {
    let fields: Vec<_> = pod.as_struct().ok()?.fields().collect();
    let id = integer(fields.first()?)? as u32;
    let node = fields.get(1)?.get_string_raw().ok()??.to_string_lossy().into_owned();
    let signal = stamp(fields.get(3)?)?;
    let awake = stamp(fields.get(4)?)?;
    let finish = stamp(fields.get(5)?)?;
    let busy = finish.saturating_sub(awake).max(0) as f64 / 1_000_000.;
    let wait = awake.saturating_sub(signal).max(0) as f64 / 1_000_000.;
    let xruns = fields.get(8).and_then(|pod| integer(pod)).unwrap_or(0);
    Some(DriverRow { id, node, driver, rate, quantum, busy, wait, errors: fallback_errors, xruns })
}

pub fn parse(pod: &Pod) -> Vec<DriverRow> {
    let Ok(pod) = pod.as_struct() else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for item in pod.fields() {
        let Ok(object) = item.as_object() else {
            continue;
        };
        let mut rate = 0;
        let mut quantum = 0;
        let mut errors = 0;
        // The clock and info precede driver/follower blocks in a profiler point.
        for prop in object.props() {
            match prop.key().0 {
                spa::sys::SPA_PROFILER_info => {
                    errors =
                        prop.value().as_struct().ok().and_then(|s| s.fields().nth(4)).and_then(integer).unwrap_or(0);
                }
                spa::sys::SPA_PROFILER_clock => {
                    if let Ok(clock) = prop.value().as_struct() {
                        let mut fields = clock.fields();
                        let fraction = fields.nth(4).and_then(|p| p.get_fraction().ok());
                        let duration = fields.nth(1).and_then(stamp);
                        if let Some(frac) = fraction.filter(|f| f.num > 0) {
                            rate = frac.denom / frac.num;
                        }
                        quantum = duration.unwrap_or(0).max(0).min(u32::MAX as i64) as u32;
                    }
                }
                spa::sys::SPA_PROFILER_driverBlock => {
                    if let Some(row) = block(prop.value(), true, rate, quantum, errors) {
                        rows.push(row);
                    }
                }
                spa::sys::SPA_PROFILER_followerBlock => {
                    if let Some(row) = block(prop.value(), false, rate, quantum, errors) {
                        rows.push(row);
                    }
                }
                _ => {}
            }
        }
    }
    rows
}
