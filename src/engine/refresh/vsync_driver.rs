//! ### English
//! External-vsync driven refresh driver (Java-side tick).
//!
//! ### 中文
//! 外部 vsync 驱动的 refresh driver（Java 侧 tick）。

use std::rc::Rc;
use std::sync::Arc;

use servo::RefreshDriver;

use crate::engine::vsync::VsyncCallbackQueue;

/// ### English
/// Refresh driver driven by an external vsync tick (Java side).
///
/// ### 中文
/// 由外部 vsync tick（Java 侧）驱动的 refresh driver。
pub struct VsyncRefreshDriver {
    /// ### English
    /// Shared vsync callback queue drained by Java-side tick.
    ///
    /// ### 中文
    /// 由 Java 侧 tick drain 的共享 vsync 回调队列。
    queue: Arc<VsyncCallbackQueue>,
}

impl VsyncRefreshDriver {
    /// ### English
    /// Creates a vsync-driven refresh driver.
    ///
    /// #### Parameters
    /// - `queue`: Vsync callback queue drained by the embedder tick.
    ///
    /// ### 中文
    /// 创建由 vsync 驱动的 refresh driver。
    ///
    /// #### 参数
    /// - `queue`：由宿主 tick drain 的 vsync 回调队列。
    pub fn new(queue: Arc<VsyncCallbackQueue>) -> Rc<Self> {
        Rc::new(Self { queue })
    }
}

impl RefreshDriver for VsyncRefreshDriver {
    /// ### English
    /// Pushes the callback into the shared vsync callback queue (executed on Java tick).
    ///
    /// #### Parameters
    /// - `start_frame_callback`: Callback executed on the next embedder tick.
    ///
    /// ### 中文
    /// 将回调 push 到共享 vsync 回调队列（在 Java tick 中执行）。
    ///
    /// #### 参数
    /// - `start_frame_callback`：在下一次宿主 tick 执行的回调。
    fn observe_next_frame(&self, start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.queue.push(start_frame_callback);
    }
}
