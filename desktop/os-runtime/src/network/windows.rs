use super::{Handler, State};
use std::sync::Arc;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::NetworkManagement::{IpHelper::*, Ndis::*};
use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;
pub struct PathMonitor {
    handle: usize,
    context: usize,
    state: Arc<State>,
}
unsafe extern "system" fn changed(
    context: *const std::ffi::c_void,
    _: *const MIB_IPINTERFACE_ROW,
    _: MIB_NOTIFICATION_TYPE,
) {
    let state = &*context.cast::<State>();
    if let Some(online) = available() {
        state.update(if online { 1 } else { 2 });
    }
}
impl PathMonitor {
    pub fn new() -> Result<Self, &'static str> {
        let state = Arc::new(State::default());
        if let Some(online) = available() {
            state.update(if online { 1 } else { 2 });
        }
        let context = Arc::into_raw(state.clone());
        let mut handle: HANDLE = std::ptr::null_mut();
        let result = unsafe {
            NotifyIpInterfaceChange(AF_UNSPEC, Some(changed), context.cast(), true, &mut handle)
        };
        if result != 0 {
            unsafe {
                drop(Arc::from_raw(context));
            }
            return Err("network_observation_unavailable");
        }
        Ok(Self {
            handle: handle as usize,
            context: context as usize,
            state,
        })
    }
    pub fn bind(&self, handler: Handler) -> Result<(), &'static str> {
        self.state.bind(handler)
    }
    pub fn available(&self) -> Option<bool> {
        available()
    }
}
impl Drop for PathMonitor {
    fn drop(&mut self) {
        self.state.close();
        // Cancellation waits for the serialized callback; never cancel from it.
        // On an unexpected cancellation error retain the context to avoid UAF.
        if unsafe { CancelMibChangeNotify2(self.handle as HANDLE) } == 0 {
            unsafe {
                drop(Arc::from_raw(self.context as *const State));
            }
        }
    }
}
fn available() -> Option<bool> {
    let mut bytes = 16384u32;
    for _ in 0..3 {
        let mut storage = vec![0u64; (bytes as usize + 7) / 8];
        let first = storage.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let result = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER,
                std::ptr::null(),
                first,
                &mut bytes,
            )
        };
        if result == 111 {
            if bytes > 1024 * 1024 {
                return None;
            }
            continue;
        }
        if result == 232 {
            return Some(false);
        }
        if result != 0 {
            return None;
        }
        let mut next = first;
        while !next.is_null() {
            let adapter = unsafe { &*next };
            if adapter.OperStatus == IfOperStatusUp && !matches!(adapter.IfType, 24 | 131) {
                return Some(true);
            }
            next = adapter.Next;
        }
        return Some(false);
    }
    None
}
