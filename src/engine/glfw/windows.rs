//! ### English
//! Windows implementation of the minimal GLFW symbol loader.
//!
//! Uses an embedder-provided function table (`EmbedderGlfwApi`) instead of dynamic library lookup.
//!
//! ### 中文
//! 最小 GLFW 符号 loader 的 Windows 实现。
//!
//! 使用宿主提供的函数表（`EmbedderGlfwApi`），不做动态库按名查找。

use std::ffi::{CStr, c_char, c_int, c_void};
use std::sync::OnceLock;

#[repr(C)]
/// ### English
/// Opaque GLFW window type (`GLFWwindow`).
///
/// ### 中文
/// 不透明 GLFW window 类型（`GLFWwindow`）。
pub struct GLFWwindow {
    /// ### English
    /// Opaque zero-sized marker to prevent construction.
    ///
    /// ### 中文
    /// 不透明的零大小占位字段，用于阻止外部构造。
    _private: [u8; 0],
}

#[repr(C)]
/// ### English
/// Opaque GLFW monitor type (`GLFWmonitor`).
///
/// ### 中文
/// 不透明 GLFW monitor 类型（`GLFWmonitor`）。
pub struct GLFWmonitor {
    /// ### English
    /// Opaque zero-sized marker to prevent construction.
    ///
    /// ### 中文
    /// 不透明的零大小占位字段，用于阻止外部构造。
    _private: [u8; 0],
}

/// ### English
/// Raw OpenGL function pointer type returned by `glfwGetProcAddress`.
///
/// ### 中文
/// `glfwGetProcAddress` 返回的原始 OpenGL 函数指针类型。
type GLFWglproc = *const c_void;
/// ### English
/// Function pointer type for `glfwGetProcAddress`.
///
/// ### 中文
/// `glfwGetProcAddress` 的函数指针类型。
type GlfwGetProcAddress = unsafe extern "C" fn(*const c_char) -> GLFWglproc;
/// ### English
/// Function pointer type for `glfwMakeContextCurrent`.
///
/// ### 中文
/// `glfwMakeContextCurrent` 的函数指针类型。
type GlfwMakeContextCurrent = unsafe extern "C" fn(*mut GLFWwindow);
/// ### English
/// Function pointer type for `glfwDefaultWindowHints`.
///
/// ### 中文
/// `glfwDefaultWindowHints` 的函数指针类型。
type GlfwDefaultWindowHints = unsafe extern "C" fn();
/// ### English
/// Function pointer type for `glfwWindowHint`.
///
/// ### 中文
/// `glfwWindowHint` 的函数指针类型。
type GlfwWindowHint = unsafe extern "C" fn(c_int, c_int);
/// ### English
/// Function pointer type for `glfwGetWindowAttrib`.
///
/// ### 中文
/// `glfwGetWindowAttrib` 的函数指针类型。
type GlfwGetWindowAttrib = unsafe extern "C" fn(*mut GLFWwindow, c_int) -> c_int;
/// ### English
/// Function pointer type for `glfwCreateWindow`.
///
/// ### 中文
/// `glfwCreateWindow` 的函数指针类型。
type GlfwCreateWindow = unsafe extern "C" fn(
    c_int,
    c_int,
    *const c_char,
    *mut GLFWmonitor,
    *mut GLFWwindow,
) -> *mut GLFWwindow;
/// ### English
/// Function pointer type for `glfwDestroyWindow`.
///
/// ### 中文
/// `glfwDestroyWindow` 的函数指针类型。
type GlfwDestroyWindow = unsafe extern "C" fn(*mut GLFWwindow);

static EMBEDDER_GLFW_API: OnceLock<GlfwApi> = OnceLock::new();

