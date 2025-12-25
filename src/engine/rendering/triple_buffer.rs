//! ### English
//! Triple-buffered offscreen rendering context for Servo (OpenGL).
//!
//! ### 中文
//! Servo 的三缓冲离屏渲染上下文（OpenGL）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use dpi::PhysicalSize;
use gleam::gl::{self, Gl};
use glow::HasContext as _;
use surfman::Connection;

use crate::engine::frame::{
    SLOT_FREE, SLOT_READY, SLOT_RELEASE_PENDING, SLOT_RENDERING, SharedFrameState,
    TRIPLE_BUFFER_COUNT,
};
use crate::engine::refresh::{FixedIntervalRefreshDriver, VsyncRefreshDriver};
use crate::engine::vsync::VsyncCallbackQueue;

use super::shared_context::GlfwSharedContext;

struct TripleBufferSlot {
    /// ### English
    /// Offscreen framebuffer object ID.
    ///
    /// ### 中文
    /// 离屏 framebuffer 对象 ID。
    framebuffer_id: gl::GLuint,
    /// ### English
    /// Color texture attached to `framebuffer_id` (shared with the Java context).
    ///
    /// ### 中文
    /// 绑定到 `framebuffer_id` 的颜色纹理（与 Java 上下文共享）。
    texture_id: gl::GLuint,
    /// ### English
    /// Current allocated texture size (pixels).
    ///
    /// ### 中文
    /// 当前纹理分配尺寸（像素）。
    size: PhysicalSize<u32>,
}

impl TripleBufferSlot {
    fn new(gl: &Rc<dyn Gl>, depth_stencil_rb: gl::GLuint, size: PhysicalSize<u32>) -> Self {
        let framebuffer_ids = gl.gen_framebuffers(1);
        gl.bind_framebuffer(gl::FRAMEBUFFER, framebuffer_ids[0]);

        let texture_ids = gl.gen_textures(1);
        gl.bind_texture(gl::TEXTURE_2D, texture_ids[0]);
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as gl::GLint,
            size.width as gl::GLsizei,
            size.height as gl::GLsizei,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            None,
        );
        gl.tex_parameter_i(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::NEAREST as gl::GLint,
        );
        gl.tex_parameter_i(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::NEAREST as gl::GLint,
        );
        gl.framebuffer_texture_2d(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            texture_ids[0],
            0,
        );
        gl.bind_texture(gl::TEXTURE_2D, 0);
        gl.framebuffer_renderbuffer(
            gl::FRAMEBUFFER,
            gl::DEPTH_STENCIL_ATTACHMENT,
            gl::RENDERBUFFER,
            depth_stencil_rb,
        );

