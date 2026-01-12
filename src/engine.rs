//! ### English
//! High-performance Servo embedding core (single-threaded public API).
//!
//! ### 中文
//! 最高性能 Servo 嵌入核心（对外 API 单线程）。

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Once, RwLock};

use dpi::PhysicalSize;
use servo::{RefreshDriver, WebView, WebViewBuilder, WebViewDelegate};

use crate::error::EngineInitError;
use crate::input::{XianWebEngineInputEvent, map_input_event};
use crate::rendering::{GlApi, GlShared, GlfwContext, GlfwTextureContext};

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
/// ### 中文
/// 返回 Servo 是否已在本进程中初始化。
pub(crate) fn is_initialized() -> bool {
    SERVO_INITIALIZED.load(Ordering::Relaxed)
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
    if is_initialized() {
        return false;
    }
    *CONFIG_DIR.write().unwrap() = path;
    true
}

#[inline]
/// ### English
/// Gets the process-global Servo config directory override.
///
/// ### 中文
/// 获取进程全局的 Servo 配置目录覆盖值。
pub(crate) fn config_dir() -> Option<PathBuf> {
    CONFIG_DIR.read().unwrap().clone()
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
    if is_initialized() {
        return false;
    }
    THREAD_POOL_CAP.store(cap, Ordering::Relaxed);
    true
}

#[inline]
/// ### English
/// Gets the process-global worker thread cap (`0` = no cap).
///
/// ### 中文
/// 获取进程全局的工作线程上限（`0` = 不限制）。
pub(crate) fn thread_pool_cap() -> u32 {
    THREAD_POOL_CAP.load(Ordering::Relaxed)
}

/// ### English
/// Parameters for creating an engine (parsed from the C ABI config).
///
/// ### 中文
/// 引擎创建参数（由 C ABI 配置解析而来）。
pub(crate) struct EngineCreateParams {
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
pub(crate) struct ViewCreateParams {
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
}

#[derive(Clone, Copy)]
/// ### English
/// ### English
/// Engine behavior toggles (kept small for hot-path checks).
///
/// ### 中文
/// 引擎行为开关（保持精简以便热路径判断）。
struct EngineOptions {
    /// ### English
    /// Whether to assume the embedder context is already current on the calling thread.
    ///
    /// ### 中文
    /// 是否假定宿主上下文已在调用线程 current。
    assume_context_current: bool,

    /// ### English
    /// Whether to paint dirty views automatically in `tick`.
    ///
    /// ### 中文
    /// 是否在 `tick` 中自动绘制 dirty view。
    auto_paint: bool,
}

#[derive(Clone)]
/// ### English
/// Servo event-loop waker that flips a shared atomic flag.
///
/// ### 中文
/// Servo 事件循环唤醒器：翻转一个共享的原子标记。
struct PendingWaker {
    /// ### English
    /// `true` means the engine likely has pending work and should be ticked.
    ///
    /// ### 中文
    /// `true` 表示引擎可能有待处理工作，应进行 tick。
    tick_pending: Arc<AtomicBool>,
}

impl servo::EventLoopWaker for PendingWaker {
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
struct EngineRefreshDriver {
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

impl EngineRefreshDriver {
    /// ### English
    /// Creates a new refresh driver.
    ///
    /// ### 中文
    /// 创建新的刷新驱动。
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

impl RefreshDriver for EngineRefreshDriver {
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
struct ViewDirtyDelegate {
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
    dirty_view_count: Rc<Cell<usize>>,
}

impl ViewDirtyDelegate {
    /// ### English
    /// Creates a new delegate marked as dirty.
    ///
    /// This increments the shared dirty-view counter.
    ///
    /// #### Parameters
    /// - `dirty_view_count`: Shared dirty-view counter owned by the engine.
    ///
    /// ### 中文
    /// 创建新的 delegate（初始标记为 dirty）。
    ///
    /// 该调用会递增引擎共享的 dirty-view 计数。
    ///
    /// #### 参数
    /// - `dirty_view_count`：引擎持有的共享 dirty-view 计数器。
    fn new(dirty_view_count: Rc<Cell<usize>>) -> Self {
        dirty_view_count.set(dirty_view_count.get().saturating_add(1));
        Self {
            dirty: Cell::new(true),
            dirty_view_count,
        }
    }

