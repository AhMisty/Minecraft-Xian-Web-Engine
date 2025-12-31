use std::ptr;
use std::sync::atomic::Ordering;

use super::super::VsyncCallback;
use super::super::overflow::VsyncCallbackNode;
use super::{VSYNC_OVERFLOW_MAX, VsyncCallbackQueue};

impl VsyncCallbackQueue {
    pub(super) fn enqueue_overflow(&self, callback: VsyncCallback) {
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

    pub(super) fn drain_overflow_list(&self, mut overflow: *mut VsyncCallbackNode) {
        if overflow.is_null() {
            return;
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

    fn pop_free_node(&self) -> Option<*mut VsyncCallbackNode> {
        let local = unsafe { *self.producer_free_cache.get() };
        if !local.is_null() {
            unsafe {
                *self.producer_free_cache.get() = (*local).next;
                (*local).next = ptr::null_mut();
            }
            return Some(local);
        }

        let list = self.free.swap(ptr::null_mut(), Ordering::AcqRel);
        if list.is_null() {
            return None;
        }

        unsafe {
            *self.producer_free_cache.get() = (*list).next;
            (*list).next = ptr::null_mut();
        }
        Some(list)
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
