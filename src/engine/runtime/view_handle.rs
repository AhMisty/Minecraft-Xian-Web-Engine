use std::sync::Arc;
use std::thread;

use crossbeam_channel as channel;
use dpi::PhysicalSize;
use url::Url;

use crate::engine::frame::{AcquiredFrame, SharedFrameState, TRIPLE_BUFFER_COUNT};
use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};
use crate::engine::input_types::XianWebEngineInputEvent;

use super::command::Command;
use super::pending::PendingIdQueue;

pub(super) struct WebEngineViewHandleInit {
    pub id: u32,
    pub shared: Arc<SharedFrameState>,
    pub mouse_move: Arc<CoalescedMouseMove>,
    pub resize: Arc<CoalescedResize>,
    pub input_queue: Arc<InputEventQueue>,
    pub mouse_move_pending: Arc<PendingIdQueue>,
    pub input_pending: Arc<PendingIdQueue>,
    pub resize_pending: Arc<PendingIdQueue>,
    pub command_tx: channel::Sender<Command>,
    pub thread_handle: thread::Thread,
    pub unsafe_no_consumer_fence: bool,
}

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
    /// Coalesced resize state (only keeps the latest size).
    ///
    /// ### 中文
    /// resize 合并状态（只保留最新尺寸）。
    resize: Arc<CoalescedResize>,
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
    /// Shared pending-ID queue for coalesced resize notifications.
    ///
    /// ### 中文
    /// 用于合并 resize 通知的共享 pending-ID 队列。
    resize_pending: Arc<PendingIdQueue>,
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
}

impl WebEngineViewHandle {
    pub(super) fn new(init: WebEngineViewHandleInit) -> Self {
        let WebEngineViewHandleInit {
            id,
            shared,
            mouse_move,
            resize,
            input_queue,
            mouse_move_pending,
            input_pending,
            resize_pending,
            command_tx,
            thread_handle,
            unsafe_no_consumer_fence,
        } = init;
        Self {
            id,
            shared,
            mouse_move,
            resize,
            input_queue,
            mouse_move_pending,
            input_pending,
            resize_pending,
            command_tx,
            thread_handle,
            unsafe_no_consumer_fence,
        }
    }

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
    /// Coalesces resize requests (keeps only the latest) and schedules a drain on Servo thread.
    /// Returns `true` iff this call transitions from "not pending" to "pending" (caller should wake).
    ///
    /// ### 中文
    /// 合并 resize 请求（只保留最新一次），并通知 Servo 线程进行批量处理。
    /// 当且仅当本次调用把 pending 从 0→1 时返回 `true`（调用方应 wake/unpark Servo 线程）。
    pub fn queue_resize(&self, size: PhysicalSize<u32>) -> bool {
        let width = size.width.max(1);
        let height = size.height.max(1);
        if !self.resize.set(width, height) {
            return false;
        }

        let _ = self.resize_pending.push(self.id);
        true
    }

    /// ### English
    /// Enqueues a batch of non-mouse-move input events into the per-view queue.
    /// Returns the number of accepted events (may be less than `events.len()` if the queue is full).
    ///
    /// ### 中文
    /// 批量将非 mouse-move 的输入事件入队到每个 view 的队列中。
    /// 返回实际接收的事件数（若队列已满，可能小于 `events.len()`）。
    pub fn try_enqueue_input_events(&self, events: &[XianWebEngineInputEvent]) -> usize {
        self.input_queue.try_push_slice(events)
    }

    /// ### English
    /// Notifies the Servo thread that there are pending input events to drain.
    /// This is coalesced and uses a lock-free pending-ID queue to avoid channel traffic.
    ///
    /// Returns `true` iff this call transitions the pending flag from 0→1 (caller should wake/unpark).
    ///
    /// ### 中文
    /// 通知 Servo 线程存在待处理的输入事件。
    /// 该通知会被合并，并通过无锁 pending-ID 队列避免频繁 channel 通信。
    /// 当且仅当本次调用把 pending 从 0→1 时返回 `true`（调用方应 wake/unpark Servo 线程）。
    pub fn notify_input_pending(&self) -> bool {
        if !self.input_queue.mark_pending() {
            return false;
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
    /// Tries to acquire the latest READY frame (triple-buffer) for the consumer thread (Java).
    ///
    /// ### 中文
    /// 尝试为消费线程（Java）获取最新的 READY 帧（三缓冲）。
    pub fn acquire_frame(&self) -> Option<AcquiredFrame> {
        self.shared.try_acquire_front()
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
