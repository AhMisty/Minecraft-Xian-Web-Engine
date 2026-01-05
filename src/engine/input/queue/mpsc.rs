//! ### English
//! Multi-producer push/pop implementation for `InputEventQueue`.
//!
//! ### 中文
//! `InputEventQueue` 的多生产者（MPSC）push/pop 实现。

use crate::engine::input_types::XianWebEngineInputEvent;
use crate::engine::lockfree::{bounded_mpsc_pop, bounded_mpsc_try_push};

use super::{INPUT_QUEUE_CAPACITY, INPUT_QUEUE_MASK, InputEventQueue};

impl InputEventQueue {
    /// ### English
    /// Multi-producer bounded push path.
    ///
    /// #### Parameters
    /// - `event`: Input event to push.
    ///
    /// ### 中文
    /// 多生产者有界 push 路径。
    ///
    /// #### 参数
    /// - `event`：要 push 的输入事件。
    pub(super) fn try_push_mpsc(&self, event: XianWebEngineInputEvent) -> bool {
        bounded_mpsc_try_push(&self.head, &self.slots, INPUT_QUEUE_MASK, event).is_ok()
    }

    /// ### English
    /// Single-consumer pop path for multi-producer mode (Servo thread).
    ///
    /// ### 中文
    /// 多生产者模式下的单消费者 pop 路径（Servo 线程）。
    pub(super) fn pop_mpsc(&self) -> Option<XianWebEngineInputEvent> {
        bounded_mpsc_pop(
            &self.tail,
            &self.slots,
            INPUT_QUEUE_MASK,
            INPUT_QUEUE_CAPACITY,
        )
    }
}
