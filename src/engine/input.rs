//! ### English
//! Lock-free input queues and coalescing helpers.
//! Designed for minimal overhead between the Java thread(s) and the Servo thread.
//!
//! ### 中文
//! 无锁输入队列与合并（coalescing）工具。
//! 旨在让 Java 线程与 Servo 线程之间的开销最小化。

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};

use crate::engine::input_types::XianWebEngineInputEvent;

#[repr(C, align(64))]
/// ### English
/// Coalesced mouse-move state: keeps only the latest `(x, y)` until the Servo thread drains it.
///
/// ### 中文
/// 鼠标移动合并状态：只保留最新的 `(x, y)`，等待 Servo 线程 drain。
pub struct CoalescedMouseMove {
    /// ### English
    /// Pending flag (`0` = no pending move, `1` = pending).
    ///
    /// ### 中文
    /// pending 标记（`0` = 无待处理移动，`1` = 有待处理移动）。
    pending: AtomicU8,
    /// ### English
    /// Padding to keep `packed_pos` on a separate cache line from unrelated atomics.
    ///
    /// ### 中文
    /// 填充：让 `packed_pos` 与其它原子尽量避免同一缓存行（降低伪共享）。
    _padding: [u8; 7],
    /// ### English
    /// Packed `(x, y)` mouse position as two `f32` bit patterns.
    ///
    /// ### 中文
    /// 将 `(x, y)` 鼠标位置以两个 `f32` 的 bit pattern 打包到一个 `u64` 中。
    packed_pos: AtomicU64,
}

impl Default for CoalescedMouseMove {
    fn default() -> Self {
        Self {
            pending: AtomicU8::new(0),
            _padding: [0; 7],
            packed_pos: AtomicU64::new(0),
        }
    }
}

impl CoalescedMouseMove {
    /// ### English
    /// Stores the latest mouse position and marks it pending.
    /// Returns `true` if this call transitions from "not pending" to "pending".
    ///
    /// ### 中文
    /// 写入最新鼠标位置并标记为 pending。
    /// 若本次调用把状态从“非 pending”切换为“pending”，则返回 `true`。
    pub fn set(&self, x: f32, y: f32) -> bool {
        self.packed_pos.store(pack_f32x2(x, y), Ordering::Relaxed);
        self.pending.swap(1, Ordering::Release) == 0
    }

    /// ### English
    /// Takes the latest mouse position if pending.
    ///
    /// ### 中文
    /// 若处于 pending，则取出最新鼠标位置。
    pub fn take(&self) -> Option<(f32, f32)> {
        if self.pending.swap(0, Ordering::Acquire) == 0 {
            return None;
        }
        let packed = self.packed_pos.load(Ordering::Relaxed);
        Some(unpack_f32x2(packed))
    }
}

#[inline]
fn pack_f32x2(x: f32, y: f32) -> u64 {
    (x.to_bits() as u64) | ((y.to_bits() as u64) << 32)
}

#[inline]
fn unpack_f32x2(packed: u64) -> (f32, f32) {
    let x = (packed & 0xFFFF_FFFF) as u32;
    let y = (packed >> 32) as u32;
    (f32::from_bits(x), f32::from_bits(y))
}

const INPUT_QUEUE_CAPACITY: usize = 256;
const INPUT_QUEUE_MASK: usize = INPUT_QUEUE_CAPACITY - 1;
const CACHE_LINE_BYTES: usize = 64;
const ATOMIC_USIZE_BYTES: usize = std::mem::size_of::<AtomicUsize>();
const USIZE_BYTES: usize = std::mem::size_of::<usize>();
const CACHE_PAD_BYTES: usize = CACHE_LINE_BYTES - ATOMIC_USIZE_BYTES - USIZE_BYTES;

#[repr(C)]
struct InputQueueSlot {
    /// ### English
    /// Sequence number used by the lock-free bounded queue algorithm.
    ///
    /// ### 中文
    /// 无锁有界队列算法使用的序号。
    seq: AtomicUsize,
    /// ### English
    /// Stored event payload (written by producer, read by the single consumer).
    ///
    /// ### 中文
    /// 事件载荷（由生产者写入，单消费者读取）。
    value: UnsafeCell<MaybeUninit<XianWebEngineInputEvent>>,
}

unsafe impl Send for InputQueueSlot {}
unsafe impl Sync for InputQueueSlot {}

