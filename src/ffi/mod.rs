/// ### English
/// C ABI surface for `xian_web_engine`.
/// All exported symbols are `extern "C"` functions; structs are `#[repr(C)]`.
/// Strings passed from Java/Panama must be NUL-terminated UTF-8 (C string); they will be
/// validated as UTF-8 and will be truncated at the first NUL byte.
///
/// ### 中文
/// `xian_web_engine` 的 C ABI 接口层。
/// 所有导出符号均为 `extern "C"` 函数；结构体使用 `#[repr(C)]`。
/// Java/Panama 传入的字符串必须是以 NUL 结尾的 UTF-8（C 字符串）；Rust 会校验 UTF-8，
/// 且在遇到第一个 NUL 字节处截断。
mod abi;
mod engine;
mod frame;
mod glfw;
mod input;
mod view;

use std::ffi::{CStr, c_char};
use std::path::PathBuf;

use crate::engine::{AcquiredFrame, EngineRuntime, WebEngineViewHandle};

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
    /// Thread-safe handle that sends commands / enqueues work to the dedicated Servo thread.
    ///
    /// ### 中文
    /// 线程安全句柄：向独立 Servo 线程发送命令/入队工作。
    handle: WebEngineViewHandle,
}

#[repr(C)]
/// ### English
/// One acquired frame returned to the embedder (Java thread).
///
/// ### 中文
/// 返回给宿主（Java 线程）的单个已获取帧。
pub struct XianWebEngineFrame {
    /// ### English
    /// Triple-buffer slot index (0..=2).
    ///
    /// ### 中文
    /// 三缓冲槽位索引（0..=2）。
    pub slot: u32,
    /// ### English
    /// GL texture ID containing the frame.
    ///
    /// ### 中文
    /// 包含该帧的 GL 纹理 ID。
    pub texture_id: u32,
    /// ### English
    /// Producer fence handle (`GLsync` cast to `u64`), or 0 if unavailable.
    ///
    /// The embedder should wait on this fence before sampling the texture to avoid reading an
    /// incomplete frame. If it is `0`, the embedder must provide its own synchronization if needed.
    ///
    /// Ownership: this sync object is owned by Rust; the embedder may wait on it, but must NOT
    /// delete it (Rust will delete it when the slot is recycled/destroyed).
    ///
    /// ### 中文
    /// 生产者 fence 句柄（`GLsync` 转为 `u64`），不可用则为 0。
    ///
    /// 宿主在采样该纹理前应等待该 fence，以避免读到未完成帧；若该值为 `0`，则宿主需自行保证同步。
    ///
    /// 所有权：该 sync 对象由 Rust 持有；宿主可等待它，但不要自行删除（Rust 会在槽位复用/销毁时删除）。
    pub producer_fence: u64,
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

/// ### English
/// C ABI version for `xian_web_engine`.
///
/// ### 中文
/// `xian_web_engine` 的 C ABI 版本号。
const XIAN_WEB_ENGINE_ABI_VERSION: u32 = 1;

impl From<AcquiredFrame> for XianWebEngineFrame {
    fn from(value: AcquiredFrame) -> Self {
        Self {
            slot: value.slot as u32,
            texture_id: value.texture_id,
            producer_fence: value.producer_fence,
            width: value.width,
            height: value.height,
        }
    }
}

unsafe fn cstr_to_path(ptr: *const c_char) -> Option<PathBuf> {
    if ptr.is_null() {
        return None;
    }

    let value = unsafe { CStr::from_ptr(ptr) }.to_str().ok()?;
    if value.is_empty() {
        return None;
    }

    Some(PathBuf::from(value))
}
