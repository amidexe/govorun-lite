//! Content-free capture startup measurements. OS adapters supply monotonic milliseconds.
#[derive(Default)]
pub struct Startup {
    started: u64,
    opening: u64,
    opened: u64,
    first_packet: Option<u64>,
}
impl Startup {
    pub fn new(started: u64) -> Self {
        Self {
            started,
            opening: started,
            opened: started,
            first_packet: None,
        }
    }
    pub fn input_opening(&mut self, at: u64) {
        self.opening = at;
    }
    pub fn input_opened(&mut self, at: u64) {
        self.opened = at;
    }
    pub fn first_packet(&mut self, at: u64) {
        self.first_packet.get_or_insert(at);
    }
    pub fn has_first_packet(&self) -> bool {
        self.first_packet.is_some()
    }
    pub fn detail(&self, now: u64) -> String {
        let Some(first) = self.first_packet else {
            return String::new();
        };
        format!(
            "open_ms={};prepare_wait_ms={};device_open_ms={};first_packet_ms={}",
            now.saturating_sub(self.started),
            self.opening.saturating_sub(self.started),
            self.opened.saturating_sub(self.opening),
            first.saturating_sub(self.started)
        )
    }
}
/// Exact four-field wire shape; paths, device identifiers and text cannot enter it.
pub fn valid_detail(detail: &str) -> bool {
    let keys = [
        "open_ms",
        "prepare_wait_ms",
        "device_open_ms",
        "first_packet_ms",
    ];
    let parts: Vec<_> = detail.split(';').collect();
    parts.len() == keys.len()
        && parts.iter().zip(keys).all(|(part, key)| {
            let Some((name, value)) = part.split_once('=') else {
                return false;
            };
            name == key
                && !value.is_empty()
                && value.len() <= 20
                && value.bytes().all(|b| b.is_ascii_digit())
                && value.parse::<u64>().is_ok()
        })
}
