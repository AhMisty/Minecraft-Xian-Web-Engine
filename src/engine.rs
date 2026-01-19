//! ### English
//! High-performance Servo embedding core (single-threaded public API).
//!
//! ### 中文
//! 最高性能 Servo 嵌入核心（对外 API 单线程）。

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Once, RwLock};

use dpi::PhysicalSize;
use servo::{RefreshDriver, WebView, WebViewBuilder, WebViewDelegate};

use crate::abi::XianWebEngineInputEvent;
use crate::error::InitError;
use crate::gl::{GlApi, GlHandles, GlfwContext, TextureContext};
use crate::input::map_input_event;

/// ### English
/// One-time initialization for rustls crypto provider installation.
///
/// ### 中文
/// rustls 密码提供者安装的一次性初始化。
static RUSTLS_PROVIDER_INIT: Once = Once::new();

/// ### English
/// Whether Servo has been initialized in this process.
///
/// ### 中文
/// Servo 是否已在本进程中初始化。
static SERVO_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// ### English
/// Process-global embedder GLFW window pointer (`GLFWwindow*` as `usize`, 0 = unset).
///
/// ### 中文
/// 进程全局的宿主 GLFW window 指针（`GLFWwindow*` 以 `usize` 保存，0 表示未设置）。
static GLFW_WINDOW: AtomicUsize = AtomicUsize::new(0);

/// ### English
/// Process-global `glfwGetProcAddress` function pointer address (0 = unset).
///
/// ### 中文
/// 进程全局的 `glfwGetProcAddress` 函数指针地址（0 表示未设置）。
static GLFW_GET_PROC_ADDRESS: AtomicUsize = AtomicUsize::new(0);

/// ### English
/// Process-global optional `glfwMakeContextCurrent` function pointer address (0 = unset).
///
/// ### 中文
/// 进程全局的可选 `glfwMakeContextCurrent` 函数指针地址（0 表示未设置）。
static GLFW_MAKE_CONTEXT_CURRENT: AtomicUsize = AtomicUsize::new(0);

/// ### English
/// Process-global OpenGL API selector (`crate::abi::XIAN_WEB_ENGINE_GL_API_*`).
///
/// ### 中文
/// 进程全局的 OpenGL API 选择值（`crate::abi::XIAN_WEB_ENGINE_GL_API_*`）。
static GL_API: AtomicU32 = AtomicU32::new(crate::abi::XIAN_WEB_ENGINE_GL_API_GL);

/// ### English
/// Process-global "assume current context" toggle (captured on init).
///
/// ### 中文
/// 进程全局的“假定上下文已 current”开关（在初始化时捕获）。
static ASSUME_CONTEXT_CURRENT: AtomicBool = AtomicBool::new(true);

/// ### English
/// Process-global auto-paint toggle (captured on init).
///
/// ### 中文
/// 进程全局的自动绘制开关（在初始化时捕获）。
static AUTO_PAINT: AtomicBool = AtomicBool::new(true);

/// ### English
/// Process-global Servo config directory override.
///
/// ### 中文
/// 进程全局的 Servo 配置目录覆盖值。
static CONFIG_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

/// ### English
/// Process-global worker thread cap (`0` = no cap).
///
/// ### 中文
/// 进程全局的工作线程上限（`0` = 不限制）。
static THREAD_POOL_CAP: AtomicU32 = AtomicU32::new(0);

#[inline]
/// ### English
/// Returns whether Servo has been initialized in this process.
///
/// #### Returns
/// - `true` if Servo has been initialized.
/// - `false` otherwise.
///
/// ### 中文
/// 返回 Servo 是否已在本进程中初始化。
///
/// #### 返回
/// - 已初始化则返回 `true`。
/// - 否则返回 `false`。
pub(crate) fn is_servo_initialized() -> bool {
    SERVO_INITIALIZED.load(Ordering::Relaxed)
}

#[inline]
/// ### English
/// Sets the process-global embedder GLFW context/proc addresses.
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `glfw_window`: Embedder-owned `GLFWwindow*` (as `*mut c_void`).
/// - `glfw_get_proc_address`: Address of `glfwGetProcAddress` (as `uintptr_t`).
/// - `glfw_make_context_current`: Address of `glfwMakeContextCurrent` (as `uintptr_t`, 0 allowed).
///
/// #### Returns
/// - `true` if the values were accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置进程全局的宿主 GLFW 上下文/函数指针。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `glfw_window`：宿主侧 `GLFWwindow*`（以 `*mut c_void` 形式传入）。
/// - `glfw_get_proc_address`：`glfwGetProcAddress` 的地址（`uintptr_t`）。
/// - `glfw_make_context_current`：`glfwMakeContextCurrent` 的地址（`uintptr_t`，允许为 0）。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub(crate) fn set_glfw_context(
    glfw_window: *mut std::ffi::c_void,
    glfw_get_proc_address: usize,
    glfw_make_context_current: usize,
) -> bool {
    if is_servo_initialized() {
        return false;
    }
    GLFW_WINDOW.store(glfw_window as usize, Ordering::Relaxed);
    GLFW_GET_PROC_ADDRESS.store(glfw_get_proc_address, Ordering::Relaxed);
    GLFW_MAKE_CONTEXT_CURRENT.store(glfw_make_context_current, Ordering::Relaxed);
    true
}

