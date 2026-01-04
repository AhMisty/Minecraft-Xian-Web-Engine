//! ### English
//! Lock-free input queues and coalescing helpers.
//!
//! Designed for minimal overhead between the Java thread(s) and the Servo thread.
//!
//! ### 中文
//! 无锁输入队列与合并（coalescing）工具。
//!
//! 旨在让 Java 线程与 Servo 线程之间的开销最小化。
mod coalesced;
mod queue;

pub use coalesced::{CoalescedMouseMove, CoalescedResize};
pub use queue::InputEventQueue;
