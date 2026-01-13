//! ### English
//! Servo resource reader integration.
//!
//! This embedder installs a directory-based resource reader when the embedder calls
//! `xian_web_engine_set_resources_dir` (C ABI).
//!
//! ### 中文
//! Servo 资源读取器集成。
//!
//! 当宿主调用 `xian_web_engine_set_resources_dir`（C ABI）时，本嵌入层会安装一个基于目录的资源读取器。

use std::path::PathBuf;

/// ### English
/// Directory-based `ResourceReader` for Servo.
///
/// ### 中文
/// 基于目录的 Servo `ResourceReader`。
pub struct DirectoryResourceReader {
    /// ### English
    /// Root directory that contains Servo resources.
    ///
    /// ### 中文
    /// Servo 资源根目录。
    root: PathBuf,
}

impl DirectoryResourceReader {
    /// ### English
    /// Creates a directory-based resource reader.
    ///
    /// #### Parameters
    /// - `root`: Root directory path.
    ///
    /// #### Returns
    /// - A new `DirectoryResourceReader`.
    ///
    /// ### 中文
    /// 创建基于目录的资源读取器。
    ///
    /// #### 参数
    /// - `root`：根目录路径。
    ///
    /// #### 返回
    /// - 新的 `DirectoryResourceReader`。
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl servo::resources::ResourceReaderMethods for DirectoryResourceReader {
    /// ### English
    /// Reads a resource file into memory.
    ///
    /// #### Parameters
    /// - `file`: Servo resource descriptor.
    ///
    /// #### Returns
    /// - File bytes, or empty when missing/unreadable.
    ///
    /// ### 中文
    /// 将资源文件读入内存。
    ///
    /// #### 参数
    /// - `file`：Servo 资源描述符。
    ///
    /// #### 返回
    /// - 文件字节；若缺失/不可读则返回空。
    fn read(&self, file: servo::resources::Resource) -> Vec<u8> {
        let path = self.root.join(file.filename());
        std::fs::read(path).unwrap_or_default()
    }

    /// ### English
    /// Lists specific files that must be accessible inside the Servo sandbox.
    ///
    /// #### Returns
    /// - File path list (empty in this embedder).
    ///
    /// ### 中文
    /// 列出 Servo 沙箱内需要访问的具体文件。
    ///
    /// #### 返回
    /// - 文件路径列表（本实现始终为空）。
    fn sandbox_access_files(&self) -> Vec<PathBuf> {
        vec![]
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

/// ### English
/// Installs a directory-based resource reader for Servo.
///
/// #### Parameters
/// - `resources_dir`: Root directory that contains Servo resources.
///
/// ### 中文
/// 为 Servo 安装基于目录的资源读取器。
///
/// #### 参数
/// - `resources_dir`：包含 Servo 资源的根目录。
pub fn set_resources_dir(resources_dir: PathBuf) {
    servo::resources::set(Box::new(DirectoryResourceReader::new(resources_dir)));
}
