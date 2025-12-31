use crate::engine::frame::TRIPLE_BUFFER_COUNT;

use super::GlfwTripleBufferRenderingContext;

impl GlfwTripleBufferRenderingContext {
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
