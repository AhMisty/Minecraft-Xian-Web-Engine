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

use crate::engine::{EngineCreateParams, ViewCreateParams, XianWebEngine, XianWebEngineView};
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
/// Engine creation configuration.
///
/// ### 中文
/// 引擎创建配置。
pub struct XianWebEngineConfig {
    /// ### English
    /// Pointer to the embedder-owned `GLFWwindow*` whose OpenGL context will be used by Servo.
    ///
    /// ### 中文
    /// 宿主侧 `GLFWwindow*` 指针（其 OpenGL 上下文将被 Servo 使用）。
    pub glfw_window: *mut c_void,

    /// ### English
    /// Minimal GLFW function table.
    ///
    /// ### 中文
    /// 最小 GLFW 函数表。
    pub glfw_api: XianWebEngineGlfwApi,

    /// ### English
    /// OpenGL API kind: `XIAN_WEB_ENGINE_GL_API_GL` / `XIAN_WEB_ENGINE_GL_API_GLES`.
    ///
    /// ### 中文
    /// OpenGL API 类型：`XIAN_WEB_ENGINE_GL_API_GL` / `XIAN_WEB_ENGINE_GL_API_GLES`。
    pub gl_api: u32,

    /// ### English
    /// Whether to assume the GLFW context is already current on the calling thread (`0`/`1`).
    ///
    /// When set to `1` (default), this crate will NOT call `glfwMakeContextCurrent` for maximum
    /// performance. The embedder must ensure the context is current before calling into the ABI.
    ///
    /// When set to `0`, the embedder must provide `glfw_make_context_current`, and this crate may
    /// call it when Servo asks to make the context current.
    ///
    /// ### 中文
    /// 是否假定 GLFW 上下文在调用线程上已经是 current（`0`/`1`）。
    ///
    /// 设为 `1`（默认）时：本库不会调用 `glfwMakeContextCurrent`，以获得最高性能；宿主必须保证
    /// 调用 ABI 之前上下文已 current。
    ///
    /// 设为 `0` 时：宿主必须提供 `glfw_make_context_current`，本库会在 Servo 需要时调用它。
    ///
    /// 注意：使用 `uint32_t` 而不是 `bool`，可避免不同语言/ABI 的对齐差异。
    pub assume_context_current: u32,

    /// ### English
    /// Whether to automatically paint dirty views inside `xian_web_engine_tick` (`0`/`1`).
    ///
    /// ### 中文
    /// 是否在 `xian_web_engine_tick` 内自动绘制 dirty 的 view（`0`/`1`）。
    ///
    /// 注意：使用 `uint32_t` 而不是 `bool`，可避免不同语言/ABI 的对齐差异。
    pub auto_paint: u32,

    /// ### English
    /// Reserved (must be 0). Keeps the struct 8-byte aligned without implicit padding.
    ///
    /// ### 中文
    /// 保留字段（必须为 0）。用于保持结构体 8 字节对齐并避免隐式 padding。
    pub _reserved0: u32,
}

