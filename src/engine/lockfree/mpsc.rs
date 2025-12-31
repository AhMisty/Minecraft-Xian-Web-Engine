use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

#[repr(C)]
struct MpscNode<T> {
    next: AtomicPtr<MpscNode<T>>,
    value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send> Send for MpscNode<T> {}
unsafe impl<T: Send> Sync for MpscNode<T> {}

impl<T> MpscNode<T> {
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
    head: UnsafeCell<*mut MpscNode<T>>,
    tail: AtomicPtr<MpscNode<T>>,
    free_cache: AtomicPtr<MpscNode<T>>,
}

unsafe impl<T: Send> Send for MpscQueue<T> {}
unsafe impl<T: Send> Sync for MpscQueue<T> {}

impl<T> MpscQueue<T> {
    pub(crate) fn new() -> Self {
        let stub = Box::into_raw(Box::new(MpscNode::new()));
        Self {
            head: UnsafeCell::new(stub),
            tail: AtomicPtr::new(stub),
            free_cache: AtomicPtr::new(ptr::null_mut()),
        }
    }

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

    #[inline]
    pub(crate) fn pop(&self) -> Option<T> {
        let head = unsafe { *self.head.get() };
        let mut next = unsafe { (*head).next.load(Ordering::Acquire) };

        if next.is_null() {
            if self.tail.load(Ordering::Acquire) == head {
                return None;
            }
            loop {
                next = unsafe { (*head).next.load(Ordering::Acquire) };
                if !next.is_null() {
                    break;
                }
                std::hint::spin_loop();
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

    #[inline]
    fn pop_free_node(&self) -> Option<*mut MpscNode<T>> {
        let node = self.free_cache.swap(ptr::null_mut(), Ordering::AcqRel);
        (!node.is_null()).then_some(node)
    }
}

impl<T> Drop for MpscQueue<T> {
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
