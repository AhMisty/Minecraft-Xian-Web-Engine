//! ### English
//! Bitflags controlling optional view behaviors.
//! These are passed through the C ABI as a `u32` bitmask.
//!
//! ### 中文
//! 控制 view 可选行为的位标志（bitflags）。
//! 通过 C ABI 以 `u32` 位掩码传入。

/// ### English
/// Unsafe mode: skip Java-side consumer fences (can sample incomplete frames; fastest).
///
/// In this mode, pass `consumer_fence = 0` to `xian_web_engine_release_view_frame` /
/// `xian_web_engine_release_view_frames` (the fence argument is ignored).
///
/// ### 中文
/// 不安全模式：跳过 Java 侧 consumer fence（可能采样到未完成帧；最快）。
///
/// 该模式下请对 `xian_web_engine_release_view_frame` / `xian_web_engine_release_view_frames`
/// 传入 `consumer_fence = 0`（fence 参数会被忽略）。
pub const XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_UNSAFE_NO_CONSUMER_FENCE: u32 = 1 << 0;

/// ### English
/// Hint: the embedder guarantees a single input-producer thread (enables a faster push path).
///
/// ### 中文
/// 提示：宿主保证输入生产者只有一个线程（启用更快的 push 路径）。
pub const XIAN_WEB_ENGINE_VIEW_CREATE_FLAG_INPUT_SINGLE_PRODUCER: u32 = 1 << 1;
