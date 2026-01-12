//! ### English
//! Internal error types.
//!
//! This crate primarily exposes a C ABI and typically maps failures to NULL/false on the boundary.
//! Internally we still keep small, allocation-free error values to make control flow explicit while
//! staying performance-friendly.
//!
//! ### 中文
//! 内部错误类型。
//!
//! 本库主要对外暴露 C ABI，边界处通常将失败映射为 NULL/false。
//! 但在内部依然使用“零分配、体积小”的错误值，让控制流更清晰，同时保持性能友好。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ### English
/// Engine initialization error.
///
/// This type is:
/// - Allocation-free (`Copy`)
/// - FFI-friendly (can be ignored or logged by the caller)
///
/// ### 中文
/// 引擎初始化错误。
///
/// 该类型具备：
/// - 零分配（可 `Copy`）
/// - 适合 FFI（调用方可选择忽略或记录）
pub(crate) enum EngineInitError {
    /// ### English
    /// Servo has already been initialized in this process.
    ///
    /// ### 中文
    /// Servo 已在本进程中初始化过（不支持重复初始化）。
    ServoAlreadyInitialized,

    /// ### English
    /// Unsupported OpenGL API selector value from the C ABI.
    ///
    /// #### Fields
    /// - `value`: Raw `gl_api` value from the ABI.
    ///
    /// ### 中文
    /// 不支持的 OpenGL API 选择值（来自 C ABI）。
    ///
    /// #### 字段
    /// - `value`：ABI 传入的原始 `gl_api` 值。
    UnsupportedGlApi { value: u32 },

    /// ### English
    /// A required embedder-provided function pointer was NULL.
    ///
    /// #### Fields
    /// - `name`: Symbol name (for debugging).
    ///
    /// ### 中文
    /// 宿主提供的必需函数指针为 NULL。
    ///
    /// #### 字段
    /// - `name`：符号名（用于调试定位）。
    NullFunctionPointer { name: &'static str },

    /// ### English
    /// A required OpenGL entry point could not be loaded.
    ///
    /// #### Fields
    /// - `name`: Entry point name.
    ///
    /// ### 中文
    /// 必需的 OpenGL 入口无法加载。
    ///
    /// #### 字段
    /// - `name`：入口函数名。
    MissingOpenGlEntryPoint { name: &'static str },
}
