//! ### English
//! C ABI surface for `xian_web_engine`.
//!
//! Threading model (for performance):
//! - All functions must be called on the same thread that owns the provided GLFW OpenGL context.
//! - This crate does not spawn a dedicated "Servo thread".
//! - The embedder drives Servo by calling `xian_web_engine_tick(...)` regularly (e.g. once per frame).
//!
//! Rendering model (for performance):
//! - Servo renders into per-view OpenGL textures created in the embedder's context.
//! - No shared/offscreen GLFW window is created.
//!
//! ### 中文
//! `xian_web_engine` 的 C ABI 接口层。
//!
//! 线程模型（为性能而设计）：
//! - 所有函数必须在同一线程调用，并且该线程持有并绑定（current）传入的 GLFW OpenGL 上下文。
//! - 本库不再创建独立的“Servo 线程”。
//! - 宿主通过定期调用 `xian_web_engine_tick(...)`（例如每帧一次）来驱动 Servo。
//!
//! 渲染模型（为性能而设计）：
//! - Servo 渲染到“宿主上下文中创建的、每个 view 独立的 OpenGL 纹理”。
//! - 不会创建共享/离屏的 GLFW window。

use std::ffi::{CStr, c_char, c_void};
use std::path::PathBuf;
use std::str::Utf8Error;
use std::{mem, ptr};

use crate::engine::{ViewCreateParams, XianWebEngineView};
use crate::input::XianWebEngineInputEvent;

/// ### English
/// C ABI version.
///
/// ### 中文
/// C ABI 版本号。
const XIAN_WEB_ENGINE_ABI_VERSION: u32 = 1;

/// ### English
/// OpenGL API kind (desktop OpenGL).
///
/// ### 中文
/// OpenGL API 类型（桌面 OpenGL）。
pub const XIAN_WEB_ENGINE_GL_API_GL: u32 = 1;

/// ### English
/// OpenGL API kind (OpenGL ES).
///
/// ### 中文
/// OpenGL API 类型（OpenGL ES）。
pub const XIAN_WEB_ENGINE_GL_API_GLES: u32 = 2;

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

    /// ### English
    /// `glfwMakeContextCurrent` function pointer (optional when assuming current context).
    ///
    /// Signature (C): `void glfwMakeContextCurrent(GLFWwindow* window)`.
    ///
    /// ### 中文
    /// `glfwMakeContextCurrent` 函数指针（在“假定 current”模式下可选）。
    pub glfw_make_context_current: usize,
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
    /// HiDPI scale factor (`1.0` by default, currently ignored / reserved).
    ///
    /// ### 中文
    /// HiDPI 缩放（默认 `1.0`，当前忽略 / 预留）。
    pub hidpi_scale_factor: f32,

    /// ### English
    /// Optional initial URL (NUL-terminated UTF-8). NULL/empty means "do not load".
    ///
    /// ### 中文
    /// 可选初始 URL（NUL 结尾 UTF-8）。传 NULL/空字符串表示“不加载”。
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

#[unsafe(no_mangle)]
/// ### English
/// Returns the C ABI version.
///
/// #### Returns
/// - C ABI version number.
///
/// ### 中文
/// 返回 C ABI 版本号。
///
/// #### 返回
/// - C ABI 版本号。
pub extern "C" fn xian_web_engine_abi_version() -> u32 {
    XIAN_WEB_ENGINE_ABI_VERSION
}

