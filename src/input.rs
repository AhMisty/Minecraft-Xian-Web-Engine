//! ### English
//! Input event ABI + mapping into Servo `InputEvent`.
//!
//! ### 中文
//! 输入事件 ABI + 映射到 Servo `InputEvent`。

use servo::{
    DevicePoint, InputEvent, Key, KeyState, KeyboardEvent, Location, Modifiers, MouseButton,
    MouseButtonAction, MouseButtonEvent, MouseMoveEvent, WebViewPoint, WheelDelta, WheelEvent,
    WheelMode,
};

/// ### English
/// Input event kind: mouse move.
///
/// ### 中文
/// 输入事件类型：鼠标移动。
pub const XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE: u32 = 1;

/// ### English
/// Input event kind: mouse button.
///
/// ### 中文
/// 输入事件类型：鼠标按键。
pub const XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON: u32 = 2;

/// ### English
/// Input event kind: wheel.
///
/// ### 中文
/// 输入事件类型：滚轮。
pub const XIAN_WEB_ENGINE_INPUT_KIND_WHEEL: u32 = 3;

/// ### English
/// Input event kind: keyboard.
///
/// ### 中文
/// 输入事件类型：键盘。
pub const XIAN_WEB_ENGINE_INPUT_KIND_KEY: u32 = 4;

/// ### English
/// Modifier bit: SHIFT.
///
/// ### 中文
/// 修饰键位：SHIFT。
pub const XIAN_WEB_ENGINE_MOD_SHIFT: u32 = 1 << 0;

/// ### English
/// Modifier bit: CONTROL.
///
/// ### 中文
/// 修饰键位：CONTROL。
pub const XIAN_WEB_ENGINE_MOD_CONTROL: u32 = 1 << 1;

/// ### English
/// Modifier bit: ALT.
///
/// ### 中文
/// 修饰键位：ALT。
pub const XIAN_WEB_ENGINE_MOD_ALT: u32 = 1 << 2;

/// ### English
/// Modifier bit: META (Windows/Super/Command).
///
/// ### 中文
/// 修饰键位：META（Windows/Super/Command）。
pub const XIAN_WEB_ENGINE_MOD_META: u32 = 1 << 3;

#[repr(C)]
#[derive(Clone, Copy)]
/// ### English
/// Compact input event struct for the C ABI.
///
/// One struct carries all event types; interpretation depends on `kind`.
///
/// ### 中文
/// C ABI 使用的紧凑输入事件结构。
///
/// 单一结构承载所有事件类型，具体语义由 `kind` 决定。
pub struct XianWebEngineInputEvent {
    /// ### English
    /// Event kind: `XIAN_WEB_ENGINE_INPUT_KIND_*`.
    ///
    /// ### 中文
    /// 事件类型：`XIAN_WEB_ENGINE_INPUT_KIND_*`。
    pub kind: u32,

    /// ### English
    /// X position in device pixels (used by mouse move/button/wheel).
    ///
    /// ### 中文
    /// X 坐标（设备像素；用于鼠标移动/按键/滚轮）。
    pub x: f32,

    /// ### English
    /// Y position in device pixels (used by mouse move/button/wheel).
    ///
    /// ### 中文
    /// Y 坐标（设备像素；用于鼠标移动/按键/滚轮）。
    pub y: f32,

    /// ### English
    /// Modifier mask: `XIAN_WEB_ENGINE_MOD_*`.
    ///
    /// ### 中文
    /// 修饰键位掩码：`XIAN_WEB_ENGINE_MOD_*`。
    pub modifiers: u32,

    /// ### English
    /// Mouse button id (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON`).
    ///
    /// ### 中文
    /// 鼠标按键编号（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON` 时使用）。
    pub mouse_button: u32,

    /// ### English
    /// Mouse action (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON`).
    ///
    /// - `0`: down
    /// - non-zero: up
    ///
    /// ### 中文
    /// 鼠标动作（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON` 时使用）。
    ///
    /// - `0`：按下
    /// - 非 0：抬起
    pub mouse_action: u32,

    /// ### English
    /// Wheel delta X (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL`).
    ///
    /// ### 中文
    /// 滚轮增量 X（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL` 时使用）。
    pub wheel_delta_x: f64,

    /// ### English
    /// Wheel delta Y (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL`).
    ///
    /// ### 中文
    /// 滚轮增量 Y（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL` 时使用）。
    pub wheel_delta_y: f64,

    /// ### English
    /// Wheel delta Z (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL`).
    ///
    /// ### 中文
    /// 滚轮增量 Z（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL` 时使用）。
    pub wheel_delta_z: f64,

