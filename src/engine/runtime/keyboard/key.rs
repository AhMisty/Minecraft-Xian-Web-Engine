//! ### English
//! GLFW key-code + codepoint to Servo `Key` mapping.
//!
//! ### 中文
//! GLFW 键码 + 码点到 Servo `Key` 的映射。

use super::char_fallback::glfw_key_to_char;
use super::named_key::glfw_key_to_named_key;

/// ### English
/// Maps GLFW key codes + optional Unicode codepoint into Servo `Key`.
/// Prefer the actual typed codepoint when provided (better for IME/text input).
///
/// #### Parameters
/// - `glfw_key`: GLFW key code.
/// - `key_codepoint`: Unicode scalar value (0 means "not provided").
/// - `modifiers`: Modifier bitset used by the ASCII fallback mapping.
///
/// ### 中文
/// 将 GLFW 键码 + 可选 Unicode 码点映射到 Servo `Key`。
/// 如果提供了实际输入的码点，则优先使用（更利于 IME/文本输入）。
///
/// #### 参数
/// - `glfw_key`：GLFW 键码。
/// - `key_codepoint`：Unicode 标量值（`0` 表示“未提供”）。
/// - `modifiers`：修饰键集合（用于 ASCII 回退映射）。
pub(in super::super) fn glfw_key_to_key(
    glfw_key: u32,
    key_codepoint: u32,
    modifiers: servo::Modifiers,
) -> servo::Key {
    if key_codepoint != 0
        && let Some(ch) = char::from_u32(key_codepoint)
        && !ch.is_control()
    {
        return servo::Key::Character(ch.to_string());
    }

    if let Some(named) = glfw_key_to_named_key(glfw_key) {
        return servo::Key::Named(named);
    }

    if let Some(ch) = glfw_key_to_char(glfw_key, modifiers) {
        return servo::Key::Character(ch.to_string());
    }

    servo::Key::default()
}