#[unsafe(no_mangle)]
/// ### English
/// Installs a directory-based Servo `ResourceReader` (process-global).
///
/// This is not tied to any engine instance. Call it once at startup (before Servo is initialized,
/// i.e. before the first successful `xian_web_engine_view_create`) if you want Servo's built-in
/// resources (net error pages, placeholders, etc.) to be loadable.
///
/// #### Parameters
/// - `resources_dir`: NUL-terminated UTF-8 directory path.
///
/// #### Returns
/// - `true` if the path was accepted.
/// - `false` if Servo is already initialized.
///
/// #### Safety
/// - If non-NULL, `resources_dir` must point to a valid NUL-terminated UTF-8 string.
///
/// ### 中文
/// 安装一个基于目录的 Servo `ResourceReader`（进程全局）。
///
/// 该设置不与 engine 实例绑定，建议在应用启动时（Servo 初始化之前，即首次成功调用
/// `xian_web_engine_view_create` 之前）设置，以便 Servo 能正常读取内置资源（网络错误页、占位图等）。
///
/// #### 参数
/// - `resources_dir`：NUL 结尾的 UTF-8 目录路径。
///
/// #### 返回
/// - 路径被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
///
/// #### 安全性
/// - 若非 NULL，`resources_dir` 必须指向有效的 NUL 结尾 UTF-8 字符串。
pub unsafe extern "C" fn xian_web_engine_set_resources_dir(resources_dir: *const c_char) -> bool {
    if crate::engine::is_servo_initialized() {
        return false;
    }
    let Some(path) = (unsafe { cstr_to_path(resources_dir) }) else {
        return false;
    };
    crate::resources::set_resources_dir(path);
    true
}

