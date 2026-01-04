//! ### English
//! Shared GLFW OpenGL context wrapper.
//!
//! Creates an offscreen shared context so the Servo thread can render into textures that the
//! Java/GLFW context can sample.
//!
//! ### 中文
//! 共享 GLFW OpenGL 上下文封装。
//!
//! 创建离屏共享上下文，使 Servo 线程能渲染到纹理，供 Java/GLFW 上下文采样。
use std::cell::Cell;
use std::ffi::{CStr, c_void};
use std::rc::Rc;
use std::sync::Arc;

use gleam::gl::{self, Gl};
use glow::HasContext as _;
use surfman::Connection;

use crate::engine::glfw;

/// ### English
/// Parses an OpenGL version string and returns `(major, minor)`.
///
/// #### Parameters
/// - `version`: Raw OpenGL version string returned by the driver.
///
/// Supported inputs include:
///
/// - `"4.6.0 ..."`
/// - `"OpenGL ES 3.2 ..."`
///
/// ### 中文
/// 解析 OpenGL 版本字符串并返回 `(major, minor)`。
///
/// #### 参数
/// - `version`：驱动返回的原始 OpenGL 版本字符串。
///
/// 支持的输入示例：
///
/// - `"4.6.0 ..."`
/// - `"OpenGL ES 3.2 ..."`
fn parse_gl_version(version: &str) -> (u32, u32) {
    let mut major = 0u32;
    let mut minor = 0u32;
    if let Some(token) = version.split_whitespace().find(|t| {
        t.as_bytes()
            .first()
            .copied()
            .is_some_and(|b| b.is_ascii_digit())
    }) {
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

thread_local! {
    static CURRENT_GLFW_WINDOW: Cell<glfw::GlfwWindowPtr> =
        const { Cell::new(std::ptr::null_mut()) };
}

#[inline]
/// ### English
/// Destroys the offscreen GLFW window/context and clears the per-thread current cache if needed.
///
/// This assumes the caller is on the thread allowed to manipulate the GLFW context.
///
/// #### Parameters
/// - `glfw`: Loaded GLFW API table.
/// - `window`: Offscreen GLFW window to destroy (NULL is a no-op).
///
/// ### 中文
/// 销毁离屏 GLFW window/context，并在需要时清理“每线程 current 缓存”。
///
/// 该函数假设调用方位于允许操作该 GLFW 上下文的线程上。
///
/// #### 参数
/// - `glfw`：已加载的 GLFW API 表。
/// - `window`：要销毁的离屏 window（NULL 则无操作）。
fn destroy_offscreen_window(glfw: &glfw::LoadedGlfwApi, window: glfw::GlfwWindowPtr) {
    if window.is_null() {
        return;
    }

    unsafe {
        glfw.make_current(std::ptr::null_mut());
    }

    CURRENT_GLFW_WINDOW.with(|current| {
        if current.get() == window {
            current.set(std::ptr::null_mut());
        }
    });

    unsafe {
        glfw.destroy_window(window);
    }
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
        let glfw_shared_window = glfw_shared_window as glfw::GlfwWindowPtr;

        let glfw_window = unsafe { glfw.create_shared_offscreen_window(glfw_shared_window)? };

        unsafe {
            glfw.make_current(glfw_window);
        }
        CURRENT_GLFW_WINDOW.with(|current| current.set(glfw_window));

        /// ### English
        /// Drop guard that destroys the offscreen window/context if `new()` fails part-way through.
        ///
        /// ### 中文
        /// 用于 `new()` 中途失败时的 drop guard：负责销毁离屏 window/context。
        struct OffscreenWindowGuard {
            /// ### English
            /// Loaded GLFW API table used to destroy the window.
            ///
            /// ### 中文
            /// 已加载的 GLFW API 表，用于销毁该 window。
            glfw: glfw::LoadedGlfwApi,
            /// ### English
            /// Offscreen window to destroy (NULL means already handed off).
            ///
            /// ### 中文
            /// 需要销毁的离屏 window（NULL 表示已移交）。
            window: glfw::GlfwWindowPtr,
        }

        impl Drop for OffscreenWindowGuard {
            /// ### English
            /// Cleans up the offscreen window if initialization fails before it is handed off.
            ///
            /// ### 中文
            /// 若初始化在移交前失败，则清理离屏 window。
            fn drop(&mut self) {
                destroy_offscreen_window(&self.glfw, self.window);
            }
        }

        let mut offscreen_guard = OffscreenWindowGuard {
            glfw,
            window: glfw_window,
        };

        #[cold]
        #[inline(never)]
        /// ### English
        /// Loads a GL proc by allocating a temporary NUL-terminated buffer on the heap.
        ///
        /// #### Parameters
        /// - `glfw`: Loaded GLFW API table.
        /// - `bytes`: Proc name bytes (must not contain NUL).
        ///
        /// ### 中文
        /// 通过在堆上临时分配 NUL 结尾缓冲区来加载 GL 函数指针。
        ///
        /// #### 参数
        /// - `glfw`：已加载的 GLFW API 表。
        /// - `bytes`：函数名字节（不得包含 NUL）。
        fn load_gl_proc_heap(glfw: &glfw::LoadedGlfwApi, bytes: &[u8]) -> *const c_void {
            if bytes.contains(&0) {
                panic!("gl proc name contains NUL");
            }
            let mut buf = Vec::with_capacity(bytes.len() + 1);
            buf.extend_from_slice(bytes);
            buf.push(0);
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(&buf) };
            unsafe { glfw.get_proc_address(cstr) }
        }

        #[inline]
        /// ### English
        /// Loads a GL proc using a stack buffer when possible (falls back to heap for long names).
        ///
        /// #### Parameters
        /// - `glfw`: Loaded GLFW API table.
        /// - `name`: Proc name (ASCII, must not contain NUL).
        ///
        /// ### 中文
        /// 尽可能使用栈缓冲区加载 GL 函数指针（名称过长时回退到堆分配）。
        ///
        /// #### 参数
        /// - `glfw`：已加载的 GLFW API 表。
        /// - `name`：函数名（ASCII，不得包含 NUL）。
        fn load_gl_proc(glfw: &glfw::LoadedGlfwApi, name: &str) -> *const c_void {
            const STACK_BUF_SIZE: usize = 128;

            let bytes = name.as_bytes();
            if bytes.len() < STACK_BUF_SIZE {
                if bytes.contains(&0) {
                    panic!("gl proc name contains NUL");
                }
                let mut buf = [0u8; STACK_BUF_SIZE];
                buf[..bytes.len()].copy_from_slice(bytes);
                buf[bytes.len()] = 0;
                let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(&buf[..bytes.len() + 1]) };
                unsafe { glfw.get_proc_address(cstr) }
            } else {
                load_gl_proc_heap(glfw, bytes)
            }
        }

        let glow = unsafe { glow::Context::from_loader_function(|name| load_gl_proc(&glfw, name)) };

        let gl_version = unsafe { glow.get_parameter_string(glow::VERSION) };
        let is_gles = gl_version.starts_with("OpenGL ES");
        let (major, minor) = parse_gl_version(&gl_version);
        let srgb_supported = if is_gles {
            major >= 3
        } else {
            major >= 3 || (major == 2 && minor >= 1)
        };

        let gl: Rc<dyn Gl> = unsafe {
            if is_gles {
                gl::GlesFns::load_with(|name| load_gl_proc(&glfw, name))
            } else {
                gl::GlFns::load_with(|name| load_gl_proc(&glfw, name))
            }
        };

        let surfman_connection = Connection::new()
            .map_err(|err| format!("Failed to create surfman Connection: {err:?}"))?;
        offscreen_guard.window = std::ptr::null_mut();

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
    #[inline]
    pub(in crate::engine::rendering) fn make_current(&self) {
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
    #[inline]
    pub(in crate::engine::rendering) fn gl(&self) -> Rc<dyn Gl> {
        self.gl.clone()
    }

    /// ### English
    /// Returns the glow GL API wrapper (cheap clone of an `Arc`).
    ///
    /// ### 中文
    /// 返回 glow GL API 封装（`Arc` 的低成本 clone）。
    #[inline]
    pub(in crate::engine::rendering) fn glow(&self) -> Arc<glow::Context> {
        self.glow.clone()
    }

    /// ### English
    /// Returns a clone of the surfman connection (used by Servo/WebRender integration).
    ///
    /// ### 中文
    /// 返回 surfman connection 的克隆（用于 Servo/WebRender 集成）。
    #[inline]
    pub(in crate::engine::rendering) fn connection(&self) -> Connection {
        self.surfman_connection.clone()
    }

    /// ### English
    /// Returns whether sRGB framebuffer/texture formats are supported.
    ///
    /// ### 中文
    /// 返回是否支持 sRGB framebuffer/纹理格式。
    #[inline]
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
        destroy_offscreen_window(&self.glfw, self.glfw_window);
    }
}
