//! ### English
//! Triple-buffered offscreen rendering context for Servo (OpenGL).
//!
//! ### 中文
//! Servo 的三缓冲离屏渲染上下文（OpenGL）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::engine::frame::{
    SLOT_FREE, SLOT_READY, SLOT_RELEASE_PENDING, SLOT_RENDERING, SharedFrameState,
    TRIPLE_BUFFER_COUNT,
};
use crate::engine::refresh::{FixedIntervalRefreshDriver, RefreshScheduler, VsyncRefreshDriver};
use crate::engine::vsync::VsyncCallbackQueue;
use dpi::PhysicalSize;
use gleam::gl::{self, Gl};
use glow::HasContext as _;

use super::super::shared_context::GlfwSharedContext;
use super::slot::TripleBufferSlot;

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
    pub refresh_scheduler: Arc<RefreshScheduler>,
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

impl GlfwTripleBufferRenderingContext {
    /// ### English
    /// Creates a triple-buffered offscreen rendering context.
    ///
    /// Must be called on the Servo thread (the thread that owns `shared_ctx`).
    /// If `target_fps == 0`, refresh is driven by external vsync (`VsyncRefreshDriver`).
    ///
    /// ### 中文
    /// 创建三缓冲离屏渲染上下文。
    ///
    /// 必须在 Servo 线程（持有 `shared_ctx` 的线程）调用。
    /// 若 `target_fps == 0`，则由外部 vsync（`VsyncRefreshDriver`）驱动刷新。
    pub fn new(init: GlfwTripleBufferContextInit) -> Result<Self, String> {
        let GlfwTripleBufferContextInit {
            shared_ctx,
            initial_size,
            shared,
            vsync_queue,
            target_fps,
            unsafe_no_consumer_fence,
            unsafe_no_producer_fence,
            refresh_scheduler,
        } = init;

        shared_ctx.make_current_unsafe();

        let gl = shared_ctx.gleam_gl();
        let glow = shared_ctx.glow_gl();
        let use_srgb = shared_ctx.supports_srgb();
        let internal_format = if use_srgb {
            gl::SRGB8_ALPHA8 as gl::GLint
        } else {
            gl::RGBA as gl::GLint
        };

        let renderbuffer_ids = gl.gen_renderbuffers(1);
        let depth_stencil_rb = renderbuffer_ids[0];
        gl.bind_renderbuffer(gl::RENDERBUFFER, depth_stencil_rb);
        gl.renderbuffer_storage(
            gl::RENDERBUFFER,
            gl::DEPTH24_STENCIL8,
            initial_size.width as gl::GLsizei,
            initial_size.height as gl::GLsizei,
        );
        gl.bind_renderbuffer(gl::RENDERBUFFER, 0);

        let slots: [TripleBufferSlot; TRIPLE_BUFFER_COUNT] = std::array::from_fn(|_| {
            TripleBufferSlot::new(&gl, depth_stencil_rb, initial_size, internal_format)
        });
        for (i, slot) in slots.iter().enumerate() {
            shared.set_texture_id(i, slot.texture_id);
        }

        let refresh_driver: Option<Rc<dyn servo::RefreshDriver>> = if target_fps == 0 {
            Some(VsyncRefreshDriver::new(vsync_queue))
        } else {
            let fps = target_fps.max(1) as u64;
            let nanos = (1_000_000_000u64 / fps).max(1);
            let driver: Rc<dyn servo::RefreshDriver> =
                FixedIntervalRefreshDriver::new(refresh_scheduler, Duration::from_nanos(nanos));
            Some(driver)
        };

        let ctx = Self {
            shared_ctx,
            gl,
            glow,
            refresh_driver,
            size: Cell::new(initial_size),
            depth_stencil_rb,
            slots: RefCell::new(slots),
            back_slot: Cell::new(0),
            reserved_next_back: Cell::new(None),
            next_frame_seq: Cell::new(0),
            shared,
            unsafe_no_consumer_fence,
            unsafe_no_producer_fence,
            destroyed: Cell::new(false),
            internal_format,
            use_srgb,
            srgb_enabled: Cell::new(false),
        };
        ctx.shared.store_state(0, SLOT_RENDERING);
        Ok(ctx)
    }

