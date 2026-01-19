//! ### English
//! OpenGL glue: external GLFW context + offscreen framebuffer rendering context for Servo.
//!
//! ### 中文
//! OpenGL 粘合层：外部 GLFW 上下文 + Servo 使用的离屏 FBO 渲染上下文。

use std::cell::{Cell, OnceCell, RefCell};
use std::ffi::{CString, c_char, c_void};
use std::rc::Rc;
use std::sync::Arc;

use dpi::PhysicalSize;
use gleam::gl::{self, Gl};
use servo::RgbaImage;
use surfman::{Connection, Error};

use crate::error::InitError;

#[repr(u32)]
#[derive(Clone, Copy)]
/// ### English
/// OpenGL API selection for function loading and feature differences.
///
/// ### 中文
/// 用于函数加载与差异处理的 OpenGL API 选择。
pub(crate) enum GlApi {
    /// ### English
    /// Desktop OpenGL.
    ///
    /// ### 中文
    /// 桌面 OpenGL。
    Gl,

    /// ### English
    /// OpenGL ES.
    ///
    /// ### 中文
    /// OpenGL ES。
    Gles,
}

impl GlApi {
    /// ### English
    /// Parses a C ABI `gl_api` value into `GlApi`.
    ///
    /// #### Parameters
    /// - `value`: C ABI constant (`XIAN_WEB_ENGINE_GL_API_*`).
    ///
    /// #### Returns
    /// - `Ok(GlApi)` on success.
    /// - `Err(InitError)` when the value is unsupported.
    ///
    /// ### 中文
    /// 将 C ABI 的 `gl_api` 值解析为 `GlApi`。
    ///
    /// #### 参数
    /// - `value`：C ABI 常量（`XIAN_WEB_ENGINE_GL_API_*`）。
    ///
    /// #### 返回
    /// - 成功返回 `Ok(GlApi)`。
    /// - 不支持时返回 `Err(InitError)`。
    pub(crate) fn from_u32(value: u32) -> Result<Self, InitError> {
        match value {
            crate::abi::XIAN_WEB_ENGINE_GL_API_GL => Ok(Self::Gl),
            crate::abi::XIAN_WEB_ENGINE_GL_API_GLES => Ok(Self::Gles),
            _ => Err(InitError::UnsupportedGlApi { value }),
        }
    }
}

/// ### English
/// `glfwGetProcAddress` function signature used by this embedder.
///
/// ### 中文
/// 本嵌入层使用的 `glfwGetProcAddress` 函数签名。
type GlfwGetProcAddressFn = unsafe extern "C" fn(*const c_char) -> *const c_void;

/// ### English
/// `glfwMakeContextCurrent` function signature used by this embedder.
///
/// ### 中文
/// 本嵌入层使用的 `glfwMakeContextCurrent` 函数签名。
type GlfwMakeContextCurrentFn = unsafe extern "C" fn(*mut c_void);

thread_local! {
    /// ### English
    /// Thread-local Surfman `Connection` cache (created lazily).
    ///
    /// `surfman::Connection` is not guaranteed to be `Sync` on all platforms, so this cache is
    /// thread-local by design.
    ///
    /// ### 中文
    /// 线程本地的 Surfman `Connection` 缓存（惰性创建）。
    ///
    /// `surfman::Connection` 在所有平台上都不保证实现 `Sync`，因此该缓存刻意设计为 thread-local。
    static SURFMAN_CONNECTION: OnceCell<Option<Connection>> = const { OnceCell::new() };
}

#[inline]
/// ### English
/// Safety: converts a function pointer address into a typed non-null function pointer.
///
/// #### Parameters
/// - `addr`: Function pointer address (`0` means "NULL").
/// - `name`: Symbol name (for diagnostics).
///
/// #### Returns
/// - `Ok(T)` when `addr != 0`.
/// - `Err(InitError)` when `addr == 0`.
///
/// #### Safety
/// - `addr` must be a valid function pointer address whose signature matches `T`.
///
/// ### 中文
/// 安全性：将函数指针地址转换为带签名的非空函数指针。
///
/// #### 参数
/// - `addr`：函数指针地址（`0` 表示“NULL”）。
/// - `name`：符号名（用于诊断定位）。
///
/// #### 返回
/// - `addr != 0` 时返回 `Ok(T)`。
/// - `addr == 0` 时返回 `Err(InitError)`。
///
/// #### 安全性
/// - `addr` 必须是有效的函数指针地址，且其签名必须与 `T` 匹配。
unsafe fn fn_from_addr<T>(addr: usize, name: &'static str) -> Result<T, InitError> {
    if addr == 0 {
        return Err(InitError::NullPointer { name });
    }
    Ok(unsafe { std::mem::transmute_copy::<usize, T>(&addr) })
}

