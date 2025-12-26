//! ### English
//! Servo `RefreshDriver` implementations.
//! Supports external-vsync driven refresh (fast path for games) and fixed-interval refresh.
//!
//! ### 中文
//! Servo `RefreshDriver` 的实现。
//! 支持外部 vsync 驱动刷新（游戏场景快路径）以及固定间隔刷新。

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::rc::Rc;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering as AtomicOrdering},
};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel as channel;
use servo::RefreshDriver;

use crate::engine::vsync::VsyncCallbackQueue;

struct ScheduledTask {
    /// ### English
    /// Target time when the callback should run.
    ///
    /// ### 中文
    /// 回调应执行的目标时间。
    deadline: Instant,
    /// ### English
    /// Monotonic sequence used as a tiebreaker in the heap.
    ///
    /// ### 中文
    /// 在堆中用作平局判定的单调序号。
    seq: u64,
    /// ### English
    /// Callback to execute on the scheduler thread.
    ///
    /// ### 中文
    /// 在调度线程中执行的回调。
    callback: Box<dyn Fn() + Send + 'static>,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.seq == other.seq
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.deadline.cmp(&self.deadline) {
            Ordering::Equal => other.seq.cmp(&self.seq),
            ord => ord,
        }
    }
}

enum SchedulerMsg {
    Schedule(ScheduledTask),
    Shutdown,
}

/// ### English
/// Global scheduler for fixed-interval refresh (avoids per-view timer threads).
///
/// ### 中文
/// 固定间隔 refresh 的全局调度器（避免每个 view 单独创建计时线程）。
pub struct RefreshScheduler {
    /// ### English
    /// Channel sender into the global scheduler thread.
    ///
    /// ### 中文
    /// 向全局调度线程发送任务的 channel sender。
    tx: channel::Sender<SchedulerMsg>,
    /// ### English
    /// Monotonic task sequence generator.
    ///
    /// ### 中文
    /// 单调递增的任务序号生成器。
    next_seq: AtomicU64,
    /// ### English
    /// Join handle for the scheduler thread (taken on Drop for clean shutdown).
    ///
    /// ### 中文
    /// 调度线程的 JoinHandle，Drop 时获取以便干净退出。
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl RefreshScheduler {
    /// ### English
    /// Creates a scheduler backed by a single worker thread.
    ///
    /// ### 中文
    /// 创建一个由单线程驱动的调度器。
    pub fn new() -> Arc<Self> {
        let (tx, rx) = channel::unbounded::<SchedulerMsg>();
        let thread = thread::Builder::new()
            .name("XianRefreshDriver".to_string())
            .spawn(move || run_scheduler(rx))
            .expect("failed to spawn refresh scheduler thread");

        Arc::new(RefreshScheduler {
            tx,
            next_seq: AtomicU64::new(1),
            thread: Mutex::new(Some(thread)),
        })
    }

    /// ### English
    /// Schedules one callback to run after `delay`.
    ///
    /// ### 中文
    /// 计划在 `delay` 之后执行一个回调。
    pub fn schedule(&self, delay: Duration, callback: Box<dyn Fn() + Send + 'static>) {
        let seq = self.next_seq.fetch_add(1, AtomicOrdering::Relaxed);
        let task = ScheduledTask {
            deadline: Instant::now() + delay,
            seq,
            callback,
        };
        let _ = self.tx.send(SchedulerMsg::Schedule(task));
    }
}

impl Drop for RefreshScheduler {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.lock().expect("lock poisoned").take() {
            let _ = self.tx.send(SchedulerMsg::Shutdown);
            let _ = thread.join();
        }
    }
}

fn run_scheduler(rx: channel::Receiver<SchedulerMsg>) {
    let mut queue: BinaryHeap<ScheduledTask> = BinaryHeap::new();

    loop {
        while let Some(next) = queue.peek() {
            let now = Instant::now();
            if next.deadline > now {
                break;
            }

            let task = queue.pop().expect("queue had a peeked item");
            (task.callback)();
        }

        let timeout = queue
            .peek()
            .map(|task| task.deadline.saturating_duration_since(Instant::now()));

        let msg = match timeout {
            Some(timeout) => match rx.recv_timeout(timeout) {
                Ok(msg) => Some(msg),
                Err(channel::RecvTimeoutError::Timeout) => None,
                Err(channel::RecvTimeoutError::Disconnected) => return,
            },
            None => match rx.recv() {
                Ok(msg) => Some(msg),
                Err(channel::RecvError) => return,
            },
        };

        let Some(msg) = msg else {
            continue;
        };

        match msg {
            SchedulerMsg::Schedule(task) => queue.push(task),
            SchedulerMsg::Shutdown => return,
        }
    }
}

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
        })
    }
}

impl RefreshDriver for FixedIntervalRefreshDriver {
    fn observe_next_frame(&self, start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.scheduler
            .schedule(self.frame_duration, start_frame_callback);
    }
}

/// ### English
/// Refresh driver driven by an external vsync tick (Java side).
///
/// ### 中文
/// 由外部 vsync tick（Java 侧）驱动的 refresh driver。
pub struct VsyncRefreshDriver {
    /// ### English
    /// Shared vsync callback queue drained by Java-side tick.
    ///
    /// ### 中文
    /// 由 Java 侧 tick drain 的共享 vsync 回调队列。
    queue: Arc<VsyncCallbackQueue>,
}

impl VsyncRefreshDriver {
    /// ### English
    /// Creates a vsync-driven refresh driver.
    ///
    /// ### 中文
    /// 创建由 vsync 驱动的 refresh driver。
    pub fn new(queue: Arc<VsyncCallbackQueue>) -> Rc<Self> {
        Rc::new(Self { queue })
    }
}

impl RefreshDriver for VsyncRefreshDriver {
    fn observe_next_frame(&self, start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.queue.enqueue(start_frame_callback);
    }
}
