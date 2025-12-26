//! ### English
//! Lock-free triple-buffer frame state shared between the Servo thread (producer) and Java thread
//! (consumer). Uses atomics to avoid OS locks on the hot path.
//!
//! ### 中文
//! Servo 线程（生产者）与 Java 线程（消费者）共享的无锁三缓冲帧状态。
//! 热路径使用原子操作避免系统锁。

use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};

use dpi::PhysicalSize;

/// ### English
/// Fixed triple-buffer slot count (always 3 for maximum performance / simplicity).
///
/// ### 中文
/// 固定三缓冲槽位数量（始终为 3，以最大化性能并简化分支）。
pub const TRIPLE_BUFFER_COUNT: usize = 3;
const SLOT_INDEX_BITS: u64 = 2;

pub(crate) const SLOT_FREE: u8 = 0;
pub(crate) const SLOT_READY: u8 = 1;
pub(crate) const SLOT_HELD: u8 = 2;
pub(crate) const SLOT_RELEASE_PENDING: u8 = 3;
pub(crate) const SLOT_RENDERING: u8 = 4;

/// ### English
/// Metadata for one acquired frame (consumer side / Java thread).
///
/// ### 中文
/// 单个已获取帧的元数据（消费者侧 / Java 线程）。
#[derive(Clone, Copy, Debug)]
pub(crate) struct AcquiredFrame {
    /// ### English
    /// Triple-buffer slot index.
    ///
    /// ### 中文
    /// 三缓冲槽位索引。
    pub slot: usize,
    /// ### English
    /// GL texture ID containing the frame.
    ///
    /// ### 中文
    /// 包含该帧的 GL 纹理 ID。
    pub texture_id: u32,
    /// ### English
    /// Producer fence handle (`GLsync` cast to `u64`), or 0 if unavailable.
    ///
    /// ### 中文
    /// 生产者 fence 句柄（`GLsync` 转为 `u64`），不可用则为 0。
    pub producer_fence: u64,
    /// ### English
    /// Frame width in pixels.
    ///
    /// ### 中文
    /// 帧宽度（像素）。
    pub width: u32,
    /// ### English
    /// Frame height in pixels.
    ///
    /// ### 中文
    /// 帧高度（像素）。
    pub height: u32,
}

#[repr(C, align(64))]
struct SlotAtomics {
    /// ### English
    /// Slot state (`SLOT_*`).
    ///
    /// ### 中文
    /// 槽位状态（`SLOT_*`）。
    state: AtomicU8,
    /// ### English
    /// GL texture ID backing this slot.
    ///
    /// ### 中文
    /// 该槽位对应的 GL 纹理 ID。
    texture_id: AtomicU32,
    /// ### English
    /// Producer fence (`GLsync` cast to `u64`) inserted after rendering into this slot.
    ///
    /// ### 中文
    /// 生产者 fence（`GLsync` 转为 `u64`）：渲染写入该槽位后插入。
    producer_fence: AtomicU64,
    /// ### English
    /// Consumer fence (`GLsync` cast to `u64`) inserted after sampling this slot (safe mode).
    ///
    /// ### 中文
    /// 消费者 fence（`GLsync` 转为 `u64`）：采样该槽位后插入（安全模式）。
    consumer_fence: AtomicU64,
    /// ### English
    /// Frame sequence number stored for this slot.
    ///
    /// ### 中文
    /// 该槽位对应的帧序号。
    frame_seq: AtomicU64,
    /// ### English
    /// Cached frame width (pixels) for this slot.
    ///
    /// ### 中文
    /// 该槽位缓存的帧宽度（像素）。
    width: AtomicU32,
    /// ### English
    /// Cached frame height (pixels) for this slot.
    ///
    /// ### 中文
    /// 该槽位缓存的帧高度（像素）。
    height: AtomicU32,
}

