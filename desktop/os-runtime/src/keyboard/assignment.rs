//! Focused physical assignment capture. It emits OS codes, never characters.
//! Conflict, reserved shortcuts, persistence and expiry stay in the host/Core.
use super::Platform;
use handy_keys::{KeyEvent, Modifiers};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub key: u16,
    pub modifiers: u8,
    pub modifier_only: bool,
    pub control_side: u8,
}
#[derive(Clone, Debug)]
pub struct CaptureEvent {
    pub id: String,
    pub sequence: u64,
    pub preview: Option<Candidate>,
    pub gesture: Option<Candidate>,
    pub cancelled: bool,
}
pub struct Packet {
    pub event: Option<CaptureEvent>,
    pub block: bool,
    pub exclusive: bool,
}
struct Attempt {
    id: String,
    sequence: u64,
    down: HashSet<u16>,
    pending: Option<Candidate>,
    primary: Option<Candidate>,
    combination: bool,
}
impl Attempt {
    fn event(
        &mut self,
        preview: Option<Candidate>,
        gesture: Option<Candidate>,
        cancelled: bool,
    ) -> CaptureEvent {
        self.sequence += 1;
        CaptureEvent {
            id: self.id.clone(),
            sequence: self.sequence,
            preview,
            gesture,
            cancelled,
        }
    }
}
pub struct Capture {
    platform: Platform,
    active: Option<Attempt>,
    // Survives cancel/save/replacement until each matching physical release.
    held: HashSet<u16>,
}
pub fn mask(modifiers: Modifiers) -> u8 {
    [
        (1, Modifiers::CTRL),
        (2, Modifiers::SHIFT),
        (4, Modifiers::OPT),
        (8, Modifiers::CMD),
    ]
    .into_iter()
    .fold(0, |value, (bit, group)| {
        if modifiers.intersects(group) {
            value | bit
        } else {
            value
        }
    })
}
impl Capture {
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            active: None,
            held: HashSet::new(),
        }
    }
    pub fn active(&self) -> bool {
        self.active.is_some()
    }
    pub fn owns(&self, key: u16) -> bool {
        self.held.contains(&key)
    }
    pub fn start(&mut self, id: String) -> Result<(), &'static str> {
        if id.is_empty() || id.len() > 64 || id.chars().any(char::is_control) {
            return Err("hotkeys_invalid_capture");
        }
        self.active = Some(Attempt {
            id,
            sequence: 0,
            down: HashSet::new(),
            pending: None,
            primary: None,
            combination: false,
        });
        Ok(())
    }
    pub fn stop(&mut self, id: &str) {
        if self.active.as_ref().is_some_and(|value| value.id == id) {
            self.active = None;
        }
    }
    pub fn cancel(&mut self) -> Option<CaptureEvent> {
        self.active
            .take()
            .map(|mut value| value.event(None, None, true))
    }
    pub fn recover(&mut self) -> Option<CaptureEvent> {
        let event = self.cancel();
        self.held.clear();
        event
    }
    pub fn process(&mut self, code: u16, raw: KeyEvent, allowed: bool) -> Packet {
        let paired = self.held.contains(&code);
        let Some(active) = self.active.as_mut() else {
            if !raw.is_key_down {
                self.held.remove(&code);
            }
            return Packet {
                event: None,
                block: paired,
                exclusive: paired,
            };
        };
        if !allowed {
            let event = self.cancel();
            if !raw.is_key_down {
                self.held.remove(&code);
            }
            return Packet {
                event,
                block: paired,
                exclusive: true,
            };
        }
        if raw.is_key_down {
            // A key held across capture replacement may drain, never nominate.
            if paired {
                return Packet {
                    event: None,
                    block: true,
                    exclusive: true,
                };
            }
            self.held.insert(code);
            active.down.insert(code);
        } else {
            if !self.held.remove(&code) {
                return Packet {
                    event: None,
                    block: false,
                    exclusive: true,
                };
            }
            if !active.down.remove(&code) {
                return Packet {
                    event: None,
                    block: true,
                    exclusive: true,
                };
            }
        }
        let mask = mask(raw.modifiers);
        let escape = if self.platform == Platform::MacOS {
            code == 53 && mask == 0
        } else {
            code == 0x1B
        };
        if raw.is_key_down && escape {
            return Packet {
                event: self.cancel(),
                block: true,
                exclusive: true,
            };
        }
        let side = if self.platform == Platform::Windows {
            if raw.modifiers.intersects(Modifiers::CTRL_LEFT) {
                1
            } else if raw.modifiers.intersects(Modifiers::CTRL_RIGHT) {
                2
            } else {
                0
            }
        } else {
            0
        };
        let modifier = raw.changed_modifier.is_some();
        let mut gesture = None;
        let preview;
        if raw.is_key_down && !modifier {
            let candidate = Candidate {
                key: code,
                modifiers: mask,
                modifier_only: false,
                control_side: side,
            };
            active.combination = true;
            active.pending = None;
            preview = Some(candidate);
            if self.platform == Platform::MacOS {
                gesture = Some(candidate);
            } else {
                active.primary = Some(candidate);
            }
        } else if modifier {
            if raw.is_key_down {
                let candidate = Candidate {
                    key: code,
                    modifiers: mask,
                    modifier_only: true,
                    control_side: 0,
                };
                if self.platform == Platform::MacOS {
                    let peak = active.pending.map_or(mask, |value| value.modifiers | mask);
                    active.pending = Some(Candidate {
                        modifiers: peak,
                        ..candidate
                    });
                } else if active.pending.is_none() {
                    active.pending = Some(candidate);
                }
            } else if !active.combination {
                let complete = if self.platform == Platform::MacOS {
                    mask == 0
                } else {
                    active.pending.is_some_and(|value| value.key == code)
                };
                if complete {
                    gesture = active.pending.take();
                }
            }
            preview = (mask != 0).then_some(Candidate {
                key: 0,
                modifiers: mask,
                modifier_only: true,
                control_side: side,
            });
        } else {
            if active.primary.is_some_and(|value| value.key == code) {
                gesture = active.primary.take();
            }
            preview = gesture;
        }
        if active.down.is_empty() {
            active.pending = None;
            active.combination = false;
        }
        let event = Some(active.event(preview, gesture, false));
        Packet {
            event,
            block: true,
            exclusive: true,
        }
    }
}
