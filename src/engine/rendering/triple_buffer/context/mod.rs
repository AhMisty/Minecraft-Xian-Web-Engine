/// ### English
/// Triple-buffered offscreen rendering context for Servo (OpenGL).
///
/// ### 中文
/// Servo 的三缓冲离屏渲染上下文（OpenGL）。
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use crate::engine::frame::{SharedFrameState, TRIPLE_BUFFER_COUNT};
use crate::engine::refresh::RefreshScheduler;
use crate::engine::vsync::VsyncCallbackQueue;
use dpi::PhysicalSize;
use gleam::gl::{self, Gl};

use super::super::shared_context::GlfwSharedContext;
use super::slot::TripleBufferSlot;

mod fences;
mod init;
mod reserve;
mod teardown;

/// ### English
/// Initialization parameters for `GlfwTripleBufferRenderingContext`.
///
/// ### 中文
/// `GlfwTripleBufferRenderingContext` 的初始化参数。
pub struct GlfwTripleBufferContextInit {
    pub shared_ctx: Rc<GlfwSharedContext>,
    pub initial_size: PhysicalSize<u32>,
    pub shared: Arc<SharedFrameState>,
    pub vsync_queue: Arc<VsyncCallbackQueue>,
    pub target_fps: u32,
    pub unsafe_no_consumer_fence: bool,
    pub unsafe_no_producer_fence: bool,
    pub refresh_scheduler: Option<Arc<RefreshScheduler>>,
}

/// ### English
/// Triple-buffered offscreen rendering context used by Servo on the dedicated thread.
///
/// ### 中文
/// 供 Servo 在独立线程使用的三缓冲离屏渲染上下文。
pub struct GlfwTripleBufferRenderingContext {
    /// ### English
    /// Shared offscreen GLFW context used by the Servo thread.
    ///
    /// ### 中文
    /// Servo 线程使用的共享离屏 GLFW 上下文。
    pub(super) shared_ctx: Rc<GlfwSharedContext>,
    /// ### English
    /// gleam GL API wrapper used by Servo/WebRender.
    ///
    /// ### 中文
    /// Servo/WebRender 使用的 gleam GL API 封装。
    pub(super) gl: Rc<dyn Gl>,
    /// ### English
    /// glow GL API used for fence/sync operations.
    ///
    /// ### 中文
    /// 用于 fence/sync 操作的 glow GL API。
    pub(super) glow: Arc<glow::Context>,
    /// ### English
    /// Optional refresh driver (external-vsync or fixed interval).
    ///
    /// ### 中文
    /// 可选 refresh driver（外部 vsync 或固定间隔）。
    pub(super) refresh_driver: Option<Rc<dyn servo::RefreshDriver>>,
    /// ### English
    /// Current logical size of the rendering surface.
    ///
    /// ### 中文
    /// 当前渲染表面的逻辑尺寸。
    pub(super) size: Cell<PhysicalSize<u32>>,
    /// ### English
    /// Shared depth-stencil renderbuffer (rebound on each slot FBO).
    ///
    /// ### 中文
    /// 共享的深度/模板 renderbuffer（绑定到各槽位 FBO）。
    pub(super) depth_stencil_rb: gl::GLuint,
    /// ### English
    /// Triple-buffer slot storage (FBO + texture per slot).
    ///
    /// ### 中文
    /// 三缓冲槽位存储（每槽位一个 FBO + 纹理）。
    pub(super) slots: RefCell<[TripleBufferSlot; TRIPLE_BUFFER_COUNT]>,
    /// ### English
    /// Index of the current producer-owned back slot.
    ///
    /// ### 中文
    /// 当前生产者持有的 back 槽位索引。
    pub(super) back_slot: Cell<usize>,
    /// ### English
    /// Reserved next back slot (preflight reservation to reduce stalls).
    ///
    /// ### 中文
    /// 预留的下一 back 槽位（用于 preflight 以减少卡顿）。
    pub(super) reserved_next_back: Cell<Option<usize>>,
    /// ### English
    /// Monotonic frame sequence generator (wraps; 0 is reserved).
    ///
    /// ### 中文
    /// 单调递增帧序号生成器（会回绕；0 保留不用）。
    pub(super) next_frame_seq: Cell<u64>,
    /// ### English
    /// Lock-free shared frame state consumed by Java.
    ///
    /// ### 中文
    /// 供 Java 消费的无锁共享帧状态。
    pub(super) shared: Arc<SharedFrameState>,
    /// ### English
    /// Unsafe mode: ignore Java-side consumer fences (faster but may overwrite in-use textures).
    ///
    /// ### 中文
    /// 不安全模式：忽略 Java 侧 consumer fence（更快但可能覆盖正在使用的纹理）。
    pub(super) unsafe_no_consumer_fence: bool,
    /// ### English
    /// Unsafe mode: skip producer-side fences for new frames (lower overhead).
    ///
    /// ### 中文
    /// 不安全模式：跳过生产者侧 fence（开销更低）。
    pub(super) unsafe_no_producer_fence: bool,
    /// ### English
    /// Guard flag to make GL teardown idempotent.
    ///
    /// ### 中文
    /// 防重入标记：保证 GL 资源销毁幂等。
    pub(super) destroyed: Cell<bool>,
    /// ### English
    /// Internal format used for color attachments (sRGB or linear RGBA).
    ///
    /// ### 中文
    /// 颜色附件使用的内部格式（sRGB 或线性 RGBA）。
    pub(super) internal_format: gl::GLint,
    /// ### English
    /// Whether sRGB framebuffer output is enabled.
    ///
    /// ### 中文
    /// 是否启用 sRGB framebuffer 输出。
    pub(super) use_srgb: bool,
    /// ### English
    /// Cached sRGB state to avoid redundant GL state toggles.
    ///
    /// ### 中文
    /// 缓存的 sRGB 状态，避免重复切换 GL 状态。
    pub(super) srgb_enabled: Cell<bool>,
}
