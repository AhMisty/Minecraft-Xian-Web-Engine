//! ### English
//! Coalesced pointer to a boxed payload (latest-wins).
//!
//! ### 中文
//! `Box<T>` 的原子合并槽（latest-wins）。

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
    /// ### English
    /// Pending payload pointer (NULL means empty).
    ///
    /// ### 中文
    /// 待处理载荷指针（NULL 表示为空）。
    ptr: AtomicPtr<T>,
    /// ### English
    /// Single-node free cache pointer for reuse (NULL means empty).
    ///
    /// ### 中文
    /// 用于复用的单节点 free cache 指针（NULL 表示为空）。
    free: AtomicPtr<T>,
}

unsafe impl<T: Send> Send for CoalescedBox<T> {}
unsafe impl<T: Send> Sync for CoalescedBox<T> {}

impl<T> Default for CoalescedBox<T> {
    /// ### English
    /// Creates an empty coalescer with no pending payload and an empty free cache.
    ///
    /// ### 中文
    /// 创建一个空的 coalescer：无待处理载荷，free cache 为空。
    fn default() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
            free: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl<T> CoalescedBox<T> {
    /// ### English
    /// Returns whether a payload is currently pending (Acquire load).
    ///
    /// ### 中文
    /// 返回当前是否存在待处理载荷（Acquire 读取）。
    #[inline]
    pub(crate) fn is_pending(&self) -> bool {
        !self.ptr.load(Ordering::Acquire).is_null()
    }

    /// ### English
    /// Replaces the pending payload with `node` and returns the previous payload (if any).
    ///
    /// ### 中文
    /// 用 `node` 替换待处理载荷，并返回之前的载荷（若存在）。
    #[inline]
    pub(crate) fn replace(&self, node: Box<T>) -> Option<Box<T>> {
        let new_ptr = Box::into_raw(node);
        let old_ptr = self.ptr.swap(new_ptr, Ordering::AcqRel);
        if old_ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(old_ptr) })
        }
    }

    /// ### English
    /// Takes the pending payload, leaving the coalescer empty.
    ///
    /// ### 中文
    /// 取出待处理载荷，并将 coalescer 置空。
    #[inline]
    pub(crate) fn take(&self) -> Option<Box<T>> {
        let ptr = self.ptr.swap(ptr::null_mut(), Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(ptr) })
        }
    }

    /// ### English
    /// Pops one cached free node (if present) to reuse without allocating.
    ///
    /// ### 中文
    /// 取出一个缓存的空闲节点（若存在），用于复用以避免分配。
    #[inline]
    pub(crate) fn pop_free(&self) -> Option<Box<T>> {
        let ptr = self.free.swap(ptr::null_mut(), Ordering::AcqRel);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { Box::from_raw(ptr) })
        }
    }

    /// ### English
    /// Pushes `node` into the free cache (dropping the previous cached node, if any).
    ///
    /// ### 中文
    /// 将 `node` 放入 free cache（若已存在缓存节点，则丢弃旧节点）。
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
    /// ### English
    /// Drops any pending payload and cached free node.
    ///
    /// ### 中文
    /// drop 时释放待处理载荷与缓存的空闲节点。
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
