//! ### English
//! GLFW key-code to Servo `NamedKey` mapping.
//!
//! ### 中文
//! GLFW 键码到 Servo `NamedKey` 的映射。

/// ### English
/// Named key mapping (Enter/Escape/Arrows/Function keys...).
///
/// #### Parameters
/// - `glfw_key`: GLFW key code.
///
/// ### 中文
/// 命名按键映射（Enter/Escape/方向键/功能键等）。
///
/// #### 参数
/// - `glfw_key`：GLFW 键码。
pub(super) fn glfw_key_to_named_key(glfw_key: u32) -> Option<servo::NamedKey> {
    use servo::NamedKey;
    match glfw_key {
        256 => Some(NamedKey::Escape),
        257 => Some(NamedKey::Enter),
        258 => Some(NamedKey::Tab),
        259 => Some(NamedKey::Backspace),
        260 => Some(NamedKey::Insert),
        261 => Some(NamedKey::Delete),
        262 => Some(NamedKey::ArrowRight),
        263 => Some(NamedKey::ArrowLeft),
        264 => Some(NamedKey::ArrowDown),
        265 => Some(NamedKey::ArrowUp),
        266 => Some(NamedKey::PageUp),
        267 => Some(NamedKey::PageDown),
        268 => Some(NamedKey::Home),
        269 => Some(NamedKey::End),
        280 => Some(NamedKey::CapsLock),
        281 => Some(NamedKey::ScrollLock),
        282 => Some(NamedKey::NumLock),
        283 => Some(NamedKey::PrintScreen),
        284 => Some(NamedKey::Pause),
        290 => Some(NamedKey::F1),
        291 => Some(NamedKey::F2),
        292 => Some(NamedKey::F3),
        293 => Some(NamedKey::F4),
        294 => Some(NamedKey::F5),
        295 => Some(NamedKey::F6),
        296 => Some(NamedKey::F7),
        297 => Some(NamedKey::F8),
        298 => Some(NamedKey::F9),
        299 => Some(NamedKey::F10),
        300 => Some(NamedKey::F11),
        301 => Some(NamedKey::F12),
        340 | 344 => Some(NamedKey::Shift),
        341 | 345 => Some(NamedKey::Control),
        342 | 346 => Some(NamedKey::Alt),
        343 | 347 => Some(NamedKey::Meta),
        348 => Some(NamedKey::ContextMenu),
        _ => None,
    }
}
