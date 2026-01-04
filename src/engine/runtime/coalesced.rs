//! ### English
//! Shared coalesced state between embedder threads and the dedicated Servo thread.
//!
//! ### 中文
//! 宿主线程与独立 Servo 线程之间共享的合并（coalesced）状态。
use std::sync::atomic::{AtomicU8, Ordering};

use crate::engine::lockfree::CoalescedBox;

#[repr(C, align(64))]
/// ### English
/// Per-view pending work bitmask shared across threads.
///
/// The high bit is reserved as an internal "busy" marker to coalesce wakeups.
///
/// ### 中文
/// 跨线程共享的每 view 待处理工作位图。
///
/// 最高位保留为内部 “busy” 标记，用于合并唤醒。
pub(super) struct PendingWork {
    /// ### English
    /// Pending bitmask with an internal busy bit for wake coalescing.
    ///
    /// ### 中文
    /// 待处理位图，包含用于合并唤醒的内部 busy 位。
    mask: AtomicU8,
    /// ### English
    /// Padding for cache-line alignment.
    ///
    /// ### 中文
    /// cache line 对齐填充。
    _padding: [u8; 7],
}

const BUSY_BIT: u8 = 1 << 7;

pub(super) const PENDING_MOUSE_MOVE: u8 = 1 << 0;
pub(super) const PENDING_RESIZE: u8 = 1 << 1;
pub(super) const PENDING_INPUT: u8 = 1 << 2;
pub(super) const PENDING_LOAD_URL: u8 = 1 << 3;
pub(super) const PENDING_ACTIVE: u8 = 1 << 4;

impl Default for PendingWork {
    /// ### English
    /// Creates an empty pending-work bitmask.
    ///
    /// ### 中文
    /// 创建一个空的 pending-work 位图。
    fn default() -> Self {
        Self {
            mask: AtomicU8::new(0),
            _padding: [0; 7],
        }
    }
}

impl PendingWork {
    /// ### English
    /// Marks work bits as pending and sets the internal busy bit.
    ///
    /// Returns `true` iff this call transitions from idle (`0`) to busy (`!= 0`), meaning the view ID
    /// should be pushed into the global pending queue.
    ///
    /// #### Parameters
    /// - `bits`: Work bits to mark as pending (without the internal busy bit).
    ///
    /// ### 中文
    /// 标记若干 work bit 为 pending，并设置内部 busy bit。
    /// 若本次调用把状态从 idle（`0`）切换为 busy（`!= 0`），则返回 `true`
    /// （表示需要把 view ID push 到全局 pending 队列）。
    ///
    /// #### 参数
    /// - `bits`：要标记为 pending 的 work bit（不包含内部 busy 位）。
    #[inline]
    pub(super) fn mark(&self, bits: u8) -> bool {
        let prev = self.mask.fetch_or(bits | BUSY_BIT, Ordering::Release);
        (prev & BUSY_BIT) == 0
    }

    /// ### English
    /// Takes and clears all pending work bits, keeping the internal busy bit set.
    ///
    /// ### 中文
    /// 取出并清除所有 pending work bit，同时保持内部 busy bit 为已设置状态。
    #[inline]
    pub(super) fn take(&self) -> u8 {
        self.mask.swap(BUSY_BIT, Ordering::Acquire) & !BUSY_BIT
    }

    /// ### English
    /// Returns whether any work (or the busy bit) is currently marked.
    ///
    /// ### 中文
    /// 返回是否存在任何 work（或 busy）标记。
    #[inline]
    pub(super) fn is_marked(&self) -> bool {
        self.mask.load(Ordering::Relaxed) != 0
    }

    /// ### English
    /// Returns whether only the internal busy bit is set (no actual pending work bits).
    ///
    /// ### 中文
    /// 返回是否仅设置了内部 busy bit（没有实际的 pending work bit）。
    #[inline]
    pub(super) fn is_busy_only(&self) -> bool {
        self.mask.load(Ordering::Relaxed) == BUSY_BIT
    }