    pub(super) fn ensure_slot_size(&self, slot: usize) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        let desired_size = self.size.get();
        let mut slots = self.slots.borrow_mut();
        let existing = &mut slots[slot];

        if existing.size == desired_size {
            return;
        }

        existing.resize(&self.gl, desired_size, self.internal_format);
        self.shared.set_slot_size(slot, desired_size);
    }

    pub(super) fn delete_producer_fence_if_any(&self, slot: usize) {
        let fence_value = self.shared.get_producer_fence(slot);
        if fence_value == 0 {
            return;
        }
        let sync = glow::NativeFence(fence_value as usize as *mut _);
        unsafe {
            self.glow.delete_sync(sync);
        }
        self.shared.clear_producer_fence(slot);
    }

    pub(super) fn delete_consumer_fence_if_any(&self, slot: usize) {
        let fence_value = self.shared.get_consumer_fence(slot);
        if fence_value == 0 {
            return;
        }
        let sync = glow::NativeFence(fence_value as usize as *mut _);
        unsafe {
            self.glow.delete_sync(sync);
        }
        self.shared.clear_consumer_fence(slot);
    }

    fn reclaim_release_pending_slots(&self) {
        for slot in 0..TRIPLE_BUFFER_COUNT {
            if self.shared.slot_state(slot) != SLOT_RELEASE_PENDING {
                continue;
            }

            let consumer_fence = self.shared.get_consumer_fence(slot);
            if consumer_fence == 0 {
                if self
                    .shared
                    .compare_exchange_state(slot, SLOT_RELEASE_PENDING, SLOT_FREE)
                    .is_ok()
                {
                    self.shared.clear_consumer_fence(slot);
                }
                continue;
            }

            let sync = glow::NativeFence(consumer_fence as usize as *mut _);
            let status = unsafe { self.glow.client_wait_sync(sync, 0, 0) };
            if status != glow::ALREADY_SIGNALED && status != glow::CONDITION_SATISFIED {
                continue;
            }

            if self
                .shared
                .compare_exchange_state(slot, SLOT_RELEASE_PENDING, SLOT_FREE)
                .is_ok()
            {
                unsafe {
                    self.glow.delete_sync(sync);
                }
                self.shared.clear_consumer_fence(slot);
            }
        }
    }

    pub(super) fn try_reserve_next_back_slot(&self, current_back: usize) -> Option<usize> {
        debug_assert_eq!(TRIPLE_BUFFER_COUNT, 3);
        let slot_a = (current_back + 1) % TRIPLE_BUFFER_COUNT;
        let slot_b = (current_back + 2) % TRIPLE_BUFFER_COUNT;

        /*
        ### English
        Fast path: most of the time, triple-buffering will have at least one FREE slot.

        ### 中文
        快路径：大多数情况下三缓冲至少会有一个 FREE 槽位。
        */
        if self
            .shared
            .compare_exchange_state_relaxed(slot_a, SLOT_FREE, SLOT_RENDERING)
        {
            self.delete_producer_fence_if_any(slot_a);
            if !self.unsafe_no_consumer_fence {
                self.delete_consumer_fence_if_any(slot_a);
            }
            self.ensure_slot_size(slot_a);
            return Some(slot_a);
        }

        if self
            .shared
            .compare_exchange_state_relaxed(slot_b, SLOT_FREE, SLOT_RENDERING)
        {
            self.delete_producer_fence_if_any(slot_b);
            if !self.unsafe_no_consumer_fence {
                self.delete_consumer_fence_if_any(slot_b);
            }
            self.ensure_slot_size(slot_b);
            return Some(slot_b);
        }

        /*
        ### English
        No FREE slots; steal a READY slot.
        Prefer stealing the oldest READY slot so the newest stays available to the consumer thread.

        ### 中文
        没有 FREE 槽位；抢占一个 READY 槽位。
        优先抢占最旧的 READY，避免把最新帧从消费者手里抢走。
        */
        let state_a = self.shared.slot_state_relaxed(slot_a);
        let state_b = self.shared.slot_state_relaxed(slot_b);

        let mut first = None::<usize>;
        let mut second = None::<usize>;

        match (state_a == SLOT_READY, state_b == SLOT_READY) {
            (true, true) => {
                let seq_a = self.shared.slot_seq_relaxed(slot_a);
                let seq_b = self.shared.slot_seq_relaxed(slot_b);
                if seq_a <= seq_b {
                    first = Some(slot_a);
                    second = Some(slot_b);
                } else {
                    first = Some(slot_b);
                    second = Some(slot_a);
                }
            }
            (true, false) => first = Some(slot_a),
            (false, true) => first = Some(slot_b),
            (false, false) => {}
        }

        for slot in [first, second].into_iter().flatten() {
            if self
                .shared
                .compare_exchange_state_relaxed(slot, SLOT_READY, SLOT_RENDERING)
            {
                self.delete_producer_fence_if_any(slot);
                if !self.unsafe_no_consumer_fence {
                    self.delete_consumer_fence_if_any(slot);
                }
                self.ensure_slot_size(slot);
                return Some(slot);
            }
        }

        /*
        ### English
        No FREE/READY slots.
        In safe mode, try to reclaim RELEASE_PENDING slots by polling consumer fences (non-blocking).
        Only do this on the slow path to avoid an extra GL sync query on every frame.

        ### 中文
        没有 FREE/READY 槽位。
        安全模式下通过轮询 consumer fence（非阻塞）回收 RELEASE_PENDING 槽位。
        仅在慢路径执行，避免每帧额外的 GL 同步查询。
        */
        if !self.unsafe_no_consumer_fence {
            self.reclaim_release_pending_slots();

            if self
                .shared
                .compare_exchange_state_relaxed(slot_a, SLOT_FREE, SLOT_RENDERING)
            {
                self.delete_producer_fence_if_any(slot_a);
                self.delete_consumer_fence_if_any(slot_a);
                self.ensure_slot_size(slot_a);
                return Some(slot_a);
            }

            if self
                .shared
                .compare_exchange_state_relaxed(slot_b, SLOT_FREE, SLOT_RENDERING)
            {
                self.delete_producer_fence_if_any(slot_b);
                self.delete_consumer_fence_if_any(slot_b);
                self.ensure_slot_size(slot_b);
                return Some(slot_b);
            }
        }

        None
    }

    /// ### English
    /// Returns whether the associated view is active.
    ///
    /// ### 中文
    /// 返回关联 view 是否 active。
    pub fn is_active(&self) -> bool {
        self.shared.is_active()
    }

    /// ### English
    /// Tries to reserve the next back slot before Servo paints.
    ///
    /// This reduces the chance that `present()` fails due to a lack of slots when the consumer
    /// is temporarily holding a texture.
    ///
    /// ### 中文
    /// 在 Servo paint 之前预留下一 back 槽位。
    ///
    /// 这可降低 `present()` 因暂时没有可用槽位而失败的概率（例如消费者线程短暂持有纹理时）。
    pub fn preflight_reserve_next_back_slot(&self) -> bool {
        if self.reserved_next_back.get().is_some() {
            return true;
        }

        let _ = servo::RenderingContext::make_current(self);
        let current_back = self.back_slot.get();
        let Some(next_back) = self.try_reserve_next_back_slot(current_back) else {
            return false;
        };

        self.reserved_next_back.set(Some(next_back));
        true
    }

    /// ### English
    /// Destroys all GL resources owned by this context (idempotent).
    ///
    /// Must run on the thread that owns the GL context (Servo thread).
    ///
    /// ### 中文
    /// 销毁该上下文持有的所有 GL 资源（幂等）。
    ///
    /// 必须在持有 GL 上下文的线程（Servo 线程）执行。
    pub fn destroy_gl_resources(&self) {
        if self.destroyed.replace(true) {
            return;
        }

        self.shared.set_resizing(true);

        let _ = servo::RenderingContext::make_current(self);
        if !self.unsafe_no_consumer_fence {
            self.reclaim_release_pending_slots();
        }

        for slot in 0..TRIPLE_BUFFER_COUNT {
            self.delete_producer_fence_if_any(slot);
            if !self.unsafe_no_consumer_fence {
                self.delete_consumer_fence_if_any(slot);
            }
        }

        let slots = self.slots.borrow();
        for slot in slots.iter() {
            slot.delete(&self.gl);
        }

        self.gl.delete_renderbuffers(&[self.depth_stencil_rb]);
    }
}

impl Drop for GlfwTripleBufferRenderingContext {
    fn drop(&mut self) {
        self.destroy_gl_resources();
    }
}