#[derive(Clone, Copy)]
/// ### English
/// Minimal GLFW context + proc table wrapper used for OpenGL function loading and context binding.
///
/// ### 中文
/// 用于 OpenGL 函数加载与上下文绑定的最小 GLFW 上下文 + 函数表封装。
pub(crate) struct GlfwContext {
    /// ### English
    /// Pointer to embedder-owned `GLFWwindow*`.
    ///
    /// ### 中文
    /// 宿主侧 `GLFWwindow*` 指针。
    window: *mut c_void,

    /// ### English
    /// `glfwGetProcAddress` entry point.
    ///
    /// ### 中文
    /// `glfwGetProcAddress` 入口。
    get_proc_address: GlfwGetProcAddressFn,

    /// ### English
    /// Optional `glfwMakeContextCurrent` entry point.
    ///
    /// When "assume current" is enabled, this is stored as `None` and all calls become no-ops.
    ///
    /// ### 中文
    /// 可选的 `glfwMakeContextCurrent` 入口。
    ///
    /// 当启用“假定 current”时该字段为 `None`，相关调用将变为 no-op。
    make_context_current: Option<GlfwMakeContextCurrentFn>,
}

impl GlfwContext {
    /// ### English
    /// Creates a `GlfwContext` from raw function pointer addresses.
    ///
    /// #### Parameters
    /// - `window`: Embedder-owned `GLFWwindow*`.
    /// - `glfw_get_proc_address`: Address of `glfwGetProcAddress` (as `uintptr_t`).
    /// - `glfw_make_context_current`: Address of `glfwMakeContextCurrent` (as `uintptr_t`, required when `assume_current == false`).
    /// - `assume_current`: Whether to skip calling `glfwMakeContextCurrent` on hot paths.
    ///
    /// #### Returns
    /// - `Ok(GlfwContext)` on success.
    /// - `Err(InitError)` if required function pointers are missing.
    ///
    /// #### Safety
    /// - `window` must be a valid `GLFWwindow*` for the embedder.
    /// - Function pointer addresses must match the declared signatures.
    ///
    /// ### 中文
    /// 由原始函数指针地址创建 `GlfwContext`。
    ///
    /// #### 参数
    /// - `window`：宿主侧 `GLFWwindow*`。
    /// - `glfw_get_proc_address`：`glfwGetProcAddress` 的地址（`uintptr_t`）。
    /// - `glfw_make_context_current`：`glfwMakeContextCurrent` 的地址（`uintptr_t`；当 `assume_current == false` 时必须提供）。
    /// - `assume_current`：是否在热路径上跳过 `glfwMakeContextCurrent`。
    ///
    /// #### 返回
    /// - 成功返回 `Ok(GlfwContext)`。
    /// - 必需函数指针缺失时返回 `Err(InitError)`。
    ///
    /// #### 安全性
    /// - `window` 必须是宿主侧有效的 `GLFWwindow*`。
    /// - 函数指针地址必须与声明的签名一致。
    pub(crate) unsafe fn from_raw(
        window: *mut c_void,
        glfw_get_proc_address: usize,
        glfw_make_context_current: usize,
        assume_current: bool,
    ) -> Result<Self, InitError> {
        let get_proc_address: GlfwGetProcAddressFn =
            unsafe { fn_from_addr(glfw_get_proc_address, "glfwGetProcAddress")? };
        let make_context_current: Option<GlfwMakeContextCurrentFn> = if assume_current {
            None
        } else {
            Some(unsafe { fn_from_addr(glfw_make_context_current, "glfwMakeContextCurrent")? })
        };
        Ok(Self {
            window,
            get_proc_address,
            make_context_current,
        })
    }

