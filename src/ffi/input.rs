use crate::engine::{
    XIAN_WEB_ENGINE_INPUT_KIND_KEY, XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON,
    XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE, XIAN_WEB_ENGINE_INPUT_KIND_WHEEL,
    XianWebEngineInputEvent,
};

use super::XianWebEngineView;

#[unsafe(no_mangle)]
/// ### English
/// Sends a batch of input events to a view.
///
/// Returns the number of accepted events (may be less than `count` if the queue is full).
/// If the view is inactive, events are treated as accepted and dropped (fast path).
/// Unknown event kinds are treated as accepted and dropped.
///
/// ### 中文
/// 向 view 发送一批输入事件。
///
/// 返回实际接收的事件数量（若队列满，可能小于 `count`）。
/// 若 view 处于 inactive，则会把事件视为“已接收”并直接丢弃（快路径）。
/// 未知事件类型会视为“已接收”并直接丢弃。
pub unsafe extern "C" fn xian_web_engine_view_send_input_events(
    view: *mut XianWebEngineView,
    events: *const XianWebEngineInputEvent,
    count: u32,
) -> u32 {
    if view.is_null() || events.is_null() || count == 0 {
        return 0;
    }

    let handle = unsafe { &(*view).handle };

    if !handle.is_active() {
        return count;
    }

    let mut accepted: u32 = 0;
    let mut wake_needed = false;
    let mut last_mouse_move: Option<(f32, f32)> = None;
    let mut input_pending = false;

    let count = count as usize;
    let event_slice = unsafe { std::slice::from_raw_parts(events, count) };
    let mut index: usize = 0;
    while index < count {
        let ev = event_slice[index];
        match ev.kind {
            XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE => {
                last_mouse_move = Some((ev.x, ev.y));
                accepted += 1;
                index += 1;
            }
            XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON
            | XIAN_WEB_ENGINE_INPUT_KIND_WHEEL
            | XIAN_WEB_ENGINE_INPUT_KIND_KEY => {
                let start = index;
                index += 1;
                while index < count {
                    let kind = event_slice[index].kind;
                    if kind == XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON
                        || kind == XIAN_WEB_ENGINE_INPUT_KIND_WHEEL
                        || kind == XIAN_WEB_ENGINE_INPUT_KIND_KEY
                    {
                        index += 1;
                    } else {
                        break;
                    }
                }

                let segment = &event_slice[start..index];
                let pushed = handle.try_enqueue_input_events(segment);
                accepted += pushed as u32;
                if pushed > 0 {
                    input_pending = true;
                }
                if pushed < segment.len() {
                    break;
                }
            }
            _ => {
                accepted += 1;
                index += 1;
            }
        }
    }

    if let Some((x, y)) = last_mouse_move {
        wake_needed |= handle.queue_mouse_move(x, y);
    }

    if input_pending && handle.notify_input_pending() {
        wake_needed = true;
    }

    if wake_needed {
        handle.wake();
    }

    accepted
}
