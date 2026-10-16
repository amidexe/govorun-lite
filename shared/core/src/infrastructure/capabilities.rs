//! Bounded command names shared by feature admission and transport adapters.
use serde_json::{Map, Value};
use std::sync::OnceLock;
fn commands() -> &'static Map<String, Value> {
    static COMMANDS: OnceLock<Map<String, Value>> = OnceLock::new();
    COMMANDS.get_or_init(|| {
        serde_json::from_str(include_str!("../../contracts/agent-capabilities.json"))
            .expect("bundled agent capability names")
    })
}

pub fn supported(method: &str) -> bool {
    commands().contains_key(method)
}
pub fn agent_capability_count() -> usize {
    commands().len()
}

pub fn renderer_command_allowed(method: &str) -> bool {
    static RENDERER: OnceLock<Map<String, Value>> = OnceLock::new();
    RENDERER
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../contracts/renderer-commands.json"))
                .expect("bundled renderer command names")
        })
        .contains_key(method)
}