        Self {
            framebuffer_id: framebuffer_ids[0],
            texture_id: texture_ids[0],
            size,
        }
    }

    fn resize(&mut self, gl: &Rc<dyn Gl>, new_size: PhysicalSize<u32>) {
        if self.size == new_size {
            return;
        }

        gl.bind_texture(gl::TEXTURE_2D, self.texture_id);
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as gl::GLint,
            new_size.width as gl::GLsizei,
            new_size.height as gl::GLsizei,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            None,
        );
        gl.bind_texture(gl::TEXTURE_2D, 0);

        self.size = new_size;
    }

    fn delete(&self, gl: &Rc<dyn Gl>) {
        gl.delete_textures(&[self.texture_id]);
        gl.delete_framebuffers(&[self.framebuffer_id]);
    }

    fn bind(&self, gl: &Rc<dyn Gl>) {
        gl.bind_framebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
    }

    fn read_to_image(
        &self,
        gl: &Rc<dyn Gl>,
        source_rectangle: servo::DeviceIntRect,
    ) -> Option<servo::RgbaImage> {
        gl.bind_framebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
        gl.bind_vertex_array(0);

        let mut pixels = gl.read_pixels(
            source_rectangle.min.x,
            source_rectangle.min.y,
            source_rectangle.width(),
            source_rectangle.height(),
            gl::RGBA,
            gl::UNSIGNED_BYTE,
        );

        /*
        ### English
        Flip image vertically (texture coordinate origin differs).

        ### 中文
        垂直翻转图像（纹理坐标原点方向不同）。
        */
        let source_rectangle = source_rectangle.to_usize();
        let stride = source_rectangle.width() * 4;
        let height = source_rectangle.height();
        for y in 0..(height / 2) {
            let top_start = y * stride;
            let bottom_start = (height - y - 1) * stride;
            let (head, tail) = pixels.split_at_mut(bottom_start);
            let top = &mut head[top_start..top_start + stride];
            let bottom = &mut tail[..stride];
            top.swap_with_slice(bottom);
        }

        servo::RgbaImage::from_raw(
            source_rectangle.width() as u32,
            source_rectangle.height() as u32,
            pixels,
        )
    }
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
    shared_ctx: Rc<GlfwSharedContext>,
    /// ### English
    /// gleam GL API wrapper used by Servo/WebRender.
    ///
    /// ### 中文
    /// Servo/WebRender 使用的 gleam GL API 封装。
    gl: Rc<dyn Gl>,
    /// ### English
    /// glow GL API used for fence/sync operations.
    ///
    /// ### 中文
    /// 用于 fence/sync 操作的 glow GL API。
    glow: Arc<glow::Context>,
    /// ### English
    /// Optional refresh driver (external-vsync or fixed interval).
    ///
    /// ### 中文
    /// 可选 refresh driver（外部 vsync 或固定间隔）。
    refresh_driver: Option<Rc<dyn servo::RefreshDriver>>,
    /// ### English
    /// Current logical size of the rendering surface.
    ///
    /// ### 中文
    /// 当前渲染表面的逻辑尺寸。
    size: Cell<PhysicalSize<u32>>,
    /// ### English
    /// Shared depth-stencil renderbuffer (rebound on each slot FBO).
    ///
    /// ### 中文
    /// 共享的深度/模板 renderbuffer（绑定到各槽位 FBO）。
    depth_stencil_rb: gl::GLuint,
    /// ### English
    /// Triple-buffer slot storage (FBO + texture per slot).
    ///
    /// ### 中文
    /// 三缓冲槽位存储（每槽位一个 FBO + 纹理）。
    slots: RefCell<[TripleBufferSlot; TRIPLE_BUFFER_COUNT]>,
    /// ### English
    /// Index of the current producer-owned back slot.
    ///
    /// ### 中文
    /// 当前生产者持有的 back 槽位索引。
    back_slot: Cell<usize>,
    /// ### English
    /// Reserved next back slot (preflight reservation to reduce stalls).
    ///
    /// ### 中文
    /// 预留的下一 back 槽位（用于 preflight 以减少卡顿）。
    reserved_next_back: Cell<Option<usize>>,
    /// ### English
    /// Monotonic frame sequence generator (wraps; 0 is reserved).
    ///
    /// ### 中文
    /// 单调递增帧序号生成器（会回绕；0 保留不用）。
    next_frame_seq: Cell<u64>,
    /// ### English
    /// Lock-free shared frame state consumed by Java.
    ///
    /// ### 中文
    /// 供 Java 消费的无锁共享帧状态。
    shared: Arc<SharedFrameState>,
    /// ### English
    /// Unsafe mode: ignore Java-side consumer fences (faster but may overwrite in-use textures).
    ///
    /// ### 中文
    /// 不安全模式：忽略 Java 侧 consumer fence（更快但可能覆盖正在使用的纹理）。
    unsafe_no_consumer_fence: bool,
    /// ### English
    /// Guard flag to make GL teardown idempotent.
    ///
    /// ### 中文
    /// 防重入标记：保证 GL 资源销毁幂等。
    destroyed: Cell<bool>,
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
    pub fn new(
        shared_ctx: Rc<GlfwSharedContext>,
        initial_size: PhysicalSize<u32>,
        shared: Arc<SharedFrameState>,
        vsync_queue: Arc<VsyncCallbackQueue>,
        target_fps: u32,
        unsafe_no_consumer_fence: bool,
    ) -> Result<Self, String> {
        shared_ctx.make_current_unsafe();

        let gl = shared_ctx.gleam_gl();
        let glow = shared_ctx.glow_gl();

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

        let slots: [TripleBufferSlot; TRIPLE_BUFFER_COUNT] =
            std::array::from_fn(|_| TripleBufferSlot::new(&gl, depth_stencil_rb, initial_size));
        for (i, slot) in slots.iter().enumerate() {
            shared.set_texture_id(i, slot.texture_id);
        }

        let refresh_driver: Option<Rc<dyn servo::RefreshDriver>> = if target_fps == 0 {
            Some(VsyncRefreshDriver::new(vsync_queue))
        } else {
            let fps = target_fps.max(1) as u64;
            let nanos = (1_000_000_000u64 / fps).max(1);
            let driver: Rc<dyn servo::RefreshDriver> =
                FixedIntervalRefreshDriver::new(Duration::from_nanos(nanos));
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
            destroyed: Cell::new(false),
        };
        ctx.shared.store_state(0, SLOT_RENDERING);
        Ok(ctx)
    }

    fn ensure_slot_size(&self, slot: usize) {
        if slot >= TRIPLE_BUFFER_COUNT {
            return;
        }

        let desired_size = self.size.get();
        let mut slots = self.slots.borrow_mut();
        let existing = &mut slots[slot];

        if existing.size == desired_size {
            return;
        }

        existing.resize(&self.gl, desired_size);
        self.shared.set_slot_size(slot, desired_size);
    }

    fn delete_producer_fence_if_any(&self, slot: usize) {
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

    fn delete_consumer_fence_if_any(&self, slot: usize) {
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

    fn try_reserve_next_back_slot(&self, current_back: usize) -> Option<usize> {
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

impl servo::RenderingContext for GlfwTripleBufferRenderingContext {
    fn read_to_image(&self, source_rectangle: servo::DeviceIntRect) -> Option<servo::RgbaImage> {
        let slot = self.back_slot.get();
        let slots = self.slots.borrow();
        slots.get(slot)?.read_to_image(&self.gl, source_rectangle)
    }

    fn size(&self) -> PhysicalSize<u32> {
        self.size.get()
    }

    fn resize(&self, new_size: PhysicalSize<u32>) {
        let old_size = self.size.get();
        if old_size == new_size {
            return;
        }

        self.shared.set_resizing(true);
        let _ = self.make_current();

        let back_slot = self.back_slot.get();
        let mut slots = self.slots.borrow_mut();

        /*
        ### English
        Always resize the back slot; the producer thread owns it.

        ### 中文
        总是先 resize back 槽位；生产者线程拥有它的独占写权限。
        */
        self.gl
            .bind_renderbuffer(gl::RENDERBUFFER, self.depth_stencil_rb);
        self.gl.renderbuffer_storage(
            gl::RENDERBUFFER,
            gl::DEPTH24_STENCIL8,
            new_size.width as gl::GLsizei,
            new_size.height as gl::GLsizei,
        );
        self.gl.bind_renderbuffer(gl::RENDERBUFFER, 0);

        self.delete_producer_fence_if_any(back_slot);
        if !self.unsafe_no_consumer_fence {
            self.delete_consumer_fence_if_any(back_slot);
        }
        slots[back_slot].resize(&self.gl, new_size);
        self.shared.set_slot_size(back_slot, new_size);
        self.shared.clear_producer_fence(back_slot);
        if !self.unsafe_no_consumer_fence {
            self.shared.clear_consumer_fence(back_slot);
        }
        self.shared.store_state(back_slot, SLOT_RENDERING);

        for slot in 0..TRIPLE_BUFFER_COUNT {
            if slot == back_slot {
                continue;
            }

            let locked = self
                .shared
                .compare_exchange_state(slot, SLOT_READY, SLOT_RENDERING)
                .is_ok()
                || self
                    .shared
                    .compare_exchange_state(slot, SLOT_FREE, SLOT_RENDERING)
                    .is_ok();
            if !locked {
                continue;
            }

            self.delete_producer_fence_if_any(slot);
            if !self.unsafe_no_consumer_fence {
                self.delete_consumer_fence_if_any(slot);
            }
            slots[slot].resize(&self.gl, new_size);
            self.shared.set_slot_size(slot, new_size);
            self.shared.clear_producer_fence(slot);
            if !self.unsafe_no_consumer_fence {
                self.shared.clear_consumer_fence(slot);
            }
            self.shared.store_state(slot, SLOT_FREE);
        }

        self.size.set(new_size);
        self.shared.set_resizing(false);
    }

    fn prepare_for_rendering(&self) {
        let idx = self.back_slot.get();
        self.ensure_slot_size(idx);
        let slots = self.slots.borrow();
        slots[idx].bind(&self.gl);
    }

    fn present(&self) {
        let current_back = self.back_slot.get();

        let next_back = self.reserved_next_back.take();
        let Some(next_back) = next_back.or_else(|| self.try_reserve_next_back_slot(current_back))
        else {
            return;
        };

        /*
        ### English
        Insert a fence for the consumer thread.

        ### 中文
        为消费者线程插入 GPU fence。
        */
        let sync = unsafe { self.glow.fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0) }.ok();
        unsafe {
            self.glow.flush();
        }

        let sync_value = sync.map(|s| s.0 as usize as u64).unwrap_or(0);
        let mut new_seq = self.next_frame_seq.get().wrapping_add(1);
        if new_seq == 0 {
            new_seq = 1;
        }
        self.next_frame_seq.set(new_seq);
        self.shared.publish(current_back, sync_value, new_seq);

        self.back_slot.set(next_back);
    }

    fn make_current(&self) -> Result<(), surfman::Error> {
        self.shared_ctx.make_current_unsafe();
        Ok(())
    }

    fn gleam_gl_api(&self) -> Rc<dyn gleam::gl::Gl> {
        self.gl.clone()
    }

    fn glow_gl_api(&self) -> Arc<glow::Context> {
        self.glow.clone()
    }

    fn connection(&self) -> Option<Connection> {
        Some(self.shared_ctx.connection_clone())
    }

    fn refresh_driver(&self) -> Option<Rc<dyn servo::RefreshDriver>> {
        self.refresh_driver.clone()
    }
}
