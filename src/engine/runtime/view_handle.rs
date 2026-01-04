//! ### English
//! Thread-safe view handle used by the embedder to interact with the Servo thread.
//!
//! ### 中文
//! 宿主用于与 Servo 线程交互的线程安全 view 句柄。

use std::sync::Arc;
use std::thread;

use dpi::PhysicalSize;

use crate::engine::frame::{AcquiredFrame, SharedFrameState, TRIPLE_BUFFER_COUNT};
use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};
use crate::engine::input_types::XianWebEngineInputEvent;

use super::coalesced::{
    CoalescedLoadUrl, PENDING_ACTIVE, PENDING_INPUT, PENDING_LOAD_URL, PENDING_MOUSE_MOVE,
    PENDING_RESIZE, PendingWork,
};
use super::command::Command;
use super::pending::PendingIdQueue;
use super::queue::CommandQueue;

/// ### English
/// Internal initializer for `WebEngineViewHandle` (constructed by `EngineRuntime`).
///
/// ### 中文
/// `WebEngineViewHandle` 的内部初始化参数（由 `EngineRuntime` 构造）。
pub(super) struct WebEngineViewHandleInit {
    /// ### English
    /// View ID allocated on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程分配的 view ID。
    pub id: u32,
    /// ### English
    /// Monotonic token paired with `id` to detect stale destroy commands.
    ///
    /// ### 中文
    /// 与 `id` 配对的单调递增 token，用于识别陈旧的销毁命令。
    pub token: u64,
    /// ### English
    /// Shared triple-buffer frame state for this view.
    ///
    /// ### 中文
    /// 该 view 的三缓冲共享帧状态。
    pub shared: Arc<SharedFrameState>,
    /// ### English
    /// Coalesced mouse-move state (latest-wins).
    ///
    /// ### 中文
    /// 鼠标移动合并状态（latest-wins）。
    pub mouse_move: Arc<CoalescedMouseMove>,
    /// ### English
    /// Coalesced resize state (latest-wins).
    ///
    /// ### 中文
    /// resize 合并状态（latest-wins）。
    pub resize: Arc<CoalescedResize>,
    /// ### English
    /// Bounded input-event queue (mouse move is handled separately).
    ///
    /// ### 中文
    /// 有界输入事件队列（鼠标移动单独处理）。
    pub input_queue: Arc<InputEventQueue>,
    /// ### English
    /// Coalesced URL load request (latest-wins).
    ///
    /// ### 中文
    /// URL 加载合并请求（latest-wins）。
    pub load_url: Arc<CoalescedLoadUrl>,
    /// ### English
    /// Per-view pending-work bitmask.
    ///
    /// ### 中文
    /// 每 view 的 pending-work 位图。
    pub pending: Arc<PendingWork>,
    /// ### English
    /// Global pending view-id queue shared with the Servo thread.
    ///
    /// ### 中文
    /// 与 Servo 线程共享的全局 pending view-id 队列。
    pub pending_queue: Arc<PendingIdQueue>,
    /// ### English
    /// Global command queue into the Servo thread.
    ///
    /// ### 中文
    /// 发送到 Servo 线程的全局命令队列。
    pub command_queue: Arc<CommandQueue>,
    /// ### English
    /// Servo thread handle used to wake it (`unpark`).
    ///
    /// ### 中文
    /// Servo 线程句柄（用于 `unpark` 唤醒）。
    pub thread_handle: thread::Thread,
    /// ### English
    /// Whether the view runs without recording consumer fences (unsafe, for advanced embedders).
    ///
    /// ### 中文
    /// 是否不记录 consumer fence（不安全，仅供高级宿主使用）。
    pub unsafe_no_consumer_fence: bool,
}

/// ### English
/// Opaque handle for a single view (thread-safe to use from the embedder thread).
///
/// ### 中文
/// 单个 view 的不透明句柄（可在宿主线程安全调用）。
pub struct WebEngineViewHandle {
    /// ### English
    /// View ID allocated on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程分配的 view ID。
    id: u32,
    /// ### English
    /// Monotonic token paired with `id` to detect stale destroy commands.
    ///
    /// ### 中文
    /// 与 `id` 配对的单调递增 token，用于识别陈旧的销毁命令。
    token: u64,
    /// ### English
    /// Shared triple-buffer frame state for this view.
    ///
    /// ### 中文
    /// 该 view 的三缓冲共享帧状态。
    shared: Arc<SharedFrameState>,
    /// ### English
    /// Coalesced mouse-move state (latest-wins).
    ///
    /// ### 中文
    /// 鼠标移动合并状态（latest-wins）。
    mouse_move: Arc<CoalescedMouseMove>,
    /// ### English
    /// Coalesced resize state (latest-wins).
    ///
    /// ### 中文
    /// resize 合并状态（latest-wins）。
    resize: Arc<CoalescedResize>,
    /// ### English
    /// Bounded input-event queue (mouse move is handled separately).
    ///
    /// ### 中文
    /// 有界输入事件队列（鼠标移动单独处理）。
    input_queue: Arc<InputEventQueue>,
    /// ### English
    /// Coalesced URL load request (latest-wins).
    ///
    /// ### 中文
    /// URL 加载合并请求（latest-wins）。
    load_url: Arc<CoalescedLoadUrl>,
    /// ### English
    /// Per-view pending-work bitmask.
    ///
    /// ### 中文
    /// 每 view 的 pending-work 位图。
    pending: Arc<PendingWork>,
    /// ### English
    /// Global pending view-id queue shared with the Servo thread.
    ///
    /// ### 中文
    /// 与 Servo 线程共享的全局 pending view-id 队列。
    pending_queue: Arc<PendingIdQueue>,
    /// ### English
    /// Global command queue into the Servo thread.
    ///
    /// ### 中文
    /// 发送到 Servo 线程的全局命令队列。
    command_queue: Arc<CommandQueue>,
    /// ### English
    /// Servo thread handle used to wake it (`unpark`).
    ///
    /// ### 中文
    /// Servo 线程句柄（用于 `unpark` 唤醒）。
    thread_handle: thread::Thread,
    /// ### English
    /// Whether the view runs without recording consumer fences (unsafe, for advanced embedders).
    ///
    /// ### 中文
    /// 是否不记录 consumer fence（不安全，仅供高级宿主使用）。
    unsafe_no_consumer_fence: bool,
}

