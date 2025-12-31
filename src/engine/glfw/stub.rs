use std::ffi::{CStr, c_void};

pub type GlfwWindowPtr = *mut c_void;

/// ### English
/// Placeholder GLFW loader for non-Windows builds (this crate currently targets Windows).
///
/// ### 中文
/// 非 Windows 构建的占位 GLFW loader（本 crate 当前主要目标为 Windows）。
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
    /// ### 中文
    /// 非 Windows 构建下为 no-op。
    pub unsafe fn make_current(&self, _window: GlfwWindowPtr) {}

    /// ### English
    /// Always returns NULL on non-Windows builds.
    ///
    /// ### 中文
    /// 非 Windows 构建下总是返回 NULL。
    pub unsafe fn get_proc_address(&self, _name: &CStr) -> *const c_void {
        std::ptr::null()
    }

    /// ### English
    /// No-op on non-Windows builds.
    ///
    /// ### 中文
    /// 非 Windows 构建下为 no-op。
    pub unsafe fn destroy_window(&self, _window: GlfwWindowPtr) {}

    /// ### English
    /// Always returns an error on non-Windows builds.
    ///
    /// ### 中文
    /// 非 Windows 构建下总是返回错误。
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
