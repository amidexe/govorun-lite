//! Pair only accepted presses; a modifier release cannot release a held chord
//! key. No keyboard characters or translated labels leave this owner.
use super::{Binding, Platform};
use handy_keys::KeyEvent;

pub use govorun_core::features::dictation::input_events::{Action, Event, Target};
struct Held {
    target: Target,
    binding: Binding,
    at_nanos: u64,
}
pub struct Tracker {
    platform: Platform,
    bindings: Vec<(Target, Binding)>,
    held: Option<Held>,
    cancel_down: bool,
    modifier_released_at: Option<u64>,
    storm_started: Option<u64>,
    storm_count: u8,
    disabled_until: u64,
}
impl Tracker {
    pub fn new(platform: Platform, bindings: Vec<(Target, Binding)>) -> Self {
        Self {
            platform,
            bindings,
            held: None,
            cancel_down: false,
            modifier_released_at: None,
            storm_started: None,
            storm_count: 0,
            disabled_until: 0,
        }
    }
    pub fn reset(&mut self, at_nanos: u64) -> Option<Event> {
        self.cancel_down = false;
        self.modifier_released_at = None;
        self.held.take().map(|held| Event {
            target: held.target,
            action: Action::Abort,
            at_nanos,
        })
    }
    pub fn replace(&mut self, bindings: Vec<(Target, Binding)>, at_nanos: u64) -> Option<Event> {
        let event = self.reset(at_nanos);
        self.bindings = bindings;
        event
    }
    pub fn process(&mut self, raw: KeyEvent, at_nanos: u64, cancel_enabled: bool) -> Option<Event> {
        // Cancellation takes priority over modifier/chord interruption.
        for &(target, binding) in &self.bindings {
            if target != Target::Cancel {
                continue;
            }
            let primary = if let Some(key) = binding.key {
                raw.key == Some(key)
            } else {
                raw.key.is_none() && raw.changed_modifier == binding.side
            };
            if !raw.is_key_down && primary {
                self.cancel_down = false;
            }
            if raw.is_key_down
                && primary
                && binding.matches(raw.modifiers)
                && cancel_enabled
                && !self.cancel_down
            {
                self.cancel_down = true;
                return Some(Event {
                    target,
                    action: Action::Press,
                    at_nanos,
                });
            }
        }
        if let Some(held) = self.held.as_ref() {
            if at_nanos < held.at_nanos {
                return None;
            }
            let released = !raw.is_key_down
                && if let Some(key) = held.binding.key {
                    raw.key == Some(key)
                } else {
                    raw.changed_modifier == held.binding.side
                        && !raw.modifiers.intersects(held.binding.side.unwrap())
                };
            let abort = self.platform == Platform::MacOS
                && held.binding.key.is_none()
                && at_nanos.saturating_sub(held.at_nanos) <= 1_000_000_000
                && ((raw.is_key_down && raw.key.is_some()) || !held.binding.matches(raw.modifiers));
            if released || abort {
                let held = self.held.take().unwrap();
                if released && held.binding.key.is_none() {
                    self.modifier_released_at = Some(at_nanos);
                }
                return Some(Event {
                    target: held.target,
                    action: if released {
                        Action::Release
                    } else {
                        Action::Abort
                    },
                    at_nanos,
                });
            }
        }
        for &(target, binding) in &self.bindings {
            let primary = if let Some(key) = binding.key {
                raw.key == Some(key)
            } else {
                raw.key.is_none() && raw.changed_modifier == binding.side
            };
            if target == Target::Cancel
                || !raw.is_key_down
                || !primary
                || !binding.matches(raw.modifiers)
            {
                continue;
            }
            if self.held.is_some() {
                continue;
            }
            if self.platform == Platform::MacOS
                && binding.key.is_none()
                && self
                    .modifier_released_at
                    .is_some_and(|end| at_nanos.saturating_sub(end) < 120_000_000)
            {
                continue;
            }
            if self.platform == Platform::MacOS && binding.key.is_none() {
                if at_nanos < self.disabled_until {
                    continue;
                }
                if self
                    .storm_started
                    .is_none_or(|start| at_nanos.saturating_sub(start) > 2_000_000_000)
                {
                    self.storm_started = Some(at_nanos);
                    self.storm_count = 1;
                } else {
                    self.storm_count = self.storm_count.saturating_add(1);
                }
                if self.storm_count > 6 {
                    self.disabled_until = at_nanos.saturating_add(5_000_000_000);
                    self.storm_started = None;
                    self.storm_count = 0;
                    continue;
                }
            }
            self.held = Some(Held {
                target,
                binding,
                at_nanos,
            });
            return Some(Event {
                target,
                action: Action::Press,
                at_nanos,
            });
        }
        None
    }
}
