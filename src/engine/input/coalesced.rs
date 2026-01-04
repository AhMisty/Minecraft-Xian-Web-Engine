//! ### English
//! Coalesced input state helpers (latest-wins).
//!
//! ### 中文
//! 输入状态合并（latest-wins）工具。

use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use dpi::PhysicalSize;

#[repr(C, align(64))]
/// ### English
/// Coalesced mouse-move state: keeps only the latest `(x, y)` until the Servo thread drains it.
///
/// ### 中文
/// 鼠标移动合并状态：只保留最新的 `(x, y)`，等待 Servo 线程 drain。
pub struct CoalescedMouseMove {
    /// ### English
    /// Pending flag (`0` = no pending move, `1` = pending).
    ///
    /// ### 中文
    /// pending 标记（`0` = 无待处理移动，`1` = 有待处理移动）。
    pending: AtomicU8,
    /// ### English
    /// Padding to keep `packed_pos` on a separate cache line from unrelated atomics.
    ///
    /// ### 中文
    /// 填充：让 `packed_pos` 与其它原子尽量避免同一缓存行（降低伪共享）。
    _padding: [u8; 7],
    /// ### English
    /// Packed `(x, y)` mouse position as two `f32` bit patterns.
    ///
    /// ### 中文
    /// 将 `(x, y)` 鼠标位置以两个 `f32` 的 bit pattern 打包到一个 `u64` 中。
    packed_pos: AtomicU64,
}

impl Default for CoalescedMouseMove {
    /// ### English
    /// Creates an empty mouse-move coalescer.
    ///
    /// ### 中文
    /// 创建一个空的鼠标移动合并器。
    fn default() -> Self {
        Self {
            pending: AtomicU8::new(0),
            _padding: [0; 7],
            packed_pos: AtomicU64::new(0),
        }
    }
}

impl CoalescedMouseMove {
    /// ### English
    /// Stores the latest mouse position and marks it pending.
    /// Returns `true` if this call transitions from "not pending" to "pending".
    ///
    /// #### Parameters
    /// - `x`: X position in device pixels (f32).
    /// - `y`: Y position in device pixels (f32).
    ///
    /// ### 中文
    /// 写入最新鼠标位置并标记为 pending。
    /// 若本次调用把状态从“非 pending”切换为“pending”，则返回 `true`。
    ///
    /// #### 参数
    /// - `x`：设备像素坐标 X（f32）。
    /// - `y`：设备像素坐标 Y（f32）。
    pub fn set(&self, x: f32, y: f32) -> bool {
        self.packed_pos.store(pack_f32x2(x, y), Ordering::Relaxed);
        self.pending.swap(1, Ordering::Release) == 0
    }

    /// ### English
    /// Takes the latest mouse position if pending.
    ///
    /// ### 中文
    /// 若处于 pending，则取出最新鼠标位置。
    pub fn take(&self) -> Option<(f32, f32)> {
        if self.pending.swap(0, Ordering::Acquire) == 0 {
            return None;
        }
        let packed = self.packed_pos.load(Ordering::Relaxed);
        Some(unpack_f32x2(packed))
    }
}

#[inline]
/// ### English
/// Packs two `f32` values into a single `u64` using their IEEE-754 bit patterns.
///
/// #### Parameters
/// - `x`: Low 32-bit lane (`f32` bits).
/// - `y`: High 32-bit lane (`f32` bits).
///
/// ### 中文
/// 使用 IEEE-754 bit pattern 将两个 `f32` 打包为一个 `u64`。
///
/// #### 参数
/// - `x`：低 32 位通道（`f32` bit）。
/// - `y`：高 32 位通道（`f32` bit）。
fn pack_f32x2(x: f32, y: f32) -> u64 {
    (x.to_bits() as u64) | ((y.to_bits() as u64) << 32)
}

