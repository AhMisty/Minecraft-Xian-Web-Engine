//! ### English
//! Per-view Servo-thread state and delegate integration.
//!
//! ### 中文
//! Servo 线程内的每 view 状态与 delegate 集成。

use std::rc::Rc;
use std::sync::Arc;

use dpi::PhysicalSize;
use url::Url;

use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};
use crate::engine::rendering::GlfwTripleBufferRenderingContext;

use super::super::coalesced::{
    CoalescedLoadUrl, PENDING_ACTIVE, PENDING_INPUT, PENDING_LOAD_URL, PENDING_MOUSE_MOVE,
    PENDING_RESIZE, PendingWork,
};
use super::super::input_dispatch::dispatch_queued_input_event;

/// ### English
/// Servo `WebViewDelegate` implementation that drives paint/present for a view.
///
/// ### 中文
/// 驱动 view paint/present 的 Servo `WebViewDelegate` 实现。
pub(super) struct Delegate {
    /// ### English
    /// Rendering context used for `paint/present` on `notify_new_frame_ready`.
    ///
    /// ### 中文
    /// 在 `notify_new_frame_ready` 中用于 `paint/present` 的渲染上下文。
    rendering_context: Rc<GlfwTripleBufferRenderingContext>,
}

impl Delegate {
    /// ### English
    /// Creates a new delegate bound to the given rendering context.
    ///
    /// #### Parameters
    /// - `rendering_context`: Rendering context used for `paint/present`.
    ///
    /// ### 中文
    /// 创建一个绑定到指定渲染上下文的 delegate。
    ///
    /// #### 参数
    /// - `rendering_context`：用于 `paint/present` 的渲染上下文。
    pub(super) fn new(rendering_context: Rc<GlfwTripleBufferRenderingContext>) -> Self {
        Self { rendering_context }
    }
}

impl servo::WebViewDelegate for Delegate {
    /// ### English
    /// Called by Servo when a new frame can be rendered/presented.
    ///
    /// #### Parameters
    /// - `servo_webview`: WebView that should be painted and presented.
    ///
    /// ### 中文
    /// 当 Servo 通知有新帧可渲染/呈现时调用。
    ///
    /// #### 参数
    /// - `servo_webview`：需要执行 paint/present 的 WebView。
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

/// ### English
/// Per-view bookkeeping stored only on the Servo thread.
///
/// ### 中文
/// 仅 Servo 线程持有的每个 view 状态。
pub(super) struct ViewEntry {
    /// ### English
    /// Monotonic token associated with this view ID allocation.
    ///
    /// ### 中文
    /// 该 view ID 分配时绑定的单调 token，用于忽略“ID 复用后”的陈旧销毁命令。
    pub(super) token: u64,
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
    /// Shared coalesced resize state for this view.
    ///
    /// ### 中文
    /// 该 view 的 resize 合并状态（共享）。
    resize: Arc<CoalescedResize>,
    /// ### English
    /// Shared coalesced URL load state (latest URL wins).
    ///
    /// ### 中文
    /// 共享的 URL 合并状态（只保留最新一次）。
    load_url: Arc<CoalescedLoadUrl>,
    /// ### English
    /// Per-view pending work bitmask (coalesces wakeups and queueing).
    ///
    /// ### 中文
    /// 每 view 的 pending work bitmask（用于合并唤醒与 push）。
    pending: Arc<PendingWork>,
    /// ### English
    /// Last applied active flag (avoids redundant show/hide calls).
    ///
    /// ### 中文
    /// 上一次已应用的 active 值（用于避免重复 show/hide）。
    last_active: bool,
    /// ### English
    /// Last applied size (avoids redundant resize calls).
    ///
    /// ### 中文
    /// 上一次已应用的尺寸（用于避免重复 resize）。
    last_size: PhysicalSize<u32>,
}

impl ViewEntry {
    #[allow(clippy::too_many_arguments)]
    /// ### English
    /// Creates a per-view entry stored only on the Servo thread.
    ///
    /// #### Parameters
    /// - `token`: Monotonic token paired with the view ID.
    /// - `servo_webview`: Servo WebView instance for this view.
    /// - `rendering_context`: Rendering context owned by this view.
    /// - `mouse_move`: Shared coalesced mouse-move state.
    /// - `input_queue`: Shared bounded input queue.
    /// - `resize`: Shared coalesced resize state.
    /// - `load_url`: Shared coalesced URL load state.
    /// - `pending`: Shared pending-work bitmask.
    /// - `initial_size`: Initial view size used to seed cached state.
    ///
    /// ### 中文
    /// 创建一个仅由 Servo 线程持有的 view 条目。
    ///
    /// #### 参数
    /// - `token`：与 view ID 配对的单调递增 token。
    /// - `servo_webview`：该 view 对应的 Servo WebView 实例。
    /// - `rendering_context`：该 view 持有的渲染上下文。
    /// - `mouse_move`：共享的鼠标移动合并状态。
    /// - `input_queue`：共享的有界输入队列。
    /// - `resize`：共享的 resize 合并状态。
    /// - `load_url`：共享的 URL 合并状态。
    /// - `pending`：共享的 pending-work 位图。
    /// - `initial_size`：用于初始化缓存状态的初始尺寸。
    pub(super) fn new(
        token: u64,
        servo_webview: servo::WebView,
        rendering_context: Rc<GlfwTripleBufferRenderingContext>,
        mouse_move: Arc<CoalescedMouseMove>,
        input_queue: Arc<InputEventQueue>,
        resize: Arc<CoalescedResize>,
        load_url: Arc<CoalescedLoadUrl>,
        pending: Arc<PendingWork>,
        initial_size: PhysicalSize<u32>,
    ) -> Self {
        Self {
            token,
            servo_webview,
            rendering_context,
            mouse_move,
            input_queue,
            resize,
            load_url,
            pending,
            last_active: true,
            last_size: initial_size,
        }
    }

