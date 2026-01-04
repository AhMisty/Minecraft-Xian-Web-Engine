//! ### English
//! Lock-free shared state for triple-buffered frames.
//!
//! Shared between the Servo thread (producer) and the Java thread (consumer).
//! Includes the packed "latest READY slot" pointer and global flags.
//!
//! ### 中文
//! 三缓冲帧的无锁共享状态。
//!
//! 由 Servo 线程（生产者）与 Java 线程（消费者）共享。
//! 包含打包后的“latest READY 槽位”指针与全局标记位。

use std::sync::atomic::{AtomicU8, AtomicU64};

use dpi::PhysicalSize;

use crate::engine::cache::{pad_after, pad_after2};

use super::TRIPLE_BUFFER_COUNT;
use super::slot::SlotAtomics;

const CACHE_PAD_U64_BYTES: usize = pad_after::<AtomicU64>();
const FRAME_FLAGS_PAD_BYTES: usize = pad_after2::<AtomicU8, AtomicU8>();

const SLOT_INDEX_BITS: u64 = 2;

#[inline]
/// ### English
/// Packs `(frame_seq, slot)` into a single `u64` used for the global "latest" pointer.
///
/// #### Parameters
/// - `frame_seq`: Frame sequence number.
/// - `slot`: Triple-buffer slot index.
///
/// ### 中文
/// 将 `(frame_seq, slot)` 打包到一个 `u64` 中，用于全局 “latest” 指针。
///
/// #### 参数
/// - `frame_seq`：帧序号。
/// - `slot`：三缓冲槽位索引。
fn pack_latest(frame_seq: u64, slot: usize) -> u64 {
    (frame_seq << SLOT_INDEX_BITS) | (slot as u64 & ((1u64 << SLOT_INDEX_BITS) - 1))
}

#[inline]
/// ### English
/// Unpacks a `u64` produced by `pack_latest` into `(frame_seq, slot)`.
///
/// #### Parameters
/// - `packed`: Value returned by `pack_latest`.
///
/// ### 中文
/// 将 `pack_latest` 产生的 `u64` 解包为 `(frame_seq, slot)`。
///
/// #### 参数
/// - `packed`：由 `pack_latest` 产生的值。
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
/// ### English
/// Cache-line separated global metadata shared by all slots.
///
/// ### 中文
/// 与槽位分离、按 cache line 隔离的全局元数据。
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
    /// ### English
    /// Global flags shared by all slots.
    ///
    /// ### 中文
    /// 由所有槽位共享的全局标记位。
    flags: FrameFlags,
}

#[repr(C, align(64))]
/// ### English
/// Cache-line separated global flags shared by all slots.
///
/// ### 中文
/// 与槽位分离、按 cache line 隔离的全局标记位。
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
    /// ### English
    /// Padding for cache-line separation.
    ///
    /// ### 中文
    /// cache line 隔离填充。
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
