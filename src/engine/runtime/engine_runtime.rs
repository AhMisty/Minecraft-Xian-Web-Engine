//! ### English
//! Engine runtime that spawns and owns the dedicated Servo thread.
//!
//! ### 中文
//! 创建并持有独立 Servo 线程的引擎运行时。

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use dpi::PhysicalSize;

use crate::engine::flags;
use crate::engine::frame::SharedFrameState;
use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};
use crate::engine::lockfree::OneShot;
use crate::engine::vsync::VsyncCallbackQueue;

use super::coalesced::{CoalescedLoadUrl, PendingWork};
use super::command::Command;
use super::pending::PendingIdQueue;
use super::queue::CommandQueue;
use super::servo_thread;
use super::view_handle::{WebEngineViewHandle, WebEngineViewHandleInit};

/// ### English
/// Engine runtime that owns the dedicated Servo thread.
///
/// ### 中文
/// 持有独立 Servo 线程的运行时。
pub struct EngineRuntime {
    /// ### English
    /// Default view size used when the embedder passes an invalid size.
    ///
    /// ### 中文
    /// 宿主传入无效尺寸时使用的默认 view 尺寸。
    default_size: PhysicalSize<u32>,
    /// ### English
    /// Command queue for control messages into the Servo thread.
    ///
    /// ### 中文
    /// 发送到 Servo 线程的控制命令队列。
    command_queue: Arc<CommandQueue>,
    /// ### English
    /// Join handle for the dedicated Servo thread (owned by this runtime).
    ///
    /// ### 中文
    /// 独立 Servo 线程的 join handle（由本运行时持有）。
    thread: Option<thread::JoinHandle<()>>,
    /// ### English
    /// Thread handle used to wake the Servo thread.
    ///
    /// ### 中文
    /// 用于唤醒 Servo 线程的线程句柄。
    thread_handle: thread::Thread,
    /// ### English
    /// Queue of vsync callbacks produced by Servo and consumed by the embedder tick.
    ///
    /// ### 中文
    /// Vsync 回调队列：由 Servo 产生、由宿主 tick 消费。
    vsync_queue: Arc<VsyncCallbackQueue>,
    /// ### English
    /// Pending view-id queue used to coalesce per-view work scheduling.
    ///
    /// ### 中文
    /// pending view-id 队列：用于合并每 view 的工作调度。
    pending_queue: Arc<PendingIdQueue>,
}

impl EngineRuntime {
    /// ### English
    /// Creates a new engine runtime and initializes the dedicated Servo thread.
    ///
    /// `glfw_shared_window` must be the embedder-owned GLFW window whose context will be shared.
    /// This function blocks until the Servo thread finishes initialization (or times out).
    ///
    /// `thread_pool_cap` controls the maximum worker threads used by Servo's internal thread pools.
    /// `0` means "no cap" (use CPU parallelism).
    ///
    /// #### Parameters
    /// - `glfw_shared_window`: Embedder-owned GLFW window whose context will be shared with the Servo thread.
    /// - `default_size`: Fallback view size used when the embedder passes an invalid size.
    /// - `resources_dir`: Optional resource directory override.
    /// - `config_dir`: Optional config directory override.
    /// - `thread_pool_cap`: Servo worker thread cap (`0` means no cap).
    ///
    /// ### 中文
    /// 创建一个新的引擎运行时，并初始化独立的 Servo 线程。
    ///
    /// `glfw_shared_window` 必须是宿主侧持有、用于共享上下文的 GLFW window。
    /// 该函数会阻塞等待 Servo 线程完成初始化（或超时）。
    ///
    /// `thread_pool_cap` 用于限制 Servo 内部线程池的最大工作线程数；
    /// `0` 表示“不封顶”（使用 CPU 并行度）。
    ///
    /// #### 参数
    /// - `glfw_shared_window`：宿主侧 GLFW window；其上下文会与 Servo 线程共享。
    /// - `default_size`：当宿主传入无效尺寸时使用的兜底尺寸。
    /// - `resources_dir`：可选的资源目录覆盖。
    /// - `config_dir`：可选的配置目录覆盖。
    /// - `thread_pool_cap`：Servo 工作线程上限（`0` 表示不封顶）。
    pub fn new(
        glfw_shared_window: *mut c_void,
        default_size: PhysicalSize<u32>,
        resources_dir: Option<PathBuf>,
        config_dir: Option<PathBuf>,
        thread_pool_cap: u32,
    ) -> Result<Self, String> {
        let glfw_shared_window_handle = glfw_shared_window as usize;

        let vsync_queue = Arc::new(VsyncCallbackQueue::with_capacity(4096));
        let vsync_queue_for_thread = vsync_queue.clone();

        let pending_queue = Arc::new(PendingIdQueue::with_capacity(64 * 1024));
        let pending_queue_for_thread = pending_queue.clone();

        let command_queue = Arc::new(CommandQueue::new());
        let command_queue_for_thread = command_queue.clone();

        let init = Arc::new(OneShot::new(thread::current()));
        let init_for_thread = init.clone();

        let thread = thread::spawn(move || {
            servo_thread::run_servo_thread(
                glfw_shared_window_handle,
                resources_dir,
                config_dir,
                vsync_queue_for_thread,
                pending_queue_for_thread,
                command_queue_for_thread,
                thread_pool_cap,
                init_for_thread,
            );
        });

        let thread_handle = thread.thread().clone();

        match init.recv_timeout(Duration::from_secs(30)) {
            Some(Ok(())) => Ok(Self {
                default_size,
                command_queue,
                thread: Some(thread),
                thread_handle,
                vsync_queue,
                pending_queue,
            }),
            Some(Err(err)) => {
                thread_handle.unpark();
                let _ = thread.join();
                Err(err)
            }
            None => {
                command_queue.push(Command::Shutdown);
                thread_handle.unpark();
                let _ = thread.join();
                Err("Timed out initializing Servo thread".to_string())
            }
        }
    }

