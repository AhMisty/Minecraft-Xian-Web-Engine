//! ### English
//! GL fence management for the triple-buffered rendering context.
//!
//! ### 中文
//! 三缓冲渲染上下文的 GL fence 管理。

use crate::engine::frame::{SLOT_FREE, SLOT_RELEASE_PENDING, TRIPLE_BUFFER_COUNT};
use glow::HasContext as _;

use super::GlfwTripleBufferRenderingContext;

impl GlfwTripleBufferRenderingContext {
    /// ### English
    /// Deletes one `GLsync` represented as a `u64` handle.
    ///
    /// #### Parameters
    /// - `fence_value`: Fence handle (`GLsync` cast to `u64`).
    ///
    /// ### 中文
    /// 删除一个以 `u64` 句柄表示的 `GLsync`。
    ///
    /// #### 参数
    /// - `fence_value`：fence 句柄（`GLsync` 转为 `u64`）。
    #[inline]
    fn delete_sync_unsafe(&self, fence_value: u64) {
        let sync = glow::NativeFence(fence_value as usize as *mut _);
        unsafe {
            self.glow.delete_sync(sync);
        }
    }

    /// ### English
    /// Deletes the producer fence for `slot` if present and clears it in shared state.
    ///
    /// ### 中文
    /// 若 `slot` 存在生产者 fence，则删除该 fence 并在共享状态中清空。
    pub(in crate::engine::rendering::triple_buffer) fn delete_producer_fence_if_any(
        &self,
        slot: usize,
    ) {
        let fence_value = self.shared.get_producer_fence(slot);
        if fence_value == 0 {
            return;
        }
        self.delete_sync_unsafe(fence_value);
        self.shared.clear_producer_fence(slot);
    }

    /// ### English
    /// Deletes the consumer fence for `slot` if present and clears it in shared state.
    ///
    /// ### 中文
    /// 若 `slot` 存在消费者 fence，则删除该 fence 并在共享状态中清空。
    pub(in crate::engine::rendering::triple_buffer) fn delete_consumer_fence_if_any(
        &self,
        slot: usize,
    ) {
        let fence_value = self.shared.get_consumer_fence(slot);
        if fence_value == 0 {
            return;
        }
        self.delete_sync_unsafe(fence_value);
        self.shared.clear_consumer_fence(slot);
    }

    /// ### English
    /// Reclaims slots in `SLOT_RELEASE_PENDING` by polling consumer fences (non-blocking).
    ///
    /// Slots with signaled fences are transitioned back to `SLOT_FREE`.
    ///
    /// ### 中文
    /// 通过轮询 consumer fence（非阻塞）回收处于 `SLOT_RELEASE_PENDING` 的槽位。
    ///
    /// fence 已 signal 的槽位会被转换回 `SLOT_FREE`。
    pub(super) fn reclaim_release_pending_slots(&self) {
        for slot in 0..TRIPLE_BUFFER_COUNT {
            if self.shared.slot_state(slot) != SLOT_RELEASE_PENDING {
                continue;
            }

            let consumer_fence = self.shared.get_consumer_fence(slot);
            if consumer_fence == 0 {
                if self
                    .shared
                    .compare_exchange_state(slot, SLOT_RELEASE_PENDING, SLOT_FREE)
                    .is_ok()
                {
                    self.shared.clear_consumer_fence(slot);
                }
                continue;
            }

            let sync = glow::NativeFence(consumer_fence as usize as *mut _);
            let status = unsafe { self.glow.client_wait_sync(sync, 0, 0) };
            if status != glow::ALREADY_SIGNALED && status != glow::CONDITION_SATISFIED {
                continue;
            }

            if self
                .shared
                .compare_exchange_state(slot, SLOT_RELEASE_PENDING, SLOT_FREE)
                .is_ok()
            {
                self.delete_sync_unsafe(consumer_fence);
                self.shared.clear_consumer_fence(slot);
            }
        }
    }
}
