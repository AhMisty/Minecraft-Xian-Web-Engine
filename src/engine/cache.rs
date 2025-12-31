/// ### English
/// Cache-line sized padding helpers shared by lock-free structures in this crate.
///
/// ### 中文
/// 本 crate 内无锁结构共用的 cache line padding 工具。
use std::sync::atomic::AtomicUsize;

/// ### English
/// The cache line size we optimize for (bytes).
///
/// ### 中文
/// 作为优化目标的 cache line 大小（字节）。
pub(crate) const CACHE_LINE_BYTES: usize = 64;

/// ### English
/// Padding bytes needed to separate two `AtomicUsize` fields onto different cache lines when the
/// struct is `#[repr(align(64))]` / `#[repr(C, align(64))]`.
///
/// ### 中文
/// 当结构体采用 `#[repr(align(64))]` / `#[repr(C, align(64))]` 时，用于将两个 `AtomicUsize`
/// 字段隔离到不同 cache line 的 padding 字节数。
pub(crate) const CACHE_PAD_BYTES: usize = CACHE_LINE_BYTES - std::mem::size_of::<AtomicUsize>();
