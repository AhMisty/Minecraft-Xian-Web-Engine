/// ### English
/// Lock-free primitives used across the engine (no `Mutex`, no std/crossbeam channels).
///
/// ### 中文
/// 引擎内复用的无锁原语（不使用 `Mutex`，不依赖 std/crossbeam channel）。
mod bounded_mpsc;
mod coalesced;
mod mpsc;
mod oneshot;

pub(crate) use bounded_mpsc::BoundedMpscQueue;
pub(crate) use coalesced::CoalescedBox;
pub(crate) use mpsc::MpscQueue;
pub(crate) use oneshot::OneShot;
