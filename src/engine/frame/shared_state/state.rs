//! ### English
//! Slot state accessors for `SharedFrameState`.
//!
//! Provides hot-path loads and CAS helpers with carefully chosen memory orderings.
//!
//! ### 中文
//! `SharedFrameState` 的槽位状态访问器。
//!
//! 提供热路径读取与 CAS 辅助方法，并使用精心选择的内存序。

use std::sync::atomic::Ordering;

use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Loads a slot sequence number with Relaxed ordering (producer-side heuristic).
    ///
    /// #### Parameters
    /// - `slot`: Slot index to read.
    ///
    /// ### 中文
    /// 以 Relaxed 顺序读取槽位序列号（生产者侧启发式用）。
    ///
    /// #### 参数
    /// - `slot`：要读取的槽位索引。
    pub fn slot_seq_relaxed(&self, slot: usize) -> u64 {
        self.slots[slot].frame_seq.load(Ordering::Relaxed)
    }

    /// ### English
    /// Loads a slot state with Acquire ordering.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to read.
    ///
    /// ### 中文
    /// 以 Acquire 顺序读取槽位状态。
    ///
    /// #### 参数
    /// - `slot`：要读取的槽位索引。
    pub fn slot_state(&self, slot: usize) -> u8 {
        self.slots[slot].state.load(Ordering::Acquire)
    }

    /// ### English
    /// Loads a slot state with Relaxed ordering (hot-path probing).
    ///
    /// #### Parameters
    /// - `slot`: Slot index to read.
    ///
    /// ### 中文
    /// 以 Relaxed 顺序读取槽位状态（热路径探测）。
    ///
    /// #### 参数
    /// - `slot`：要读取的槽位索引。
    pub fn slot_state_relaxed(&self, slot: usize) -> u8 {
        self.slots[slot].state.load(Ordering::Relaxed)
    }

    /// ### English
    /// CAS a slot state with `AcqRel` on success and `Acquire` on failure.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to update.
    /// - `current`: Expected current state.
    /// - `new`: New state to store on success.
    ///
    /// ### 中文
    /// 以 `AcqRel`（成功）/`Acquire`（失败）对槽位状态做 CAS。
    ///
    /// #### 参数
    /// - `slot`：要更新的槽位索引。
    /// - `current`：期望的当前状态。
    /// - `new`：CAS 成功时写入的新状态。
    pub fn compare_exchange_state(&self, slot: usize, current: u8, new: u8) -> Result<u8, u8> {
        self.slots[slot]
            .state
            .compare_exchange(current, new, Ordering::AcqRel, Ordering::Acquire)
    }

    /// ### English
    /// CAS a slot state with Relaxed ordering (used where fences already imply ordering).
    ///
    /// #### Parameters
    /// - `slot`: Slot index to update.
    /// - `current`: Expected current state.
    /// - `new`: New state to store on success.
    ///
    /// ### 中文
    /// 以 Relaxed 顺序对槽位状态做 CAS（用于已由 fence 保证顺序的场景）。
    ///
    /// #### 参数
    /// - `slot`：要更新的槽位索引。
    /// - `current`：期望的当前状态。
    /// - `new`：CAS 成功时写入的新状态。
    pub fn compare_exchange_state_relaxed(&self, slot: usize, current: u8, new: u8) -> bool {
        self.slots[slot]
            .state
            .compare_exchange(current, new, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }

    /// ### English
    /// Stores a slot state with Release ordering.
    ///
    /// #### Parameters
    /// - `slot`: Slot index to update.
    /// - `state`: State value to store.
    ///
    /// ### 中文
    /// 以 Release 顺序写入槽位状态。
    ///
    /// #### 参数
    /// - `slot`：要更新的槽位索引。
    /// - `state`：要写入的状态值。
    pub fn store_state(&self, slot: usize, state: u8) {
        self.slots[slot].state.store(state, Ordering::Release);
    }
}
