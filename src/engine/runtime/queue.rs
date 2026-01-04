//! ### English
//! Lock-free queues used inside the runtime.
//!
//! ### 中文
//! 运行时内部使用的无锁队列实现。
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::engine::lockfree::{Backoff, MpscQueue};

use super::command::Command;

/// ### English
/// Command queue used by embedder threads to send control messages to the Servo thread.
///
/// ### 中文
/// 宿主线程向 Servo 线程发送控制消息的命令队列。
pub(super) struct CommandQueue {
    /// ### English
    /// Underlying unbounded MPSC queue.
    ///
    /// ### 中文
    /// 底层无界 MPSC 队列。
    queue: MpscQueue<Command>,
    /// ### English
    /// Number of producers currently publishing into the queue.
    ///
    /// ### 中文
    /// 当前正在向队列发布的生产者数量。
    in_flight: AtomicUsize,
    /// ### English
    /// Close flag used to reject new commands during shutdown.
    ///
    /// ### 中文
    /// 关闭标记：用于在 shutdown 期间拒绝新命令。
    closed: AtomicBool,
}

impl CommandQueue {
    /// ### English
    /// Creates a new open command queue.
    ///
    /// ### 中文
    /// 创建一个处于 open 状态的新命令队列。
    pub(super) fn new() -> Self {
        Self {
            queue: MpscQueue::new(),
            in_flight: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
        }
    }

    /// ### English
    /// Enqueues one command unless the queue is closed.
    ///
    /// This is a best-effort helper that drops the command if the queue is closing; use
    /// [`Self::try_push`] if the caller must observe success/failure.
    ///
    /// #### Parameters
    /// - `command`: Command to push.
    ///
    /// ### 中文
    /// 若队列未关闭，则 push 一个命令。
    ///
    /// 该方法为 best-effort：队列关闭/关闭中时会丢弃命令；若调用方需要感知成功/失败，请使用
    /// [`Self::try_push`]。
    ///
    /// #### 参数
    /// - `command`：要 push 的命令。
    pub(super) fn push(&self, command: Command) {
        let _ = self.try_push(command);
    }

    /// ### English
    /// Tries to push one command; returns `false` if the queue is closed.
    ///
    /// #### Parameters
    /// - `command`: Command to push.
    ///
    /// ### 中文
    /// 尝试 push 一个命令；若队列已关闭则返回 `false`。
    ///
    /// #### 参数
    /// - `command`：要 push 的命令。
    pub(super) fn try_push(&self, command: Command) -> bool {
        if self.closed.load(Ordering::Acquire) {
            return false;
        }
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        if self.closed.load(Ordering::Acquire) {
            self.in_flight.fetch_sub(1, Ordering::Release);
            return false;
        }

        self.queue.push(command);
        self.in_flight.fetch_sub(1, Ordering::Release);
        true
    }

    /// ### English
    /// Pops one command from the queue.
    ///
    /// ### 中文
    /// 从队列 pop 一个命令。
    pub(super) fn pop(&self) -> Option<Command> {
        self.queue.pop()
    }

    /// ### English
    /// Closes the queue and drains any remaining commands.
    ///
    /// This waits for in-flight producers to finish publishing.
    /// The wait uses a short spin-then-yield backoff to avoid burning CPU during shutdown.
    /// While draining, any pending `CreateView` commands are completed with an error to avoid
    /// leaving callers blocked on their one-shot response.
    ///
    /// ### 中文
    /// 关闭队列并 drain 所有剩余命令。
    ///
    /// 该操作会等待正在进行中的生产者完成发布。
    /// 等待过程使用短暂自旋 + `yield` 退避，避免 shutdown 时空转占用 CPU。
    /// drain 过程中会将所有未处理的 `CreateView` 命令用错误回包，以避免调用方卡在 oneshot 等待中。
    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        let mut backoff = Backoff::new();
        while self.in_flight.load(Ordering::Acquire) != 0 {
            backoff.snooze();
        }
        while let Some(command) = self.queue.pop() {
            match command {
                Command::CreateView { response, .. } => {
                    let _ = response.send(Err("Engine is shutting down".to_string()));
                }
                Command::DestroyView { .. } | Command::Shutdown => {}
            }
        }
    }
}
