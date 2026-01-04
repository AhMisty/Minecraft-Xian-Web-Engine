//! ### English
//! Unbounded lock-free MPSC queue (multi-producer, single-consumer).
//!
//! ### 中文
//! 无界无锁 MPSC 队列（多生产者、单消费者）。

use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::Backoff;

#[repr(C)]
/// ### English
/// Intrusive node used by the Vyukov MPSC linked-list algorithm.
///
/// ### 中文
/// Vyukov MPSC 链表算法使用的侵入式节点。
struct MpscNode<T> {
    /// ### English
    /// Intrusive next pointer used by the linked-list algorithm.
    ///
    /// ### 中文
    /// 链表算法使用的侵入式 next 指针。
    next: AtomicPtr<MpscNode<T>>,
    /// ### English
    /// Payload storage written by the producer that owns this node.
    ///
    /// ### 中文
    /// 载荷存储区：由持有该节点的生产者写入。
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send> Send for MpscNode<T> {}
unsafe impl<T: Send> Sync for MpscNode<T> {}

impl<T> MpscNode<T> {
    /// ### English
    /// Creates an empty node with a NULL `next` pointer.
    ///
    /// ### 中文
    /// 创建一个空节点，并将 `next` 初始化为 NULL。
    #[inline]
    fn new() -> Self {
        Self {
            next: AtomicPtr::new(ptr::null_mut()),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

/// ### English
/// Unbounded lock-free MPSC queue (multi-producer, single-consumer).
///
/// - FIFO (Vyukov MPSC linked-list algorithm).
/// - Consumer must be single-threaded.
///
/// ### 中文
/// 无界无锁 MPSC 队列（多生产者、单消费者）。
///
/// - FIFO（Vyukov MPSC 链表算法）。
/// - 消费端必须是单线程调用。
pub(crate) struct MpscQueue<T> {
    /// ### English
    /// Consumer-owned head pointer (stub/next).
    ///
    /// ### 中文
    /// 由消费者持有的 head 指针（stub/next）。
    head: UnsafeCell<*mut MpscNode<T>>,
    /// ### English
    /// Producer-owned tail pointer (atomic swap).
    ///
    /// ### 中文
    /// 由生产者更新的 tail 指针（原子 swap）。
    tail: AtomicPtr<MpscNode<T>>,
    /// ### English
    /// Single-node free cache used to reuse one node and cap allocator churn on hot paths.
    ///
    /// Note: this does NOT make the queue bounded; the backlog can still grow without bound.
    ///
    /// ### 中文
    /// 单节点 free cache：用于复用一个节点以减少热路径分配抖动。
    ///
    /// 注意：这并不意味着队列“有界”；队列 backlog 仍可能无界增长。
    free_cache: AtomicPtr<MpscNode<T>>,
}

unsafe impl<T: Send> Send for MpscQueue<T> {}
unsafe impl<T: Send> Sync for MpscQueue<T> {}

impl<T> MpscQueue<T> {
    /// ### English
    /// Creates an empty MPSC queue.
    ///
    /// Internally this initializes the algorithm's stub node.
    ///
    /// ### 中文
    /// 创建一个空的 MPSC 队列。
    ///
    /// 内部会初始化算法所需的 stub 节点。
    pub(crate) fn new() -> Self {
        let stub = Box::into_raw(Box::new(MpscNode::new()));
        Self {
            head: UnsafeCell::new(stub),
            tail: AtomicPtr::new(stub),
            free_cache: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// ### English
    /// Pushes one value into the queue (multi-producer safe).
    ///
    /// #### Parameters
    /// - `value`: Value to push.
    ///
    /// ### 中文
    /// push 一个值（支持多生产者并发）。
    ///
    /// #### 参数
    /// - `value`：要 push 的值。
    #[inline]
    pub(crate) fn push(&self, value: T) {
        let node = self
            .pop_free_node()
            .unwrap_or_else(|| Box::into_raw(Box::new(MpscNode::new())));
        unsafe {
            (*node).next.store(ptr::null_mut(), Ordering::Relaxed);
            (*(*node).value.get()).write(value);
        }

        let prev = self.tail.swap(node, Ordering::AcqRel);
        unsafe {
            (*prev).next.store(node, Ordering::Release);
        }
    }

    /// ### English
    /// Pops one value from the queue (single-consumer only).
    ///
    /// If a producer has swapped the tail but hasn't linked `prev.next` yet, the consumer may
    /// need to wait briefly; we spin for a short time and then yield to reduce CPU burn on
    /// oversubscribed systems.
    ///
    /// ### 中文
    /// pop 一个值（仅允许单消费者调用）。
    ///
    /// 当生产者已更新 tail 但尚未把 `prev.next` 链接完成时，消费端可能需要短暂等待；
    /// 这里先自旋一小段时间，然后 `yield` 让出 CPU，避免在单核/过载时空转占满。
    #[inline]
    pub(crate) fn pop(&self) -> Option<T> {
        let head = unsafe { *self.head.get() };
        let mut next = unsafe { (*head).next.load(Ordering::Acquire) };

        if next.is_null() {
            if self.tail.load(Ordering::Acquire) == head {
                return None;
            }
            let mut backoff = Backoff::new();
            loop {
                next = unsafe { (*head).next.load(Ordering::Acquire) };
                if !next.is_null() {
                    break;
                }
                backoff.snooze();
            }
        }

        let value = unsafe { (*(*next).value.get()).assume_init_read() };
        unsafe {
            *self.head.get() = next;
            (*head).next.store(ptr::null_mut(), Ordering::Relaxed);
        }
        self.push_free_node(head);
        Some(value)
    }

    /// ### English
    /// Recycles one node into the single-node free cache (drops the previous cached node).
    ///
    /// #### Parameters
    /// - `node`: Node pointer to recycle.
    ///
    /// ### 中文
    /// 将一个节点回收到单节点 free cache（并丢弃之前缓存的节点）。
    ///
    /// #### 参数
    /// - `node`：需要回收的节点指针。
    #[inline]
    fn push_free_node(&self, node: *mut MpscNode<T>) {
        unsafe {
            (*node).next.store(ptr::null_mut(), Ordering::Relaxed);
        }
        let prev = self.free_cache.swap(node, Ordering::AcqRel);
        if !prev.is_null() {
            unsafe {
                drop(Box::from_raw(prev));
            }
        }
    }

    /// ### English
    /// Pops the cached free node if present.
    ///
    /// ### 中文
    /// 若缓存中存在可复用节点，则取出它。
    #[inline]
    fn pop_free_node(&self) -> Option<*mut MpscNode<T>> {
        let node = self.free_cache.swap(ptr::null_mut(), Ordering::AcqRel);
        (!node.is_null()).then_some(node)
    }
}

impl<T> Drop for MpscQueue<T> {
    /// ### English
    /// Drains the queue and frees internal nodes (including the stub node).
    ///
    /// ### 中文
    /// drop 时 drain 队列并释放内部节点（包括 stub 节点）。
    fn drop(&mut self) {
        while let Some(value) = self.pop() {
            drop(value);
        }

        let head = unsafe { *self.head.get() };
        unsafe {
            drop(Box::from_raw(head));
        }

        let free = self.free_cache.swap(ptr::null_mut(), Ordering::AcqRel);
        if !free.is_null() {
            unsafe {
                drop(Box::from_raw(free));
            }
        }
    }
}
