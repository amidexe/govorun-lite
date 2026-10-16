//! Keyboard key and mouse button definitions and parsing

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};

/// Keyboard keys and mouse buttons that can be used in hotkey combinations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Key {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    // Media keys
    PlayPause,
    Stop,
    PrevTrack,
    NextTrack,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    /// F21–F24 exist on Windows (VK 0x84–0x87) and in evdev; macOS virtual
    /// keycodes stop at F20, so these are unreachable from macOS hardware.
    F21,
    F22,
    F23,
    F24,

    // Special keys
    Space,
    Return,
    Tab,
    Escape,
    Delete,
    ForwardDelete,
    Insert,
    /// The Pause/Break key.
    ///
    /// Windows delivers this key's release together with its press: the
    /// hardware break sequence is part of the make sequence (`E1 1D 45 E1
    /// 9D C5`, nothing on release — PS/2 behavior the HID driver preserves
    /// for USB keyboards), so the hook sees key-down and key-up back to
    /// back at press time. Treat it as tap/toggle only on Windows; a hold
    /// binding would release immediately. Ctrl+Pause reports `VK_CANCEL`
    /// rather than `VK_PAUSE`; both map here. Linux evdev reports normal
    /// press/release for USB keyboards, so holds work there. macOS has no
    /// virtual keycode for Pause — PC keyboards' Pause key arrives as F15.
    Pause,
    Home,
    End,
    PageUp,
    PageDown,

    // Arrow keys
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,

    // Punctuation and symbols
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Grave,
    Section,
    // JIS keyboard keys
    JisYen,
    JisUnderscore,
    JisEisu,
    JisKana,

    // Keypad
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    KeypadDecimal,
    KeypadMultiply,
    KeypadPlus,
    KeypadClear,
    KeypadDivide,
    KeypadEnter,
    KeypadMinus,
    KeypadEquals,
    KeypadComma,

    // Lock keys
    CapsLock,
    ScrollLock,
    NumLock,

    /// The PrintScreen/SysRq key. Windows and Linux only: macOS has no
    /// virtual keycode for it — PC keyboards' PrintScreen arrives as F13
    /// there.
    PrintScreen,

    /// The context-menu (Application) key, right of AltGr on full-size PC
    /// keyboards. `VK_APPS` on Windows, `KEY_COMPOSE` on Linux evdev;
    /// macOS reports PC keyboards' Menu key as keycode `0x6E`.
    ContextMenu,

    /// The dictation / voice key (macOS virtual keycode `0xB0`, macOS only).
    ///
    /// Emitted by the microphone/dictation key on the media function row
    /// (the F5 position on recent Apple keyboards), and by the Globe/Fn key
    /// when System Settings configures it to start dictation. A Globe tap
    /// configured for anything else emits `Fn` modifier flags plus a
    /// different keycode instead.
    ///
    /// Hardware events carry `MaskSecondaryFn` in their flags, so hotkeys
    /// recorded from a key press pair this with the `Fn` modifier (a
    /// string-configured `"dictation"` hotkey without `fn` will not match
    /// hardware presses until FN matching is reworked).
    Dictation,

    // Mouse buttons
    MouseLeft,
    MouseRight,
    MouseMiddle,
    /// Extra button 1 (often "back" on mice with side buttons)
    MouseX1,
    /// Extra button 2 (often "forward" on mice with side buttons)
    MouseX2,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::A => write!(f, "A"),
            Key::B => write!(f, "B"),
            Key::C => write!(f, "C"),
            Key::D => write!(f, "D"),
            Key::E => write!(f, "E"),
            Key::F => write!(f, "F"),
            Key::G => write!(f, "G"),
            Key::H => write!(f, "H"),
            Key::I => write!(f, "I"),
            Key::J => write!(f, "J"),
            Key::K => write!(f, "K"),
            Key::L => write!(f, "L"),
            Key::M => write!(f, "M"),
            Key::N => write!(f, "N"),
            Key::O => write!(f, "O"),
            Key::P => write!(f, "P"),
            Key::Q => write!(f, "Q"),
            Key::R => write!(f, "R"),
            Key::S => write!(f, "S"),
            Key::T => write!(f, "T"),
            Key::U => write!(f, "U"),
            Key::V => write!(f, "V"),
            Key::W => write!(f, "W"),
            Key::X => write!(f, "X"),
            Key::Y => write!(f, "Y"),
            Key::Z => write!(f, "Z"),
            Key::Num0 => write!(f, "0"),
            Key::Num1 => write!(f, "1"),
            Key::Num2 => write!(f, "2"),
            Key::Num3 => write!(f, "3"),
            Key::Num4 => write!(f, "4"),
            Key::Num5 => write!(f, "5"),
            Key::Num6 => write!(f, "6"),
            Key::Num7 => write!(f, "7"),
            Key::Num8 => write!(f, "8"),
            Key::Num9 => write!(f, "9"),
            Key::PlayPause => write!(f, "PlayPause"),
            Key::Stop => write!(f, "Stop"),
            Key::PrevTrack => write!(f, "PrevTrack"),
            Key::NextTrack => write!(f, "NextTrack"),
            Key::F1 => write!(f, "F1"),
            Key::F2 => write!(f, "F2"),
            Key::F3 => write!(f, "F3"),
            Key::F4 => write!(f, "F4"),
            Key::F5 => write!(f, "F5"),
            Key::F6 => write!(f, "F6"),
            Key::F7 => write!(f, "F7"),
            Key::F8 => write!(f, "F8"),
            Key::F9 => write!(f, "F9"),
            Key::F10 => write!(f, "F10"),
            Key::F11 => write!(f, "F11"),
            Key::F12 => write!(f, "F12"),
            Key::F13 => write!(f, "F13"),
            Key::F14 => write!(f, "F14"),
            Key::F15 => write!(f, "F15"),
            Key::F16 => write!(f, "F16"),
            Key::F17 => write!(f, "F17"),
            Key::F18 => write!(f, "F18"),
            Key::F19 => write!(f, "F19"),
            Key::F20 => write!(f, "F20"),
            Key::F21 => write!(f, "F21"),
            Key::F22 => write!(f, "F22"),
            Key::F23 => write!(f, "F23"),
            Key::F24 => write!(f, "F24"),
            Key::Space => write!(f, "Space"),
            Key::Return => write!(f, "Return"),
            Key::Tab => write!(f, "Tab"),
            Key::Escape => write!(f, "Escape"),
            Key::Delete => write!(f, "Delete"),
            Key::ForwardDelete => write!(f, "ForwardDelete"),
            Key::Insert => write!(f, "Insert"),
            Key::Pause => write!(f, "Pause"),
            Key::Home => write!(f, "Home"),
            Key::End => write!(f, "End"),
            Key::PageUp => write!(f, "PageUp"),
            Key::PageDown => write!(f, "PageDown"),
            Key::LeftArrow => write!(f, "Left"),
            Key::RightArrow => write!(f, "Right"),
            Key::UpArrow => write!(f, "Up"),
            Key::DownArrow => write!(f, "Down"),
            Key::Minus => write!(f, "-"),
            Key::Equal => write!(f, "="),
            Key::LeftBracket => write!(f, "["),
            Key::RightBracket => write!(f, "]"),
            Key::Backslash => write!(f, "\\"),
            Key::Semicolon => write!(f, ";"),
            Key::Quote => write!(f, "'"),
            Key::Comma => write!(f, ","),
            Key::Period => write!(f, "."),
            Key::Slash => write!(f, "/"),
            Key::Grave => write!(f, "`"),
            Key::Section => write!(f, "§"),
            Key::JisYen => write!(f, "¥"),
            Key::JisUnderscore => write!(f, "JisUnderscore"),
            Key::JisEisu => write!(f, "Eisu"),
            Key::JisKana => write!(f, "Kana"),
            Key::Keypad0 => write!(f, "Keypad0"),
            Key::Keypad1 => write!(f, "Keypad1"),
            Key::Keypad2 => write!(f, "Keypad2"),
            Key::Keypad3 => write!(f, "Keypad3"),
            Key::Keypad4 => write!(f, "Keypad4"),
            Key::Keypad5 => write!(f, "Keypad5"),
            Key::Keypad6 => write!(f, "Keypad6"),
            Key::Keypad7 => write!(f, "Keypad7"),
            Key::Keypad8 => write!(f, "Keypad8"),
            Key::Keypad9 => write!(f, "Keypad9"),
            Key::KeypadDecimal => write!(f, "KeypadDecimal"),
            Key::KeypadMultiply => write!(f, "KeypadMultiply"),
            Key::KeypadPlus => write!(f, "KeypadPlus"),
            Key::KeypadClear => write!(f, "KeypadClear"),
            Key::KeypadDivide => write!(f, "KeypadDivide"),
            Key::KeypadEnter => write!(f, "KeypadEnter"),
            Key::KeypadMinus => write!(f, "KeypadMinus"),
            Key::KeypadEquals => write!(f, "KeypadEquals"),
            Key::KeypadComma => write!(f, "KeypadComma"),
            Key::CapsLock => write!(f, "CapsLock"),
            Key::ScrollLock => write!(f, "ScrollLock"),
            Key::NumLock => write!(f, "NumLock"),
            Key::PrintScreen => write!(f, "PrintScreen"),
            Key::ContextMenu => write!(f, "ContextMenu"),
            Key::MouseLeft => write!(f, "MouseLeft"),
            Key::MouseRight => write!(f, "MouseRight"),
            Key::MouseMiddle => write!(f, "MouseMiddle"),
            Key::MouseX1 => write!(f, "MouseX1"),
            Key::MouseX2 => write!(f, "MouseX2"),
            Key::Dictation => write!(f, "Dictation"),
        }
    }
}

