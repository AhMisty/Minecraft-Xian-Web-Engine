//! ### English
//! Fixed-interval refresh driver implementation (`target_fps != 0`).
//!
//! ### 中文
//! 固定间隔 refresh driver 实现（`target_fps != 0`）。

use std::rc::Rc;
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering as AtomicOrdering},
};
use std::time::Duration;

use servo::RefreshDriver;

use crate::engine::lockfree::CoalescedBox;

use super::scheduler::{RefreshScheduler, SchedulerRunnable};

/// ### English
/// Refresh driver that ticks at a fixed interval (`target_fps != 0` path).
///
/// ### 中文
/// 固定间隔的 refresh driver（`target_fps != 0` 路径）。
pub struct FixedIntervalRefreshDriver {
    /// ### English
    /// Coalesced state that keeps only the latest callback and limits scheduling to one tick.
    ///
    /// ### 中文
    /// 合并状态：只保留最新回调，并把调度限制为最多一个 tick。
    coalesced: Arc<FixedIntervalCoalesced>,
}

impl FixedIntervalRefreshDriver {
    /// ### English
    /// Creates a fixed-interval refresh driver.
    ///
    /// #### Parameters
    /// - `scheduler`: Shared refresh scheduler used to run ticks.
    /// - `frame_duration`: Fixed interval between ticks.
    ///
    /// ### 中文
    /// 创建固定间隔 refresh driver。
    ///
    /// #### 参数
    /// - `scheduler`：用于执行 tick 的共享调度器。
    /// - `frame_duration`：tick 的固定时间间隔。
    pub fn new(scheduler: Arc<RefreshScheduler>, frame_duration: Duration) -> Rc<Self> {
        Rc::new(Self {
            coalesced: Arc::new(FixedIntervalCoalesced::new(scheduler, frame_duration)),
        })
    }
}

impl RefreshDriver for FixedIntervalRefreshDriver {
    /// ### English
    /// Requests that the next frame be observed; the callback is coalesced and executed on a
    /// fixed interval.
    ///
    /// #### Parameters
    /// - `start_frame_callback`: Callback executed on the next scheduled tick (latest wins).
    ///
    /// ### 中文
    /// 请求观察下一帧；该回调会被合并，并按固定间隔执行。
    ///
    /// #### 参数
    /// - `start_frame_callback`：下一次 tick 执行的回调（latest-wins）。
    fn observe_next_frame(&self, start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.coalesced.submit(start_frame_callback);
    }
}

/// ### English
/// Boxed callback wrapper stored in `CoalescedBox` to avoid repeated allocations.
///
/// ### 中文
/// 存储在 `CoalescedBox` 中的 boxed 回调封装，用于避免重复分配。
struct FrameCallbackNode {
    /// ### English
    /// Stored callback payload (taken and executed on tick).
    ///
    /// ### 中文
    /// 存储的回调载荷（tick 时取出并执行）。
    callback: Option<Box<dyn Fn() + Send + 'static>>,
}

impl FrameCallbackNode {
    /// ### English
    /// Creates an empty callback node.
    ///
    /// ### 中文
    /// 创建一个空的回调节点。
    #[inline]
    fn new() -> Self {
        Self { callback: None }
    }
}

/// ### English
/// Coalescer that schedules at most one outstanding tick and keeps only the latest callback.
///
/// ### 中文
/// 合并器：最多只安排一个未完成的 tick，并仅保留最新回调。
struct FixedIntervalCoalesced {
    /// ### English
    /// Latest callback payload (latest-wins).
    ///
    /// ### 中文
    /// 最新回调载荷（latest-wins）。
    callback: CoalescedBox<FrameCallbackNode>,
    /// ### English
    /// Whether a tick is already scheduled.
    ///
    /// ### 中文
    /// 是否已经安排了一个 tick。
    scheduled: AtomicBool,
    /// ### English
    /// Scheduler handle stored as a weak ref to avoid `scheduler -> task -> state -> scheduler`
    /// cycles while tasks are pending in the scheduler.
    ///
    /// ### 中文
    /// 调度器句柄（弱引用）：用于避免任务挂起时形成 `scheduler -> task -> state -> scheduler` 的强引用环。
    scheduler: Weak<RefreshScheduler>,
    /// ### English
    /// Fixed delay between ticks.
    ///
    /// ### 中文
    /// tick 之间的固定延迟。
    delay: Duration,
}

