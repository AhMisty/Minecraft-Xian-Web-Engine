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
use std::time::{Duration, Instant};

use dpi::PhysicalSize;
use servo::{RefreshDriver, WebView, WebViewBuilder, WebViewDelegate};

use crate::abi::XianWebEngineInputEvent;
use crate::error::InitError;
use crate::gl::{GlApi, GlHandles, GlProcLoader, TextureContext};
use crate::input::map_input_event;

/// ### English
/// One-time initialization for rustls crypto provider installation.
///
/// ### 中文
/// rustls 密码提供者安装的一次性初始化。
static RUSTLS_PROVIDER_INIT: Once = Once::new();

/// ### English
/// One-time initialization for Rust-side logging.
///
/// ### 中文
/// Rust 侧日志的一次性初始化。
static LOG_INIT: Once = Once::new();

fn try_init_logging() {
    LOG_INIT.call_once(|| {
        let env = env_logger::Env::default().default_filter_or("warn");
        let logger = env_logger::Builder::from_env(env).build();
        let max_level = logger.filter();
        if log::set_boxed_logger(Box::new(logger)).is_ok() {
            log::set_max_level(max_level);
        }
    });
}

/// ### English
/// Whether Servo has been initialized in this process.
///
/// ### 中文
/// Servo 是否已在本进程中初始化。
static SERVO_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// ### English
/// Process-global `glfwGetProcAddress` function pointer address (0 = unset).
///
/// ### 中文
/// 进程全局的 `glfwGetProcAddress` 函数指针地址（0 表示未设置）。
static GLFW_GET_PROC_ADDRESS: AtomicUsize = AtomicUsize::new(0);

/// ### English
/// Process-global OpenGL API selector (`crate::abi::XIAN_WEB_ENGINE_GL_API_*`).
///
/// ### 中文
/// 进程全局的 OpenGL API 选择值（`crate::abi::XIAN_WEB_ENGINE_GL_API_*`）。
static GL_API: AtomicU32 = AtomicU32::new(crate::abi::XIAN_WEB_ENGINE_GL_API_GL);

/// ### English
/// Process-global Servo config directory override.
///
/// ### 中文
/// 进程全局的 Servo 配置目录覆盖值。
static CONFIG_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

/// ### English
/// Process-global web root directory used by the `xian://` custom protocol.
///
/// ### 中文
/// `xian://` 自定义协议使用的 Web 根目录（进程全局）。
static WEB_ROOT_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

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
/// Sets the process-global `glfwGetProcAddress` address.
///
/// This must be called before Servo is initialized.
///
/// #### Parameters
/// - `glfw_get_proc_address`: Address of `glfwGetProcAddress` (as `uintptr_t`).
///
/// #### Returns
/// - `true` if the values were accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置进程全局的 `glfwGetProcAddress` 地址。
///
/// 必须在 Servo 初始化之前调用。
///
/// #### 参数
/// - `glfw_get_proc_address`：`glfwGetProcAddress` 的地址（`uintptr_t`）。
///
/// #### 返回
/// - 值被接受则返回 `true`。
/// - Servo 已初始化则返回 `false`。
pub(crate) fn set_glfw_api(glfw_get_proc_address: usize) -> bool {
    if is_servo_initialized() {
        return false;
    }
    GLFW_GET_PROC_ADDRESS.store(glfw_get_proc_address, Ordering::Relaxed);
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
/// Sets the process-global Servo config directory override.
///
/// This must be called before calling `init`.
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
/// 必须在调用 `init` 之前调用。
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
/// Sets the process-global web root directory used by the `xian://` custom protocol.
///
/// This must be called before calling `init`.
///
/// #### Parameters
/// - `path`: Root directory; `None` clears the override.
///
/// #### Returns
/// - `true` if the value was accepted.
/// - `false` if Servo is already initialized.
///
/// ### 中文
/// 设置 `xian://` 自定义协议使用的 Web 根目录（进程全局）。
///
/// 必须在调用 `init` 之前调用。
pub(crate) fn set_web_root_dir(path: Option<PathBuf>) -> bool {
    if is_servo_initialized() {
        return false;
    }
    let mut root = WEB_ROOT_DIR.write().unwrap_or_else(|e| e.into_inner());
    *root = path;
    true
}

#[inline]
/// ### English
/// Gets the process-global web root directory used by the `xian://` custom protocol.
///
/// #### Returns
/// - `Some(PathBuf)` when configured.
/// - `None` when not configured.
///
/// ### 中文
/// 获取 `xian://` 自定义协议使用的 Web 根目录（进程全局）。
pub(crate) fn web_root_dir() -> Option<PathBuf> {
    WEB_ROOT_DIR.read().unwrap_or_else(|e| e.into_inner()).clone()
}

#[inline]
/// ### English
/// Sets the process-global worker thread cap (`0` = no cap).
///
/// This must be called before calling `init`.
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
/// 必须在调用 `init` 之前调用。
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
    /// Thread-local engine instance (created by `init`).
    ///
    /// ### 中文
    /// 线程本地的引擎实例（由 `init` 创建）。
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
}

