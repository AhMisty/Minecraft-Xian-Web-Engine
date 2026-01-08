//! ### English
//! C ABI surface for `xian_web_engine`.
//!
//! All exported symbols are `extern "C"` functions; structs are `#[repr(C)]`.
//! Strings passed from Java/Panama must be NUL-terminated UTF-8 (C string); they will be
//! validated as UTF-8 and will be truncated at the first NUL byte.
//!
//! ### 中文
//! `xian_web_engine` 的 C ABI 接口层。
//!
//! 所有导出符号均为 `extern "C"` 函数；结构体使用 `#[repr(C)]`。
//! Java/Panama 传入的字符串必须是以 NUL 结尾的 UTF-8（C 字符串）；Rust 会校验 UTF-8，
//! 且在遇到第一个 NUL 字节处截断。
mod config;
mod engine;
mod frame;
mod input;
mod view;

use std::ffi::{CStr, c_char, c_void};
use std::path::PathBuf;

use crate::engine::{AcquiredFrame, EmbedderGlfwApi, EngineRuntime, WebEngineViewHandle};

#[repr(C)]
/// ### English
/// Opaque engine handle owning the dedicated Servo thread.
///
/// ### 中文
/// 不透明引擎句柄，持有独立的 Servo 线程。
pub struct XianWebEngine {
    /// ### English
    /// Engine runtime that owns the dedicated Servo thread.
    ///
    /// ### 中文
    /// 引擎运行时，持有独立的 Servo 线程。
    runtime: EngineRuntime,
}

