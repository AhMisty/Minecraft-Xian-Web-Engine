//! ### English
//! Servo `RenderingContext` implementation for `GlfwTripleBufferRenderingContext`.
//!
//! ### 中文
//! `GlfwTripleBufferRenderingContext` 的 Servo `RenderingContext` 实现。

use std::rc::Rc;
use std::sync::Arc;

use dpi::PhysicalSize;
use gleam::gl;
use glow::HasContext as _;
use surfman::Connection;

use crate::engine::frame::{SLOT_FREE, SLOT_READY, SLOT_RENDERING};

use super::context::GlfwTripleBufferRenderingContext;

impl servo::RenderingContext for GlfwTripleBufferRenderingContext {
    /// ### English
    /// Reads pixels from the current producer-owned back slot into an RGBA image.
    ///
    /// #### Parameters
    /// - `source_rectangle`: Rectangle in device pixels to read back.
    ///
    /// ### 中文
    /// 从当前生产者持有的 back 槽位读回像素并生成 RGBA 图像。
    ///
    /// #### 参数
    /// - `source_rectangle`：需要读回的设备像素矩形区域。
    fn read_to_image(&self, source_rectangle: servo::DeviceIntRect) -> Option<servo::RgbaImage> {
        let slot = self.back_slot.get();
        self.with_slots(|slots| slots.get(slot)?.read_to_image(&self.gl, source_rectangle))
    }

    /// ### English
    /// Returns the current logical size of the rendering surface.
    ///
    /// ### 中文
    /// 返回当前渲染表面的逻辑尺寸。
    fn size(&self) -> PhysicalSize<u32> {
        self.size.get()
    }

    /// ### English
    /// Resizes all per-slot GL resources to `new_size`.
    ///
    /// This sets a shared "resizing" flag to stop the consumer from acquiring while we mutate
    /// shared state, and prefers resizing the producer-owned back slot first (exclusive ownership).
    ///
    /// ### 中文
    /// 将所有槽位的 GL 资源 resize 到 `new_size`。
    ///
    /// 该过程会设置共享的 “resizing” 标记以阻止消费者 acquire，并优先 resize 生产者持有的 back 槽位
    ///（生产者对其具有独占写权限）。
    fn resize(&self, new_size: PhysicalSize<u32>) {
        let old_size = self.size.get();
        if old_size == new_size {
            return;
        }

        self.shared.set_resizing(true);
        let _ = self.make_current();

        let back_slot = self.back_slot.get();
        self.with_slots_mut(|slots| {
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
            slots[back_slot].resize(&self.gl, new_size, self.internal_format);
            self.shared.set_slot_size(back_slot, new_size);
            self.shared.store_state(back_slot, SLOT_RENDERING);

            for (slot, slot_data) in slots.iter_mut().enumerate() {
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
                slot_data.resize(&self.gl, new_size, self.internal_format);
                self.shared.set_slot_size(slot, new_size);
                self.shared.store_state(slot, SLOT_FREE);
            }
        });

        self.size.set(new_size);
        self.shared.set_resizing(false);
    }

    /// ### English
    /// Prepares the current back slot for rendering (sRGB state + FBO binding).
    ///
    /// The sRGB enable state is cached to avoid redundant driver calls.
    ///
    /// ### 中文
    /// 为渲染准备当前 back 槽位（sRGB 状态 + FBO 绑定）。
    ///
    /// sRGB 启用状态会做缓存，以避免重复的驱动调用。
    fn prepare_for_rendering(&self) {
        if self.use_srgb {
            if !self.srgb_enabled.replace(true) {
                self.gl.enable(gl::FRAMEBUFFER_SRGB);
            }
        } else if self.srgb_enabled.replace(false) {
            self.gl.disable(gl::FRAMEBUFFER_SRGB);
        }
        let idx = self.back_slot.get();
        self.ensure_slot_size(idx);
        self.with_slots(|slots| slots[idx].bind(&self.gl));
    }

    /// ### English
    /// Publishes the current back slot as READY and rotates to the next back slot.
    ///
    /// When enabled, inserts a producer fence (`GLsync`) to let the consumer wait before sampling.
    ///
    /// ### 中文
    /// 将当前 back 槽位发布为 READY，并切换到下一 back 槽位。
    ///
    /// 启用时会插入生产者 fence（`GLsync`），供消费者在采样前等待。
    fn present(&self) {
        let current_back = self.back_slot.get();

        let next_back = self.reserved_next_back.take();
        let Some(next_back) = next_back.or_else(|| self.try_reserve_next_back_slot(current_back))
        else {
            return;
        };

        let sync_value = if self.unsafe_no_producer_fence {
            0
        } else {
            let sync = unsafe { self.glow.fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0) }.ok();
            if sync.is_some() {
                unsafe {
                    self.glow.flush();
                }
            }
            sync.map(|s| s.0 as usize as u64).unwrap_or(0)
        };
        let mut new_seq = self.next_frame_seq.get().wrapping_add(1);
        if new_seq == 0 {
            new_seq = 1;
        }
        self.next_frame_seq.set(new_seq);
        self.shared.publish(current_back, sync_value, new_seq);

        self.back_slot.set(next_back);
    }

    /// ### English
    /// Makes the shared GLFW context current on the calling thread.
    ///
    /// ### 中文
    /// 使共享 GLFW 上下文在调用线程上变为 current。
    fn make_current(&self) -> Result<(), surfman::Error> {
        self.shared_ctx.make_current();
        Ok(())
    }

    /// ### English
    /// Returns the gleam GL API wrapper used by Servo/WebRender.
    ///
    /// ### 中文
    /// 返回 Servo/WebRender 使用的 gleam GL API 封装。
    fn gleam_gl_api(&self) -> Rc<dyn gleam::gl::Gl> {
        self.gl.clone()
    }

    /// ### English
    /// Returns the glow GL API wrapper used for fence/sync operations.
    ///
    /// ### 中文
    /// 返回用于 fence/sync 操作的 glow GL API 封装。
    fn glow_gl_api(&self) -> Arc<glow::Context> {
        self.glow.clone()
    }

    /// ### English
    /// Returns the surfman connection (cloned) used by Servo integration.
    ///
    /// ### 中文
    /// 返回 Servo 集成所需的 surfman connection（克隆）。
    fn connection(&self) -> Option<Connection> {
        Some(self.shared_ctx.connection())
    }

    /// ### English
    /// Returns the optional refresh driver used to schedule frames.
    ///
    /// ### 中文
    /// 返回用于调度帧的可选 refresh driver。
    fn refresh_driver(&self) -> Option<Rc<dyn servo::RefreshDriver>> {
        self.refresh_driver.clone()
    }
}
