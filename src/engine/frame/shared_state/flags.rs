use std::sync::atomic::Ordering;

use super::SharedFrameState;

impl SharedFrameState {
    /// ### English
    /// Marks the whole triple buffer as "resizing" (consumer should stop acquiring).
    ///
    /// ### 中文
    /// 标记整个三缓冲处于 “resizing” 状态（消费者应停止 acquire）。
    pub fn set_resizing(&self, resizing: bool) {
        self.frame_meta
            .flags
            .resizing
            .store(u8::from(resizing), Ordering::Relaxed);
    }

    /// ### English
    /// Returns whether resizing is in progress.
    ///
    /// ### 中文
    /// 返回是否处于 resizing 状态。
    pub fn is_resizing(&self) -> bool {
        self.frame_meta.flags.resizing.load(Ordering::Relaxed) != 0
    }

    /// ### English
    /// Sets the active flag (used by the embedder to throttle/hide a view).
    ///
    /// ### 中文
    /// 设置 active 标记（宿主用来 throttle/hide view）。
    pub fn set_active(&self, active: bool) {
        self.frame_meta
            .flags
            .active
            .store(u8::from(active), Ordering::Relaxed);
    }

    /// ### English
    /// Returns whether the view is active.
    ///
    /// ### 中文
    /// 返回 view 是否 active。
    pub fn is_active(&self) -> bool {
        self.frame_meta.flags.active.load(Ordering::Relaxed) != 0
    }
}
