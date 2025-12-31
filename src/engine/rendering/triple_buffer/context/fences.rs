use crate::engine::frame::{SLOT_FREE, SLOT_RELEASE_PENDING, TRIPLE_BUFFER_COUNT};
use glow::HasContext as _;

use super::GlfwTripleBufferRenderingContext;

impl GlfwTripleBufferRenderingContext {
    pub(in crate::engine::rendering::triple_buffer) fn delete_producer_fence_if_any(
        &self,
        slot: usize,
    ) {
        let fence_value = self.shared.get_producer_fence(slot);
        if fence_value == 0 {
            return;
        }
        let sync = glow::NativeFence(fence_value as usize as *mut _);
        unsafe {
            self.glow.delete_sync(sync);
        }
        self.shared.clear_producer_fence(slot);
    }

    pub(in crate::engine::rendering::triple_buffer) fn delete_consumer_fence_if_any(
        &self,
        slot: usize,
    ) {
        let fence_value = self.shared.get_consumer_fence(slot);
        if fence_value == 0 {
            return;
        }
        let sync = glow::NativeFence(fence_value as usize as *mut _);
        unsafe {
            self.glow.delete_sync(sync);
        }
        self.shared.clear_consumer_fence(slot);
    }

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
                unsafe {
                    self.glow.delete_sync(sync);
                }
                self.shared.clear_consumer_fence(slot);
            }
        }
    }
}
