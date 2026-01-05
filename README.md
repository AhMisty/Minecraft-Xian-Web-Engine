# xian_web_engine C ABI

## 中文

本仓库产出一个 `cdylib`（`xian_web_engine`），对外仅暴露 **C ABI**：导出符号为 `extern "C"` + `#[no_mangle]`。

如果你是 Java/Panama、C/C++、Rust(FFI) 宿主，这份文档是你集成时的“对照表 + 避坑清单”。

---

## 一句话理解这个 ABI

- 你提供一个 **已存在且可共享的 GLFW 上下文窗口**（`glfw_shared_window`）和一组 **GLFW 函数指针表**（`EmbedderGlfwApi`）。
- 引擎在独立 Servo 线程创建一个离屏共享上下文，把每帧渲染结果写到 **共享纹理**里。
- 宿主线程批量 `acquire` 最新帧（得到 `texture_id` + fence），在自己的 GL 上下文里采样，然后用 `release` 归还槽位。

---

## 最常见的痛点（必须看）

### 1) `struct_size/abi_version` 没填对：所有 create 都会“静默失败”

- `xian_web_engine_create` / `xian_web_engine_view_create` 失败时只返回 `NULL`，不会给错误码。
- 正确做法：永远先调用初始化函数填好头部：
  - `xian_web_engine_config_init(&cfg)`
  - `xian_web_engine_view_config_init(&vcfg)`

### 2) 字符串必须是 NUL 结尾 UTF-8（C 字符串）

- 所有 `*const c_char` 都按 **C 字符串**解析：遇到第一个 `\\0` 就截断。
- 不是 NUL 结尾会导致越界读取（UB）；包含内部 NUL 会被截断（“看起来传了，实际没传全”）。

### 3) 外部 vsync 模式下必须周期性调用 `xian_web_engine_tick`

- `target_fps == 0` 表示由宿主驱动 vsync：Servo 会把回调塞进队列，等待宿主 `tick` 执行。
- 如果你不 `tick`：刷新会停、回调可能丢弃（队列溢出有上限）。
- `tick` 只能由 **单线程**消费（不要并发调用）。

### 4) fence 同步是“帧稳定性”的核心：错用会撕裂/花屏/泄漏

- `producer_fence`（引擎提供）：
  - 非 0 时，宿主在采样纹理前应等待它（推荐 `glWaitSync`）。
  - 宿主 **不得删除** 该 fence（Rust 负责删除）。
- `consumer_fence`（宿主提供）：
  - 你在采样纹理的 draw 命令之后创建 `GLsync`，把它传给 `release`，所有权转移给 Rust。
  - 宿主 **不得删除** 该 fence（Rust 在确认 signal 后删除）。
- `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE`：
  - 该模式下 Rust 会忽略 consumer fence；你如果还创建 fence 但又不删除，会造成 **GLsync 泄漏**。
  - 正确做法：传 `consumer_fences = NULL` 或者每个 fence 值为 0。

### 5) `acquire` 输出是“紧凑数组”，`release` 输入是“对齐数组”

- `xian_web_engine_views_acquire_frames`：只写入成功 acquire 的帧，输出是 0..N 的紧凑数组，并额外给出 `out_view_indices` 映射到输入 `views[]` 的下标。
- `xian_web_engine_views_release_frames`：要求 `views[i]` / `slots[i]` / `consumer_fences[i]` 一一对应（长度就是你要 release 的数量）。
- 典型做法：用 `out_view_indices` 把 `views[]` 重新映射成一个紧凑的 `release_views[]` 再调用 `release`。

最小示例（多 view 时不要直接拿原始 `views[]` 去 release）：

```c
uint32_t acquired = xian_web_engine_views_acquire_frames(views, indices, frames, view_count);

XianWebEngineView* release_views[MAX_VIEWS];
uint32_t release_slots[MAX_VIEWS];
uint64_t release_fences[MAX_VIEWS]; // 或者传 NULL 表示全 0

for (uint32_t i = 0; i < acquired; i++) {
  uint32_t input_index = indices[i];
  release_views[i] = views[input_index];
  release_slots[i] = frames[i].slot;
  release_fences[i] = 0; // 示例：不使用 consumer fence（需自行保证 GPU 已不再使用纹理）
}

xian_web_engine_views_release_frames(release_views, release_slots, release_fences, acquired);
```

