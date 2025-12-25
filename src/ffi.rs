//! ### English
//! C ABI surface for `xian_web_engine`.
//! All exported symbols are `extern "C"` functions; structs are `#[repr(C)]`.
//! Strings passed from Java/Panama must be NUL-terminated UTF-8 (C string); they will be
//! validated as UTF-8 and will be truncated at the first NUL byte.
//!
//! ### 中文
//! `xian_web_engine` 的 C ABI 接口层。
//! 所有导出符号均为 `extern "C"` 函数；结构体使用 `#[repr(C)]`。
//! Java/Panama 传入的字符串必须是以 NUL 结尾的 UTF-8（C 字符串）；Rust 会校验 UTF-8，
//! 且在遇到第一个 NUL 字节处截断。

use std::ffi::{CStr, c_char, c_void};
use std::path::PathBuf;

use dpi::PhysicalSize;

use crate::engine::flags;
use crate::engine::frame::AcquiredFrame;
use crate::engine::input_types::{
    XIAN_WEB_ENGINE_INPUT_KIND_KEY, XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON,
    XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE, XIAN_WEB_ENGINE_INPUT_KIND_WHEEL,
    XianWebEngineInputEvent,
};
use crate::engine::runtime::{EngineRuntime, WebEngineViewHandle};

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
    /// Ownership: this sync object is owned by Rust; the embedder may wait on it, but must NOT
    /// delete it (Rust will delete it when the slot is recycled/destroyed).
    ///
    /// ### 中文
    /// 生产者 fence 句柄（`GLsync` 转为 `u64`），不可用则为 0。
    ///
    /// 所有权：该 sync 对象由 Rust 持有；宿主可等待它，但不要自行删除（Rust 会在槽位复用/销毁时删除）。
    pub fence: u64,
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
    /// ### English
    /// Monotonic frame sequence number (wraps; 0 is reserved).
    ///
    /// ### 中文
    /// 单调递增帧序号（会回绕；0 保留不用）。
    pub frame_seq: u64,
}

/// ### English
/// C ABI version for `xian_web_engine`.
///
/// ### 中文
/// `xian_web_engine` 的 C ABI 版本号。
const XIAN_WEB_ENGINE_ABI_VERSION: u32 = 3;

#[unsafe(no_mangle)]
/// ### English
/// Returns the C ABI version.
///
/// ### 中文
/// 返回 C ABI 版本号。
pub extern "C" fn xian_web_engine_abi_version() -> u32 {
    XIAN_WEB_ENGINE_ABI_VERSION
}

#[unsafe(no_mangle)]
/// ### English
/// Returns `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE`.
/// (Panama-friendly constant getter; avoids relying on C headers.)
///
/// ### 中文
/// 返回 `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE`。
/// （Panama 友好的常量获取函数；避免依赖 C 头文件。）
pub extern "C" fn xian_web_engine_view_create_flag_unsafe_no_consumer_fence() -> u32 {
    flags::XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE
}

#[unsafe(no_mangle)]
/// ### English
/// Returns `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_INPUT_SINGLE_PRODUCER`.
/// (Panama-friendly constant getter; avoids relying on C headers.)
///
/// ### 中文
/// 返回 `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_INPUT_SINGLE_PRODUCER`。
/// （Panama 友好的常量获取函数；避免依赖 C 头文件。）
pub extern "C" fn xian_web_engine_view_create_flag_input_single_producer() -> u32 {
    flags::XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_INPUT_SINGLE_PRODUCER
}

#[unsafe(no_mangle)]
/// ### English
/// Returns `XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE`.
///
/// ### 中文
/// 返回 `XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE`。
pub extern "C" fn xian_web_engine_input_kind_mouse_move() -> u32 {
    XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE
}

#[unsafe(no_mangle)]
/// ### English
/// Returns `XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON`.
///
/// ### 中文
/// 返回 `XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON`。
pub extern "C" fn xian_web_engine_input_kind_mouse_button() -> u32 {
    XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON
}

#[unsafe(no_mangle)]
/// ### English
/// Returns `XIAN_WEB_ENGINE_INPUT_KIND_WHEEL`.
///
/// ### 中文
/// 返回 `XIAN_WEB_ENGINE_INPUT_KIND_WHEEL`。
pub extern "C" fn xian_web_engine_input_kind_wheel() -> u32 {
    XIAN_WEB_ENGINE_INPUT_KIND_WHEEL
}

