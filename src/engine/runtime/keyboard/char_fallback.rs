//! ### English
//! Fallback mapping from GLFW key codes to ASCII characters.
//!
//! ### 中文
//! GLFW key code 到 ASCII 字符的回退映射。

/// ### English
/// ASCII fallback for characters when codepoint is not provided.
/// This assumes a US-like layout and applies SHIFT for common punctuation.
///
/// #### Parameters
/// - `glfw_key`: GLFW key code.
/// - `modifiers`: Modifier bitset (SHIFT affects punctuation/letters).
///
/// ### 中文
/// 当未提供码点时的 ASCII 回退方案。
/// 假设接近 US 键盘布局，并对常见符号应用 SHIFT。
///
/// #### 参数
/// - `glfw_key`：GLFW 键码。
/// - `modifiers`：修饰键集合（SHIFT 会影响符号/字母）。
pub(super) fn glfw_key_to_char(glfw_key: u32, modifiers: servo::Modifiers) -> Option<char> {
    let shift = modifiers.contains(servo::Modifiers::SHIFT);

    let ch = match glfw_key {
        32 => ' ',
        39 => {
            if shift {
                '"'
            } else {
                '\''
            }
        }
        44 => {
            if shift {
                '<'
            } else {
                ','
            }
        }
        45 => {
            if shift {
                '_'
            } else {
                '-'
            }
        }
        46 => {
            if shift {
                '>'
            } else {
                '.'
            }
        }
        47 => {
            if shift {
                '?'
            } else {
                '/'
            }
        }
        48 => {
            if shift {
                ')'
            } else {
                '0'
            }
        }
        49 => {
            if shift {
                '!'
            } else {
                '1'
            }
        }
        50 => {
            if shift {
                '@'
            } else {
                '2'
            }
        }
        51 => {
            if shift {
                '#'
            } else {
                '3'
            }
        }
        52 => {
            if shift {
                '$'
            } else {
                '4'
            }
        }
        53 => {
            if shift {
                '%'
            } else {
                '5'
            }
        }
        54 => {
            if shift {
                '^'
            } else {
                '6'
            }
        }
        55 => {
            if shift {
                '&'
            } else {
                '7'
            }
        }
        56 => {
            if shift {
                '*'
            } else {
                '8'
            }
        }
        57 => {
            if shift {
                '('
            } else {
                '9'
            }
        }
        59 => {
            if shift {
                ':'
            } else {
                ';'
            }
        }
        61 => {
            if shift {
                '+'
            } else {
                '='
            }
        }
        65..=90 => {
            let base = glfw_key as u8;
            let ascii = if shift { base } else { base + 32 };
            return Some(ascii as char);
        }
        91 => {
            if shift {
                '{'
            } else {
                '['
            }
        }
        92 => {
            if shift {
                '|'
            } else {
                '\\'
            }
        }
        93 => {
            if shift {
                '}'
            } else {
                ']'
            }
        }
        96 => {
            if shift {
                '~'
            } else {
                '`'
            }
        }
        _ => return None,
    };

    Some(ch)
}
