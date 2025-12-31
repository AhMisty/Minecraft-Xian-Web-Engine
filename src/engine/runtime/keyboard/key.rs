use super::char_fallback::glfw_key_to_char;
use super::named_key::glfw_key_to_named_key;

/// ### English
/// Maps GLFW key codes + optional Unicode codepoint into Servo `Key`.
/// Prefer the actual typed codepoint when provided (better for IME/text input).
///
/// ### 中文
/// 将 GLFW 键码 + 可选 Unicode 码点映射到 Servo `Key`。
/// 如果提供了实际输入的码点，则优先使用（更利于 IME/文本输入）。
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
