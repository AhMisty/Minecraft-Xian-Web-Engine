//! ### English
//! High-performance Servo embedder (`cdylib`).
//!
//! #### Model
//! - The embedder provides one GLFW OpenGL context.
//! - Servo renders into per-view OpenGL textures created in that context.
//!
//! #### Threading
//! - All API calls must happen on the thread where the context is current.
//! - The embedder must call `xian_web_engine_init()` before creating views.
//!
//! ### 中文
//! 高性能 Servo 嵌入层（`cdylib`）。
//!
//! #### 模型
//! - 宿主提供一个 GLFW OpenGL 上下文。
//! - Servo 在该上下文内渲染到每个 view 独立的 OpenGL 纹理。
//!
//! #### 线程
//! - 所有 API 调用必须发生在“上下文为 current”的同一线程。
//! - 创建 view 前宿主必须先调用 `xian_web_engine_init()`。

mod abi;
mod engine;
mod error;
mod ffi;
mod gl;
mod input;
mod protocols;
mod resources;
