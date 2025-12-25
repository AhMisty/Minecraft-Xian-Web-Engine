//! ### English
//! GLFW key-code translation into Servo keyboard semantics (`Code` / `Key`).
//!
//! ### 中文
//! GLFW 键码到 Servo 键盘语义（`Code` / `Key`）的转换。

/// ### English
/// Maps GLFW key codes into W3C `Code` values used by Servo.
/// This is a best-effort mapping for embedder-side key events.
///
/// ### 中文
/// 将 GLFW 键码映射到 Servo 使用的 W3C `Code`。
/// 这是宿主侧键盘事件的尽力映射（best-effort）。
pub(super) fn glfw_key_to_code(glfw_key: u32) -> servo::Code {
    use servo::Code;

    match glfw_key {
        32 => Code::Space,
        39 => Code::Quote,
        44 => Code::Comma,
        45 => Code::Minus,
        46 => Code::Period,
        47 => Code::Slash,
        48 => Code::Digit0,
        49 => Code::Digit1,
        50 => Code::Digit2,
        51 => Code::Digit3,
        52 => Code::Digit4,
        53 => Code::Digit5,
        54 => Code::Digit6,
        55 => Code::Digit7,
        56 => Code::Digit8,
        57 => Code::Digit9,
        59 => Code::Semicolon,
        61 => Code::Equal,
        65 => Code::KeyA,
        66 => Code::KeyB,
        67 => Code::KeyC,
        68 => Code::KeyD,
        69 => Code::KeyE,
        70 => Code::KeyF,
        71 => Code::KeyG,
        72 => Code::KeyH,
        73 => Code::KeyI,
        74 => Code::KeyJ,
        75 => Code::KeyK,
        76 => Code::KeyL,
        77 => Code::KeyM,
        78 => Code::KeyN,
        79 => Code::KeyO,
        80 => Code::KeyP,
        81 => Code::KeyQ,
        82 => Code::KeyR,
        83 => Code::KeyS,
        84 => Code::KeyT,
        85 => Code::KeyU,
        86 => Code::KeyV,
        87 => Code::KeyW,
        88 => Code::KeyX,
        89 => Code::KeyY,
        90 => Code::KeyZ,
        91 => Code::BracketLeft,
        92 => Code::Backslash,
        93 => Code::BracketRight,
        96 => Code::Backquote,

        256 => Code::Escape,
        257 => Code::Enter,
        258 => Code::Tab,
        259 => Code::Backspace,
        260 => Code::Insert,
        261 => Code::Delete,
        262 => Code::ArrowRight,
        263 => Code::ArrowLeft,
        264 => Code::ArrowDown,
        265 => Code::ArrowUp,
        266 => Code::PageUp,
        267 => Code::PageDown,
        268 => Code::Home,
        269 => Code::End,

        280 => Code::CapsLock,
        281 => Code::ScrollLock,
        282 => Code::NumLock,
        283 => Code::PrintScreen,
        284 => Code::Pause,

        290 => Code::F1,
        291 => Code::F2,
        292 => Code::F3,
        293 => Code::F4,
        294 => Code::F5,
        295 => Code::F6,
        296 => Code::F7,
        297 => Code::F8,
        298 => Code::F9,
        299 => Code::F10,
        300 => Code::F11,
        301 => Code::F12,

        320 => Code::Numpad0,
        321 => Code::Numpad1,
        322 => Code::Numpad2,
        323 => Code::Numpad3,
        324 => Code::Numpad4,
        325 => Code::Numpad5,
        326 => Code::Numpad6,
        327 => Code::Numpad7,
        328 => Code::Numpad8,
        329 => Code::Numpad9,
        330 => Code::NumpadDecimal,
        331 => Code::NumpadDivide,
        332 => Code::NumpadMultiply,
        333 => Code::NumpadSubtract,
        334 => Code::NumpadAdd,
        335 => Code::NumpadEnter,
        336 => Code::NumpadEqual,

        340 => Code::ShiftLeft,
        341 => Code::ControlLeft,
        342 => Code::AltLeft,
        343 => Code::MetaLeft,
        344 => Code::ShiftRight,
        345 => Code::ControlRight,
        346 => Code::AltRight,
        347 => Code::MetaRight,
        348 => Code::ContextMenu,

        _ => Code::Unidentified,
    }
}

/// ### English
/// Maps GLFW key codes + optional Unicode codepoint into Servo `Key`.
/// Prefer the actual typed codepoint when provided (better for IME/text input).
///
/// ### 中文
/// 将 GLFW 键码 + 可选 Unicode 码点映射到 Servo `Key`。
/// 如果提供了实际输入的码点，则优先使用（更利于 IME/文本输入）。
pub(super) fn glfw_key_to_key(
    glfw_key: u32,
    key_codepoint: u32,
    modifiers: servo::Modifiers,
) -> servo::Key {
    if key_codepoint != 0
        && let Some(ch) = char::from_u32(key_codepoint)
        && !ch.is_control()
    {
        return servo::Key::Character(ch.to_string());
    }

    if let Some(named) = glfw_key_to_named_key(glfw_key) {
        return servo::Key::Named(named);
    }

    if let Some(ch) = glfw_key_to_char(glfw_key, modifiers) {
        return servo::Key::Character(ch.to_string());
    }

    servo::Key::default()
}

