use std::rc::Rc;
use std::sync::Arc;

use crate::engine::refresh::RefreshScheduler;
use crate::engine::rendering::{
    GlfwSharedContext, GlfwTripleBufferContextInit, GlfwTripleBufferRenderingContext,
};
use crate::engine::vsync::VsyncCallbackQueue;

use super::super::command::Command;
use super::super::queue::CommandQueue;
use super::view::{Delegate, ViewEntry};

/// ### English
/// Drains control commands (create/destroy/shutdown) from embedder threads.
///
/// Returns `true` if a `Shutdown` command was received.
///
/// ### 中文
/// drain 来自宿主线程的控制命令（create/destroy/shutdown）。
///
/// 若收到 `Shutdown` 命令则返回 `true`。
#[allow(clippy::too_many_arguments)]
pub(super) fn drain_commands(
    servo: &servo::Servo,
    shared_ctx: &Rc<GlfwSharedContext>,
    vsync_queue: &Arc<VsyncCallbackQueue>,
    command_queue: &CommandQueue,
    refresh_scheduler: &mut Option<Arc<RefreshScheduler>>,
    views: &mut Vec<Option<ViewEntry>>,
    free_view_ids: &mut Vec<u32>,
    next_view_id: &mut u32,
    next_view_token: &mut u64,
) -> bool {
    while let Some(command) = command_queue.pop() {
        match command {
            Command::CreateView {
                initial_size,
                shared,
                mouse_move,
                resize,
                input_queue,
                load_url,
                pending,
                target_fps,
                unsafe_no_consumer_fence,
                unsafe_no_producer_fence,
                response,
            } => {
                let refresh_scheduler_for_view = if target_fps == 0 {
                    None
                } else {
                    Some(
                        refresh_scheduler
                            .get_or_insert_with(RefreshScheduler::new)
                            .clone(),
                    )
                };

                let rendering_context =
                    match GlfwTripleBufferRenderingContext::new(GlfwTripleBufferContextInit {
                        shared_ctx: shared_ctx.clone(),
                        initial_size,
                        shared,
                        vsync_queue: vsync_queue.clone(),
                        target_fps,
                        unsafe_no_consumer_fence,
                        unsafe_no_producer_fence,
                        refresh_scheduler: refresh_scheduler_for_view,
                    }) {
                        Ok(ctx) => Rc::new(ctx),
                        Err(err) => {
                            let _ = response.send(Err(err));
                            continue;
                        }
                    };

                let delegate = Rc::new(Delegate::new(rendering_context.clone()));

                let servo_webview = servo::WebViewBuilder::new(servo, rendering_context.clone())
                    .delegate(delegate)
                    .build();
                servo_webview.show();

                let id = free_view_ids.pop().unwrap_or_else(|| {
                    let id = *next_view_id;
                    *next_view_id = (*next_view_id).checked_add(1).expect("view id exhausted");
                    id
                });
                let token = {
                    let token = *next_view_token;
                    *next_view_token = (*next_view_token)
                        .checked_add(1)
                        .expect("view token exhausted");
                    token
                };

                let index = id as usize;
                if index >= views.len() {
                    views.resize_with(index + 1, || None);
                }
                views[index] = Some(ViewEntry::new(
                    token,
                    servo_webview,
                    rendering_context,
                    mouse_move,
                    input_queue,
                    resize,
                    load_url,
                    pending,
                    initial_size,
                ));

                let _ = response.send(Ok((id, token)));
            }
            Command::DestroyView { id, token } => {
                let index = id as usize;
                if let Some(slot) = views.get_mut(index)
                    && slot.as_ref().is_some_and(|entry| entry.token == token)
                {
                    *slot = None;
                    free_view_ids.push(id);
                    while views.last().is_some_and(|slot| slot.is_none()) {
                        views.pop();
                    }
                }
            }
            Command::Shutdown => return true,
        }
    }

    false
}
