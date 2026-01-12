//! ### English
//! `xian_web_engine` is a high-performance Servo embedder as a `cdylib`.
//!
//! The embedder provides a GLFW OpenGL context (usually the game's context). Servo renders
//! directly inside that context into textures owned by this crate.
//!
//! Design goals:
//! - No shared/offscreen GLFW windows
//! - No context sharing/copying between threads
//! - Minimal cross-thread synchronization (single-threaded public API)
//!
//! ### 中文
//! `xian_web_engine` 是一个以最高性能为目标的 Servo 嵌入层（`cdylib`）。
//!
//! 宿主提供 GLFW OpenGL 上下文（通常是游戏自己的上下文）。Servo 直接在该上下文内渲染到本库创建并持有的纹理。
//!
//! 设计目标：
//! - 不创建共享/离屏 GLFW window
//! - 不做跨线程的上下文共享/拷贝
//! - 对外 API 单线程（尽量减少同步成本）

mod abi;
mod engine;
mod error;
mod input;
mod rendering;
mod resources;
