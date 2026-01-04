//! ### English
//! Dedicated Servo thread: owns the shared GL context and drives Servo's event loop.
//!
//! ### 中文
//! 独立 Servo 线程：持有共享 GL 上下文并驱动 Servo 事件循环。
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

use crate::engine::lockfree::OneShot;
use crate::engine::refresh::RefreshScheduler;
use crate::engine::rendering::GlfwSharedContext;
use crate::engine::resources;
use crate::engine::vsync::VsyncCallbackQueue;

use super::pending::PendingIdQueue;
use super::queue::CommandQueue;

use view::ViewEntry;

mod commands;
mod view;

/// ### English
/// Servo thread entry function.
/// This function never returns until `Shutdown` or initialization failure.
///
/// Main phases:
///
/// 1. Install process-wide rustls provider (best-effort).
/// 2. Apply optional resource/config directories.
/// 3. Build Servo with a coalescing thread waker.
/// 4. Create a shared offscreen GLFW context (shares objects with the embedder window).
/// 5. Run the main loop:
///    - Drain control commands
///    - Process per-view pending work
///    - Spin Servo's internal event loop
///    - Park until woken
///
/// Threading notes:
/// - Servo's internal worker thread pools can be tuned via the embedder's ABI configuration.
///   `thread_pool_cap = 0` means "no cap" (use CPU parallelism); otherwise we cap to
///   `min(CPU, thread_pool_cap)`.
///
/// #### Parameters
/// - `glfw_shared_window_handle`: Embedder GLFW window handle whose context will be shared.
/// - `resources_dir`: Optional resource directory override.
/// - `config_dir`: Optional Servo config directory override.
/// - `vsync_queue`: Vsync callback queue used by Servo refresh.
/// - `pending_queue`: Pending view-id queue used to schedule per-view work.
/// - `command_queue`: Control-command queue from embedder threads.
/// - `thread_pool_cap`: Servo worker thread cap (`0` means no cap).
/// - `init`: One-shot used to report initialization success/failure to the spawner.
///
/// ### 中文
/// Servo 线程入口函数。
/// 除非收到 `Shutdown` 或初始化失败，否则不会返回。
///
/// 主要阶段：
///
/// 1. 进程内一次性安装 rustls provider（尽力而为）。
/// 2. 应用可选的资源/配置目录。
/// 3. 构建 Servo，并使用合并唤醒的线程 waker。
/// 4. 创建共享的离屏 GLFW 上下文（与宿主 window 共享对象）。
/// 5. 进入主循环：
///    - drain 控制命令
///    - 处理每 view 的 pending work
///    - 驱动 Servo 内部事件循环
///    - park 等待唤醒
///
/// 线程说明：
/// - Servo 内部工作线程池可通过宿主侧 ABI 配置调优：
///   `thread_pool_cap = 0` 表示“不封顶”（使用 CPU 并行度）；否则上限为 `min(CPU, thread_pool_cap)`。
///
/// #### 参数
/// - `glfw_shared_window_handle`：宿主 GLFW window 的句柄；其上下文会与 Servo 线程共享。
/// - `resources_dir`：可选的资源目录覆盖。
/// - `config_dir`：可选的 Servo 配置目录覆盖。
/// - `vsync_queue`：Servo refresh 使用的 vsync 回调队列。
/// - `pending_queue`：用于调度每 view 工作的 pending view-id 队列。
/// - `command_queue`：来自宿主线程的控制命令队列。
/// - `thread_pool_cap`：Servo 工作线程上限（`0` 表示不封顶）。
/// - `init`：用于向创建方回报初始化成功/失败的一次性通道。
#[allow(clippy::too_many_arguments)]
pub(super) fn run_servo_thread(
    glfw_shared_window_handle: usize,
    resources_dir: Option<PathBuf>,
    config_dir: Option<PathBuf>,
    vsync_queue: Arc<VsyncCallbackQueue>,
    pending_queue: Arc<PendingIdQueue>,
    command_queue: Arc<CommandQueue>,
    thread_pool_cap: u32,
    init: Arc<OneShot<Result<(), String>>>,
) {
    /// ### English
    /// Install rustls provider once per process (Servo uses it internally).
    ///
    /// ### 中文
    /// 全进程只安装一次 rustls provider（Servo 内部会使用）。
    static RUSTLS_PROVIDER_INSTALLED: AtomicBool = AtomicBool::new(false);
    if RUSTLS_PROVIDER_INSTALLED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }

    if let Some(resources_dir) = resources_dir {
        resources::set_resources_dir(resources_dir);
    }
    if let Some(ref config_dir) = config_dir {
        let _ = std::fs::create_dir_all(config_dir);
    }

    let wake_pending = Arc::new(AtomicBool::new(false));

    #[derive(Clone)]
    /// ### English
    /// Servo event-loop waker that unparks the Servo thread with a coalescing flag.
    ///
    /// ### 中文
    /// Servo 事件循环 waker：通过合并标记避免频繁 unpark，并唤醒 Servo 线程。
    struct ThreadWaker {
        /// ### English
        /// The Servo thread to unpark.
        ///
        /// ### 中文
        /// 需要 unpark 的 Servo 线程。
        thread: thread::Thread,
        /// ### English
        /// Coalesced "wake pending" flag to avoid unpark storms.
        ///
        /// ### 中文
        /// 合并 “wake pending” 标记，用于避免 unpark 风暴。
        pending: Arc<AtomicBool>,
    }

    impl servo::EventLoopWaker for ThreadWaker {
        /// ### English
        /// Clones this waker as a boxed trait object.
        ///
        /// ### 中文
        /// 将该 waker 克隆为 boxed trait object。
        fn clone_box(&self) -> Box<dyn servo::EventLoopWaker> {
            Box::new(self.clone())
        }

        /// ### English
        /// Requests a wakeup; coalesces multiple wakeups into a single `unpark`.
        ///
        /// ### 中文
        /// 请求唤醒；将多次唤醒合并为一次 `unpark`。
        fn wake(&self) {
            if !self.pending.swap(true, Ordering::Relaxed) {
                self.thread.unpark();
            }
        }
    }

    let waker: Box<dyn servo::EventLoopWaker> = Box::new(ThreadWaker {
        thread: thread::current(),
        pending: wake_pending.clone(),
    });

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

    let cpu_threads = std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(3)
        .max(1);
    let tuned_threads = if thread_pool_cap == 0 {
        cpu_threads
    } else {
        cpu_threads.min(thread_pool_cap as i64).max(1)
    };

    let preferences = servo::Preferences {
        gfx_precache_shaders: true,
        layout_threads: tuned_threads,
        threadpools_fallback_worker_num: tuned_threads,
        threadpools_async_runtime_workers_max: tuned_threads,
        threadpools_image_cache_workers_max: tuned_threads,
        threadpools_resource_workers_max: tuned_threads,
        threadpools_webrender_workers_max: tuned_threads,
        threadpools_indexeddb_workers_max: tuned_threads,
        threadpools_webstorage_workers_max: tuned_threads,
        ..Default::default()
    };

    let servo = servo::ServoBuilder::default()
        .opts(opts)
        .preferences(preferences)
        .event_loop_waker(waker)
        .build();

    let glfw_shared_window_ptr = glfw_shared_window_handle as *mut c_void;
    let shared_ctx = match GlfwSharedContext::new(glfw_shared_window_ptr) {
        Ok(ctx) => ctx,
        Err(err) => {
            let _ = init.send(Err(err));
            return;
        }
    };

    let _ = init.send(Ok(()));

    let mut views: Vec<Option<ViewEntry>> = Vec::with_capacity(64);
    let mut free_view_ids: Vec<u32> = Vec::new();
    let mut next_view_id: u32 = 1;
    let mut next_view_token: u64 = 1;
    let mut refresh_scheduler: Option<Arc<RefreshScheduler>> = None;

    loop {
        if commands::drain_commands(
            &servo,
            &shared_ctx,
            &vsync_queue,
            &command_queue,
            &mut refresh_scheduler,
            &mut views,
            &mut free_view_ids,
            &mut next_view_id,
            &mut next_view_token,
        ) {
            return;
        }

        while let Some(id) = pending_queue.pop() {
            let Some(entry) = views.get_mut(id as usize).and_then(Option::as_mut) else {
                continue;
            };
            entry.process_pending();
        }

        if pending_queue.take_overflowed() {
            for entry in views.iter_mut().filter_map(Option::as_mut) {
                entry.process_pending();
            }
        }

        servo.spin_event_loop();

        if wake_pending.swap(false, Ordering::Relaxed) {
            continue;
        }

        thread::park();
    }
}
