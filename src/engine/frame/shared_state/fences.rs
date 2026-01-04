//! ### English
//! Fence storage helpers for `SharedFrameState`.
//!
//! Stores producer/consumer fence handles (packed as `u64`) in per-slot atomics.
//!
//! ### 中文
//! `SharedFrameState` 的 fence 存储辅助方法。
//!
//! 以原子方式在每个槽位存储生产者/消费者 fence 句柄（以 `u64` 表示）。

use std::sync::atomic::Ordering;

use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Returns the producer fence for a slot (Relaxed load).
    ///
    /// #### Parameters
    /// - `slot`: Slot index to read.
    ///
    /// ### 中文
    /// 获取某个槽位的 producer fence（Relaxed 读取）。
    ///
    /// #### 参数
    /// - `slot`：要读取的槽位索引。
    pub fn get_producer_fence(&self, slot: usize) -> u64 {
        self.slots[slot].producer_fence.load(Ordering::Relaxed)
    }

    /// ### English
    /// Clears the producer fence for a slot.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to clear.
    ///
    /// ### 中文
    /// 清空某个槽位的 producer fence。
    ///
    /// #### 参数
    /// - `slot`：要清空的槽位索引。
    pub fn clear_producer_fence(&self, slot: usize) {
        self.slots[slot].producer_fence.store(0, Ordering::Relaxed);
    }

    /// ### English
    /// Returns the consumer fence for a slot (Relaxed load).
    ///
    /// #### Parameters
    /// - `slot`: Slot index to read.
    ///
    /// ### 中文
    /// 获取某个槽位的 consumer fence（Relaxed 读取）。
    ///
    /// #### 参数
    /// - `slot`：要读取的槽位索引。
    pub fn get_consumer_fence(&self, slot: usize) -> u64 {
        self.slots[slot].consumer_fence.load(Ordering::Relaxed)
    }

    /// ### English
    /// Clears the consumer fence for a slot.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to clear.
    ///
    /// ### 中文
    /// 清空某个槽位的 consumer fence。
    ///
    /// #### 参数
    /// - `slot`：要清空的槽位索引。
    pub fn clear_consumer_fence(&self, slot: usize) {
        self.slots[slot].consumer_fence.store(0, Ordering::Relaxed);
    }
}
