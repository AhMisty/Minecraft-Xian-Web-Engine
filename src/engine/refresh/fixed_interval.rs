use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering as AtomicOrdering},
};
use std::time::Duration;

use servo::RefreshDriver;

use crate::engine::lockfree::CoalescedBox;

use super::scheduler::RefreshScheduler;

/// ### English
/// Refresh driver that ticks at a fixed interval (`target_fps != 0` path).
///
/// ### 中文
/// 固定间隔的 refresh driver（`target_fps != 0` 路径）。
pub struct FixedIntervalRefreshDriver {
    /// ### English
    /// Shared global scheduler (single background thread).
    ///
    /// ### 中文
    /// 共享的全局调度器（单个后台线程）。
    scheduler: Arc<RefreshScheduler>,
    /// ### English
    /// Fixed frame duration used to schedule the next refresh.
    ///
    /// ### 中文
    /// 用于调度下一次 refresh 的固定帧间隔。
    frame_duration: Duration,
    coalesced: Arc<FixedIntervalCoalesced>,
}

impl FixedIntervalRefreshDriver {
    /// ### English
    /// Creates a fixed-interval refresh driver.
    ///
    /// ### 中文
    /// 创建固定间隔 refresh driver。
    pub fn new(scheduler: Arc<RefreshScheduler>, frame_duration: Duration) -> Rc<Self> {
        Rc::new(Self {
            scheduler,
            frame_duration,
            coalesced: Arc::new(FixedIntervalCoalesced::new()),
        })
    }
}

impl RefreshDriver for FixedIntervalRefreshDriver {
    fn observe_next_frame(&self, start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.coalesced.submit(
            self.scheduler.clone(),
            self.frame_duration,
            start_frame_callback,
        );
    }
}

struct FrameCallbackNode {
    callback: Option<Box<dyn Fn() + Send + 'static>>,
}

impl FrameCallbackNode {
    #[inline]
    fn new() -> Self {
        Self { callback: None }
    }
}

struct FixedIntervalCoalesced {
    callback: CoalescedBox<FrameCallbackNode>,
    scheduled: AtomicBool,
}

impl FixedIntervalCoalesced {
    #[inline]
    fn new() -> Self {
        Self {
            callback: CoalescedBox::default(),
            scheduled: AtomicBool::new(false),
        }
    }

    #[inline]
    fn set_callback(&self, callback: Box<dyn Fn() + Send + 'static>) {
        let mut node = self
            .callback
            .pop_free()
            .unwrap_or_else(|| Box::new(FrameCallbackNode::new()));
        node.callback = Some(callback);

        if let Some(old) = self.callback.replace(node) {
            self.recycle_node(old);
        }
    }

    #[inline]
    fn take_callback(&self) -> Option<Box<dyn Fn() + Send + 'static>> {
        let mut node = self.callback.take()?;
        let callback = node.callback.take();
        self.callback.push_free(node);
        callback
    }

    #[inline]
    fn recycle_node(&self, mut node: Box<FrameCallbackNode>) {
        node.callback.take();
        self.callback.push_free(node);
    }

    #[inline]
    fn submit(
        self: &Arc<Self>,
        scheduler: Arc<RefreshScheduler>,
        delay: Duration,
        callback: Box<dyn Fn() + Send + 'static>,
    ) {
        self.set_callback(callback);
        if !self.scheduled.swap(true, AtomicOrdering::AcqRel) {
            let state = self.clone();
            let scheduler_for_enqueue = scheduler.clone();
            let scheduler_for_tick = scheduler.clone();
            scheduler_for_enqueue.schedule(
                delay,
                Box::new(move || state.clone().tick(scheduler_for_tick.clone(), delay)),
            );
        }
    }

    fn tick(self: Arc<Self>, scheduler: Arc<RefreshScheduler>, delay: Duration) {
        let callback = self.take_callback();

        /// ### English
        /// Clear the scheduled flag before running the callback so a new `observe_next_frame()` call
        /// during the callback can enqueue the next tick without waiting.
        ///
        /// ### 中文
        /// 在执行回调之前清除 scheduled 标记，使回调期间发生的 `observe_next_frame()`
        /// 可以直接安排下一次 tick。
        self.scheduled.store(false, AtomicOrdering::Release);

        if let Some(callback) = callback {
            callback();
        }

        /// ### English
        /// Rare race: a producer may have published a callback while `scheduled` was still true
        /// (between `take()` and `store(false)`), so we must re-arm the timer here.
        ///
        /// ### 中文
        /// 罕见竞态：生产者可能在 `scheduled` 仍为 true 的窗口（`take()` 与 `store(false)` 之间）
        /// 写入了新回调，此时需要在这里重新 arm。
        if self.callback.is_pending() && !self.scheduled.swap(true, AtomicOrdering::AcqRel) {
            let state = self.clone();
            let scheduler_for_enqueue = scheduler.clone();
            let scheduler_for_tick = scheduler.clone();
            scheduler_for_enqueue.schedule(
                delay,
                Box::new(move || state.clone().tick(scheduler_for_tick.clone(), delay)),
            );
        }
    }
}
