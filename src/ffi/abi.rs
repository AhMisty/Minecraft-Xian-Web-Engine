#[unsafe(no_mangle)]
/// ### English
/// Returns the C ABI version.
///
/// ### 中文
/// 返回 C ABI 版本号。
pub extern "C" fn xian_web_engine_abi_version() -> u32 {
    super::XIAN_WEB_ENGINE_ABI_VERSION
}
