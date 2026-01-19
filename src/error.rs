//! ### English
//! Internal error types.
//!
//! This crate primarily exposes a C ABI. At the ABI boundary, failures are mapped to
//! `NULL` / `false` / `0`. Internally we keep small, allocation-free error values so control flow
//! stays explicit without adding overhead.
//!
//! ### 中文
//! 内部错误类型。
//!
//! 本库主要对外暴露 C ABI；在 ABI 边界通常把失败映射为 `NULL` / `false` / `0`。
//! 内部保留“体积小、零分配”的错误值，用于清晰表达控制流，同时不引入额外开销。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ### English
/// Initialization error.
///
/// #### Notes
/// - Allocation-free (`Copy`).
/// - Intended for internal control flow and optional logging.
///
/// ### 中文
/// 初始化错误。
///
/// #### 说明
/// - 零分配（`Copy`）。
/// - 用于内部控制流；如有需要可记录日志后在 ABI 边界映射为 `false`/`NULL`。
pub(crate) enum InitError {
    /// ### English
    /// Servo has already been initialized in this process (re-initialization is not supported).
    ///
    /// ### 中文
    /// Servo 已在本进程中初始化（不支持重复初始化）。
    ServoAlreadyInitialized,

    /// ### English
    /// The engine has not been initialized on this thread.
    ///
    /// #### Notes
    /// - The embedder must call the explicit init entry point before creating views.
    ///
    /// ### 中文
    /// 该线程尚未初始化引擎。
    ///
    /// #### 说明
    /// - 宿主必须先调用显式初始化入口，然后才能创建 view。
    EngineNotInitialized,

    /// ### English
    /// Unsupported OpenGL API selector from the C ABI.
    ///
    /// #### Fields
    /// - `value`: Raw selector value.
    ///
    /// ### 中文
    /// 不支持的 OpenGL API 选择值（来自 C ABI）。
    ///
    /// #### 字段
    /// - `value`：ABI 传入的原始选择值。
    UnsupportedGlApi { value: u32 },

    /// ### English
    /// A required embedder-provided pointer is NULL.
    ///
    /// #### Fields
    /// - `name`: Symbol name for diagnostics.
    ///
    /// ### 中文
    /// 宿主提供的必需指针为 NULL。
    ///
    /// #### 字段
    /// - `name`：符号名（用于诊断定位）。
    NullPointer { name: &'static str },

    /// ### English
    /// A required OpenGL entry point cannot be loaded.
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