    /// ### English
    /// Makes the embedder context current (unless "assume current" is enabled).
    ///
    /// #### Safety
    /// - Must be called on the thread that is allowed to make the context current.
    /// - The `window` pointer must remain valid.
    ///
    /// ### 中文
    /// 将宿主上下文设为 current（除非启用“假定 current”）。
    ///
    /// #### 安全性
    /// - 必须在允许切换上下文的线程调用。
    /// - `window` 指针必须保持有效。
    pub(crate) unsafe fn make_current(&self) {
        let Some(make_current) = self.make_context_current else {
            return;
        };
        unsafe { make_current(self.window) };
    }

    /// ### English
    /// Loads an OpenGL function pointer by name via `glfwGetProcAddress`.
    ///
    /// #### Parameters
    /// - `name`: Function name (ASCII, without NUL).
    ///
    /// #### Returns
    /// - Raw function pointer address (NULL if unavailable).
    ///
    /// ### 中文
    /// 通过 `glfwGetProcAddress` 按名称加载 OpenGL 函数指针。
    ///
    /// #### 参数
    /// - `name`：函数名（ASCII，不含 NUL）。
    ///
    /// #### 返回
    /// - 原始函数指针地址（不可用时为 NULL）。
    fn load(&self, name: &str) -> *const c_void {
        let bytes = name.as_bytes();

        const STACK_BUF_CAP: usize = 128;
        if bytes.len() < STACK_BUF_CAP {
            let mut buf = [0u8; STACK_BUF_CAP];
            buf[..bytes.len()].copy_from_slice(bytes);
            unsafe { (self.get_proc_address)(buf.as_ptr().cast()) }
        } else {
            let Ok(name) = CString::new(name) else {
                return std::ptr::null();
            };
            unsafe { (self.get_proc_address)(name.as_ptr()) }
        }
    }
}

#[derive(Clone)]
/// ### English
/// Shared OpenGL API handles used by Servo.
///
/// ### 中文
/// Servo 使用的共享 OpenGL API 句柄。
pub(crate) struct GlHandles {
    /// ### English
    /// Gleam GL wrapper (used by Servo and Surfman integration).
    ///
    /// ### 中文
    /// Gleam GL 封装（Servo / Surfman 集成使用）。
    gleam_gl: Rc<dyn Gl>,

    /// ### English
    /// Glow context (used by some Servo subsystems).
    ///
    /// ### 中文
    /// Glow 上下文（Servo 的部分子系统使用）。
    glow_gl: Arc<glow::Context>,
}

impl GlHandles {
    /// ### English
    /// Loads OpenGL function pointers into both Gleam and Glow contexts.
    ///
    /// #### Parameters
    /// - `gl_api`: GL vs GLES selection.
    /// - `glfw`: GLFW proc table used for function lookup.
    ///
    /// #### Returns
    /// - `Ok(GlHandles)` when loading succeeded.
    /// - `Err(InitError)` on missing required entry points.
    ///
    /// #### Safety
    /// - The OpenGL context must be current before calling.
    ///
    /// ### 中文
    /// 使用 GLFW 函数表为 Gleam 与 Glow 加载 OpenGL 函数指针。
    ///
    /// #### 参数
    /// - `gl_api`：GL 与 GLES 选择。
    /// - `glfw`：用于函数查找的 GLFW 函数表。
    ///
    /// #### 返回
    /// - 加载成功返回 `Ok(GlHandles)`。
    /// - 缺少必需入口时返回 `Err(InitError)`。
    ///
    /// #### 安全性
    /// - 调用前 OpenGL 上下文必须已 current。
    pub(crate) unsafe fn new(gl_api: GlApi, glfw: &GlfwContext) -> Result<Self, InitError> {
        validate_gl_entry_points(glfw)?;
        let gleam_gl: Rc<dyn Gl> = match gl_api {
            GlApi::Gl => unsafe { gl::GlFns::load_with(|s| glfw.load(s) as *const _) },
            GlApi::Gles => unsafe { gl::GlesFns::load_with(|s| glfw.load(s) as *const _) },
        };

        let glow_gl = unsafe { glow::Context::from_loader_function(|s| glfw.load(s) as *const _) };

        Ok(Self {
            gleam_gl,
            glow_gl: Arc::new(glow_gl),
        })
    }
}