/// ### English
/// Installs the embedder-provided GLFW function table for this process.
///
/// This is a one-time installation backed by `OnceLock`; repeated calls return an error.
///
/// #### Parameters
/// - `api`: Embedder function pointer table for required GLFW symbols.
///
/// ### 中文
/// 为当前进程安装宿主提供的 GLFW 函数表。
///
/// 该安装由 `OnceLock` 保证只执行一次；重复调用会返回错误。
///
/// #### 参数
/// - `api`：宿主提供的 GLFW 必需符号函数指针表。
pub(super) fn install_embedder_glfw_api(api: super::EmbedderGlfwApi) -> Result<(), String> {
    if api.glfw_get_proc_address == 0 {
        return Err("EmbedderGlfwApi.glfw_get_proc_address is NULL".to_string());
    }
    if api.glfw_make_context_current == 0 {
        return Err("EmbedderGlfwApi.glfw_make_context_current is NULL".to_string());
    }
    if api.glfw_default_window_hints == 0 {
        return Err("EmbedderGlfwApi.glfw_default_window_hints is NULL".to_string());
    }
    if api.glfw_window_hint == 0 {
        return Err("EmbedderGlfwApi.glfw_window_hint is NULL".to_string());
    }
    if api.glfw_get_window_attrib == 0 {
        return Err("EmbedderGlfwApi.glfw_get_window_attrib is NULL".to_string());
    }
    if api.glfw_create_window == 0 {
        return Err("EmbedderGlfwApi.glfw_create_window is NULL".to_string());
    }
    if api.glfw_destroy_window == 0 {
        return Err("EmbedderGlfwApi.glfw_destroy_window is NULL".to_string());
    }

    let table = GlfwApi {
        glfw_get_proc_address: unsafe {
            std::mem::transmute::<usize, GlfwGetProcAddress>(api.glfw_get_proc_address)
        },
        glfw_make_context_current: unsafe {
            std::mem::transmute::<usize, GlfwMakeContextCurrent>(api.glfw_make_context_current)
        },
        glfw_default_window_hints: unsafe {
            std::mem::transmute::<usize, GlfwDefaultWindowHints>(api.glfw_default_window_hints)
        },
        glfw_window_hint: unsafe {
            std::mem::transmute::<usize, GlfwWindowHint>(api.glfw_window_hint)
        },
        glfw_get_window_attrib: unsafe {
            std::mem::transmute::<usize, GlfwGetWindowAttrib>(api.glfw_get_window_attrib)
        },
        glfw_create_window: unsafe {
            std::mem::transmute::<usize, GlfwCreateWindow>(api.glfw_create_window)
        },
        glfw_destroy_window: unsafe {
            std::mem::transmute::<usize, GlfwDestroyWindow>(api.glfw_destroy_window)
        },
    };

    EMBEDDER_GLFW_API
        .set(table)
        .map_err(|_| "Embedder GLFW API is already installed".to_string())?;
    Ok(())
}

#[derive(Clone, Copy)]
/// ### English
/// Loaded minimal GLFW API used by the engine (context control + proc loading).
///
/// ### 中文
/// 引擎使用的最小 GLFW API（上下文控制 + 函数指针加载）。
pub struct GlfwApi {
    /// ### English
    /// Function pointer: `glfwGetProcAddress`.
    ///
    /// ### 中文
    /// 函数指针：`glfwGetProcAddress`。
    glfw_get_proc_address: GlfwGetProcAddress,
    /// ### English
    /// Function pointer: `glfwMakeContextCurrent`.
    ///
    /// ### 中文
    /// 函数指针：`glfwMakeContextCurrent`。
    glfw_make_context_current: GlfwMakeContextCurrent,
    /// ### English
    /// Function pointer: `glfwDefaultWindowHints`.
    ///
    /// ### 中文
    /// 函数指针：`glfwDefaultWindowHints`。
    glfw_default_window_hints: GlfwDefaultWindowHints,
    /// ### English
    /// Function pointer: `glfwWindowHint`.
    ///
    /// ### 中文
    /// 函数指针：`glfwWindowHint`。
    glfw_window_hint: GlfwWindowHint,
    /// ### English
    /// Function pointer: `glfwGetWindowAttrib`.
    ///
    /// ### 中文
    /// 函数指针：`glfwGetWindowAttrib`。
    glfw_get_window_attrib: GlfwGetWindowAttrib,
    /// ### English
    /// Function pointer: `glfwCreateWindow`.
    ///
    /// ### 中文
    /// 函数指针：`glfwCreateWindow`。
    glfw_create_window: GlfwCreateWindow,
    /// ### English
    /// Function pointer: `glfwDestroyWindow`.
    ///
    /// ### 中文
    /// 函数指针：`glfwDestroyWindow`。
    glfw_destroy_window: GlfwDestroyWindow,
}