#[unsafe(no_mangle)]
/// ### English
/// Sets the Servo config directory override (process-global).
///
/// This must be called before Servo is initialized. Passing NULL or empty string clears the
/// override.
///
/// #### Parameters
/// - `config_dir`: NUL-terminated UTF-8 directory path (NULL/empty clears the override).
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// #### Safety
/// - If non-NULL, `config_dir` must point to a valid NUL-terminated UTF-8 string.
///
/// ### 中文
/// 设置 Servo 配置目录覆盖值（进程全局）。
///
/// 必须在 Servo 初始化之前调用。传 NULL/空字符串表示清空该覆盖设置。
///
/// #### 参数
/// - `config_dir`：NUL 结尾的 UTF-8 目录路径（传 NULL/空字符串表示清空覆盖）。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
///
/// #### 安全性
/// - 若非 NULL，`config_dir` 必须指向有效的 NUL 结尾 UTF-8 字符串。
pub unsafe extern "C" fn xian_web_engine_set_config_dir(config_dir: *const c_char) -> bool {
    match unsafe { cstr_to_str_opt(config_dir) } {
        Ok(None) => crate::engine::set_config_dir(None),
        Ok(Some(path)) => crate::engine::set_config_dir(Some(PathBuf::from(path))),
        Err(_) => false,
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Sets the Servo worker thread cap (`0` = no cap, process-global).
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `thread_pool_cap`: Maximum worker threads (`0` means "no cap").
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置 Servo 工作线程上限（`0` = 不限制，进程全局）。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `thread_pool_cap`：工作线程上限（`0` 表示“不限制”）。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub extern "C" fn xian_web_engine_set_thread_pool_cap(thread_pool_cap: u32) -> bool {
    crate::engine::set_thread_pool_cap(thread_pool_cap)
}

#[unsafe(no_mangle)]
/// ### English
/// Sets the embedder-provided GLFW context + proc table (process-global).
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `glfw_window`: Embedder-owned `GLFWwindow*`.
/// - `glfw_api`: Minimal GLFW function table.
///
/// #### Returns
/// - `true` if the values were accepted.
/// - `false` if Servo is already initialized or required pointers are missing.
///
/// #### Threading
/// - All ABI calls must happen on the same thread that owns this OpenGL context.
///
/// #### Safety
/// - `glfw_window` must remain valid for the lifetime of the engine.
/// - `glfw_api` function pointers must match the documented signatures.
///
/// ### 中文
/// 设置宿主提供的 GLFW 上下文与函数表（进程全局）。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `glfw_window`：宿主侧 `GLFWwindow*`。
/// - `glfw_api`：最小 GLFW 函数表。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化或必需指针缺失则返回 `false`。
///
/// #### 线程
/// - 所有 ABI 调用必须发生在拥有该 OpenGL 上下文的同一线程。
///
/// #### 安全性
/// - `glfw_window` 必须在引擎生命周期内保持有效。
/// - `glfw_api` 中的函数指针必须与声明的签名一致。
pub extern "C" fn xian_web_engine_set_glfw_context(
    glfw_window: *mut c_void,
    glfw_api: XianWebEngineGlfwApi,
) -> bool {
    if glfw_window.is_null() {
        return false;
    }
    if glfw_api.glfw_get_proc_address == 0 {
        return false;
    }
    crate::engine::set_glfw_context(
        glfw_window,
        glfw_api.glfw_get_proc_address,
        glfw_api.glfw_make_context_current,
    )
}

#[unsafe(no_mangle)]
/// ### English
/// Selects the OpenGL API kind (`GL` / `GLES`, process-global).
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `gl_api`: One of `XIAN_WEB_ENGINE_GL_API_GL` / `XIAN_WEB_ENGINE_GL_API_GLES`.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if the value is invalid or Servo is already initialized.
///
/// ### 中文
/// 选择 OpenGL API 类型（`GL` / `GLES`，进程全局）。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `gl_api`：`XIAN_WEB_ENGINE_GL_API_GL` / `XIAN_WEB_ENGINE_GL_API_GLES` 之一。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - 值非法或 Servo 已初始化则返回 `false`。
pub extern "C" fn xian_web_engine_set_gl_api(gl_api: u32) -> bool {
    match gl_api {
        XIAN_WEB_ENGINE_GL_API_GL | XIAN_WEB_ENGINE_GL_API_GLES => {
            crate::engine::set_gl_api(gl_api)
        }
        _ => false,
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Sets whether to assume the GLFW context is already current (`0`/`1`, process-global).
///
/// When set to `1` (default), this crate will NOT call `glfwMakeContextCurrent` for maximum
/// performance; the embedder must ensure the context is current before calling into the ABI.
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `assume_context_current`: `0` or non-zero.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置是否假定 GLFW 上下文已为 current（`0`/`1`，进程全局）。
///
/// 设为 `1`（默认）时：本库不会调用 `glfwMakeContextCurrent` 以获得最高性能；宿主必须保证
/// 调用 ABI 之前上下文已 current。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `assume_context_current`：`0` 或非零。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub extern "C" fn xian_web_engine_set_assume_context_current(assume_context_current: u32) -> bool {
    crate::engine::set_assume_context_current(assume_context_current != 0)
}

#[unsafe(no_mangle)]
/// ### English
/// Sets whether to auto-paint dirty views inside `xian_web_engine_tick` (`0`/`1`, process-global).
///
/// When set to `1` (default), `xian_web_engine_tick` will paint all dirty views after driving the
/// Servo event loop.
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `auto_paint`: `0` or non-zero.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置是否在 `xian_web_engine_tick` 内自动绘制 dirty view（`0`/`1`，进程全局）。
///
/// 设为 `1`（默认）时：`xian_web_engine_tick` 在驱动 Servo 事件循环后，会绘制所有 dirty view。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `auto_paint`：`0` 或非零。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub extern "C" fn xian_web_engine_set_auto_paint(auto_paint: u32) -> bool {
    crate::engine::set_auto_paint(auto_paint != 0)
}

#[unsafe(no_mangle)]
/// ### English
/// Returns whether `xian_web_engine_tick` is likely useful (best-effort hint).
///
/// #### Returns
/// - `true` when the engine likely has pending work.
/// - `false` when the engine appears idle or has not been created yet.
///
/// ### 中文
/// 返回 `xian_web_engine_tick` 是否可能有意义（best-effort 提示）。
///
/// #### 返回
/// - 引擎可能存在待处理工作时返回 `true`。
/// - 引擎看起来空闲或尚未创建时返回 `false`。
pub extern "C" fn xian_web_engine_needs_tick() -> bool {
    crate::engine::needs_tick()
}

#[unsafe(no_mangle)]
/// ### English
/// Drives Servo once. When `AUTO_PAINT` is enabled, this also paints all dirty views.
///
/// #### Returns
/// - Number of views painted in this tick.
///
/// #### Threading
/// - Must follow this module's single-thread + current-context contract.
///
/// ### 中文
/// 驱动 Servo 一次；当启用 `AUTO_PAINT` 时，会在本次 tick 内绘制所有 dirty view。
///
/// #### 返回
/// - 本次 tick 绘制的 view 数量。
///
/// #### 线程
/// - 必须遵守本模块的单线程 + 上下文 current 约定。
pub extern "C" fn xian_web_engine_tick() -> u32 {
    crate::engine::tick()
}

#[unsafe(no_mangle)]
/// ### English
/// Fills `config` with defaults.
///
/// #### Parameters
/// - `config`: Output buffer for `XianWebEngineViewConfig`.
///
/// #### Safety
/// - `config` must be non-null, aligned, and writable for `XianWebEngineViewConfig`.
///
/// ### 中文
/// 将 `config` 填充为默认值。
///
/// #### 参数
/// - `config`：用于输出 `XianWebEngineViewConfig` 的缓冲区。
///
/// #### 安全性
/// - `config` 必须非空、对齐且可写（大小至少为 `XianWebEngineViewConfig`）。
pub unsafe extern "C" fn xian_web_engine_view_config_init(config: *mut XianWebEngineViewConfig) {
    if config.is_null() || !is_aligned_ptr(config) {
        return;
    }

    unsafe { config.write(XianWebEngineViewConfig::default()) };
}

#[unsafe(no_mangle)]
/// ### English
/// Creates a new view.
///
/// #### Parameters
/// - `config`: View creation config.
///
/// #### Returns
/// - View pointer on success.
/// - NULL on failure.
///
/// #### Threading
/// - Must follow this module's single-thread + current-context contract.
///
/// #### Safety
/// - `config` must be non-null, aligned, and readable as `XianWebEngineViewConfig`.
///
/// ### 中文
/// 创建一个新 view。
///
/// #### 参数
/// - `config`：view 创建配置。
///
/// #### 返回
/// - 成功返回 view 指针。
/// - 失败返回 NULL。
///
/// #### 线程
/// - 必须遵守本模块的单线程 + 上下文 current 约定。
///
/// #### 安全性
/// - `config` 必须非空、对齐且可读（大小至少为 `XianWebEngineViewConfig`）。
pub unsafe extern "C" fn xian_web_engine_view_create(
    config: *const XianWebEngineViewConfig,
) -> *mut XianWebEngineView {
    let Some(config) = (unsafe { aligned_ref(config) }) else {
        return ptr::null_mut();
    };

    let initial_url =
        unsafe { cstr_to_str(config.initial_url) }.and_then(|url| url::Url::parse(url).ok());

    let params = ViewCreateParams {
        width: config.width,
        height: config.height,
        initial_url,
    };

    match crate::engine::create_view(params) {
        Ok(view) => view.as_ptr(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Destroys a view.
///
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create`.
///
/// #### Safety
/// - `view` must be either NULL, or a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 销毁 view。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针。
///
/// #### 安全性
/// - `view` 必须为 NULL，或由 `xian_web_engine_view_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_view_destroy(view: *mut XianWebEngineView) {
    if view.is_null() || !is_aligned_ptr(view) {
        return;
    }
    let view = unsafe { Box::from_raw(view) };
    XianWebEngineView::destroy_boxed(view);
}

#[unsafe(no_mangle)]
/// ### English
/// Loads a URL into this view.
///
/// #### Parameters
/// - `view`: View pointer.
/// - `url`: NUL-terminated UTF-8 URL string.
///
/// #### Returns
/// - `true` if the URL was accepted.
///
/// #### Safety
/// - `view` must be either NULL, or a valid pointer returned by `xian_web_engine_view_create`.
/// - If non-NULL, `url` must point to a valid NUL-terminated UTF-8 string.
///
/// ### 中文
/// 加载 URL。
///
/// #### 参数
/// - `view`：view 指针。
/// - `url`：NUL 结尾的 UTF-8 URL 字符串。
///
/// #### 返回
/// - 返回 `true` 表示 URL 被接受。
///
/// #### 安全性
/// - `view` 必须为 NULL，或由 `xian_web_engine_view_create` 返回的有效指针。
/// - 若非 NULL，`url` 必须指向有效的 NUL 结尾 UTF-8 字符串。
pub unsafe extern "C" fn xian_web_engine_view_load_url(
    view: *mut XianWebEngineView,
    url: *const c_char,
) -> bool {
    let Some(view) = (unsafe { aligned_ref(view) }) else {
        return false;
    };
    let Some(url) = (unsafe { cstr_to_str(url) }) else {
        return false;
    };
    view.load_url(url)
}

#[unsafe(no_mangle)]
/// ### English
/// Resizes the view.
///
/// #### Parameters
/// - `view`: View pointer.
/// - `width`: New width in pixels (clamped to >= 1).
/// - `height`: New height in pixels (clamped to >= 1).
///
/// #### Safety
/// - `view` must be either NULL, or a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 调整 view 尺寸。
///
/// #### 参数
/// - `view`：view 指针。
/// - `width`：新宽度（像素，最小为 1）。
/// - `height`：新高度（像素，最小为 1）。
///
/// #### 安全性
/// - `view` 必须为 NULL，或由 `xian_web_engine_view_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_view_resize(
    view: *mut XianWebEngineView,
    width: u32,
    height: u32,
) {
    let Some(view) = (unsafe { aligned_ref(view) }) else {
        return;
    };
    view.resize(width, height);
}

#[unsafe(no_mangle)]
/// ### English
/// Returns the OpenGL texture id of this view.
///
/// #### Parameters
/// - `view`: View pointer.
///
/// #### Returns
/// - OpenGL texture id (0 on NULL/invalid).
///
/// #### Safety
/// - `view` must be either NULL, or a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 返回该 view 的 OpenGL 纹理 ID。
///
/// #### 参数
/// - `view`：view 指针。
///
/// #### 返回
/// - OpenGL 纹理 id（NULL/无效时返回 0）。
///
/// #### 安全性
/// - `view` 必须为 NULL，或由 `xian_web_engine_view_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_view_texture_id(view: *const XianWebEngineView) -> u32 {
    let Some(view) = (unsafe { aligned_ref(view) }) else {
        return 0;
    };
    view.texture_id()
}

#[unsafe(no_mangle)]
/// ### English
/// Returns whether this view needs painting.
///
/// #### Parameters
/// - `view`: View pointer.
///
/// #### Returns
/// - `true` if the view is dirty and needs painting.
///
/// #### Safety
/// - `view` must be either NULL, or a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 返回该 view 是否需要绘制。
///
/// #### 参数
/// - `view`：view 指针。
///
/// #### 返回
/// - view 为 dirty 且需要绘制时返回 `true`。
///
/// #### 安全性
/// - `view` 必须为 NULL，或由 `xian_web_engine_view_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_view_needs_paint(view: *const XianWebEngineView) -> bool {
    let Some(view) = (unsafe { aligned_ref(view) }) else {
        return false;
    };
    view.needs_paint()
}

#[unsafe(no_mangle)]
/// ### English
/// Paints this view immediately.
///
/// #### Parameters
/// - `view`: View pointer.
///
/// #### Returns
/// - `true` if a paint was performed.
/// - `false` if the view was not dirty or on NULL/invalid.
///
/// #### Safety
/// - `view` must be either NULL, or a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 立即绘制该 view。
///
/// #### 参数
/// - `view`：view 指针。
///
/// #### 返回
/// - 确实执行了绘制则返回 `true`。
/// - view 非 dirty 或 NULL/无效时返回 `false`。
///
/// #### 安全性
/// - `view` 必须为 NULL，或由 `xian_web_engine_view_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_view_paint(view: *mut XianWebEngineView) -> bool {
    let Some(view) = (unsafe { aligned_ref(view) }) else {
        return false;
    };
    view.paint()
}

#[unsafe(no_mangle)]
/// ### English
/// Sends a batch of input events to the view.
///
/// #### Parameters
/// - `view`: View pointer.
/// - `events`: Pointer to an array of `XianWebEngineInputEvent`.
/// - `count`: Number of elements in `events`.
///
/// #### Returns
/// - Number of events accepted.
///
/// #### Safety
/// - `view` must be either NULL, or a valid pointer returned by `xian_web_engine_view_create`.
/// - If non-NULL, `events` must be aligned and valid for reading `count` elements.
///
/// ### 中文
/// 向 view 发送一批输入事件。
///
/// #### 参数
/// - `view`：view 指针。
/// - `events`：指向 `XianWebEngineInputEvent` 数组的指针。
/// - `count`：`events` 中的元素数量。
///
/// #### 返回
/// - 被接受的事件数量。
///
/// #### 安全性
/// - `view` 必须为 NULL，或由 `xian_web_engine_view_create` 返回的有效指针。
/// - 若非 NULL，`events` 必须对齐且在读取 `count` 个元素时保持有效。
pub unsafe extern "C" fn xian_web_engine_view_send_input_events(
    view: *mut XianWebEngineView,
    events: *const XianWebEngineInputEvent,
    count: u32,
) -> u32 {
    let Some(view) = (unsafe { aligned_ref(view) }) else {
        return 0;
    };
    if events.is_null() || count == 0 {
        return 0;
    }
    if !is_aligned_ptr(events) {
        return 0;
    }

    let events = unsafe { std::slice::from_raw_parts(events, count as usize) };
    view.send_input_events(events)
}

#[inline]
/// ### English
/// Returns whether a raw pointer is aligned for `T`.
///
/// #### Parameters
/// - `ptr`: Raw pointer.
///
/// #### Returns
/// - `true` if aligned for `T`.
///
/// ### 中文
/// 判断原始指针是否满足 `T` 的对齐要求。
///
/// #### 参数
/// - `ptr`：原始指针。
///
/// #### 返回
/// - 满足 `T` 对齐要求则返回 `true`。
fn is_aligned_ptr<T>(ptr: *const T) -> bool {
    let align = mem::align_of::<T>();
    debug_assert!(align.is_power_of_two());
    (ptr as usize) & (align - 1) == 0
}

#[inline]
/// ### English
/// Converts a raw pointer into an aligned shared reference.
///
/// #### Parameters
/// - `ptr`: Raw pointer.
///
/// #### Returns
/// - `Some(&T)` when non-null and aligned.
/// - `None` when NULL or misaligned.
///
/// #### Safety
/// - When returning `Some`, `ptr` must be valid for reads of `T` for the chosen lifetime.
///
/// ### 中文
/// 将原始指针转换为“对齐的共享引用”。
///
/// #### 参数
/// - `ptr`：原始指针。
///
/// #### 返回
/// - 非空且对齐时返回 `Some(&T)`。
/// - 为 NULL 或未对齐时返回 `None`。
///
/// #### 安全性
/// - 返回 `Some` 时，`ptr` 必须在该生命周期内对读取 `T` 有效。
unsafe fn aligned_ref<'a, T>(ptr: *const T) -> Option<&'a T> {
    if ptr.is_null() || !is_aligned_ptr(ptr) {
        return None;
    }
    Some(unsafe { &*ptr })
}

#[inline]
/// ### English
/// Converts a C string pointer into an optional UTF-8 `&str`.
///
/// NULL and empty string become `Ok(None)`; invalid UTF-8 becomes `Err`.
///
/// #### Parameters
/// - `ptr`: NUL-terminated C string pointer.
///
/// #### Returns
/// - `Ok(Some(&str))` for non-empty valid UTF-8.
/// - `Ok(None)` for NULL or empty string.
/// - `Err(Utf8Error)` for invalid UTF-8.
///
/// #### Safety
/// - If non-NULL, `ptr` must point to a valid NUL-terminated string.
///
/// ### 中文
/// 将 C 字符串指针转换为可选的 UTF-8 `&str`。
///
/// NULL 与空字符串会返回 `Ok(None)`；无效 UTF-8 会返回 `Err`。
///
/// #### 参数
/// - `ptr`：NUL 结尾的 C 字符串指针。
///
/// #### 返回
/// - 非空且 UTF-8 有效时返回 `Ok(Some(&str))`。
/// - 为 NULL 或空字符串时返回 `Ok(None)`。
/// - UTF-8 无效时返回 `Err(Utf8Error)`。
///
/// #### 安全性
/// - 若非 NULL，`ptr` 必须指向有效的 NUL 结尾字符串。
unsafe fn cstr_to_str_opt<'a>(ptr: *const c_char) -> Result<Option<&'a str>, Utf8Error> {
    if ptr.is_null() {
        return Ok(None);
    }

    let s = unsafe { CStr::from_ptr(ptr) }.to_str()?;
    if s.is_empty() { Ok(None) } else { Ok(Some(s)) }
}

#[inline]
/// ### English
/// Converts a C string pointer into an optional non-empty UTF-8 `&str`.
///
/// This is a lossy wrapper over `cstr_to_str_opt`: invalid UTF-8 becomes `None`.
///
/// #### Parameters
/// - `ptr`: NUL-terminated C string pointer.
///
/// #### Returns
/// - `Some(&str)` for non-empty valid UTF-8.
/// - `None` for NULL/empty/invalid UTF-8.
///
/// #### Safety
/// - If non-NULL, `ptr` must point to a valid NUL-terminated string.
///
/// ### 中文
/// 将 C 字符串指针转换为“非空”的 UTF-8 `&str`。
///
/// 这是 `cstr_to_str_opt` 的有损封装：无效 UTF-8 会被视为 `None`。
///
/// #### 参数
/// - `ptr`：NUL 结尾的 C 字符串指针。
///
/// #### 返回
/// - 非空且 UTF-8 有效时返回 `Some(&str)`。
/// - NULL/空/UTF-8 无效时返回 `None`。
///
/// #### 安全性
/// - 若非 NULL，`ptr` 必须指向有效的 NUL 结尾字符串。
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    unsafe { cstr_to_str_opt(ptr) }.ok().flatten()
}

#[inline]
/// ### English
/// Converts a C string pointer into an optional `PathBuf`.
///
/// #### Parameters
/// - `ptr`: NUL-terminated UTF-8 path string.
///
/// #### Returns
/// - `Some(PathBuf)` for non-empty valid UTF-8.
/// - `None` for NULL/empty/invalid UTF-8.
///
/// #### Safety
/// - If non-NULL, `ptr` must point to a valid NUL-terminated string.
///
/// ### 中文
/// 将 C 字符串指针转换为可选的 `PathBuf`。
///
/// #### 参数
/// - `ptr`：NUL 结尾的 UTF-8 路径字符串。
///
/// #### 返回
/// - 非空且 UTF-8 有效时返回 `Some(PathBuf)`。
/// - NULL/空/UTF-8 无效时返回 `None`。
///
/// #### 安全性
/// - 若非 NULL，`ptr` 必须指向有效的 NUL 结尾字符串。
unsafe fn cstr_to_path(ptr: *const c_char) -> Option<PathBuf> {
    let s = unsafe { cstr_to_str(ptr)? };
    Some(PathBuf::from(s))
}
