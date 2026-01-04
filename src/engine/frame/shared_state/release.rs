//! ### English
//! Consumer-side release helpers for `SharedFrameState`.
//!
//! Releases a HELD slot back to FREE, optionally recording a consumer fence.
//!
//! ### 中文
//! `SharedFrameState` 的消费者侧 release 辅助方法。
//!
//! 将 HELD 槽位释放回 FREE，并可选记录 consumer fence。

use std::sync::atomic::Ordering;

use super::super::{SLOT_FREE, SLOT_HELD, SLOT_RELEASE_PENDING, TRIPLE_BUFFER_COUNT};
use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Releases a previously acquired slot, optionally recording a consumer fence.
    ///
    /// #### Parameters
    /// - `slot`: Slot index previously acquired by the consumer.
    /// - `consumer_fence`: Consumer fence handle (`GLsync` cast to `u64`), or 0 to release immediately.
    ///
    /// ### 中文
    /// 释放之前 acquire 的槽位，并可选记录 consumer fence。
    ///
    /// #### 参数
    /// - `slot`：消费者之前 acquire 的槽位索引。
    /// - `consumer_fence`：consumer fence 句柄（`GLsync` 转 `u64`），为 0 则立即释放。
    pub fn release_slot(&self, slot: usize, consumer_fence: u64) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        if consumer_fence == 0 {
            if self.slots[slot]
                .state
                .compare_exchange(SLOT_HELD, SLOT_FREE, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                self.clear_consumer_fence(slot);
            }
            return;
        }

        if self.slots[slot].state.load(Ordering::Relaxed) != SLOT_HELD {
            return;
        }
        self.slots[slot]
            .consumer_fence
            .store(consumer_fence, Ordering::Relaxed);
        let _ = self.slots[slot].state.compare_exchange(
            SLOT_HELD,
            SLOT_RELEASE_PENDING,
            Ordering::Release,
            Ordering::Relaxed,
        );
    }
}
