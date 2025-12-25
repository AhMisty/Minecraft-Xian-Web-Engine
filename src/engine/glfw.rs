//! ### English
//! Minimal GLFW dynamic loader (Windows-only in this crate).
//! Used to create a shared offscreen OpenGL context on the Servo thread.
//!
//! ### 中文
//! 最小化的 GLFW 动态加载器（本 crate 目前仅 Windows）。
//! 用于在 Servo 线程创建共享的离屏 OpenGL 上下文。

use std::ffi::{CStr, CString, c_char, c_int, c_void};

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt as _;

    #[repr(C)]
    pub struct GLFWwindow {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct GLFWmonitor {
        _private: [u8; 0],
    }

    type GLFWglproc = *const c_void;
    type GlfwGetProcAddress = unsafe extern "C" fn(*const c_char) -> GLFWglproc;
    type GlfwMakeContextCurrent = unsafe extern "C" fn(*mut GLFWwindow);
    type GlfwDefaultWindowHints = unsafe extern "C" fn();
    type GlfwWindowHint = unsafe extern "C" fn(c_int, c_int);
    type GlfwGetWindowAttrib = unsafe extern "C" fn(*mut GLFWwindow, c_int) -> c_int;
    type GlfwCreateWindow = unsafe extern "C" fn(
        c_int,
        c_int,
        *const c_char,
        *mut GLFWmonitor,
        *mut GLFWwindow,
    ) -> *mut GLFWwindow;
    type GlfwDestroyWindow = unsafe extern "C" fn(*mut GLFWwindow);

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
        fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const c_char) -> *mut c_void;
        fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
    }

    fn to_wide_nul(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    unsafe fn get_symbol<T>(module: *mut c_void, name: &CStr) -> Option<T> {
        let addr = unsafe { GetProcAddress(module, name.as_ptr()) };
        if addr.is_null() {
            return None;
        }
        Some(unsafe { std::mem::transmute_copy(&addr) })
    }

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
        /// The embedder is expected to have already loaded GLFW; we try common DLL names and
        /// fall back to `LoadLibraryW`.
        ///
        /// ### 中文
        /// 加载本 crate 所需的最小 GLFW 符号集合。
        ///
        /// 宿主通常已加载 GLFW；这里会尝试常见 DLL 名称，并在需要时回退到 `LoadLibraryW`。
        pub fn load() -> Result<Self, String> {
            let module = unsafe {
                let glfw3 = to_wide_nul("glfw3.dll");
                let glfw = to_wide_nul("glfw.dll");

                let mut module = GetModuleHandleW(glfw3.as_ptr());
                if module.is_null() {
                    module = GetModuleHandleW(glfw.as_ptr());
                }
                if module.is_null() {
                    module = LoadLibraryW(glfw3.as_ptr());
                }
                if module.is_null() {
                    module = LoadLibraryW(glfw.as_ptr());
                }
                if module.is_null() {
                    return Err(
                        "Failed to load glfw3.dll/glfw.dll; ensure GLFW is loaded by the embedder"
                            .to_string(),
                    );
                }
                module
            };

            let glfw_get_proc_address: GlfwGetProcAddress = unsafe {
                get_symbol(module, c"glfwGetProcAddress")
                    .ok_or_else(|| "glfwGetProcAddress not found".to_string())?
            };
            let glfw_make_context_current: GlfwMakeContextCurrent = unsafe {
                get_symbol(module, c"glfwMakeContextCurrent")
                    .ok_or_else(|| "glfwMakeContextCurrent not found".to_string())?
            };
            let glfw_default_window_hints: GlfwDefaultWindowHints = unsafe {
                get_symbol(module, c"glfwDefaultWindowHints")
                    .ok_or_else(|| "glfwDefaultWindowHints not found".to_string())?
            };
            let glfw_window_hint: GlfwWindowHint = unsafe {
                get_symbol(module, c"glfwWindowHint")
                    .ok_or_else(|| "glfwWindowHint not found".to_string())?
            };
            let glfw_get_window_attrib: GlfwGetWindowAttrib = unsafe {
                get_symbol(module, c"glfwGetWindowAttrib")
                    .ok_or_else(|| "glfwGetWindowAttrib not found".to_string())?
            };
            let glfw_create_window: GlfwCreateWindow = unsafe {
                get_symbol(module, c"glfwCreateWindow")
                    .ok_or_else(|| "glfwCreateWindow not found".to_string())?
            };
            let glfw_destroy_window: GlfwDestroyWindow = unsafe {
                get_symbol(module, c"glfwDestroyWindow")
                    .ok_or_else(|| "glfwDestroyWindow not found".to_string())?
            };

            Ok(Self {
                glfw_get_proc_address,
                glfw_make_context_current,
                glfw_default_window_hints,
                glfw_window_hint,
                glfw_get_window_attrib,
                glfw_create_window,
                glfw_destroy_window,
            })
        }

        /// ### English
        /// Makes `window` current on the calling thread.
        ///
        /// ### 中文
        /// 将 `window` 设置为调用线程的 current 上下文。
        pub unsafe fn make_current(&self, window: *mut GLFWwindow) {
            unsafe { (self.glfw_make_context_current)(window) };
        }

        /// ### English
        /// Loads an OpenGL function pointer via GLFW.
        ///
        /// ### 中文
        /// 通过 GLFW 加载 OpenGL 函数指针。
        pub unsafe fn get_proc_address(&self, name: &CStr) -> *const c_void {
            unsafe { (self.glfw_get_proc_address)(name.as_ptr()) }
        }

        /// ### English
        /// Destroys a GLFW window created by this loader.
        ///
        /// ### 中文
        /// 销毁由本 loader 创建的 GLFW window。
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

            let shared_client_api =
                unsafe { (self.glfw_get_window_attrib)(share, GLFW_CLIENT_API) };
            let shared_major =
                unsafe { (self.glfw_get_window_attrib)(share, GLFW_CONTEXT_VERSION_MAJOR) };
            let shared_minor =
                unsafe { (self.glfw_get_window_attrib)(share, GLFW_CONTEXT_VERSION_MINOR) };
            let shared_profile =
                unsafe { (self.glfw_get_window_attrib)(share, GLFW_OPENGL_PROFILE) };
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

            let title = CString::new("xian_web_engine-offscreen")
                .map_err(|_| "Failed to build offscreen window title".to_string())?;
            let window = unsafe {
                (self.glfw_create_window)(1, 1, title.as_ptr(), std::ptr::null_mut(), share)
            };
            unsafe { (self.glfw_default_window_hints)() };

            if window.is_null() {
                return Err(
                    "glfwCreateWindow failed; ensure the shared window's context is valid"
                        .to_string(),
                );
            }
            Ok(window)
        }
    }

    pub type GlfwWindowPtr = *mut GLFWwindow;
    pub type LoadedGlfwApi = GlfwApi;
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub type GlfwWindowPtr = *mut c_void;

    /// ### English
    /// Placeholder GLFW loader for non-Windows builds (this crate currently targets Windows).
    ///
    /// ### 中文
    /// 非 Windows 构建的占位 GLFW loader（本 crate 当前主要目标为 Windows）。
    pub struct LoadedGlfwApi;

    impl LoadedGlfwApi {
        /// ### English
        /// Always returns an error on non-Windows builds.
        ///
        /// ### 中文
        /// 在非 Windows 构建下总是返回错误。
        pub fn load() -> Result<Self, String> {
            Err("GLFW dynamic loading is only implemented on Windows in this crate".to_string())
        }

        /// ### English
        /// No-op on non-Windows builds.
        ///
        /// ### 中文
        /// 非 Windows 构建下为 no-op。
        pub unsafe fn make_current(&self, _window: GlfwWindowPtr) {}

        /// ### English
        /// Always returns NULL on non-Windows builds.
        ///
        /// ### 中文
        /// 非 Windows 构建下总是返回 NULL。
        pub unsafe fn get_proc_address(&self, _name: &CStr) -> *const c_void {
            std::ptr::null()
        }

        /// ### English
        /// No-op on non-Windows builds.
        ///
        /// ### 中文
        /// 非 Windows 构建下为 no-op。
        pub unsafe fn destroy_window(&self, _window: GlfwWindowPtr) {}

        /// ### English
        /// Always returns an error on non-Windows builds.
        ///
        /// ### 中文
        /// 非 Windows 构建下总是返回错误。
        pub unsafe fn create_shared_offscreen_window(
            &self,
            _share: GlfwWindowPtr,
        ) -> Result<GlfwWindowPtr, String> {
            Err(
                "GLFW offscreen window creation is only implemented on Windows in this crate"
                    .to_string(),
            )
        }
    }
}

pub use imp::{GlfwWindowPtr, LoadedGlfwApi};