    /// ### English
    /// Wheel delta mode (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL`).
    ///
    /// - `0`: pixel
    /// - `1`: line
    /// - `2`: page
    ///
    /// ### 中文
    /// 滚轮单位（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL` 时使用）。
    ///
    /// - `0`：像素
    /// - `1`：行
    /// - `2`：页
    pub wheel_mode: u32,

    /// ### English
    /// Key state (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY`).
    ///
    /// - `0`: down
    /// - non-zero: up
    ///
    /// ### 中文
    /// 按键状态（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY` 时使用）。
    ///
    /// - `0`：按下
    /// - 非 0：抬起
    pub key_state: u32,

    /// ### English
    /// Key location (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY`).
    ///
    /// - `0`: standard
    /// - `1`: left
    /// - `2`: right
    /// - `3`: numpad
    ///
    /// ### 中文
    /// 按键位置（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY` 时使用）。
    ///
    /// - `0`：标准
    /// - `1`：左侧
    /// - `2`：右侧
    /// - `3`：数字键盘
    pub key_location: u32,

    /// ### English
    /// Whether this key event is a repeat (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY`).
    ///
    /// - `0`: false
    /// - non-zero: true
    ///
    /// ### 中文
    /// 是否为重复按键（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY` 时使用）。
    ///
    /// - `0`：否
    /// - 非 0：是
    pub repeat: u32,

    /// ### English
    /// Whether the IME is composing (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY`).
    ///
    /// - `0`: false
    /// - non-zero: true
    ///
    /// ### 中文
    /// 是否处于输入法组合态（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY` 时使用）。
    ///
    /// - `0`：否
    /// - 非 0：是
    pub is_composing: u32,

    /// ### English
    /// Unicode codepoint (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY`).
    ///
    /// Use `0` when no printable character is available.
    ///
    /// ### 中文
    /// Unicode 码点（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY` 时使用）。
    ///
    /// 当没有可打印字符时传 `0`。
    pub key_codepoint: u32,

    /// ### English
    /// Raw GLFW key code (used when `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY`).
    ///
    /// ### 中文
    /// GLFW 原始 key code（当 `kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY` 时使用）。
    pub glfw_key: u32,
}

/// ### English
/// Maps a C ABI input event into Servo `InputEvent`.
///
/// #### Parameters
/// - `event`: Input event in ABI form.
///
/// #### Returns
/// - `Some(InputEvent)`: Supported and converted.
/// - `None`: Unknown or unsupported `kind`.
///
/// ### 中文
/// 将 C ABI 输入事件映射为 Servo `InputEvent`。
///
/// #### 参数
/// - `event`：ABI 形式的输入事件。
///
/// #### 返回
/// - `Some(InputEvent)`：支持并完成转换。
/// - `None`：未知或不支持的 `kind`。
#[inline]
pub(crate) fn map_input_event(event: &XianWebEngineInputEvent) -> Option<InputEvent> {
    let modifiers = modifiers_from_mask(event.modifiers);

    match event.kind {
        XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE => {
            let point = WebViewPoint::Device(DevicePoint::new(event.x, event.y));
            Some(InputEvent::MouseMove(MouseMoveEvent::new(point)))
        }
        XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON => {
            let point = WebViewPoint::Device(DevicePoint::new(event.x, event.y));
            let action = if event.mouse_action == 0 {
                MouseButtonAction::Down
            } else {
                MouseButtonAction::Up
            };
            Some(InputEvent::MouseButton(MouseButtonEvent::new(
                action,
                MouseButton::from(event.mouse_button as u64),
                point,
            )))
        }
        XIAN_WEB_ENGINE_INPUT_KIND_WHEEL => {
            let point = WebViewPoint::Device(DevicePoint::new(event.x, event.y));
            let mode = match event.wheel_mode {
                1 => WheelMode::DeltaLine,
                2 => WheelMode::DeltaPage,
                _ => WheelMode::DeltaPixel,
            };
            Some(InputEvent::Wheel(WheelEvent::new(
                WheelDelta {
                    x: event.wheel_delta_x,
                    y: event.wheel_delta_y,
                    z: event.wheel_delta_z,
                    mode,
                },
                point,
            )))
        }
        XIAN_WEB_ENGINE_INPUT_KIND_KEY => {
            let state = if event.key_state == 0 {
                KeyState::Down
            } else {
                KeyState::Up
            };

            let location = match event.key_location {
                1 => Location::Left,
                2 => Location::Right,
                3 => Location::Numpad,
                _ => Location::Standard,
            };

            let code = glfw_key_to_code(event.glfw_key);
            let key = glfw_key_to_key(event.glfw_key, event.key_codepoint, modifiers);

            Some(InputEvent::Keyboard(KeyboardEvent::new_without_event(
                state,
                key,
                code,
                location,
                modifiers,
                event.repeat != 0,
                event.is_composing != 0,
            )))
        }
        _ => None,
    }
}

