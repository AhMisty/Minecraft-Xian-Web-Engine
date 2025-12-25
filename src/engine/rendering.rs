//! ### English
//! Rendering module entry point.
//! Splits the shared GLFW context and the triple-buffered rendering context into submodules.
//!
//! ### 中文
//! 渲染模块入口。
//! 将共享 GLFW 上下文与三缓冲渲染上下文拆分到子模块。

mod shared_context;
mod triple_buffer;

pub use shared_context::GlfwSharedContext;
pub use triple_buffer::GlfwTripleBufferRenderingContext;
