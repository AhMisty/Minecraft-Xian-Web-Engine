//! ### English
//! Single-producer optimized implementation for `InputEventQueue` (SPSC).
//!
//! ### 中文
//! `InputEventQueue` 的单生产者优化实现（SPSC）。

use std::sync::atomic::Ordering;

use crate::engine::input_types::XianWebEngineInputEvent;

use super::{INPUT_QUEUE_CAPACITY, INPUT_QUEUE_MASK, InputEventQueue};

impl InputEventQueue {
    /// ### English
    /// Single-producer bulk push path (SPSC).
    ///
    /// #### Parameters
    /// - `events`: Events to push.
    ///
    /// ### 中文
    /// 单生产者批量 push 路径（SPSC）。
    ///
    /// #### 参数
    /// - `events`：要 push 的事件切片。
    #[inline]
    pub(super) fn try_push_slice_spsc(&self, events: &[XianWebEngineInputEvent]) -> usize {
        debug_assert!(self.single_producer);

        let head = self.head.load(Ordering::Relaxed);
        let cached_tail = unsafe { *self.producer_cached_tail.get() };

        let mut tail = cached_tail;
        let mut used = head.wrapping_sub(tail);
        if used >= INPUT_QUEUE_CAPACITY {
            tail = self.tail.load(Ordering::Acquire);
            unsafe {
                *self.producer_cached_tail.get() = tail;
            }
            used = head.wrapping_sub(tail);
            if used >= INPUT_QUEUE_CAPACITY {
                return 0;
            }
        }

        let mut free = INPUT_QUEUE_CAPACITY - used;
        if free < events.len() {
            let fresh_tail = self.tail.load(Ordering::Acquire);
            if fresh_tail != tail {
                unsafe {
                    *self.producer_cached_tail.get() = fresh_tail;
                }
                tail = fresh_tail;
                used = head.wrapping_sub(tail);
                if used >= INPUT_QUEUE_CAPACITY {
                    return 0;
                }
                free = INPUT_QUEUE_CAPACITY - used;
            }
        }

        let accepted = events.len().min(free);
        for (offset, &event) in events.iter().take(accepted).enumerate() {
            let slot = &self.slots[head.wrapping_add(offset) & INPUT_QUEUE_MASK];
            unsafe {
                (*slot.value.get()).write(event);
            }
        }
        self.head
            .store(head.wrapping_add(accepted), Ordering::Release);
        accepted
    }

    /// ### English
    /// Single-consumer pop path for SPSC mode.
    ///
    /// ### 中文
    /// SPSC 模式下的单消费者 pop 路径。
    #[inline]
    pub(super) fn pop_spsc(&self) -> Option<XianWebEngineInputEvent> {
        let tail = self.tail.load(Ordering::Relaxed);
        let cached_head = unsafe { *self.consumer_cached_head.get() };
        if tail == cached_head {
            let head = self.head.load(Ordering::Acquire);
            unsafe {
                *self.consumer_cached_head.get() = head;
            }
            if tail == head {
                return None;
            }
        }

        let slot = &self.slots[tail & INPUT_QUEUE_MASK];
        let event = unsafe { (*slot.value.get()).assume_init_read() };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(event)
    }
}
