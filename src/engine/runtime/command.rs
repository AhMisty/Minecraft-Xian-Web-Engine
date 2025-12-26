//! ### English
//! Internal command protocol between embedder threads and the dedicated Servo thread.
//!
//! ### 中文
//! 宿主线程与独立 Servo 线程之间的内部命令协议。

use std::sync::Arc;

use crossbeam_channel as channel;
use dpi::PhysicalSize;
use url::Url;

use crate::engine::frame::SharedFrameState;
use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};

/// ### English
/// Commands sent from embedder threads to the dedicated Servo thread.
/// The Servo thread drains them in batches.
///
/// ### 中文
/// 从宿主线程发送到独立 Servo 线程的命令。
/// Servo 线程会批量 drain 这些命令以降低调度/唤醒开销。
pub(super) enum Command {
    /// ### English
    /// Creates one view on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程创建一个 view。
    CreateView {
        /// ### English
        /// Unique view ID allocated by `EngineRuntime`.
        ///
        /// ### 中文
        /// 由 `EngineRuntime` 分配的 view 唯一 ID。
        id: u32,
        /// ### English
        /// Initial size in physical pixels.
        ///
        /// ### 中文
        /// 初始尺寸（物理像素）。
        initial_size: PhysicalSize<u32>,
        /// ### English
        /// Shared lock-free triple-buffer state for frame exchange with Java.
        ///
        /// ### 中文
        /// 与 Java 交换帧的无锁三缓冲共享状态。
        shared: Arc<SharedFrameState>,
        /// ### English
        /// Shared coalesced mouse-move state for this view.
        ///
        /// ### 中文
        /// 该 view 的鼠标移动合并状态（共享）。
        mouse_move: Arc<CoalescedMouseMove>,
        /// ### English
        /// Shared coalesced resize state for this view.
        ///
        /// ### 中文
        /// 该 view 的 resize 合并状态（共享）。
        resize: Arc<CoalescedResize>,
        /// ### English
        /// Per-view bounded input queue.
        ///
        /// ### 中文
        /// 每 view 的有界输入队列。
        input_queue: Arc<InputEventQueue>,
        /// ### English
        /// Target FPS hint (`0` = external vsync driven).
        ///
        /// ### 中文
        /// 目标 FPS 提示（`0` = 外部 vsync 驱动）。
        target_fps: u32,
        /// ### English
        /// Unsafe mode: ignore consumer fences (lower overhead; may overwrite textures still in use).
        ///
        /// ### 中文
        /// 不安全模式：忽略 consumer fence（开销更低；可能覆盖仍在被消费的纹理）。
        unsafe_no_consumer_fence: bool,
        /// ### English
        /// Unsafe mode: skip producer fences for new frames (lower overhead; may sample incomplete frames).
        ///
        /// ### 中文
        /// 不安全模式：跳过生产者 fence（开销更低；可能采样到未完成帧）。
        unsafe_no_producer_fence: bool,
        /// ### English
        /// One-shot response channel for reporting success/failure back to the caller.
        ///
        /// ### 中文
        /// 一次性响应 channel：把成功/失败回传给调用方。
        response: channel::Sender<Result<(), String>>,
    },
    /// ### English
    /// Requests navigation to a URL (coalesced per view).
    ///
    /// ### 中文
    /// 请求跳转到 URL（对每个 view 做合并）。
    LoadUrl {
        /// ### English
        /// Target view ID.
        ///
        /// ### 中文
        /// 目标 view ID。
        id: u32,
        /// ### English
        /// Parsed URL to load.
        ///
        /// ### 中文
        /// 要加载的已解析 URL。
        url: Url,
    },
    /// ### English
    /// Sets whether a view is active (throttled/hidden when inactive).
    ///
    /// ### 中文
    /// 设置 view 是否 active（inactive 时节流/隐藏）。
    SetActive {
        /// ### English
        /// Target view ID.
        ///
        /// ### 中文
        /// 目标 view ID。
        id: u32,
        /// ### English
        /// Active flag.
        ///
        /// ### 中文
        /// active 标记。
        active: bool,
    },
    /// ### English
    /// Destroys a view and its GL resources on the Servo thread.
    ///
    /// ### 中文
    /// 在 Servo 线程销毁 view 及其 GL 资源。
    DestroyView {
        /// ### English
        /// Target view ID.
        ///
        /// ### 中文
        /// 目标 view ID。
        id: u32,
    },
    /// ### English
    /// Shuts down the Servo thread.
    ///
    /// ### 中文
    /// 关闭 Servo 线程。
    Shutdown,
}