#[inline]
/// ### English
/// Sets the process-global OpenGL API selector (`crate::abi::XIAN_WEB_ENGINE_GL_API_*`).
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `gl_api`: Raw selector value from the C ABI.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置进程全局的 OpenGL API 选择值（`crate::abi::XIAN_WEB_ENGINE_GL_API_*`）。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `gl_api`：来自 C ABI 的原始选择值。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub(crate) fn set_gl_api(gl_api: u32) -> bool {
    if is_servo_initialized() {
        return false;
    }
    GL_API.store(gl_api, Ordering::Relaxed);
    true
}

#[inline]
/// ### English
/// Sets whether to assume the embedder context is already current on the calling thread.
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `assume_context_current`: `true` to skip calling `glfwMakeContextCurrent` on hot paths.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置是否假定宿主上下文已在调用线程 current。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `assume_context_current`：为 `true` 时在热路径跳过调用 `glfwMakeContextCurrent`。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub(crate) fn set_assume_context_current(assume_context_current: bool) -> bool {
    if is_servo_initialized() {
        return false;
    }
    ASSUME_CONTEXT_CURRENT.store(assume_context_current, Ordering::Relaxed);
    true
}

#[inline]
/// ### English
/// Sets whether to auto-paint dirty views inside `tick`.
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `auto_paint`: `true` to paint all dirty views after each `tick`.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置是否在 `tick` 内自动绘制 dirty view。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `auto_paint`：为 `true` 时在每次 `tick` 后绘制所有 dirty view。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub(crate) fn set_auto_paint(auto_paint: bool) -> bool {
    if is_servo_initialized() {
        return false;
    }
    AUTO_PAINT.store(auto_paint, Ordering::Relaxed);
    true
}

#[inline]
/// ### English
/// Sets the process-global Servo config directory override.
///
/// This must be called before creating an engine.
///
/// #### Parameters
/// - `path`: Override directory; `None` clears the override.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置进程全局的 Servo 配置目录覆盖值。
///
/// 必须在创建 engine 之前调用。
///
/// #### 参数
/// - `path`：覆盖目录；传 `None` 表示清空覆盖。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub(crate) fn set_config_dir(path: Option<PathBuf>) -> bool {
    if is_servo_initialized() {
        return false;
    }
    let mut config_dir = CONFIG_DIR.write().unwrap_or_else(|e| e.into_inner());
    *config_dir = path;
    true
}

#[inline]
/// ### English
/// Gets the process-global Servo config directory override.
///
/// #### Returns
/// - `Some(PathBuf)` when an override directory is set.
/// - `None` when no override is configured.
///
/// ### 中文
/// 获取进程全局的 Servo 配置目录覆盖值。
///
/// #### 返回
/// - 已设置覆盖目录时返回 `Some(PathBuf)`。
/// - 未设置覆盖时返回 `None`。
pub(crate) fn config_dir() -> Option<PathBuf> {
    CONFIG_DIR.read().unwrap_or_else(|e| e.into_inner()).clone()
}

#[inline]
/// ### English
/// Sets the process-global worker thread cap (`0` = no cap).
///
/// This must be called before creating an engine.
///
/// #### Parameters
/// - `cap`: Maximum number of worker threads (`0` means "no cap").
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置进程全局的工作线程上限（`0` = 不限制）。
///
/// 必须在创建 engine 之前调用。
///
/// #### 参数
/// - `cap`：工作线程上限（`0` 表示“不限制”）。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub(crate) fn set_thread_pool_cap(cap: u32) -> bool {
    if is_servo_initialized() {
        return false;
    }
    THREAD_POOL_CAP.store(cap, Ordering::Relaxed);
    true
}

#[inline]
/// ### English
/// Gets the process-global worker thread cap (`0` = no cap).
///
/// #### Returns
/// - Maximum number of worker threads (`0` means "no cap").
///
/// ### 中文
/// 获取进程全局的工作线程上限（`0` = 不限制）。
///
/// #### 返回
/// - 工作线程上限（`0` 表示“不限制”）。
pub(crate) fn thread_pool_cap() -> u32 {
    THREAD_POOL_CAP.load(Ordering::Relaxed)
}

thread_local! {
    /// ### English
    /// Thread-local engine instance (created lazily by `create_view`).
    ///
    /// ### 中文
    /// 线程本地的引擎实例（由 `create_view` 惰性创建）。
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
}

#[inline]
/// ### English
/// Builds `EngineParams` from process-global settings.
///
/// #### Returns
/// - `Ok(EngineParams)` when required values are present.
/// - `Err(InitError)` when required pointers are missing.
///
/// ### 中文
/// 从进程全局设置构建 `EngineParams`。
///
/// #### 返回
/// - 必需值齐全时返回 `Ok(EngineParams)`。
/// - 必需指针缺失时返回 `Err(InitError)`。
fn engine_params_from_globals() -> Result<EngineParams, InitError> {
    let glfw_window = GLFW_WINDOW.load(Ordering::Relaxed) as *mut std::ffi::c_void;
    if glfw_window.is_null() {
        return Err(InitError::NullPointer {
            name: "glfw_window",
        });
    }
    let glfw_get_proc_address = GLFW_GET_PROC_ADDRESS.load(Ordering::Relaxed);
    if glfw_get_proc_address == 0 {
        return Err(InitError::NullPointer {
            name: "glfwGetProcAddress",
        });
    }

    Ok(EngineParams {
        glfw_window,
        glfw_get_proc_address,
        glfw_make_context_current: GLFW_MAKE_CONTEXT_CURRENT.load(Ordering::Relaxed),
        gl_api: GL_API.load(Ordering::Relaxed),
        assume_context_current: ASSUME_CONTEXT_CURRENT.load(Ordering::Relaxed),
        auto_paint: AUTO_PAINT.load(Ordering::Relaxed),
    })
}

