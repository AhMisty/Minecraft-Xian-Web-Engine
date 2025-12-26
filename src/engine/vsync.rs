//! ### English
//! Vsync callback queue used to drive Servo's `RefreshDriver` from the Java side.
//! Hot path is a bounded ring buffer; cold overflow falls back to an intrusive list.
//!
//! ### 中文
//! 由 Java 侧驱动 Servo `RefreshDriver` 的 vsync 回调队列。
//! 热路径使用有界 ring buffer；冷路径溢出时回退到侵入式链表。

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

type VsyncCallback = Box<dyn Fn() + Send + 'static>;

const CACHE_LINE_BYTES: usize = 64;
const ATOMIC_USIZE_BYTES: usize = std::mem::size_of::<AtomicUsize>();
const CACHE_PAD_BYTES: usize = CACHE_LINE_BYTES - ATOMIC_USIZE_BYTES;
const VSYNC_OVERFLOW_NODE_PREALLOC: usize = 1024;
const VSYNC_OVERFLOW_MAX: usize = 8192;

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
    _pad_head: [u8; CACHE_PAD_BYTES],
    /// ### English
    /// Consumer tail index (dequeue position).
    ///
    /// ### 中文
    /// 消费者 tail（出队位置）。
    tail: AtomicUsize,
    _pad_tail: [u8; CACHE_PAD_BYTES],

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

    /*
    ### English
    Overflow fallback (should be cold in normal vsync usage).

    ### 中文
    溢出回退路径（正常 vsync 使用下应为冷路径）。
    */
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

        /*
        ### English
        Preallocate a small pool of overflow nodes so the cold path avoids `Box::new` under load.

        ### 中文
        预分配一小批 overflow 节点，避免在压力下冷路径触发 `Box::new` 分配。
        */
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
            _pad_head: [0; CACHE_PAD_BYTES],
            tail: AtomicUsize::new(0),
            _pad_tail: [0; CACHE_PAD_BYTES],
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
        let tail = self.tail.load(Ordering::Acquire);
        let capacity = self.mask.wrapping_add(1);

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

        let mut overflow = self.callbacks.swap(ptr::null_mut(), Ordering::AcqRel);

        /*
        ### English
        Drain ring buffer up to the snapshot head so callbacks enqueued during tick run next tick.

        ### 中文
        仅 drain 到本次 tick 的 head 快照，tick 期间新入队的回调留到下一次 tick。
        */
        let mut tail = tail;
        while tail != head_snapshot {
            let idx = tail & self.mask;
            let callback = unsafe { (*self.slots[idx].value.get()).assume_init_read() };
            tail = tail.wrapping_add(1);
            self.tail.store(tail, Ordering::Release);
            callback();
        }

        let mut free_head: *mut VsyncCallbackNode = ptr::null_mut();
        let mut free_tail: *mut VsyncCallbackNode = ptr::null_mut();
        let mut drained_overflow = 0usize;

        while !overflow.is_null() {
            unsafe {
                let current = overflow;
                overflow = (*current).next;

                if let Some(callback) = (*current).callback.take() {
                    callback();
                }
                drained_overflow += 1;

                (*current).next = ptr::null_mut();
                if free_head.is_null() {
                    free_head = current;
                    free_tail = current;
                } else {
                    (*free_tail).next = current;
                    free_tail = current;
                }
            }
        }

        if !free_head.is_null() {
            self.push_free_list(free_head, free_tail);
        }
        if drained_overflow > 0 {
            self.overflow_len
                .fetch_sub(drained_overflow, Ordering::Release);
        }
    }

    fn enqueue_overflow(&self, callback: VsyncCallback) {
        let prev = self.overflow_len.fetch_add(1, Ordering::Relaxed);
        if prev >= VSYNC_OVERFLOW_MAX {
            self.overflow_len.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        let node_ptr = self.pop_free_node().unwrap_or_else(|| {
            Box::into_raw(Box::new(VsyncCallbackNode {
                next: ptr::null_mut(),
                callback: None,
            }))
        });

        unsafe {
            (*node_ptr).callback = Some(callback);
        }

        loop {
            let head = self.callbacks.load(Ordering::Acquire);
            unsafe {
                (*node_ptr).next = head;
            }

            if self
                .callbacks
                .compare_exchange_weak(head, node_ptr, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }

    fn pop_free_node(&self) -> Option<*mut VsyncCallbackNode> {
        let mut head = self.free.load(Ordering::Acquire);
        loop {
            if head.is_null() {
                return None;
            }

            let next = unsafe { (*head).next };
            match self
                .free
                .compare_exchange_weak(head, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Some(head),
                Err(updated) => head = updated,
            }
        }
    }

    fn push_free_list(&self, head_node: *mut VsyncCallbackNode, tail_node: *mut VsyncCallbackNode) {
        loop {
            let head = self.free.load(Ordering::Acquire);
            unsafe {
                (*tail_node).next = head;
            }
            if self
                .free
                .compare_exchange_weak(head, head_node, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }
}

impl Drop for VsyncCallbackQueue {
    fn drop(&mut self) {
        /*
        ### English
        Drop any callbacks still queued in the ring buffer.

        ### 中文
        Drop 时释放 ring buffer 中仍未执行的回调。
        */
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
    }
}

struct VsyncCallbackNode {
    /// ### English
    /// Intrusive next pointer.
    ///
    /// ### 中文
    /// 侵入式 next 指针。
    next: *mut VsyncCallbackNode,
    /// ### English
    /// Callback payload for overflow path (taken on tick).
    ///
    /// ### 中文
    /// overflow 路径的回调载荷（tick 时取走执行）。
    callback: Option<VsyncCallback>,
}

fn drop_vsync_list(list: &AtomicPtr<VsyncCallbackNode>) {
    let mut node = list.swap(ptr::null_mut(), Ordering::AcqRel);
    while !node.is_null() {
        unsafe {
            let boxed = Box::from_raw(node);
            node = boxed.next;
            drop(boxed.callback);
        }
    }
}