    /// ### English
    /// Returns whether the view needs painting.
    ///
    /// ### 中文
    /// 返回该 view 是否需要绘制。
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

        let count = self.dirty_view_count.get();
        debug_assert!(count > 0);
        self.dirty_view_count.set(count.saturating_sub(1));
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
        self.dirty_view_count
            .set(self.dirty_view_count.get().saturating_add(1));
    }
}

impl WebViewDelegate for ViewDirtyDelegate {
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

#[repr(C)]
/// ### English
/// Opaque engine handle.
///
/// ### 中文
/// 不透明 engine 句柄。
pub struct XianWebEngine {
    /// ### English
    /// Engine options captured at creation time.
    ///
    /// ### 中文
    /// 创建时确定的引擎选项。
    options: EngineOptions,

    /// ### English
    /// Flag flipped by Servo's waker; used by `needs_tick`.
    ///
    /// ### 中文
    /// 由 Servo 的 waker 翻转的标记；供 `needs_tick` 判断使用。
    tick_pending: Arc<AtomicBool>,

    /// ### English
    /// Refresh driver used by Servo to schedule frame callbacks.
    ///
    /// ### 中文
    /// Servo 用于调度帧回调的刷新驱动。
    refresh_driver: Rc<EngineRefreshDriver>,

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
    gl: GlShared,

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

    /// ### English
    /// Number of views currently marked as dirty (has a frame ready).
    ///
    /// This is updated by each `ViewDirtyDelegate` so `needs_tick` can stay O(1) when `auto_paint`
    /// is enabled.
    ///
    /// ### 中文
    /// 当前被标记为 dirty（已生成新帧）的 view 数量。
    ///
    /// 该计数由每个 `ViewDirtyDelegate` 维护，使得在开启 `auto_paint` 时 `needs_tick` 保持 O(1)。
    dirty_view_count: Rc<Cell<usize>>,
}

impl XianWebEngine {
    /// ### English
    /// Creates a new engine instance.
    ///
    /// #### Parameters
    /// - `params`: Parsed creation parameters.
    ///
    /// #### Returns
    /// - `Ok(XianWebEngine)` on success.
    /// - `Err(EngineInitError)` on initialization failure.
    ///
    /// ### 中文
    /// 创建新的引擎实例。
    ///
    /// #### 参数
    /// - `params`：已解析的创建参数。
    ///
    /// #### 返回
    /// - 成功返回 `Ok(XianWebEngine)`。
    /// - 初始化失败返回 `Err(EngineInitError)`。
    pub(crate) fn new(params: EngineCreateParams) -> Result<Self, EngineInitError> {
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

        let options = EngineOptions {
            assume_context_current: params.assume_context_current,
            auto_paint: params.auto_paint,
        };

        let glfw = unsafe {
            GlfwContext::from_raw(
                params.glfw_window,
                params.glfw_get_proc_address,
                params.glfw_make_context_current,
                options.assume_context_current,
            )?
        };

        unsafe { glfw.make_current()? };

        let gl_api = GlApi::from_u32(params.gl_api)?;
        let gl = unsafe { GlShared::new(gl_api, &glfw)? };

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
        let waker: Box<dyn servo::EventLoopWaker> = Box::new(PendingWaker {
            tick_pending: tick_pending.clone(),
        });

        let refresh_driver = Rc::new(EngineRefreshDriver::new());

        if SERVO_INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(EngineInitError::ServoAlreadyInitialized);
        }

        let servo = servo::ServoBuilder::default()
            .opts(opts)
            .preferences(preferences)
            .event_loop_waker(waker)
            .build();

