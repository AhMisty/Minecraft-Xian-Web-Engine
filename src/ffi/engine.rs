//! ### English
//! C ABI bindings for engine lifecycle (create/destroy/tick).
//!
//! ### 中文
//! 引擎生命周期相关的 C ABI 绑定（create/destroy/tick）。

use std::ffi::{c_char, c_void};

use dpi::PhysicalSize;

use super::XianWebEngine;
use crate::engine::EngineRuntime;

#[unsafe(no_mangle)]
/// ### English
/// Creates an engine bound to a Java-created GLFW OpenGL context.
///
/// `resources_dir` and `config_dir` are optional NUL-terminated UTF-8 strings.
/// Passing NULL or an empty string means "unset".
///
/// `thread_pool_cap` controls the maximum worker threads used by Servo's internal thread pools.
/// - `0` means "no cap" (use CPU parallelism).
/// - Otherwise, Servo thread pools are capped to `min(CPU, thread_pool_cap)`.
///
/// ### 中文
/// 基于 Java 创建的 GLFW OpenGL 上下文创建引擎。
///
/// `resources_dir` 与 `config_dir` 为可选的 NUL 结尾 UTF-8 字符串；
/// 传入 NULL 或空字符串表示“不设置”。
///
/// `thread_pool_cap` 用于限制 Servo 内部线程池的最大工作线程数：
/// - `0` 表示“不封顶”（使用 CPU 并行度）。
/// - 非 0 时，线程池上限为 `min(CPU, thread_pool_cap)`。
pub extern "C" fn xian_web_engine_create(
    glfw_shared_window: *mut c_void,
    default_width: u32,
    default_height: u32,
    resources_dir: *const c_char,
    config_dir: *const c_char,
    thread_pool_cap: u32,
) -> *mut XianWebEngine {
    if glfw_shared_window.is_null() {
        return std::ptr::null_mut();
    }

    let default_size = PhysicalSize::new(default_width.max(1), default_height.max(1));

    let resources_dir = unsafe { super::cstr_to_path(resources_dir) };
    let config_dir = unsafe { super::cstr_to_path(config_dir) };

    let Ok(runtime) = EngineRuntime::new(
        glfw_shared_window,
        default_size,
        resources_dir,
        config_dir,
        thread_pool_cap,
    ) else {
        return std::ptr::null_mut();
    };

    Box::into_raw(Box::new(XianWebEngine { runtime }))
}

#[unsafe(no_mangle)]
/// ### English
/// Destroys an engine created by `xian_web_engine_create`.
///
/// This shuts down the dedicated Servo thread and destroys any remaining views/resources created by
/// this engine. Do not use any views after destroying the engine.
///
/// ### 中文
/// 销毁由 `xian_web_engine_create` 创建的引擎。
///
/// 该操作会关闭 Servo 线程并销毁该引擎创建的所有剩余 view/资源；engine destroy 之后不要再使用任何 view。
pub unsafe extern "C" fn xian_web_engine_destroy(engine: *mut XianWebEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(engine));
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Drains pending vsync callbacks (Java-driven refresh).
///
/// ### 中文
/// 执行待处理的 vsync 回调（由 Java 驱动 refresh）。
pub unsafe extern "C" fn xian_web_engine_tick(engine: *mut XianWebEngine) {
    if engine.is_null() {
        return;
    }

    unsafe { (*engine).runtime.tick() };
}