#[inline]
/// ### English
/// Drives the engine once.
///
/// When no engine has been created yet, this returns `0`.
///
/// #### Returns
/// - Always returns `0`.
///
/// ### 中文
/// 驱动引擎一次。
///
/// 当引擎尚未创建时返回 `0`。
///
/// #### 返回
/// - 始终返回 `0`。
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
/// Initializes the thread-local engine (explicit initialization).
///
/// #### Returns
/// - `Ok(())` when the engine is ready on this thread.
/// - `Err(InitError)` when initialization fails.
///
/// ### 中文
/// 显式初始化线程本地引擎。
///
/// #### 返回
/// - 初始化成功（或已初始化）返回 `Ok(())`。
/// - 初始化失败返回 `Err(InitError)`。
pub(crate) fn init() -> Result<(), InitError> {
    ENGINE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_some() {
            return Ok(());
        }

        let glfw_get_proc_address = GLFW_GET_PROC_ADDRESS.load(Ordering::Relaxed);
        if glfw_get_proc_address == 0 {
            return Err(InitError::NullPointer {
                name: "glfwGetProcAddress",
            });
        }

        let gl_api = GL_API.load(Ordering::Relaxed);
        *slot = Some(Engine::new(glfw_get_proc_address, gl_api)?);
        Ok(())
    })
}

/// ### English
/// Creates a view (engine must be initialized explicitly by `init`).
///
/// #### Parameters
/// - `config`: View creation configuration.
///
/// #### Returns
/// - `Ok(NonNull<View>)` on success.
/// - `Err(InitError)` when initialization fails.
///
/// ### 中文
/// 创建 view（必须先显式调用 `init` 初始化引擎）。
///
/// #### 参数
/// - `config`：view 创建配置。
///
/// #### 返回
/// - 成功返回 `Ok(NonNull<View>)`。
/// - 初始化失败返回 `Err(InitError)`。
pub(crate) fn create_view(config: ViewConfig) -> Result<NonNull<View>, InitError> {
    ENGINE.with(|cell| {
        let mut engine_slot = cell.borrow_mut();
        let Some(engine) = engine_slot.as_mut() else {
            return Err(InitError::EngineNotInitialized);
        };
        Ok(engine.create_view(config))
    })
}

/// ### English
/// View creation configuration.
///
/// ### 中文
/// View 创建配置。
pub(crate) struct ViewConfig {
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
/// Servo event-loop waker used by this embedder.
///
/// This embedder is driven explicitly by the host (typically once per frame), so waking is a
/// no-op.
///
/// ### 中文
/// 本嵌入层使用的 Servo 事件循环唤醒器。
///
/// 本嵌入层由宿主显式驱动（通常每帧一次），因此 `wake` 为 no-op。
struct FlagWaker {
    pending: Arc<AtomicBool>,
}

impl servo::EventLoopWaker for FlagWaker {
    /// ### English
    /// Clones this waker as a boxed trait object.
    ///
    /// #### Returns
    /// - A new boxed waker.
    ///
    /// ### 中文
    /// 将该唤醒器克隆为装箱的 trait object。
    ///
    /// #### 返回
    /// - 新的装箱唤醒器。
    fn clone_box(&self) -> Box<dyn servo::EventLoopWaker> {
        Box::new(Self {
            pending: self.pending.clone(),
        })
    }

