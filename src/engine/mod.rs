//! ### English
//! Engine internal modules (threading, rendering, input, and shared frame state).
//!
//! ### 中文
//! 引擎内部模块（线程、渲染、输入、共享帧状态等）。
pub(crate) mod cache;
mod flags;
mod frame;
mod glfw;
mod input;
mod input_types;
pub(crate) mod lockfree;
mod refresh;
mod rendering;
mod resources;
mod runtime;
mod vsync;

pub(crate) use frame::AcquiredFrame;
pub(crate) use glfw::{EmbedderGlfwApi, install_embedder_glfw_api};
pub(crate) use input_types::{
    XIAN_WEB_ENGINE_INPUT_KIND_KEY, XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON,
    XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE, XIAN_WEB_ENGINE_INPUT_KIND_WHEEL,
    XianWebEngineInputEvent,
};
pub(crate) use runtime::{EngineRuntime, WebEngineViewHandle};
