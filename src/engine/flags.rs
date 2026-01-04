//! ### English
//! Bitflags controlling optional view behaviors.
//!
//! These are passed through the C ABI as a `u32` bitmask.
//!
//! ### 中文
//! 控制 view 可选行为的位标志（bitflags）。
//!
//! 通过 C ABI 以 `u32` 位掩码传入。
/// ### English
/// Unsafe mode: skip Java-side consumer fences (fastest but may overwrite textures still in use).
///
/// In this mode, pass `consumer_fences = NULL` (or 0 fences) to `xian_web_engine_views_release_frames`
/// (the consumer fence is ignored).
///
/// ### 中文
/// 不安全模式：跳过 Java 侧 consumer fence（最快，但可能覆盖仍在被消费的纹理）。
///
/// 该模式下请对 `xian_web_engine_views_release_frames` 传入 `consumer_fences = NULL`
///（或确保对应 fence 为 0；fence 参数会被忽略）。
pub const XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE: u32 = 1 << 0;

/// ### English
/// Hint: the embedder guarantees a single input-producer thread (enables a faster push path).
/// If this guarantee is violated (multiple producer threads), behavior is undefined.
///
/// ### 中文
/// 提示：宿主保证输入生产者只有一个线程（启用更快的 push 路径）。
/// 若该保证被违反（存在多个生产者线程），则行为未定义。
pub const XIAN_WEB_ENGINE_VIEW_FLAG_INPUT_SINGLE_PRODUCER: u32 = 1 << 1;

/// ### English
/// Unsafe mode: skip producer-side fences (`GLsync`) for new frames (lower overhead).
///
/// In this mode, `XianWebEngineFrame.producer_fence` will always be `0`, and the embedder must ensure it
/// does not sample incomplete frames (e.g., by using other synchronization).
///
/// ### 中文
/// 不安全模式：跳过生产者侧 fence（`GLsync`）（开销更低）。
///
/// 该模式下 `XianWebEngineFrame.producer_fence` 将始终为 `0`，宿主需自行保证不会采样到未完成的帧
/// （例如使用其它同步机制）。
pub const XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_PRODUCER_FENCE: u32 = 1 << 2;