impl GlfwApi {
    /// ### English
    /// Loads the minimal subset of GLFW symbols required by this crate.
    ///
    /// An embedder-provided function table must be installed via
    /// `xian_web_engine_set_glfw_api` before calling this.
    ///
    /// ### 中文
    /// 加载本 crate 所需的最小 GLFW 符号集合。
    ///
    /// 调用前必须由宿主通过 `xian_web_engine_set_glfw_api` 安装函数表。
    #[inline]
    pub fn load() -> Result<Self, String> {
        EMBEDDER_GLFW_API.get().copied().ok_or_else(|| {
            "Embedder GLFW API is not installed; call xian_web_engine_set_glfw_api before xian_web_engine_create"
                .to_string()
        })
    }

    /// ### English
    /// Makes `window` current on the calling thread.
    ///
    /// ### 中文
    /// 将 `window` 设为调用线程的 current 上下文。
    #[inline]
    pub unsafe fn make_current(&self, window: *mut GLFWwindow) {
        unsafe { (self.glfw_make_context_current)(window) };
    }

    /// ### English
    /// Loads an OpenGL function pointer via GLFW.
    ///
    /// #### Parameters
    /// - `name`: NUL-terminated proc name.
    ///
    /// ### 中文
    /// 通过 GLFW 加载 OpenGL 函数指针。
    ///
    /// #### 参数
    /// - `name`：以 NUL 结尾的函数名。
    #[inline]
    pub unsafe fn get_proc_address(&self, name: &CStr) -> *const c_void {
        unsafe { (self.glfw_get_proc_address)(name.as_ptr()) }
    }

    /// ### English
    /// Destroys a GLFW window created by this loader.
    ///
    /// #### Parameters
    /// - `window`: Window pointer to destroy.
    ///
    /// ### 中文
    /// 销毁由本 loader 创建的 GLFW window。
    ///
    /// #### 参数
    /// - `window`：需要销毁的 window 指针。
    #[inline]
    pub unsafe fn destroy_window(&self, window: *mut GLFWwindow) {
        unsafe { (self.glfw_destroy_window)(window) };
    }