#[inline]
/// ### English
/// Returns whether the engine likely has pending work (best-effort hint).
///
/// When no engine has been created yet, this returns `false`.
///
/// #### Returns
/// - `true` when the engine likely has pending work.
/// - `false` when the engine appears idle or has not been created yet.
///
/// ### 中文
/// 返回引擎是否可能存在待处理工作（best-effort 提示）。
///
/// 当引擎尚未创建时返回 `false`。
///
/// #### 返回
/// - 可能需要 tick 时返回 `true`。
/// - 看起来空闲或尚未创建时返回 `false`。
pub(crate) fn needs_tick() -> bool {
    ENGINE.with(|cell| {
        let engine_slot = cell.borrow();
        let Some(engine) = engine_slot.as_ref() else {
            return false;
        };
        engine.needs_tick()
    })
}

#[inline]
/// ### English
/// Drives the engine once.
///
/// When no engine has been created yet, this returns `0`.
///
/// #### Returns
/// - Number of views painted in this tick.
///
/// ### 中文
/// 驱动引擎一次。
///
/// 当引擎尚未创建时返回 `0`。
///
/// #### 返回
/// - 本次 tick 绘制的 view 数量。
pub(crate) fn tick() -> u32 {
    ENGINE.with(|cell| {
        let mut engine_slot = cell.borrow_mut();
        let Some(engine) = engine_slot.as_mut() else {
            return 0;
        };
        engine.tick()
    })
}

/// ### English
/// Creates a view (initializes the engine lazily on the first call).
///
/// #### Parameters
/// - `params`: View creation parameters.
///
/// #### Returns
/// - `Ok(NonNull<XianWebEngineView>)` on success.
/// - `Err(InitError)` when initialization fails.
///
/// ### 中文
/// 创建 view（首次调用时会惰性初始化引擎）。
///
/// #### 参数
/// - `params`：view 创建参数。
///
/// #### 返回
/// - 成功返回 `Ok(NonNull<XianWebEngineView>)`。
/// - 初始化失败返回 `Err(InitError)`。
pub(crate) fn create_view(params: ViewParams) -> Result<NonNull<XianWebEngineView>, InitError> {
    ENGINE.with(|cell| {
        let mut engine_slot = cell.borrow_mut();
        if engine_slot.is_none() {
            *engine_slot = Some(Engine::new(engine_params_from_globals()?)?);
        }
        let engine = engine_slot
            .as_mut()
            .ok_or(InitError::InternalInvariant { name: "ENGINE" })?;
        Ok(engine.create_view(params))
    })
}

/// ### English
/// Unregisters a view pointer from the engine if the engine exists.
///
/// #### Parameters
/// - `view`: View pointer to remove.
///
/// ### 中文
/// 若引擎存在，则从引擎中注销一个 view 指针。
///
/// #### 参数
/// - `view`：要移除的 view 指针。
pub(crate) fn unregister_view(view: NonNull<XianWebEngineView>) {
    ENGINE.with(|cell| {
        let mut engine_slot = cell.borrow_mut();
        let Some(engine) = engine_slot.as_mut() else {
            return;
        };
        engine.unregister_view(view);
    });
}

/// ### English
/// Parameters for creating an engine (captured from process-global ABI settings).
///
/// ### 中文
/// 引擎创建参数（来自进程全局的 ABI 设置）。
pub(crate) struct EngineParams {
    /// ### English
    /// Pointer to embedder-owned `GLFWwindow*`.
    ///
    /// ### 中文
    /// 宿主侧 `GLFWwindow*` 指针。
    pub(crate) glfw_window: *mut std::ffi::c_void,

    /// ### English
    /// `glfwGetProcAddress` function pointer address (as `uintptr_t`).
    ///
    /// ### 中文
    /// `glfwGetProcAddress` 函数指针地址（`uintptr_t`）。
    pub(crate) glfw_get_proc_address: usize,

    /// ### English
    /// Optional `glfwMakeContextCurrent` function pointer address (as `uintptr_t`).
    ///
    /// ### 中文
    /// 可选的 `glfwMakeContextCurrent` 函数指针地址（`uintptr_t`）。
    pub(crate) glfw_make_context_current: usize,

    /// ### English
    /// OpenGL API kind (`XIAN_WEB_ENGINE_GL_API_*`).
    ///
    /// ### 中文
    /// OpenGL API 类型（`XIAN_WEB_ENGINE_GL_API_*`）。
    pub(crate) gl_api: u32,

    /// ### English
    /// Whether to assume the context is already current on the calling thread.
    ///
    /// ### 中文
    /// 是否假定调用线程上上下文已经 current。
    pub(crate) assume_context_current: bool,

    /// ### English
    /// Whether to automatically paint dirty views in `tick`.
    ///
    /// ### 中文
    /// 是否在 `tick` 中自动绘制 dirty view。
    pub(crate) auto_paint: bool,
}

