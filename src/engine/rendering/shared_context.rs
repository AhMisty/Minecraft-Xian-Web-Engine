/// ### English
/// Shared GLFW OpenGL context wrapper.
/// Creates an offscreen shared context so the Servo thread can render into textures that the
/// Java/GLFW context can sample.
///
/// ### 中文
/// 共享 GLFW OpenGL 上下文封装。
/// 创建离屏共享上下文，使 Servo 线程能渲染到纹理，供 Java/GLFW 上下文采样。
use std::cell::Cell;
use std::ffi::{CString, c_void};
use std::rc::Rc;
use std::sync::Arc;

use gleam::gl::{self, Gl};
use glow::HasContext as _;
use surfman::Connection;

use crate::engine::glfw;

fn parse_gl_version(version: &str) -> (u32, u32) {
    /// ### English
    /// Expected forms: `"4.6.0 ..."` or `"OpenGL ES 3.2 ..."`.
    ///
    /// ### 中文
    /// 期望的版本字符串形式：`"4.6.0 ..."` 或 `"OpenGL ES 3.2 ..."`。
    let mut major = 0u32;
    let mut minor = 0u32;
    let tokens: Vec<&str> = version.split_whitespace().collect();
    let number_token = tokens.iter().find(|t| {
        t.chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
    });
    if let Some(token) = number_token {
        let mut parts = token.split('.');
        if let Some(m) = parts.next().and_then(|s| s.parse::<u32>().ok()) {
            major = m;
        }
        if let Some(n) = parts.next().and_then(|s| s.parse::<u32>().ok()) {
            minor = n;
        }
    }
    (major, minor)
}

/// ### English
/// Per-thread "current GLFW window" cache to avoid redundant `makeCurrent` calls.
///
/// ### 中文
/// 每线程缓存“当前 GLFW window”，避免重复 `makeCurrent` 调用。
thread_local! {
    static CURRENT_GLFW_WINDOW: Cell<glfw::GlfwWindowPtr> =
        const { Cell::new(std::ptr::null_mut()) };
}

/// ### English
/// Owns the shared offscreen GLFW window/context and the GL function loaders.
///
/// ### 中文
/// 持有离屏 GLFW window/context 以及 GL 函数加载器。
pub struct GlfwSharedContext {
    /// ### English
    /// Loaded minimal GLFW API used for context control and proc loading.
    ///
    /// ### 中文
    /// 已加载的最小 GLFW API：用于上下文控制与函数指针加载。
    glfw: glfw::LoadedGlfwApi,
    /// ### English
    /// Offscreen GLFW window that owns the shared GL context (current on Servo thread).
    ///
    /// ### 中文
    /// 持有共享 GL 上下文的离屏 GLFW window（在 Servo 线程 current）。
    glfw_window: glfw::GlfwWindowPtr,
    /// ### English
    /// gleam GL API wrapper used by Servo/WebRender.
    ///
    /// ### 中文
    /// Servo/WebRender 使用的 gleam GL API 封装。
    gl: Rc<dyn Gl>,
    /// ### English
    /// glow GL API used for fence/sync operations and modern GL queries.
    ///
    /// ### 中文
    /// 用于 fence/sync 操作与现代 GL 查询的 glow GL API。
    glow: Arc<glow::Context>,
    /// ### English
    /// surfman connection used by Servo to integrate with the platform GL stack.
    ///
    /// ### 中文
    /// Servo 用于与平台 GL 栈集成的 surfman connection。
    surfman_connection: Connection,
    /// ### English
    /// Whether this context supports sRGB framebuffer/texture format.
    ///
    /// ### 中文
    /// 是否支持 sRGB framebuffer/纹理格式。
    srgb_supported: bool,
}

