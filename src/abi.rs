//! ### English
//! C ABI constants and `#[repr(C)]` data types.
//!
//! This module contains only:
//! - ABI-stable integers (versions, flags, kind tags)
//! - Plain data structs used across the FFI boundary
//!
//! It intentionally does **not** expose any Rust-only logic (engine, GL, Servo integration).
//!
//! ### 中文
//! C ABI 常量与 `#[repr(C)]` 数据类型。
//!
//! 本模块只包含：
//! - ABI 稳定的整数常量（版本号、标志位、kind 标签）
//! - 过 FFI 边界使用的纯数据结构
//!
//! 本模块刻意不包含任何 Rust 侧实现逻辑（引擎、GL、Servo 集成等）。

use std::ffi::c_char;
use std::ptr;

/// ### English
/// C ABI version.
///
/// ### 中文
/// C ABI 版本号。
pub(crate) const XIAN_WEB_ENGINE_ABI_VERSION: u32 = 1;

/// ### English
/// OpenGL API kind (desktop OpenGL).
///
/// ### 中文
/// OpenGL API 类型（桌面 OpenGL）。
pub(crate) const XIAN_WEB_ENGINE_GL_API_GL: u32 = 1;

/// ### English
/// OpenGL API kind (OpenGL ES).
///
/// ### 中文
/// OpenGL API 类型（OpenGL ES）。
pub(crate) const XIAN_WEB_ENGINE_GL_API_GLES: u32 = 2;

/// ### English
/// Input event kind: mouse move.
///
/// ### 中文
/// 输入事件类型：鼠标移动。
pub(crate) const XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE: u32 = 1;

/// ### English
/// Input event kind: mouse button.
///
/// ### 中文
/// 输入事件类型：鼠标按键。
pub(crate) const XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON: u32 = 2;

/// ### English
/// Input event kind: wheel.
///
/// ### 中文
/// 输入事件类型：滚轮。
pub(crate) const XIAN_WEB_ENGINE_INPUT_KIND_WHEEL: u32 = 3;

/// ### English
/// Input event kind: keyboard.
///
/// ### 中文
/// 输入事件类型：键盘。
pub(crate) const XIAN_WEB_ENGINE_INPUT_KIND_KEY: u32 = 4;

/// ### English
/// Modifier bit: SHIFT.
///
/// ### 中文
/// 修饰键位：SHIFT。
pub(crate) const XIAN_WEB_ENGINE_MOD_SHIFT: u32 = 1 << 0;

/// ### English
/// Modifier bit: CONTROL.
///
/// ### 中文
/// 修饰键位：CONTROL。
pub(crate) const XIAN_WEB_ENGINE_MOD_CONTROL: u32 = 1 << 1;

/// ### English
/// Modifier bit: ALT.
///
/// ### 中文
/// 修饰键位：ALT。
pub(crate) const XIAN_WEB_ENGINE_MOD_ALT: u32 = 1 << 2;

/// ### English
/// Modifier bit: META (Windows/Super/Command).
///
/// ### 中文
/// 修饰键位：META（Windows/Super/Command）。
pub(crate) const XIAN_WEB_ENGINE_MOD_META: u32 = 1 << 3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
/// ### English
/// Minimal GLFW API table required by this embedder.
///
/// All fields are raw pointers stored as `uintptr_t` (use 0 for NULL).
///
/// ### 中文
/// 本嵌入层所需的最小 GLFW API 表。
///
/// 所有字段使用 `uintptr_t` 承载函数指针（传 0 表示 NULL）。
pub struct XianWebEngineGlfwApi {
    /// ### English
    /// `glfwGetProcAddress` function pointer.
    ///
    /// Signature (C): `GLFWglproc glfwGetProcAddress(const char* name)`.
    ///
    /// ### 中文
    /// `glfwGetProcAddress` 函数指针。
    pub glfw_get_proc_address: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// ### English
/// View creation configuration.
///
/// ### 中文
/// View 创建配置。
pub struct XianWebEngineViewConfig {
    /// ### English
    /// Initial view width in pixels (clamped to >= 1).
    ///
    /// ### 中文
    /// 初始宽度（像素，最小为 1）。
    pub width: u32,

    /// ### English
    /// Initial view height in pixels (clamped to >= 1).
    ///
    /// ### 中文
    /// 初始高度（像素，最小为 1）。
    pub height: u32,

    /// ### English
    /// HiDPI scale factor (`1.0` means 1 CSS pixel = 1 device pixel).
    ///
    /// Non-finite values or values `<= 0` are treated as `1.0`.
    ///
    /// ### 中文
    /// HiDPI 缩放因子（`1.0` 表示 1 个 CSS 像素 = 1 个设备像素）。
    ///
    /// 非有限值或 `<= 0` 会被视为 `1.0`。
    pub hidpi_scale_factor: f32,

    /// ### English
    /// Optional initial URL (NUL-terminated UTF-8). NULL/empty/invalid UTF-8 means "do not load".
    ///
    /// ### 中文
    /// 可选初始 URL（NUL 结尾 UTF-8）。传 NULL/空字符串/UTF-8 无效表示“不加载”。
    pub initial_url: *const c_char,
}

impl Default for XianWebEngineViewConfig {
    /// ### English
    /// Returns the ABI default view configuration values.
    ///
    /// #### Returns
    /// - ABI default `XianWebEngineViewConfig`.
    ///
    /// ### 中文
    /// 返回 ABI 默认 view 配置值。
    ///
    /// #### 返回
    /// - ABI 默认的 `XianWebEngineViewConfig`。
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            hidpi_scale_factor: 1.0,
            initial_url: ptr::null(),
        }
    }
}

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