    /// ### English
    /// Creates one view by sending a `CreateView` command to the Servo thread.
    ///
    /// The returned `WebEngineViewHandle` is thread-safe for the embedder thread to use.
    ///
    /// #### Parameters
    /// - `initial_size`: Requested initial view size (0 is treated as `default_size`).
    /// - `target_fps`: Target FPS for fixed-interval refresh (0 means external-vsync mode).
    /// - `view_flags`: Bitflags controlling safety/performance trade-offs.
    ///
    /// ### 中文
    /// 通过向 Servo 线程发送 `CreateView` 命令来创建一个 view。
    ///
    /// 返回的 `WebEngineViewHandle` 可供宿主线程安全使用。
    ///
    /// #### 参数
    /// - `initial_size`：请求的初始尺寸（为 0 时使用 `default_size`）。
    /// - `target_fps`：固定间隔 refresh 的目标 FPS（0 表示外部 vsync 模式）。
    /// - `view_flags`：控制安全/性能权衡的位标志。
    pub fn create_view(
        &self,
        initial_size: PhysicalSize<u32>,
        target_fps: u32,
        view_flags: u32,
    ) -> Result<WebEngineViewHandle, String> {
        if self.thread.is_none() {
            return Err("Engine is shut down".to_string());
        }

        let unsafe_no_consumer_fence =
            (view_flags & flags::XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE) != 0;
        let unsafe_no_producer_fence =
            (view_flags & flags::XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_PRODUCER_FENCE) != 0;
        let input_single_producer =
            (view_flags & flags::XIAN_WEB_ENGINE_VIEW_FLAG_INPUT_SINGLE_PRODUCER) != 0;

        let initial_size = if initial_size.width == 0 || initial_size.height == 0 {
            self.default_size
        } else {
            initial_size
        };
        let initial_size = PhysicalSize::new(initial_size.width.max(1), initial_size.height.max(1));

        let shared = Arc::new(SharedFrameState::new(initial_size));
        let mouse_move = Arc::new(CoalescedMouseMove::default());
        let resize = Arc::new(CoalescedResize::default());
        let input_queue = Arc::new(InputEventQueue::new(input_single_producer));
        let load_url = Arc::new(CoalescedLoadUrl::default());
        let pending = Arc::new(PendingWork::default());

        let response = Arc::new(OneShot::new(thread::current()));

        if !self.command_queue.try_push(Command::CreateView {
            initial_size,
            shared: shared.clone(),
            mouse_move: mouse_move.clone(),
            resize: resize.clone(),
            input_queue: input_queue.clone(),
            load_url: load_url.clone(),
            pending: pending.clone(),
            target_fps,
            unsafe_no_consumer_fence,
            unsafe_no_producer_fence,
            response: response.clone(),
        }) {
            return Err("Engine is shutting down".to_string());
        }
        self.thread_handle.unpark();

        match response.recv_timeout(Duration::from_secs(30)) {
            Some(Ok((id, token))) => Ok(WebEngineViewHandle::new(WebEngineViewHandleInit {
                id,
                token,
                shared,
                mouse_move,
                resize,
                input_queue,
                load_url,
                pending,
                pending_queue: self.pending_queue.clone(),
                command_queue: self.command_queue.clone(),
                thread_handle: self.thread_handle.clone(),
                unsafe_no_consumer_fence,
            })),
            Some(Err(err)) => Err(err),
            None => Err("Timed out creating view".to_string()),
        }
    }

    /// ### English
    /// Drains pending vsync callbacks (used by the Java side to drive Servo refresh).
    ///
    /// ### 中文
    /// drain pending vsync 回调（供 Java 侧驱动 Servo refresh）。
    pub fn tick(&self) {
        self.vsync_queue.tick();
    }

    /// ### English
    /// Requests Servo thread shutdown and joins it.
    ///
    /// ### 中文
    /// 请求 Servo 线程退出并 join。
    pub fn shutdown(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.command_queue.push(Command::Shutdown);
            self.thread_handle.unpark();
            let _ = thread.join();
            self.command_queue.close();
        }
    }
}

impl Drop for EngineRuntime {
    /// ### English
    /// Ensures the Servo thread is shut down when the runtime is dropped.
    ///
    /// ### 中文
    /// 确保在运行时 drop 时关闭 Servo 线程。
    fn drop(&mut self) {
        self.shutdown();
    }
}