/// ### English
/// Parameters for creating a view.
///
/// ### 中文
/// View 创建参数。
pub(crate) struct ViewParams {
    /// ### English
    /// Initial width in pixels.
    ///
    /// ### 中文
    /// 初始宽度（像素）。
    pub(crate) width: u32,

    /// ### English
    /// Initial height in pixels.
    ///
    /// ### 中文
    /// 初始高度（像素）。
    pub(crate) height: u32,

    /// ### English
    /// Optional initial URL to load after creation.
    ///
    /// ### 中文
    /// 可选的初始 URL（创建后自动加载）。
    pub(crate) initial_url: Option<url::Url>,

    /// ### English
    /// HiDPI scale factor (`1.0` means 1 CSS pixel = 1 device pixel).
    ///
    /// Values that are non-finite or `<= 0` are treated as `1.0`.
    ///
    /// ### 中文
    /// HiDPI 缩放因子（`1.0` 表示 1 个 CSS 像素 = 1 个设备像素）。
    ///
    /// 非有限值或 `<= 0` 会被视为 `1.0`。
    pub(crate) hidpi_scale_factor: f32,
}

#[derive(Clone)]
/// ### English
/// Servo event-loop waker that flips a shared atomic flag.
///
/// ### 中文
/// Servo 事件循环唤醒器：翻转一个共享的原子标记。
struct TickWaker {
    /// ### English
    /// `true` means the engine likely has pending work and should be ticked.
    ///
    /// ### 中文
    /// `true` 表示引擎可能有待处理工作，应进行 tick。
    tick_pending: Arc<AtomicBool>,
}

impl servo::EventLoopWaker for TickWaker {
    /// ### English
    /// Clones this waker as a boxed trait object.
    ///
    /// #### Returns
    /// - A new boxed waker that shares the same pending flag.
    ///
    /// ### 中文
    /// 将该唤醒器克隆为装箱的 trait object。
    ///
    /// #### 返回
    /// - 新的装箱唤醒器，与当前实例共享同一个 pending 标记。
    fn clone_box(&self) -> Box<dyn servo::EventLoopWaker> {
        Box::new(self.clone())
    }

    /// ### English
    /// Wakes the event loop by marking the engine as having pending work.
    ///
    /// ### 中文
    /// 通过将引擎标记为“有待处理工作”来唤醒事件循环。
    fn wake(&self) {
        self.tick_pending.store(true, Ordering::Relaxed);
    }
}

/// ### English
/// A minimal `RefreshDriver` implementation that stores callbacks and runs them on `begin_frame`.
///
/// ### 中文
/// 最小化的 `RefreshDriver` 实现：存储回调并在 `begin_frame` 时执行。
struct FrameDriver {
    /// ### English
    /// Pending callbacks requested by Servo.
    ///
    /// ### 中文
    /// Servo 请求的待执行回调。
    callbacks: RefCell<Vec<Box<dyn Fn() + Send + 'static>>>,

    /// ### English
    /// Scratch buffer reused to avoid allocations on the hot path.
    ///
    /// ### 中文
    /// 可复用的临时缓冲，用于在热路径上避免分配。
    scratch: RefCell<Vec<Box<dyn Fn() + Send + 'static>>>,
}

impl FrameDriver {
    /// ### English
    /// Creates a new refresh driver.
    ///
    /// #### Returns
    /// - A new `FrameDriver`.
    ///
    /// ### 中文
    /// 创建新的刷新驱动。
    ///
    /// #### 返回
    /// - 新的 `FrameDriver`。
    fn new() -> Self {
        Self {
            callbacks: RefCell::new(Vec::new()),
            scratch: RefCell::new(Vec::new()),
        }
    }

    /// ### English
    /// Starts a new frame and runs all pending callbacks.
    ///
    /// Callbacks scheduled during execution are kept for the next frame.
    ///
    /// ### 中文
    /// 开始新的一帧并执行所有待处理回调。
    ///
    /// 执行期间新增的回调会保留到下一帧。
    fn begin_frame(&self) {
        let mut callbacks = self.callbacks.borrow_mut();
        if callbacks.is_empty() {
            return;
        }

        /*
        ### English
        Move callbacks into a reusable scratch buffer, keeping the `callbacks` Vec capacity so new
        callbacks scheduled during execution do not reallocate on the hot path.

        ### 中文
        将回调移动到可复用的临时缓冲中，同时保留 `callbacks` Vec 的 capacity，使回调执行期间新增的回调
        在热路径上尽量避免重新分配。
        */
        let mut scratch = self.scratch.borrow_mut();
        scratch.clear();
        scratch.append(&mut *callbacks);
        drop(callbacks);

        for cb in scratch.drain(..) {
            cb();
        }
    }
}

impl RefreshDriver for FrameDriver {
    /// ### English
    /// Registers a callback to be run at the start of the next frame.
    ///
    /// #### Parameters
    /// - `start_frame_callback`: Callback scheduled by Servo.
    ///
    /// ### 中文
    /// 注册一个将在下一帧开始时执行的回调。
    ///
    /// #### 参数
    /// - `start_frame_callback`：Servo 调度的回调。
    fn observe_next_frame(&self, start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.callbacks.borrow_mut().push(start_frame_callback);
    }
}

