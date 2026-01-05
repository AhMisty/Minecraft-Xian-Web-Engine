//! ### English
//! Sequence-based bounded ring primitives shared by bounded MPSC queues.
//!
//! This module provides:
//! - A reusable ring slot type (`BoundedRingSlot<T>`) with a sequence number + payload storage.
//! - Hot-path push/pop helpers for the bounded MPSC algorithm (no allocations, single consumer).
//!
//! ### 中文
//! 供“有界 MPSC 队列”复用的、基于序号的有界 ring 原语。
//!
//! 本模块提供：
//! - 可复用的 ring 槽位类型（`BoundedRingSlot<T>`），包含序号与载荷存储。
//! - 有界 MPSC 算法的热路径 push/pop 辅助函数（零分配、单消费者）。
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(C)]
/// ### English
/// One ring-buffer slot used by the bounded sequence algorithm (sequence + payload).
///
/// ### 中文
/// 有界序号 ring 算法使用的单个槽位（序号 + 载荷）。
pub(crate) struct BoundedRingSlot<T> {
    /// ### English
    /// Slot sequence number used by the bounded ring algorithm.
    ///
    /// ### 中文
    /// 有界 ring 算法使用的槽位序号。
    seq: AtomicUsize,
    /// ### English
    /// Payload storage written by producers and read by the single consumer.
    ///
    /// ### 中文
    /// 载荷存储：由生产者写入、单消费者读取。
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send> Send for BoundedRingSlot<T> {}
unsafe impl<T: Send> Sync for BoundedRingSlot<T> {}

impl<T> BoundedRingSlot<T> {
    /// ### English
    /// Creates a ring slot with the given initial sequence number.
    ///
    /// #### Parameters
    /// - `seq`: Initial sequence number for this slot.
    ///
    /// ### 中文
    /// 创建一个 ring 槽位，并设置初始序号。
    ///
    /// #### 参数
    /// - `seq`：该槽位的初始序号。
    #[inline]
    pub(crate) fn new(seq: usize) -> Self {
        Self {
            seq: AtomicUsize::new(seq),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// ### English
    /// Loads the slot sequence number with the given ordering.
    ///
    /// #### Parameters
    /// - `ordering`: Memory ordering for the atomic load.
    ///
    /// ### 中文
    /// 按指定内存序读取槽位序号。
    ///
    /// #### 参数
    /// - `ordering`：原子读取的内存序。
    #[inline]
    pub(crate) fn load_seq(&self, ordering: Ordering) -> usize {
        self.seq.load(ordering)
    }

    /// ### English
    /// Stores the slot sequence number with the given ordering.
    ///
    /// #### Parameters
    /// - `seq`: Sequence number to store.
    /// - `ordering`: Memory ordering for the atomic store.
    ///
    /// ### 中文
    /// 按指定内存序写入槽位序号。
    ///
    /// #### 参数
    /// - `seq`：要写入的序号。
    /// - `ordering`：原子写入的内存序。
    #[inline]
    pub(crate) fn store_seq(&self, seq: usize, ordering: Ordering) {
        self.seq.store(seq, ordering);
    }

    /// ### English
    /// Writes a payload value into this slot.
    ///
    /// #### Parameters
    /// - `value`: Value to write.
    ///
    /// #### Safety
    /// Callers must ensure this slot is exclusively writable at this time (queue algorithm
    /// ownership) and that the consumer will only read after proper publication.
    ///
    /// ### 中文
    /// 向该槽位写入载荷值。
    ///
    /// #### 参数
    /// - `value`：要写入的值。
    ///
    /// #### 安全
    /// 调用方必须保证当前时刻该槽位具备独占写权限（由队列算法保证），且消费者仅会在正确发布后读取。
    #[inline]
    pub(crate) unsafe fn write_value(&self, value: T) {
        unsafe {
            (*self.value.get()).write(value);
        }
    }

    /// ### English
    /// Reads a payload value from this slot.
    ///
    /// #### Safety
    /// Callers must ensure the payload has been initialized (published) and will not be read
    /// concurrently by another consumer.
    ///
    /// ### 中文
    /// 从该槽位读取载荷值。
    ///
    /// #### 安全
    /// 调用方必须保证载荷已初始化（已发布），且不会被其它消费者并发读取。
    #[inline]
    pub(crate) unsafe fn read_value(&self) -> T {
        unsafe { (*self.value.get()).assume_init_read() }
    }
}

#[inline]
/// ### English
/// Bounded MPSC (multi-producer, single-consumer) push helper for a sequence ring.
///
/// Returns `Ok(())` on success; returns `Err(value)` if the ring is full.
///
/// #### Parameters
/// - `head`: Producer head index (shared by all producers).
/// - `slots`: Ring-buffer slots (length must be a power of two).
/// - `mask`: Index bitmask (`slots.len() - 1`).
/// - `value`: Item to push.
///
/// ### 中文
/// 有界 MPSC（多生产者、单消费者）序号 ring 的 push 辅助函数。
///
/// 成功返回 `Ok(())`；若 ring 已满则返回 `Err(value)`。
///
/// #### 参数
/// - `head`：生产者 head 索引（所有生产者共享）。
/// - `slots`：ring 槽位数组（长度必须为 2 的幂）。
/// - `mask`：索引掩码（`slots.len() - 1`）。
/// - `value`：要 push 的元素。
pub(crate) fn bounded_mpsc_try_push<T>(
    head: &AtomicUsize,
    slots: &[BoundedRingSlot<T>],
    mask: usize,
    value: T,
) -> Result<(), T> {
    debug_assert!(slots.len().is_power_of_two());
    debug_assert_eq!(mask, slots.len() - 1);

    let mut pos = head.load(Ordering::Relaxed);
    loop {
        let slot = &slots[pos & mask];
        let seq = slot.load_seq(Ordering::Acquire);
        let diff = seq.wrapping_sub(pos) as isize;

        if diff == 0 {
            match head.compare_exchange_weak(
                pos,
                pos.wrapping_add(1),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    unsafe {
                        slot.write_value(value);
                    }
                    slot.store_seq(pos.wrapping_add(1), Ordering::Release);
                    return Ok(());
                }
                Err(updated) => pos = updated,
            }
        } else if diff < 0 {
            return Err(value);
        } else {
            pos = head.load(Ordering::Relaxed);
        }
    }
}

#[inline]
/// ### English
/// Bounded MPSC (multi-producer, single-consumer) pop helper for a sequence ring.
///
/// #### Parameters
/// - `tail`: Consumer tail index (single consumer only).
/// - `slots`: Ring-buffer slots (length must equal `capacity`).
/// - `mask`: Index bitmask (`capacity - 1`).
/// - `capacity`: Ring capacity (power of two).
///
/// ### 中文
/// 有界 MPSC（多生产者、单消费者）序号 ring 的 pop 辅助函数。
///
/// #### 参数
/// - `tail`：消费者 tail 索引（仅允许单消费者）。
/// - `slots`：ring 槽位数组（长度必须等于 `capacity`）。
/// - `mask`：索引掩码（`capacity - 1`）。
/// - `capacity`：ring 容量（2 的幂）。
pub(crate) fn bounded_mpsc_pop<T>(
    tail: &AtomicUsize,
    slots: &[BoundedRingSlot<T>],
    mask: usize,
    capacity: usize,
) -> Option<T> {
    debug_assert!(capacity.is_power_of_two());
    debug_assert_eq!(slots.len(), capacity);
    debug_assert_eq!(mask, capacity - 1);

    let pos = tail.load(Ordering::Relaxed);
    let slot = &slots[pos & mask];
    let seq = slot.load_seq(Ordering::Acquire);
    let diff = seq.wrapping_sub(pos.wrapping_add(1)) as isize;

    if diff != 0 {
        return None;
    }

    tail.store(pos.wrapping_add(1), Ordering::Relaxed);

    let value = unsafe { slot.read_value() };
    slot.store_seq(pos.wrapping_add(capacity), Ordering::Release);
    Some(value)
}
