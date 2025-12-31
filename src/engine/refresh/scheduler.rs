use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::thread;
use std::time::{Duration, Instant};

use crate::engine::lockfree::MpscQueue;

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
    /// Lock-free message queue into the global scheduler thread.
    ///
    /// ### 中文
    /// 发送任务到全局调度线程的无锁消息队列。
    queue: Arc<MpscQueue<SchedulerMsg>>,
    /// ### English
    /// Monotonic task sequence generator.
    ///
    /// ### 中文
    /// 单调递增的任务序号生成器。
    next_seq: AtomicU64,
    /// ### English
    /// Scheduler thread handle (for `unpark`).
    ///
    /// ### 中文
    /// 调度线程句柄（用于 `unpark`）。
    thread: thread::Thread,
    /// ### English
    /// Join handle for a clean shutdown on drop.
    ///
    /// ### 中文
    /// Drop 时用于干净退出的 JoinHandle。
    join: Option<thread::JoinHandle<()>>,
}

impl RefreshScheduler {
    /// ### English
    /// Creates a scheduler backed by a single worker thread.
    ///
    /// ### 中文
    /// 创建一个由单线程驱动的调度器。
    pub fn new() -> Arc<Self> {
        let queue = Arc::new(MpscQueue::new());
        let queue_for_thread = queue.clone();
        let join = thread::Builder::new()
            .name("XianRefreshDriver".to_string())
            .spawn(move || run_scheduler(queue_for_thread))
            .expect("failed to spawn refresh scheduler thread");
        let thread_handle = join.thread().clone();

        Arc::new(Self {
            queue,
            next_seq: AtomicU64::new(1),
            thread: thread_handle,
            join: Some(join),
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
        self.queue.push(SchedulerMsg::Schedule(task));
        self.thread.unpark();
    }
}

impl Drop for RefreshScheduler {
    fn drop(&mut self) {
        self.queue.push(SchedulerMsg::Shutdown);
        self.thread.unpark();
        if let Some(join) = self.join.take()
            && thread::current().id() != self.thread.id()
        {
            let _ = join.join();
        }
    }
}

fn run_scheduler(rx: Arc<MpscQueue<SchedulerMsg>>) {
    let mut queue: BinaryHeap<ScheduledTask> = BinaryHeap::new();

    loop {
        while let Some(msg) = rx.pop() {
            match msg {
                SchedulerMsg::Schedule(task) => queue.push(task),
                SchedulerMsg::Shutdown => return,
            }
        }

        let now = Instant::now();
        while let Some(next) = queue.peek() {
            if next.deadline > now {
                break;
            }
            let task = queue.pop().expect("queue had a peeked item");
            (task.callback)();
        }

        let timeout = queue
            .peek()
            .map(|task| task.deadline.saturating_duration_since(Instant::now()));
        match timeout {
            Some(timeout) => thread::park_timeout(timeout),
            None => thread::park(),
        }
    }
}
