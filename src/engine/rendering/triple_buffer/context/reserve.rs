use crate::engine::frame::{SLOT_FREE, SLOT_READY, SLOT_RENDERING, TRIPLE_BUFFER_COUNT};

use super::GlfwTripleBufferRenderingContext;

impl GlfwTripleBufferRenderingContext {
    pub(in crate::engine::rendering::triple_buffer) fn ensure_slot_size(&self, slot: usize) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        let desired_size = self.size.get();
        let mut slots = self.slots.borrow_mut();
        let existing = &mut slots[slot];

        if existing.size == desired_size {
            return;
        }

        existing.resize(&self.gl, desired_size, self.internal_format);
        self.shared.set_slot_size(slot, desired_size);
    }

    pub(in crate::engine::rendering::triple_buffer) fn try_reserve_next_back_slot(
        &self,
        current_back: usize,
    ) -> Option<usize> {
        debug_assert_eq!(TRIPLE_BUFFER_COUNT, 3);
        let slot_a = (current_back + 1) % TRIPLE_BUFFER_COUNT;
        let slot_b = (current_back + 2) % TRIPLE_BUFFER_COUNT;

        /// ### English
        /// Fast path: most of the time, triple-buffering will have at least one FREE slot.
        ///
        /// ### 中文
        /// 快路径：大多数情况下三缓冲至少会有一个 FREE 槽位。
        if self
            .shared
            .compare_exchange_state_relaxed(slot_a, SLOT_FREE, SLOT_RENDERING)
        {
            self.delete_producer_fence_if_any(slot_a);
            if !self.unsafe_no_consumer_fence {
                self.delete_consumer_fence_if_any(slot_a);
            }
            self.ensure_slot_size(slot_a);
            return Some(slot_a);
        }

        if self
            .shared
            .compare_exchange_state_relaxed(slot_b, SLOT_FREE, SLOT_RENDERING)
        {
            self.delete_producer_fence_if_any(slot_b);
            if !self.unsafe_no_consumer_fence {
                self.delete_consumer_fence_if_any(slot_b);
            }
            self.ensure_slot_size(slot_b);
            return Some(slot_b);
        }

        /// ### English
        /// No FREE slots; steal a READY slot.
        /// Prefer stealing the oldest READY slot so the newest stays available to the consumer thread.
        ///
        /// ### 中文
        /// 没有 FREE 槽位；抢占一个 READY 槽位。
        /// 优先抢占最旧的 READY，避免把最新帧从消费者手里抢走。
        let state_a = self.shared.slot_state_relaxed(slot_a);
        let state_b = self.shared.slot_state_relaxed(slot_b);

        let mut first = None::<usize>;
        let mut second = None::<usize>;

        match (state_a == SLOT_READY, state_b == SLOT_READY) {
            (true, true) => {
                let seq_a = self.shared.slot_seq_relaxed(slot_a);
                let seq_b = self.shared.slot_seq_relaxed(slot_b);
                if seq_a <= seq_b {
                    first = Some(slot_a);
                    second = Some(slot_b);
                } else {
                    first = Some(slot_b);
                    second = Some(slot_a);
                }
            }
            (true, false) => first = Some(slot_a),
            (false, true) => first = Some(slot_b),
            (false, false) => {}
        }

        for slot in [first, second].into_iter().flatten() {
            if self
                .shared
                .compare_exchange_state_relaxed(slot, SLOT_READY, SLOT_RENDERING)
            {
                self.delete_producer_fence_if_any(slot);
                if !self.unsafe_no_consumer_fence {
                    self.delete_consumer_fence_if_any(slot);
                }
                self.ensure_slot_size(slot);
                return Some(slot);
            }
        }

        /// ### English
        /// No FREE/READY slots.
        /// In safe mode, try to reclaim RELEASE_PENDING slots by polling consumer fences (non-blocking).
        /// Only do this on the slow path to avoid an extra GL sync query on every frame.
        ///
        /// ### 中文
        /// 没有 FREE/READY 槽位。
        /// 安全模式下通过轮询 consumer fence（非阻塞）回收 RELEASE_PENDING 槽位。
        /// 仅在慢路径执行，避免每帧额外的 GL 同步查询。
        if !self.unsafe_no_consumer_fence {
            self.reclaim_release_pending_slots();

            if self
                .shared
                .compare_exchange_state_relaxed(slot_a, SLOT_FREE, SLOT_RENDERING)
            {
                self.delete_producer_fence_if_any(slot_a);
                self.delete_consumer_fence_if_any(slot_a);
                self.ensure_slot_size(slot_a);
                return Some(slot_a);
            }

            if self
                .shared
                .compare_exchange_state_relaxed(slot_b, SLOT_FREE, SLOT_RENDERING)
            {
                self.delete_producer_fence_if_any(slot_b);
                self.delete_consumer_fence_if_any(slot_b);
                self.ensure_slot_size(slot_b);
                return Some(slot_b);
            }
        }

        None
    }

    /// ### English
    /// Returns whether the associated view is active.
    ///
    /// ### 中文
    /// 返回关联 view 是否 active。
    pub fn is_active(&self) -> bool {
        self.shared.is_active()
    }

    /// ### English
    /// Tries to reserve the next back slot before Servo paints.
    ///
    /// This reduces the chance that `present()` fails due to a lack of slots when the consumer
    /// is temporarily holding a texture.
    ///
    /// ### 中文
    /// 在 Servo paint 之前预留下一 back 槽位。
    ///
    /// 这可降低 `present()` 因暂时没有可用槽位而失败的概率（例如消费者线程短暂持有纹理时）。
    pub fn preflight_reserve_next_back_slot(&self) -> bool {
        if self.reserved_next_back.get().is_some() {
            return true;
        }

        let _ = servo::RenderingContext::make_current(self);
        let current_back = self.back_slot.get();
        let Some(next_back) = self.try_reserve_next_back_slot(current_back) else {
            return false;
        };

        self.reserved_next_back.set(Some(next_back));
        true
    }
}
