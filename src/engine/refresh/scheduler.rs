//! ### English
//! Global scheduler backing fixed-interval refresh (single worker thread).
//!
//! ### 中文
//! 固定间隔 refresh 的全局调度器（单工作线程）。

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::thread;
use std::time::{Duration, Instant};

use crate::engine::lockfree::{BoundedMpscQueue, MpscQueue};

/// ### English
/// Runnable task scheduled onto the refresh scheduler thread.
///
/// Implementations should be cheap to clone (typically via `Arc`) and must be thread-safe.
///
/// ### 中文
/// 可在 refresh 调度线程执行的任务。
///
/// 实现应当易于克隆（通常使用 `Arc`），并且必须线程安全。
pub(crate) trait SchedulerRunnable: Send + Sync + 'static {
    /// ### English
    /// Runs the task on the scheduler thread.
    ///
    /// ### 中文
    /// 在调度线程执行任务。
    fn run(self: Arc<Self>);
}

/// ### English
/// Hot-path ring capacity for the scheduler queue (power-of-two).
///
/// ### 中文
/// 调度器队列热路径 ring 的容量（2 的幂）。
const SCHEDULER_RING_CAPACITY: usize = 8192;

/// ### English
/// Type-erased `Arc<T>` task stored inside the scheduler heap/queues.
///
/// This avoids per-tick allocations by keeping the task payload owned by an `Arc`, and stores only
/// a raw pointer + monomorphized function pointers in the hot path.
///
/// ### 中文
/// 调度器堆/队列中存储的“类型擦除 `Arc<T>`”任务。
///
/// 该实现通过让任务载荷由 `Arc` 持有，热路径只保存原始指针与（单态化的）函数指针，从而避免每 tick 分配。
struct ErasedArcTask {
    ptr: *const (),
    run: unsafe fn(*const ()),
    drop: unsafe fn(*const ()),
}

unsafe impl Send for ErasedArcTask {}
unsafe impl Sync for ErasedArcTask {}

impl ErasedArcTask {
    #[inline]
    fn from_arc<T>(task: Arc<T>) -> Self
    where
        T: SchedulerRunnable,
    {
        Self {
            ptr: Arc::into_raw(task).cast::<()>(),
            run: run_arc_task::<T>,
            drop: drop_arc_task::<T>,
        }
    }

    #[inline]
    fn run(self) {
        let ptr = self.ptr;
        let run = self.run;
        std::mem::forget(self);
        unsafe {
            run(ptr);
        }
    }
}

impl Drop for ErasedArcTask {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.ptr);
        }
    }
}

unsafe fn run_arc_task<T>(ptr: *const ())
where
    T: SchedulerRunnable,
{
    let task = unsafe { Arc::from_raw(ptr.cast::<T>()) };
    <T as SchedulerRunnable>::run(task);
}

unsafe fn drop_arc_task<T>(ptr: *const ())
where
    T: SchedulerRunnable,
{
    unsafe {
        drop(Arc::from_raw(ptr.cast::<T>()));
    }
}

/// ### English
/// One scheduled task stored in the internal priority queue.
///
/// `BinaryHeap` is a max-heap, so we reverse the ordering in `Ord` to pop the earliest deadline.
///
/// ### 中文
/// 存储在内部优先队列中的单个调度任务。
///
/// `BinaryHeap` 是最大堆，因此在 `Ord` 中反转排序以便弹出最早的 deadline。
struct ScheduledTask {
    /// ### English
    /// Target time when the task should run.
    ///
    /// ### 中文
    /// 任务应执行的目标时间。
    deadline: Instant,
    /// ### English
    /// Monotonic sequence used as a tiebreaker in the heap.
    ///
    /// ### 中文
    /// 在堆中用作平局判定的单调序号。
    seq: u64,
    /// ### English
    /// Task payload to execute on the scheduler thread.
    ///
    /// ### 中文
    /// 在调度线程中执行的任务载荷。
    task: ErasedArcTask,
}

impl PartialEq for ScheduledTask {
    /// ### English
    /// Equality is based on `(deadline, seq)` so the heap ordering remains deterministic.
    ///
    /// #### Parameters
    /// - `other`: Value to compare against.
    ///
    /// ### 中文
    /// 以 `(deadline, seq)` 判等，以保证堆排序行为确定。
    ///
    /// #### 参数
    /// - `other`：用于比较的另一个值。
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.seq == other.seq
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    /// ### English
    /// Delegates ordering to `Ord` (total order for heap usage).
    ///
    /// #### Parameters
    /// - `other`: Value to compare against.
    ///
    /// ### 中文
    /// 将排序委托给 `Ord`（提供堆所需的全序）。
    ///
    /// #### 参数
    /// - `other`：用于比较的另一个值。
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    /// ### English
    /// Reversed ordering so earlier deadlines have higher priority in a max-heap.
    ///
    /// #### Parameters
    /// - `other`: Value to compare against.
    ///
    /// ### 中文
    /// 反转排序：在最大堆中让更早的 deadline 拥有更高优先级。
    ///
    /// #### 参数
    /// - `other`：用于比较的另一个值。
    fn cmp(&self, other: &Self) -> Ordering {
        match other.deadline.cmp(&self.deadline) {
            Ordering::Equal => other.seq.cmp(&self.seq),
            ord => ord,
        }
    }
}

