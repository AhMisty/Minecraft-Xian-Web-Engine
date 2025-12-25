//! ### English
//! Fast hash map for `u32 -> V` lookups (avoids SipHash overhead).
//!
//! ### 中文
//! `u32 -> V` 的快速 HashMap（避免 SipHash 的开销）。

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// ### English
/// Identity hasher optimized for `u32` keys.
/// We intentionally avoid cryptographic hashing for hot-path lookups.
///
/// ### 中文
/// 为 `u32` key 优化的恒等哈希。
/// 热路径上刻意避免使用加密级别哈希（SipHash）。
#[derive(Default)]
pub(super) struct U32IdentityHasher(
    /// ### English
    /// Accumulated hash value.
    ///
    /// ### 中文
    /// 累积的哈希值。
    u64,
);

impl Hasher for U32IdentityHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = 0u64;
        for chunk in bytes.chunks(8) {
            let mut buf = [0u8; 8];
            buf[..chunk.len()].copy_from_slice(chunk);
            hash ^= u64::from_le_bytes(buf);
        }
        self.0 = hash;
    }

    fn write_u32(&mut self, i: u32) {
        self.0 = i as u64;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// ### English
/// HashMap specialized for `u32` keys using the identity hasher.
///
/// ### 中文
/// 使用恒等哈希的 `u32` key 专用 HashMap。
pub(super) type U32HashMap<V> = HashMap<u32, V, BuildHasherDefault<U32IdentityHasher>>;