/// ### English
/// `WebViewDelegate` that tracks whether a view is "dirty" (has a new frame ready).
///
/// ### 中文
/// 用于跟踪 view 是否“脏”（已有新帧可用）的 `WebViewDelegate`。
struct DirtyTracker {
    /// ### English
    /// `true` when a new frame is ready and the view should be painted.
    ///
    /// ### 中文
    /// `true` 表示已有新帧可用，需要进行绘制。
    dirty: Cell<bool>,

    /// ### English
    /// Shared dirty-view counter owned by the engine.
    ///
    /// This is incremented on the clean→dirty transition and decremented on the dirty→clean
    /// transition.
    ///
    /// ### 中文
    /// 引擎持有的“dirty view 计数器”共享引用。
    ///
    /// 该计数在“从干净→变脏”时递增，在“从变脏→清理”时递减。
    dirty_count: Rc<Cell<usize>>,
}

impl DirtyTracker {
    /// ### English
    /// Creates a new delegate marked as dirty.
    ///
    /// This increments the shared dirty-view counter.
    ///
    /// #### Parameters
    /// - `dirty_count`: Shared dirty-view counter owned by the engine.
    ///
    /// #### Returns
    /// - A new `DirtyTracker` instance.
    ///
    /// ### 中文
    /// 创建新的 delegate（初始标记为 dirty）。
    ///
    /// 该调用会递增引擎共享的 dirty-view 计数。
    ///
    /// #### 参数
    /// - `dirty_count`：引擎持有的共享 dirty-view 计数器。
    ///
    /// #### 返回
    /// - 新的 `DirtyTracker` 实例。
    fn new(dirty_count: Rc<Cell<usize>>) -> Self {
        dirty_count.set(dirty_count.get().saturating_add(1));
        Self {
            dirty: Cell::new(true),
            dirty_count,
        }
    }

    /// ### English
    /// Returns whether the view needs painting.
    ///
    /// #### Returns
    /// - `true` when a new frame is ready and the view should be painted.
    /// - `false` when the view is clean.
    ///
    /// ### 中文
    /// 返回该 view 是否需要绘制。
    ///
    /// #### 返回
    /// - 已有新帧可用、需要绘制时返回 `true`。
    /// - view 干净时返回 `false`。
    fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// ### English
    /// Clears the dirty flag.
    ///
    /// Also updates the engine-level dirty-view counter.
    ///
    /// ### 中文
    /// 清除 dirty 标记。
    ///
    /// 同时更新引擎级的 dirty-view 计数。
    fn clear(&self) {
        let _ = self.take_dirty();
    }

    /// ### English
    /// Clears the dirty flag and returns whether it was previously set.
    ///
    /// This updates the engine-level dirty-view counter on the dirty→clean transition.
    ///
    /// #### Returns
    /// - `true` if the flag was dirty and is now cleared.
    /// - `false` if the flag was already clean.
    ///
    /// ### 中文
    /// 清除 dirty 标记，并返回清除前是否为 dirty。
    ///
    /// 在“dirty→clean”时，该函数会同步更新引擎级的 dirty-view 计数。
    ///
    /// #### 返回
    /// - 之前为 dirty 且本次已清除时返回 `true`。
    /// - 之前已为 clean 时返回 `false`。
    fn take_dirty(&self) -> bool {
        if !self.dirty.replace(false) {
            return false;
        }

        let count = self.dirty_count.get();
        self.dirty_count.set(count.saturating_sub(1));
        true
    }

    /// ### English
    /// Marks the view as dirty.
    ///
    /// This updates the engine-level dirty-view counter only on the clean→dirty transition.
    ///
    /// ### 中文
    /// 将 view 标记为 dirty。
    ///
    /// 仅在“clean→dirty”时更新引擎级的 dirty-view 计数。
    fn mark_dirty(&self) {
        if self.dirty.replace(true) {
            return;
        }
        self.dirty_count
            .set(self.dirty_count.get().saturating_add(1));
    }
}

impl WebViewDelegate for DirtyTracker {
    /// ### English
    /// Marks the view as dirty when Servo reports a new frame is ready.
    ///
    /// #### Parameters
    /// - `_webview`: WebView handle provided by Servo (unused).
    ///
    /// ### 中文
    /// 当 Servo 通知新帧就绪时，将 view 标记为 dirty。
    ///
    /// #### 参数
    /// - `_webview`：Servo 提供的 WebView 句柄（未使用）。
    fn notify_new_frame_ready(&self, _webview: WebView) {
        self.mark_dirty();
    }
}

/// ### English
/// Thread-local engine state.
///
/// ### 中文
/// 线程本地的引擎状态。
struct Engine {
    /// ### English
    /// Flag flipped by Servo's waker; used by `needs_tick`.
    ///
    /// ### 中文
    /// 由 Servo 的 waker 翻转的标记；供 `needs_tick` 判断使用。
    tick_pending: Arc<AtomicBool>,

    /// ### English
    /// Number of views currently marked as dirty (has a frame ready).
    ///
    /// This is updated by each `DirtyTracker` so `needs_tick` can stay O(1) when `auto_paint`
    /// is enabled.
    ///
    /// ### 中文
    /// 当前被标记为 dirty（已生成新帧）的 view 数量。
    ///
    /// 该计数由每个 `DirtyTracker` 维护，使得在开启 `auto_paint` 时 `needs_tick` 保持 O(1)。
    dirty_count: Rc<Cell<usize>>,

    /// ### English
    /// Whether to paint dirty views automatically in `tick`.
    ///
    /// ### 中文
    /// 是否在 `tick` 中自动绘制 dirty view。
    auto_paint: bool,