### 6) 输入发送可能“部分接受”，而且 inactive 会“计数=全收但实际丢弃”

- `xian_web_engine_view_send_input_events` 返回“已接受数量”，队列满时会小于 `count`。
- view inactive 时：直接返回 `count`（视为已接受）但事件会被丢弃（快路径）。
- `XIAN_WEB_ENGINE_VIEW_FLAG_INPUT_SINGLE_PRODUCER` 仅在你 **保证只有一个线程** 调用发送输入时才能开启；违反即未定义行为。

最小示例（处理部分接受）：

```c
uint32_t accepted = xian_web_engine_view_send_input_events(view, events, count);
if (accepted < count) {
  // 队列满：剩余事件要么丢弃，要么延后重试（由宿主策略决定）
}
```

---

## ABI 版本

- `src/ffi/mod.rs`：`xian_web_engine_abi_version() -> u32`
- 当前 ABI 版本：`3`
- `struct_size` 规则：只要 `struct_size >= sizeof(Struct)` 即视为兼容（允许宿主传入更大的结构体以实现前向兼容）。

---

## 导出函数（C ABI）

声明位置：`src/ffi/*`

- `src/ffi/mod.rs:292` `uint32_t xian_web_engine_abi_version(void);`
- `src/ffi/config.rs:35` `void xian_web_engine_config_init(XianWebEngineConfig* cfg);`
- `src/ffi/config.rs:74` `void xian_web_engine_view_config_init(XianWebEngineViewConfig* cfg);`
- `src/ffi/engine.rs:42` `XianWebEngine* xian_web_engine_create(const XianWebEngineConfig* cfg);`
- `src/ffi/engine.rs:96` `void xian_web_engine_destroy(XianWebEngine* engine);`
- `src/ffi/engine.rs:123` `void xian_web_engine_tick(XianWebEngine* engine);`
- `src/ffi/view.rs:38` `XianWebEngineView* xian_web_engine_view_create(const XianWebEngineViewConfig* cfg);`
- `src/ffi/view.rs:86` `void xian_web_engine_view_destroy(XianWebEngineView* view);`
- `src/ffi/view.rs:115` `void xian_web_engine_view_set_active(XianWebEngineView* view, uint8_t active);`
- `src/ffi/view.rs:160` `bool xian_web_engine_view_load_url(XianWebEngineView* view, const char* url);`
- `src/ffi/view.rs:205` `void xian_web_engine_view_resize(XianWebEngineView* view, uint32_t w, uint32_t h);`
- `src/ffi/input.rs:47` `uint32_t xian_web_engine_view_send_input_events(XianWebEngineView* view, const XianWebEngineInputEvent* events, uint32_t count);`
- `src/ffi/frame.rs:53` `uint32_t xian_web_engine_views_acquire_frames(XianWebEngineView* const* views, uint32_t* out_view_indices, XianWebEngineFrame* out_frames, uint32_t count);`
- `src/ffi/frame.rs:132` `void xian_web_engine_views_release_frames(XianWebEngineView* const* views, const uint32_t* slots, const uint64_t* consumer_fences, uint32_t count);`

---

## 主要 ABI 结构体与常量

### 不透明句柄

宿主只应保存/传回指针，不要解引用其内部字段：

- `XianWebEngine`：由 `xian_web_engine_create` 返回，用 `xian_web_engine_destroy` 释放
- `XianWebEngineView`：由 `xian_web_engine_view_create` 返回，用 `xian_web_engine_view_destroy` 释放

### 配置结构体（必须先 init）

- `XianWebEngineConfig`：`src/ffi/mod.rs:148`
  - 必填：`glfw_shared_window`、`glfw_api`（所有函数指针必须非 0，否则创建失败）
  - 可选：`default_width/default_height`、`thread_pool_cap`、`engine_flags`、`resources_dir/config_dir`
- `XianWebEngineViewConfig`：`src/ffi/mod.rs:221`
  - 必填：`engine`
  - 可选：`width/height`、`target_fps`、`view_flags`
  - 约定：`width==0 || height==0` 会使用引擎默认尺寸（来自 `XianWebEngineConfig`）

### 帧结构体

