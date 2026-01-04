//! ### English
//! Per-slot GL resources for triple-buffered offscreen rendering (FBO + texture).
//!
//! ### 中文
//! 三缓冲离屏渲染的每槽位 GL 资源（FBO + 纹理）。

use std::rc::Rc;

use dpi::PhysicalSize;
use gleam::gl::{self, Gl};

/// ### English
/// One triple-buffer slot containing an offscreen FBO and its color texture.
///
/// ### 中文
/// 三缓冲的一个槽位：包含离屏 FBO 及其颜色纹理。
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
    /// ### English
    /// Creates a new slot (FBO + texture) and attaches the shared depth-stencil renderbuffer.
    ///
    /// #### Parameters
    /// - `gl`: GL API used to create resources.
    /// - `depth_stencil_rb`: Shared depth-stencil renderbuffer ID to attach.
    /// - `size`: Initial texture size.
    /// - `internal_format`: Color internal format (sRGB or linear RGBA).
    ///
    /// ### 中文
    /// 创建一个新槽位（FBO + 纹理），并绑定共享的深度/模板 renderbuffer。
    ///
    /// #### 参数
    /// - `gl`：用于创建资源的 GL API。
    /// - `depth_stencil_rb`：需要绑定的共享深度/模板 renderbuffer ID。
    /// - `size`：初始纹理尺寸。
    /// - `internal_format`：颜色内部格式（sRGB 或线性 RGBA）。
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

    /// ### English
    /// Resizes the color texture storage if the size changed.
    ///
    /// #### Parameters
    /// - `gl`: GL API used to resize resources.
    /// - `new_size`: New texture size.
    /// - `internal_format`: Color internal format (sRGB or linear RGBA).
    ///
    /// ### 中文
    /// 当尺寸变化时，调整颜色纹理的存储大小。
    ///
    /// #### 参数
    /// - `gl`：用于调整资源的 GL API。
    /// - `new_size`：新的纹理尺寸。
    /// - `internal_format`：颜色内部格式（sRGB 或线性 RGBA）。
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

    /// ### English
    /// Deletes the GL resources owned by this slot.
    ///
    /// #### Parameters
    /// - `gl`: GL API used to delete resources.
    ///
    /// ### 中文
    /// 删除该槽位持有的 GL 资源。
    ///
    /// #### 参数
    /// - `gl`：用于删除资源的 GL API。
    pub(super) fn delete(&self, gl: &Rc<dyn Gl>) {
        gl.delete_textures(&[self.texture_id]);
        gl.delete_framebuffers(&[self.framebuffer_id]);
    }

    /// ### English
    /// Binds this slot's framebuffer for rendering or readback.
    ///
    /// #### Parameters
    /// - `gl`: GL API used to bind the framebuffer.
    ///
    /// ### 中文
    /// 绑定该槽位的 framebuffer，用于渲染或读回。
    ///
    /// #### 参数
    /// - `gl`：用于绑定 framebuffer 的 GL API。
    pub(super) fn bind(&self, gl: &Rc<dyn Gl>) {
        gl.bind_framebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
    }

    /// ### English
    /// Reads pixels from this slot's framebuffer into an RGBA image.
    ///
    /// The image is vertically flipped to match the expected coordinate origin.
    ///
    /// #### Parameters
    /// - `gl`: GL API used to read pixels.
    /// - `source_rectangle`: Rectangle in device pixels to read back.
    ///
    /// ### 中文
    /// 从该槽位的 framebuffer 读回像素并生成 RGBA 图像。
    ///
    /// 图像会做一次垂直翻转，以匹配期望的坐标原点方向。
    ///
    /// #### 参数
    /// - `gl`：用于读回像素的 GL API。
    /// - `source_rectangle`：需要读回的设备像素矩形区域。
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
