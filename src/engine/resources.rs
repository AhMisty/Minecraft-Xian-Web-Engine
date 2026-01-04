//! ### English
//! Resource reader integration for Servo.
//!
//! Allows configuring a directory-based `ResourceReader` from the embedder side.
//!
//! ### 中文
//! Servo 的资源读取器集成。
//!
//! 允许宿主侧配置基于目录的 `ResourceReader`。
use std::path::PathBuf;

/// ### English
/// Directory-based `ResourceReader` for Servo.
///
/// ### 中文
/// 基于目录的 Servo `ResourceReader`。
pub struct DirResourceReader {
    /// ### English
    /// Root directory from which Servo resource files are read.
    ///
    /// ### 中文
    /// Servo 资源文件的读取根目录。
    root: PathBuf,
}

impl DirResourceReader {
    /// ### English
    /// Creates a reader rooted at `root`.
    ///
    /// ### 中文
    /// 创建一个以 `root` 为根目录的读取器。
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl servo::resources::ResourceReaderMethods for DirResourceReader {
    /// ### English
    /// Reads one Servo resource file relative to the configured root directory.
    ///
    /// #### Parameters
    /// - `file`: Resource identifier (provides the relative filename).
    ///
    /// Returns an empty buffer on I/O errors (Servo treats missing resources as empty).
    ///
    /// ### 中文
    /// 从配置的根目录读取一个 Servo 资源文件。
    ///
    /// #### 参数
    /// - `file`：资源标识（提供相对文件名）。
    ///
    /// 发生 I/O 错误时返回空缓冲区（Servo 会把缺失资源视为空）。
    fn read(&self, file: servo::resources::Resource) -> Vec<u8> {
        let mut path = self.root.clone();
        path.push(file.filename());
        std::fs::read(path).unwrap_or_default()
    }

    /// ### English
    /// Returns the explicit file allowlist for sandboxing (empty in this embedder).
    ///
    /// ### 中文
    /// 返回 sandbox 的文件白名单（本嵌入实现为空）。
    fn sandbox_access_files(&self) -> Vec<PathBuf> {
        vec![]
    }

    /// ### English
    /// Returns the directory allowlist for sandboxing (the configured root directory).
    ///
    /// ### 中文
    /// 返回 sandbox 的目录白名单（即配置的根目录）。
    fn sandbox_access_files_dirs(&self) -> Vec<PathBuf> {
        vec![self.root.clone()]
    }
}

/// ### English
/// Installs a directory-based resource reader for Servo.
///
/// #### Parameters
/// - `resources_dir`: Root directory used to resolve Servo resource files.
///
/// ### 中文
/// 为 Servo 安装基于目录的资源读取器。
///
/// #### 参数
/// - `resources_dir`：Servo 资源文件的根目录。
pub fn set_resources_dir(resources_dir: PathBuf) {
    servo::resources::set(Box::new(DirResourceReader::new(resources_dir)));
}
