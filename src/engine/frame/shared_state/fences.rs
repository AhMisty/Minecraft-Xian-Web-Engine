use std::sync::atomic::Ordering;

use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Returns the producer fence for a slot (Relaxed load).
    ///
    /// ### 中文
    /// 获取某个槽位的 producer fence（Relaxed 读取）。
    pub fn get_producer_fence(&self, slot: usize) -> u64 {
        self.slots[slot].producer_fence.load(Ordering::Relaxed)
    }

    /// ### English
    /// Clears the producer fence for a slot.
    ///
    /// ### 中文
    /// 清空某个槽位的 producer fence。
    pub fn clear_producer_fence(&self, slot: usize) {
        self.slots[slot].producer_fence.store(0, Ordering::Relaxed);
    }

    /// ### English
    /// Returns the consumer fence for a slot (Relaxed load).
    ///
    /// ### 中文
    /// 获取某个槽位的 consumer fence（Relaxed 读取）。
    pub fn get_consumer_fence(&self, slot: usize) -> u64 {
        self.slots[slot].consumer_fence.load(Ordering::Relaxed)
    }

    /// ### English
    /// Clears the consumer fence for a slot.
    ///
    /// ### 中文
    /// 清空某个槽位的 consumer fence。
    pub fn clear_consumer_fence(&self, slot: usize) {
        self.slots[slot].consumer_fence.store(0, Ordering::Relaxed);
    }
}
