//! ### English
//! ABI configuration helpers for `xian_web_engine`.
//!
//! Provides simple init functions for config structs so embedders can avoid manual zeroing and can
//! start from consistent defaults.
//!
//! ### 中文
//! `xian_web_engine` 的 ABI 配置辅助方法。
//!
//! 提供配置结构体的初始化函数，避免宿主手动清零，并提供一致的默认值。

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

    unsafe {
        ptr::write_unaligned(
            config,
            XianWebEngineConfig {
                glfw_shared_window: ptr::null_mut(),
                glfw_api: Default::default(),
                resources_dir: ptr::null(),
                config_dir: ptr::null(),
                default_width: 1,
                default_height: 1,
                thread_pool_cap: 0,
                engine_flags: 0,
            },
        );
    }
}

#[unsafe(no_mangle)]
/// ### English
/// Initializes a view config struct with defaults.
///
/// The caller may overwrite any fields after this call.
/// The caller must fill `engine` before calling `xian_web_engine_view_create`.
///
/// Note: `width/height` default to 0, which means "use engine default size".
///
/// #### Safety
/// `config` must be valid for writes of `sizeof(XianWebEngineViewConfig)` bytes.
///
/// ### 中文
/// 使用默认值初始化 view 配置结构体。
///
/// 调用方可在此调用后覆盖任意字段。
/// 调用方在调用 `xian_web_engine_view_create` 前必须填充 `engine`。
///
/// 注意：`width/height` 默认值为 0，表示“使用引擎默认尺寸”。
///
/// #### 安全
/// `config` 必须可写，且至少可写入 `sizeof(XianWebEngineViewConfig)` 字节。
pub unsafe extern "C" fn xian_web_engine_view_config_init(config: *mut XianWebEngineViewConfig) {
    if config.is_null() {
        return;
    }

    unsafe {
        ptr::write_unaligned(
            config,
            XianWebEngineViewConfig {
                engine: ptr::null_mut(),
                width: 0,
                height: 0,
                target_fps: 0,
                view_flags: 0,
            },
        );
    }
}
