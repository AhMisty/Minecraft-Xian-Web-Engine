use std::sync::atomic::{AtomicU8, AtomicU64};

use dpi::PhysicalSize;

use crate::engine::cache::CACHE_LINE_BYTES;

use super::TRIPLE_BUFFER_COUNT;
use super::slot::SlotAtomics;

const CACHE_PAD_U64_BYTES: usize = CACHE_LINE_BYTES - std::mem::size_of::<AtomicU64>();
const FRAME_FLAGS_PAD_BYTES: usize = CACHE_LINE_BYTES - 2 * std::mem::size_of::<AtomicU8>();

const SLOT_INDEX_BITS: u64 = 2;

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
    /// Padding to keep `flags` on a separate cache line from `latest_packed` (reduces false sharing).
    ///
    /// ### 中文
    /// 填充：让 `flags` 与 `latest_packed` 尽量处于不同缓存行（降低伪共享）。
    _pad_latest: [u8; CACHE_PAD_U64_BYTES],
    flags: FrameFlags,
}

#[repr(C, align(64))]
struct FrameFlags {
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
    _padding: [u8; FRAME_FLAGS_PAD_BYTES],
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
                _pad_latest: [0; CACHE_PAD_U64_BYTES],
                flags: FrameFlags {
                    resizing: AtomicU8::new(0),
                    active: AtomicU8::new(1),
                    _padding: [0; FRAME_FLAGS_PAD_BYTES],
                },
            },
        }
    }
}

mod acquire;
mod fences;
mod flags;
mod publish;
mod release;
mod state;
