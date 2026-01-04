//! ### English
//! Producer-side publish helpers for `SharedFrameState`.
//!
//! Marks a slot as READY and updates the global "latest" pointer.
//!
//! ### 中文
//! `SharedFrameState` 的生产者侧 publish 辅助方法。
//!
//! 将槽位标记为 READY，并更新全局 “latest” 指针。

use std::sync::atomic::Ordering;

use dpi::PhysicalSize;

use super::super::{SLOT_READY, TRIPLE_BUFFER_COUNT};
use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Publishes a rendered slot as READY and updates the global "latest" pointer.
    ///
    /// #### Parameters
    /// - `slot`: Slot index that was rendered.
    /// - `producer_fence`: Producer fence handle (`GLsync` cast to `u64`), or 0 if disabled/unavailable.
    /// - `new_frame_seq`: New frame sequence number (must be non-zero).
    ///
    /// ### 中文
    /// 将渲染完成的槽位发布为 READY，并更新全局 “latest” 指针。
    ///
    /// #### 参数
    /// - `slot`：已渲染完成的槽位索引。
    /// - `producer_fence`：生产者 fence 句柄（`GLsync` 转 `u64`），不可用/禁用则为 0。
    /// - `new_frame_seq`：新的帧序号（必须非 0）。
    pub fn publish(&self, slot: usize, producer_fence: u64, new_frame_seq: u64) {
        let slot_state = &self.slots[slot];
        slot_state.frame_seq.store(new_frame_seq, Ordering::Relaxed);
        slot_state
            .producer_fence
            .store(producer_fence, Ordering::Relaxed);
        slot_state.state.store(SLOT_READY, Ordering::Release);
        self.frame_meta
            .latest_packed
            .store(super::pack_latest(new_frame_seq, slot), Ordering::Release);
    }

    /// ### English
    /// Updates the cached size for a slot (used by the producer).
    ///
    /// #### Parameters
    /// - `slot`: Slot index to update.
    /// - `size`: New slot size in pixels.
    ///
    /// ### 中文
    /// 更新某个槽位的缓存尺寸（由生产者使用）。
    ///
    /// #### 参数
    /// - `slot`：需要更新的槽位索引。
    /// - `size`：新的槽位尺寸（像素）。
    pub fn set_slot_size(&self, slot: usize, size: PhysicalSize<u32>) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        let slot_state = &self.slots[slot];
        slot_state.width.store(size.width, Ordering::Relaxed);
        slot_state.height.store(size.height, Ordering::Relaxed);
    }

    /// ### English
    /// Stores the GL texture ID for a slot.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to update.
    /// - `texture_id`: GL texture ID backing the slot.
    ///
    /// ### 中文
    /// 写入某个槽位对应的 GL 纹理 ID。
    ///
    /// #### 参数
    /// - `slot`：需要更新的槽位索引。
    /// - `texture_id`：该槽位对应的 GL 纹理 ID。
    pub fn set_texture_id(&self, slot: usize, texture_id: u32) {
        self.slots[slot]
            .texture_id
            .store(texture_id, Ordering::Relaxed);
    }
}
