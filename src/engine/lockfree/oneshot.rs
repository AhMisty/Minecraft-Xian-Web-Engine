use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// ### English
/// One-shot, single-producer single-consumer (SPSC) value handoff.
///
/// - No locks.
/// - Receiver thread is stored so the sender can `unpark()` it.
///
/// ### 中文
/// 一次性（oneshot）的单生产者/单消费者值传递。
///
/// - 无锁。
/// - 保存接收方线程句柄，发送方在完成后可 `unpark()` 唤醒。
pub(crate) struct OneShot<T> {
    /// ### English
    /// State machine for this oneshot:
    ///
    /// - `0` = empty
    /// - `1` = writing
    /// - `2` = ready
    /// - `3` = taken
    ///
    /// ### 中文
    /// 本 oneshot 的状态机：
    ///
    /// - `0` = 空
    /// - `1` = 写入中
    /// - `2` = 就绪
    /// - `3` = 已取走
    state: AtomicU8,
    /// ### English
    /// Storage for the payload written by the sender and read by the receiver.
    ///
    /// ### 中文
    /// 载荷存储区：由发送方写入、由接收方读取。
    value: UnsafeCell<MaybeUninit<T>>,
    /// ### English
    /// Receiver thread handle used to `unpark()` on send.
    ///
    /// ### 中文
    /// 接收方线程句柄：发送完成后用于 `unpark()` 唤醒。
    waiter: thread::Thread,
}

unsafe impl<T: Send> Send for OneShot<T> {}
unsafe impl<T: Send> Sync for OneShot<T> {}

impl<T> OneShot<T> {
    #[inline]
    pub(crate) fn new(waiter: thread::Thread) -> Self {
        Self {
            state: AtomicU8::new(0),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            waiter,
        }
    }

    /// ### English
    /// Sends the value. Returns `false` if it was already sent.
    ///
    /// ### 中文
    /// 发送值；若已发送过则返回 `false`。
    #[inline]
    pub(crate) fn send(&self, value: T) -> bool {
        if self
            .state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return false;
        }

        unsafe {
            (*self.value.get()).write(value);
        }
        self.state.store(2, Ordering::Release);
        self.waiter.unpark();
        true
    }

    /// ### English
    /// Tries to receive the value.
    ///
    /// ### 中文
    /// 尝试接收值（非阻塞）。
    #[inline]
    pub(crate) fn try_recv(&self) -> Option<T> {
        self.state
            .compare_exchange(2, 3, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| unsafe { (*self.value.get()).assume_init_read() })
    }

    /// ### English
    /// Receives the value with a timeout.
    ///
    /// ### 中文
    /// 在超时时间内等待接收值。
    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Option<T> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(value) = self.try_recv() {
                return Some(value);
            }
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            thread::park_timeout(deadline - now);
        }
    }
}

impl<T> Drop for OneShot<T> {
    fn drop(&mut self) {
        if self.state.load(Ordering::Acquire) == 2 {
            unsafe {
                drop((*self.value.get()).assume_init_read());
            }
        }
    }
}
