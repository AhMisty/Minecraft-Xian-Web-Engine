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
