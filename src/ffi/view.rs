//! ### English
//! C ABI bindings for view lifecycle and view-level requests.
//!
//! ### 中文
//! view 生命周期与 view 级别请求的 C ABI 绑定。

use std::ffi::c_char;
use std::ptr;

use dpi::PhysicalSize;

use super::{XianWebEngineView, XianWebEngineViewConfig};

#[unsafe(no_mangle)]
/// ### English
/// Creates one view using a config struct.
///
/// Return value: NULL on failure.
///
/// #### Parameters
/// - `config`: View configuration (must not be NULL).
///
/// #### Safety
/// - `config` must be valid for reads of at least `sizeof(XianWebEngineViewConfig)` bytes.
/// - The config header must have a compatible `struct_size` and the correct ABI version.
///
/// ### 中文
/// 使用配置结构体创建一个 view。
///
/// 返回值：失败返回 NULL。
///
/// #### 参数
/// - `config`：view 配置（必须非 NULL）。
///
/// #### 安全
/// - `config` 必须至少可读 `sizeof(XianWebEngineViewConfig)` 字节。
/// - 配置头部的 `struct_size` 必须兼容，且 ABI 版本必须匹配。
pub unsafe extern "C" fn xian_web_engine_view_create(
    config: *const XianWebEngineViewConfig,
) -> *mut XianWebEngineView {
    let Some(config) = (unsafe { super::read_abi_struct(config) }) else {
        return ptr::null_mut();
    };
    let engine = config.engine;
    if engine.is_null() {
        return ptr::null_mut();
    }

    let size = PhysicalSize::new(config.width, config.height);
    let handle = unsafe {
        (*engine)
            .runtime
            .create_view(size, config.target_fps, config.view_flags)
    };
    let Ok(handle) = handle else {
        return ptr::null_mut();
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
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create` (may be NULL).
///
/// #### Safety
/// If non-NULL, `view` must be a valid pointer returned by `xian_web_engine_view_create`, and must
/// not be used after this call returns.
///
/// ### 中文
/// 销毁由 `xian_web_engine_view_create` 创建的 view。
///
/// 宿主必须确保没有未释放的 acquired frame，并且 destroy 之后不再采样该 view 的纹理。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针（允许为 NULL）。
///
/// #### 安全
/// 若 `view` 非 NULL，则它必须是由 `xian_web_engine_view_create` 返回的有效指针，且本次调用返回后不得再使用。
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
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create` (may be NULL).
/// - `active`: `0` = inactive, non-zero = active.
///
/// #### Safety
/// If non-NULL, `view` must be a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 设置 view 是否 active（active 的 view 才会渲染并接收输入）。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针（允许为 NULL）。
/// - `active`：`0` = inactive，非 0 = active。
///
/// #### 安全
/// 若 `view` 非 NULL，则它必须是由 `xian_web_engine_view_create` 返回的有效指针。
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
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create` (must not be NULL).
/// - `url`: NUL-terminated UTF-8 C string (must not be NULL).
///
/// #### Safety
/// - `view` must be a valid pointer returned by `xian_web_engine_view_create`.
/// - `url` must be valid and point to a NUL-terminated string for the duration of the call.
///
/// ### 中文
/// 请求跳转到指定 URL。
///
/// URL 必须是 NUL 结尾的 UTF-8 字符串。
///
/// 返回值：
/// - 当 `view`/`url` 为空指针，或字符串不是合法 UTF-8 时返回 `false`。
/// - 其它情况返回 `true`（请求会被记录并合并；URL 解析在 Servo 线程进行）。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针（必须非 NULL）。
/// - `url`：NUL 结尾 UTF-8 C 字符串（必须非 NULL）。
///
/// #### 安全
/// - `view` 必须是由 `xian_web_engine_view_create` 返回的有效指针。
/// - `url` 在本次调用期间必须有效，并指向以 NUL 结尾的字符串。
pub unsafe extern "C" fn xian_web_engine_view_load_url(
    view: *mut XianWebEngineView,
    url: *const c_char,
) -> bool {
    if view.is_null() || url.is_null() {
        return false;
    }

    let Some(url_str) = (unsafe { super::cstr_to_str(url) }) else {
        return false;
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
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create` (may be NULL).
/// - `width`: New width in pixels (0 is treated as 1).
/// - `height`: New height in pixels (0 is treated as 1).
///
/// #### Safety
/// If non-NULL, `view` must be a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 请求 resize（单位：像素）。
///
/// 该调用会被合并：只保留最新尺寸，等待 Servo 线程 drain。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针（允许为 NULL）。
/// - `width`：新的宽度（像素；0 会被视为 1）。
/// - `height`：新的高度（像素；0 会被视为 1）。
///
/// #### 安全
/// 若 `view` 非 NULL，则它必须是由 `xian_web_engine_view_create` 返回的有效指针。
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