    /// ### English
    /// Wakes the event loop.
    ///
    /// ### 中文
    /// 唤醒事件循环。
    fn wake(&self) {
        // Servo uses this to tell the embedder that there is new work to process.
        self.pending.store(true, Ordering::Release);
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
}

impl DirtyTracker {
    /// ### English
    /// Creates a new delegate marked as dirty.
    ///
    /// #### Returns
    /// - A new `DirtyTracker` instance.
    ///
    /// ### 中文
    /// 创建新的 delegate（初始标记为 dirty）。
    ///
    /// #### 返回
    /// - 新的 `DirtyTracker` 实例。
    fn new() -> Self {
        Self {
            dirty: Cell::new(true),
        }
    }

    /// ### English
    /// Clears the dirty flag and returns whether it was previously set.
    ///
    /// #### Returns
    /// - `true` if the flag was dirty and is now cleared.
    /// - `false` if the flag was already clean.
    ///
    /// ### 中文
    /// 清除 dirty 标记，并返回清除前是否为 dirty。
    ///
    /// #### 返回
    /// - 之前为 dirty 且本次已清除时返回 `true`。
    /// - 之前已为 clean 时返回 `false`。
    fn take_dirty(&self) -> bool {
        self.dirty.replace(false)
    }

    /// ### English
    /// Marks the view as dirty.
    ///
    /// ### 中文
    /// 将 view 标记为 dirty。
    fn mark_dirty(&self) {
        if self.dirty.get() {
            return;
        }
        self.dirty.set(true);
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
    /// Refresh driver used by Servo to schedule frame callbacks.
    ///
    /// ### 中文
    /// Servo 用于调度帧回调的刷新驱动。
    frame_driver: Rc<FrameDriver>,

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

    /// Set to `true` when Servo wakes the embedder (new work available).
    wake_pending: Arc<AtomicBool>,
}

impl Engine {
    /// ### English
    /// Creates a new engine instance.
    ///
    /// #### Parameters
    /// - `glfw_get_proc_address`: Address of `glfwGetProcAddress` (as `uintptr_t`).
    /// - `gl_api`: OpenGL API selector (`XIAN_WEB_ENGINE_GL_API_*`).
    ///
    /// #### Returns
    /// - `Ok(Engine)` on success.
    /// - `Err(InitError)` on initialization failure.
    ///
    /// ### 中文
    /// 创建新的引擎实例。
    ///
    /// #### 参数
    /// - `glfw_get_proc_address`：`glfwGetProcAddress` 的地址（`uintptr_t`）。
    /// - `gl_api`：OpenGL API 选择值（`XIAN_WEB_ENGINE_GL_API_*`）。
    ///
    /// #### 返回
    /// - 成功返回 `Ok(Engine)`。
    /// - 初始化失败返回 `Err(InitError)`。
    pub(crate) fn new(glfw_get_proc_address: usize, gl_api: u32) -> Result<Self, InitError> {
        try_init_logging();

        RUSTLS_PROVIDER_INIT.call_once(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });

        let config_dir = config_dir();
        if let Some(ref config_dir) = config_dir {
            let _ = std::fs::create_dir_all(config_dir);
        }

        let loader = unsafe { GlProcLoader::from_raw(glfw_get_proc_address)? };
        let gl_api = GlApi::from_u32(gl_api)?;
        let gl = unsafe { GlHandles::new(gl_api, &loader)? };

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
            // Disable shader precache to avoid long stalls during startup/first page load.
            // This may shift some compilation cost to the first time a specific render path is used.
            gfx_precache_shaders: false,
            // Make the compositor clear color fully transparent so pages with transparent
            // canvas background render with alpha=0 instead of solid white.
            shell_background_color_rgba: [0.0, 0.0, 0.0, 0.0],
            layout_threads: tuned_threads,
            threadpools_fallback_worker_num: tuned_threads,
            threadpools_async_runtime_workers_max: tuned_threads,
            threadpools_image_cache_workers_max: tuned_threads,
            threadpools_webrender_workers_max: tuned_threads,
            threadpools_indexeddb_workers_max: tuned_threads,
            threadpools_webstorage_workers_max: tuned_threads,
            ..Default::default()
        };

        let wake_pending = Arc::new(AtomicBool::new(false));
        let waker: Box<dyn servo::EventLoopWaker> = Box::new(FlagWaker {
            pending: wake_pending.clone(),
        });

