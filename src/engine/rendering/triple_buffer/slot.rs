use std::rc::Rc;

use dpi::PhysicalSize;
use gleam::gl::{self, Gl};

pub(super) struct TripleBufferSlot {
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
    pub(super) texture_id: gl::GLuint,
    /// ### English
    /// Current allocated texture size (pixels).
    ///
    /// ### 中文
    /// 当前纹理分配尺寸（像素）。
    pub(super) size: PhysicalSize<u32>,
}

impl TripleBufferSlot {
    pub(super) fn new(
        gl: &Rc<dyn Gl>,
        depth_stencil_rb: gl::GLuint,
        size: PhysicalSize<u32>,
        internal_format: gl::GLint,
    ) -> Self {
        let framebuffer_ids = gl.gen_framebuffers(1);
        gl.bind_framebuffer(gl::FRAMEBUFFER, framebuffer_ids[0]);

        let texture_ids = gl.gen_textures(1);
        gl.bind_texture(gl::TEXTURE_2D, texture_ids[0]);
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            internal_format,
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
            gl::LINEAR as gl::GLint,
        );
        gl.tex_parameter_i(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as gl::GLint,
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

    pub(super) fn resize(
        &mut self,
        gl: &Rc<dyn Gl>,
        new_size: PhysicalSize<u32>,
        internal_format: gl::GLint,
    ) {
        if self.size == new_size {
            return;
        }

        gl.bind_texture(gl::TEXTURE_2D, self.texture_id);
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            internal_format,
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

    pub(super) fn delete(&self, gl: &Rc<dyn Gl>) {
        gl.delete_textures(&[self.texture_id]);
        gl.delete_framebuffers(&[self.framebuffer_id]);
    }

    pub(super) fn bind(&self, gl: &Rc<dyn Gl>) {
        gl.bind_framebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
    }

    pub(super) fn read_to_image(
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
