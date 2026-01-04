use std::rc::Rc;
use std::sync::Arc;

use dpi::PhysicalSize;
use gleam::gl;
use glow::HasContext as _;
use surfman::Connection;

use crate::engine::frame::{SLOT_FREE, SLOT_READY, SLOT_RENDERING, TRIPLE_BUFFER_COUNT};

use super::context::GlfwTripleBufferRenderingContext;

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

        // 总是先 resize back 槽位；生产者线程拥有它的独占写权限。
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
            slots[slot].resize(&self.gl, new_size, self.internal_format);
            self.shared.set_slot_size(slot, new_size);
            self.shared.store_state(slot, SLOT_FREE);
        }

        self.size.set(new_size);
        self.shared.set_resizing(false);
    }

    fn prepare_for_rendering(&self) {
        // 仅在状态发生变化时才启用/禁用 sRGB 写入，避免重复的驱动调用。
        if self.use_srgb {
            if !self.srgb_enabled.replace(true) {
                self.gl.enable(gl::FRAMEBUFFER_SRGB);
            }
        } else if self.srgb_enabled.replace(false) {
            self.gl.disable(gl::FRAMEBUFFER_SRGB);
        }
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

        // 可选地为消费者线程插入生产者 fence。
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
