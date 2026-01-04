//! ### English
//! Lock-free bounded queue for input events.
//!
//! Supports a fast single-producer path (SPSC) and a multi-producer path (MPSC).
//!
//! ### 中文
//! 输入事件的无锁有界队列。
//!
//! 支持快速的单生产者路径（SPSC）与多生产者路径（MPSC）。

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use crate::engine::cache::pad_after2;
use crate::engine::input_types::XianWebEngineInputEvent;

/// ### English
/// Lock-free bounded queue for input events.
///
/// ### 中文
/// 输入事件的无锁有界队列。
const INPUT_QUEUE_CAPACITY: usize = 256;
const INPUT_QUEUE_MASK: usize = INPUT_QUEUE_CAPACITY - 1;
const PAD_INDEX_BYTES: usize = pad_after2::<AtomicUsize, UnsafeCell<usize>>();

#[repr(C)]
/// ### English
/// One ring-buffer slot for the bounded input queue (sequence + payload).
///
/// ### 中文
/// 有界输入队列的单个 ring 槽位（序号 + 载荷）。
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
    /// Producer head index (push position).
    ///
    /// ### 中文
    /// 生产者 head（push 位置）。
    head: AtomicUsize,
    /// ### English
    /// Cached consumer tail index used by the SPSC producer to avoid frequent atomic loads.
    ///
    /// ### 中文
    /// SPSC 生产者缓存的 consumer tail，用于减少原子读取次数。
    producer_cached_tail: UnsafeCell<usize>,
    /// ### English
    /// Padding to keep producer and consumer indices on different cache lines.
    ///
    /// ### 中文
    /// 填充：让生产者/消费者索引尽量不共用 cache line（降低伪共享）。
    _pad_head: [u8; PAD_INDEX_BYTES],
    /// ### English
    /// Consumer tail index (pop position).
    ///
    /// ### 中文
    /// 消费者 tail（pop 位置）。
    tail: AtomicUsize,
    /// ### English
    /// Cached producer head index used by the SPSC consumer to avoid frequent atomic loads.
    ///
    /// ### 中文
    /// SPSC 消费者缓存的 producer head，用于减少原子读取次数。
    consumer_cached_head: UnsafeCell<usize>,
    /// ### English
    /// Padding to keep producer and consumer indices on different cache lines.
    ///
    /// ### 中文
    /// 填充：让生产者/消费者索引尽量不共用 cache line（降低伪共享）。
    _pad_tail: [u8; PAD_INDEX_BYTES],
    /// ### English
    /// Whether to use the optimized single-producer (SPSC) algorithm.
    /// This must only be enabled when all pushes come from a single producer thread.
    /// If violated (multiple producers), behavior is undefined.
    ///
    /// ### 中文
    /// 是否启用优化的单生产者（SPSC）算法。
    /// 仅当所有 push 都来自同一个生产者线程时才可启用。
    /// 若被违反（存在多个生产者），则行为未定义。
    single_producer: bool,
    /// ### English
    /// Coalesced "pending input" flag (`0`/`1`) used to reduce wake/notify overhead.
    ///
    /// ### 中文
    /// 合并后的 “输入待处理” 标记（`0`/`1`），用于减少 wake/notify 开销。
    pending: AtomicU8,
    /// ### English
    /// Padding for cache-line alignment.
    ///
    /// ### 中文
    /// cache line 对齐填充。
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
    /// #### Parameters
    /// - `single_producer`: Whether the queue will be used by a single producer (enables the SPSC fast path; must be `false` for multi-producer).
    ///
    /// ### 中文
    /// 创建一个固定容量的新队列。
    ///
    /// #### 参数
    /// - `single_producer`：是否为单生产者使用（启用 SPSC 快路径；多生产者时必须为 `false`）。
    pub fn new(single_producer: bool) -> Self {
        debug_assert!(INPUT_QUEUE_CAPACITY.is_power_of_two());
        Self {
            head: AtomicUsize::new(0),
            producer_cached_tail: UnsafeCell::new(0),
            _pad_head: [0; PAD_INDEX_BYTES],
            tail: AtomicUsize::new(0),
            consumer_cached_head: UnsafeCell::new(0),
            _pad_tail: [0; PAD_INDEX_BYTES],
            single_producer,
            pending: AtomicU8::new(0),
            _padding: [0; 7],
            slots: std::array::from_fn(|i| InputQueueSlot {
                seq: AtomicUsize::new(i),
                value: UnsafeCell::new(MaybeUninit::uninit()),
            }),
        }
    }

    /// ### English
    /// Marks "input pending" and returns `true` iff this call transitions from `0` to `1`.
    ///
    /// ### 中文
    /// 标记 “输入待处理”，并且仅在 `0 -> 1` 的首次标记时返回 `true`。
    #[inline]
    pub fn mark_pending(&self) -> bool {
        self.pending.swap(1, Ordering::Release) == 0
    }

    /// ### English
    /// Clears the pending flag (called by the consumer after draining).
    ///
    /// ### 中文
    /// 清除 pending 标记（消费者 drain 完成后调用）。
    #[inline]
    pub fn clear_pending(&self) {
        self.pending.store(0, Ordering::Release);
    }

    /// ### English
    /// Pops one queued input event (single consumer / Servo thread).
    ///
    /// ### 中文
    /// pop 一个输入事件（单消费者 / Servo 线程）。
    pub fn pop(&self) -> Option<XianWebEngineInputEvent> {
        if self.single_producer {
            return self.pop_spsc();
        }

        self.pop_mpsc()
    }

    /// ### English
    /// Tries to push a slice of events.
    ///
    /// #### Parameters
    /// - `events`: Events to push.
    ///
    /// Returns number of accepted events (may be less than `events.len()` if full).
    ///
    /// ### 中文
    /// 尝试 push 一段事件切片。
    ///
    /// #### 参数
    /// - `events`：要 push 的事件切片。
    ///
    /// 返回成功接收的数量（队列满时可能小于 `events.len()`）。
    pub fn try_push_slice(&self, events: &[XianWebEngineInputEvent]) -> usize {
        if events.is_empty() {
            return 0;
        }

        if self.single_producer {
            return self.try_push_slice_spsc(events);
        }

        let mut accepted = 0usize;
        for &event in events {
            if self.try_push_mpsc(event) {
                accepted += 1;
            } else {
                break;
            }
        }
        accepted
    }
}

mod mpsc;
mod spsc;
