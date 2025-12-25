//! ### English
//! C ABI input event types.
//! Kept as POD (plain-old-data) so Java/Panama can pass arrays efficiently.
//!
//! ### 中文
//! C ABI 输入事件类型。
//! 保持为 POD（纯数据结构），方便 Java/Panama 高效传递数组。

/// ### English
/// One input event in a single struct.
/// All fields are numeric to avoid UTF-8 parsing / allocation on the Rust hot path.
///
/// ### 中文
/// 单个输入事件结构体。
/// 全部字段为数值，避免 Rust 热路径进行 UTF-8 解析/分配。
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct XianWebEngineInputEvent {
    /// ### English
    /// Event kind (one of `XIAN_WEB_ENGINE_INPUT_KIND_*`).
    ///
    /// ### 中文
    /// 事件类型（`XIAN_WEB_ENGINE_INPUT_KIND_*` 之一）。
    pub kind: u32,
    /// ### English
    /// Cursor X in device pixels (for pointer-related events).
    ///
    /// ### 中文
    /// 光标 X（设备像素；用于指针相关事件）。
    pub x: f32,
    /// ### English
    /// Cursor Y in device pixels (for pointer-related events).
    ///
    /// ### 中文
    /// 光标 Y（设备像素；用于指针相关事件）。
    pub y: f32,
    /// ### English
    /// Modifier bitmask (embedder-defined; mapped to Servo modifiers on the Servo thread).
    ///
    /// ### 中文
    /// 修饰键位掩码（宿主定义；在 Servo 线程映射为 Servo modifiers）。
    pub modifiers: u32,

    /// ### English
    /// Mouse button (GLFW button value).
    ///
    /// ### 中文
    /// 鼠标按键（GLFW button 值）。
    pub mouse_button: u32,
    /// ### English
    /// Mouse button action (`0` = down, otherwise up).
    ///
    /// ### 中文
    /// 鼠标按键动作（`0` = down，其它 = up）。
    pub mouse_action: u32,

    /// ### English
    /// Wheel delta X.
    ///
    /// ### 中文
    /// 滚轮 delta X。
    pub wheel_delta_x: f64,
    /// ### English
    /// Wheel delta Y.
    ///
    /// ### 中文
    /// 滚轮 delta Y。
    pub wheel_delta_y: f64,
    /// ### English
    /// Wheel delta Z.
    ///
    /// ### 中文
    /// 滚轮 delta Z。
    pub wheel_delta_z: f64,
    /// ### English
    /// Wheel mode (`0` = pixel, `1` = line, `2` = page).
    ///
    /// ### 中文
    /// 滚轮模式（`0` = pixel，`1` = line，`2` = page）。
    pub wheel_mode: u32,

    /// ### English
    /// Key state (`0` = down, otherwise up).
    ///
    /// ### 中文
    /// 按键状态（`0` = down，其它 = up）。
    pub key_state: u32,
    /// ### English
    /// Key location (`0` = standard, `1` = left, `2` = right, `3` = numpad).
    ///
    /// ### 中文
    /// 按键位置（`0` = standard，`1` = left，`2` = right，`3` = numpad）。
    pub key_location: u32,
    /// ### English
    /// Repeat flag (`0` = not repeat, otherwise repeat).
    ///
    /// ### 中文
    /// 重复标记（`0` = 非重复，其它 = 重复）。
    pub repeat: u32,
    /// ### English
    /// IME composing flag (`0` = false, otherwise true).
    ///
    /// ### 中文
    /// IME composing 标记（`0` = false，其它 = true）。
    pub is_composing: u32,
    /// ### English
    /// Unicode codepoint for the typed character (0 if unknown).
    ///
    /// ### 中文
    /// 输入字符的 Unicode 码点（未知则为 0）。
    pub key_codepoint: u32,
    /// ### English
    /// Raw GLFW key code.
    ///
    /// ### 中文
    /// 原始 GLFW key code。
    pub glfw_key: u32,
}

/// ### English
/// Input kind: mouse move.
///
/// ### 中文
/// 输入类型：鼠标移动。
pub const XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE: u32 = 1;

/// ### English
/// Input kind: mouse button.
///
/// ### 中文
/// 输入类型：鼠标按键。
pub const XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON: u32 = 2;

/// ### English
/// Input kind: wheel.
///
/// ### 中文
/// 输入类型：滚轮。
pub const XIAN_WEB_ENGINE_INPUT_KIND_WHEEL: u32 = 3;

/// ### English
/// Input kind: keyboard.
///
/// ### 中文
/// 输入类型：键盘。
pub const XIAN_WEB_ENGINE_INPUT_KIND_KEY: u32 = 4;
