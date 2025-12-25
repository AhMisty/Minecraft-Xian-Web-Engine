//! ### English
//! Servo runtime orchestration (public API).
//!
//! ### 中文
//! Servo 运行时编排（对外公开 API）。

mod command;
mod input_dispatch;
mod keyboard;
mod pending;
mod servo_thread;
mod u32_hash;

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
use url::Url;

use crate::engine::flags;
use crate::engine::frame::{AcquiredFrame, SharedFrameState, TRIPLE_BUFFER_COUNT};
use crate::engine::input::{CoalescedMouseMove, InputEventQueue};
use crate::engine::input_types::XianWebEngineInputEvent;
use crate::engine::vsync::VsyncCallbackQueue;

use self::command::Command;
use self::pending::PendingIdQueue;

/// ### English
/// Opaque handle for a single view (thread-safe to use from the embedder thread).
/// It only sends commands / enqueues input to the dedicated Servo thread.
///
/// ### 中文
/// 单个 view 的不透明句柄（可在宿主线程安全调用）。
/// 仅负责发送命令/入队输入到独立的 Servo 线程。
pub struct WebEngineViewHandle {
    /// ### English
    /// Unique ID for this view.
    ///
    /// ### 中文
    /// 该 view 的唯一 ID。
    id: u32,
    /// ### English
    /// Lock-free shared frame state (triple buffer) for Java-side consumption.
    ///
    /// ### 中文
    /// 供 Java 侧消费的无锁共享帧状态（三缓冲）。
    shared: Arc<SharedFrameState>,
    /// ### English
    /// Coalesced mouse-move state (only keeps the latest move).
    ///
    /// ### 中文
    /// 鼠标移动合并状态（只保留最新一次）。
    mouse_move: Arc<CoalescedMouseMove>,
    /// ### English
    /// Bounded lock-free input queue (mouse-move is handled separately).
    ///
    /// ### 中文
    /// 有界无锁输入队列（鼠标移动单独合并处理）。
    input_queue: Arc<InputEventQueue>,
    /// ### English
    /// Shared pending-ID queue for coalesced mouse-move notifications.
    ///
    /// ### 中文
    /// 用于合并 mouse-move 通知的共享 pending-ID 队列。
    mouse_move_pending: Arc<PendingIdQueue>,
    /// ### English
    /// Shared pending-ID queue for batched input notifications.
    ///
    /// ### 中文
    /// 用于批量输入通知的共享 pending-ID 队列。
    input_pending: Arc<PendingIdQueue>,
    /// ### English
    /// Command sender into the dedicated Servo thread.
    ///
    /// ### 中文
    /// 向独立 Servo 线程发送命令的发送端。
    command_tx: channel::Sender<Command>,
    /// ### English
    /// Servo thread handle used for `unpark`.
    ///
    /// ### 中文
    /// 用于 `unpark` 的 Servo 线程句柄。
    thread_handle: thread::Thread,
    /// ### English
    /// If true, ignore Java-side consumer fences (unsafe but lower overhead).
    ///
    /// ### 中文
    /// 为 true 时忽略 Java 侧 consumer fence（不安全但开销更低）。
    unsafe_no_consumer_fence: bool,
    /// ### English
    /// If true, the embedder guarantees a single input-producer thread.
    ///
    /// ### 中文
    /// 为 true 时宿主保证输入生产者只有一个线程。
    input_single_producer: bool,
}

impl WebEngineViewHandle {
    /// ### English
    /// Returns whether this view is currently active (not throttled/hidden).
    ///
    /// ### 中文
    /// 返回该 view 是否处于 active（非节流/非隐藏）状态。
    pub fn is_active(&self) -> bool {
        self.shared.is_active()
    }

    /// ### English
    /// Coalesces mouse-move events (keeps only the latest) and schedules a drain on Servo thread.
    ///
    /// ### 中文
    /// 合并鼠标移动事件（只保留最新一次），并通知 Servo 线程进行批量处理。
    pub fn queue_mouse_move(&self, x: f32, y: f32) -> bool {
        if !self.mouse_move.set(x, y) {
            return false;
        }

        let _ = self.mouse_move_pending.push(self.id);
        true
    }

    /// ### English
    /// Enqueues a non-mouse-move input event into the per-view bounded queue.
    /// Uses a single-producer fast path when the embedder guarantees one input producer thread.
    ///
    /// ### 中文
    /// 将非 mouse-move 的输入事件入队到每个 view 的有界队列。
    /// 若宿主保证只有一个输入生产者线程，则使用单生产者快路径。
    pub fn try_enqueue_input_event(&self, event: XianWebEngineInputEvent) -> bool {
        if self.input_single_producer {
            self.input_queue.try_push_single_producer(event)
        } else {
            self.input_queue.try_push(event)
        }
    }

    /// ### English
    /// Notifies the Servo thread that there are pending input events to drain.
    /// This is coalesced and uses a lock-free pending-ID queue to avoid channel traffic.
    ///
    /// ### 中文
    /// 通知 Servo 线程存在待处理的输入事件。
    /// 该通知会被合并，并通过无锁 pending-ID 队列避免频繁 channel 通信。
    pub fn notify_input_pending(&self) -> bool {
        if !self.input_queue.mark_pending() {
            return true;
        }

        let _ = self.input_pending.push(self.id);
        true
    }

    /// ### English
    /// Wakes the Servo thread (`unpark`).
    ///
    /// ### 中文
    /// 唤醒 Servo 线程（`unpark`）。
    pub fn wake(&self) {
        self.thread_handle.unpark();
    }

