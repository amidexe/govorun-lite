use crate::features::dto::dto;
dto! { AccountProvider { id: String, enabled: bool } }
dto! { AccountDevice {
    id: String, name: String, subtitle: String, confirmation: String, current: bool
} }
dto! { AccountStorage { available: bool, state: String } }
dto! { AccountSnapshot {
    scope: String, status: String, display_name: Option<String>, message_code: Option<String>,
    busy: bool, sync_mode: String, providers: Vec<AccountProvider>, provider_notice_code: Option<String>,
    devices: Vec<AccountDevice>, device_limit: u32, replacing_device: bool, storage: AccountStorage
} }
