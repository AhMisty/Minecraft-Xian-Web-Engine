//! ### English
//! Translation from ABI input events to Servo input events.
//!
//! ### 中文
//! ABI 输入事件到 Servo 输入事件的转换与派发。

use crate::engine::input_types::{
    XIAN_WEB_ENGINE_INPUT_KIND_KEY, XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON,
    XIAN_WEB_ENGINE_INPUT_KIND_WHEEL, XianWebEngineInputEvent,
};

use super::keyboard::{glfw_key_to_code, glfw_key_to_key};

/// ### English
/// Dispatches one queued input event into Servo's `WebView`.
/// Called on the Servo thread only (single consumer).
///
/// ### 中文
/// 将一个入队的输入事件派发给 Servo 的 `WebView`。
/// 仅在 Servo 线程调用（单消费者）。
pub(super) fn dispatch_queued_input_event(
    servo_webview: &servo::WebView,
    raw: XianWebEngineInputEvent,
) {
    match raw.kind {
        XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON => {
            let action = match raw.mouse_action {
                0 => servo::MouseButtonAction::Down,
                _ => servo::MouseButtonAction::Up,
            };
            let button = servo::MouseButton::from(raw.mouse_button as u64);
            let point = servo::WebViewPoint::from(servo::DevicePoint::new(raw.x, raw.y));
            servo_webview.notify_input_event(servo::InputEvent::MouseButton(
                servo::MouseButtonEvent::new(action, button, point),
            ));
        }
        XIAN_WEB_ENGINE_INPUT_KIND_WHEEL => {
            let mode = match raw.wheel_mode {
                1 => servo::WheelMode::DeltaLine,
                2 => servo::WheelMode::DeltaPage,
                _ => servo::WheelMode::DeltaPixel,
            };
            let delta = servo::WheelDelta {
                x: raw.wheel_delta_x,
                y: raw.wheel_delta_y,
                z: raw.wheel_delta_z,
                mode,
            };
            let point = servo::WebViewPoint::from(servo::DevicePoint::new(raw.x, raw.y));
            servo_webview.notify_input_event(servo::InputEvent::Wheel(servo::WheelEvent::new(
                delta, point,
            )));
        }
        XIAN_WEB_ENGINE_INPUT_KIND_KEY => {
            let state = match raw.key_state {
                0 => servo::KeyState::Down,
                _ => servo::KeyState::Up,
            };
            let location = match raw.key_location {
                1 => servo::Location::Left,
                2 => servo::Location::Right,
                3 => servo::Location::Numpad,
                _ => servo::Location::Standard,
            };
            let modifiers = servo::Modifiers::from_bits_truncate(raw.modifiers);
            let repeat = raw.repeat != 0;
            let is_composing = raw.is_composing != 0;

            let key = glfw_key_to_key(raw.glfw_key, raw.key_codepoint, modifiers);
            let code = glfw_key_to_code(raw.glfw_key);

            let keyboard = servo::KeyboardEvent::new_without_event(
                state,
                key,
                code,
                location,
                modifiers,
                repeat,
                is_composing,
            );
            servo_webview.notify_input_event(servo::InputEvent::Keyboard(keyboard));
        }
        _ => {}
    }
}
