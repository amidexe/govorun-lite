//! The hook blocks key-up only for a key-down this owner blocked. Modifier
//! changes and repeat cannot swallow unrelated releases or leave a key stuck.
use super::{Action, Binding, Event, Platform, Tracker};
use handy_keys::{Key, KeyEvent};
use std::collections::HashSet;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Primary {
    Key(Key),
    Modifier(u32),
}
fn primary(raw: &KeyEvent) -> Option<Primary> {
    raw.key.map(Primary::Key).or_else(|| {
        raw.changed_modifier
            .map(|side| Primary::Modifier(side.bits()))
    })
}
pub struct Packet {
    pub event: Option<Event>,
    pub block: bool,
}
pub struct Pairing {
    tracker: Tracker,
    held: HashSet<Primary>,
    last_nanos: u64,
}
impl Pairing {
    pub fn new(platform: Platform, bindings: Vec<(super::Target, Binding)>) -> Self {
        Self {
            tracker: Tracker::new(platform, bindings),
            held: HashSet::new(),
            last_nanos: 0,
        }
    }
    pub fn process(&mut self, raw: KeyEvent, at_nanos: u64, cancel_enabled: bool) -> Packet {
        if at_nanos < self.last_nanos {
            return Packet {
                event: None,
                block: false,
            };
        }
        self.last_nanos = at_nanos;
        let primary = primary(&raw);
        let paired = primary.is_some_and(|key| self.held.contains(&key));
        let event = self.tracker.process(raw, at_nanos, cancel_enabled);
        let accepted = raw.is_key_down && event.is_some_and(|event| event.action == Action::Press);
        if let Some(key) = primary {
            if accepted {
                self.held.insert(key);
            } else if !raw.is_key_down {
                self.held.remove(&key);
            }
        }
        Packet {
            event,
            block: accepted || paired,
        }
    }
    /// The OS hook must clear its physical pairing in the same reconfiguration
    /// operation. A queued old key-up cannot block or finish a new gesture.
    pub fn replace(
        &mut self,
        bindings: Vec<(super::Target, Binding)>,
        at_nanos: u64,
    ) -> Option<Event> {
        self.held.clear();
        self.last_nanos = self.last_nanos.max(at_nanos);
        self.tracker.replace(bindings, at_nanos)
    }
    pub fn reset(&mut self, at_nanos: u64) -> Option<Event> {
        self.held.clear();
        self.last_nanos = self.last_nanos.max(at_nanos);
        self.tracker.reset(at_nanos)
    }
}
