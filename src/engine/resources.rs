//! ### English
//! Resource reader integration for Servo.
//! Allows configuring a directory-based `ResourceReader` from the embedder side.
//!
//! ### 中文
//! Servo 的资源读取器集成。
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
    fn read(&self, file: servo::resources::Resource) -> Vec<u8> {
        let mut path = self.root.clone();
        path.push(file.filename());
        std::fs::read(path).unwrap_or_default()
    }

    fn sandbox_access_files(&self) -> Vec<PathBuf> {
        vec![]
    }

    fn sandbox_access_files_dirs(&self) -> Vec<PathBuf> {
        vec![self.root.clone()]
    }
}

/// ### English
/// Installs a directory-based resource reader for Servo.
///
/// ### 中文
/// 为 Servo 安装基于目录的资源读取器。
pub fn set_resources_dir(resources_dir: PathBuf) {
    servo::resources::set(Box::new(DirResourceReader::new(resources_dir)));
}