/// ### English
/// Validates a minimal set of OpenGL entry points required by this embedder.
///
/// This is a best-effort sanity check to fail fast before calling into NULL function pointers.
///
/// #### Parameters
/// - `glfw`: Embedder context handle used for symbol lookup.
///
/// #### Returns
/// - `Ok(())` when all required symbols are available.
/// - `Err(InitError)` when any required symbol is missing.
///
/// ### 中文
/// 校验本嵌入层所需的最小 OpenGL 入口集合。
///
/// 这是一个 best-effort 的快速健全性检查，用于在调用到 NULL 函数指针前尽早失败。
///
/// #### 参数
/// - `glfw`：用于符号查找的宿主上下文句柄。
///
/// #### 返回
/// - 全部必需符号可用时返回 `Ok(())`。
/// - 任意必需符号缺失时返回 `Err(InitError)`。
fn validate_gl_entry_points(glfw: &GlfwContext) -> Result<(), InitError> {
    const REQUIRED: &[&str] = &[
        "glGenFramebuffers",
        "glBindFramebuffer",
        "glGenTextures",
        "glBindTexture",
        "glTexImage2D",
        "glTexParameteri",
        "glFramebufferTexture2D",
        "glGenRenderbuffers",
        "glBindRenderbuffer",
        "glRenderbufferStorage",
        "glFramebufferRenderbuffer",
        "glViewport",
        "glBindVertexArray",
        "glReadPixels",
        "glDeleteTextures",
        "glDeleteRenderbuffers",
        "glDeleteFramebuffers",
    ];

    for &name in REQUIRED {
        if glfw.load(name).is_null() {
            return Err(InitError::MissingOpenGlEntryPoint { name });
        }
    }
    Ok(())
}

/// ### English
/// Offscreen framebuffer (FBO + RGBA texture + depth renderbuffer) used as Servo render target.
///
/// The size is tracked by the parent `TextureContext`.
///
/// ### 中文
/// 作为 Servo 渲染目标的离屏帧缓冲（FBO + RGBA 纹理 + 深度 Renderbuffer）。
///
/// 尺寸由上层 `TextureContext` 追踪。
struct Framebuffer {
    /// ### English
    /// Gleam GL entry used to create and operate GL objects.
    ///
    /// ### 中文
    /// 用于创建与操作 GL 对象的 Gleam GL 入口。
    gl: Rc<dyn Gl>,

    /// ### English
    /// GL framebuffer id.
    ///
    /// ### 中文
    /// GL framebuffer id。
    framebuffer_id: gl::GLuint,

    /// ### English
    /// GL renderbuffer id (depth attachment).
    ///
    /// ### 中文
    /// GL renderbuffer id（深度附件）。
    renderbuffer_id: gl::GLuint,

    /// ### English
    /// GL texture id (RGBA color attachment, 8-bit per channel).
    ///
    /// ### 中文
    /// GL texture id（RGBA 颜色附件，每通道 8-bit）。
    texture_id: gl::GLuint,
}

impl Framebuffer {
    /// ### English
    /// Creates a new offscreen framebuffer of the given size.
    ///
    /// #### Parameters
    /// - `gl`: Gleam GL entry bound to the current context.
    /// - `size`: Physical size in pixels (must be >= 1 in both dimensions).
    ///
    /// #### Returns
    /// - `Framebuffer` with an RGBA texture color attachment and a depth renderbuffer.
    ///
    /// ### 中文
    /// 创建指定尺寸的离屏帧缓冲。
    ///
    /// #### 参数
    /// - `gl`：绑定到当前上下文的 Gleam GL 入口。
    /// - `size`：物理像素尺寸（两个维度都必须 >= 1）。
    ///
    /// #### 返回
    /// - 包含 RGBA 纹理颜色附件与深度 Renderbuffer 的 `Framebuffer`。
    fn new(gl: Rc<dyn Gl>, size: PhysicalSize<u32>) -> Self {
        let framebuffer_ids = gl.gen_framebuffers(1);
        let framebuffer_id = framebuffer_ids[0];
        gl.bind_framebuffer(gl::FRAMEBUFFER, framebuffer_id);

        let texture_ids = gl.gen_textures(1);
        let texture_id = texture_ids[0];
        gl.bind_texture(gl::TEXTURE_2D, texture_id);
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            /*
            ### English
            Use the base `RGBA` internal format (matches Servo offscreen context).

            ### 中文
            使用基础 `RGBA` 内部格式（与 Servo 离屏上下文一致）。
            */
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
            texture_id,
            0,
        );
        gl.bind_texture(gl::TEXTURE_2D, 0);

