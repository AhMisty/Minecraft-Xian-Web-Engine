//! ### English
//! Slot reservation logic for the triple-buffered rendering context.
//!
//! ### 中文
//! 三缓冲渲染上下文的槽位预留逻辑。

use crate::engine::frame::{SLOT_FREE, SLOT_READY, SLOT_RENDERING, TRIPLE_BUFFER_COUNT};

use super::GlfwTripleBufferRenderingContext;

impl GlfwTripleBufferRenderingContext {
    /// ### English
    /// Prepares a slot for rendering by cleaning up fences and ensuring the texture size.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to prepare.
    ///
    /// ### 中文
    /// 为渲染准备一个槽位：清理 fences，并确保纹理尺寸正确。
    ///
    /// #### 参数
    /// - `slot`：需要准备的槽位索引。
    #[inline]
    fn prepare_slot_for_rendering(&self, slot: usize) {
        self.delete_producer_fence_if_any(slot);
        if !self.unsafe_no_consumer_fence {
            self.delete_consumer_fence_if_any(slot);
        }
        self.ensure_slot_size(slot);
    }

    /// ### English
    /// Ensures the GL resources for `slot` match the current desired size.
    ///
    /// ### 中文
    /// 确保 `slot` 的 GL 资源尺寸与当前期望尺寸一致。
    pub(in crate::engine::rendering::triple_buffer) fn ensure_slot_size(&self, slot: usize) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        let desired_size = self.size.get();
        self.with_slots_mut(|slots| {
            let existing = &mut slots[slot];
            if existing.size == desired_size {
                return;
            }

            existing.resize(&self.gl, desired_size, self.internal_format);
            self.shared.set_slot_size(slot, desired_size);
        });
    }

    /// ### English
    /// Tries to reserve the next back slot for the producer.
    ///
    /// Strategy (triple-buffer, two candidates besides `current_back`):
    /// - Fast path: reserve any FREE slot.
    /// - Fallback: steal a READY slot, preferring the older READY to avoid stealing the newest frame.
    /// - Safe mode: if no FREE/READY, poll consumer fences to reclaim RELEASE_PENDING and retry.
    ///
    /// ### 中文
    /// 尝试为生产者预留下一 back 槽位。
    ///
    /// 策略（三缓冲，候选为 `current_back` 之外的两个槽位）：
    /// - 快路径：优先预留任意 FREE 槽位。
    /// - 回退：抢占 READY 槽位，并优先抢占更旧的 READY，避免把最新帧从消费者手里抢走。
    /// - 安全模式：若没有 FREE/READY，则轮询 consumer fence 回收 RELEASE_PENDING，再重试。
    pub(in crate::engine::rendering::triple_buffer) fn try_reserve_next_back_slot(
        &self,
        current_back: usize,
    ) -> Option<usize> {
        debug_assert_eq!(TRIPLE_BUFFER_COUNT, 3);
        let slot_a = (current_back + 1) % TRIPLE_BUFFER_COUNT;
        let slot_b = (current_back + 2) % TRIPLE_BUFFER_COUNT;

        for slot in [slot_a, slot_b] {
            if self
                .shared
                .compare_exchange_state_relaxed(slot, SLOT_FREE, SLOT_RENDERING)
            {
                self.prepare_slot_for_rendering(slot);
                return Some(slot);
            }
        }

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
                self.prepare_slot_for_rendering(slot);
                return Some(slot);
            }
        }

        if !self.unsafe_no_consumer_fence {
            self.reclaim_release_pending_slots();

            for slot in [slot_a, slot_b] {
                if self
                    .shared
                    .compare_exchange_state_relaxed(slot, SLOT_FREE, SLOT_RENDERING)
                {
                    self.prepare_slot_for_rendering(slot);
                    return Some(slot);
                }
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
