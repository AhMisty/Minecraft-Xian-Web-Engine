//! ### English
//! C ABI bindings for frame acquisition and release.
//!
//! ### 中文
//! 帧获取与释放相关的 C ABI 绑定。

use std::ptr;

use super::{XianWebEngineFrame, XianWebEngineView};

#[unsafe(no_mangle)]
/// ### English
/// Tries to acquire the latest READY frame for one view.
///
/// If a frame is acquired, writes it to `out_frame` and returns `true`.
///
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create` (must not be NULL).
/// - `out_frame`: Output pointer receiving one acquired frame (must not be NULL).
///
/// #### Safety
/// - `view` must be a valid pointer returned by `xian_web_engine_view_create`.
/// - `out_frame` must be valid for writes of `sizeof(XianWebEngineFrame)` bytes.
///
/// ### 中文
/// 尝试获取单个 view 的最新 READY 帧。
///
/// 若成功 acquire，则写入 `out_frame` 并返回 `true`。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针（必须非 NULL）。
/// - `out_frame`：输出指针，用于接收一个 acquired frame（必须非 NULL）。
///
/// #### 安全
/// - `view` 必须是由 `xian_web_engine_view_create` 返回的有效指针。
/// - `out_frame` 必须至少可写 `sizeof(XianWebEngineFrame)` 字节。
pub unsafe extern "C" fn xian_web_engine_view_acquire_frame(
    view: *mut XianWebEngineView,
    out_frame: *mut XianWebEngineFrame,
) -> bool {
    if view.is_null() || out_frame.is_null() {
        return false;
    }

    let handle = unsafe { &(*view).handle };
    let Some(frame) = handle.acquire_frame() else {
        return false;
    };

    unsafe {
        ptr::write_unaligned(out_frame, frame.into());
    }
    true
}

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
/// Each acquired frame includes a `slot` index; the embedder must later release it via
/// `xian_web_engine_views_release_frames`.
///
/// #### Parameters
/// - `views`: Pointer to an array of `count` view pointers.
/// - `out_view_indices`: Output array receiving input indices for each acquired frame.
/// - `out_frames`: Output array receiving acquired frames (dense, 0..return_value).
/// - `count`: Number of entries in the `views` array (and capacity required for the outputs).
///
/// #### Safety
/// - If `count != 0`, `views` must be valid for reads of `count` pointers.
/// - `out_view_indices` and `out_frames` must be valid for writes of at least `count` elements.
/// - Pointers may be unaligned; this function handles unaligned loads/stores.
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
///
/// 每个 acquired frame 都包含 `slot` 索引；宿主必须在之后通过 `xian_web_engine_views_release_frames`
/// 释放该槽位。
///
/// #### 参数
/// - `views`：指向长度为 `count` 的 view 指针数组。
/// - `out_view_indices`：输出数组，写入每个 acquired frame 对应的输入下标。
/// - `out_frames`：输出数组，写入 acquired frames（0..返回值紧凑排列）。
/// - `count`：`views` 数组长度（也是输出数组需要满足的最小容量）。
///
/// #### 安全
/// - 若 `count != 0`，则 `views` 必须至少可读 `count` 个指针。
/// - `out_view_indices` 与 `out_frames` 必须至少可写 `count` 个元素。
/// - 指针允许非对齐；本函数会使用非对齐读写进行兼容处理。
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
    let mut acquired = 0usize;
    if super::is_aligned_ptr(views)
        && super::is_aligned_ptr(out_view_indices)
        && super::is_aligned_ptr(out_frames)
    {
        let view_ptrs = unsafe { std::slice::from_raw_parts(views, count) };
        let indices_out = unsafe { std::slice::from_raw_parts_mut(out_view_indices, count) };
        let frames_out = unsafe { std::slice::from_raw_parts_mut(out_frames, count) };

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
    } else {
        for i in 0..count {
            let view_ptr = unsafe { ptr::read_unaligned(views.add(i)) };
            if view_ptr.is_null() {
                continue;
            }

            let view_handle = unsafe { &(*view_ptr).handle };
            if let Some(frame) = view_handle.acquire_frame() {
                unsafe {
                    ptr::write_unaligned(out_view_indices.add(acquired), i as u32);
                    ptr::write_unaligned(out_frames.add(acquired), frame.into());
                }
                acquired += 1;
            }
        }
    }

    acquired as u32
}

#[unsafe(no_mangle)]
/// ### English
/// Releases one previously acquired frame slot for one view.
///
/// If `consumer_fence` is `0`, the slot becomes immediately reusable; the embedder must ensure the
/// texture is no longer in use by the GPU before releasing (e.g., via other synchronization).
///
/// If the view was created with `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE`, the fence value
/// is ignored (treated as `0`).
///
/// #### Parameters
/// - `view`: View pointer returned by `xian_web_engine_view_create` (may be NULL).
/// - `slot`: Slot index returned by acquire (see `XianWebEngineFrame.slot`).
/// - `consumer_fence`: Optional consumer fence (`GLsync` cast to `u64`), or 0 to skip.
///
/// #### Safety
/// If non-NULL, `view` must be a valid pointer returned by `xian_web_engine_view_create`.
///
/// ### 中文
/// 释放单个 view 之前 acquire 的帧槽位。
///
/// 若 `consumer_fence` 为 `0`，则该槽位会被立即复用；宿主必须确保 GPU 已完成对该纹理的采样后再 release
///（例如使用其它同步机制）。
///
/// 若 view 创建时指定了 `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE`，则 fence 会被忽略（视为 0）。
///
/// #### 参数
/// - `view`：由 `xian_web_engine_view_create` 返回的 view 指针（允许为 NULL）。
/// - `slot`：acquire 返回的槽位索引（见 `XianWebEngineFrame.slot`）。
/// - `consumer_fence`：可选 consumer fence（`GLsync` 转为 `u64`），为 0 则跳过。
///
/// #### 安全
/// 若 `view` 非 NULL，则它必须是由 `xian_web_engine_view_create` 返回的有效指针。
pub unsafe extern "C" fn xian_web_engine_view_release_frame(
    view: *mut XianWebEngineView,
    slot: u32,
    consumer_fence: u64,
) {
    if view.is_null() {
        return;
    }

    unsafe {
        (*view).handle.release_slot_with_fence(slot, consumer_fence);
    }
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
/// #### Parameters
/// - `views`: Pointer to an array of `count` view pointers.
/// - `slots`: Pointer to an array of `count` slot indices to release (one per view).
/// - `consumer_fences`: Optional pointer to an array of `count` `GLsync` values (cast to `u64`).
/// - `count`: Number of entries in the arrays.
///
/// #### Safety
/// - If `count != 0`, `views` and `slots` must be valid for reads of `count` elements.
/// - If `consumer_fences` is non-NULL, it must be valid for reads of `count` elements.
/// - Pointers may be unaligned; this function handles unaligned loads.
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
///
/// #### 参数
/// - `views`：指向长度为 `count` 的 view 指针数组。
/// - `slots`：指向长度为 `count` 的 slot 索引数组（每个 view 一个）。
/// - `consumer_fences`：可选数组指针，长度为 `count`，元素为 `GLsync`（转为 `u64`）。
/// - `count`：数组长度。
///
/// #### 安全
/// - 若 `count != 0`，则 `views` 与 `slots` 必须至少可读 `count` 个元素。
/// - 若 `consumer_fences` 非 NULL，则它必须至少可读 `count` 个元素。
/// - 指针允许非对齐；本函数会使用非对齐读写进行兼容处理。
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
    if super::is_aligned_ptr(views)
        && super::is_aligned_ptr(slots)
        && (consumer_fences.is_null() || super::is_aligned_ptr(consumer_fences))
    {
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
        return;
    }

    for i in 0..count {
        let view = unsafe { ptr::read_unaligned(views.add(i)) };
        if view.is_null() {
            continue;
        }

        let slot = unsafe { ptr::read_unaligned(slots.add(i)) };
        let fence = if consumer_fences.is_null() {
            0
        } else {
            unsafe { ptr::read_unaligned(consumer_fences.add(i)) }
        };
        unsafe { (*view).handle.release_slot_with_fence(slot, fence) };
    }
}
