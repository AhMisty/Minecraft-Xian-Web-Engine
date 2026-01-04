//! ### English
//! Lock-free `u32` ID queue used to signal pending work to the dedicated Servo thread.
//!
//! On overflow we set a flag so the consumer can fall back to a slow-path scan.
//!
//! ### 中文
//! 用于向独立 Servo 线程“信号化有待处理工作”的无锁 `u32` ID 队列。
//!
//! 溢出时会设置标记，消费者可回退到扫描兜底以避免漏处理。
use std::sync::atomic::{AtomicBool, Ordering};

use crate::engine::lockfree::BoundedMpscQueue;

/// ### English
/// Pending ID queue for coalescing per-view wakeups into a single drain on the Servo thread.
///
/// ### 中文
/// 用于把每 view 的唤醒合并为 Servo 线程一次 drain 的 pending ID 队列。
pub(super) struct PendingIdQueue {
    /// ### English
    /// Bounded ring buffer storing pending view IDs.
    ///
    /// ### 中文
    /// 存放 pending view ID 的有界 ring buffer。
    ring: BoundedMpscQueue<u32>,
    /// ### English
    /// Overflow marker: when set, the consumer should fall back to a full scan.
    ///
    /// ### 中文
    /// 溢出标记：置位时，消费者应回退到全量扫描兜底。
    overflowed: AtomicBool,
}

impl PendingIdQueue {
    /// ### English
    /// Creates a pending ID queue with the given ring capacity.
    ///
    /// #### Parameters
    /// - `capacity`: Ring capacity (rounded up internally as needed).
    ///
    /// ### 中文
    /// 创建一个指定 ring 容量的 pending ID 队列。
    ///
    /// #### 参数
    /// - `capacity`：ring 容量（内部会按需向上取整）。
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            ring: BoundedMpscQueue::with_capacity(capacity),
            overflowed: AtomicBool::new(false),
        }
    }

    /// ### English
    /// Tries to push an ID.
    ///
    /// #### Parameters
    /// - `id`: View ID to push.
    ///
    /// Returns `true` on success; returns `false` if the ring is full (and sets the overflow flag).
    ///
    /// ### 中文
    /// 尝试 push 一个 ID。
    ///
    /// #### 参数
    /// - `id`：要 push 的 view ID。
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
    /// pop 一个 ID（单消费者 / Servo 线程）。
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