    /// ### English
    /// Requests navigation to a URL on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程请求加载指定 URL。
    pub fn load_url(&self, url: Url) {
        let _ = self.command_tx.send(Command::LoadUrl { id: self.id, url });
        self.thread_handle.unpark();
    }

    /// ### English
    /// Requests a resize for this view on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程请求调整该 view 尺寸。
    pub fn resize(&self, size: PhysicalSize<u32>) {
        let _ = self.command_tx.send(Command::Resize { id: self.id, size });
        self.thread_handle.unpark();
    }

    /// ### English
    /// Tries to acquire the latest READY frame (triple-buffer) for the consumer thread (Java).
    ///
    /// ### 中文
    /// 尝试为消费线程（Java）获取最新的 READY 帧（三缓冲）。
    pub fn acquire_frame(&self) -> Option<AcquiredFrame> {
        self.shared.try_acquire_front()
    }

    /// ### English
    /// Tries to acquire the latest READY frame only if it's newer than `last_seq`.
    ///
    /// ### 中文
    /// 仅当存在比 `last_seq` 更新的帧时，才尝试获取最新 READY 帧。
    pub fn acquire_frame_if_newer(&self, last_seq: u64) -> Option<AcquiredFrame> {
        self.shared.try_acquire_front_if_newer(last_seq)
    }

    /// ### English
    /// Marks this view active/inactive and applies hide/throttle on Servo thread.
    ///
    /// ### 中文
    /// 设置该 view 是否 active，并在 Servo 线程执行 hide/throttle。
    pub fn set_active(&self, active: bool) {
        if self.shared.is_active() == active {
            return;
        }

        self.shared.set_active(active);
        let _ = self.command_tx.send(Command::SetActive {
            id: self.id,
            active,
        });
        self.thread_handle.unpark();
    }

    /// ### English
    /// Releases a previously acquired frame slot; optionally attaches a consumer fence.
    /// Safe mode uses consumer fences; unsafe mode ignores the fence to avoid cross-thread sync.
    ///
    /// ### 中文
    /// 释放之前获取的帧槽位；可选传入 consumer fence。
    /// 安全模式使用 consumer fence；不安全模式忽略 fence 以避免跨线程同步。
    pub fn release_slot_with_fence(&self, slot: u32, consumer_fence: u64) {
        let slot = slot as usize;
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        if self.unsafe_no_consumer_fence {
            self.shared.release_slot(slot, 0);
        } else {
            self.shared.release_slot(slot, consumer_fence);
        }
    }
}

impl Drop for WebEngineViewHandle {
    /// ### English
    /// On drop, schedule view destruction on the Servo thread (GL resource teardown must
    /// happen in the context-owning thread).
    ///
    /// ### 中文
    /// Drop 时在 Servo 线程安排销毁 view（GL 资源释放必须在持有上下文的线程执行）。
    fn drop(&mut self) {
        let _ = self.command_tx.send(Command::DestroyView { id: self.id });
        self.thread_handle.unpark();
    }
}

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
}

impl EngineRuntime {
    /// ### English
    /// Creates a runtime bound to a Java-created GLFW OpenGL context (the shared context is created
    /// inside Rust on the Servo thread).
    ///
    /// ### 中文
    /// 基于 Java 创建的 GLFW OpenGL 上下文创建运行时（Rust 在 Servo 线程内创建共享上下文）。
    pub fn new_glfw_shared_context(
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
    pub fn create_view_with_target_fps(
        &self,
        initial_size: PhysicalSize<u32>,
        target_fps: u32,
        flags: u32,
    ) -> Result<WebEngineViewHandle, String> {
        let unsafe_no_consumer_fence =
            (flags & flags::XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE) != 0;
        let input_single_producer =
            (flags & flags::XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_INPUT_SINGLE_PRODUCER) != 0;

        let initial_size = if initial_size.width == 0 || initial_size.height == 0 {
            self.default_size
        } else {
            initial_size
        };
        let initial_size = PhysicalSize::new(initial_size.width.max(1), initial_size.height.max(1));

        let id = self.next_view_id.fetch_add(1, Ordering::Relaxed);
        let shared = Arc::new(SharedFrameState::new(initial_size));
        let mouse_move = Arc::new(CoalescedMouseMove::default());
        let input_queue = Arc::new(InputEventQueue::new(input_single_producer));
        let (response_tx, response_rx) = channel::bounded::<Result<(), String>>(1);

        self.command_tx
            .send(Command::CreateView {
                id,
                initial_size,
                shared: shared.clone(),
                mouse_move: mouse_move.clone(),
                input_queue: input_queue.clone(),
                target_fps,
                flags,
                response: response_tx,
            })
            .map_err(|_| "Servo thread is not running".to_string())?;
        self.thread_handle.unpark();

        match response_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(WebEngineViewHandle {
                id,
                shared,
                mouse_move,
                input_queue,
                mouse_move_pending: self.mouse_move_pending.clone(),
                input_pending: self.input_pending.clone(),
                command_tx: self.command_tx.clone(),
                thread_handle: self.thread_handle.clone(),
                unsafe_no_consumer_fence,
                input_single_producer,
            }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err("Timed out creating view".to_string()),
        }
    }

    /// ### English
    /// Drains pending vsync callbacks (used by the Java side to drive Servo refresh).
    ///
    /// ### 中文
    /// 处理待执行的 vsync 回调（Java 侧用于驱动 Servo refresh）。
    pub fn vsync_tick(&self) {
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
