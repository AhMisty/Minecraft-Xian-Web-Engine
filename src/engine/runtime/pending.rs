//! ### English
//! Lock-free `u32` ID queue used to signal pending work to the dedicated Servo thread.
//! This replaces sending tiny "pending" commands over a channel to avoid allocation and reduce
//! cross-thread overhead in hot paths.
//!
//! The queue is multi-producer (embedder threads) / single-consumer (Servo thread).
//! On overflow we set a flag so the consumer can fall back to a slow-path scan.
//!
//! ### 中文
//! 用于向独立 Servo 线程信号化“有待处理工作”的无锁 `u32` ID 队列。
//! 这用于替代通过 channel 发送很小的 “pending” 命令，以避免分配并降低热路径的跨线程开销。
//!
//! 该队列为多生产者（宿主线程）/单消费者（Servo 线程）。
//! 若队列溢出会设置标记，消费者可走慢路径扫描兜底。

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const CACHE_LINE_BYTES: usize = 64;
const ATOMIC_USIZE_BYTES: usize = std::mem::size_of::<AtomicUsize>();
const CACHE_PAD_BYTES: usize = CACHE_LINE_BYTES - ATOMIC_USIZE_BYTES;

#[repr(C)]
struct Slot {
    /// ### English
    /// Sequence number used by the lock-free bounded queue algorithm.
    ///
    /// ### 中文
    /// 无锁有界队列算法使用的序号。
    seq: AtomicUsize,
    /// ### English
    /// Stored ID payload (written by producer, read by the single consumer).
    ///
    /// ### 中文
    /// ID 载荷（由生产者写入，单消费者读取）。
    value: UnsafeCell<MaybeUninit<u32>>,
}

unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

#[repr(C, align(64))]
pub(super) struct PendingIdQueue {
    /// ### English
    /// Producer head index (enqueue position).
    ///
    /// ### 中文
    /// 生产者 head（入队位置）。
    enqueue_pos: AtomicUsize,
    _pad_enqueue: [u8; CACHE_PAD_BYTES],
    /// ### English
    /// Consumer tail index (dequeue position).
    ///
    /// ### 中文
    /// 消费者 tail（出队位置）。
    dequeue_pos: AtomicUsize,
    _pad_dequeue: [u8; CACHE_PAD_BYTES],

    /// ### English
    /// Overflow flag set by producers when the ring is full (consumer falls back to scan).
    ///
    /// ### 中文
    /// 溢出标记：当 ring 满时由生产者置位（消费者会回退到扫描兜底）。
    overflowed: AtomicBool,

    /// ### English
    /// Bitmask for indexing into `slots` (capacity is a power of two).
    ///
    /// ### 中文
    /// 用于索引 `slots` 的掩码（capacity 为 2 的幂）。
    mask: usize,
    /// ### English
    /// Ring capacity (power of two).
    ///
    /// ### 中文
    /// ring 容量（2 的幂）。
    capacity: usize,
    /// ### English
    /// Ring storage slots.
    ///
    /// ### 中文
    /// ring 存储槽位。
    slots: Box<[Slot]>,
}

impl PendingIdQueue {
    /// ### English
    /// Creates a bounded MPSC queue with at least `capacity` slots (rounded up to power-of-two).
    ///
    /// ### 中文
    /// 创建一个有界 MPSC 队列，至少包含 `capacity` 个槽位（向上取整为 2 的幂）。
    pub(super) fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1).next_power_of_two();
        debug_assert!(capacity.is_power_of_two());

        let mut slots = Vec::with_capacity(capacity);
        for i in 0..capacity {
            slots.push(Slot {
                seq: AtomicUsize::new(i),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }

        Self {
            enqueue_pos: AtomicUsize::new(0),
            _pad_enqueue: [0; CACHE_PAD_BYTES],
            dequeue_pos: AtomicUsize::new(0),
            _pad_dequeue: [0; CACHE_PAD_BYTES],
            overflowed: AtomicBool::new(false),
            mask: capacity - 1,
            capacity,
            slots: slots.into_boxed_slice(),
        }
    }

    /// ### English
    /// Tries to enqueue an ID.
    ///
    /// Returns `true` on success; returns `false` if the ring is full (and sets the overflow flag).
    ///
    /// ### 中文
    /// 尝试入队一个 ID。
    ///
    /// 成功返回 `true`；若 ring 已满则返回 `false`（并设置溢出标记）。
    pub(super) fn push(&self, id: u32) -> bool {
        let mut pos = self.enqueue_pos.load(Ordering::Relaxed);
        loop {
            let slot = &self.slots[pos & self.mask];
            let seq = slot.seq.load(Ordering::Acquire);
            let dif = seq as isize - pos as isize;

            if dif == 0 {
                match self.enqueue_pos.compare_exchange_weak(
                    pos,
                    pos.wrapping_add(1),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        unsafe {
                            (*slot.value.get()).write(id);
                        }
                        slot.seq.store(pos.wrapping_add(1), Ordering::Release);
                        return true;
                    }
                    Err(updated) => pos = updated,
                }
            } else if dif < 0 {
                self.overflowed.store(true, Ordering::Relaxed);
                return false;
            } else {
                pos = self.enqueue_pos.load(Ordering::Relaxed);
            }
        }
    }

    /// ### English
    /// Pops one queued ID (single consumer / Servo thread).
    ///
    /// ### 中文
    /// pop 一个已入队的 ID（单消费者 / Servo 线程）。
    pub(super) fn pop(&self) -> Option<u32> {
        let pos = self.dequeue_pos.load(Ordering::Relaxed);
        let slot = &self.slots[pos & self.mask];
        let seq = slot.seq.load(Ordering::Acquire);
        let dif = seq as isize - pos.wrapping_add(1) as isize;

        if dif != 0 {
            return None;
        }

        self.dequeue_pos
            .store(pos.wrapping_add(1), Ordering::Relaxed);

        let id = unsafe { (*slot.value.get()).assume_init_read() };
        slot.seq
            .store(pos.wrapping_add(self.capacity), Ordering::Release);
        Some(id)
    }

    /// ### English
    /// Returns and clears the overflow flag.
    ///
    /// When this returns `true`, the consumer should fall back to a slow-path scan to avoid
    /// missing work.
    ///
    /// ### 中文
    /// 读取并清空溢出标记。
    ///
    /// 若返回 `true`，消费者应回退到慢路径扫描兜底，避免漏处理工作。
    pub(super) fn take_overflowed(&self) -> bool {
        self.overflowed.swap(false, Ordering::AcqRel)
    }
}
