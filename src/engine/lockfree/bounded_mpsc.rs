//! ### English
//! Bounded lock-free MPSC queue (multi-producer, single-consumer).
//!
//! ### 中文
//! 有界无锁 MPSC 队列（多生产者、单消费者）。

use std::sync::atomic::AtomicUsize;

use crate::engine::cache::pad_after;

use super::{BoundedRingSlot, bounded_mpsc_pop, bounded_mpsc_try_push};

const PAD_ATOMIC_USIZE: usize = pad_after::<AtomicUsize>();

/// ### English
/// Bounded lock-free MPSC queue (multi-producer, single-consumer).
///
/// - FIFO (bounded ring).
/// - Returns `Err(value)` when full (caller decides backpressure policy).
///
/// ### 中文
/// 有界无锁 MPSC 队列（多生产者、单消费者）。
///
/// - FIFO（有界 ring）。
/// - 满时返回 `Err(value)`，由调用方决定背压策略。
#[repr(C, align(64))]
pub(crate) struct BoundedMpscQueue<T> {
    /// ### English
    /// Producer head index (push position).
    ///
    /// ### 中文
    /// 生产者 head（push 位置）。
    head: AtomicUsize,
    /// ### English
    /// Padding to keep producer and consumer indices on different cache lines.
    ///
    /// ### 中文
    /// 填充：让生产者/消费者索引尽量不共用 cache line（降低伪共享）。
    _pad_head: [u8; PAD_ATOMIC_USIZE],
    /// ### English
    /// Consumer tail index (pop position).
    ///
    /// ### 中文
    /// 消费者 tail（pop 位置）。
    tail: AtomicUsize,
    /// ### English
    /// Padding to keep producer and consumer indices on different cache lines.
    ///
    /// ### 中文
    /// 填充：让生产者/消费者索引尽量不共用 cache line（降低伪共享）。
    _pad_tail: [u8; PAD_ATOMIC_USIZE],
    /// ### English
    /// Bitmask for indexing into `slots` (capacity is a power of two).
    ///
    /// ### 中文
    /// 用于索引 `slots` 的掩码（capacity 为 2 的幂）。
    mask: usize,
    /// ### English
    /// Ring-buffer storage.
    ///
    /// ### 中文
    /// ring buffer 存储区。
    slots: Box<[BoundedRingSlot<T>]>,
}

unsafe impl<T: Send> Send for BoundedMpscQueue<T> {}
unsafe impl<T: Send> Sync for BoundedMpscQueue<T> {}

impl<T> BoundedMpscQueue<T> {
    /// ### English
    /// Creates a bounded MPSC queue with at least `capacity` slots (rounded up to power-of-two).
    ///
    /// ### 中文
    /// 创建一个至少包含 `capacity` 个槽位的有界 MPSC 队列（向上取整为 2 的幂）。
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1).next_power_of_two();
        debug_assert!(capacity.is_power_of_two());

        let slots = (0..capacity)
            .map(BoundedRingSlot::new)
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            head: AtomicUsize::new(0),
            _pad_head: [0; PAD_ATOMIC_USIZE],
            tail: AtomicUsize::new(0),
            _pad_tail: [0; PAD_ATOMIC_USIZE],
            mask: capacity - 1,
            slots,
        }
    }

    /// ### English
    /// Tries to push one item.
    ///
    /// #### Parameters
    /// - `value`: Item to push.
    ///
    /// Returns `Ok(())` on success; returns `Err(value)` if the ring is full.
    ///
    /// ### 中文
    /// 尝试 push 一个元素。
    ///
    /// #### 参数
    /// - `value`：要 push 的元素。
    ///
    /// 成功返回 `Ok(())`；若 ring 已满则返回 `Err(value)`。
    pub(crate) fn try_push(&self, value: T) -> Result<(), T> {
        bounded_mpsc_try_push(&self.head, &self.slots, self.mask, value)
    }

    /// ### English
    /// Pops one queued item (single consumer).
    ///
    /// ### 中文
    /// pop 一个元素（单消费者）。
    pub(crate) fn pop(&self) -> Option<T> {
        bounded_mpsc_pop(
            &self.tail,
            &self.slots,
            self.mask,
            self.mask.wrapping_add(1),
        )
    }
}

impl<T> Drop for BoundedMpscQueue<T> {
    /// ### English
    /// Drains remaining queued items on drop.
    ///
    /// ### 中文
    /// drop 时 drain 队列中仍未消费的元素。
    fn drop(&mut self) {
        while let Some(value) = self.pop() {
            drop(value);
        }
    }
}
