/// ### English
/// Lock-free queues used inside the runtime.
///
/// ### 中文
/// 运行时内部使用的无锁队列实现。
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::engine::lockfree::MpscQueue;

use super::command::Command;

/// ### English
/// Command queue used by embedder threads to send control messages to the Servo thread.
///
/// ### 中文
/// 宿主线程向 Servo 线程发送控制消息的命令队列。
pub(super) struct CommandQueue {
    queue: MpscQueue<Command>,
    in_flight: AtomicUsize,
    closed: AtomicBool,
}

impl CommandQueue {
    pub(super) fn new() -> Self {
        Self {
            queue: MpscQueue::new(),
            in_flight: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
        }
    }

    pub(super) fn push(&self, command: Command) {
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        self.in_flight.fetch_add(1, Ordering::Relaxed);
        if self.closed.load(Ordering::Acquire) {
            self.in_flight.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        self.queue.push(command);
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    pub(super) fn pop(&self) -> Option<Command> {
        self.queue.pop()
    }

    pub(super) fn close(&self) {
        self.closed.store(true, Ordering::Release);
        while self.in_flight.load(Ordering::Acquire) != 0 {
            std::hint::spin_loop();
        }
        while let Some(command) = self.queue.pop() {
            drop(command);
        }
    }
}