#[repr(C, align(64))]
/// ### English
/// Bounded lock-free input queue.
/// Supports multi-producer mode and an optimized single-producer (SPSC) ring-buffer mode.
///
/// ### 中文
/// 有界无锁输入队列。
/// 支持多生产者模式，以及优化的单生产者（SPSC）ring-buffer 模式。
pub struct InputEventQueue {
    /// ### English
    /// Producer head index (enqueue position).
    ///
    /// ### 中文
    /// 生产者 head（入队位置）。
    enqueue_pos: AtomicUsize,
    /// ### English
    /// Cached consumer tail index used by the SPSC producer to avoid frequent atomic loads.
    ///
    /// ### 中文
    /// SPSC 生产者缓存的 consumer tail，用于减少原子读取次数。
    producer_cached_dequeue: UnsafeCell<usize>,
    _pad_enqueue: [u8; CACHE_PAD_BYTES],
    /// ### English
    /// Consumer tail index (dequeue position).
    ///
    /// ### 中文
    /// 消费者 tail（出队位置）。
    dequeue_pos: AtomicUsize,
    /// ### English
    /// Cached producer head index used by the SPSC consumer to avoid frequent atomic loads.
    ///
    /// ### 中文
    /// SPSC 消费者缓存的 producer head，用于减少原子读取次数。
    consumer_cached_enqueue: UnsafeCell<usize>,
    _pad_dequeue: [u8; CACHE_PAD_BYTES],
    /// ### English
    /// Whether to use the optimized single-producer (SPSC) algorithm.
    ///
    /// ### 中文
    /// 是否启用优化的单生产者（SPSC）算法。
    single_producer: bool,
    /// ### English
    /// Coalesced "pending input" flag (`0`/`1`) used to reduce wake/notify overhead.
    ///
    /// ### 中文
    /// 合并后的 “输入待处理” 标记（`0`/`1`），用于减少 wake/notify 开销。
    pending: AtomicU8,
    _padding: [u8; 7],
    /// ### English
    /// Fixed-capacity ring buffer storage.
    ///
    /// ### 中文
    /// 固定容量 ring buffer 存储区。
    slots: [InputQueueSlot; INPUT_QUEUE_CAPACITY],
}

unsafe impl Send for InputEventQueue {}
unsafe impl Sync for InputEventQueue {}

impl InputEventQueue {
    /// ### English
    /// Creates a new queue with a fixed capacity.
    ///
    /// ### 中文
    /// 创建一个固定容量的新队列。
    pub fn new(single_producer: bool) -> Self {
        debug_assert!(INPUT_QUEUE_CAPACITY.is_power_of_two());
        Self {
            enqueue_pos: AtomicUsize::new(0),
            producer_cached_dequeue: UnsafeCell::new(0),
            _pad_enqueue: [0; CACHE_PAD_BYTES],
            dequeue_pos: AtomicUsize::new(0),
            consumer_cached_enqueue: UnsafeCell::new(0),
            _pad_dequeue: [0; CACHE_PAD_BYTES],
            single_producer,
            pending: AtomicU8::new(0),
            _padding: [0; 7],
            slots: std::array::from_fn(|i| InputQueueSlot {
                seq: AtomicUsize::new(i),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            }),
        }
    }

    #[inline]
    /// ### English
    /// Marks the queue as having pending items (coalesced notification).
    ///
    /// ### 中文
    /// 标记队列存在待处理项（用于合并通知）。
    pub fn mark_pending(&self) -> bool {
        self.pending.swap(1, Ordering::Relaxed) == 0
    }

    #[inline]
    /// ### English
    /// Clears the pending mark.
    ///
    /// ### 中文
    /// 清除 pending 标记。
    pub fn clear_pending(&self) {
        self.pending.store(0, Ordering::Relaxed);
    }