    /// ### English
    /// Refresh driver used by Servo to schedule frame callbacks.
    ///
    /// ### 中文
    /// Servo 用于调度帧回调的刷新驱动。
    frame_driver: Rc<FrameDriver>,

    /// ### English
    /// Embedder GLFW proc table (copied into each view context).
    ///
    /// ### 中文
    /// 宿主 GLFW 函数表（会拷贝到每个 view 的上下文中）。
    glfw: GlfwContext,

    /// ### English
    /// Shared OpenGL API handles.
    ///
    /// ### 中文
    /// 共享 OpenGL API 句柄。
    gl: GlHandles,

    /// ### English
    /// Servo instance (driven by `tick`).
    ///
    /// ### 中文
    /// Servo 实例（由 `tick` 驱动）。
    servo: servo::Servo,

    /// ### English
    /// Raw pointers to currently registered views (stable addresses from `Box`).
    ///
    /// ### 中文
    /// 当前注册的 view 的原始指针（来自 `Box` 的稳定地址）。
    views: Vec<NonNull<XianWebEngineView>>,
}

impl Engine {
    /// ### English
    /// Creates a new engine instance.
    ///
    /// #### Parameters
    /// - `params`: Parsed creation parameters.
    ///
    /// #### Returns
    /// - `Ok(Engine)` on success.
    /// - `Err(InitError)` on initialization failure.
    ///
    /// ### 中文
    /// 创建新的引擎实例。
    ///
    /// #### 参数
    /// - `params`：已解析的创建参数。
    ///
    /// #### 返回
    /// - 成功返回 `Ok(Engine)`。
    /// - 初始化失败返回 `Err(InitError)`。
    pub(crate) fn new(params: EngineParams) -> Result<Self, InitError> {
        RUSTLS_PROVIDER_INIT.call_once(|| {
            /*
            ### English
            Best-effort: install rustls crypto provider (Servo uses rustls internally).

            This is process-global and must be installed at most once.

            ### 中文
            尽力而为：安装 rustls 密码提供者（Servo 内部使用 rustls）。

            该设置为进程全局，且最多只能安装一次。
            */
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });

        let config_dir = config_dir();
        if let Some(ref config_dir) = config_dir {
            let _ = std::fs::create_dir_all(config_dir);
        }

        let glfw = unsafe {
            GlfwContext::from_raw(
                params.glfw_window,
                params.glfw_get_proc_address,
                params.glfw_make_context_current,
                params.assume_context_current,
            )?
        };

        unsafe { glfw.make_current() };

        let gl_api = GlApi::from_u32(params.gl_api)?;
        let gl = unsafe { GlHandles::new(gl_api, &glfw)? };

        let cpu_threads = std::thread::available_parallelism()
            .map(|n| n.get() as i64)
            .unwrap_or(3)
            .max(1);
        let thread_pool_cap = thread_pool_cap();
        let tuned_threads = if thread_pool_cap == 0 {
            cpu_threads
        } else {
            cpu_threads.min(thread_pool_cap as i64).max(1)
        };

        let opts = servo::Opts {
            multiprocess: false,
            force_ipc: false,
            nonincremental_layout: false,
            time_profiling: None,
            time_profiler_trace_path: None,
            debug: Default::default(),
            background_hang_monitor: false,
            unminify_js: false,
            local_script_source: None,
            unminify_css: false,
            print_pwm: false,
            random_pipeline_closure_probability: None,
            random_pipeline_closure_seed: None,
            config_dir,
            ..Default::default()
        };

        let preferences = servo::Preferences {
            gfx_precache_shaders: true,
            layout_threads: tuned_threads,
            threadpools_fallback_worker_num: tuned_threads,
            threadpools_async_runtime_workers_max: tuned_threads,
            threadpools_image_cache_workers_max: tuned_threads,
            threadpools_webrender_workers_max: tuned_threads,
            threadpools_indexeddb_workers_max: tuned_threads,
            threadpools_webstorage_workers_max: tuned_threads,
            ..Default::default()
        };

        let tick_pending = Arc::new(AtomicBool::new(true));
        let waker: Box<dyn servo::EventLoopWaker> = Box::new(TickWaker {
            tick_pending: tick_pending.clone(),
        });

        let frame_driver = Rc::new(FrameDriver::new());

        if SERVO_INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(InitError::ServoAlreadyInitialized);
        }

        let servo = servo::ServoBuilder::default()
            .opts(opts)
            .preferences(preferences)
            .event_loop_waker(waker)
            .build();