    #[inline]
    /// ### English
    /// Applies a pending resize if present (coalesced; latest wins).
    ///
    /// ### 中文
    /// 应用待处理的 resize（合并；只保留最新一次）。
    fn apply_resize(&mut self) {
        let Some(size) = self.resize.take() else {
            return;
        };
        if size == self.last_size {
            return;
        }
        self.last_size = size;
        self.servo_webview.resize(size);
    }

    #[inline]
    /// ### English
    /// Applies a pending mouse-move if present (coalesced; latest wins).
    ///
    /// ### 中文
    /// 应用待处理的鼠标移动（合并；只保留最新一次）。
    fn apply_mouse_move(&self) {
        if !self.rendering_context.is_active() {
            return;
        }

        let Some((x, y)) = self.mouse_move.take() else {
            return;
        };

        let point = servo::WebViewPoint::from(servo::DevicePoint::new(x, y));
        self.servo_webview
            .notify_input_event(servo::InputEvent::MouseMove(servo::MouseMoveEvent::new(
                point,
            )));
    }

    #[inline]
    /// ### English
    /// Drains the bounded input queue and dispatches events into Servo.
    ///
    /// ### 中文
    /// drain 有界输入队列并把事件派发到 Servo。
    fn drain_input_queue(&self) {
        loop {
            let active = self.rendering_context.is_active();
            while let Some(raw) = self.input_queue.pop() {
                if active {
                    dispatch_queued_input_event(&self.servo_webview, raw);
                }
            }

            self.input_queue.clear_pending();
            let Some(raw) = self.input_queue.pop() else {
                break;
            };
            self.input_queue.mark_pending();

            if self.rendering_context.is_active() {
                dispatch_queued_input_event(&self.servo_webview, raw);
            }
        }
    }

    #[inline]
    /// ### English
    /// Processes all pending work bits for this view.
    ///
    /// ### 中文
    /// 处理该 view 的所有待处理 work bit。
    pub(super) fn process_pending(&mut self) {
        if !self.pending.is_marked() {
            return;
        }

        loop {
            let bits = self.pending.take();

            if (bits & PENDING_LOAD_URL) != 0
                && let Some(request) = self.load_url.take()
            {
                if let Ok(url) = Url::parse(request.as_str()) {
                    self.servo_webview.load(url);
                }
                self.load_url.recycle(request);
            }

            if (bits & PENDING_ACTIVE) != 0 {
                let active = self.rendering_context.is_active();
                if active != self.last_active {
                    self.last_active = active;
                    if active {
                        self.servo_webview.set_throttled(false);
                        self.servo_webview.show();
                    } else {
                        self.servo_webview.set_throttled(true);
                        self.servo_webview.hide();
                    }
                }
            }

            if (bits & PENDING_RESIZE) != 0 {
                self.apply_resize();
            }

            if (bits & PENDING_MOUSE_MOVE) != 0 {
                self.apply_mouse_move();
            }

            if (bits & PENDING_INPUT) != 0 {
                self.drain_input_queue();
            }

            if self.pending.is_busy_only() && self.pending.clear_busy_if_idle() {
                break;
            }
        }
    }
}
