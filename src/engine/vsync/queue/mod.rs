//! ### English
//! Lock-free vsync callback queue implementation.
//!
//! Fast path uses a ring buffer; cold overflow falls back to an intrusive list.
//!
//! ### 中文
//! Vsync 回调队列的无锁实现。
//!
//! 热路径使用 ring buffer；溢出时回退到冷路径的侵入式链表。

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::engine::cache::{pad_after, pad_after3};

use super::VsyncCallback;
use super::overflow::{VsyncCallbackNode, drop_vsync_list, drop_vsync_raw_list};

const VSYNC_OVERFLOW_NODE_PREALLOC: usize = 1024;
const VSYNC_OVERFLOW_MAX: usize = 8192;
const VSYNC_PAD_HEAD_BYTES: usize =
    pad_after3::<AtomicUsize, AtomicUsize, UnsafeCell<*mut VsyncCallbackNode>>();
const VSYNC_PAD_TAIL_BYTES: usize = pad_after::<AtomicUsize>();

/// ### English
/// One ring-buffer slot storing a single vsync callback payload.
///
/// ### 中文
/// ring buffer 的单个槽位，存储一个 vsync 回调载荷。
struct VsyncRingSlot {
    /// ### English
    /// Stored callback payload (written by producer, read by the single consumer).
    ///
    /// ### 中文
    /// 回调载荷（由生产者写入，单消费者读取）。
    value: UnsafeCell<MaybeUninit<VsyncCallback>>,
}

unsafe impl Send for VsyncRingSlot {}
unsafe impl Sync for VsyncRingSlot {}

#[repr(C, align(64))]
/// ### English
/// Lock-free queue of vsync callbacks (hot path ring buffer + cold overflow list).
///
/// Threading model:
/// - Single producer: Servo thread calls `push()`.
/// - Single consumer: embedder tick thread calls `tick()`.
///
/// `push()` is not multi-producer safe.
///
/// ### 中文
/// Vsync 回调的无锁队列（热路径 ring buffer + 冷路径 overflow 链表）。
///
/// 线程模型：
/// - 单生产者：Servo 线程调用 `push()`。
/// - 单消费者：宿主 tick 线程调用 `tick()`。
///
/// `push()` 不支持多生产者并发调用。
pub struct VsyncCallbackQueue {
    /// ### English
    /// Producer head index (push position).
    ///
    /// ### 中文
    /// 生产者 head（push 位置）。
    head: AtomicUsize,
    /// ### English
    /// Producer-side cached consumer tail (reduces atomic loads on the hot path).
    ///
    /// ### 中文
    /// 生产者侧缓存的消费者 tail（减少热路径原子读取）。
    producer_cached_tail: AtomicUsize,
    /// ### English
    /// Producer-local cache for overflow nodes.
    /// Avoids per-node CAS pops on the shared free-list (prevents ABA and reduces contention).
    ///
    /// ### 中文
    /// 生产者本地的 overflow 节点缓存。
    /// 避免对共享 free-list 做逐节点 CAS pop（消除 ABA 并降低争用）。
    producer_free_cache: UnsafeCell<*mut VsyncCallbackNode>,
    /// ### English
    /// Padding to keep producer and consumer indices on different cache lines.
    ///
    /// ### 中文
    /// 填充：让生产者/消费者索引尽量不共用 cache line（降低伪共享）。
    _pad_head: [u8; VSYNC_PAD_HEAD_BYTES],
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
    _pad_tail: [u8; VSYNC_PAD_TAIL_BYTES],

    /// ### English
    /// Bitmask for indexing into `slots` (capacity is a power of two).
    ///
    /// ### 中文
    /// 用于索引 `slots` 的掩码（capacity 为 2 的幂）。
    mask: usize,
    /// ### English
    /// Hot-path ring buffer storage.
    ///
    /// ### 中文
    /// 热路径 ring buffer 存储区。
    slots: Box<[VsyncRingSlot]>,

    /// ### English
    /// Overflow fallback (should be cold in normal vsync usage).
    ///
    /// ### 中文
    /// 溢出回退路径（正常 vsync 使用下应为冷路径）。
    callbacks: AtomicPtr<VsyncCallbackNode>,
    /// ### English
    /// Free-list for overflow nodes (reused to avoid allocations).
    ///
    /// ### 中文
    /// overflow 节点的 free-list（复用以避免分配）。
    free: AtomicPtr<VsyncCallbackNode>,
    /// ### English
    /// Count of overflow callbacks queued (caps growth when tick stalls).
    ///
    /// ### 中文
    /// 当前排队的溢出回调数量（tick 停滞时用于限制增长）。
    overflow_len: AtomicUsize,
}

unsafe impl Sync for VsyncCallbackQueue {}
unsafe impl Send for VsyncCallbackQueue {}