#[unsafe(no_mangle)]
/// ### English
/// Returns `XIAN_WEB_ENGINE_INPUT_KIND_KEY`.
///
/// ### 中文
/// 返回 `XIAN_WEB_ENGINE_INPUT_KIND_KEY`。
pub extern "C" fn xian_web_engine_input_kind_key() -> u32 {
    XIAN_WEB_ENGINE_INPUT_KIND_KEY
}

impl From<AcquiredFrame> for XianWebEngineFrame {
    fn from(value: AcquiredFrame) -> Self {
        Self {
            slot: value.slot as u32,
            texture_id: value.texture_id,
            fence: value.producer_fence,
            width: value.width,
            height: value.height,
            frame_seq: value.frame_seq,
        }
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Creates an engine bound to a Java-created GLFW OpenGL context.
///
/// `resources_dir` and `config_dir` are optional NUL-terminated UTF-8 strings.
/// Passing NULL or an empty string means "unset".
///
/// ### 中文
/// 基于 Java 创建的 GLFW OpenGL 上下文创建引擎。
///
/// `resources_dir` 与 `config_dir` 为可选的 NUL 结尾 UTF-8 字符串；
/// 传入 NULL 或空字符串表示“不设置”。
pub extern "C" fn xian_web_engine_create_glfw_shared_context(
    glfw_shared_window: *mut c_void,
    default_width: u32,
    default_height: u32,
    resources_dir: *const c_char,
    config_dir: *const c_char,
) -> *mut XianWebEngine {
    if glfw_shared_window.is_null() {
        return std::ptr::null_mut();
    }

    let default_size = PhysicalSize::new(default_width.max(1), default_height.max(1));

    let resources_dir = unsafe { cstr_to_path(resources_dir) };
    let config_dir = unsafe { cstr_to_path(config_dir) };

    let Ok(runtime) = EngineRuntime::new_glfw_shared_context(
        glfw_shared_window,
        default_size,
        resources_dir,
        config_dir,
    ) else {
        return std::ptr::null_mut();
    };

    Box::into_raw(Box::new(XianWebEngine { runtime }))
}

#[unsafe(no_mangle)]
/// ### English
/// Destroys an engine created by `xian_web_engine_create_glfw_shared_context`.
///
/// ### 中文
/// 销毁由 `xian_web_engine_create_glfw_shared_context` 创建的引擎。
pub unsafe extern "C" fn xian_web_engine_destroy(engine: *mut XianWebEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(engine));
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Drains pending vsync callbacks (Java-driven refresh).
///
/// ### 中文
/// 执行待处理的 vsync 回调（由 Java 驱动 refresh）。
pub unsafe extern "C" fn xian_web_engine_vsync_tick(engine: *mut XianWebEngine) {
    if engine.is_null() {
        return;
    }

    unsafe { (*engine).runtime.vsync_tick() };
}

#[unsafe(no_mangle)]
/// ### English
/// Convenience API: runs `vsync_tick` and then tries to acquire frames for a batch of views.
///
/// ### 中文
/// 便捷 API：先执行 `vsync_tick`，再批量尝试为多个 view acquire 帧。
pub unsafe extern "C" fn xian_web_engine_tick_and_acquire_view_frames(
    engine: *mut XianWebEngine,
    web_engine_views: *const *mut XianWebEngineView,
    last_seqs: *const u64,
    out_frames: *mut XianWebEngineFrame,
    count: u32,
) -> u32 {
    if engine.is_null() || web_engine_views.is_null() || last_seqs.is_null() || out_frames.is_null()
    {
        return 0;
    }

    unsafe { (*engine).runtime.vsync_tick() };

    let count = count as usize;

    unsafe { std::ptr::write_bytes(out_frames, 0, count) };
    let view_ptrs = unsafe { std::slice::from_raw_parts(web_engine_views, count) };
    let last_seq_values = unsafe { std::slice::from_raw_parts(last_seqs, count) };
    let frames_out = unsafe { std::slice::from_raw_parts_mut(out_frames, count) };

    let mut acquired = 0u32;
    for i in 0..count {
        let view_ptr = view_ptrs[i];
        if view_ptr.is_null() {
            continue;
        }

        let view_handle = unsafe { &(*view_ptr).handle };
        let last_seq = last_seq_values[i];

        let frame = if last_seq == 0 {
            view_handle.acquire_frame()
        } else {
            view_handle.acquire_frame_if_newer(last_seq)
        };

        if let Some(frame) = frame {
            frames_out[i] = frame.into();
            acquired += 1;
        }
    }

    acquired
}

#[unsafe(no_mangle)]
/// ### English
/// Creates one view.
///
/// `target_fps = 0` means the view is driven by external vsync (`xian_web_engine_vsync_tick`).
///
/// ### 中文
/// 创建一个 view。
///
/// `target_fps = 0` 表示由外部 vsync（`xian_web_engine_vsync_tick`）驱动。
pub unsafe extern "C" fn xian_web_engine_create_view(
    engine: *mut XianWebEngine,
    width: u32,
    height: u32,
    target_fps: u32,
    flags: u32,
) -> *mut XianWebEngineView {
    if engine.is_null() {
        return std::ptr::null_mut();
    }

    let size = PhysicalSize::new(width, height);
    let handle = unsafe {
        (*engine)
            .runtime
            .create_view_with_target_fps(size, target_fps, flags)
    };
    let Ok(handle) = handle else {
        return std::ptr::null_mut();
    };

    Box::into_raw(Box::new(XianWebEngineView { handle }))
}

#[unsafe(no_mangle)]
/// ### English
/// Destroys a view created by `xian_web_engine_create_view`.
///
/// ### 中文
/// 销毁由 `xian_web_engine_create_view` 创建的 view。
pub unsafe extern "C" fn xian_web_engine_destroy_view(view: *mut XianWebEngineView) {
    if view.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(view));
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Sets whether the view is active (active views render and accept input).
///
/// ### 中文
/// 设置 view 是否 active（active 的 view 才会渲染并接收输入）。
pub unsafe extern "C" fn xian_web_engine_set_view_active(view: *mut XianWebEngineView, active: u8) {
    if view.is_null() {
        return;
    }

    unsafe { (*view).handle.set_active(active != 0) };
}

#[unsafe(no_mangle)]
/// ### English
/// Requests navigation to the given URL.
///
/// The URL must be a NUL-terminated UTF-8 string.
///
/// ### 中文
/// 请求跳转到指定 URL。
///
/// URL 必须是 NUL 结尾的 UTF-8 字符串。
pub unsafe extern "C" fn xian_web_engine_load_view_url(
    view: *mut XianWebEngineView,
    url: *const c_char,
) -> bool {
    if view.is_null() || url.is_null() {
        return false;
    }

    let url_str = match unsafe { CStr::from_ptr(url) }.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let parsed_url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return false,
    };

    unsafe { (*view).handle.load_url(parsed_url) };
    true
}

#[unsafe(no_mangle)]
/// ### English
/// Requests a resize (in pixels).
///
/// ### 中文
/// 请求 resize（单位：像素）。
pub unsafe extern "C" fn xian_web_engine_resize_view(
    view: *mut XianWebEngineView,
    width: u32,
    height: u32,
) {
    if view.is_null() {
        return;
    }

    unsafe {
        (*view)
            .handle
            .resize(PhysicalSize::new(width.max(1), height.max(1)))
    };
}

#[unsafe(no_mangle)]
/// ### English
/// Tries to acquire the latest READY frame for a view.
/// If `last_seq != 0`, only returns a frame if it's newer than `last_seq`.
///
/// ### 中文
/// 尝试获取 view 最新的 READY 帧。
/// 若 `last_seq != 0`，仅当存在更新帧时才会返回。
pub unsafe extern "C" fn xian_web_engine_acquire_view_frame(
    view: *mut XianWebEngineView,
    last_seq: u64,
    out_frame: *mut XianWebEngineFrame,
) -> bool {
    if view.is_null() || out_frame.is_null() {
        return false;
    }

    let view_handle = unsafe { &(*view).handle };
    let frame = if last_seq == 0 {
        view_handle.acquire_frame()
    } else {
        view_handle.acquire_frame_if_newer(last_seq)
    };
    let Some(frame) = frame else {
        return false;
    };

    unsafe {
        *out_frame = frame.into();
    }
    true
}

#[unsafe(no_mangle)]
/// ### English
/// Releases a previously acquired frame slot.
///
/// If `consumer_fence != 0`, it must be a `GLsync` created by the embedder *after* sampling the
/// texture. Ownership transfers to Rust and the embedder must NOT delete it; Rust will delete it
/// after the producer sees it signaled.
///
/// If the view was created with `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE`, then
/// `consumer_fence` is ignored and MUST be `0` (the embedder must not transfer fence ownership).
///
/// ### 中文
/// 释放之前 acquire 的帧槽位。
///
/// 若 `consumer_fence != 0`，它必须是宿主在采样纹理完成后创建的 `GLsync`。
/// 所有权会转移给 Rust，宿主不要自行删除；Rust 会在生产者确认其已 signal 后删除它。
///
/// 若 view 创建时指定了 `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE`，则
/// `consumer_fence` 会被忽略且必须为 `0`（宿主不要转移 fence 所有权）。
pub unsafe extern "C" fn xian_web_engine_release_view_frame(
    view: *mut XianWebEngineView,
    slot: u32,
    consumer_fence: u64,
) {
    if view.is_null() {
        return;
    }

    unsafe { (*view).handle.release_slot_with_fence(slot, consumer_fence) };
}

#[unsafe(no_mangle)]
/// ### English
/// Releases a batch of frame slots for multiple views.
///
/// If `consumer_fences` is NULL, all fences are treated as 0.
/// If non-NULL, each fence follows the same ownership rule as `xian_web_engine_release_view_frame`.
///
/// If a view was created with `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE`, its
/// corresponding consumer fence MUST be 0 (ignored).
///
/// ### 中文
/// 批量释放多个 view 的帧槽位。
///
/// 若 `consumer_fences` 为 NULL，则所有 fence 视为 0。
/// 若非 NULL，则每个 fence 的所有权规则与 `xian_web_engine_release_view_frame` 相同。
///
/// 若某个 view 创建时指定了 `XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE`，则该 view
/// 对应的 consumer fence 必须为 0（会被忽略）。
pub unsafe extern "C" fn xian_web_engine_release_view_frames(
    engine: *mut XianWebEngine,
    web_engine_views: *const *mut XianWebEngineView,
    slots: *const u32,
    consumer_fences: *const u64,
    count: u32,
) {
    if engine.is_null() || web_engine_views.is_null() || slots.is_null() {
        return;
    }

    let count = count as usize;
    let view_ptrs = unsafe { std::slice::from_raw_parts(web_engine_views, count) };
    let slot_values = unsafe { std::slice::from_raw_parts(slots, count) };

    if consumer_fences.is_null() {
        for i in 0..count {
            let view = view_ptrs[i];
            if view.is_null() {
                continue;
            }

            unsafe { (*view).handle.release_slot_with_fence(slot_values[i], 0) };
        }
        return;
    }

    let consumer_fence_values = unsafe { std::slice::from_raw_parts(consumer_fences, count) };
    for i in 0..count {
        let view = view_ptrs[i];
        if view.is_null() {
            continue;
        }

        unsafe {
            (*view)
                .handle
                .release_slot_with_fence(slot_values[i], consumer_fence_values[i])
        };
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Sends a batch of input events to a view.
///
/// Returns the number of accepted events (may be less than `count` if the queue is full).
/// If the view is inactive, events are treated as accepted and dropped (fast path).
///
/// ### 中文
/// 向 view 发送一批输入事件。
///
/// 返回实际接收的事件数量（若队列满，可能小于 `count`）。
/// 若 view 处于 inactive，则会把事件视为“已接收”并直接丢弃（快路径）。
pub unsafe extern "C" fn xian_web_engine_send_view_input_events(
    view: *mut XianWebEngineView,
    events: *const XianWebEngineInputEvent,
    count: u32,
) -> u32 {
    if view.is_null() || events.is_null() || count == 0 {
        return 0;
    }

    let handle = unsafe { &(*view).handle };

    if !handle.is_active() {
        return count;
    }

    let mut accepted: u32 = 0;
    let mut wake_needed = false;
    let mut input_pending = false;
    let mut last_mouse_move: Option<(f32, f32)> = None;

    let count = count as usize;
    let event_slice = unsafe { std::slice::from_raw_parts(events, count) };
    for &ev in event_slice {
        match ev.kind {
            XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE => {
                last_mouse_move = Some((ev.x, ev.y));
                accepted += 1;
            }
            XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON => {
                if !handle.try_enqueue_input_event(ev) {
                    break;
                }
                input_pending = true;
                accepted += 1;
            }
            XIAN_WEB_ENGINE_INPUT_KIND_WHEEL => {
                if !handle.try_enqueue_input_event(ev) {
                    break;
                }
                input_pending = true;
                accepted += 1;
            }
            XIAN_WEB_ENGINE_INPUT_KIND_KEY => {
                if !handle.try_enqueue_input_event(ev) {
                    break;
                }
                input_pending = true;
                accepted += 1;
            }
            _ => {}
        }
    }

    if let Some((x, y)) = last_mouse_move {
        wake_needed |= handle.queue_mouse_move(x, y);
    }

    if input_pending && handle.notify_input_pending() {
        wake_needed = true;
    }

    if wake_needed {
        handle.wake();
    }

    accepted
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
