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

pub(super) struct WebEngineViewHandleInit {
    pub id: u32,
    pub token: u64,
    pub shared: Arc<SharedFrameState>,
    pub mouse_move: Arc<CoalescedMouseMove>,
    pub resize: Arc<CoalescedResize>,
    pub input_queue: Arc<InputEventQueue>,
    pub load_url: Arc<CoalescedLoadUrl>,
    pub pending: Arc<PendingWork>,
    pub pending_queue: Arc<PendingIdQueue>,
    pub command_queue: Arc<CommandQueue>,
    pub thread_handle: thread::Thread,
    pub unsafe_no_consumer_fence: bool,
}

/// ### English
/// Opaque handle for a single view (thread-safe to use from the embedder thread).
///
/// ### 中文
/// 单个 view 的不透明句柄（可在宿主线程安全调用）。
pub struct WebEngineViewHandle {
    id: u32,
    token: u64,
    shared: Arc<SharedFrameState>,
    mouse_move: Arc<CoalescedMouseMove>,
    resize: Arc<CoalescedResize>,
    input_queue: Arc<InputEventQueue>,
    load_url: Arc<CoalescedLoadUrl>,
    pending: Arc<PendingWork>,
    pending_queue: Arc<PendingIdQueue>,
    command_queue: Arc<CommandQueue>,
    thread_handle: thread::Thread,
    unsafe_no_consumer_fence: bool,
}

impl WebEngineViewHandle {
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

    #[inline]
    fn mark_pending(&self, bits: u8) -> bool {
        if !self.pending.mark(bits) {
            return false;
        }
        let _ = self.pending_queue.push(self.id);
        true
    }

    pub fn is_active(&self) -> bool {
        self.shared.is_active()
    }

    pub fn queue_mouse_move(&self, x: f32, y: f32) -> bool {
        self.mouse_move.set(x, y);
        self.mark_pending(PENDING_MOUSE_MOVE)
    }

    pub fn queue_resize(&self, size: PhysicalSize<u32>) -> bool {
        let width = size.width.max(1);
        let height = size.height.max(1);
        self.resize.set(width, height);
        self.mark_pending(PENDING_RESIZE)
    }

    pub fn try_enqueue_input_events(&self, events: &[XianWebEngineInputEvent]) -> usize {
        self.input_queue.try_push_slice(events)
    }

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
    /// ### 中文
    /// 请求在 Servo 线程加载一个 URL 字符串（每 view 合并；只保留最新一次）。
    pub fn load_url(&self, url: &str) -> bool {
        self.load_url.set_str(url);
        self.mark_pending(PENDING_LOAD_URL)
    }

    pub fn acquire_frame(&self) -> Option<AcquiredFrame> {
        self.shared.try_acquire_front()
    }

    /// ### English
    /// Marks this view active/inactive and applies hide/throttle on Servo thread.
    ///
    /// ### 中文
    /// 设置该 view 的 active/inactive，并在 Servo 线程应用 hide/throttle。
    pub fn set_active(&self, active: bool) -> bool {
        if self.shared.is_active() == active {
            return false;
        }

        self.shared.set_active(active);
        self.mark_pending(PENDING_ACTIVE)
    }

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
    fn drop(&mut self) {
        self.command_queue.push(Command::DestroyView {
            id: self.id,
            token: self.token,
        });
        self.thread_handle.unpark();
    }
}
