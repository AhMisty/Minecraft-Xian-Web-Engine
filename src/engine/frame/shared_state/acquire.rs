//! ### English
//! Consumer-side acquire logic for `SharedFrameState`.
//!
//! Promotes a READY slot into HELD and returns an `AcquiredFrame` snapshot.
//!
//! ### 中文
//! `SharedFrameState` 的消费者侧 acquire 逻辑。
//!
//! 将 READY 槽位提升为 HELD，并返回 `AcquiredFrame` 快照。

use std::sync::atomic::Ordering;

use dpi::PhysicalSize;

use super::super::{AcquiredFrame, SLOT_HELD, SLOT_READY, TRIPLE_BUFFER_COUNT};
use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Tries to acquire the latest READY slot as HELD (consumer-side).
    ///
    /// ### 中文
    /// 尝试将最新的 READY 槽位 acquire 为 HELD（消费者侧）。
    pub fn try_acquire_front(&self) -> Option<AcquiredFrame> {
        if self.is_resizing() {
            return None;
        }

        let packed = self.frame_meta.latest_packed.load(Ordering::Acquire);
        let (latest, front_hint) = super::unpack_latest(packed);
        if latest == 0 {
            return None;
        }

        self.try_acquire_ready_slot(front_hint)
    }

    /// ### English
    /// Tries to acquire a READY slot as HELD, preferring `front` as a hint.
    ///
    /// This performs a fast-path CAS on the hinted slot, then falls back to probing the other two
    /// slots (triple-buffer) and prefers the newest frame sequence.
    ///
    /// ### 中文
    /// 尝试将某个 READY 槽位 acquire 为 HELD，并优先使用 `front` 作为 hint。
    ///
    /// 先对 hint 槽位做快路径 CAS；失败后回退到探测另外两个槽位（三缓冲），并优先选择帧序号更新的槽位。
    fn try_acquire_ready_slot(&self, front: usize) -> Option<AcquiredFrame> {
        let front = if front < TRIPLE_BUFFER_COUNT {
            front
        } else {
            0
        };

        if self.slots[front]
            .state
            .compare_exchange(SLOT_READY, SLOT_HELD, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            return Some(self.acquired_frame(front));
        }

        debug_assert_eq!(TRIPLE_BUFFER_COUNT, 3);
        let slot_a = (front + 1) % TRIPLE_BUFFER_COUNT;
        let slot_b = (front + 2) % TRIPLE_BUFFER_COUNT;

        let state_a = self.slots[slot_a].state.load(Ordering::Relaxed);
        let state_b = self.slots[slot_b].state.load(Ordering::Relaxed);

        let mut first = None::<usize>;
        let mut second = None::<usize>;

        match (state_a == SLOT_READY, state_b == SLOT_READY) {
            (true, true) => {
                let seq_a = self.slots[slot_a].frame_seq.load(Ordering::Relaxed);
                let seq_b = self.slots[slot_b].frame_seq.load(Ordering::Relaxed);
                if seq_a >= seq_b {
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
            if self.slots[slot]
                .state
                .compare_exchange(SLOT_READY, SLOT_HELD, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Some(self.acquired_frame(slot));
            }
        }

        None
    }

    /// ### English
    /// Builds an `AcquiredFrame` snapshot for the given slot using Relaxed loads.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to snapshot.
    ///
    /// ### 中文
    /// 使用 Relaxed load 为指定槽位构造 `AcquiredFrame` 快照。
    ///
    /// #### 参数
    /// - `slot`：需要构造快照的槽位索引。
    fn acquired_frame(&self, slot: usize) -> AcquiredFrame {
        let slot_state = &self.slots[slot];
        let size = PhysicalSize::new(
            slot_state.width.load(Ordering::Relaxed),
            slot_state.height.load(Ordering::Relaxed),
        );
        AcquiredFrame {
            slot,
            texture_id: slot_state.texture_id.load(Ordering::Relaxed),
            producer_fence: slot_state.producer_fence.load(Ordering::Relaxed),
            width: size.width,
            height: size.height,
        }
    }
}
