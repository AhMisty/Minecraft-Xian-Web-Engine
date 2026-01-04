use std::sync::atomic::Ordering;

use crate::engine::input_types::XianWebEngineInputEvent;

use super::{INPUT_QUEUE_CAPACITY, INPUT_QUEUE_MASK, InputEventQueue};

impl InputEventQueue {
    /// ### English
    /// Single-producer bulk enqueue path (SPSC).
    ///
    /// ### 中文
    /// 单生产者批量入队路径（SPSC）。
    #[inline]
    pub(super) fn try_push_slice_spsc(&self, events: &[XianWebEngineInputEvent]) -> usize {
        debug_assert!(self.single_producer);

        let head = self.enqueue_pos.load(Ordering::Relaxed);
        let cached_tail = unsafe { *self.producer_cached_dequeue.get() };

        let mut tail = cached_tail;
        let mut used = head.wrapping_sub(tail);
        if used >= INPUT_QUEUE_CAPACITY {
            tail = self.dequeue_pos.load(Ordering::Acquire);
            unsafe {
                *self.producer_cached_dequeue.get() = tail;
            }
            used = head.wrapping_sub(tail);
            if used >= INPUT_QUEUE_CAPACITY {
                return 0;
            }
        }

        let mut free = INPUT_QUEUE_CAPACITY - used;
        if free < events.len() {
            let fresh_tail = self.dequeue_pos.load(Ordering::Acquire);
            if fresh_tail != tail {
                unsafe {
                    *self.producer_cached_dequeue.get() = fresh_tail;
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
        self.enqueue_pos
            .store(head.wrapping_add(accepted), Ordering::Release);
        accepted
    }

    /// ### English
    /// Single-consumer dequeue path for SPSC mode.
    ///
    /// ### 中文
    /// SPSC 模式下的单消费者出队路径。
    #[inline]
    pub(super) fn pop_spsc(&self) -> Option<XianWebEngineInputEvent> {
        let tail = self.dequeue_pos.load(Ordering::Relaxed);
        let cached_head = unsafe { *self.consumer_cached_enqueue.get() };
        if tail == cached_head {
            let head = self.enqueue_pos.load(Ordering::Acquire);
            unsafe {
                *self.consumer_cached_enqueue.get() = head;
            }
            if tail == head {
                return None;
            }
        }

        let slot = &self.slots[tail & INPUT_QUEUE_MASK];
        let event = unsafe { (*slot.value.get()).assume_init_read() };
        self.dequeue_pos
            .store(tail.wrapping_add(1), Ordering::Release);
        Some(event)
    }
}
