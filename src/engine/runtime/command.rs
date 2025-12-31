/// ### English
/// Internal command protocol between embedder threads and the dedicated Servo thread.
///
/// ### 中文
/// 宿主线程与独立 Servo 线程之间的内部命令协议。
use std::sync::Arc;

use dpi::PhysicalSize;

use crate::engine::frame::SharedFrameState;
use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};
use crate::engine::lockfree::OneShot;

use super::coalesced::{CoalescedLoadUrl, PendingWork};

/// ### English
/// Commands sent from embedder threads to the dedicated Servo thread.
///
/// ### 中文
/// 从宿主线程发送到独立 Servo 线程的控制命令。
pub(super) enum Command {
    /// ### English
    /// Creates one view on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程创建一个 view。
    CreateView {
        initial_size: PhysicalSize<u32>,
        shared: Arc<SharedFrameState>,
        mouse_move: Arc<CoalescedMouseMove>,
        resize: Arc<CoalescedResize>,
        input_queue: Arc<InputEventQueue>,
        /// ### English
        /// Coalesced URL load state (latest URL wins).
        ///
        /// ### 中文
        /// URL load 的合并状态（只保留最新一次）。
        load_url: Arc<CoalescedLoadUrl>,
        /// ### English
        /// Per-view pending work bitmask (used to coalesce wakeups and queueing).
        ///
        /// ### 中文
        /// 每 view 的 pending work bitmask（用于合并唤醒与入队）。
        pending: Arc<PendingWork>,
        target_fps: u32,
        unsafe_no_consumer_fence: bool,
        unsafe_no_producer_fence: bool,
        /// ### English
        /// One-shot response for reporting `(id, token)` or an error back to the caller.
        ///
        /// ### 中文
        /// 一次性回包：把 `(id, token)` 或错误返回给调用方。
        response: Arc<OneShot<Result<(u32, u64), String>>>,
    },
    /// ### English
    /// Destroys a view and its GL resources on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程销毁 view 并释放其 GL 资源。
    DestroyView { id: u32, token: u64 },
    /// ### English
    /// Shuts down the Servo thread.
    ///
    /// ### 中文
    /// 关闭 Servo 线程。
    Shutdown,
}
