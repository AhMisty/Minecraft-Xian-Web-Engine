/// ### English
/// Lock-free triple-buffer frame state shared between the Servo thread (producer) and Java thread
/// (consumer). Uses atomics to avoid OS locks on the hot path.
///
/// ### 中文
/// Servo 线程（生产者）与 Java 线程（消费者）共享的无锁三缓冲帧状态。
/// 热路径使用原子操作避免系统锁。
mod shared_state;
mod slot;

pub use shared_state::SharedFrameState;

/// ### English
/// Fixed triple-buffer slot count (always 3 for maximum performance / simplicity).
///
/// ### 中文
/// 固定三缓冲槽位数量（始终为 3，以最大化性能并简化分支）。
pub const TRIPLE_BUFFER_COUNT: usize = 3;

pub(crate) const SLOT_FREE: u8 = 0;
pub(crate) const SLOT_READY: u8 = 1;
pub(crate) const SLOT_HELD: u8 = 2;
pub(crate) const SLOT_RELEASE_PENDING: u8 = 3;
pub(crate) const SLOT_RENDERING: u8 = 4;

/// ### English
/// Metadata for one acquired frame (consumer side / Java thread).
///
/// ### 中文
/// 单个已获取帧的元数据（消费者侧 / Java 线程）。
#[derive(Clone, Copy, Debug)]
pub(crate) struct AcquiredFrame {
    /// ### English
    /// Triple-buffer slot index.
    ///
    /// ### 中文
    /// 三缓冲槽位索引。
    pub slot: usize,
    /// ### English
    /// GL texture ID containing the frame.
    ///
    /// ### 中文
    /// 包含该帧的 GL 纹理 ID。
    pub texture_id: u32,
    /// ### English
    /// Producer fence handle (`GLsync` cast to `u64`), or 0 if unavailable.
    ///
    /// ### 中文
    /// 生产者 fence 句柄（`GLsync` 转为 `u64`），不可用则为 0。
    pub producer_fence: u64,
    /// ### English
    /// Frame width in pixels.
    ///
    /// ### 中文
    /// 帧宽度（像素）。
    pub width: u32,
    /// ### English
    /// Frame height in pixels.
    ///
    /// ### 中文
    /// 帧高度（像素）。
    pub height: u32,
}
