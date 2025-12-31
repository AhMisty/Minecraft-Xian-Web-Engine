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
    /// ### 中文
    /// 创建由 vsync 驱动的 refresh driver。
    pub fn new(queue: Arc<VsyncCallbackQueue>) -> Rc<Self> {
        Rc::new(Self { queue })
    }
}

impl RefreshDriver for VsyncRefreshDriver {
    fn observe_next_frame(&self, start_frame_callback: Box<dyn Fn() + Send + 'static>) {
        self.queue.enqueue(start_frame_callback);
    }
}