        Ok(Self {
            tick_pending,
            dirty_count: Rc::new(Cell::new(0)),
            auto_paint: params.auto_paint,
            frame_driver,
            glfw,
            gl,
            servo,
            views: Vec::new(),
        })
    }

    /// ### English
    /// Returns whether the engine likely has pending work (best-effort hint).
    ///
    /// #### Returns
    /// - `true` when `tick` is likely useful.
    /// - `false` when the engine appears idle.
    ///
    /// ### 中文
    /// 返回引擎是否可能存在待处理工作（best-effort 提示）。
    ///
    /// #### 返回
    /// - 可能需要 tick 时返回 `true`。
    /// - 看起来空闲时返回 `false`。
    pub(crate) fn needs_tick(&self) -> bool {
        if self.tick_pending.load(Ordering::Relaxed) {
            return true;
        }

        if self.auto_paint && self.dirty_count.get() != 0 {
            return true;
        }

        false
    }

    /// ### English
    /// Drives Servo once.
    ///
    /// When auto-paint is enabled, this also paints all dirty views.
    ///
    /// #### Returns
    /// - Number of views painted in this tick.
    ///
    /// ### 中文
    /// 驱动 Servo 一次。
    ///
    /// 当启用自动绘制时，会在本次 tick 内绘制所有 dirty view。
    ///
    /// #### 返回
    /// - 本次 tick 绘制的 view 数量。
    pub(crate) fn tick(&mut self) -> u32 {
        self.tick_pending.store(false, Ordering::Relaxed);
        self.frame_driver.begin_frame();
        self.servo.spin_event_loop();

        if !self.auto_paint {
            return 0;
        }

        if self.dirty_count.get() == 0 {
            return 0;
        }

        let mut painted = 0;
        for &ptr in &self.views {
            let view = unsafe { ptr.as_ref() };
            if view.paint() {
                painted += 1;
                if self.dirty_count.get() == 0 {
                    break;
                }
            }
        }
        painted
    }

    /// ### English
    /// Creates a new view and registers it inside this engine.
    ///
    /// #### Parameters
    /// - `params`: View creation parameters.
    ///
    /// #### Returns
    /// - A non-null view pointer.
    ///
    /// ### 中文
    /// 创建新的 view 并注册到该引擎中。
    ///
    /// #### 参数
    /// - `params`：View 创建参数。
    ///
    /// #### 返回
    /// - 非空的 view 指针。
    pub(crate) fn create_view(&mut self, params: ViewParams) -> NonNull<XianWebEngineView> {
        let size = PhysicalSize::new(params.width.max(1), params.height.max(1));
        let hidpi_scale_factor =
            if params.hidpi_scale_factor.is_finite() && params.hidpi_scale_factor > 0.0 {
                params.hidpi_scale_factor
            } else {
                1.0
            };

        let rendering_context = Rc::new(TextureContext::new(
            self.glfw,
            self.gl.clone(),
            size,
            Some(self.frame_driver.clone() as Rc<dyn RefreshDriver>),
        ));

        let dirty_tracker = Rc::new(DirtyTracker::new(Rc::clone(&self.dirty_count)));

        let webview = WebViewBuilder::new(&self.servo, rendering_context.clone())
            .hidpi_scale_factor(euclid::Scale::new(hidpi_scale_factor))
            .delegate(dirty_tracker.clone())
            .build();

        webview.show();

        if let Some(url) = params.initial_url {
            webview.load(url);
        }

        let view = Box::new(XianWebEngineView {
            webview,
            rendering_context,
            dirty_tracker,
        });

        let ptr = NonNull::from(Box::leak(view));
        self.views.push(ptr);
        ptr
    }

    /// ### English
    /// Unregisters a view pointer from the internal list.
    ///
    /// #### Parameters
    /// - `view`: View pointer to remove.
    ///
    /// ### 中文
    /// 从内部列表中注销一个 view 指针。
    ///
    /// #### 参数
    /// - `view`：要移除的 view 指针。
    fn unregister_view(&mut self, view: NonNull<XianWebEngineView>) {
        if let Some(idx) = self.views.iter().position(|&p| p == view) {
            self.views.swap_remove(idx);
        }
    }
}

#[repr(C)]
/// ### English
/// Opaque view handle returned by the C ABI.
///
/// #### Threading
/// - Must be used on the same thread that owns the embedder GLFW OpenGL context.
///
/// #### Ownership
/// - Created by `xian_web_engine_view_create`.
/// - Destroyed by `xian_web_engine_view_destroy`.
///
/// ### 中文
/// C ABI 返回的“不透明”view 句柄。
///
/// #### 线程
/// - 必须在拥有宿主 GLFW OpenGL 上下文的同一线程使用。
///
/// #### 所有权
/// - 由 `xian_web_engine_view_create` 创建。
/// - 由 `xian_web_engine_view_destroy` 销毁。
pub struct XianWebEngineView {
    /// ### English
    /// Servo `WebView` instance.
    ///
    /// ### 中文
    /// Servo `WebView` 实例。
    webview: WebView,

    /// ### English
    /// Rendering context that owns the offscreen texture for this view.
    ///
    /// ### 中文
    /// 持有该 view 离屏纹理的渲染上下文。
    rendering_context: Rc<TextureContext>,

    /// ### English
    /// Dirty-tracking delegate.
    ///
    /// ### 中文
    /// 用于 dirty 跟踪的 delegate。
    dirty_tracker: Rc<DirtyTracker>,
}

impl XianWebEngineView {
    /// ### English
    /// Destroys a boxed view and unregisters it from its engine when still attached.
    ///
    /// This also clears the dirty flag to keep the engine's dirty-view counter consistent.
    ///
    /// #### Parameters
    /// - `view`: Boxed view to destroy.
    ///
    /// ### 中文
    /// 销毁装箱的 view；若仍绑定引擎则同时从引擎中注销。
    ///
    /// 同时会清除 dirty 标记，以保持引擎 dirty-view 计数的一致性。
    ///
    /// #### 参数
    /// - `view`：要销毁的装箱 view。
    pub(crate) fn destroy_boxed(view: Box<Self>) {
        view.dirty_tracker.clear();

        unregister_view(NonNull::from(view.as_ref()));
    }

