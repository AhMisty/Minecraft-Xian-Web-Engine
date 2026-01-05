//! ### English
//! Minimal GLFW wrapper (cross-platform).
//!
//! Used to create a shared offscreen OpenGL context on the Servo thread.
//!
//! ### 中文
//! 最小化的 GLFW 封装（跨平台）。
//!
//! 用于在 Servo 线程创建共享的离屏 OpenGL 上下文。
mod embedder;

pub use embedder::{GlfwWindowPtr, LoadedGlfwApi};

#[repr(C)]
#[derive(Clone, Copy, Default)]
/// ### English
/// Function pointer table for GLFW symbols provided by the embedder (e.g., Java/LWJGL).
///
/// All fields are raw addresses (`usize`) and must be non-zero when used.
///
/// ### 中文
/// 由宿主（例如 Java/LWJGL）提供的 GLFW 符号函数指针表。
///
/// 所有字段都是原始地址（`usize`），在使用时必须全部为非 0。
pub struct EmbedderGlfwApi {
    /// ### English
    /// Pointer to `glfwGetProcAddress`.
    ///
    /// ### 中文
    /// 指向 `glfwGetProcAddress` 的函数指针地址。
    pub glfw_get_proc_address: usize,
    /// ### English
    /// Pointer to `glfwMakeContextCurrent`.
    ///
    /// ### 中文
    /// 指向 `glfwMakeContextCurrent` 的函数指针地址。
    pub glfw_make_context_current: usize,
    /// ### English
    /// Pointer to `glfwDefaultWindowHints`.
    ///
    /// ### 中文
    /// 指向 `glfwDefaultWindowHints` 的函数指针地址。
    pub glfw_default_window_hints: usize,
    /// ### English
    /// Pointer to `glfwWindowHint`.
    ///
    /// ### 中文
    /// 指向 `glfwWindowHint` 的函数指针地址。
    pub glfw_window_hint: usize,
    /// ### English
    /// Pointer to `glfwGetWindowAttrib`.
    ///
    /// ### 中文
    /// 指向 `glfwGetWindowAttrib` 的函数指针地址。
    pub glfw_get_window_attrib: usize,
    /// ### English
    /// Pointer to `glfwCreateWindow`.
    ///
    /// ### 中文
    /// 指向 `glfwCreateWindow` 的函数指针地址。
    pub glfw_create_window: usize,
    /// ### English
    /// Pointer to `glfwDestroyWindow`.
    ///
    /// ### 中文
    /// 指向 `glfwDestroyWindow` 的函数指针地址。
    pub glfw_destroy_window: usize,
}
