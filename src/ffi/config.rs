//! ### English
//! ABI configuration helpers for `xian_web_engine`.
//!
//! Provides simple init functions for config structs so embedders can avoid manual zeroing and can
//! always fill required `struct_size`/`abi_version` fields.
//!
//! ### 中文
//! `xian_web_engine` 的 ABI 配置辅助方法。
//!
//! 提供配置结构体的初始化函数，避免宿主手动清零，并统一填写必需的 `struct_size`/`abi_version` 字段。

use std::mem::size_of;
use std::ptr;

use super::{XianWebEngineConfig, XianWebEngineViewConfig};

#[unsafe(no_mangle)]
/// ### English
/// Initializes an engine config struct with defaults.
///
/// The caller may overwrite any fields after this call.
/// The caller must fill `glfw_shared_window` and `glfw_api` before calling `xian_web_engine_create`.
///
/// #### Safety
/// `config` must be valid for writes of `sizeof(XianWebEngineConfig)` bytes.
///
/// ### 中文
/// 使用默认值初始化引擎配置结构体。
///
/// 调用方可在此调用后覆盖任意字段。
/// 调用方在调用 `xian_web_engine_create` 前必须填充 `glfw_shared_window` 与 `glfw_api`。
///
/// #### 安全
/// `config` 必须可写，且至少可写入 `sizeof(XianWebEngineConfig)` 字节。
pub unsafe extern "C" fn xian_web_engine_config_init(config: *mut XianWebEngineConfig) {
    if config.is_null() {
        return;
    }

    let value = XianWebEngineConfig {
        struct_size: size_of::<XianWebEngineConfig>() as u32,
        abi_version: super::XIAN_WEB_ENGINE_ABI_VERSION,
        glfw_shared_window: ptr::null_mut(),
        glfw_api: Default::default(),
        default_width: 1,
        default_height: 1,
        thread_pool_cap: 0,
        engine_flags: 0,
        resources_dir: ptr::null(),
        config_dir: ptr::null(),
    };

    unsafe {
        ptr::write_unaligned(config, value);
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Initializes a view config struct with defaults.
///
/// The caller may overwrite any fields after this call.
///
/// #### Safety
/// `config` must be valid for writes of `sizeof(XianWebEngineViewConfig)` bytes.
///
/// ### 中文
/// 使用默认值初始化 view 配置结构体。
///
/// 调用方可在此调用后覆盖任意字段。
///
/// #### 安全
/// `config` 必须可写，且至少可写入 `sizeof(XianWebEngineViewConfig)` 字节。
pub unsafe extern "C" fn xian_web_engine_view_config_init(config: *mut XianWebEngineViewConfig) {
    if config.is_null() {
        return;
    }

    let value = XianWebEngineViewConfig {
        struct_size: size_of::<XianWebEngineViewConfig>() as u32,
        abi_version: super::XIAN_WEB_ENGINE_ABI_VERSION,
        engine: ptr::null_mut(),
        width: 1,
        height: 1,
        target_fps: 0,
        view_flags: 0,
    };

    unsafe {
        ptr::write_unaligned(config, value);
    }
}