/// ### English
/// Hybrid lock-free queue used by the scheduler thread:
/// - Hot path: bounded ring (no allocations).
/// - Cold path: unbounded MPSC overflow (allocates only on rare bursts).
///
/// ### 中文
/// 调度器使用的混合无锁队列：
/// - 热路径：有界 ring（无分配）。
/// - 冷路径：无界 MPSC 溢出队列（仅在少量突发时分配）。
struct SchedulerQueue {
    /// ### English
    /// Hot-path bounded ring buffer (no allocations when not full).
    ///
    /// ### 中文
    /// 热路径：有界 ring buffer（未满时无分配）。
    ring: BoundedMpscQueue<ScheduledTask>,
    /// ### English
    /// Cold-path unbounded overflow queue (used when the ring is full).
    ///
    /// ### 中文
    /// 冷路径：无界溢出队列（ring 满时使用）。
    overflow: MpscQueue<ScheduledTask>,
}

impl SchedulerQueue {
    #[inline]
    fn new() -> Self {
        Self {
            ring: BoundedMpscQueue::with_capacity(SCHEDULER_RING_CAPACITY),
            overflow: MpscQueue::new(),
        }
    }

    #[inline]
    fn push(&self, task: ScheduledTask) {
        match self.ring.try_push(task) {
            Ok(()) => {}
            Err(task) => self.overflow.push(task),
        }
    }

    #[inline]
    fn pop(&self) -> Option<ScheduledTask> {
        self.ring.pop().or_else(|| self.overflow.pop())
    }
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
    queue: Arc<SchedulerQueue>,
    /// ### English
    /// Coalesced wake flag to avoid unpark storms on bursts of scheduling.
    ///
    /// ### 中文
    /// 合并唤醒标记：用于避免调度突发时的频繁 unpark。
    wake_pending: Arc<AtomicBool>,
    /// ### English
    /// Shutdown flag shared with the scheduler thread.
    ///
    /// ### 中文
    /// 与调度线程共享的 shutdown 标记。
    shutdown: Arc<AtomicBool>,
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
    /// Returns an error if the worker thread cannot be spawned.
    ///
    /// ### 中文
    /// 创建一个由单线程驱动的调度器。
    ///
    /// 若无法创建工作线程则返回错误。
    pub fn try_new() -> Result<Arc<Self>, String> {
        let queue = Arc::new(SchedulerQueue::new());
        let wake_pending = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let queue_thread = queue.clone();
        let wake_pending_thread = wake_pending.clone();
        let shutdown_thread = shutdown.clone();
        let join = thread::Builder::new()
            .name("XianRefreshThread".to_string())
            .spawn(move || run_scheduler(queue_thread, wake_pending_thread, shutdown_thread))
            .map_err(|err| format!("Failed to spawn refresh thread: {err}"))?;
        let thread_handle = join.thread().clone();

        Ok(Arc::new(Self {
            queue,
            wake_pending,
            shutdown,
            next_seq: AtomicU64::new(1),
            thread: thread_handle,
            join: Some(join),
        }))
    }

    /// ### English
    /// Schedules one callback to run after `delay`.
    ///
    /// #### Parameters
    /// - `delay`: Delay before running the callback.
    /// - `task`: Task executed on the scheduler thread.
    ///
    /// ### 中文
    /// 计划在 `delay` 之后执行一个任务。
    ///
    /// #### 参数
    /// - `delay`：任务执行前的延迟时间。
    /// - `task`：在调度线程执行的任务。
    pub fn schedule<T>(&self, delay: Duration, task: Arc<T>)
    where
        T: SchedulerRunnable,
    {
        let seq = self.next_seq.fetch_add(1, AtomicOrdering::Relaxed);
        let task = ScheduledTask {
            deadline: Instant::now() + delay,
            seq,
            task: ErasedArcTask::from_arc(task),
        };
        self.queue.push(task);
        if !self.wake_pending.swap(true, AtomicOrdering::Release) {
            self.thread.unpark();
        }
    }
}

impl Drop for RefreshScheduler {
    /// ### English
    /// Requests shutdown and joins the scheduler thread when dropping on a different thread.
    ///
    /// ### 中文
    /// drop 时请求调度线程退出；若不在同一线程 drop，则等待 join。
    fn drop(&mut self) {
        self.shutdown.store(true, AtomicOrdering::Release);
        self.thread.unpark();
        if let Some(join) = self.join.take()
            && thread::current().id() != self.thread.id()
        {
            let _ = join.join();
        }
    }
}

/// ### English
/// Scheduler thread main loop.
///
/// #### Parameters
/// - `inbox`: Lock-free message queue into the scheduler thread.
/// - `wake_pending`: Coalesced wake flag shared with the producers.
/// - `shutdown`: Shutdown flag shared with the producers.
///
/// ### 中文
/// 调度线程主循环。
///
/// #### 参数
/// - `inbox`：发送到调度线程的无锁消息队列。
/// - `wake_pending`：与生产者共享的合并唤醒标记。
/// - `shutdown`：与生产者共享的 shutdown 标记。
fn run_scheduler(
    inbox: Arc<SchedulerQueue>,
    wake_pending: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
) {
    let mut heap: BinaryHeap<ScheduledTask> = BinaryHeap::new();

    loop {
        while let Some(task) = inbox.pop() {
            heap.push(task);
        }
        if shutdown.load(AtomicOrdering::Acquire) {
            return;
        }

        let now = Instant::now();
        while let Some(next) = heap.peek() {
            if next.deadline > now {
                break;
            }
            let Some(task) = heap.pop() else {
                break;
            };
            task.task.run();
            if shutdown.load(AtomicOrdering::Acquire) {
                return;
            }
        }

        if shutdown.load(AtomicOrdering::Acquire) {
            return;
        }

        if wake_pending.swap(false, AtomicOrdering::Acquire) {
            continue;
        }

        let now = Instant::now();
        let timeout = heap
            .peek()
            .map(|task| task.deadline.saturating_duration_since(now));
        match timeout {
            Some(Duration::ZERO) => continue,
            Some(timeout) => thread::park_timeout(timeout),
            None => thread::park(),
        }
    }
}
