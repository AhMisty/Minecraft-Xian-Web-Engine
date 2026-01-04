//! ### English
//! C ABI bindings for installing the embedder-provided GLFW API table.
//!
//! ### 中文
//! 安装宿主提供的 GLFW API 函数表的 C ABI 绑定。

use crate::engine::{EmbedderGlfwApi, install_embedder_glfw_api};

#[unsafe(no_mangle)]
/// ### English
/// Installs an embedder-provided GLFW function table.
///
/// This must be called before `xian_web_engine_create`.
/// The engine will not attempt to locate `glfw3.dll/glfw.dll` by name.
///
/// All function pointers must come from the same GLFW library instance that produced the
/// `GLFWwindow*` passed to `xian_web_engine_create`.
///
/// Returns `true` on success.
///
/// ### 中文
/// 安装由宿主提供的 GLFW 函数表。
///
/// 必须在 `xian_web_engine_create` 之前调用；引擎不会再按名称查找/加载 `glfw3.dll/glfw.dll`。
///
/// 所有函数指针必须来自同一个 GLFW 库实例（也就是创建 `xian_web_engine_create` 传入的
/// `GLFWwindow*` 的那个实例）。
///
/// 成功返回 `true`。
pub unsafe extern "C" fn xian_web_engine_set_glfw_api(api: *const EmbedderGlfwApi) -> bool {
    if api.is_null() {
        return false;
    }

    let api = unsafe { std::ptr::read_unaligned(api) };
    install_embedder_glfw_api(api).is_ok()
}
