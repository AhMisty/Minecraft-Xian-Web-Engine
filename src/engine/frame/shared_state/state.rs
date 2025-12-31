use std::sync::atomic::Ordering;

use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Loads a slot sequence number with Relaxed ordering (producer-side heuristic).
    ///
    /// ### 中文
    /// 以 Relaxed 顺序读取槽位序列号（生产者侧启发式用）。
    pub fn slot_seq_relaxed(&self, slot: usize) -> u64 {
        self.slots[slot].frame_seq.load(Ordering::Relaxed)
    }

    /// ### English
    /// Loads a slot state with Acquire ordering.
    ///
    /// ### 中文
    /// 以 Acquire 顺序读取槽位状态。
    pub fn slot_state(&self, slot: usize) -> u8 {
        self.slots[slot].state.load(Ordering::Acquire)
    }

    /// ### English
    /// Loads a slot state with Relaxed ordering (hot-path probing).
    ///
    /// ### 中文
    /// 以 Relaxed 顺序读取槽位状态（热路径探测）。
    pub fn slot_state_relaxed(&self, slot: usize) -> u8 {
        self.slots[slot].state.load(Ordering::Relaxed)
    }

    /// ### English
    /// CAS a slot state with `AcqRel` on success and `Acquire` on failure.
    ///
    /// ### 中文
    /// 以 `AcqRel`（成功）/`Acquire`（失败）对槽位状态做 CAS。
    pub fn compare_exchange_state(&self, slot: usize, current: u8, new: u8) -> Result<u8, u8> {
        self.slots[slot]
            .state
            .compare_exchange(current, new, Ordering::AcqRel, Ordering::Acquire)
    }

    /// ### English
    /// CAS a slot state with Relaxed ordering (used where fences already imply ordering).
    ///
    /// ### 中文
    /// 以 Relaxed 顺序对槽位状态做 CAS（用于已由 fence 保证顺序的场景）。
    pub fn compare_exchange_state_relaxed(&self, slot: usize, current: u8, new: u8) -> bool {
        self.slots[slot]
            .state
            .compare_exchange(current, new, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }

    /// ### English
    /// Stores a slot state with Release ordering.
    ///
    /// ### 中文
    /// 以 Release 顺序写入槽位状态。
    pub fn store_state(&self, slot: usize, state: u8) {
        self.slots[slot].state.store(state, Ordering::Release);
    }
}