- `XianWebEngineFrame`：`src/ffi/mod.rs:63`
  - `texture_id`：宿主可在自己的 GL 上下文里采样的纹理 ID
  - `producer_fence`：非 0 时采样前应等待；宿主不得删除
  - `slot`：三缓冲槽位索引（0..=2），release 时必须原样传回

### 输入结构体与 kind 常量

声明位置：`src/engine/input_types.rs`

- `XianWebEngineInputEvent`：POD 结构体，方便宿主批量传数组
- `XIAN_WEB_ENGINE_INPUT_KIND_*`：
  - `MOUSE_MOVE=1`、`MOUSE_BUTTON=2`、`WHEEL=3`、`KEY=4`

### flags 常量（性能/安全权衡）

声明位置：`src/engine/flags.rs`

- 引擎 flags：
  - `XIAN_WEB_ENGINE_ENGINE_FLAG_NO_PARK`：禁用 park/unpark（busy-spin，低延迟但高 CPU）
- view flags：
  - `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE`：不记录 consumer fence（最快，但需要你自行保证不覆盖仍在采样的纹理；且不得创建/传入 fence）
  - `XIAN_WEB_ENGINE_VIEW_FLAG_INPUT_SINGLE_PRODUCER`：你保证单线程发送输入（启用更快路径；违反即 UB）
  - `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_PRODUCER_FENCE`：不提供 producer fence（更低开销；你必须自行保证采样不会读到未完成帧）

---

## 示例（C 伪代码，重点演示“最容易踩坑的地方”）

> 说明：示例省略了 GLFW 初始化、OpenGL loader、以及你自己的渲染代码；重点是 ABI 调用顺序与 fence 生命周期。

