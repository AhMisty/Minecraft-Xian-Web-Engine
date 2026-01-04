//! ### English
//! Cold overflow list node and helpers for `VsyncCallbackQueue`.
//!
//! ### 中文
//! `VsyncCallbackQueue` 的冷路径 overflow 链表节点与辅助方法。

use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::VsyncCallback;

/// ### English
/// Intrusive node used by the cold overflow list.
///
/// Ownership model:
/// - The list stores `Box<VsyncCallbackNode>` converted to raw pointers.
/// - Nodes are recycled via free-lists to avoid allocations on bursty overflow.
///
/// ### 中文
/// 冷路径 overflow 链表使用的侵入式节点。
///
/// 所有权模型：
/// - 链表中存放的是 `Box<VsyncCallbackNode>` 转换后的裸指针。
/// - 节点通过 free-list 复用，避免突发 overflow 时频繁分配。
pub(super) struct VsyncCallbackNode {
    /// ### English
    /// Intrusive next pointer.
    ///
    /// ### 中文
    /// 侵入式 next 指针。
    pub(super) next: *mut VsyncCallbackNode,
    /// ### English
    /// Callback payload for overflow path (taken on tick).
    ///
    /// ### 中文
    /// overflow 路径的回调载荷（tick 时取走执行）。
    pub(super) callback: Option<VsyncCallback>,
}

/// ### English
/// Drops and clears an intrusive `VsyncCallbackNode` list stored in an `AtomicPtr`.
///
/// #### Parameters
/// - `list`: Atomic pointer storing the list head.
///
/// ### 中文
/// drop 并清空一个存储在 `AtomicPtr` 中的侵入式 `VsyncCallbackNode` 链表。
///
/// #### 参数
/// - `list`：存放链表头指针的原子指针。
pub(super) fn drop_vsync_list(list: &AtomicPtr<VsyncCallbackNode>) {
    let node = list.swap(ptr::null_mut(), Ordering::AcqRel);
    drop_vsync_raw_list(node);
}

/// ### English
/// Drops an intrusive `VsyncCallbackNode` list starting from `node`.
///
/// #### Parameters
/// - `node`: List head pointer (NULL is a no-op).
///
/// ### 中文
/// 从 `node` 开始 drop 一条侵入式 `VsyncCallbackNode` 链表。
///
/// #### 参数
/// - `node`：链表头指针（NULL 则无操作）。
pub(super) fn drop_vsync_raw_list(mut node: *mut VsyncCallbackNode) {
    while !node.is_null() {
        unsafe {
            let boxed = Box::from_raw(node);
            node = boxed.next;
            drop(boxed.callback);
        }
    }
}
