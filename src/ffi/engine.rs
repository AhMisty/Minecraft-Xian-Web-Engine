//! ### English
//! C ABI bindings for engine lifecycle (create/destroy/tick).
//!
//! ### 中文
//! 引擎生命周期相关的 C ABI 绑定（create/destroy/tick）。

use std::ptr;

use dpi::PhysicalSize;

use super::{XianWebEngine, XianWebEngineConfig};
use crate::engine::EngineRuntime;

#[unsafe(no_mangle)]
/// ### English
/// Creates an engine using a config struct.
///
/// Return value: NULL on failure.
///
/// The caller must provide `glfw_shared_window` and `glfw_api` in the config.
///
/// #### Parameters
/// - `config`: Engine configuration (must not be NULL).
///
/// #### Safety
/// - `config` must be valid for reads of at least `sizeof(XianWebEngineConfig)` bytes.
/// - The config header must have a compatible `struct_size` and the correct ABI version.
///
/// ### 中文
/// 使用配置结构体创建引擎。
///
/// 返回值：失败返回 NULL。
///
/// 调用方必须在 config 中提供 `glfw_shared_window` 与 `glfw_api`。
///
/// #### 参数
/// - `config`：引擎配置（必须非 NULL）。
///
/// #### 安全
/// - `config` 必须至少可读 `sizeof(XianWebEngineConfig)` 字节。
/// - 配置头部的 `struct_size` 必须兼容，且 ABI 版本必须匹配。
pub unsafe extern "C" fn xian_web_engine_create(
    config: *const XianWebEngineConfig,
) -> *mut XianWebEngine {
    let Some(config) = (unsafe { super::read_abi_struct(config) }) else {
        return ptr::null_mut();
    };
    if config.glfw_shared_window.is_null() {
        return ptr::null_mut();
    }

    let default_size = PhysicalSize::new(config.default_width.max(1), config.default_height.max(1));

    let resources_dir = unsafe { super::cstr_to_path(config.resources_dir) };
    let config_dir = unsafe { super::cstr_to_path(config.config_dir) };

    let Ok(runtime) = EngineRuntime::new(
        config.glfw_shared_window,
        config.glfw_api,
        default_size,
        resources_dir,
        config_dir,
        config.thread_pool_cap,
        config.engine_flags,
    ) else {
        return ptr::null_mut();
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
/// #### Parameters
/// - `engine`: Engine pointer returned by `xian_web_engine_create` (may be NULL).
///
/// #### Safety
/// If non-NULL, `engine` must be a valid pointer returned by `xian_web_engine_create`, and must not
/// be used after this call returns.
///
/// ### 中文
/// 销毁由 `xian_web_engine_create` 创建的引擎。
///
/// 该操作会关闭 Servo 线程并销毁该引擎创建的所有剩余 view/资源；engine destroy 之后不要再使用任何 view。
///
/// #### 参数
/// - `engine`：由 `xian_web_engine_create` 返回的引擎指针（允许为 NULL）。
///
/// #### 安全
/// 若 `engine` 非 NULL，则它必须是由 `xian_web_engine_create` 返回的有效指针，且本次调用返回后不得再使用。
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
/// #### Parameters
/// - `engine`: Engine pointer returned by `xian_web_engine_create` (may be NULL).
///
/// #### Safety
/// If non-NULL, `engine` must be a valid pointer returned by `xian_web_engine_create`.
///
/// ### 中文
/// 执行待处理的 vsync 回调（由 Java 驱动 refresh）。
///
/// #### 参数
/// - `engine`：由 `xian_web_engine_create` 返回的引擎指针（允许为 NULL）。
///
/// #### 安全
/// 若 `engine` 非 NULL，则它必须是由 `xian_web_engine_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_tick(engine: *mut XianWebEngine) {
    if engine.is_null() {
        return;
    }

    unsafe { (*engine).runtime.tick() };
}
