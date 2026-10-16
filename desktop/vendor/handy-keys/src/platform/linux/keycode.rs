//! Linux key conversion utilities for evdev key codes
//!
//! Maps kernel input event codes (`include/uapi/linux/input-event-codes.h`,
//! exposed as `evdev::KeyCode` constants) to the cross-platform `Key` and
//! `Modifiers` types. Codes are layout-independent: they describe the
//! physical key position, matching how the macOS and Windows backends map
//! virtual keycodes.

use evdev::KeyCode;

use crate::types::{Key, Modifiers};

/// Convert an evdev key code to our Key type
pub fn key_code_to_key(code: KeyCode) -> Option<Key> {
    match code {
        // Letters
        KeyCode::KEY_A => Some(Key::A),
        KeyCode::KEY_B => Some(Key::B),
        KeyCode::KEY_C => Some(Key::C),
        KeyCode::KEY_D => Some(Key::D),
        KeyCode::KEY_E => Some(Key::E),
        KeyCode::KEY_F => Some(Key::F),
        KeyCode::KEY_G => Some(Key::G),
        KeyCode::KEY_H => Some(Key::H),
        KeyCode::KEY_I => Some(Key::I),
        KeyCode::KEY_J => Some(Key::J),
        KeyCode::KEY_K => Some(Key::K),
        KeyCode::KEY_L => Some(Key::L),
        KeyCode::KEY_M => Some(Key::M),
        KeyCode::KEY_N => Some(Key::N),
        KeyCode::KEY_O => Some(Key::O),
        KeyCode::KEY_P => Some(Key::P),
        KeyCode::KEY_Q => Some(Key::Q),
        KeyCode::KEY_R => Some(Key::R),
        KeyCode::KEY_S => Some(Key::S),
        KeyCode::KEY_T => Some(Key::T),
        KeyCode::KEY_U => Some(Key::U),
        KeyCode::KEY_V => Some(Key::V),
        KeyCode::KEY_W => Some(Key::W),
        KeyCode::KEY_X => Some(Key::X),
        KeyCode::KEY_Y => Some(Key::Y),
        KeyCode::KEY_Z => Some(Key::Z),

        // Numbers
        KeyCode::KEY_0 => Some(Key::Num0),
        KeyCode::KEY_1 => Some(Key::Num1),
        KeyCode::KEY_2 => Some(Key::Num2),
        KeyCode::KEY_3 => Some(Key::Num3),
        KeyCode::KEY_4 => Some(Key::Num4),
        KeyCode::KEY_5 => Some(Key::Num5),
        KeyCode::KEY_6 => Some(Key::Num6),
        KeyCode::KEY_7 => Some(Key::Num7),
        KeyCode::KEY_8 => Some(Key::Num8),
        KeyCode::KEY_9 => Some(Key::Num9),

        // Function keys. F13-F24 (codes 183-194) matter for dictation
        // hotkeys: foot pedals and macro pads emit them.
        KeyCode::KEY_F1 => Some(Key::F1),
        KeyCode::KEY_F2 => Some(Key::F2),
        KeyCode::KEY_F3 => Some(Key::F3),
        KeyCode::KEY_F4 => Some(Key::F4),
        KeyCode::KEY_F5 => Some(Key::F5),
        KeyCode::KEY_F6 => Some(Key::F6),
        KeyCode::KEY_F7 => Some(Key::F7),
        KeyCode::KEY_F8 => Some(Key::F8),
        KeyCode::KEY_F9 => Some(Key::F9),
        KeyCode::KEY_F10 => Some(Key::F10),
        KeyCode::KEY_F11 => Some(Key::F11),
        KeyCode::KEY_F12 => Some(Key::F12),
        KeyCode::KEY_F13 => Some(Key::F13),
        KeyCode::KEY_F14 => Some(Key::F14),
        KeyCode::KEY_F15 => Some(Key::F15),
        KeyCode::KEY_F16 => Some(Key::F16),
        KeyCode::KEY_F17 => Some(Key::F17),
        KeyCode::KEY_F18 => Some(Key::F18),
        KeyCode::KEY_F19 => Some(Key::F19),
        KeyCode::KEY_F20 => Some(Key::F20),
        KeyCode::KEY_F21 => Some(Key::F21),
        KeyCode::KEY_F22 => Some(Key::F22),
        KeyCode::KEY_F23 => Some(Key::F23),
        KeyCode::KEY_F24 => Some(Key::F24),

        // Special keys
        KeyCode::KEY_SPACE => Some(Key::Space),
        KeyCode::KEY_ENTER => Some(Key::Return),
        KeyCode::KEY_TAB => Some(Key::Tab),
        KeyCode::KEY_ESC => Some(Key::Escape),
        KeyCode::KEY_BACKSPACE => Some(Key::Delete),
        KeyCode::KEY_DELETE => Some(Key::ForwardDelete),
        KeyCode::KEY_INSERT => Some(Key::Insert),
        // Pause/Break. USB keyboards report real press/release through
        // evdev; PS/2 (atkbd) delivers the release with the press because
        // the hardware break sequence is part of the make sequence.
        KeyCode::KEY_PAUSE => Some(Key::Pause),
        KeyCode::KEY_HOME => Some(Key::Home),
        KeyCode::KEY_END => Some(Key::End),
        KeyCode::KEY_PAGEUP => Some(Key::PageUp),
        KeyCode::KEY_PAGEDOWN => Some(Key::PageDown),

        // Arrow keys
        KeyCode::KEY_LEFT => Some(Key::LeftArrow),
        KeyCode::KEY_RIGHT => Some(Key::RightArrow),
        KeyCode::KEY_UP => Some(Key::UpArrow),
        KeyCode::KEY_DOWN => Some(Key::DownArrow),

        // Punctuation and symbols
        KeyCode::KEY_MINUS => Some(Key::Minus),
        KeyCode::KEY_EQUAL => Some(Key::Equal),
        KeyCode::KEY_LEFTBRACE => Some(Key::LeftBracket),
        KeyCode::KEY_RIGHTBRACE => Some(Key::RightBracket),
        KeyCode::KEY_BACKSLASH => Some(Key::Backslash),
        KeyCode::KEY_SEMICOLON => Some(Key::Semicolon),
        KeyCode::KEY_APOSTROPHE => Some(Key::Quote),
        KeyCode::KEY_COMMA => Some(Key::Comma),
        KeyCode::KEY_DOT => Some(Key::Period),
        KeyCode::KEY_SLASH => Some(Key::Slash),
        KeyCode::KEY_GRAVE => Some(Key::Grave),
        // The extra key on ISO layouts next to left shift; macOS reports
        // the same physical key as Section on its ISO keyboards.
        KeyCode::KEY_102ND => Some(Key::Section),

        // JIS keyboard keys
        KeyCode::KEY_YEN => Some(Key::JisYen),
        KeyCode::KEY_RO => Some(Key::JisUnderscore),

        // Keypad
        KeyCode::KEY_KP0 => Some(Key::Keypad0),
        KeyCode::KEY_KP1 => Some(Key::Keypad1),
        KeyCode::KEY_KP2 => Some(Key::Keypad2),
        KeyCode::KEY_KP3 => Some(Key::Keypad3),
        KeyCode::KEY_KP4 => Some(Key::Keypad4),
        KeyCode::KEY_KP5 => Some(Key::Keypad5),
        KeyCode::KEY_KP6 => Some(Key::Keypad6),
        KeyCode::KEY_KP7 => Some(Key::Keypad7),
        KeyCode::KEY_KP8 => Some(Key::Keypad8),
        KeyCode::KEY_KP9 => Some(Key::Keypad9),
        KeyCode::KEY_KPDOT => Some(Key::KeypadDecimal),
        KeyCode::KEY_KPASTERISK => Some(Key::KeypadMultiply),
        KeyCode::KEY_KPPLUS => Some(Key::KeypadPlus),
        KeyCode::KEY_KPSLASH => Some(Key::KeypadDivide),
        KeyCode::KEY_KPENTER => Some(Key::KeypadEnter),
        KeyCode::KEY_KPMINUS => Some(Key::KeypadMinus),
        KeyCode::KEY_KPEQUAL => Some(Key::KeypadEquals),
        KeyCode::KEY_KPCOMMA | KeyCode::KEY_KPJPCOMMA => Some(Key::KeypadComma),

        // Lock keys
        KeyCode::KEY_CAPSLOCK => Some(Key::CapsLock),
        KeyCode::KEY_SCROLLLOCK => Some(Key::ScrollLock),
        KeyCode::KEY_NUMLOCK => Some(Key::NumLock),

        // PrintScreen (the kernel names its event code after SysRq)
        KeyCode::KEY_SYSRQ => Some(Key::PrintScreen),

        // Context-menu (Application) key
        KeyCode::KEY_COMPOSE => Some(Key::ContextMenu),

        // Mouse buttons
        KeyCode::BTN_LEFT => Some(Key::MouseLeft),
        KeyCode::BTN_RIGHT => Some(Key::MouseRight),
        KeyCode::BTN_MIDDLE => Some(Key::MouseMiddle),
        KeyCode::BTN_SIDE => Some(Key::MouseX1),
        KeyCode::BTN_EXTRA => Some(Key::MouseX2),

        _ => None,
    }
}

/// Convert an evdev modifier key to our side-specific Modifiers type
pub fn key_code_to_modifier(code: KeyCode) -> Option<Modifiers> {
    match code {
        KeyCode::KEY_LEFTSHIFT => Some(Modifiers::SHIFT_LEFT),
        KeyCode::KEY_RIGHTSHIFT => Some(Modifiers::SHIFT_RIGHT),
        KeyCode::KEY_LEFTCTRL => Some(Modifiers::CTRL_LEFT),
        KeyCode::KEY_RIGHTCTRL => Some(Modifiers::CTRL_RIGHT),
        KeyCode::KEY_LEFTALT => Some(Modifiers::OPT_LEFT),
        KeyCode::KEY_RIGHTALT => Some(Modifiers::OPT_RIGHT),
        KeyCode::KEY_LEFTMETA => Some(Modifiers::CMD_LEFT),
        KeyCode::KEY_RIGHTMETA => Some(Modifiers::CMD_RIGHT),
        _ => None,
    }
}