        let frame_driver = Rc::new(FrameDriver::new());

        if SERVO_INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(InitError::ServoAlreadyInitialized);
        }

        let mut protocol_registry = servo::protocol_handler::ProtocolRegistry::default();
        // Custom local protocol: `xian://` -> embedder-provided web root directory.
        protocol_registry
            .register("xian", crate::protocols::xian::XianProtocolHandler::default())
            .expect("Failed to register xian:// protocol handler");

        let servo = servo::ServoBuilder::default()
            .opts(opts)
            .preferences(preferences)
            .event_loop_waker(waker)
            .protocol_registry(protocol_registry)
            .build();

        Ok(Self {
            frame_driver,
            gl,
            servo,
            wake_pending,
        })
    }

    /// ### English
    /// Drives Servo once.
    ///
    /// #### Returns
    /// - Always returns `0` (painting is controlled explicitly by the embedder).
    ///
    /// ### 中文
    /// 驱动 Servo 一次。
    ///
    /// #### 返回
    /// - 始终返回 `0`（绘制由宿主显式控制）。
    pub(crate) fn tick(&mut self) -> u32 {
        self.frame_driver.begin_frame();
        // Our embedder is driven by Minecraft frames and does not have a real OS event loop.
        // Servo expects the embedder to wake and spin again when new work arrives; without this,
        // chained async tasks (timers/microtasks/network) can be artificially throttled to 1 tick/frame.
        //
        // We emulate a waker-driven loop by spinning a few extra times within a small time budget
        // when Servo requested a wake during the previous spin.
        const MAX_EXTRA_SPINS: usize = 16;
        const EXTRA_SPIN_BUDGET: Duration = Duration::from_millis(8);

        let start = Instant::now();

        self.wake_pending.store(false, Ordering::Release);
        self.servo.spin_event_loop();

        for _ in 0..MAX_EXTRA_SPINS {
            if !self.wake_pending.swap(false, Ordering::AcqRel) {
                break;
            }
            if start.elapsed() >= EXTRA_SPIN_BUDGET {
                break;
            }
            self.servo.spin_event_loop();
        }
        0
    }

    /// ### English
    /// Creates a new view.
    ///
    /// #### Parameters
    /// - `config`: View creation configuration.
    ///
    /// #### Returns
    /// - A non-null view pointer.
    ///
    /// ### 中文
    /// 创建新的 view。
    ///
    /// #### 参数
    /// - `config`：View 创建配置。
    ///
    /// #### 返回
    /// - 非空的 view 指针。
    pub(crate) fn create_view(&mut self, config: ViewConfig) -> NonNull<View> {
        let size = PhysicalSize::new(config.width.max(1), config.height.max(1));
        let hidpi_scale_factor =
            if config.hidpi_scale_factor.is_finite() && config.hidpi_scale_factor > 0.0 {
                config.hidpi_scale_factor
            } else {
                1.0
            };

        let rendering_context = Rc::new(TextureContext::new(
            self.gl.clone(),
            size,
            Some(self.frame_driver.clone() as Rc<dyn RefreshDriver>),
        ));

        let dirty_tracker = Rc::new(DirtyTracker::new());

        // Important: If we call `webview.load(...)` immediately after creating a WebView, the
        // navigation can be ignored because Constellation may already have a pending initial load
        // for the browsing context (see the `pending_changes` guard in Constellation::load_url).
        //
        // To avoid the "LoadUrl gets dropped and we stay on about:blank" race, pass the initial URL
        // to `WebViewBuilder` so it becomes the WebView's *initial* navigation via `NewWebView`.
        let mut builder = WebViewBuilder::new(&self.servo, rendering_context.clone())
            .hidpi_scale_factor(euclid::Scale::new(hidpi_scale_factor))
            .delegate(dirty_tracker.clone());

        if let Some(url) = config.initial_url {
            builder = builder.url(url);
        }

        let webview = builder.build();

        webview.show();

        let view = Box::new(View {
            webview,
            rendering_context,
            dirty_tracker,
            last_logged_device_size: Cell::new((0, 0)),
        });

        NonNull::from(Box::leak(view))
    }
}

