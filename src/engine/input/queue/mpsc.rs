//! ### English
//! Multi-producer push/pop implementation for `InputEventQueue`.
//!
//! ### 中文
//! `InputEventQueue` 的多生产者（MPSC）push/pop 实现。

use std::sync::atomic::Ordering;

use crate::engine::input_types::XianWebEngineInputEvent;

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
        let mut pos = self.head.load(Ordering::Relaxed);
        loop {
            let slot = &self.slots[pos & INPUT_QUEUE_MASK];
            let seq = slot.seq.load(Ordering::Acquire);
            let diff = seq.wrapping_sub(pos) as isize;

            if diff == 0 {
                match self.head.compare_exchange_weak(
                    pos,
                    pos.wrapping_add(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        unsafe {
                            (*slot.value.get()).write(event);
                        }
                        slot.seq.store(pos.wrapping_add(1), Ordering::Release);
                        return true;
                    }
                    Err(updated) => pos = updated,
                }
            } else if diff < 0 {
                return false;
            } else {
                pos = self.head.load(Ordering::Relaxed);
            }
        }
    }

    /// ### English
    /// Single-consumer pop path for multi-producer mode (Servo thread).
    ///
    /// ### 中文
    /// 多生产者模式下的单消费者 pop 路径（Servo 线程）。
    pub(super) fn pop_mpsc(&self) -> Option<XianWebEngineInputEvent> {
        let pos = self.tail.load(Ordering::Relaxed);
        let slot = &self.slots[pos & INPUT_QUEUE_MASK];
        let seq = slot.seq.load(Ordering::Acquire);
        let diff = seq.wrapping_sub(pos.wrapping_add(1)) as isize;

        if diff != 0 {
            return None;
        }

        self.tail.store(pos.wrapping_add(1), Ordering::Relaxed);

        let event = unsafe { (*slot.value.get()).assume_init_read() };
        slot.seq
            .store(pos.wrapping_add(INPUT_QUEUE_CAPACITY), Ordering::Release);
        Some(event)
    }
}