impl FixedIntervalCoalesced {
    /// ### English
    /// Creates an empty coalescer.
    ///
    /// ### 中文
    /// 创建一个空的合并器。
    #[inline]
    fn new(scheduler: Arc<RefreshScheduler>, delay: Duration) -> Self {
        Self {
            callback: CoalescedBox::default(),
            scheduled: AtomicBool::new(false),
            scheduler: Arc::downgrade(&scheduler),
            delay,
        }
    }

    /// ### English
    /// Stores the latest callback into the coalescer, recycling any previous node.
    ///
    /// #### Parameters
    /// - `callback`: Callback to store (latest wins).
    ///
    /// ### 中文
    /// 写入最新回调，并回收之前的节点。
    ///
    /// #### 参数
    /// - `callback`：要写入的回调（latest-wins）。
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

    /// ### English
    /// Takes the current callback if present, recycling the node.
    ///
    /// ### 中文
    /// 若存在回调则取出，并回收节点。
    #[inline]
    fn take_callback(&self) -> Option<Box<dyn Fn() + Send + 'static>> {
        let mut node = self.callback.take()?;
        let callback = node.callback.take();
        self.callback.push_free(node);
        callback
    }

    /// ### English
    /// Recycles a node by clearing and pushing it into the free cache.
    ///
    /// #### Parameters
    /// - `node`: Node to recycle.
    ///
    /// ### 中文
    /// 清空并回收节点到 free cache。
    ///
    /// #### 参数
    /// - `node`：要回收的节点。
    #[inline]
    fn recycle_node(&self, mut node: Box<FrameCallbackNode>) {
        node.callback.take();
        self.callback.push_free(node);
    }

    /// ### English
    /// Submits a callback and schedules a tick if none is currently scheduled.
    ///
    /// #### Parameters
    /// - `callback`: Callback to coalesce (latest wins).
    ///
    /// ### 中文
    /// 提交一个回调；若当前没有已安排的 tick，则安排一次 tick。
    ///
    /// #### 参数
    /// - `callback`：要合并的回调（latest-wins）。
    #[inline]
    fn submit(self: &Arc<Self>, callback: Box<dyn Fn() + Send + 'static>) {
        self.set_callback(callback);
        if self.scheduled.swap(true, AtomicOrdering::AcqRel) {
            return;
        }

        let Some(scheduler) = self.scheduler.upgrade() else {
            self.clear_pending();
            return;
        };

        scheduler.schedule(self.delay, self.clone());
    }

    /// ### English
    /// Drops any pending callback and clears the scheduling state.
    ///
    /// This is used when the backing scheduler has been dropped, so ticks can no longer run.
    ///
    /// ### 中文
    /// 丢弃所有待处理回调并清除调度状态。
    ///
    /// 当底层 scheduler 已被 drop（无法再执行 tick）时使用。
    #[inline]
    fn clear_pending(&self) {
        self.scheduled.store(false, AtomicOrdering::Release);
        let _ = self.take_callback();
    }

    /// ### English
    /// Executes the coalesced callback and re-arms scheduling if another callback arrives during
    /// execution.
    ///
    /// The `scheduled` flag is cleared before running the callback so `observe_next_frame()` can
    /// schedule the next tick from within the callback without extra coordination.
    ///
    /// Threading: this method runs on the `RefreshScheduler` worker thread.
    ///
    /// ### 中文
    /// 执行合并后的回调；若回调执行期间又有新回调提交，则会重新 arm 调度。
    ///
    /// 在执行回调之前清除 `scheduled` 标记，使回调内部的 `observe_next_frame()` 可直接安排下一次 tick。
    ///
    /// 线程：该方法在 `RefreshScheduler` 工作线程中执行。
    fn tick(self: Arc<Self>) {
        let callback = self.take_callback();

        self.scheduled.store(false, AtomicOrdering::Release);

        if let Some(callback) = callback {
            callback();
        }

        if !self.callback.is_pending() {
            return;
        }
        if self.scheduled.swap(true, AtomicOrdering::AcqRel) {
            return;
        }
        let Some(scheduler) = self.scheduler.upgrade() else {
            self.clear_pending();
            return;
        };
        scheduler.schedule(self.delay, self.clone());
    }
}

impl SchedulerRunnable for FixedIntervalCoalesced {
    fn run(self: Arc<Self>) {
        self.tick();
    }
}
