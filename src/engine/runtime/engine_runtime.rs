use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::thread;
use std::time::Duration;

use crossbeam_channel as channel;
use dpi::PhysicalSize;

use crate::engine::flags;
use crate::engine::frame::SharedFrameState;
use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};
use crate::engine::refresh::RefreshScheduler;
use crate::engine::vsync::VsyncCallbackQueue;

use super::command::Command;
use super::pending::PendingIdQueue;
use super::servo_thread;
use super::view_handle::{WebEngineViewHandle, WebEngineViewHandleInit};

/// ### English
/// Engine runtime that owns the dedicated Servo thread.
///
/// ### 中文
/// 引擎运行时，持有独立的 Servo 线程。
pub struct EngineRuntime {
    /// ### English
    /// Default view size when the caller passes (0,0).
    ///
    /// ### 中文
    /// 当调用方传入 (0,0) 时使用的默认 view 尺寸。
    default_size: PhysicalSize<u32>,
    /// ### English
    /// Command sender into the dedicated Servo thread.
    ///
    /// ### 中文
    /// 向独立 Servo 线程发送命令的发送端。
    command_tx: channel::Sender<Command>,
    /// ### English
    /// Join handle for the Servo thread (kept for shutdown/join).
    ///
    /// ### 中文
    /// Servo 线程的 JoinHandle（用于 shutdown/join）。
    thread: Option<thread::JoinHandle<()>>,
    /// ### English
    /// Servo thread handle used for `unpark`.
    ///
    /// ### 中文
    /// 用于 `unpark` 的 Servo 线程句柄。
    thread_handle: thread::Thread,
    /// ### English
    /// Monotonic view ID generator.
    ///
    /// ### 中文
    /// 单调递增的 view ID 生成器。
    next_view_id: AtomicU32,
    /// ### English
    /// Shared vsync callback queue used to drive Servo refresh.
    ///
    /// ### 中文
    /// 用于驱动 Servo refresh 的共享 vsync 回调队列。
    vsync_queue: Arc<VsyncCallbackQueue>,
    /// ### English
    /// Pending view IDs with coalesced mouse-move updates.
    ///
    /// ### 中文
    /// 存在合并 mouse-move 更新的待处理 view ID。
    mouse_move_pending: Arc<PendingIdQueue>,
    /// ### English
    /// Pending view IDs with batched input events to drain.
    ///
    /// ### 中文
    /// 存在待 drain 的批量输入事件的待处理 view ID。
    input_pending: Arc<PendingIdQueue>,
    /// ### English
    /// Pending view IDs with coalesced resize updates.
    ///
    /// ### 中文
    /// 存在合并 resize 更新的待处理 view ID。
    resize_pending: Arc<PendingIdQueue>,
    /// ### English
    /// Shared scheduler for fixed-interval refresh drivers (tied to engine lifetime).
    ///
    /// ### 中文
    /// 固定间隔 refresh driver 共享的调度器（随引擎生命周期释放）。
    _refresh_scheduler: Arc<RefreshScheduler>,
}