        Ok(Self {
            options,
            tick_pending,
            refresh_driver,
            glfw,
            gl,
            servo,
            views: Vec::new(),
            dirty_view_count: Rc::new(Cell::new(0)),
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

        if self.options.auto_paint && self.dirty_view_count.get() != 0 {
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
        self.refresh_driver.begin_frame();
        self.servo.spin_event_loop();

        if !self.options.auto_paint {
            return 0;
        }

        let mut painted = 0;
        for &ptr in &self.views {
            let view = unsafe { ptr.as_ref() };
            if view.paint() {
                painted += 1;
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
    pub(crate) fn create_view(&mut self, params: ViewCreateParams) -> NonNull<XianWebEngineView> {
        let size = PhysicalSize::new(params.width.max(1), params.height.max(1));

        let rendering_context = Rc::new(GlfwTextureContext::new(
            self.glfw,
            self.gl.clone(),
            size,
            Some(self.refresh_driver.clone() as Rc<dyn RefreshDriver>),
        ));

        let delegate = Rc::new(ViewDirtyDelegate::new(Rc::clone(&self.dirty_view_count)));

        let webview = WebViewBuilder::new(&self.servo, rendering_context.clone())
            .delegate(delegate.clone())
            .build();

        webview.show();

        if let Some(url) = params.initial_url {
            webview.load(url);
        }

        let engine_ptr = NonNull::from(&mut *self);

        let view = Box::new(XianWebEngineView {
            engine: Cell::new(Some(engine_ptr)),
            webview,
            rendering_context,
            delegate,
        });

        let ptr = NonNull::from(Box::leak(view));
        self.views.push(ptr);
        ptr
    }

    /// ### English
    /// Detaches all views from this engine (used before engine destruction).
    ///
    /// ### 中文
    /// 将所有 view 与该引擎解绑（用于引擎销毁前）。
    pub(crate) fn detach_all_views(&mut self) {
        for &ptr in &self.views {
            let view = unsafe { ptr.as_ref() };
            view.engine.set(None);
        }
        self.views.clear();
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
/// Opaque view handle.
///
/// ### 中文
/// 不透明 view 句柄。
pub struct XianWebEngineView {
    /// ### English
    /// Optional back-pointer to the owning engine (cleared when detached).
    ///
    /// ### 中文
    /// 指向所属引擎的可选反向指针（解绑时会清空）。
    engine: Cell<Option<NonNull<XianWebEngine>>>,

    /// ### English
    /// Servo WebView instance.
    ///
    /// ### 中文
    /// Servo WebView 实例。
    webview: WebView,

    /// ### English
    /// Rendering context that owns the offscreen texture for this view.
    ///
    /// ### 中文
    /// 持有该 view 离屏纹理的渲染上下文。
    rendering_context: Rc<GlfwTextureContext>,

    /// ### English
    /// Dirty-tracking delegate.
    ///
    /// ### 中文
    /// 用于 dirty 跟踪的 delegate。
    delegate: Rc<ViewDirtyDelegate>,
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
    pub(crate) fn destroy_boxed(mut view: Box<Self>) {
        view.delegate.clear();

        let view_ptr = NonNull::new(view.as_mut() as *mut XianWebEngineView)
            .expect("NonNull from Box is guaranteed");

        if let Some(mut engine) = view.engine.get() {
            unsafe { engine.as_mut() }.unregister_view(view_ptr);
        }
        view.engine.set(None);
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
    /// Returns the OpenGL texture id of this view.
    ///
    /// ### 中文
    /// 返回该 view 的 OpenGL 纹理 id。
    pub(crate) fn texture_id(&self) -> u32 {
        self.rendering_context.texture_id()
    }

    /// ### English
    /// Returns whether this view needs painting.
    ///
    /// ### 中文
    /// 返回该 view 是否需要绘制。
    pub(crate) fn needs_paint(&self) -> bool {
        self.delegate.is_dirty()
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
        if !self.delegate.take_dirty() {
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
    /// - Number of events accepted.
    ///
    /// ### 中文
    /// 向该 view 发送一批输入事件。
    ///
    /// #### 参数
    /// - `events`：ABI 输入事件切片。
    ///
    /// #### 返回
    /// - 被接受的事件数量。
    pub(crate) fn send_input_events(&self, events: &[XianWebEngineInputEvent]) -> u32 {
        let mut accepted = 0u32;
        for e in events {
            let Some(event) = map_input_event(e) else {
                continue;
            };
            let _ = self.webview.notify_input_event(event);
            accepted += 1;
        }
        accepted
    }
}
