/// ### English
/// Dedicated Servo thread: owns the shared GL context and drives Servo's event loop.
///
/// ### 中文
/// 独立 Servo 线程：持有共享 GL 上下文并驱动 Servo 事件循环。
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
/// ### 中文
/// Servo 线程入口函数。
/// 除非收到 `Shutdown` 或初始化失败，否则不会返回。
#[allow(clippy::too_many_arguments)]
pub(super) fn run_servo_thread(
    glfw_shared_window_handle: usize,
    resources_dir: Option<PathBuf>,
    config_dir: Option<PathBuf>,
    vsync_queue: Arc<VsyncCallbackQueue>,
    pending_queue: Arc<PendingIdQueue>,
    command_queue: Arc<CommandQueue>,
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

    /// ### English
    /// Optional resource/config directories.
    ///
    /// ### 中文
    /// 可选的资源/配置目录。
    if let Some(resources_dir) = resources_dir {
        resources::set_resources_dir(resources_dir);
    }
    if let Some(ref config_dir) = config_dir {
        let _ = std::fs::create_dir_all(config_dir);
    }

    /// ### English
    /// Coalesced wake flag to avoid unpark storms.
    ///
    /// ### 中文
    /// 合并唤醒标记，避免频繁 unpark 的风暴。
    let wake_pending = Arc::new(AtomicBool::new(false));

    #[derive(Clone)]
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
        fn clone_box(&self) -> Box<dyn servo::EventLoopWaker> {
            Box::new(self.clone())
        }

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

    /// ### English
    /// Servo opts/preferences chosen for low overhead in an embedder.
    ///
    /// ### 中文
    /// 为宿主嵌入场景选择的 Servo 参数/偏好设置（低开销优先）。
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
        ..Default::default()
    };

    let servo = servo::ServoBuilder::default()
        .opts(opts)
        .preferences(preferences)
        .event_loop_waker(waker)
        .build();

    /// ### English
    /// Create a shared offscreen GLFW window/context that shares objects with Java's context.
    ///
    /// ### 中文
    /// 创建一个离屏 GLFW window/context，与 Java 的上下文共享 GL 对象。
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
        /// ### English
        /// 1) Drain control commands from embedder threads.
        ///
        /// ### 中文
        /// 1) Drain 来自宿主线程的控制命令。
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

        /// ### English
        /// 2) Drain per-view pending work (coalesced).
        ///
        /// ### 中文
        /// 2) drain 每 view 的 pending work（合并处理）。
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

        /// ### English
        /// 3) Let Servo do its internal work (timers/layout/script/painting scheduling).
        ///
        /// ### 中文
        /// 3) 让 Servo 执行内部任务（计时器/布局/脚本/绘制调度）。
        servo.spin_event_loop();

        /// ### English
        /// 4) If anything woke us while spinning, continue without parking.
        ///
        /// ### 中文
        /// 4) 如果 spin 期间发生唤醒，则不 park 直接继续循环。
        if wake_pending.swap(false, Ordering::Relaxed) {
            continue;
        }

        /// ### English
        /// 5) Park until an embedder command or waker unpark arrives.
        ///
        /// ### 中文
        /// 5) park 等待宿主命令或 waker 的 unpark。
        thread::park();
    }
}