        let renderbuffer_ids = gl.gen_renderbuffers(1);
        let renderbuffer_id = renderbuffer_ids[0];
        gl.bind_renderbuffer(gl::RENDERBUFFER, renderbuffer_id);
        gl.renderbuffer_storage(
            gl::RENDERBUFFER,
            /*
            ### English
            Use a 24-bit depth renderbuffer (matches Servo offscreen context).

            ### 中文
            使用 24-bit 深度 Renderbuffer（与 Servo 离屏上下文一致）。
            */
            gl::DEPTH_COMPONENT24,
            size.width as gl::GLsizei,
            size.height as gl::GLsizei,
        );
        gl.framebuffer_renderbuffer(
            gl::FRAMEBUFFER,
            gl::DEPTH_ATTACHMENT,
            gl::RENDERBUFFER,
            renderbuffer_id,
        );

        Self {
            gl,
            framebuffer_id,
            renderbuffer_id,
            texture_id,
        }
    }

    /// ### English
    /// Binds this framebuffer.
    ///
    /// ### 中文
    /// 绑定该帧缓冲。
    fn bind(&self) {
        self.gl
            .bind_framebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
    }

    /// ### English
    /// Reads pixels into a `RgbaImage`.
    ///
    /// The buffer is flipped vertically in-place to match the top-left origin expected by Servo.
    ///
    /// #### Parameters
    /// - `source_rectangle`: Rectangle to read (in device pixels).
    ///
    /// #### Returns
    /// - `Some(RgbaImage)` on success.
    /// - `None` if Servo rejects the buffer (e.g. size mismatch).
    ///
    /// ### 中文
    /// 读取像素并返回 `RgbaImage`。
    ///
    /// 会原地做一次垂直翻转，以匹配 Servo 期望的左上角原点坐标系。
    ///
    /// #### 参数
    /// - `source_rectangle`：读取区域（设备像素）。
    ///
    /// #### 返回
    /// - 成功返回 `Some(RgbaImage)`。
    /// - 失败返回 `None`（例如尺寸不匹配）。
    fn read_to_image(&self, source_rectangle: servo::DeviceIntRect) -> Option<RgbaImage> {
        self.gl
            .bind_framebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
        self.gl.bind_vertex_array(0);

        let mut pixels = self.gl.read_pixels(
            source_rectangle.min.x,
            source_rectangle.min.y,
            source_rectangle.width(),
            source_rectangle.height(),
            gl::RGBA,
            gl::UNSIGNED_BYTE,
        );

        let source_rectangle = source_rectangle.to_usize();
        let width = source_rectangle.width();
        let height = source_rectangle.height();
        let stride = width * 4;

        for y in 0..(height / 2) {
            let top = y * stride;
            let bottom = (height - 1 - y) * stride;
            unsafe {
                std::ptr::swap_nonoverlapping(
                    pixels.as_mut_ptr().add(top),
                    pixels.as_mut_ptr().add(bottom),
                    stride,
                );
            }
        }

        RgbaImage::from_raw(
            source_rectangle.width() as u32,
            source_rectangle.height() as u32,
            pixels,
        )
    }
}

impl Drop for Framebuffer {
    /// ### English
    /// Deletes GL objects owned by this framebuffer.
    ///
    /// ### 中文
    /// 删除该帧缓冲持有的 GL 对象。
    fn drop(&mut self) {
        /*
        ### English
        Unbind before deletion to avoid keeping deleted ids bound (matches Servo behavior).

        ### 中文
        删除前先解绑，避免已删除的 id 仍处于绑定状态（与 Servo 行为一致）。
        */
        self.gl.bind_framebuffer(gl::FRAMEBUFFER, 0);
        self.gl.delete_textures(&[self.texture_id]);
        self.gl.delete_renderbuffers(&[self.renderbuffer_id]);
        self.gl.delete_framebuffers(&[self.framebuffer_id]);
    }
}

