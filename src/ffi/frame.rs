use super::{XianWebEngineFrame, XianWebEngineView};

#[unsafe(no_mangle)]
/// ### English
/// Tries to acquire the latest READY frames for a batch of views.
///
/// This function is "compact": it only writes successfully acquired frames.
///
/// - `views` is an array of `count` view pointers.
/// - `out_view_indices` receives the corresponding input view index for each acquired frame.
/// - `out_frames` receives the acquired frames, packed densely from 0..return_value.
/// - Both output arrays must have capacity for at least `count` entries.
///
/// Returns the number of acquired frames written.
///
/// ### 中文
/// 批量尝试获取多个 view 的最新 READY 帧。
///
/// 该函数采用“紧凑输出”：仅把成功 acquire 的帧写入输出缓冲区。
///
/// - `views` 为长度 `count` 的 view 指针数组。
/// - `out_view_indices` 写入每个 acquired frame 对应的输入 view 下标。
/// - `out_frames` 写入 acquired frames（从 0..返回值 紧凑排列）。
/// - 两个输出数组都必须至少能容纳 `count` 个元素。
///
/// 返回写入的 acquired frame 数量。
pub unsafe extern "C" fn xian_web_engine_views_acquire_frames(
    views: *const *mut XianWebEngineView,
    out_view_indices: *mut u32,
    out_frames: *mut XianWebEngineFrame,
    count: u32,
) -> u32 {
    if views.is_null() || out_view_indices.is_null() || out_frames.is_null() || count == 0 {
        return 0;
    }

    let count = count as usize;
    let view_ptrs = unsafe { std::slice::from_raw_parts(views, count) };
    let indices_out = unsafe { std::slice::from_raw_parts_mut(out_view_indices, count) };
    let frames_out = unsafe { std::slice::from_raw_parts_mut(out_frames, count) };

    let mut acquired = 0usize;
    for (i, &view_ptr) in view_ptrs.iter().enumerate() {
        if view_ptr.is_null() {
            continue;
        }

        let view_handle = unsafe { &(*view_ptr).handle };
        if let Some(frame) = view_handle.acquire_frame() {
            indices_out[acquired] = i as u32;
            frames_out[acquired] = frame.into();
            acquired += 1;
        }
    }

    acquired as u32
}

#[unsafe(no_mangle)]
/// ### English
/// Releases a batch of previously acquired frame slots for multiple views.
///
/// If `consumer_fences` is NULL, all fences are treated as 0.
/// If non-NULL, each fence must be a `GLsync` created by the embedder *after* sampling the texture.
/// Ownership transfers to Rust and the embedder must NOT delete it; Rust will delete it after the
/// producer sees it signaled.
///
/// If a fence value is `0`, the slot becomes immediately reusable; the embedder must ensure the
/// texture is no longer in use by the GPU before releasing (e.g., via other synchronization).
///
/// If a view was created with `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE`, its corresponding
/// consumer fence MUST be 0 (ignored).
///
/// ### 中文
/// 批量释放多个 view 之前 acquire 的帧槽位。
///
/// 若 `consumer_fences` 为 NULL，则所有 fence 视为 0。
/// 若非 NULL，则每个 fence 必须是宿主在采样纹理完成后创建的 `GLsync`。
/// 所有权会转移给 Rust，宿主不要自行删除；Rust 会在生产者确认其已 signal 后删除它。
///
/// 若 fence 为 `0`，则该槽位会被立即复用；宿主必须确保 GPU 已完成对该纹理的采样后再 release
///（例如使用其它同步机制）。
///
/// 若某个 view 创建时指定了 `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE`，则该 view
/// 对应的 consumer fence 必须为 0（会被忽略）。
pub unsafe extern "C" fn xian_web_engine_views_release_frames(
    views: *const *mut XianWebEngineView,
    slots: *const u32,
    consumer_fences: *const u64,
    count: u32,
) {
    if views.is_null() || slots.is_null() || count == 0 {
        return;
    }

    let count = count as usize;
    let view_ptrs = unsafe { std::slice::from_raw_parts(views, count) };
    let slot_values = unsafe { std::slice::from_raw_parts(slots, count) };

    if consumer_fences.is_null() {
        for i in 0..count {
            let view = view_ptrs[i];
            if view.is_null() {
                continue;
            }
            unsafe { (*view).handle.release_slot_with_fence(slot_values[i], 0) };
        }
        return;
    }

    let consumer_fence_values = unsafe { std::slice::from_raw_parts(consumer_fences, count) };
    for i in 0..count {
        let view = view_ptrs[i];
        if view.is_null() {
            continue;
        }

        unsafe {
            (*view)
                .handle
                .release_slot_with_fence(slot_values[i], consumer_fence_values[i])
        };
    }
}