    /// ### English
    /// Clears the busy bit if there is no pending work.
    ///
    /// Returns `true` if we successfully transition from `BUSY` to `0`.
    ///
    /// ### 中文
    /// 若无 pending work，则尝试清除 busy bit。
    /// 若成功把状态从 `BUSY` 切换到 `0`，返回 `true`。
    #[inline]
    pub(super) fn clear_busy_if_idle(&self) -> bool {
        self.mask
            .compare_exchange(BUSY_BIT, 0, Ordering::Release, Ordering::Relaxed)
            .is_ok()
    }
}

/// ### English
/// Boxed URL payload used by `CoalescedLoadUrl`.
///
/// ### 中文
/// `CoalescedLoadUrl` 使用的 boxed URL 载荷。
pub(super) struct LoadUrlRequest {
    /// ### English
    /// URL string payload (UTF-8).
    ///
    /// ### 中文
    /// URL 字符串载荷（UTF-8）。
    url: String,
}

impl LoadUrlRequest {
    /// ### English
    /// Returns the stored URL string.
    ///
    /// ### 中文
    /// 返回存储的 URL 字符串。
    #[inline]
    pub(super) fn as_str(&self) -> &str {
        &self.url
    }
}

/// ### English
/// Coalesced URL load request: stores only the latest URL string until drained by the Servo thread.
///
/// ### 中文
/// 合并后的 URL load 请求：只保留最新一次 URL 字符串，等待 Servo 线程 drain。
#[derive(Default)]
#[repr(C, align(64))]
pub(super) struct CoalescedLoadUrl {
    /// ### English
    /// Latest-wins boxed request storage with a small free cache.
    ///
    /// ### 中文
    /// latest-wins 的 boxed 请求存储，并带小型 free cache。
    inner: CoalescedBox<LoadUrlRequest>,
}

impl CoalescedLoadUrl {
    /// ### English
    /// Pops a reusable URL request node from the free cache (if present).
    ///
    /// ### 中文
    /// 从 free cache 取出一个可复用的 URL 请求节点（若存在）。
    #[inline]
    fn pop_free(&self) -> Option<Box<LoadUrlRequest>> {
        self.inner.pop_free()
    }

    /// ### English
    /// Pushes a URL request node back into the free cache after clearing its string.
    ///
    /// #### Parameters
    /// - `node`: Node to recycle.
    ///
    /// ### 中文
    /// 清空字符串后，将 URL 请求节点推回 free cache。
    ///
    /// #### 参数
    /// - `node`：要回收的节点。
    #[inline]
    fn push_free(&self, mut node: Box<LoadUrlRequest>) {
        node.url.clear();
        self.inner.push_free(node);
    }

    /// ### English
    /// Stores the latest URL string (coalesced; latest wins).
    ///
    /// #### Parameters
    /// - `url`: URL string to store.
    ///
    /// ### 中文
    /// 写入最新 URL 字符串（合并；只保留最新一次）。
    ///
    /// #### 参数
    /// - `url`：要写入的 URL 字符串。
    #[inline]
    pub(super) fn set_str(&self, url: &str) {
        let mut node = self.pop_free().unwrap_or_else(|| {
            Box::new(LoadUrlRequest {
                url: String::with_capacity(url.len()),
            })
        });

        node.url.clear();
        node.url.push_str(url);
        if let Some(old) = self.inner.replace(node) {
            self.push_free(old);
        }
    }

    /// ### English
    /// Takes the latest URL request if pending.
    ///
    /// ### 中文
    /// 若处于 pending，则取出最新的 URL 请求。
    #[inline]
    pub(super) fn take(&self) -> Option<Box<LoadUrlRequest>> {
        self.inner.take()
    }

    /// ### English
    /// Recycles a drained URL request node for reuse (avoids allocations on hot path).
    ///
    /// #### Parameters
    /// - `node`: Drained request node to recycle.
    ///
    /// ### 中文
    /// 回收已 drain 的 URL 请求节点以复用（避免热路径分配）。
    ///
    /// #### 参数
    /// - `node`：需要回收复用的请求节点。
    #[inline]
    pub(super) fn recycle(&self, node: Box<LoadUrlRequest>) {
        self.push_free(node);
    }
}