/// ### English
/// Servo `RenderingContext` backed by an embedder-provided GLFW OpenGL context and an offscreen
/// framebuffer texture.
///
/// Each instance owns one FBO + RGBA texture and can be resized.
///
/// ### 中文
/// 基于宿主提供的 GLFW OpenGL 上下文与离屏帧缓冲纹理的 Servo `RenderingContext`。
///
/// 每个实例持有一套 FBO + RGBA 纹理，并支持 resize。
pub(crate) struct TextureContext {
    /// ### English
    /// GLFW proc table used for making context current and loading functions.
    ///
    /// ### 中文
    /// 用于切换上下文与加载函数的 GLFW 函数表。
    glfw: GlfwContext,

    /// ### English
    /// Current size in physical pixels.
    ///
    /// ### 中文
    /// 当前尺寸（物理像素）。
    size: Cell<PhysicalSize<u32>>,

    /// ### English
    /// Offscreen framebuffer storage.
    ///
    /// ### 中文
    /// 离屏帧缓冲存储。
    framebuffer: RefCell<Framebuffer>,

    /// ### English
    /// Gleam GL entry used by Servo.
    ///
    /// ### 中文
    /// Servo 使用的 Gleam GL 入口。
    gleam_gl: Rc<dyn Gl>,

    /// ### English
    /// Glow context used by Servo.
    ///
    /// ### 中文
    /// Servo 使用的 Glow 上下文。
    glow_gl: Arc<glow::Context>,

    /// ### English
    /// Optional refresh driver used by Servo to schedule future frames.
    ///
    /// ### 中文
    /// Servo 用于安排后续帧的可选刷新驱动。
    refresh_driver: Option<Rc<dyn servo::RefreshDriver>>,
}

impl TextureContext {
    /// ### English
    /// Creates a new texture-backed rendering context.
    ///
    /// #### Parameters
    /// - `glfw`: GLFW proc table for the embedder context.
    /// - `gl`: Shared GL API handles.
    /// - `size`: Initial size (clamped by caller to >= 1).
    /// - `refresh_driver`: Optional refresh driver for frame scheduling.
    ///
    /// #### Returns
    /// - A new `TextureContext` instance.
    ///
    /// ### 中文
    /// 创建一个基于纹理的渲染上下文。
    ///
    /// #### 参数
    /// - `glfw`：宿主上下文的 GLFW 函数表。
    /// - `gl`：共享 GL API 句柄。
    /// - `size`：初始尺寸（调用方需保证 >= 1）。
    /// - `refresh_driver`：可选刷新驱动，用于帧调度。
    ///
    /// #### 返回
    /// - 新的 `TextureContext` 实例。
    pub(crate) fn new(
        glfw: GlfwContext,
        gl: GlHandles,
        size: PhysicalSize<u32>,
        refresh_driver: Option<Rc<dyn servo::RefreshDriver>>,
    ) -> Self {
        let framebuffer = RefCell::new(Framebuffer::new(gl.gleam_gl.clone(), size));
        Self {
            glfw,
            size: Cell::new(size),
            framebuffer,
            gleam_gl: gl.gleam_gl,
            glow_gl: gl.glow_gl,
            refresh_driver,
        }
    }

    /// ### English
    /// Returns the OpenGL texture id of the color attachment.
    ///
    /// #### Notes
    /// - The texture id may change after `resize` (framebuffer is recreated to match Servo behavior).
    ///
    /// #### Returns
    /// - OpenGL texture id.
    ///
    /// ### 中文
    /// 返回颜色附件的 OpenGL 纹理 id。
    ///
    /// #### 说明
    /// - 纹理 id 可能在 `resize` 后变化（为对齐 Servo 行为会重建帧缓冲）。
    ///
    /// #### 返回
    /// - OpenGL 纹理 id。
    pub(crate) fn texture_id(&self) -> u32 {
        self.framebuffer.borrow().texture_id
    }
}

impl servo::RenderingContext for TextureContext {
    /// ### English
    /// Prepares the current frame for rendering.
    ///
    /// ### 中文
    /// 为当前帧渲染做准备。
    fn prepare_for_rendering(&self) {
        self.framebuffer.borrow().bind();
    }

    /// ### English
    /// Reads pixels from the underlying framebuffer into an image.
    ///
    /// #### Parameters
    /// - `source_rectangle`: Rectangle to read (in device pixels).
    ///
    /// #### Returns
    /// - `Some(RgbaImage)` on success.
    /// - `None` when the readback fails or is rejected by Servo.
    ///
    /// ### 中文
    /// 从底层帧缓冲读取像素并生成图片。
    ///
    /// #### 参数
    /// - `source_rectangle`：读取区域（设备像素）。
    ///
    /// #### 返回
    /// - 成功返回 `Some(RgbaImage)`。
    /// - 读取失败或被 Servo 拒绝时返回 `None`。
    fn read_to_image(&self, source_rectangle: servo::DeviceIntRect) -> Option<RgbaImage> {
        self.framebuffer.borrow().read_to_image(source_rectangle)
    }

