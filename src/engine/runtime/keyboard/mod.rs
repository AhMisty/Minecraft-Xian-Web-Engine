//! ### English
//! GLFW key-code translation into Servo keyboard semantics (`Code` / `Key`).
//!
//! ### 中文
//! GLFW 键码到 Servo 键盘语义（`Code` / `Key`）的转换。
mod char_fallback;
mod code;
mod key;
mod named_key;

pub(super) use code::glfw_key_to_code;
pub(super) use key::glfw_key_to_key;