impl GlfwSharedContext {
    /// ### English
    /// Creates an offscreen GLFW window that shares objects with `glfw_shared_window`.
    /// Must be called from the thread that will own the GL context (Servo thread).
    ///
    /// ### 中文
    /// 创建一个与 `glfw_shared_window` 共享 GL 对象的离屏 GLFW window。
    /// 必须在将要持有 GL 上下文的线程（Servo 线程）中调用。
    pub fn new(glfw_shared_window: *mut c_void) -> Result<Rc<Self>, String> {
        let glfw = glfw::LoadedGlfwApi::load()?;
        let glfw_shared_window = glfw_shared_window.cast::<c_void>() as glfw::GlfwWindowPtr;

        let glfw_window = unsafe { glfw.create_shared_offscreen_window(glfw_shared_window)? };

        unsafe {
            glfw.make_current(glfw_window);
        }
        CURRENT_GLFW_WINDOW.with(|current| current.set(glfw_window));

        let glow = unsafe {
            glow::Context::from_loader_function(|name| {
                let cstr = CString::new(name).expect("gl proc name contains NUL");
                glfw.get_proc_address(cstr.as_c_str()) as *const _
            })
        };

        let gl_version = unsafe { glow.get_parameter_string(glow::VERSION) };
        let is_gles = gl_version.starts_with("OpenGL ES");
        let (major, minor) = parse_gl_version(&gl_version);
        /// ### English
        /// Desktop GL: sRGB is core since 3.0; GLES since 3.0. Assume supported for newer versions.
        ///
        /// ### 中文
        /// Desktop GL：sRGB 从 3.0 起为核心特性；GLES：sRGB 从 3.0 起为核心特性。对更高版本直接假设可用。
        let srgb_supported = if is_gles {
            major >= 3
        } else {
            major >= 3 || (major == 2 && minor >= 1)
        };

        let gl: Rc<dyn Gl> = unsafe {
            if is_gles {
                gl::GlesFns::load_with(|name| {
                    let cstr = CString::new(name).expect("gl proc name contains NUL");
                    glfw.get_proc_address(cstr.as_c_str()) as *const _
                })
            } else {
                gl::GlFns::load_with(|name| {
                    let cstr = CString::new(name).expect("gl proc name contains NUL");
                    glfw.get_proc_address(cstr.as_c_str()) as *const _
                })
            }
        };

        let surfman_connection = Connection::new()
            .map_err(|err| format!("Failed to create surfman Connection: {err:?}"))?;

        Ok(Rc::new(Self {
            glfw,
            glfw_window,
            gl,
            glow: Arc::new(glow),
            surfman_connection,
            srgb_supported,
        }))
    }

    /// ### English
    /// Makes this shared context current on the calling thread.
    ///
    /// ### 中文
    /// 使该共享上下文在调用线程上变为 current。
    pub(in crate::engine::rendering) fn make_current_unsafe(&self) {
        CURRENT_GLFW_WINDOW.with(|current| {
            if current.get() == self.glfw_window {
                return;
            }

            unsafe {
                self.glfw.make_current(self.glfw_window);
            }
            current.set(self.glfw_window);
        });
    }

    /// ### English
    /// Returns the gleam GL API wrapper (cheap clone of an `Rc`).
    ///
    /// ### 中文
    /// 返回 gleam GL API 封装（`Rc` 的低成本 clone）。
    pub(in crate::engine::rendering) fn gleam_gl(&self) -> Rc<dyn Gl> {
        self.gl.clone()
    }

    /// ### English
    /// Returns the glow GL API wrapper (cheap clone of an `Arc`).
    ///
    /// ### 中文
    /// 返回 glow GL API 封装（`Arc` 的低成本 clone）。
    pub(in crate::engine::rendering) fn glow_gl(&self) -> Arc<glow::Context> {
        self.glow.clone()
    }

    /// ### English
    /// Returns a clone of the surfman connection (used by Servo/WebRender integration).
    ///
    /// ### 中文
    /// 返回 surfman connection 的克隆（用于 Servo/WebRender 集成）。
    pub(in crate::engine::rendering) fn connection_clone(&self) -> Connection {
        self.surfman_connection.clone()
    }

    /// ### English
    /// Returns whether sRGB framebuffer/texture formats are supported.
    ///
    /// ### 中文
    /// 返回是否支持 sRGB framebuffer/纹理格式。
    pub(in crate::engine::rendering) fn supports_srgb(&self) -> bool {
        self.srgb_supported
    }
}

impl Drop for GlfwSharedContext {
    /// ### English
    /// Ensures the offscreen window/context is destroyed on drop.
    ///
    /// ### 中文
    /// Drop 时销毁离屏 window/context。
    fn drop(&mut self) {
        unsafe {
            self.glfw.make_current(std::ptr::null_mut());
            CURRENT_GLFW_WINDOW.with(|current| {
                if current.get() == self.glfw_window {
                    current.set(std::ptr::null_mut());
                }
            });
            self.glfw.destroy_window(self.glfw_window);
        }
    }
}
