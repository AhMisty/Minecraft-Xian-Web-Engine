//! ### English
//! High-performance Servo embedder (`cdylib`).
//!
//! - The embedder provides an external GLFW OpenGL context (usually the game's context).
//! - Servo renders directly in that context into per-view textures owned by this crate.
//!
//! #### Design goals
//! - No shared/offscreen GLFW windows
//! - No cross-thread context sharing/copying
//! - Single-threaded public API (minimal synchronization)
//!
//! ### 中文
//! 高性能 Servo 嵌入层（`cdylib`）。
//!
//! - 宿主提供外部 GLFW OpenGL 上下文（通常为游戏的上下文）。
//! - Servo 直接在该上下文内渲染到本库持有的每个 view 纹理。
//!
//! #### 设计目标
//! - 不创建共享/离屏 GLFW window
//! - 不进行跨线程上下文共享/拷贝
//! - 对外 API 单线程（最小化同步开销）

mod engine;
mod error;
mod ffi;
mod gl;
mod input;
mod resources;