#[repr(C)]
/// ### English
/// Opaque view handle (thread-safe for the embedder to use via pointers).
///
/// ### 中文
/// 不透明 view 句柄（宿主可通过指针线程安全使用）。
pub struct XianWebEngineView {
    /// ### English
    /// Thread-safe handle that sends commands / queues work to the dedicated Servo thread.
    ///
    /// ### 中文
    /// 线程安全句柄：向独立 Servo 线程发送命令/排队工作。
    handle: WebEngineViewHandle,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// ### English
/// One acquired frame returned to the embedder (Java thread).
///
/// ### 中文
/// 返回给宿主（Java 线程）的单个已获取帧。
pub struct XianWebEngineFrame {
    /// ### English
    /// Producer fence handle (`GLsync` cast to `u64`), or 0 if unavailable.
    ///
    /// Wait on this fence before sampling `texture_id` to avoid reading an incomplete frame.
    ///
    /// Ownership: owned by Rust; the embedder may wait on it but must NOT delete it. Rust deletes it
    /// when the corresponding slot is recycled/destroyed.
    ///
    /// ### 中文
    /// 生产者 fence 句柄（`GLsync` 转为 `u64`），不可用则为 0。
    ///
    /// 在采样 `texture_id` 前等待该 fence，以避免读到未完成帧。
    ///
    /// 所有权：由 Rust 持有；宿主可等待它，但不要自行删除。Rust 会在对应槽位复用/销毁时删除它。
    pub producer_fence: u64,
    /// ### English
    /// GL texture ID containing the frame.
    ///
    /// ### 中文
    /// 包含该帧的 GL 纹理 ID。
    pub texture_id: u32,
    /// ### English
    /// Triple-buffer slot index (0..=2).
    ///
    /// The embedder must pass this value back to `xian_web_engine_views_release_frames` to release
    /// the acquired slot.
    ///
    /// ### 中文
    /// 三缓冲槽位索引（0..=2）。
    ///
    /// 宿主必须把该值回传给 `xian_web_engine_views_release_frames`，用于释放已 acquire 的槽位。
    pub slot: u32,
    /// ### English
    /// Frame width in pixels.
    ///
    /// ### 中文
    /// 帧宽度（像素）。
    pub width: u32,
    /// ### English
    /// Frame height in pixels.
    ///
    /// ### 中文
    /// 帧高度（像素）。
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// ### English
/// Engine creation configuration passed via the C ABI.
///
/// All string pointers are optional NUL-terminated UTF-8 C strings; NULL or empty means "unset".
///
/// ### 中文
/// 通过 C ABI 传递的引擎创建配置。
///
/// 所有字符串指针均为可选的 NUL 结尾 UTF-8 C 字符串；NULL 或空字符串表示“不设置”。
pub struct XianWebEngineConfig {
    /// ### English
    /// Embedder-owned GLFW window whose context will be shared with the Servo thread.
    ///
    /// ### 中文
    /// 宿主侧 GLFW window；其上下文会与 Servo 线程共享。
    pub glfw_shared_window: *mut c_void,
    /// ### English
    /// Embedder-provided GLFW function pointer table.
    ///
    /// On Windows builds, all function pointers must be non-zero.
    ///
    /// ### 中文
    /// 宿主提供的 GLFW 函数指针表。
    ///
    /// Windows 构建下，所有函数指针必须非 0。
    pub glfw_api: EmbedderGlfwApi,
    /// ### English
    /// Optional resource directory override (NUL-terminated UTF-8).
    ///
    /// ### 中文
    /// 可选资源目录覆盖（NUL 结尾 UTF-8）。
    pub resources_dir: *const c_char,
    /// ### English
    /// Optional config directory override (NUL-terminated UTF-8).
    ///
    /// ### 中文
    /// 可选配置目录覆盖（NUL 结尾 UTF-8）。
    pub config_dir: *const c_char,
    /// ### English
    /// Default view width in pixels (clamped to >= 1).
    ///
    /// ### 中文
    /// 默认 view 宽度（像素；会 clamp 至 >= 1）。
    pub default_width: u32,
    /// ### English
    /// Default view height in pixels (clamped to >= 1).
    ///
    /// ### 中文
    /// 默认 view 高度（像素；会 clamp 至 >= 1）。
    pub default_height: u32,
    /// ### English
    /// Servo worker thread cap (`0` means no cap).
    ///
    /// ### 中文
    /// Servo 工作线程上限（`0` 表示不封顶）。
    pub thread_pool_cap: u32,
    /// ### English
    /// Engine flags bitmask (see `XIAN_WEB_ENGINE_FLAG_*`).
    ///
    /// ### 中文
    /// 引擎标志位掩码（见 `XIAN_WEB_ENGINE_FLAG_*`）。
    pub engine_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// ### English
/// View creation configuration passed via the C ABI.
///
/// ### 中文
/// 通过 C ABI 传递的 view 创建配置。
pub struct XianWebEngineViewConfig {
    /// ### English
    /// Engine handle returned by `xian_web_engine_create` (must be non-NULL).
    ///
    /// ### 中文
    /// 由 `xian_web_engine_create` 返回的 engine 句柄（必须非 NULL）。
    pub engine: *mut XianWebEngine,
    /// ### English
    /// Initial view width in pixels (0 is treated as engine default; clamped to >= 1).
    ///
    /// ### 中文
    /// 初始 view 宽度（像素；0 表示使用引擎默认值；会 clamp 至 >= 1）。
    pub width: u32,
    /// ### English
    /// Initial view height in pixels (0 is treated as engine default; clamped to >= 1).
    ///
    /// ### 中文
    /// 初始 view 高度（像素；0 表示使用引擎默认值；会 clamp 至 >= 1）。
    pub height: u32,
    /// ### English
    /// Target FPS for fixed-interval refresh (0 means external-vsync mode).
    ///
    /// ### 中文
    /// 固定间隔 refresh 的目标 FPS（0 表示外部 vsync 模式）。
    pub target_fps: u32,
    /// ### English
    /// View flags bitmask (see `XIAN_WEB_ENGINE_VIEW_FLAG_*`).
    ///
    /// ### 中文
    /// view 标志位掩码（见 `XIAN_WEB_ENGINE_VIEW_FLAG_*`）。
    pub view_flags: u32,
}

/// ### English
/// C ABI version for `xian_web_engine`.
///
/// ### 中文
/// `xian_web_engine` 的 C ABI 版本号。
const XIAN_WEB_ENGINE_ABI_VERSION: u32 = 1;

#[unsafe(no_mangle)]
/// ### English
/// Returns the C ABI version.
///
/// ### 中文
/// 返回 C ABI 版本号。
pub extern "C" fn xian_web_engine_abi_version() -> u32 {
    XIAN_WEB_ENGINE_ABI_VERSION
}

impl From<AcquiredFrame> for XianWebEngineFrame {
    /// ### English
    /// Converts an internal `AcquiredFrame` into the C ABI `XianWebEngineFrame`.
    ///
    /// #### Parameters
    /// - `value`: Source frame payload.
    ///
    /// ### 中文
    /// 将内部 `AcquiredFrame` 转换为 C ABI 的 `XianWebEngineFrame`。
    ///
    /// #### 参数
    /// - `value`：源帧数据。
    fn from(value: AcquiredFrame) -> Self {
        Self {
            producer_fence: value.producer_fence,
            texture_id: value.texture_id,
            slot: value.slot as u32,
            width: value.width,
            height: value.height,
        }
    }
}

#[inline]
/// ### English
/// Converts an optional NUL-terminated UTF-8 C string into `&str`.
///
/// Returns `None` for NULL pointers or invalid UTF-8.
///
/// #### Parameters
/// - `ptr`: Optional C string pointer (may be NULL).
///
/// #### Safety
/// `ptr` must be valid and point to a NUL-terminated string for the duration of the call.
///
/// ### 中文
/// 将可选的 NUL 结尾 UTF-8 C 字符串转换为 `&str`。
///
/// 对 NULL 指针或 UTF-8 非法返回 `None`。
///
/// #### 参数
/// - `ptr`：可选 C 字符串指针（允许为 NULL）。
///
/// #### 安全
/// `ptr` 在本次调用期间必须有效，并指向以 NUL 结尾的字符串。
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

/// ### English
/// Converts an optional NUL-terminated UTF-8 C string into a `PathBuf`.
///
/// Returns `None` for NULL pointers, invalid UTF-8, or empty strings.
///
/// #### Parameters
/// - `ptr`: Optional C string pointer (may be NULL).
///
/// #### Safety
/// `ptr` must be valid and point to a NUL-terminated string for the duration of the call.
///
/// ### 中文
/// 将可选的 NUL 结尾 UTF-8 C 字符串转换为 `PathBuf`。
///
/// 对 NULL 指针、UTF-8 非法或空字符串返回 `None`。
///
/// #### 参数
/// - `ptr`：可选 C 字符串指针（允许为 NULL）。
///
/// #### 安全
/// `ptr` 在本次调用期间必须有效，并指向以 NUL 结尾的字符串。
unsafe fn cstr_to_path(ptr: *const c_char) -> Option<PathBuf> {
    let value = unsafe { cstr_to_str(ptr)? };
    if value.is_empty() {
        return None;
    }

    Some(PathBuf::from(value))
}