#[repr(C)]
/// ### English
/// Opaque view handle returned by the C ABI.
///
/// #### Threading
/// - Must be used on the same thread where the embedder OpenGL context is current.
///
/// #### Ownership
/// - Created by `xian_web_engine_view_create`.
/// - Destroyed by `xian_web_engine_view_destroy`.
///
/// ### 中文
/// C ABI 返回的“不透明”view 句柄。
///
/// #### 线程
/// - 必须在宿主 OpenGL 上下文为 current 的同一线程使用。
///
/// #### 所有权
/// - 由 `xian_web_engine_view_create` 创建。
/// - 由 `xian_web_engine_view_destroy` 销毁。
pub struct View {
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

    /// Last size we logged for this view (in device pixels), used to avoid spamming logs.
    last_logged_device_size: Cell<(u32, u32)>,
}

impl View {
    fn log_device_size_if_changed(&self, reason: &'static str) {
        let size = self.webview.size();
        let w = size.width.round().max(0.0) as u32;
        let h = size.height.round().max(0.0) as u32;

        let prev = self.last_logged_device_size.get();
        if prev == (w, h) {
            return;
        }
        self.last_logged_device_size.set((w, h));

        let hidpi = self.webview.hidpi_scale_factor().0;
        let css_w = if hidpi.is_finite() && hidpi > 0.0 {
            size.width / hidpi
        } else {
            size.width
        };
        let css_h = if hidpi.is_finite() && hidpi > 0.0 {
            size.height / hidpi
        } else {
            size.height
        };

        log::warn!(
            target: "xian::webview",
            "Servo WebView size={}x{} device_px, hidpi_scale_factor={}, css_px={:.1}x{:.1} (reason={}, texture_id={})",
            w,
            h,
            hidpi,
            css_w,
            css_h,
            reason,
            self.rendering_context.texture_id()
        );
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
        // Ensure we repaint at least once even if the embedder missed a frame-ready notification.
        self.dirty_tracker.mark_dirty();
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
        // Resizing changes the render target; force a repaint so the new texture isn't left blank.
        self.dirty_tracker.mark_dirty();
        self.log_device_size_if_changed("resize");
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
        // HiDPI affects CSS<->device pixel conversion; ensure the next frame repaints.
        self.dirty_tracker.mark_dirty();
        true
    }

    /// ### English
    /// Marks the view as throttled/unthrottled.
    ///
    /// When throttled, Servo may reduce work for animations and timers.
    ///
    /// ### 中文
    /// 设置该 view 是否节流（throttled）。
    ///
    /// 当节流时，Servo 可能会减少动画/计时器等工作量，从而降低 CPU 占用。
    pub(crate) fn set_throttled(&self, throttled: bool) {
        self.webview.set_throttled(throttled);
        // When becoming active again, force at least one repaint so the embedder can refresh
        // immediately even if a frame-ready notification is delayed.
        if !throttled {
            self.dirty_tracker.mark_dirty();
        }
    }

    /// ### English
    /// Returns the OpenGL texture id of this view.
    ///
    /// #### Notes
    /// - The texture id is stable across `resize`.
    ///
    /// #### Returns
    /// - OpenGL texture id.
    ///
    /// ### 中文
    /// 返回该 view 的 OpenGL 纹理 id。
    ///
    /// #### 说明
    /// - `resize` 过程中会原地调整纹理存储，因此纹理 id 保持不变。
    ///
    /// #### 返回
    /// - OpenGL 纹理 id。
    pub(crate) fn texture_id(&self) -> u32 {
        self.rendering_context.texture_id()
    }

    /// ### English
    /// Returns the current `LoadStatus` of this view.
    ///
    /// ### 中文
    /// 返回该 view 当前的 `LoadStatus`。
    pub(crate) fn load_status(&self) -> servo::LoadStatus {
        self.webview.load_status()
    }

    /// ### English
    /// Returns the current URL of this view (if any).
    ///
    /// ### 中文
    /// 返回该 view 当前的 URL（若存在）。
    pub(crate) fn current_url(&self) -> Option<url::Url> {
        self.webview.url()
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
        self.log_device_size_if_changed("paint");

        // Only repaint when Servo says a new frame is ready. The embedder can still call `paint()`
        // every frame; we will cheaply return `false` when nothing changed.
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
