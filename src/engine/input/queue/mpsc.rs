use std::sync::atomic::Ordering;

use crate::engine::input_types::XianWebEngineInputEvent;

use super::{INPUT_QUEUE_CAPACITY, INPUT_QUEUE_MASK, InputEventQueue};

impl InputEventQueue {
    /// ### English
    /// Multi-producer bounded enqueue path.
    ///
    /// ### 中文
    /// 多生产者有界入队路径。
    pub(super) fn try_push_mpsc(&self, event: XianWebEngineInputEvent) -> bool {
        let mut pos = self.enqueue_pos.load(Ordering::Relaxed);
        loop {
            let slot = &self.slots[pos & INPUT_QUEUE_MASK];
            let seq = slot.seq.load(Ordering::Acquire);
            let dif = seq as isize - pos as isize;

            if dif == 0 {
                match self.enqueue_pos.compare_exchange_weak(
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
            } else if dif < 0 {
                return false;
            } else {
                pos = self.enqueue_pos.load(Ordering::Relaxed);
            }
        }
    }

    /// ### English
    /// Single-consumer dequeue path for multi-producer mode (Servo thread).
    ///
    /// ### 中文
    /// 多生产者模式下的单消费者出队路径（Servo 线程）。
    pub(super) fn pop_mpsc(&self) -> Option<XianWebEngineInputEvent> {
        let pos = self.dequeue_pos.load(Ordering::Relaxed);
        let slot = &self.slots[pos & INPUT_QUEUE_MASK];
        let seq = slot.seq.load(Ordering::Acquire);
        let dif = seq as isize - pos.wrapping_add(1) as isize;

        if dif != 0 {
            return None;
        }

        self.dequeue_pos
            .store(pos.wrapping_add(1), Ordering::Relaxed);

        let event = unsafe { (*slot.value.get()).assume_init_read() };
        slot.seq
            .store(pos.wrapping_add(INPUT_QUEUE_CAPACITY), Ordering::Release);
        Some(event)
    }
}
