//! ### English
//! Minimal spin-then-yield backoff helper for lock-free hot paths.
//!
//! This is intentionally tiny and `#[inline]` friendly:
//! - Spin briefly to cover short producer/consumer gaps.
//! - Yield after the spin budget to avoid burning CPU on oversubscribed systems.
//!
//! ### 中文
//! 为无锁热路径提供的“短自旋 + 让出调度”退避工具。
//!
//! 该实现刻意保持极小且易于内联：
//! - 先短暂自旋，用于覆盖生产者/消费者之间的短间隙；
//! - 超过自旋预算后调用 `yield`，避免在 CPU 过载时空转占满。

use std::thread;

/// ### English
/// Spin budget before switching to `yield_now()`.
///
/// ### 中文
/// 在切换到 `yield_now()` 之前允许的自旋次数预算。
const SPIN_LIMIT: u32 = 64;

/// ### English
/// Spin-then-yield backoff state.
///
/// ### 中文
/// “短自旋 + 让出调度”的退避状态。
pub(crate) struct Backoff {
    /// ### English
    /// Spin counter used to decide when to yield.
    ///
    /// ### 中文
    /// 自旋计数器，用于决定何时让出调度。
    spins: u32,
}

impl Backoff {
    /// ### English
    /// Creates a new backoff state.
    ///
    /// ### 中文
    /// 创建一个新的退避状态。
    #[inline]
    pub(crate) fn new() -> Self {
        Self { spins: 0 }
    }

    /// ### English
    /// Performs one backoff step.
    ///
    /// ### 中文
    /// 执行一次退避步骤。
    #[inline]
    pub(crate) fn snooze(&mut self) {
        if self.spins < SPIN_LIMIT {
            std::hint::spin_loop();
        } else {
            thread::yield_now();
        }
        self.spins = self.spins.wrapping_add(1);
    }
}