impl FromStr for Key {
    type Err = Error;

    /// Parse a key from its string representation (case-insensitive)
    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        match s.to_lowercase().as_str() {
            // Letters
            "a" => Ok(Key::A),
            "b" => Ok(Key::B),
            "c" => Ok(Key::C),
            "d" => Ok(Key::D),
            "e" => Ok(Key::E),
            "f" => Ok(Key::F),
            "g" => Ok(Key::G),
            "h" => Ok(Key::H),
            "i" => Ok(Key::I),
            "j" => Ok(Key::J),
            "k" => Ok(Key::K),
            "l" => Ok(Key::L),
            "m" => Ok(Key::M),
            "n" => Ok(Key::N),
            "o" => Ok(Key::O),
            "p" => Ok(Key::P),
            "q" => Ok(Key::Q),
            "r" => Ok(Key::R),
            "s" => Ok(Key::S),
            "t" => Ok(Key::T),
            "u" => Ok(Key::U),
            "v" => Ok(Key::V),
            "w" => Ok(Key::W),
            "x" => Ok(Key::X),
            "y" => Ok(Key::Y),
            "z" => Ok(Key::Z),

            // Numbers
            "0" | "num0" => Ok(Key::Num0),
            "1" | "num1" => Ok(Key::Num1),
            "2" | "num2" => Ok(Key::Num2),
            "3" | "num3" => Ok(Key::Num3),
            "4" | "num4" => Ok(Key::Num4),
            "5" | "num5" => Ok(Key::Num5),
            "6" | "num6" => Ok(Key::Num6),
            "7" | "num7" => Ok(Key::Num7),
            "8" | "num8" => Ok(Key::Num8),
            "9" | "num9" => Ok(Key::Num9),

            // Media keys
            "playpause" => Ok(Key::PlayPause),
            "stop" => Ok(Key::Stop),
            "prevtrack" => Ok(Key::PrevTrack),
            "nexttrack" => Ok(Key::NextTrack),

            // Function keys
            "f1" => Ok(Key::F1),
            "f2" => Ok(Key::F2),
            "f3" => Ok(Key::F3),
            "f4" => Ok(Key::F4),
            "f5" => Ok(Key::F5),
            "f6" => Ok(Key::F6),
            "f7" => Ok(Key::F7),
            "f8" => Ok(Key::F8),
            "f9" => Ok(Key::F9),
            "f10" => Ok(Key::F10),
            "f11" => Ok(Key::F11),
            "f12" => Ok(Key::F12),
            "f13" => Ok(Key::F13),
            "f14" => Ok(Key::F14),
            "f15" => Ok(Key::F15),
            "f16" => Ok(Key::F16),
            "f17" => Ok(Key::F17),
            "f18" => Ok(Key::F18),
            "f19" => Ok(Key::F19),
            "f20" => Ok(Key::F20),
            "f21" => Ok(Key::F21),
            "f22" => Ok(Key::F22),
            "f23" => Ok(Key::F23),
            "f24" => Ok(Key::F24),

            // Special keys
            "space" | " " => Ok(Key::Space),
            "return" | "enter" => Ok(Key::Return),
            "tab" => Ok(Key::Tab),
            "escape" | "esc" => Ok(Key::Escape),
            "delete" | "backspace" => Ok(Key::Delete),
            "forwarddelete" | "del" => Ok(Key::ForwardDelete),
            "insert" | "ins" => Ok(Key::Insert),
            "pause" | "break" => Ok(Key::Pause),
            "home" => Ok(Key::Home),
            "end" => Ok(Key::End),
            "pageup" => Ok(Key::PageUp),
            "pagedown" => Ok(Key::PageDown),

            // Arrow keys
            "left" | "leftarrow" => Ok(Key::LeftArrow),
            "right" | "rightarrow" => Ok(Key::RightArrow),
            "up" | "uparrow" => Ok(Key::UpArrow),
            "down" | "downarrow" => Ok(Key::DownArrow),

            // Punctuation and symbols
            "-" | "minus" => Ok(Key::Minus),
            "=" | "equal" | "equals" => Ok(Key::Equal),
            "[" | "leftbracket" => Ok(Key::LeftBracket),
            "]" | "rightbracket" => Ok(Key::RightBracket),
            "\\" | "backslash" => Ok(Key::Backslash),
            ";" | "semicolon" => Ok(Key::Semicolon),
            "'" | "quote" => Ok(Key::Quote),
            "," | "comma" => Ok(Key::Comma),
            "." | "period" => Ok(Key::Period),
            "/" | "slash" => Ok(Key::Slash),
            "`" | "grave" | "backtick" => Ok(Key::Grave),
            "§" | "section" => Ok(Key::Section),
            "¥" | "jisyen" | "yen" => Ok(Key::JisYen),
            "jisunderscore" => Ok(Key::JisUnderscore),
            "eisu" | "jiseisu" | "英数" => Ok(Key::JisEisu),
            "kana" | "jiskana" | "かな" => Ok(Key::JisKana),

            // Keypad
            "keypad0" => Ok(Key::Keypad0),
            "keypad1" => Ok(Key::Keypad1),
            "keypad2" => Ok(Key::Keypad2),
            "keypad3" => Ok(Key::Keypad3),
            "keypad4" => Ok(Key::Keypad4),
            "keypad5" => Ok(Key::Keypad5),
            "keypad6" => Ok(Key::Keypad6),
            "keypad7" => Ok(Key::Keypad7),
            "keypad8" => Ok(Key::Keypad8),
            "keypad9" => Ok(Key::Keypad9),
            "keypad." | "keypaddecimal" => Ok(Key::KeypadDecimal),
            "keypad*" | "keypadmultiply" => Ok(Key::KeypadMultiply),
            "keypad+" | "keypadplus" => Ok(Key::KeypadPlus),
            "keypadclear" => Ok(Key::KeypadClear),
            "keypad/" | "keypaddivide" => Ok(Key::KeypadDivide),
            "keypadenter" => Ok(Key::KeypadEnter),
            "keypad-" | "keypadminus" => Ok(Key::KeypadMinus),
            "keypad=" | "keypadequals" => Ok(Key::KeypadEquals),
            "keypad," | "keypadcomma" => Ok(Key::KeypadComma),

            // Lock keys
            "capslock" | "caps" => Ok(Key::CapsLock),
            "scrolllock" | "scroll" => Ok(Key::ScrollLock),
            "numlock" => Ok(Key::NumLock),

            "printscreen" | "prtsc" | "sysrq" => Ok(Key::PrintScreen),
            "contextmenu" | "menu" | "apps" | "application" => Ok(Key::ContextMenu),

            // Mouse buttons
            "mouseleft" | "leftclick" | "lmb" | "mouse1" => Ok(Key::MouseLeft),
            "mouseright" | "rightclick" | "rmb" | "mouse2" => Ok(Key::MouseRight),
            "mousemiddle" | "middleclick" | "mmb" | "mouse3" => Ok(Key::MouseMiddle),
            "mousex1" | "mouse4" | "back" | "xbutton1" => Ok(Key::MouseX1),
            "mousex2" | "mouse5" | "forward" | "xbutton2" => Ok(Key::MouseX2),

            // Dictation / voice key (macOS keycode 0xB0). "globe" is accepted
            // because the Globe key emits this keycode when configured to
            // start dictation.
            "dictation" | "voice" | "globe" => Ok(Key::Dictation),

            _ => Err(Error::UnknownKey(s.to_string())),
        }
    }
}