impl VsyncCallbackQueue {
    /// ### English
    /// Creates a queue with at least `capacity` ring slots (rounded up to power-of-two).
    ///
    /// The hot path is a lock-free ring buffer; overflow falls back to a cold intrusive list.
    ///
    /// A small batch of overflow nodes is preallocated to avoid allocations when the cold path is
    /// first hit under pressure.
    ///
    /// ### 中文
    /// 创建一个至少包含 `capacity` 个 ring 槽位的队列（向上取整为 2 的幂）。
    ///
    /// 热路径是无锁 ring buffer；溢出时回退到冷路径的侵入式链表。
    ///
    /// 为避免压力下首次进入冷路径触发分配，会预先分配少量 overflow 节点。
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1).next_power_of_two();
        debug_assert!(capacity.is_power_of_two());
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(VsyncRingSlot {
                value: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }

        let mut free_head: *mut VsyncCallbackNode = ptr::null_mut();
        let prealloc = VSYNC_OVERFLOW_NODE_PREALLOC.min(capacity);
        for _ in 0..prealloc {
            let node = Box::into_raw(Box::new(VsyncCallbackNode {
                next: free_head,
                callback: None,
            }));
            free_head = node;
        }

        Self {
            head: AtomicUsize::new(0),
            producer_cached_tail: AtomicUsize::new(0),
            producer_free_cache: UnsafeCell::new(ptr::null_mut()),
            _pad_head: [0; VSYNC_PAD_HEAD_BYTES],
            tail: AtomicUsize::new(0),
            _pad_tail: [0; VSYNC_PAD_TAIL_BYTES],
            mask: capacity - 1,
            slots: slots.into_boxed_slice(),
            callbacks: AtomicPtr::new(ptr::null_mut()),
            free: AtomicPtr::new(free_head),
            overflow_len: AtomicUsize::new(0),
        }
    }

    /// ### English
    /// Pushes one vsync callback into the queue.
    ///
    /// This is called by Servo's `RefreshDriver` implementation. The callback will be executed
    /// by `tick()` on the embedder thread (Java-driven vsync).
    ///
    /// #### Parameters
    /// - `callback`: Callback to execute on the next embedder tick.
    ///
    /// ### 中文
    /// 向队列 push 一个 vsync 回调。
    ///
    /// 该函数由 Servo 的 `RefreshDriver` 调用；回调会在宿主线程（Java 驱动 vsync）调用
    /// `tick()` 时执行。
    ///
    /// #### 参数
    /// - `callback`：将在下一次宿主 tick 执行的回调。
    pub fn push(&self, callback: VsyncCallback) {
        let head = self.head.load(Ordering::Relaxed);
        let mut cached_tail = self.producer_cached_tail.load(Ordering::Relaxed);
        let capacity = self.mask.wrapping_add(1);

        if head.wrapping_sub(cached_tail) >= capacity {
            cached_tail = self.tail.load(Ordering::Acquire);
            self.producer_cached_tail
                .store(cached_tail, Ordering::Relaxed);
            if head.wrapping_sub(cached_tail) >= capacity {
                self.push_overflow(callback);
                return;
            }
        }

        let idx = head & self.mask;
        unsafe {
            (*self.slots[idx].value.get()).write(callback);
        }
        self.head.store(head.wrapping_add(1), Ordering::Release);
    }

    /// ### English
    /// Drains queued callbacks and executes them on the calling thread.
    ///
    /// The embedder should call this from its vsync loop (or render loop) to drive Servo refresh.
    ///
    /// This drains only up to the head snapshot taken at the start of the tick; callbacks pushed
    /// during the tick are deferred to the next tick to keep ordering simple and avoid extra
    /// synchronization.
    ///
    /// ### 中文
    /// drain 并在调用线程执行所有回调。
    ///
    /// 宿主应在自身的 vsync 循环（或渲染循环）中调用它来驱动 Servo refresh。
    ///
    /// tick 开始时会获取 head 的快照；本次 tick 仅 drain 到该快照为止，tick 期间新 push 的回调留到下一次，
    /// 以保持顺序简单并避免额外同步。
    pub fn tick(&self) {
        let tail = self.tail.load(Ordering::Relaxed);
        let head_snapshot = self.head.load(Ordering::Acquire);
        if tail == head_snapshot && self.callbacks.load(Ordering::Relaxed).is_null() {
            return;
        }

        let overflow = self.callbacks.swap(ptr::null_mut(), Ordering::AcqRel);

        let mut tail = tail;
        while tail != head_snapshot {
            let idx = tail & self.mask;
            let callback = unsafe { (*self.slots[idx].value.get()).assume_init_read() };
            tail = tail.wrapping_add(1);
            self.tail.store(tail, Ordering::Release);
            callback();
        }

        self.drain_overflow_list(overflow);
    }
}

impl Drop for VsyncCallbackQueue {
    /// ### English
    /// Drops any remaining queued callbacks and releases the overflow/free lists.
    ///
    /// ### 中文
    /// drop 时释放队列中仍未执行的回调，并回收 overflow/free 链表。
    fn drop(&mut self) {
        let head = self.head.load(Ordering::Relaxed);
        let mut tail = self.tail.load(Ordering::Relaxed);
        while tail != head {
            let idx = tail & self.mask;
            unsafe {
                drop((*self.slots[idx].value.get()).assume_init_read());
            }
            tail = tail.wrapping_add(1);
        }

        drop_vsync_list(&self.callbacks);
        drop_vsync_list(&self.free);
        drop_vsync_raw_list(unsafe { *self.producer_free_cache.get() });
    }
}

mod fallback;
