use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64};

use dpi::PhysicalSize;

use super::SLOT_FREE;

#[repr(C, align(64))]
pub(super) struct SlotAtomics {
    /// ### English
    /// Slot state (`SLOT_*`).
    ///
    /// ### 中文
    /// 槽位状态（`SLOT_*`）。
    pub(super) state: AtomicU8,
    /// ### English
    /// GL texture ID backing this slot.
    ///
    /// ### 中文
    /// 该槽位对应的 GL 纹理 ID。
    pub(super) texture_id: AtomicU32,
    /// ### English
    /// Producer fence (`GLsync` cast to `u64`) inserted after rendering into this slot.
    ///
    /// ### 中文
    /// 生产者 fence（`GLsync` 转为 `u64`）：渲染写入该槽位后插入。
    pub(super) producer_fence: AtomicU64,
    /// ### English
    /// Consumer fence (`GLsync` cast to `u64`) inserted after sampling this slot (safe mode).
    ///
    /// ### 中文
    /// 消费者 fence（`GLsync` 转为 `u64`）：采样该槽位后插入（安全模式）。
    pub(super) consumer_fence: AtomicU64,
    /// ### English
    /// Frame sequence number stored for this slot.
    ///
    /// ### 中文
    /// 该槽位对应的帧序号。
    pub(super) frame_seq: AtomicU64,
    /// ### English
    /// Cached frame width (pixels) for this slot.
    ///
    /// ### 中文
    /// 该槽位缓存的帧宽度（像素）。
    pub(super) width: AtomicU32,
    /// ### English
    /// Cached frame height (pixels) for this slot.
    ///
    /// ### 中文
    /// 该槽位缓存的帧高度（像素）。
    pub(super) height: AtomicU32,
}

impl SlotAtomics {
    pub(super) fn new(initial_size: PhysicalSize<u32>) -> Self {
        Self {
            state: AtomicU8::new(SLOT_FREE),
            texture_id: AtomicU32::new(0),
            producer_fence: AtomicU64::new(0),
            consumer_fence: AtomicU64::new(0),
            frame_seq: AtomicU64::new(0),
            width: AtomicU32::new(initial_size.width),
            height: AtomicU32::new(initial_size.height),
        }
    }
}