impl Default for XianWebEngineConfig {
    fn default() -> Self {
        Self {
            glfw_window: ptr::null_mut(),
            glfw_api: XianWebEngineGlfwApi::default(),
            gl_api: XIAN_WEB_ENGINE_GL_API_GL,
            assume_context_current: 1,
            auto_paint: 1,
            _reserved0: 0,
        }
    }
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
    /// Owning engine.
    ///
    /// ### 中文
    /// 所属引擎。
    pub engine: *mut XianWebEngine,

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
    fn default() -> Self {
        Self {
            engine: ptr::null_mut(),
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
/// Fills `config` with defaults.
///
/// #### Parameters
/// - `config`: Output buffer for `XianWebEngineConfig`.
///
/// #### Safety
/// - `config` must be non-null, aligned, and writable for `XianWebEngineConfig`.
///
/// ### 中文
/// 将 `config` 填充为默认值。
///
/// #### 参数
/// - `config`：用于输出 `XianWebEngineConfig` 的缓冲区。
///
/// #### 安全性
/// - `config` 必须非空、对齐且可写（大小至少为 `XianWebEngineConfig`）。
pub unsafe extern "C" fn xian_web_engine_config_init(config: *mut XianWebEngineConfig) {
    if config.is_null() || !is_aligned_ptr(config) {
        return;
    }

    unsafe { config.write(XianWebEngineConfig::default()) };
}

#[unsafe(no_mangle)]
/// ### English
/// Installs a directory-based Servo `ResourceReader` (process-global).
///
/// This is not tied to any engine instance. Call it once at startup (before creating an engine)
/// if you want Servo's built-in resources (net error pages, placeholders, etc.) to be loadable.
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
/// 该设置不与 engine 实例绑定，建议在应用启动时（创建 engine 之前）调用，
/// 以便 Servo 能正常读取内置资源（网络错误页、占位图等）。
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
    if crate::engine::is_initialized() {
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
/// This must be called before creating an engine. Passing NULL or empty string clears the
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
/// 必须在创建 engine 之前调用。传 NULL/空字符串表示清空该覆盖设置。
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
/// This must be called before creating an engine.
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
/// 必须在创建 engine 之前调用。
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
/// Creates an engine instance.
///
/// #### Parameters
/// - `config`: Engine creation config.
///
/// #### Returns
/// - Engine pointer on success.
/// - NULL on failure.
///
/// #### Threading
/// - Must follow this module's single-thread + current-context contract.
///
/// #### Safety
/// - `config` must be non-null, aligned, and readable as `XianWebEngineConfig`.
///
/// ### 中文
/// 创建引擎实例。
///
/// #### 参数
/// - `config`：引擎创建配置。
///
/// #### 返回
/// - 成功返回 engine 指针。
/// - 失败返回 NULL。
///
/// #### 线程
/// - 必须遵守本模块的单线程 + 上下文 current 约定。
///
/// #### 安全性
/// - `config` 必须非空、对齐且可读（大小至少为 `XianWebEngineConfig`）。
pub unsafe extern "C" fn xian_web_engine_create(
    config: *const XianWebEngineConfig,
) -> *mut XianWebEngine {
    let Some(config) = (unsafe { aligned_ref(config) }) else {
        return ptr::null_mut();
    };

    if config._reserved0 != 0 {
        return ptr::null_mut();
    };

    if config.glfw_window.is_null() {
        return ptr::null_mut();
    }

    if config.glfw_api.glfw_get_proc_address == 0 {
        return ptr::null_mut();
    }

    let params = EngineCreateParams {
        glfw_window: config.glfw_window,
        glfw_get_proc_address: config.glfw_api.glfw_get_proc_address,
        glfw_make_context_current: config.glfw_api.glfw_make_context_current,
        gl_api: config.gl_api,
        assume_context_current: config.assume_context_current != 0,
        auto_paint: config.auto_paint != 0,
    };

    let engine = match XianWebEngine::new(params) {
        Ok(engine) => engine,
        Err(_) => return ptr::null_mut(),
    };

    Box::into_raw(Box::new(engine))
}

#[unsafe(no_mangle)]
/// ### English
/// Destroys an engine instance.
///
/// #### Parameters
/// - `engine`: Engine pointer returned by `xian_web_engine_create`.
///
/// #### Threading
/// - Must follow this module's single-thread + current-context contract.
///
/// #### Safety
/// - `engine` must be either NULL, or a valid pointer returned by `xian_web_engine_create`.
/// - The engine must not be used after this call.
///
/// ### 中文
/// 销毁引擎实例。
///
/// #### 参数
/// - `engine`：由 `xian_web_engine_create` 返回的引擎指针。
///
/// #### 线程
/// - 必须遵守本模块的单线程 + 上下文 current 约定。
///
/// #### 安全性
/// - `engine` 必须为 NULL，或由 `xian_web_engine_create` 返回的有效指针。
/// - 调用后不得再使用该 engine。
pub unsafe extern "C" fn xian_web_engine_destroy(engine: *mut XianWebEngine) {
    let Some(engine) = (unsafe { aligned_mut(engine) }) else {
        return;
    };

    engine.detach_all_views();
    unsafe { drop(Box::from_raw(engine)) };
}

#[unsafe(no_mangle)]
/// ### English
/// Returns whether the engine has pending work (best-effort hint).
///
/// #### Parameters
/// - `engine`: Engine pointer.
///
/// #### Returns
/// - `true` if `xian_web_engine_tick` is likely useful.
///
/// ### 中文
/// 返回引擎是否存在待处理工作（best-effort 提示）。
///
/// #### 参数
/// - `engine`：引擎指针。
///
/// #### 返回
/// - 若 `xian_web_engine_tick` 可能有意义则返回 `true`。
pub unsafe extern "C" fn xian_web_engine_needs_tick(engine: *const XianWebEngine) -> bool {
    let Some(engine) = (unsafe { aligned_ref(engine) }) else {
        return false;
    };
    engine.needs_tick()
}

#[unsafe(no_mangle)]
/// ### English
/// Drives Servo once. When `AUTO_PAINT` is enabled, this also paints all dirty views.
///
/// #### Parameters
/// - `engine`: Engine pointer.
///
/// #### Returns
/// - Number of views painted in this tick.
///
/// #### Threading
/// - Must follow this module's single-thread + current-context contract.
///
/// #### Safety
/// - `engine` must be either NULL, or a valid pointer returned by `xian_web_engine_create`.
///
/// ### 中文
/// 驱动 Servo 一次；当启用 `AUTO_PAINT` 时，会在本次 tick 内绘制所有 dirty view。
///
/// #### 参数
/// - `engine`：引擎指针。
///
/// #### 返回
/// - 本次 tick 绘制的 view 数量。
///
/// #### 线程
/// - 必须遵守本模块的单线程 + 上下文 current 约定。
///
/// #### 安全性
/// - `engine` 必须为 NULL，或由 `xian_web_engine_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_tick(engine: *mut XianWebEngine) -> u32 {
    let Some(engine) = (unsafe { aligned_mut(engine) }) else {
        return 0;
    };
    engine.tick()
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
/// Creates a new view owned by `config.engine`.
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
/// 在 `config.engine` 上创建一个新 view。
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

    let Some(engine) = (unsafe { aligned_mut(config.engine) }) else {
        return ptr::null_mut();
    };

    let initial_url = unsafe { cstr_to_str(config.initial_url) }.map(str::to_owned);

    let params = ViewCreateParams {
        width: config.width,
        height: config.height,
        initial_url,
    };

    match engine.create_view(params) {
        Ok(ptr) => ptr.as_ptr(),
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
/// Converts a raw pointer into an aligned mutable reference.
///
/// #### Parameters
/// - `ptr`: Raw pointer.
///
/// #### Returns
/// - `Some(&mut T)` when non-null and aligned.
/// - `None` when NULL or misaligned.
///
/// #### Safety
/// - When returning `Some`, `ptr` must be valid for unique mutable access of `T` for the chosen lifetime.
///
/// ### 中文
/// 将原始指针转换为“对齐的可变引用”。
///
/// #### 参数
/// - `ptr`：原始指针。
///
/// #### 返回
/// - 非空且对齐时返回 `Some(&mut T)`。
/// - 为 NULL 或未对齐时返回 `None`。
///
/// #### 安全性
/// - 返回 `Some` 时，`ptr` 必须在该生命周期内对 `T` 具有唯一可变访问权。
unsafe fn aligned_mut<'a, T>(ptr: *mut T) -> Option<&'a mut T> {
    if ptr.is_null() || !is_aligned_ptr(ptr) {
        return None;
    }
    Some(unsafe { &mut *ptr })
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
