/// ### English
/// Vsync callback queue used to drive Servo's `RefreshDriver` from the Java side.
/// Hot path is a bounded ring buffer; cold overflow falls back to an intrusive list.
/// Single producer is the Servo thread; single consumer is the Java-side tick.
///
/// Vsync callback payload type used by Servo.
///
/// ### 中文
/// 由 Java 侧驱动 Servo `RefreshDriver` 的 vsync 回调队列。
/// 热路径使用有界 ring buffer；冷路径溢出时回退到侵入式链表。
/// 单生产者为 Servo 线程；单消费者为 Java 侧 tick。
type VsyncCallback = Box<dyn Fn() + Send + 'static>;

mod overflow;
mod queue;

pub use queue::VsyncCallbackQueue;