    /// ### English
    /// Loads a URL into this view.
    ///
    /// #### Parameters
    /// - `url`: URL string.
    ///
    /// #### Returns
    /// - `true` if the URL was accepted (parsed successfully).
    ///
    /// ### 中文
    /// 向该 view 加载一个 URL。
    ///
    /// #### 参数
    /// - `url`：URL 字符串。
    ///
    /// #### 返回
    /// - URL 被接受（解析成功）则返回 `true`。
    pub(crate) fn load_url(&self, url: &str) -> bool {
        let Ok(url) = url::Url::parse(url) else {
            return false;
        };
        self.webview.load(url);
        true
    }

    /// ### English
    /// Resizes this view.
    ///
    /// #### Parameters
    /// - `width`: New width in pixels (clamped to >= 1).
    /// - `height`: New height in pixels (clamped to >= 1).
    ///
    /// ### 中文
    /// 调整该 view 尺寸。
    ///
    /// #### 参数
    /// - `width`：新宽度（像素，最小为 1）。
    /// - `height`：新高度（像素，最小为 1）。
    pub(crate) fn resize(&self, width: u32, height: u32) {
        let size = PhysicalSize::new(width.max(1), height.max(1));
        self.webview.resize(size);
    }

    /// ### English
    /// Sets the HiDPI scale factor for this view.
    ///
    /// This affects CSS pixel <-> device pixel conversion (e.g. `devicePixelRatio`) and can be
    /// updated at runtime (for example when moving between monitors).
    ///
    /// #### Parameters
    /// - `hidpi_scale_factor`: Scale factor (`1.0` means 1 CSS pixel = 1 device pixel).
    ///
    /// #### Returns
    /// - `true` if the value was accepted.
    /// - `false` if the value is non-finite or `<= 0`.
    ///
    /// ### 中文
    /// 设置该 view 的 HiDPI 缩放因子。
    ///
    /// 该值会影响 CSS 像素与设备像素的换算（例如 `devicePixelRatio`），并且支持运行期更新
    ///（例如窗口移动到不同 DPI 的显示器上）。
    ///
    /// #### 参数
    /// - `hidpi_scale_factor`：缩放因子（`1.0` 表示 1 个 CSS 像素 = 1 个设备像素）。
    ///
    /// #### 返回
    /// - 值被接受则返回 `true`。
    /// - 非有限值或 `<= 0` 则返回 `false`。
    pub(crate) fn set_hidpi_scale_factor(&self, hidpi_scale_factor: f32) -> bool {
        if !hidpi_scale_factor.is_finite() || hidpi_scale_factor <= 0.0 {
            return false;
        }
        self.webview
            .set_hidpi_scale_factor(euclid::Scale::new(hidpi_scale_factor));
        true
    }

    /// ### English
    /// Returns the OpenGL texture id of this view.
    ///
    /// #### Notes
    /// - The texture id may change after `resize`; query again after resizing.
    ///
    /// #### Returns
    /// - OpenGL texture id.
    ///
    /// ### 中文
    /// 返回该 view 的 OpenGL 纹理 id。
    ///
    /// #### 说明
    /// - 纹理 id 可能在 `resize` 后发生变化；resize 后请重新获取。
    ///
    /// #### 返回
    /// - OpenGL 纹理 id。
    pub(crate) fn texture_id(&self) -> u32 {
        self.rendering_context.texture_id()
    }

    /// ### English
    /// Returns whether this view needs painting.
    ///
    /// #### Returns
    /// - `true` when a new frame is ready and the view should be painted.
    /// - `false` when the view is clean.
    ///
    /// ### 中文
    /// 返回该 view 是否需要绘制。
    ///
    /// #### 返回
    /// - 已有新帧可用、需要绘制时返回 `true`。
    /// - view 干净时返回 `false`。
    pub(crate) fn needs_paint(&self) -> bool {
        self.dirty_tracker.is_dirty()
    }

    /// ### English
    /// Paints this view immediately if it is dirty.
    ///
    /// #### Returns
    /// - `true` if a paint was performed.
    /// - `false` if the view was not dirty.
    ///
    /// ### 中文
    /// 若该 view 为 dirty，则立即绘制。
    ///
    /// #### 返回
    /// - 确实执行了绘制则返回 `true`。
    /// - view 非 dirty 则返回 `false`。
    pub(crate) fn paint(&self) -> bool {
        if !self.dirty_tracker.take_dirty() {
            return false;
        }

        self.webview.paint();
        true
    }

    /// ### English
    /// Sends a batch of input events to this view.
    ///
    /// #### Parameters
    /// - `events`: ABI input events.
    ///
    /// #### Returns
    /// - Number of events forwarded to Servo (supported/converted).
    ///
    /// ### 中文
    /// 向该 view 发送一批输入事件。
    ///
    /// #### 参数
    /// - `events`：ABI 输入事件切片。
    ///
    /// #### 返回
    /// - 实际转发给 Servo 的事件数量（支持且完成转换的事件）。
    pub(crate) fn send_input_events(&self, events: &[XianWebEngineInputEvent]) -> u32 {
        let mut forwarded = 0u32;
        for e in events {
            let Some(event) = map_input_event(e) else {
                continue;
            };
            self.webview.notify_input_event(event);
            forwarded += 1;
        }
        forwarded
    }
}
