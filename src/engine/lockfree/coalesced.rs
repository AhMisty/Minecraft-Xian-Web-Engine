use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

/// ### English
/// Coalesced pointer to a boxed payload (latest-wins), with a single-node free cache to avoid
/// allocation churn on hot paths.
///
/// - Multi-producer friendly: writers use an atomic swap.
/// - Single cached free node: keeps peak memory bounded to O(1) per coalescer.
///
/// ### 中文
/// “只保留最新值（latest-wins）”的 `Box<T>` 原子交换槽，并带一个单节点 free cache，用于避免热路径的反复分配。
///
/// - 支持多生产者：写端使用原子 swap。
/// - 单节点复用：每个 coalescer 的缓存节点数量为 O(1)，避免 free-list 无界增长。
pub(crate) struct CoalescedBox<T> {
    ptr: AtomicPtr<T>,
    free: AtomicPtr<T>,
}

unsafe impl<T: Send> Send for CoalescedBox<T> {}
unsafe impl<T: Send> Sync for CoalescedBox<T> {}

impl<T> Default for CoalescedBox<T> {
    fn default() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
            free: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl<T> CoalescedBox<T> {
    #[inline]
    pub(crate) fn is_pending(&self) -> bool {
        !self.ptr.load(Ordering::Acquire).is_null()
    }

    #[inline]
    pub(crate) fn replace(&self, node: Box<T>) -> Option<Box<T>> {
        let new_ptr = Box::into_raw(node);
        let old_ptr = self.ptr.swap(new_ptr, Ordering::Release);
        if old_ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(old_ptr) })
        }
    }

    #[inline]
    pub(crate) fn take(&self) -> Option<Box<T>> {
        let ptr = self.ptr.swap(ptr::null_mut(), Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(ptr) })
        }
    }

    #[inline]
    pub(crate) fn pop_free(&self) -> Option<Box<T>> {
        let ptr = self.free.swap(ptr::null_mut(), Ordering::AcqRel);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(ptr) })
        }
    }

    #[inline]
    pub(crate) fn push_free(&self, node: Box<T>) {
        let new_ptr = Box::into_raw(node);
        let old_ptr = self.free.swap(new_ptr, Ordering::AcqRel);
        if !old_ptr.is_null() {
            unsafe {
                drop(Box::from_raw(old_ptr));
            }
        }
    }
}

impl<T> Drop for CoalescedBox<T> {
    fn drop(&mut self) {
        let current = self.ptr.swap(ptr::null_mut(), Ordering::AcqRel);
        if !current.is_null() {
            unsafe {
                drop(Box::from_raw(current));
            }
        }

        let free = self.free.swap(ptr::null_mut(), Ordering::AcqRel);
        if !free.is_null() {
            unsafe {
                drop(Box::from_raw(free));
            }
        }
    }
}
