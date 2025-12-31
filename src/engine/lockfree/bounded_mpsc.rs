use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::engine::cache::CACHE_PAD_BYTES;

#[repr(C)]
struct RingSlot<T> {
    seq: AtomicUsize,
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send> Send for RingSlot<T> {}
unsafe impl<T: Send> Sync for RingSlot<T> {}

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
    enqueue_pos: AtomicUsize,
    _pad_enqueue: [u8; CACHE_PAD_BYTES],
    dequeue_pos: AtomicUsize,
    _pad_dequeue: [u8; CACHE_PAD_BYTES],
    mask: usize,
    capacity: usize,
    slots: Box<[RingSlot<T>]>,
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

        let mut slots = Vec::with_capacity(capacity);
        for i in 0..capacity {
            slots.push(RingSlot {
                seq: AtomicUsize::new(i),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }

        Self {
            enqueue_pos: AtomicUsize::new(0),
            _pad_enqueue: [0; CACHE_PAD_BYTES],
            dequeue_pos: AtomicUsize::new(0),
            _pad_dequeue: [0; CACHE_PAD_BYTES],
            mask: capacity - 1,
            capacity,
            slots: slots.into_boxed_slice(),
        }
    }

    /// ### English
    /// Tries to enqueue one item.
    ///
    /// Returns `Ok(())` on success; returns `Err(value)` if the ring is full.
    ///
    /// ### 中文
    /// 尝试入队一个元素。
    ///
    /// 成功返回 `Ok(())`；若 ring 已满则返回 `Err(value)`。
    pub(crate) fn try_push(&self, value: T) -> Result<(), T> {
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
                            (*slot.value.get()).write(value);
                        }
                        slot.seq.store(pos.wrapping_add(1), Ordering::Release);
                        return Ok(());
                    }
                    Err(updated) => pos = updated,
                }
            } else if dif < 0 {
                return Err(value);
            } else {
                pos = self.enqueue_pos.load(Ordering::Relaxed);
            }
        }
    }

    /// ### English
    /// Pops one queued item (single consumer).
    ///
    /// ### 中文
    /// pop 一个已入队元素（单消费者）。
    pub(crate) fn pop(&self) -> Option<T> {
        let pos = self.dequeue_pos.load(Ordering::Relaxed);
        let slot = &self.slots[pos & self.mask];
        let seq = slot.seq.load(Ordering::Acquire);
        let dif = seq as isize - pos.wrapping_add(1) as isize;

        if dif != 0 {
            return None;
        }

        self.dequeue_pos
            .store(pos.wrapping_add(1), Ordering::Relaxed);

        let value = unsafe { (*slot.value.get()).assume_init_read() };
        slot.seq
            .store(pos.wrapping_add(self.capacity), Ordering::Release);
        Some(value)
    }
}

impl<T> Drop for BoundedMpscQueue<T> {
    fn drop(&mut self) {
        while let Some(value) = self.pop() {
            drop(value);
        }
    }
}