    /// ### English
    /// Creates an invisible 1x1 offscreen window whose GL context shares objects with `share`.
    ///
    /// This is used so the Servo thread can render into textures that the Java context can
    /// sample (shared objects within the same share group).
    ///
    /// ### 中文
    /// 创建一个不可见的 1x1 离屏 window，使其 GL 上下文与 `share` 共享对象。
    ///
    /// 用于让 Servo 线程渲染到纹理，并由 Java 上下文采样（同一 share group 共享对象）。
    pub unsafe fn create_shared_offscreen_window(
        &self,
        share: *mut GLFWwindow,
    ) -> Result<*mut GLFWwindow, String> {
        const GLFW_FALSE: c_int = 0;

        const GLFW_VISIBLE: c_int = 0x0002_0004;
        const GLFW_FOCUSED: c_int = 0x0002_0001;
        const GLFW_RESIZABLE: c_int = 0x0002_0003;

        const GLFW_CLIENT_API: c_int = 0x0002_2001;
        const GLFW_CONTEXT_VERSION_MAJOR: c_int = 0x0002_2002;
        const GLFW_CONTEXT_VERSION_MINOR: c_int = 0x0002_2003;
        const GLFW_OPENGL_FORWARD_COMPAT: c_int = 0x0002_2006;
        const GLFW_OPENGL_DEBUG_CONTEXT: c_int = 0x0002_2007;
        const GLFW_OPENGL_PROFILE: c_int = 0x0002_2008;
        const GLFW_CONTEXT_CREATION_API: c_int = 0x0002_200B;

        let shared_client_api = unsafe { (self.glfw_get_window_attrib)(share, GLFW_CLIENT_API) };
        let shared_major =
            unsafe { (self.glfw_get_window_attrib)(share, GLFW_CONTEXT_VERSION_MAJOR) };
        let shared_minor =
            unsafe { (self.glfw_get_window_attrib)(share, GLFW_CONTEXT_VERSION_MINOR) };
        let shared_profile = unsafe { (self.glfw_get_window_attrib)(share, GLFW_OPENGL_PROFILE) };
        let shared_forward =
            unsafe { (self.glfw_get_window_attrib)(share, GLFW_OPENGL_FORWARD_COMPAT) };
        let shared_debug =
            unsafe { (self.glfw_get_window_attrib)(share, GLFW_OPENGL_DEBUG_CONTEXT) };
        let shared_creation_api =
            unsafe { (self.glfw_get_window_attrib)(share, GLFW_CONTEXT_CREATION_API) };

        unsafe { (self.glfw_default_window_hints)() };
        unsafe { (self.glfw_window_hint)(GLFW_VISIBLE, GLFW_FALSE) };
        unsafe { (self.glfw_window_hint)(GLFW_FOCUSED, GLFW_FALSE) };
        unsafe { (self.glfw_window_hint)(GLFW_RESIZABLE, GLFW_FALSE) };

        if shared_client_api != 0 {
            unsafe { (self.glfw_window_hint)(GLFW_CLIENT_API, shared_client_api) };
        }
        if shared_major > 0 {
            unsafe { (self.glfw_window_hint)(GLFW_CONTEXT_VERSION_MAJOR, shared_major) };
        }
        if shared_minor > 0 {
            unsafe { (self.glfw_window_hint)(GLFW_CONTEXT_VERSION_MINOR, shared_minor) };
        }
        if shared_profile != 0 {
            unsafe { (self.glfw_window_hint)(GLFW_OPENGL_PROFILE, shared_profile) };
        }
        unsafe { (self.glfw_window_hint)(GLFW_OPENGL_FORWARD_COMPAT, shared_forward) };
        unsafe { (self.glfw_window_hint)(GLFW_OPENGL_DEBUG_CONTEXT, shared_debug) };
        if shared_creation_api != 0 {
            unsafe { (self.glfw_window_hint)(GLFW_CONTEXT_CREATION_API, shared_creation_api) };
        }

        let title = c"xian_web_engine-offscreen";
        let window =
            unsafe { (self.glfw_create_window)(1, 1, title.as_ptr(), std::ptr::null_mut(), share) };
        unsafe { (self.glfw_default_window_hints)() };

        if window.is_null() {
            return Err(
                "glfwCreateWindow failed; ensure the shared window's context is valid".to_string(),
            );
        }
        Ok(window)
    }
}

/// ### English
/// Raw window pointer type used by this crate (alias for `*mut GLFWwindow`).
///
/// ### 中文
/// 本 crate 使用的 window 裸指针类型（`*mut GLFWwindow` 的别名）。
pub type GlfwWindowPtr = *mut GLFWwindow;
/// ### English
/// Public alias for the loaded GLFW API type.
///
/// ### 中文
/// 已加载 GLFW API 类型的公开别名。
pub type LoadedGlfwApi = GlfwApi;
