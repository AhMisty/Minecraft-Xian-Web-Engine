//! ### English
//! Servo resource reader integration.
//!
//! This module installs a directory-backed `ResourceReader` when the embedder calls the C ABI
//! function `xian_web_engine_set_resources_dir`.
//!
//! ### 中文
//! Servo 资源读取器集成。
//!
//! 当宿主通过 C ABI 调用 `xian_web_engine_set_resources_dir` 时，本模块会安装一个基于目录的
//! `ResourceReader` 供 Servo 读取内置资源。

use std::path::PathBuf;

/// ### English
/// Installs a directory-backed resource reader for Servo (process-global).
///
/// #### Parameters
/// - `root`: Root directory that contains Servo resources.
///
/// ### 中文
/// 为 Servo 安装一个基于目录的资源读取器（进程全局）。
///
/// #### 参数
/// - `root`：包含 Servo 资源的根目录。
pub(crate) fn set_resources_dir(root: PathBuf) {
    servo::resources::set(Box::new(DirResourceReader { root }));
}

/// ### English
/// Directory-backed `ResourceReader` implementation.
///
/// ### 中文
/// 基于目录的 `ResourceReader` 实现。
struct DirResourceReader {
    /// ### English
    /// Root directory that contains Servo resources.
    ///
    /// ### 中文
    /// Servo 资源根目录。
    root: PathBuf,
}

impl servo::resources::ResourceReaderMethods for DirResourceReader {
    /// ### English
    /// Reads a resource file into memory.
    ///
    /// #### Parameters
    /// - `file`: Servo resource descriptor.
    ///
    /// #### Returns
    /// - File bytes; returns an empty buffer when missing/unreadable.
    ///
    /// ### 中文
    /// 将资源文件读入内存。
    ///
    /// #### 参数
    /// - `file`：Servo 资源描述符。
    ///
    /// #### 返回
    /// - 文件字节；若缺失或不可读则返回空缓冲。
    fn read(&self, file: servo::resources::Resource) -> Vec<u8> {
        let path = self.root.join(file.filename());
        std::fs::read(path).unwrap_or_default()
    }

    /// ### English
    /// Lists specific files that must be accessible inside the Servo sandbox.
    ///
    /// #### Returns
    /// - File path list (empty in this implementation).
    ///
    /// ### 中文
    /// 列出 Servo 沙箱内需要访问的具体文件。
    ///
    /// #### 返回
    /// - 文件路径列表（本实现始终为空）。
    fn sandbox_access_files(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// ### English
    /// Lists directories that must be accessible inside the Servo sandbox.
    ///
    /// #### Returns
    /// - Directory path list (contains the resource root directory).
    ///
    /// ### 中文
    /// 列出 Servo 沙箱内需要访问的目录。
    ///
    /// #### 返回
    /// - 目录路径列表（包含资源根目录）。
    fn sandbox_access_files_dirs(&self) -> Vec<PathBuf> {
        vec![self.root.clone()]
    }
}