    /// ### English
    /// Returns the current physical size.
    ///
    /// #### Returns
    /// - Current size in physical pixels.
    ///
    /// ### 中文
    /// 返回当前物理尺寸。
    ///
    /// #### 返回
    /// - 当前尺寸（物理像素）。
    fn size(&self) -> PhysicalSize<u32> {
        self.size.get()
    }

    /// ### English
    /// Resizes the framebuffer (no-op if unchanged).
    ///
    /// #### Parameters
    /// - `size`: New size in physical pixels.
    ///
    /// ### 中文
    /// 调整帧缓冲尺寸（若无变化则无操作）。
    ///
    /// #### 参数
    /// - `size`：新尺寸（物理像素）。
    fn resize(&self, size: PhysicalSize<u32>) {
        let size = PhysicalSize::new(size.width.max(1), size.height.max(1));
        if self.size.get() == size {
            return;
        }

        *self.framebuffer.borrow_mut() = Framebuffer::new(self.gleam_gl.clone(), size);
        self.size.set(size);
    }

    /// ### English
    /// Presents the current frame.
    ///
    /// This embedder renders into an offscreen texture, so present is a no-op.
    ///
    /// ### 中文
    /// 提交当前帧。
    ///
    /// 本嵌入层渲染到离屏纹理，因此该操作为 no-op。
    fn present(&self) {}

    /// ### English
    /// Returns a Surfman `Connection` required by Servo's paint subsystem.
    ///
    /// On Windows this is effectively a no-op; on Unix/macOS this may connect to the display
    /// server.
    ///
    /// #### Returns
    /// - `Some(Connection)` when created successfully.
    /// - `None` when the connection cannot be created.
    ///
    /// ### 中文
    /// 返回 Servo 绘制子系统所需的 Surfman `Connection`。
    ///
    /// 在 Windows 上该对象基本是 no-op；在 Unix/macOS 上可能会连接到显示服务器。
    ///
    /// #### 返回
    /// - 创建成功时返回 `Some(Connection)`。
    /// - 创建失败时返回 `None`。
    fn connection(&self) -> Option<Connection> {
        SURFMAN_CONNECTION.with(|cell| cell.get_or_init(|| Connection::new().ok()).clone())
    }

    /// ### English
    /// Ensures the embedder context is current.
    ///
    /// #### Returns
    /// - `Ok(())` when the context is current (or assumed current).
    ///
    /// ### 中文
    /// 确保宿主上下文为 current。
    ///
    /// #### 返回
    /// - 上下文为 current（或被假定为 current）时返回 `Ok(())`。
    ///
    fn make_current(&self) -> Result<(), Error> {
        unsafe { self.glfw.make_current() };
        Ok(())
    }

    /// ### English
    /// Returns the Gleam GL entry.
    ///
    /// #### Returns
    /// - Gleam GL API wrapper.
    ///
    /// ### 中文
    /// 返回 Gleam GL 入口。
    ///
    /// #### 返回
    /// - Gleam GL API 封装。
    fn gleam_gl_api(&self) -> Rc<dyn Gl> {
        self.gleam_gl.clone()
    }

    /// ### English
    /// Returns the Glow GL context.
    ///
    /// #### Returns
    /// - Glow GL context.
    ///
    /// ### 中文
    /// 返回 Glow GL 上下文。
    ///
    /// #### 返回
    /// - Glow GL 上下文。
    fn glow_gl_api(&self) -> Arc<glow::Context> {
        self.glow_gl.clone()
    }

    /// ### English
    /// Returns the refresh driver used by Servo.
    ///
    /// #### Returns
    /// - Optional refresh driver.
    ///
    /// ### 中文
    /// 返回 Servo 使用的刷新驱动。
    ///
    /// #### 返回
    /// - 可选的刷新驱动。
    fn refresh_driver(&self) -> Option<Rc<dyn servo::RefreshDriver>> {
        self.refresh_driver.clone()
    }
}