```c
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

typedef struct XianWebEngine XianWebEngine;
typedef struct XianWebEngineView XianWebEngineView;

typedef struct EmbedderGlfwApi {
  uintptr_t glfw_get_proc_address;
  uintptr_t glfw_make_context_current;
  uintptr_t glfw_default_window_hints;
  uintptr_t glfw_window_hint;
  uintptr_t glfw_get_window_attrib;
  uintptr_t glfw_create_window;
  uintptr_t glfw_destroy_window;
} EmbedderGlfwApi;

typedef struct XianWebEngineConfig {
  uint32_t struct_size;
  uint32_t abi_version;
  void*    glfw_shared_window;
  EmbedderGlfwApi glfw_api;
  uint32_t default_width;
  uint32_t default_height;
  uint32_t thread_pool_cap;
  uint32_t engine_flags;
  const char* resources_dir;
  const char* config_dir;
} XianWebEngineConfig;

typedef struct XianWebEngineViewConfig {
  uint32_t struct_size;
  uint32_t abi_version;
  XianWebEngine* engine;
  uint32_t width;
  uint32_t height;
  uint32_t target_fps;   // 0 = 外部 vsync（需要 tick），非 0 = 固定间隔
  uint32_t view_flags;
} XianWebEngineViewConfig;

typedef struct XianWebEngineFrame {
  uint32_t slot;
  uint32_t texture_id;
  uint64_t producer_fence; // GLsync cast to u64 (0 = 无)
  uint32_t width;
  uint32_t height;
} XianWebEngineFrame;

typedef struct XianWebEngineInputEvent {
  uint32_t kind;
  float x;
  float y;
  uint32_t modifiers;      // 直接映射到 Servo modifiers 位；未知 bit 会被忽略

  uint32_t mouse_button;   // GLFW button 值
  uint32_t mouse_action;   // 0=down, 其它=up

  double wheel_delta_x;
  double wheel_delta_y;
  double wheel_delta_z;
  uint32_t wheel_mode;     // 0=pixel, 1=line, 2=page

  uint32_t key_state;      // 0=down, 其它=up
  uint32_t key_location;   // 0=standard, 1=left, 2=right, 3=numpad
  uint32_t repeat;         // 0=false, 其它=true
  uint32_t is_composing;   // 0=false, 其它=true
  uint32_t key_codepoint;  // Unicode codepoint（未知=0）
  uint32_t glfw_key;       // GLFW key code
} XianWebEngineInputEvent;

#define XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE 1u
#define XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON 2u
#define XIAN_WEB_ENGINE_INPUT_KIND_WHEEL 3u
#define XIAN_WEB_ENGINE_INPUT_KIND_KEY 4u

#define XIAN_WEB_ENGINE_ENGINE_FLAG_NO_PARK (1u << 0)

#define XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE (1u << 0)
#define XIAN_WEB_ENGINE_VIEW_FLAG_INPUT_SINGLE_PRODUCER (1u << 1)
#define XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_PRODUCER_FENCE (1u << 2)

uint32_t xian_web_engine_abi_version(void);
void xian_web_engine_config_init(XianWebEngineConfig* cfg);
XianWebEngine* xian_web_engine_create(const XianWebEngineConfig* cfg);
void xian_web_engine_destroy(XianWebEngine* engine);
void xian_web_engine_tick(XianWebEngine* engine);

void xian_web_engine_view_config_init(XianWebEngineViewConfig* cfg);
XianWebEngineView* xian_web_engine_view_create(const XianWebEngineViewConfig* cfg);
void xian_web_engine_view_destroy(XianWebEngineView* view);
void xian_web_engine_view_set_active(XianWebEngineView* view, uint8_t active);
bool xian_web_engine_view_load_url(XianWebEngineView* view, const char* url);
void xian_web_engine_view_resize(XianWebEngineView* view, uint32_t width, uint32_t height);

uint32_t xian_web_engine_view_send_input_events(
  XianWebEngineView* view,
  const XianWebEngineInputEvent* events,
  uint32_t count);

uint32_t xian_web_engine_views_acquire_frames(
  XianWebEngineView* const* views,
  uint32_t* out_view_indices,
  XianWebEngineFrame* out_frames,
  uint32_t count);

void xian_web_engine_views_release_frames(
  XianWebEngineView* const* views,
  const uint32_t* slots,
  const uint64_t* consumer_fences, // 可为 NULL
  uint32_t count);

void example_create_and_loop(void* shared_glfw_window, EmbedderGlfwApi glfw_api) {
  XianWebEngineConfig cfg;
  xian_web_engine_config_init(&cfg); // 痛点：必须先 init
  cfg.glfw_shared_window = shared_glfw_window;
  cfg.glfw_api = glfw_api;
  cfg.default_width = 1280;
  cfg.default_height = 720;

  XianWebEngine* engine = xian_web_engine_create(&cfg);
  if (!engine) {
    // 痛点：create 失败没有错误码
    return;
  }

  XianWebEngineViewConfig vcfg;
  xian_web_engine_view_config_init(&vcfg); // 痛点：必须先 init
  vcfg.engine = engine;
  vcfg.width = 0;      // 0 表示使用 engine default
  vcfg.height = 0;
  vcfg.target_fps = 0; // 外部 vsync：你必须周期性 tick
  vcfg.view_flags = 0;

  XianWebEngineView* view = xian_web_engine_view_create(&vcfg);
  if (!view) {
    xian_web_engine_destroy(engine);
    return;
  }
  xian_web_engine_view_set_active(view, 1);

  // 你的渲染/主循环
  for (;;) {
    // 外部 vsync 模式：每个 vsync 或每帧调用一次（单线程）
    xian_web_engine_tick(engine);

    XianWebEngineView* views[1] = { view };
    uint32_t indices[1] = { 0 };
    XianWebEngineFrame frames[1];
    uint32_t acquired = xian_web_engine_views_acquire_frames(views, indices, frames, 1);
    if (acquired == 0) {
      continue;
    }

    // 取出 frame
    XianWebEngineFrame f = frames[0];

    // 痛点：producer_fence 非 0 时，采样前必须等待（且不得删除）
    // if (f.producer_fence != 0) glWaitSync((GLsync)f.producer_fence, 0, GL_TIMEOUT_IGNORED);

    // ...在你的 GL 上下文里采样 f.texture_id...

    // 痛点：安全释放需要 consumer fence（所有权转移给 Rust，宿主不得删除）
    // uint64_t consumer_fence = (uint64_t)(uintptr_t)glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
    // xian_web_engine_views_release_frames(views, &f.slot, &consumer_fence, 1);

    // 如果你不提供 consumer fence，则必须确保 GPU 已不再使用该纹理后再 release：
    xian_web_engine_views_release_frames(views, &f.slot, NULL, 1);
  }

  // 建议：先 destroy view，再 destroy engine（destroy engine 后 view 指针不可再用）
  // xian_web_engine_view_destroy(view);
  // xian_web_engine_destroy(engine);
}
```
