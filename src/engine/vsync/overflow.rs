use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::VsyncCallback;

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

pub(super) fn drop_vsync_list(list: &AtomicPtr<VsyncCallbackNode>) {
    let mut node = list.swap(ptr::null_mut(), Ordering::AcqRel);
    while !node.is_null() {
        unsafe {
            let boxed = Box::from_raw(node);
            node = boxed.next;
            drop(boxed.callback);
        }
    }
}

/// ### English
/// Drops an intrusive `VsyncCallbackNode` list starting from `node`.
///
/// ### 中文
/// 从 `node` 开始 drop 一条侵入式 `VsyncCallbackNode` 链表。
pub(super) fn drop_vsync_raw_list(mut node: *mut VsyncCallbackNode) {
    while !node.is_null() {
        unsafe {
            let boxed = Box::from_raw(node);
            node = boxed.next;
            drop(boxed.callback);
        }
    }
}
