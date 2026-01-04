//! ### English
//! Lock-free primitives shared across the engine.
//!
//! These utilities are designed for hot paths (atomics, bounded allocation, spin/yield backoff).
//!
//! ### 中文
//! 引擎内复用的无锁原语。
//!
//! 这些工具面向热路径设计（原子操作、有界分配、短自旋/让出调度退避）。
mod backoff;
mod bounded_mpsc;
mod coalesced;
mod mpsc;
mod oneshot;

pub(crate) use backoff::Backoff;
pub(crate) use bounded_mpsc::BoundedMpscQueue;
pub(crate) use coalesced::CoalescedBox;
pub(crate) use mpsc::MpscQueue;
pub(crate) use oneshot::OneShot;
