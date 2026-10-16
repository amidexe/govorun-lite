//! OS/network composition only; feature lifetimes and decisions belong to Core.
pub use govorun_core::account_host::*;
use std::sync::Arc;
pub fn new() -> Arc<AccountHost> { with_effects(Arc::new(|_, _| {}), Arc::new(crate::features::about::open::open)) }
pub fn with_effects(events: crate::features::billing::host::Events, open: crate::features::billing::host::Opener) -> Arc<AccountHost> {
    let owner = AccountHost::with_effects(events, open);
    owner.attach_network(NetworkPorts {
        platform: if cfg!(windows) { "windows" } else { "macos" },
        format: if cfg!(windows) { govorun_core::style_pass_vault::format::Format::FlatV4 } else { govorun_core::style_pass_vault::format::Format::LedgerV5 },
        account: Arc::new(|origin| Ok(Arc::new(govorun_core_http::HttpTransport::new(origin, env!("CARGO_PKG_VERSION")).map_err(|error| error.code())?))),
        gateway: Arc::new(|| Ok(Arc::new(govorun_core_http::stylization::Client::new(govorun_core::stylization::configured_origin(), if cfg!(windows) { "Govorun-Windows/2" } else { "Govorun-macOS/2" }).and_then(|client|client.with_version(env!("CARGO_PKG_VERSION"))).map_err(|error| error.code())?))),
    }).expect("new owner receives network ports once");
    owner
}
