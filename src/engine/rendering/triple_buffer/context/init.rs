//! ### English
//! Initialization for `GlfwTripleBufferRenderingContext`.
//!
//! ### 中文
//! `GlfwTripleBufferRenderingContext` 的初始化逻辑。

use std::cell::{Cell, UnsafeCell};
use std::rc::Rc;
use std::time::Duration;

use crate::engine::frame::{SLOT_RENDERING, TRIPLE_BUFFER_COUNT};
use crate::engine::refresh::{FixedIntervalRefreshDriver, VsyncRefreshDriver};
use gleam::gl;

use super::super::slot::TripleBufferSlot;
use super::{GlfwTripleBufferContextInit, GlfwTripleBufferRenderingContext};

impl GlfwTripleBufferRenderingContext {
    /// ### English
    /// Creates a triple-buffered offscreen rendering context.
    ///
    /// Must be called on the Servo thread (the thread that owns `shared_ctx`).
    /// If `target_fps == 0`, refresh is driven by external vsync (`VsyncRefreshDriver`).
    ///
    /// #### Parameters
    /// - `init`: Initialization bundle for the rendering context.
    ///
    /// ### 中文
    /// 创建三缓冲离屏渲染上下文。
    ///
    /// 必须在 Servo 线程（持有 `shared_ctx` 的线程）调用。
    /// 若 `target_fps == 0`，则由外部 vsync（`VsyncRefreshDriver`）驱动刷新。
    ///
    /// #### 参数
    /// - `init`：渲染上下文的初始化参数包。
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

        shared_ctx.make_current();

        let gl = shared_ctx.gl();
        let glow = shared_ctx.glow();
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
            let Some(refresh_scheduler) = refresh_scheduler else {
                return Err("Missing RefreshScheduler for fixed-interval refresh".to_string());
            };

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
            slots: UnsafeCell::new(slots),
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
}
