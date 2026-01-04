//! ### English
//! Servo `RefreshDriver` implementations.
//!
//! Supports external-vsync driven refresh (fast path for games) and fixed-interval refresh.
//!
//! ### 中文
//! Servo `RefreshDriver` 的实现。
//!
//! 支持外部 vsync 驱动（游戏场景快路径）与固定间隔刷新。
mod fixed_interval;
mod scheduler;
mod vsync_driver;

pub use fixed_interval::FixedIntervalRefreshDriver;
pub use scheduler::RefreshScheduler;
pub use vsync_driver::VsyncRefreshDriver;
