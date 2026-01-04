//! ### English
//! Non-Windows placeholder implementation for the internal GLFW loader.
//!
//! ### 中文
//! 内部 GLFW loader 的非 Windows 占位实现。

use std::ffi::{CStr, c_void};

/// ### English
/// Raw window pointer type used by this crate on non-Windows targets.
///
/// ### 中文
/// 本 crate 在非 Windows 目标上的 window 裸指针类型。
pub type GlfwWindowPtr = *mut c_void;

/// ### English
/// Placeholder GLFW loader for non-Windows builds.
///
/// ### 中文
/// 非 Windows 构建的占位 GLFW loader。
pub struct LoadedGlfwApi;

impl LoadedGlfwApi {
    /// ### English
    /// Always returns an error on non-Windows builds.
    ///
    /// ### 中文
    /// 在非 Windows 构建下总是返回错误。
    pub fn load() -> Result<Self, String> {
        Err("GLFW dynamic loading is only implemented on Windows in this crate".to_string())
    }

    /// ### English
    /// No-op on non-Windows builds.
    ///
    /// #### Parameters
    /// - `_window`: Window handle (ignored on this stub implementation).
    ///
    /// ### 中文
    /// 非 Windows 构建下为 no-op。
    ///
    /// #### 参数
    /// - `_window`：window 句柄（该占位实现中忽略）。
    pub unsafe fn make_current(&self, _window: GlfwWindowPtr) {}

    /// ### English
    /// Always returns NULL on non-Windows builds.
    ///
    /// #### Parameters
    /// - `_name`: Function name (ignored on this stub implementation).
    ///
    /// ### 中文
    /// 非 Windows 构建下总是返回 NULL。
    ///
    /// #### 参数
    /// - `_name`：函数名（该占位实现中忽略）。
    pub unsafe fn get_proc_address(&self, _name: &CStr) -> *const c_void {
        std::ptr::null()
    }

    /// ### English
    /// No-op on non-Windows builds.
    ///
    /// #### Parameters
    /// - `_window`: Window handle (ignored on this stub implementation).
    ///
    /// ### 中文
    /// 非 Windows 构建下为 no-op。
    ///
    /// #### 参数
    /// - `_window`：window 句柄（该占位实现中忽略）。
    pub unsafe fn destroy_window(&self, _window: GlfwWindowPtr) {}

    /// ### English
    /// Always returns an error on non-Windows builds.
    ///
    /// #### Parameters
    /// - `_share`: Window handle whose context would be shared (ignored on this stub implementation).
    ///
    /// ### 中文
    /// 非 Windows 构建下总是返回错误。
    ///
    /// #### 参数
    /// - `_share`：用于共享上下文的 window 句柄（该占位实现中忽略）。
    pub unsafe fn create_shared_offscreen_window(
        &self,
        _share: GlfwWindowPtr,
    ) -> Result<GlfwWindowPtr, String> {
        Err(
            "GLFW offscreen window creation is only implemented on Windows in this crate"
                .to_string(),
        )
    }
}
