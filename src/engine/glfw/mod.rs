/// ### English
/// Minimal GLFW dynamic loader (Windows-only in this crate).
/// Used to create a shared offscreen OpenGL context on the Servo thread.
///
/// ### 中文
/// 最小化的 GLFW 动态加载器（本 crate 目前仅 Windows）。
/// 用于在 Servo 线程创建共享的离屏 OpenGL 上下文。
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
mod stub;

#[cfg(windows)]
pub use windows::{GlfwWindowPtr, LoadedGlfwApi};

#[cfg(not(windows))]
pub use stub::{GlfwWindowPtr, LoadedGlfwApi};

#[repr(C)]
#[derive(Clone, Copy, Default)]
/// ### English
/// Function pointer table for GLFW symbols provided by the embedder (e.g., Java/LWJGL).
///
/// All fields are raw addresses (`usize`) and must be non-zero when installing.
///
/// ### 中文
/// 由宿主（例如 Java/LWJGL）提供的 GLFW 符号函数指针表。
///
/// 所有字段都是原始地址（`usize`），安装时必须全部为非 0。
pub struct EmbedderGlfwApi {
    pub glfw_get_proc_address: usize,
    pub glfw_make_context_current: usize,
    pub glfw_default_window_hints: usize,
    pub glfw_window_hint: usize,
    pub glfw_get_window_attrib: usize,
    pub glfw_create_window: usize,
    pub glfw_destroy_window: usize,
}

/// ### English
/// Installs an embedder-provided GLFW function table used by the internal loader.
/// Must be called before any engine creation / Servo thread initialization.
///
/// ### 中文
/// 安装由宿主提供的 GLFW 函数表（供内部 loader 使用）。
/// 必须在创建引擎 / 初始化 Servo 线程之前调用。
pub(crate) fn install_embedder_glfw_api(api: EmbedderGlfwApi) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows::install_embedder_glfw_api(api)
    }

    #[cfg(not(windows))]
    {
        let _ = api;
        Err("Embedder-provided GLFW API is only supported on Windows in this crate".to_string())
    }
}
