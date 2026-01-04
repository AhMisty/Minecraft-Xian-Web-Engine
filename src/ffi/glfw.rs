use crate::engine::{EmbedderGlfwApi, install_embedder_glfw_api};

#[unsafe(no_mangle)]
/// ### English
/// Installs an embedder-provided GLFW function table.
///
/// If this is installed, the engine will prefer it over trying to locate `glfw3.dll/glfw.dll` by
/// name. This must be called before `xian_web_engine_create`.
///
/// All function pointers must come from the same GLFW library instance that produced the
/// `GLFWwindow*` passed to `xian_web_engine_create`.
///
/// Returns `true` on success.
///
/// ### 中文
/// 安装由宿主提供的 GLFW 函数表。
///
/// 安装后，引擎会优先使用该表，而不是通过固定名字去查找 `glfw3.dll/glfw.dll`。
/// 必须在 `xian_web_engine_create` 之前调用。
///
/// 所有函数指针必须来自同一个 GLFW 库实例（也就是创建 `xian_web_engine_create` 传入的
/// `GLFWwindow*` 的那个实例）。
///
/// 成功返回 `true`。
pub unsafe extern "C" fn xian_web_engine_set_glfw_api(api: *const EmbedderGlfwApi) -> bool {
    if api.is_null() {
        return false;
    }

    let api = unsafe { *api };
    install_embedder_glfw_api(api).is_ok()
}
