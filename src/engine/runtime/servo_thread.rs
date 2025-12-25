//! ### English
//! Dedicated Servo thread: owns the shared GL context and drives Servo's event loop.
//!
//! ### 中文
//! 独立 Servo 线程：持有共享 GL 上下文并驱动 Servo 事件循环。

use std::ffi::c_void;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

use crossbeam_channel as channel;
use dpi::PhysicalSize;
use url::Url;

use crate::engine::flags;
use crate::engine::input::{CoalescedMouseMove, InputEventQueue};
use crate::engine::rendering::{GlfwSharedContext, GlfwTripleBufferRenderingContext};
use crate::engine::resources;
use crate::engine::vsync::VsyncCallbackQueue;

use super::command::Command;
use super::input_dispatch::dispatch_queued_input_event;
use super::pending::PendingIdQueue;
use super::u32_hash::U32HashMap;

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
    mouse_move_pending: Arc<PendingIdQueue>,
    input_pending: Arc<PendingIdQueue>,
    command_rx: channel::Receiver<Command>,
    init_tx: channel::Sender<Result<(), String>>,
) {
    /*
    ### English
    Install rustls provider once per process (Servo uses it internally).

    ### 中文
    全进程只安装一次 rustls provider（Servo 内部会使用）。
    */
    static RUSTLS_PROVIDER: OnceLock<()> = OnceLock::new();
    RUSTLS_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });

    /*
    ### English
    Optional resource/config directories.

    ### 中文
    可选的资源/配置目录。
    */
    if let Some(resources_dir) = resources_dir {
        resources::set_resources_dir(resources_dir);
    }
    if let Some(ref config_dir) = config_dir {
        let _ = std::fs::create_dir_all(config_dir);
    }

    /*
    ### English
    Coalesced wake flag to avoid unpark storms.

    ### 中文
    合并唤醒标记，避免频繁 unpark 的风暴。
    */
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

    /*
    ### English
    Servo opts/preferences chosen for low overhead in an embedder.

    ### 中文
    为宿主嵌入场景选择的 Servo 参数/偏好设置（低开销优先）。
    */
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

    /*
    ### English
    Create a shared offscreen GLFW window/context that shares objects with Java's context.

    ### 中文
    创建一个离屏 GLFW window/context，与 Java 的上下文共享 GL 对象。
    */
    let glfw_shared_window_ptr = glfw_shared_window_handle as *mut c_void;
    let shared_ctx = match GlfwSharedContext::new(glfw_shared_window_ptr) {
        Ok(ctx) => ctx,
        Err(err) => {
            let _ = init_tx.send(Err(err));
            return;
        }
    };

    let _ = init_tx.send(Ok(()));

    /// ### English
    /// Per-view bookkeeping stored only on the Servo thread.
    ///
    /// ### 中文
    /// 仅 Servo 线程持有的每个 view 状态。
    struct ViewEntry {
        /// ### English
        /// Servo WebView instance (lives on Servo thread only).
        ///
        /// ### 中文
        /// Servo WebView 实例（仅 Servo 线程持有）。
        servo_webview: servo::WebView,
        /// ### English
        /// Rendering context + triple-buffer resources owned by this view.
        ///
        /// ### 中文
        /// 该 view 持有的渲染上下文 + 三缓冲资源。
        rendering_context: Rc<GlfwTripleBufferRenderingContext>,
        /// ### English
        /// Shared coalesced mouse-move state for this view.
        ///
        /// ### 中文
        /// 该 view 的鼠标移动合并状态（共享）。
        mouse_move: Arc<CoalescedMouseMove>,
        /// ### English
        /// Per-view bounded input queue (mouse move is handled separately).
        ///
        /// ### 中文
        /// 每 view 的有界输入队列（鼠标移动单独处理）。
        input_queue: Arc<InputEventQueue>,
        /// ### English
        /// Latest pending resize request (coalesced).
        ///
        /// ### 中文
        /// 最新的待处理 resize 请求（合并后）。
        pending_resize: Option<PhysicalSize<u32>>,
        /// ### English
        /// Latest pending URL load request (coalesced).
        ///
        /// ### 中文
        /// 最新的待处理 URL load 请求（合并后）。
        pending_load_url: Option<Url>,
        /// ### English
        /// Whether this entry is already queued in `pending_update_ids`.
        ///
        /// ### 中文
        /// 该条目是否已进入 `pending_update_ids` 队列。
        update_queued: bool,
    }

    /// ### English
    /// Servo WebView delegate runs on the Servo thread and drives `paint`/`present`.
    ///
    /// ### 中文
    /// Servo WebView delegate 在 Servo 线程执行，用于驱动 `paint`/`present`。
    struct Delegate {
        /// ### English
        /// Rendering context used for `paint/present` on `notify_new_frame_ready`.
        ///
        /// ### 中文
        /// 在 `notify_new_frame_ready` 中用于 `paint/present` 的渲染上下文。
        rendering_context: Rc<GlfwTripleBufferRenderingContext>,
    }

    impl servo::WebViewDelegate for Delegate {
        fn notify_new_frame_ready(&self, servo_webview: servo::WebView) {
            if !self.rendering_context.is_active() {
                return;
            }
            if !self.rendering_context.preflight_reserve_next_back_slot() {
                return;
            }

            servo_webview.paint();
            servo::RenderingContext::present(&*self.rendering_context);
        }
    }

    let mut views: U32HashMap<ViewEntry> =
        U32HashMap::with_capacity_and_hasher(64, Default::default());
    let mut pending_update_ids: Vec<u32> = Vec::with_capacity(64);

    loop {
        /*
        ### English
        1) Drain control commands from embedder threads.

        ### 中文
        1) Drain 来自宿主线程的控制命令。
        */
        while let Ok(command) = command_rx.try_recv() {
            match command {
                Command::CreateView {
                    id,
                    initial_size,
                    shared,
                    mouse_move,
                    input_queue,
                    target_fps,
                    flags,
                    response,
                } => {
                    let unsafe_no_consumer_fence = (flags
                        & flags::XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE)
                        != 0;
                    let rendering_context = match GlfwTripleBufferRenderingContext::new(
                        shared_ctx.clone(),
                        initial_size,
                        shared,
                        vsync_queue.clone(),
                        target_fps,
                        unsafe_no_consumer_fence,
                    ) {
                        Ok(ctx) => Rc::new(ctx),
                        Err(err) => {
                            let _ = response.send(Err(err));
                            continue;
                        }
                    };

                    let delegate = Rc::new(Delegate {
                        rendering_context: rendering_context.clone(),
                    });

                    let servo_webview =
                        servo::WebViewBuilder::new(&servo, rendering_context.clone())
                            .delegate(delegate)
                            .build();
                    servo_webview.show();

                    views.insert(
                        id,
                        ViewEntry {
                            servo_webview,
                            rendering_context,
                            mouse_move,
                            input_queue,
                            pending_resize: None,
                            pending_load_url: None,
                            update_queued: false,
                        },
                    );

                    let _ = response.send(Ok(()));
                }
                Command::LoadUrl { id, url } => {
                    let Some(entry) = views.get_mut(&id) else {
                        continue;
                    };
                    entry.pending_load_url = Some(url);
                    if !entry.update_queued {
                        entry.update_queued = true;
                        pending_update_ids.push(id);
                    }
                }
                Command::Resize { id, size } => {
                    let Some(entry) = views.get_mut(&id) else {
                        continue;
                    };
                    entry.pending_resize = Some(size);
                    if !entry.update_queued {
                        entry.update_queued = true;
                        pending_update_ids.push(id);
                    }
                }
                Command::SetActive { id, active } => {
                    let Some(entry) = views.get(&id) else {
                        continue;
                    };

                    if active {
                        entry.servo_webview.set_throttled(false);
                        entry.servo_webview.show();
                    } else {
                        entry.servo_webview.set_throttled(true);
                        entry.servo_webview.hide();
                    }
                }
                Command::DestroyView { id } => {
                    if let Some(entry) = views.remove(&id) {
                        entry.rendering_context.destroy_gl_resources();
                    }
                }
                Command::Shutdown => {
                    for (_, entry) in views.drain() {
                        entry.rendering_context.destroy_gl_resources();
                    }
                    return;
                }
            }
        }

        /*
        ### English
        2) Apply coalesced URL/resize updates.

        ### 中文
        2) 应用合并后的 URL/resize 更新。
        */
        for id in pending_update_ids.drain(..) {
            let Some(entry) = views.get_mut(&id) else {
                continue;
            };
            entry.update_queued = false;

            if let Some(size) = entry.pending_resize.take() {
                entry.servo_webview.resize(size);
            }
            if let Some(url) = entry.pending_load_url.take() {
                entry.servo_webview.load(url);
            }
        }

        /*
        ### English
        3) Dispatch coalesced mouse moves (only latest per view).

        ### 中文
        3) 派发合并后的鼠标移动（每个 view 只取最新一次）。
        */
        while let Some(id) = mouse_move_pending.pop() {
            let Some(entry) = views.get(&id) else {
                continue;
            };
            if !entry.rendering_context.is_active() {
                continue;
            }

            let Some((x, y)) = entry.mouse_move.take() else {
                continue;
            };

            let point = servo::WebViewPoint::from(servo::DevicePoint::new(x, y));
            entry
                .servo_webview
                .notify_input_event(servo::InputEvent::MouseMove(servo::MouseMoveEvent::new(
                    point,
                )));
        }

        if mouse_move_pending.take_overflowed() {
            for entry in views.values() {
                if !entry.rendering_context.is_active() {
                    continue;
                }

                let Some((x, y)) = entry.mouse_move.take() else {
                    continue;
                };

                let point = servo::WebViewPoint::from(servo::DevicePoint::new(x, y));
                entry
                    .servo_webview
                    .notify_input_event(servo::InputEvent::MouseMove(servo::MouseMoveEvent::new(
                        point,
                    )));
            }
        }

        /*
        ### English
        4) Drain batched input queues and dispatch into Servo.

        ### 中文
        4) Drain 批量输入队列并派发给 Servo。
        */
        while let Some(id) = input_pending.pop() {
            let Some(entry) = views.get(&id) else {
                continue;
            };

            loop {
                let active = entry.rendering_context.is_active();
                while let Some(raw) = entry.input_queue.pop() {
                    if active {
                        dispatch_queued_input_event(&entry.servo_webview, raw);
                    }
                }

                entry.input_queue.clear_pending();
                let Some(raw) = entry.input_queue.pop() else {
                    break;
                };
                entry.input_queue.mark_pending();

                if entry.rendering_context.is_active() {
                    dispatch_queued_input_event(&entry.servo_webview, raw);
                }
            }
        }

        if input_pending.take_overflowed() {
            for entry in views.values() {
                if !entry.input_queue.is_pending() {
                    continue;
                }

                loop {
                    let active = entry.rendering_context.is_active();
                    while let Some(raw) = entry.input_queue.pop() {
                        if active {
                            dispatch_queued_input_event(&entry.servo_webview, raw);
                        }
                    }

                    entry.input_queue.clear_pending();
                    let Some(raw) = entry.input_queue.pop() else {
                        break;
                    };
                    entry.input_queue.mark_pending();

                    if entry.rendering_context.is_active() {
                        dispatch_queued_input_event(&entry.servo_webview, raw);
                    }
                }
            }
        }

        /*
        ### English
        5) Let Servo do its internal work (timers/layout/script/painting scheduling).

        ### 中文
        5) 让 Servo 执行内部任务（计时器/布局/脚本/绘制调度）。
        */
        servo.spin_event_loop();

        /*
        ### English
        6) If anything woke us while spinning, continue without parking.

        ### 中文
        6) 如果 spin 期间发生唤醒，则不 park 直接继续循环。
        */
        if wake_pending.swap(false, Ordering::Relaxed) {
            continue;
        }

        /*
        ### English
        7) Park until an embedder command or waker unpark arrives.

        ### 中文
        7) park 等待宿主命令或 waker 的 unpark。
        */
        thread::park();
    }
}
