//! ### English
//! C ABI bindings for view lifecycle and view-level requests.
//!
//! ### 中文
//! view 生命周期与 view 级别请求的 C ABI 绑定。

use std::ffi::{CStr, c_char};

use dpi::PhysicalSize;

use super::{XianWebEngine, XianWebEngineView};

#[unsafe(no_mangle)]
/// ### English
/// Creates one view.
///
/// `target_fps = 0` means the view is driven by external vsync (`xian_web_engine_tick`).
///
/// ### 中文
/// 创建一个 view。
///
/// `target_fps = 0` 表示由外部 vsync（`xian_web_engine_tick`）驱动。
pub unsafe extern "C" fn xian_web_engine_view_create(
    engine: *mut XianWebEngine,
    width: u32,
    height: u32,
    target_fps: u32,
    view_flags: u32,
) -> *mut XianWebEngineView {
    if engine.is_null() {
        return std::ptr::null_mut();
    }

    let size = PhysicalSize::new(width, height);
    let handle = unsafe { (*engine).runtime.create_view(size, target_fps, view_flags) };
    let Ok(handle) = handle else {
        return std::ptr::null_mut();
    };

    Box::into_raw(Box::new(XianWebEngineView { handle }))
}

#[unsafe(no_mangle)]
/// ### English
/// Destroys a view created by `xian_web_engine_view_create`.
///
/// The caller must ensure there are no outstanding acquired frames, and must not sample any textures
/// from this view after destruction.
///
/// ### 中文
/// 销毁由 `xian_web_engine_view_create` 创建的 view。
///
/// 宿主必须确保没有未释放的 acquired frame，并且 destroy 之后不再采样该 view 的纹理。
pub unsafe extern "C" fn xian_web_engine_view_destroy(view: *mut XianWebEngineView) {
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
pub unsafe extern "C" fn xian_web_engine_view_set_active(view: *mut XianWebEngineView, active: u8) {
    if view.is_null() {
        return;
    }

    let handle = unsafe { &(*view).handle };
    if handle.set_active(active != 0) {
        handle.wake();
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Requests navigation to the given URL.
///
/// The URL must be a NUL-terminated UTF-8 string.
///
/// Return value:
/// - `false` if `view`/`url` is NULL or the string is not valid UTF-8.
/// - `true` otherwise (the request is recorded and coalesced; URL parsing happens on the Servo thread).
///
/// ### 中文
/// 请求跳转到指定 URL。
///
/// URL 必须是 NUL 结尾的 UTF-8 字符串。
///
/// 返回值：
/// - 当 `view`/`url` 为空指针，或字符串不是合法 UTF-8 时返回 `false`。
/// - 其它情况返回 `true`（请求会被记录并合并；URL 解析在 Servo 线程进行）。
pub unsafe extern "C" fn xian_web_engine_view_load_url(
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

    let handle = unsafe { &(*view).handle };
    if handle.load_url(url_str) {
        handle.wake();
    }
    true
}

#[unsafe(no_mangle)]
/// ### English
/// Requests a resize (in pixels).
///
/// This call is coalesced: only the latest size is kept until the Servo thread drains it.
///
/// ### 中文
/// 请求 resize（单位：像素）。
///
/// 该调用会被合并：只保留最新尺寸，等待 Servo 线程 drain。
pub unsafe extern "C" fn xian_web_engine_view_resize(
    view: *mut XianWebEngineView,
    width: u32,
    height: u32,
) {
    if view.is_null() {
        return;
    }

    let handle = unsafe { &(*view).handle };
    if handle.queue_resize(PhysicalSize::new(width.max(1), height.max(1))) {
        handle.wake();
    }
}
