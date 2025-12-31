use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use dpi::PhysicalSize;

use crate::engine::flags;
use crate::engine::frame::SharedFrameState;
use crate::engine::input::{CoalescedMouseMove, CoalescedResize, InputEventQueue};
use crate::engine::lockfree::OneShot;
use crate::engine::vsync::VsyncCallbackQueue;

use super::coalesced::{CoalescedLoadUrl, PendingWork};
use super::command::Command;
use super::pending::PendingIdQueue;
use super::queue::CommandQueue;
use super::servo_thread;
use super::view_handle::{WebEngineViewHandle, WebEngineViewHandleInit};

/// ### English
/// Engine runtime that owns the dedicated Servo thread.
///
/// ### 中文
/// 持有独立 Servo 线程的运行时。
pub struct EngineRuntime {
    default_size: PhysicalSize<u32>,
    command_queue: Arc<CommandQueue>,
    thread: Option<thread::JoinHandle<()>>,
    thread_handle: thread::Thread,
    vsync_queue: Arc<VsyncCallbackQueue>,
    pending_queue: Arc<PendingIdQueue>,
}

impl EngineRuntime {
    pub fn new(
        glfw_shared_window: *mut c_void,
        default_size: PhysicalSize<u32>,
        resources_dir: Option<PathBuf>,
        config_dir: Option<PathBuf>,
    ) -> Result<Self, String> {
        let glfw_shared_window_handle = glfw_shared_window as usize;

        let vsync_queue = Arc::new(VsyncCallbackQueue::with_capacity(4096));
        let vsync_queue_for_thread = vsync_queue.clone();

        let pending_queue = Arc::new(PendingIdQueue::with_capacity(64 * 1024));
        let pending_queue_for_thread = pending_queue.clone();

        let command_queue = Arc::new(CommandQueue::new());
        let command_queue_for_thread = command_queue.clone();

        let init = Arc::new(OneShot::new(thread::current()));
        let init_for_thread = init.clone();

        let thread = thread::spawn(move || {
            servo_thread::run_servo_thread(
                glfw_shared_window_handle,
                resources_dir,
                config_dir,
                vsync_queue_for_thread,
                pending_queue_for_thread,
                command_queue_for_thread,
                init_for_thread,
            );
        });

        let thread_handle = thread.thread().clone();

        match init.recv_timeout(Duration::from_secs(30)) {
            Some(Ok(())) => Ok(Self {
                default_size,
                command_queue,
                thread: Some(thread),
                thread_handle,
                vsync_queue,
                pending_queue,
            }),
            Some(Err(err)) => {
                thread_handle.unpark();
                let _ = thread.join();
                Err(err)
            }
            None => {
                command_queue.push(Command::Shutdown);
                thread_handle.unpark();
                let _ = thread.join();
                Err("Timed out initializing Servo thread".to_string())
            }
        }
    }

    pub fn create_view(
        &self,
        initial_size: PhysicalSize<u32>,
        target_fps: u32,
        view_flags: u32,
    ) -> Result<WebEngineViewHandle, String> {
        let unsafe_no_consumer_fence =
            (view_flags & flags::XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE) != 0;
        let unsafe_no_producer_fence =
            (view_flags & flags::XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_PRODUCER_FENCE) != 0;
        let input_single_producer =
            (view_flags & flags::XIAN_WEB_ENGINE_VIEW_FLAG_INPUT_SINGLE_PRODUCER) != 0;

        let initial_size = if initial_size.width == 0 || initial_size.height == 0 {
            self.default_size
        } else {
            initial_size
        };
        let initial_size = PhysicalSize::new(initial_size.width.max(1), initial_size.height.max(1));

        let shared = Arc::new(SharedFrameState::new(initial_size));
        let mouse_move = Arc::new(CoalescedMouseMove::default());
        let resize = Arc::new(CoalescedResize::default());
        let input_queue = Arc::new(InputEventQueue::new(input_single_producer));
        let load_url = Arc::new(CoalescedLoadUrl::default());
        let pending = Arc::new(PendingWork::default());

        let response = Arc::new(OneShot::new(thread::current()));

        self.command_queue.push(Command::CreateView {
            initial_size,
            shared: shared.clone(),
            mouse_move: mouse_move.clone(),
            resize: resize.clone(),
            input_queue: input_queue.clone(),
            load_url: load_url.clone(),
            pending: pending.clone(),
            target_fps,
            unsafe_no_consumer_fence,
            unsafe_no_producer_fence,
            response: response.clone(),
        });
        self.thread_handle.unpark();

        match response.recv_timeout(Duration::from_secs(30)) {
            Some(Ok((id, token))) => Ok(WebEngineViewHandle::new(WebEngineViewHandleInit {
                id,
                token,
                shared,
                mouse_move,
                resize,
                input_queue,
                load_url,
                pending,
                pending_queue: self.pending_queue.clone(),
                command_queue: self.command_queue.clone(),
                thread_handle: self.thread_handle.clone(),
                unsafe_no_consumer_fence,
            })),
            Some(Err(err)) => Err(err),
            None => Err("Timed out creating view".to_string()),
        }
    }

    /// ### English
    /// Drains pending vsync callbacks (used by the Java side to drive Servo refresh).
    ///
    /// ### 中文
    /// drain 已入队的 vsync 回调（供 Java 侧驱动 Servo refresh）。
    pub fn tick(&self) {
        self.vsync_queue.tick();
    }

    /// ### English
    /// Requests Servo thread shutdown and joins it.
    ///
    /// ### 中文
    /// 请求 Servo 线程退出并 join。
    pub fn shutdown(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.command_queue.push(Command::Shutdown);
            self.thread_handle.unpark();
            let _ = thread.join();
            self.command_queue.close();
        }
    }
}

impl Drop for EngineRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}