/// ### English
/// Converts an ABI modifier mask into Servo `Modifiers`.
///
/// #### Parameters
/// - `mask`: Bitmask composed of `XIAN_WEB_ENGINE_MOD_*`.
///
/// #### Returns
/// - Servo `Modifiers`.
///
/// ### 中文
/// 将 ABI 修饰键位掩码转换为 Servo `Modifiers`。
///
/// #### 参数
/// - `mask`：由 `XIAN_WEB_ENGINE_MOD_*` 组合的位掩码。
///
/// #### 返回
/// - Servo `Modifiers`。
#[inline]
fn modifiers_from_mask(mask: u32) -> Modifiers {
    let mut mods = Modifiers::empty();
    if (mask & XIAN_WEB_ENGINE_MOD_SHIFT) != 0 {
        mods.insert(Modifiers::SHIFT);
    }
    if (mask & XIAN_WEB_ENGINE_MOD_CONTROL) != 0 {
        mods.insert(Modifiers::CONTROL);
    }
    if (mask & XIAN_WEB_ENGINE_MOD_ALT) != 0 {
        mods.insert(Modifiers::ALT);
    }
    if (mask & XIAN_WEB_ENGINE_MOD_META) != 0 {
        mods.insert(Modifiers::META);
    }
    mods
}

/// ### English
/// Converts a GLFW key code into Servo `Code`.
///
/// #### Parameters
/// - `glfw_key`: GLFW key code.
///
/// #### Returns
/// - Servo `Code` (or `Code::Unidentified` if unknown).
///
/// ### 中文
/// 将 GLFW key code 转换为 Servo `Code`。
///
/// #### 参数
/// - `glfw_key`：GLFW key code。
///
/// #### 返回
/// - Servo `Code`（未知时为 `Code::Unidentified`）。
fn glfw_key_to_code(glfw_key: u32) -> servo::Code {
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
/// Converts a GLFW key code (and optional codepoint) into Servo `Key`.
///
/// #### Parameters
/// - `glfw_key`: GLFW key code.
/// - `key_codepoint`: Unicode codepoint (0 means "none").
/// - `modifiers`: Modifier state (used by ASCII fallback rules).
///
/// #### Returns
/// - Servo `Key`.
///
/// ### 中文
/// 将 GLFW key code（以及可选码点）转换为 Servo `Key`。
///
/// #### 参数
/// - `glfw_key`：GLFW key code。
/// - `key_codepoint`：Unicode 码点（0 表示“无”）。
/// - `modifiers`：修饰键位状态（用于 ASCII 回退规则）。
///
/// #### 返回
/// - Servo `Key`。
fn glfw_key_to_key(glfw_key: u32, key_codepoint: u32, modifiers: Modifiers) -> Key {
    if key_codepoint != 0
        && let Some(ch) = char::from_u32(key_codepoint)
        && !ch.is_control()
    {
        return Key::Character(ch.to_string());
    }

    if let Some(named) = glfw_key_to_named_key(glfw_key) {
        return Key::Named(named);
    }

    if let Some(ch) = glfw_key_to_ascii_fallback(glfw_key, modifiers) {
        return Key::Character(ch.to_string());
    }

    Key::default()
}

/// ### English
/// Converts a GLFW key code into Servo `NamedKey` when applicable.
///
/// #### Parameters
/// - `glfw_key`: GLFW key code.
///
/// #### Returns
/// - `Some(NamedKey)` if the key is a known "named" key; otherwise `None`.
///
/// ### 中文
/// 在适用时将 GLFW key code 转换为 Servo `NamedKey`。
///
/// #### 参数
/// - `glfw_key`：GLFW key code。
///
/// #### 返回
/// - 若为已知“命名键”则返回 `Some(NamedKey)`，否则返回 `None`。
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
/// Provides an ASCII fallback character for a GLFW key code.
///
/// #### Parameters
/// - `glfw_key`: GLFW key code.
/// - `modifiers`: Modifier state (SHIFT affects the produced character).
///
/// #### Returns
/// - `Some(char)` for supported ASCII keys; otherwise `None`.
///
/// ### 中文
/// 为 GLFW key code 提供 ASCII 回退字符。
///
/// #### 参数
/// - `glfw_key`：GLFW key code。
/// - `modifiers`：修饰键位状态（SHIFT 会影响结果）。
///
/// #### 返回
/// - 对于支持的 ASCII 键返回 `Some(char)`，否则返回 `None`。
fn glfw_key_to_ascii_fallback(glfw_key: u32, modifiers: Modifiers) -> Option<char> {
    let shift = modifiers.contains(Modifiers::SHIFT);
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
            let base = if shift { 'A' } else { 'a' };
            char::from_u32(base as u32 + (glfw_key - 65))?
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