impl WebEngineViewHandle {
    /// ### English
    /// Creates a thread-safe view handle from the pre-built initialization bundle.
    ///
    /// #### Parameters
    /// - `init`: Pre-built initialization bundle from `EngineRuntime`.
    ///
    /// ### 中文
    /// 根据已构造的初始化参数创建线程安全 view 句柄。
    ///
    /// #### 参数
    /// - `init`：由 `EngineRuntime` 构造的初始化参数包。
    pub(super) fn new(init: WebEngineViewHandleInit) -> Self {
        let WebEngineViewHandleInit {
            id,
            token,
            shared,
            mouse_move,
            resize,
            input_queue,
            load_url,
            pending,
            pending_queue,
            command_queue,
            thread_handle,
            unsafe_no_consumer_fence,
        } = init;
        Self {
            id,
            token,
            shared,
            mouse_move,
            resize,
            input_queue,
            load_url,
            pending,
            pending_queue,
            command_queue,
            thread_handle,
            unsafe_no_consumer_fence,
        }
    }

    /// ### English
    /// Marks pending work bits and pushes this view ID if it transitions from idle to busy.
    ///
    /// Return value contract:
    /// - `true`: this call transitioned the view from idle to busy; the caller should wake the Servo
    ///   thread (e.g. call [`Self::wake`]) to ensure timely processing.
    /// - `false`: the view was already busy; waking again is redundant and can hurt performance.
    ///
    /// #### Parameters
    /// - `bits`: Work bits to mark for this view.
    ///
    /// ### 中文
    /// 标记待处理 work bit；若从 idle 变为 busy，则把该 view ID push 到 pending 队列。
    ///
    /// 返回值约定：
    /// - `true`：本次调用把 view 从 idle 切换为 busy；调用方应唤醒 Servo 线程（例如调用 [`Self::wake`]）以便及时处理。
    /// - `false`：该 view 已处于 busy；再次唤醒属于冗余操作，可能降低性能。
    ///
    /// #### 参数
    /// - `bits`：要标记的 work bit。
    #[inline]
    fn mark_pending(&self, bits: u8) -> bool {
        if !self.pending.mark(bits) {
            return false;
        }
        let _ = self.pending_queue.push(self.id);
        true
    }

    /// ### English
    /// Returns whether this view is active.
    ///
    /// ### 中文
    /// 返回该 view 是否 active。
    pub fn is_active(&self) -> bool {
        self.shared.is_active()
    }

    /// ### English
    /// Coalesces one mouse-move and marks it pending.
    ///
    /// Returns `true` iff the caller should wake the Servo thread (see [`Self::wake`]).
    ///
    /// #### Parameters
    /// - `x`: X position in device pixels (f32).
    /// - `y`: Y position in device pixels (f32).
    ///
    /// ### 中文
    /// 合并一次鼠标移动并标记为 pending。
    ///
    /// 仅当返回 `true` 时建议唤醒 Servo 线程（见 [`Self::wake`]）。
    ///
    /// #### 参数
    /// - `x`：设备像素坐标 X（f32）。
    /// - `y`：设备像素坐标 Y（f32）。
    #[must_use = "returns whether the caller should wake the Servo thread"]
    pub fn queue_mouse_move(&self, x: f32, y: f32) -> bool {
        self.mouse_move.set(x, y);
        self.mark_pending(PENDING_MOUSE_MOVE)
    }

    /// ### English
    /// Coalesces a resize request and marks it pending.
    ///
    /// The size is clamped to at least 1x1.
    ///
    /// #### Parameters
    /// - `size`: Requested size (will be clamped to at least 1x1).
    ///
    /// ### 中文
    /// 合并一次 resize 请求并标记为 pending。
    ///
    /// 尺寸会被 clamp 至至少 1x1。
    ///
    /// #### 参数
    /// - `size`：请求的尺寸（会 clamp 至至少 1x1）。
    ///
    /// 仅当返回 `true` 时建议唤醒 Servo 线程（见 [`Self::wake`]）。
    #[must_use = "returns whether the caller should wake the Servo thread"]
    pub fn queue_resize(&self, size: PhysicalSize<u32>) -> bool {
        let width = size.width.max(1);
        let height = size.height.max(1);
        self.resize.set(width, height);
        self.mark_pending(PENDING_RESIZE)
    }

