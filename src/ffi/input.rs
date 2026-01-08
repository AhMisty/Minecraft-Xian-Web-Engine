//! ### English
//! C ABI bindings for sending input events to a view.
//!
//! ### 中文
//! 向 view 发送输入事件的 C ABI 绑定。
use std::ptr;

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
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create` (must not be NULL).
/// - `events`: Pointer to an array of `count` input events (must not be NULL).
/// - `count`: Number of events in the `events` array.
///
/// #### Safety
/// - `view` must be a valid pointer returned by `xian_web_engine_view_create`.
/// - If `count != 0`, `events` must be valid for reads of `count` `XianWebEngineInputEvent` values.
/// - `events` may be unaligned; this function handles unaligned loads.
///
/// ### 中文
/// 向 view 发送一批输入事件。
///
/// 返回实际接收的事件数量（若队列满，可能小于 `count`）。
/// 若 view 处于 inactive，则会把事件视为“已接收”并直接丢弃（快路径）。
/// 未知事件类型会视为“已接收”并直接丢弃。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针（必须非 NULL）。
/// - `events`：指向长度为 `count` 的输入事件数组（必须非 NULL）。
/// - `count`：`events` 数组长度。
///
/// #### 安全
/// - `view` 必须是由 `xian_web_engine_view_create` 返回的有效指针。
/// - 若 `count != 0`，则 `events` 必须至少可读 `count` 个 `XianWebEngineInputEvent` 元素。
/// - `events` 允许非对齐；本函数会使用非对齐读取进行兼容处理。
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
    let mut has_mouse_move = false;
    let mut last_mouse_move_x: f32 = 0.0;
    let mut last_mouse_move_y: f32 = 0.0;
    let mut input_pending = false;

    let count = count as usize;
    let mut index: usize = 0;
    if super::is_aligned_ptr(events) {
        let event_slice = unsafe { std::slice::from_raw_parts(events, count) };
        while index < count {
            let event = &event_slice[index];
            match event.kind {
                XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE => {
                    has_mouse_move = true;
                    last_mouse_move_x = event.x;
                    last_mouse_move_y = event.y;
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
                    let pushed = handle.push_input_events(segment);
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
    } else {
        while index < count {
            let event = unsafe { ptr::read_unaligned(events.add(index)) };
            match event.kind {
                XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE => {
                    has_mouse_move = true;
                    last_mouse_move_x = event.x;
                    last_mouse_move_y = event.y;
                    accepted += 1;
                    index += 1;
                }
                XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON
                | XIAN_WEB_ENGINE_INPUT_KIND_WHEEL
                | XIAN_WEB_ENGINE_INPUT_KIND_KEY => {
                    let pushed = handle.push_input_events(std::slice::from_ref(&event));
                    accepted += pushed as u32;
                    if pushed > 0 {
                        input_pending = true;
                        index += 1;
                    } else {
                        break;
                    }
                }
                _ => {
                    accepted += 1;
                    index += 1;
                }
            }
        }
    }

    if has_mouse_move {
        wake_needed |= handle.queue_mouse_move(last_mouse_move_x, last_mouse_move_y);
    }

    if input_pending && handle.notify_input_pending() {
        wake_needed = true;
    }

    if wake_needed {
        handle.wake();
    }

    accepted
}