impl SlotAtomics {
    fn new(initial_size: PhysicalSize<u32>) -> Self {
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

#[inline]
fn pack_latest(frame_seq: u64, slot: usize) -> u64 {
    (frame_seq << SLOT_INDEX_BITS) | (slot as u64 & ((1u64 << SLOT_INDEX_BITS) - 1))
}

#[inline]
fn unpack_latest(packed: u64) -> (u64, usize) {
    (
        packed >> SLOT_INDEX_BITS,
        (packed & ((1u64 << SLOT_INDEX_BITS) - 1)) as usize,
    )
}

/// ### English
/// Lock-free shared state for triple-buffered frames.
///
/// ### 中文
/// 三缓冲帧的无锁共享状态。
#[repr(C)]
pub struct SharedFrameState {
    /// ### English
    /// Per-slot atomics (triple buffer).
    ///
    /// ### 中文
    /// 每个槽位的原子状态（三缓冲）。
    slots: [SlotAtomics; TRIPLE_BUFFER_COUNT],
    /// ### English
    /// Global metadata shared by all slots (latest pointer / flags).
    ///
    /// ### 中文
    /// 全局元数据（latest 指针/标记位等）。
    frame_meta: FrameMeta,
}

#[repr(C, align(64))]
struct FrameMeta {
    /// ### English
    /// Packed `(frame_seq, slot)` pointer to the latest READY frame.
    ///
    /// ### 中文
    /// 指向最新 READY 帧的 packed `(frame_seq, slot)`。
    latest_packed: AtomicU64,
    /// ### English
    /// Global resizing flag (consumer should stop acquiring when non-zero).
    ///
    /// ### 中文
    /// 全局 resizing 标记（非 0 时消费者应停止 acquire）。
    resizing: AtomicU8,
    /// ### English
    /// Active flag used to throttle rendering/input (non-zero = active).
    ///
    /// ### 中文
    /// active 标记，用于节流渲染/输入（非 0 = active）。
    active: AtomicU8,
}

impl SharedFrameState {
    /// ### English
    /// Creates a new shared frame state with all slots initialized to `initial_size`.
    ///
    /// ### 中文
    /// 创建新的共享帧状态，并将所有槽位初始化为 `initial_size`。
    pub fn new(initial_size: PhysicalSize<u32>) -> Self {
        Self {
            slots: std::array::from_fn(|_| SlotAtomics::new(initial_size)),
            frame_meta: FrameMeta {
                latest_packed: AtomicU64::new(0),
                resizing: AtomicU8::new(0),
                active: AtomicU8::new(1),
            },
        }
    }

    /// ### English
    /// Publishes a rendered slot as READY and updates the global "latest" pointer.
    ///
    /// ### 中文
    /// 将渲染完成的槽位发布为 READY，并更新全局 “latest” 指针。
    pub fn publish(&self, slot: usize, producer_fence: u64, new_frame_seq: u64) {
        let slot_state = &self.slots[slot];
        slot_state.frame_seq.store(new_frame_seq, Ordering::Relaxed);
        slot_state
            .producer_fence
            .store(producer_fence, Ordering::Relaxed);
        slot_state.state.store(SLOT_READY, Ordering::Release);
        self.frame_meta
            .latest_packed
            .store(pack_latest(new_frame_seq, slot), Ordering::Release);
    }

    /// ### English
    /// Updates the cached size for a slot (used by the producer).
    ///
    /// ### 中文
    /// 更新某个槽位的缓存尺寸（由生产者使用）。
    pub fn set_slot_size(&self, slot: usize, size: PhysicalSize<u32>) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        let slot_state = &self.slots[slot];
        slot_state.width.store(size.width, Ordering::Release);
        slot_state.height.store(size.height, Ordering::Release);
    }

    /// ### English
    /// Stores the GL texture ID for a slot.
    ///
    /// ### 中文
    /// 写入某个槽位对应的 GL 纹理 ID。
    pub fn set_texture_id(&self, slot: usize, texture_id: u32) {
        self.slots[slot]
            .texture_id
            .store(texture_id, Ordering::Release);
    }

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
        self.slots[slot].producer_fence.store(0, Ordering::Release);
    }

    /// ### English
    /// Returns the consumer fence for a slot (Acquire load).
    ///
    /// ### 中文
    /// 获取某个槽位的 consumer fence（Acquire 读取）。
    pub fn get_consumer_fence(&self, slot: usize) -> u64 {
        self.slots[slot].consumer_fence.load(Ordering::Acquire)
    }

    /// ### English
    /// Clears the consumer fence for a slot.
    ///
    /// ### 中文
    /// 清空某个槽位的 consumer fence。
    pub fn clear_consumer_fence(&self, slot: usize) {
        self.slots[slot].consumer_fence.store(0, Ordering::Release);
    }

    /// ### English
    /// Marks the whole triple buffer as "resizing" (consumer should stop acquiring).
    ///
    /// ### 中文
    /// 标记整个三缓冲处于 “resizing” 状态（消费者应停止 acquire）。
    pub fn set_resizing(&self, resizing: bool) {
        self.frame_meta
            .resizing
            .store(u8::from(resizing), Ordering::Release);
    }

    /// ### English
    /// Returns whether resizing is in progress.
    ///
    /// ### 中文
    /// 返回是否处于 resizing 状态。
    pub fn is_resizing(&self) -> bool {
        self.frame_meta.resizing.load(Ordering::Acquire) != 0
    }

    /// ### English
    /// Sets the active flag (used by the embedder to throttle/hide a view).
    ///
    /// ### 中文
    /// 设置 active 标记（宿主用来 throttle/hide view）。
    pub fn set_active(&self, active: bool) {
        self.frame_meta
            .active
            .store(u8::from(active), Ordering::Release);
    }

    /// ### English
    /// Returns whether the view is active.
    ///
    /// ### 中文
    /// 返回 view 是否 active。
    pub fn is_active(&self) -> bool {
        self.frame_meta.active.load(Ordering::Acquire) != 0
    }

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
        let (latest, front_hint) = unpack_latest(packed);
        if latest == 0 {
            return None;
        }

        self.try_acquire_ready_slot(front_hint)
    }

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

        /*
        ### English
        Fallback: acquire any READY slot, preferring the newest sequence.
        (TRIPLE_BUFFER_COUNT is fixed to 3 for maximum performance / simpler branching.)

        ### 中文
        回退路径：获取任意 READY 槽位，并优先选择 frame_seq 最新的那个。
        （TRIPLE_BUFFER_COUNT 固定为 3，以最大化性能并简化分支。）
        */
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

    /// ### English
    /// Releases a previously acquired slot, optionally recording a consumer fence.
    ///
    /// ### 中文
    /// 释放之前 acquire 的槽位，并可选记录 consumer fence。
    pub fn release_slot(&self, slot: usize, consumer_fence: u64) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        if consumer_fence == 0 {
            if self.slots[slot]
                .state
                .compare_exchange(SLOT_HELD, SLOT_FREE, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                self.clear_consumer_fence(slot);
            }
            return;
        }

        self.slots[slot]
            .consumer_fence
            .store(consumer_fence, Ordering::Relaxed);
        let _ = self.slots[slot].state.compare_exchange(
            SLOT_HELD,
            SLOT_RELEASE_PENDING,
            Ordering::Release,
            Ordering::Relaxed,
        );
    }
}