    /// ### English
    /// Pushes a slice of input events into the bounded input queue.
    ///
    /// Returns the number of accepted events (may be less than the slice length if full).
    ///
    /// #### Parameters
    /// - `events`: Events to push.
    ///
    /// ### 中文
    /// 将一段输入事件 push 到有界输入队列。
    ///
    /// 返回实际接收的事件数量（队列满时可能小于切片长度）。
    ///
    /// #### 参数
    /// - `events`：要 push 的事件切片。
    pub fn push_input_events(&self, events: &[XianWebEngineInputEvent]) -> usize {
        self.input_queue.try_push_slice(events)
    }

    /// ### English
    /// Marks that non-mouse-move input is pending (coalesced flag) and schedules processing.
    ///
    /// Returns `true` iff this call is the first pending mark and the caller should wake the Servo
    /// thread (see [`Self::wake`]).
    ///
    /// ### 中文
    /// 标记“非鼠标移动输入”待处理（合并标记），并调度处理。
    ///
    /// 仅当返回 `true` 时表示本次为首次 pending 标记，且建议唤醒 Servo 线程（见 [`Self::wake`]）。
    #[must_use = "returns whether the caller should wake the Servo thread"]
    pub fn notify_input_pending(&self) -> bool {
        if !self.input_queue.mark_pending() {
            return false;
        }
        self.mark_pending(PENDING_INPUT)
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
    /// Requests navigation to a URL string on the Servo thread (coalesced per view; latest wins).
    ///
    /// Returns `true` iff the caller should wake the Servo thread (see [`Self::wake`]).
    ///
    /// #### Parameters
    /// - `url`: URL string to load (latest wins).
    ///
    /// ### 中文
    /// 请求在 Servo 线程加载一个 URL 字符串（每 view 合并；只保留最新一次）。
    ///
    /// 仅当返回 `true` 时建议唤醒 Servo 线程（见 [`Self::wake`]）。
    ///
    /// #### 参数
    /// - `url`：要加载的 URL 字符串（latest-wins）。
    #[must_use = "returns whether the caller should wake the Servo thread"]
    pub fn load_url(&self, url: &str) -> bool {
        self.load_url.set_str(url);
        self.mark_pending(PENDING_LOAD_URL)
    }

    /// ### English
    /// Tries to acquire the latest READY frame (consumer-side).
    ///
    /// ### 中文
    /// 尝试 acquire 最新 READY 帧（消费者侧）。
    pub fn acquire_frame(&self) -> Option<AcquiredFrame> {
        self.shared.try_acquire_front()
    }

    /// ### English
    /// Marks this view active/inactive and applies hide/throttle on Servo thread.
    ///
    /// Returns `true` iff this call changes the active state and the caller should wake the Servo
    /// thread (see [`Self::wake`]).
    ///
    /// #### Parameters
    /// - `active`: Whether the view should be active.
    ///
    /// ### 中文
    /// 设置该 view 的 active/inactive，并在 Servo 线程应用 hide/throttle。
    ///
    /// 仅当返回 `true` 时表示 active 状态发生变化，且建议唤醒 Servo 线程（见 [`Self::wake`]）。
    ///
    /// #### 参数
    /// - `active`：是否将该 view 设为 active。
    #[must_use = "returns whether the caller should wake the Servo thread"]
    pub fn set_active(&self, active: bool) -> bool {
        if self.shared.is_active() == active {
            return false;
        }

        self.shared.set_active(active);
        self.mark_pending(PENDING_ACTIVE)
    }

    /// ### English
    /// Releases a previously acquired slot, optionally recording a consumer fence.
    ///
    /// If the view is in `unsafe_no_consumer_fence` mode, the fence value is ignored (treated as 0).
    ///
    /// #### Parameters
    /// - `slot`: Triple-buffer slot index (0..=2).
    /// - `consumer_fence`: Consumer fence handle (`GLsync` cast to `u64`), or 0 to skip.
    ///
    /// ### 中文
    /// 释放之前 acquire 的槽位，并可选记录 consumer fence。
    ///
    /// 若 view 处于 `unsafe_no_consumer_fence` 模式，则 fence 会被忽略（视为 0）。
    ///
    /// #### 参数
    /// - `slot`：三缓冲槽位索引（0..=2）。
    /// - `consumer_fence`：consumer fence 句柄（`GLsync` 转 `u64`），为 0 则跳过。
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
    /// Sends a `DestroyView` command to the Servo thread on drop.
    ///
    /// ### 中文
    /// drop 时向 Servo 线程发送 `DestroyView` 命令。
    fn drop(&mut self) {
        self.command_queue.push(Command::DestroyView {
            id: self.id,
            token: self.token,
        });
        self.thread_handle.unpark();
    }
}
