/// ### English
/// Lock-free `u32` ID queue used to signal pending work to the dedicated Servo thread.
/// On overflow we set a flag so the consumer can fall back to a slow-path scan.
///
/// ### 中文
/// 用于向独立 Servo 线程“信号化有待处理工作”的无锁 `u32` ID 队列。
/// 溢出时会设置标记，消费者可回退到扫描兜底以避免漏处理。
use std::sync::atomic::{AtomicBool, Ordering};

use crate::engine::lockfree::BoundedMpscQueue;

pub(super) struct PendingIdQueue {
    ring: BoundedMpscQueue<u32>,
    overflowed: AtomicBool,
}

impl PendingIdQueue {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            ring: BoundedMpscQueue::with_capacity(capacity),
            overflowed: AtomicBool::new(false),
        }
    }

    /// ### English
    /// Tries to enqueue an ID.
    ///
    /// Returns `true` on success; returns `false` if the ring is full (and sets the overflow flag).
    ///
    /// ### 中文
    /// 尝试入队一个 ID。
    ///
    /// 成功返回 `true`；若 ring 已满则返回 `false`（并设置 overflow 标记）。
    pub(super) fn push(&self, id: u32) -> bool {
        match self.ring.try_push(id) {
            Ok(()) => true,
            Err(_) => {
                self.overflowed.store(true, Ordering::Release);
                false
            }
        }
    }

    /// ### English
    /// Pops one queued ID (single consumer / Servo thread).
    ///
    /// ### 中文
    /// pop 一个已入队 ID（单消费者 / Servo 线程）。
    pub(super) fn pop(&self) -> Option<u32> {
        self.ring.pop()
    }

    /// ### English
    /// Returns and clears the overflow flag.
    ///
    /// ### 中文
    /// 返回并清除 overflow 标记。
    pub(super) fn take_overflowed(&self) -> bool {
        self.overflowed.swap(false, Ordering::AcqRel)
    }
}
