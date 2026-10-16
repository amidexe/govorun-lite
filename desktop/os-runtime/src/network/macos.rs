use super::{Handler, State};
use block2::RcBlock;
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSWorkspace, NSWorkspaceDidWakeNotification};
use objc2_foundation::{NSNotification, NSObjectProtocol};
use std::{ffi::c_void, sync::Arc};
#[link(name = "Network", kind = "framework")]
extern "C" {
    fn nw_path_monitor_create() -> *mut c_void;
    fn nw_path_monitor_set_update_handler(monitor: *mut c_void, block: *const c_void);
    fn nw_path_monitor_set_queue(monitor: *mut c_void, queue: *mut c_void);
    fn nw_path_monitor_start(monitor: *mut c_void);
    fn nw_path_monitor_cancel(monitor: *mut c_void);
    fn nw_path_get_status(path: *mut c_void) -> i32;
    fn nw_release(object: *mut c_void);
}
extern "C" {
    fn dispatch_queue_create(label: *const i8, attributes: *mut c_void) -> *mut c_void;
    fn dispatch_release(object: *mut c_void);
}
pub struct PathMonitor {
    monitor: usize,
    queue: usize,
    status: Arc<State>,
    wake_observer: usize,
}
impl PathMonitor {
    pub fn new() -> Result<Self, &'static str> {
        let monitor = unsafe { nw_path_monitor_create() };
        if monitor.is_null() {
            return Err("network_observation_unavailable");
        }
        let queue = unsafe {
            dispatch_queue_create(c"com.govorun.rust-path".as_ptr(), std::ptr::null_mut())
        };
        if queue.is_null() {
            unsafe {
                nw_release(monitor);
            }
            return Err("network_observation_unavailable");
        }
        let status = Arc::new(State::default());
        let observed = status.clone();
        let callback = RcBlock::new(move |path: *mut c_void| {
            let value = unsafe { nw_path_get_status(path) };
            observed.update(value);
        });
        unsafe {
            nw_path_monitor_set_update_handler(monitor, &*callback as *const _ as *const c_void);
            nw_path_monitor_set_queue(monitor, queue);
            nw_path_monitor_start(monitor);
        }
        let wake_state = status.clone();
        let wake = RcBlock::new(move |_: std::ptr::NonNull<NSNotification>| wake_state.wake());
        let observer = unsafe {
            NSWorkspace::sharedWorkspace()
                .notificationCenter()
                .addObserverForName_object_queue_usingBlock(
                    Some(NSWorkspaceDidWakeNotification),
                    None,
                    None,
                    &wake,
                )
        };
        Ok(Self {
            monitor: monitor as usize,
            queue: queue as usize,
            status,
            wake_observer: Retained::into_raw(observer) as usize,
        })
    }
    pub fn available(&self) -> Option<bool> {
        self.status.available()
    }
    pub fn bind(&self, handler: Handler) -> Result<(), &'static str> {
        self.status.bind(handler)
    }
}
impl Drop for PathMonitor {
    fn drop(&mut self) {
        self.status.close();
        unsafe {
            let observer = Retained::<ProtocolObject<dyn NSObjectProtocol>>::from_raw(
                self.wake_observer as *mut _,
            )
            .expect("retained wake token");
            NSWorkspace::sharedWorkspace()
                .notificationCenter()
                .removeObserver((&*observer).as_ref());
            nw_path_monitor_cancel(self.monitor as *mut c_void);
            nw_release(self.monitor as *mut c_void);
            dispatch_release(self.queue as *mut c_void);
        }
    }
}
