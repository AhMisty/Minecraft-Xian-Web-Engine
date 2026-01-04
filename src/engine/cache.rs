//! ### English
//! Cache-line sized padding helpers shared by lock-free structures in this crate.
//!
//! ### 中文
//! 本 crate 内无锁结构共用的 cache line padding 工具。

/// ### English
/// The cache line size we optimize for (bytes).
///
/// ### 中文
/// 作为优化目标的 cache line 大小（字节）。
pub(crate) const CACHE_LINE_BYTES: usize = 64;

/// ### English
/// Returns the padding bytes needed to advance to the next cache-line boundary.
///
/// This is intended to be used with `#[repr(align(64))]` / `#[repr(C, align(64))]` structs to
/// separate frequently-contended fields and reduce false sharing.
///
/// #### Parameters
/// - `bytes_used`: Number of bytes already occupied by preceding fields.
///
/// ### 中文
/// 返回将偏移推进到下一个 cache line 边界所需的 padding 字节数。
///
/// 该函数通常配合 `#[repr(align(64))]` / `#[repr(C, align(64))]` 结构体使用，用于隔离争用字段并降低伪共享。
///
/// #### 参数
/// - `bytes_used`：前置字段已占用的字节数。
#[inline]
pub(crate) const fn pad_to_cache_line(bytes_used: usize) -> usize {
    let rem = bytes_used % CACHE_LINE_BYTES;
    if rem == 0 { 0 } else { CACHE_LINE_BYTES - rem }
}

/// ### English
/// Returns the padding bytes needed after a single field of type `T` to reach the next cache line.
///
/// ### 中文
/// 返回在单个 `T` 字段之后推进到下一个 cache line 所需的 padding 字节数。
#[inline]
pub(crate) const fn pad_after<T>() -> usize {
    pad_to_cache_line(std::mem::size_of::<T>())
}

/// ### English
/// Returns the padding bytes needed after two fields (`A` then `B`) to reach the next cache line.
///
/// ### 中文
/// 返回在两个字段（先 `A` 后 `B`）之后推进到下一个 cache line 所需的 padding 字节数。
#[inline]
pub(crate) const fn pad_after2<A, B>() -> usize {
    pad_to_cache_line(std::mem::size_of::<A>() + std::mem::size_of::<B>())
}

/// ### English
/// Returns the padding bytes needed after three fields (`A`, `B`, `C`) to reach the next cache line.
///
/// ### 中文
/// 返回在三个字段（`A`、`B`、`C`）之后推进到下一个 cache line 所需的 padding 字节数。
#[inline]
pub(crate) const fn pad_after3<A, B, C>() -> usize {
    pad_to_cache_line(
        std::mem::size_of::<A>() + std::mem::size_of::<B>() + std::mem::size_of::<C>(),
    )
}