    #[inline]
    /// ### English
    /// Returns whether the queue is marked pending (coalesced notification flag).
    ///
    /// ### 中文
    /// 返回队列是否处于 pending 标记（合并通知标记）。
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed) != 0
    }

    /// ### English
    /// Tries to push one event (MPMC-safe); returns `false` if the queue is full.
    ///
    /// ### 中文
    /// 尝试 push 一个事件（支持多生产者）；若队列已满返回 `false`。
    pub fn try_push(&self, event: XianWebEngineInputEvent) -> bool {
        if self.single_producer {
            return self.try_push_spsc(event);
        }

        let mut pos = self.enqueue_pos.load(Ordering::Relaxed);
        loop {
            let slot = &self.slots[pos & INPUT_QUEUE_MASK];
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
                            (*slot.value.get()).write(event);
                        }
                        slot.seq.store(pos.wrapping_add(1), Ordering::Release);
                        return true;
                    }
                    Err(updated) => pos = updated,
                }
            } else if dif < 0 {
                return false;
            } else {
                pos = self.enqueue_pos.load(Ordering::Relaxed);
            }
        }
    }

    #[inline]
    /// ### English
    /// Single-producer fast path. Only use when the embedder guarantees one producer thread.
    ///
    /// ### 中文
    /// 单生产者快路径。仅在宿主保证只有一个生产者线程时使用。
    pub fn try_push_single_producer(&self, event: XianWebEngineInputEvent) -> bool {
        if self.single_producer {
            return self.try_push_spsc(event);
        }

        let pos = self.enqueue_pos.load(Ordering::Relaxed);
        let slot = &self.slots[pos & INPUT_QUEUE_MASK];
        let seq = slot.seq.load(Ordering::Acquire);
        let dif = seq as isize - pos as isize;

        if dif != 0 {
            return false;
        }

        unsafe {
            (*slot.value.get()).write(event);
        }
        slot.seq.store(pos.wrapping_add(1), Ordering::Release);
        self.enqueue_pos
            .store(pos.wrapping_add(1), Ordering::Relaxed);
        true
    }

    /// ### English
    /// Pops one event (single-consumer; Servo thread).
    ///
    /// ### 中文
    /// pop 一个事件（单消费者；Servo 线程）。
    pub fn pop(&self) -> Option<XianWebEngineInputEvent> {
        if self.single_producer {
            return self.pop_spsc();
        }

        /*
        ### English
        Single-consumer fast path: the Servo thread is the only consumer.

        ### 中文
        单消费者快路径：Servo 线程是唯一的消费者。
        */
        let pos = self.dequeue_pos.load(Ordering::Relaxed);
        let slot = &self.slots[pos & INPUT_QUEUE_MASK];
        let seq = slot.seq.load(Ordering::Acquire);
        let dif = seq as isize - pos.wrapping_add(1) as isize;

        if dif != 0 {
            return None;
        }

        self.dequeue_pos
            .store(pos.wrapping_add(1), Ordering::Relaxed);

        let event = unsafe { (*slot.value.get()).assume_init_read() };
        slot.seq
            .store(pos.wrapping_add(INPUT_QUEUE_CAPACITY), Ordering::Release);
        Some(event)
    }

    #[inline]
    fn try_push_spsc(&self, event: XianWebEngineInputEvent) -> bool {
        let head = self.enqueue_pos.load(Ordering::Relaxed);
        let cached_tail = unsafe { *self.producer_cached_dequeue.get() };
        if head.wrapping_sub(cached_tail) >= INPUT_QUEUE_CAPACITY {
            let tail = self.dequeue_pos.load(Ordering::Acquire);
            unsafe {
                *self.producer_cached_dequeue.get() = tail;
            }
            if head.wrapping_sub(tail) >= INPUT_QUEUE_CAPACITY {
                return false;
            }
        }

        let slot = &self.slots[head & INPUT_QUEUE_MASK];
        unsafe {
            (*slot.value.get()).write(event);
        }
        self.enqueue_pos
            .store(head.wrapping_add(1), Ordering::Release);
        true
    }

    #[inline]
    fn pop_spsc(&self) -> Option<XianWebEngineInputEvent> {
        let tail = self.dequeue_pos.load(Ordering::Relaxed);
        let cached_head = unsafe { *self.consumer_cached_enqueue.get() };
        if tail == cached_head {
            let head = self.enqueue_pos.load(Ordering::Acquire);
            unsafe {
                *self.consumer_cached_enqueue.get() = head;
            }
            if tail == head {
                return None;
            }
        }

        let slot = &self.slots[tail & INPUT_QUEUE_MASK];
        let event = unsafe { (*slot.value.get()).assume_init_read() };
        self.dequeue_pos
            .store(tail.wrapping_add(1), Ordering::Release);
        Some(event)
    }
}