#[inline]
/// ### English
/// Unpacks a `u64` produced by `pack_f32x2` back into two `f32` values.
///
/// #### Parameters
/// - `packed`: Value returned by `pack_f32x2`.
///
/// ### 中文
/// 将 `pack_f32x2` 产生的 `u64` 解包为两个 `f32`。
///
/// #### 参数
/// - `packed`：由 `pack_f32x2` 产生的值。
fn unpack_f32x2(packed: u64) -> (f32, f32) {
    let x = (packed & 0xFFFF_FFFF) as u32;
    let y = (packed >> 32) as u32;
    (f32::from_bits(x), f32::from_bits(y))
}

#[repr(C, align(64))]
/// ### English
/// Coalesced resize state: keeps only the latest `(width, height)` until the Servo thread drains it.
///
/// ### 中文
/// resize 合并状态：只保留最新的 `(width, height)`，等待 Servo 线程 drain。
pub struct CoalescedResize {
    /// ### English
    /// Pending flag (`0` = no pending resize, `1` = pending).
    ///
    /// ### 中文
    /// pending 标记（`0` = 无待处理 resize，`1` = 有待处理 resize）。
    pending: AtomicU8,
    /// ### English
    /// Padding to keep `packed_size` on a separate cache line from unrelated atomics.
    ///
    /// ### 中文
    /// 填充：让 `packed_size` 与无关原子尽量不共用 cache line（降低伪共享）。
    _padding: [u8; 7],
    /// ### English
    /// Packed `(width, height)` as two `u32` values.
    ///
    /// ### 中文
    /// 将 `(width, height)` 以两个 `u32` 打包到一个 `u64` 中。
    packed_size: AtomicU64,
}

impl Default for CoalescedResize {
    /// ### English
    /// Creates an empty resize coalescer.
    ///
    /// ### 中文
    /// 创建一个空的 resize 合并器。
    fn default() -> Self {
        Self {
            pending: AtomicU8::new(0),
            _padding: [0; 7],
            packed_size: AtomicU64::new(0),
        }
    }
}

impl CoalescedResize {
    /// ### English
    /// Stores the latest size and marks it pending.
    /// Returns `true` if this call transitions from "not pending" to "pending".
    ///
    /// #### Parameters
    /// - `width`: Width in pixels.
    /// - `height`: Height in pixels.
    ///
    /// ### 中文
    /// 写入最新尺寸并标记为 pending。
    /// 若本次调用把状态从“非 pending”切换为“pending”，则返回 `true`。
    ///
    /// #### 参数
    /// - `width`：宽度（像素）。
    /// - `height`：高度（像素）。
    pub fn set(&self, width: u32, height: u32) -> bool {
        self.packed_size
            .store(pack_u32x2(width, height), Ordering::Relaxed);
        self.pending.swap(1, Ordering::Release) == 0
    }

    /// ### English
    /// Takes the latest size if pending.
    ///
    /// ### 中文
    /// 若处于 pending，则取出最新尺寸。
    pub fn take(&self) -> Option<PhysicalSize<u32>> {
        if self.pending.swap(0, Ordering::Acquire) == 0 {
            return None;
        }
        let packed = self.packed_size.load(Ordering::Relaxed);
        let (width, height) = unpack_u32x2(packed);
        Some(PhysicalSize::new(width, height))
    }
}

#[inline]
/// ### English
/// Packs two `u32` values into a single `u64` (low/high 32-bit lanes).
///
/// #### Parameters
/// - `width`: Low 32-bit lane.
/// - `height`: High 32-bit lane.
///
/// ### 中文
/// 将两个 `u32` 打包为一个 `u64`（低/高 32 位通道）。
///
/// #### 参数
/// - `width`：低 32 位通道。
/// - `height`：高 32 位通道。
fn pack_u32x2(width: u32, height: u32) -> u64 {
    (width as u64) | ((height as u64) << 32)
}

#[inline]
/// ### English
/// Unpacks a `u64` produced by `pack_u32x2` back into two `u32` values.
///
/// #### Parameters
/// - `packed`: Value returned by `pack_u32x2`.
///
/// ### 中文
/// 将 `pack_u32x2` 产生的 `u64` 解包为两个 `u32`。
///
/// #### 参数
/// - `packed`：由 `pack_u32x2` 产生的值。
fn unpack_u32x2(packed: u64) -> (u32, u32) {
    (packed as u32, (packed >> 32) as u32)
}
