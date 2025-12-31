use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::engine::cache::CACHE_LINE_BYTES;

use super::VsyncCallback;
use super::overflow::{VsyncCallbackNode, drop_vsync_list, drop_vsync_raw_list};

const VSYNC_OVERFLOW_NODE_PREALLOC: usize = 1024;
const VSYNC_OVERFLOW_MAX: usize = 8192;
const ATOMIC_USIZE_BYTES: usize = std::mem::size_of::<AtomicUsize>();
const VSYNC_PAD_HEAD_BYTES: usize = CACHE_LINE_BYTES - 2 * ATOMIC_USIZE_BYTES;
const VSYNC_PAD_TAIL_BYTES: usize = CACHE_LINE_BYTES - ATOMIC_USIZE_BYTES;

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
/// ### 中文
/// Vsync 回调的无锁队列（热路径 ring buffer + 冷路径 overflow 链表）。
pub struct VsyncCallbackQueue {
    /// ### English
    /// Producer head index (enqueue position).
    ///
    /// ### 中文
    /// 生产者 head（入队位置）。
    head: AtomicUsize,
    producer_cached_tail: AtomicUsize,
    /// ### English
    /// Producer-local cache for overflow nodes.
    /// Avoids per-node CAS pops on the shared free-list (prevents ABA and reduces contention).
    ///
    /// ### 中文
    /// 生产者本地的 overflow 节点缓存。
    /// 避免对共享 free-list 做逐节点 CAS pop（消除 ABA 并降低争用）。
    producer_free_cache: UnsafeCell<*mut VsyncCallbackNode>,
    _pad_head: [u8; VSYNC_PAD_HEAD_BYTES],
    /// ### English
    /// Consumer tail index (dequeue position).
    ///
    /// ### 中文
    /// 消费者 tail（出队位置）。
    tail: AtomicUsize,
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
    /// ### 中文
    /// 创建一个至少包含 `capacity` 个 ring 槽位的队列（向上取整为 2 的幂）。
    ///
    /// 热路径是无锁 ring buffer；溢出时回退到冷路径的侵入式链表。
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1).next_power_of_two();
        debug_assert!(capacity.is_power_of_two());
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(VsyncRingSlot {
                value: UnsafeCell::new(MaybeUninit::uninit()),
            });
        }

        /// ### English
        /// Preallocate a small pool of overflow nodes so the cold path avoids `Box::new` under load.
        ///
        /// ### 中文
        /// 预分配一小批 overflow 节点，避免在压力下冷路径触发 `Box::new` 分配。
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
    /// Enqueues one vsync callback.
    ///
    /// This is called by Servo's `RefreshDriver` implementation. The callback will be executed
    /// by `tick()` on the embedder thread (Java-driven vsync).
    ///
    /// ### 中文
    /// 入队一个 vsync 回调。
    ///
    /// 该函数由 Servo 的 `RefreshDriver` 调用；回调会在宿主线程（Java 驱动 vsync）调用
    /// `tick()` 时执行。
    pub fn enqueue(&self, callback: VsyncCallback) {
        let head = self.head.load(Ordering::Relaxed);
        let cached_tail = self.producer_cached_tail.load(Ordering::Relaxed);
        let capacity = self.mask.wrapping_add(1);

        if head.wrapping_sub(cached_tail) < capacity {
            let idx = head & self.mask;
            unsafe {
                (*self.slots[idx].value.get()).write(callback);
            }
            self.head.store(head.wrapping_add(1), Ordering::Release);
            return;
        }

        let tail = self.tail.load(Ordering::Acquire);
        self.producer_cached_tail.store(tail, Ordering::Relaxed);
        if head.wrapping_sub(tail) < capacity {
            let idx = head & self.mask;
            unsafe {
                (*self.slots[idx].value.get()).write(callback);
            }
            self.head.store(head.wrapping_add(1), Ordering::Release);
            return;
        }

        self.enqueue_overflow(callback);
    }

    /// ### English
    /// Drains queued callbacks and executes them on the calling thread.
    ///
    /// The embedder should call this from its vsync loop (or render loop) to drive Servo refresh.
    ///
    /// ### 中文
    /// drain 并在调用线程执行所有已入队回调。
    ///
    /// 宿主应在自身的 vsync 循环（或渲染循环）中调用它来驱动 Servo refresh。
    pub fn tick(&self) {
        let tail = self.tail.load(Ordering::Relaxed);
        let head_snapshot = self.head.load(Ordering::Acquire);
        if tail == head_snapshot && self.callbacks.load(Ordering::Relaxed).is_null() {
            return;
        }

        let overflow = self.callbacks.swap(ptr::null_mut(), Ordering::AcqRel);

        /// ### English
        /// Drain ring buffer up to the snapshot head so callbacks enqueued during tick run next tick.
        ///
        /// ### 中文
        /// 仅 drain 到本次 tick 的 head 快照，tick 期间新入队的回调留到下一次 tick。
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
    fn drop(&mut self) {
        /// ### English
        /// Drop any callbacks still queued in the ring buffer.
        ///
        /// ### 中文
        /// Drop 时释放 ring buffer 中仍未执行的回调。
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