/// ### English
/// Named key mapping (Enter/Escape/Arrows/Function keys...).
///
/// ### 中文
/// 命名按键映射（Enter/Escape/方向键/功能键等）。
fn glfw_key_to_named_key(glfw_key: u32) -> Option<servo::NamedKey> {
    use servo::NamedKey;
    match glfw_key {
        256 => Some(NamedKey::Escape),
        257 => Some(NamedKey::Enter),
        258 => Some(NamedKey::Tab),
        259 => Some(NamedKey::Backspace),
        260 => Some(NamedKey::Insert),
        261 => Some(NamedKey::Delete),
        262 => Some(NamedKey::ArrowRight),
        263 => Some(NamedKey::ArrowLeft),
        264 => Some(NamedKey::ArrowDown),
        265 => Some(NamedKey::ArrowUp),
        266 => Some(NamedKey::PageUp),
        267 => Some(NamedKey::PageDown),
        268 => Some(NamedKey::Home),
        269 => Some(NamedKey::End),
        280 => Some(NamedKey::CapsLock),
        281 => Some(NamedKey::ScrollLock),
        282 => Some(NamedKey::NumLock),
        283 => Some(NamedKey::PrintScreen),
        284 => Some(NamedKey::Pause),
        290 => Some(NamedKey::F1),
        291 => Some(NamedKey::F2),
        292 => Some(NamedKey::F3),
        293 => Some(NamedKey::F4),
        294 => Some(NamedKey::F5),
        295 => Some(NamedKey::F6),
        296 => Some(NamedKey::F7),
        297 => Some(NamedKey::F8),
        298 => Some(NamedKey::F9),
        299 => Some(NamedKey::F10),
        300 => Some(NamedKey::F11),
        301 => Some(NamedKey::F12),
        340 | 344 => Some(NamedKey::Shift),
        341 | 345 => Some(NamedKey::Control),
        342 | 346 => Some(NamedKey::Alt),
        343 | 347 => Some(NamedKey::Meta),
        348 => Some(NamedKey::ContextMenu),
        _ => None,
    }
}

/// ### English
/// ASCII fallback for characters when codepoint is not provided.
/// This assumes a US-like layout and applies SHIFT for common punctuation.
///
/// ### 中文
/// 当未提供码点时的 ASCII 回退方案。
/// 假设接近 US 键盘布局，并对常见符号应用 SHIFT。
fn glfw_key_to_char(glfw_key: u32, modifiers: servo::Modifiers) -> Option<char> {
    let shift = modifiers.contains(servo::Modifiers::SHIFT);

    let ch = match glfw_key {
        32 => ' ',
        39 => {
            if shift {
                '"'
            } else {
                '\''
            }
        }
        44 => {
            if shift {
                '<'
            } else {
                ','
            }
        }
        45 => {
            if shift {
                '_'
            } else {
                '-'
            }
        }
        46 => {
            if shift {
                '>'
            } else {
                '.'
            }
        }
        47 => {
            if shift {
                '?'
            } else {
                '/'
            }
        }
        48 => {
            if shift {
                ')'
            } else {
                '0'
            }
        }
        49 => {
            if shift {
                '!'
            } else {
                '1'
            }
        }
        50 => {
            if shift {
                '@'
            } else {
                '2'
            }
        }
        51 => {
            if shift {
                '#'
            } else {
                '3'
            }
        }
        52 => {
            if shift {
                '$'
            } else {
                '4'
            }
        }
        53 => {
            if shift {
                '%'
            } else {
                '5'
            }
        }
        54 => {
            if shift {
                '^'
            } else {
                '6'
            }
        }
        55 => {
            if shift {
                '&'
            } else {
                '7'
            }
        }
        56 => {
            if shift {
                '*'
            } else {
                '8'
            }
        }
        57 => {
            if shift {
                '('
            } else {
                '9'
            }
        }
        59 => {
            if shift {
                ':'
            } else {
                ';'
            }
        }
        61 => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        65..=90 => {
            let base = glfw_key as u8;
            let ascii = if shift { base } else { base + 32 };
            return Some(ascii as char);
        }
        91 => {
            if shift {
                '{'
            } else {
                '['
            }
        }
        92 => {
            if shift {
                '|'
            } else {
                '\\'
            }
        }
        93 => {
            if shift {
                '}'
            } else {
                ']'
            }
        }
        96 => {
            if shift {
                '~'
            } else {
                '`'
            }
        }
        _ => return None,
    };

    Some(ch)
}