impl EngineRuntime {
    /// ### English
    /// Creates a runtime bound to a Java-created GLFW OpenGL context (the shared context is created
    /// inside Rust on the Servo thread).
    ///
    /// ### 中文
    /// 基于 Java 创建的 GLFW OpenGL 上下文创建运行时（Rust 在 Servo 线程内创建共享上下文）。
    pub fn new(
        glfw_shared_window: *mut c_void,
        default_size: PhysicalSize<u32>,
        resources_dir: Option<PathBuf>,
        config_dir: Option<PathBuf>,
    ) -> Result<Self, String> {
        let glfw_shared_window_handle = glfw_shared_window as usize;
        let vsync_queue = Arc::new(VsyncCallbackQueue::with_capacity(4096));
        let vsync_queue_for_thread = vsync_queue.clone();
        let mouse_move_pending = Arc::new(PendingIdQueue::with_capacity(16 * 1024));
        let mouse_move_pending_for_thread = mouse_move_pending.clone();
        let input_pending = Arc::new(PendingIdQueue::with_capacity(16 * 1024));
        let input_pending_for_thread = input_pending.clone();
        let resize_pending = Arc::new(PendingIdQueue::with_capacity(16 * 1024));
        let resize_pending_for_thread = resize_pending.clone();
        let refresh_scheduler = RefreshScheduler::new();
        let refresh_scheduler_for_thread = refresh_scheduler.clone();
        let (command_tx, command_rx) = channel::unbounded::<Command>();
        let (init_tx, init_rx) = channel::bounded::<Result<(), String>>(1);

        let thread = thread::spawn(move || {
            servo_thread::run_servo_thread(
                glfw_shared_window_handle,
                resources_dir,
                config_dir,
                vsync_queue_for_thread,
                mouse_move_pending_for_thread,
                input_pending_for_thread,
                resize_pending_for_thread,
                refresh_scheduler_for_thread,
                command_rx,
                init_tx,
            );
        });

        let thread_handle = thread.thread().clone();

        match init_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(Self {
                default_size,
                command_tx,
                thread: Some(thread),
                thread_handle,
                next_view_id: AtomicU32::new(1),
                vsync_queue,
                mouse_move_pending,
                input_pending,
                resize_pending,
                _refresh_scheduler: refresh_scheduler,
            }),
            Ok(Err(err)) => {
                thread_handle.unpark();
                let _ = thread.join();
                Err(err)
            }
            Err(_) => {
                let _ = command_tx.send(Command::Shutdown);
                thread_handle.unpark();
                let _ = thread.join();
                Err("Timed out initializing Servo thread".to_string())
            }
        }
    }

    /// ### English
    /// Creates a view with a per-view target FPS (`0` = external vsync driven).
    ///
    /// ### 中文
    /// 创建 view，可指定每个 view 的目标 FPS（`0` = 外部 vsync 驱动）。
    pub fn create_view(
        &self,
        initial_size: PhysicalSize<u32>,
        target_fps: u32,
        view_flags: u32,
    ) -> Result<WebEngineViewHandle, String> {
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

        let id = self.next_view_id.fetch_add(1, Ordering::Relaxed);
        let shared = Arc::new(SharedFrameState::new(initial_size));
        let mouse_move = Arc::new(CoalescedMouseMove::default());
        let resize = Arc::new(CoalescedResize::default());
        let input_queue = Arc::new(InputEventQueue::new(input_single_producer));
        let (response_tx, response_rx) = channel::bounded::<Result<(), String>>(1);

        self.command_tx
            .send(Command::CreateView {
                id,
                initial_size,
                shared: shared.clone(),
                mouse_move: mouse_move.clone(),
                resize: resize.clone(),
                input_queue: input_queue.clone(),
                target_fps,
                unsafe_no_consumer_fence,
                unsafe_no_producer_fence,
                response: response_tx,
            })
            .map_err(|_| "Servo thread is not running".to_string())?;
        self.thread_handle.unpark();

        match response_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(WebEngineViewHandle::new(WebEngineViewHandleInit {
                id,
                shared,
                mouse_move,
                resize,
                input_queue,
                mouse_move_pending: self.mouse_move_pending.clone(),
                input_pending: self.input_pending.clone(),
                resize_pending: self.resize_pending.clone(),
                command_tx: self.command_tx.clone(),
                thread_handle: self.thread_handle.clone(),
                unsafe_no_consumer_fence,
            })),
            Ok(Err(err)) => Err(err),
            Err(_) => Err("Timed out creating view".to_string()),
        }
    }

    /// ### English
    /// Drains pending vsync callbacks (used by the Java side to drive Servo refresh).
    ///
    /// ### 中文
    /// 处理待执行的 vsync 回调（Java 侧用于驱动 Servo refresh）。
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
            let _ = self.command_tx.send(Command::Shutdown);
            self.thread_handle.unpark();
            let _ = thread.join();
        }
    }
}

impl Drop for EngineRuntime {
    /// ### English
    /// Ensures background thread is joined on drop.
    ///
    /// ### 中文
    /// Drop 时确保后台线程被 join。
    fn drop(&mut self) {
        self.shutdown();
    }
}
