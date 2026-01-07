# xian_web_engine C ABI

## 中文

本仓库产出一个 `cdylib`（`xian_web_engine`），对外仅暴露 **C ABI**：导出符号为 `extern "C"` + `#[no_mangle]`（Rust 2024 下为 `#[unsafe(no_mangle)]`）。

如果你是 Java/Panama、C/C++、Rust(FFI) 宿主，这份文档是你集成时的“调用顺序 + 约束清单 + 复制粘贴示例”。

---

## TL;DR（最小调用顺序）

1. `xian_web_engine_config_init(&cfg)`；填 `cfg.glfw_shared_window` 与 `cfg.glfw_api`（所有函数指针必须非 0）；再 `xian_web_engine_create(&cfg)`。
2. `xian_web_engine_view_config_init(&vcfg)`；填 `vcfg.engine` 与 `vcfg.width/height/target_fps/view_flags`；再 `xian_web_engine_view_create(&vcfg)`。
3. `xian_web_engine_view_set_active(view, 1)`。
4. 刷新驱动（二选一）：
   - 外部 vsync：`target_fps == 0`，每个 vsync/每帧调用一次 `xian_web_engine_tick(engine)`（必须单线程）。
   - 固定间隔：`target_fps != 0`，不需要 `tick`（调用也无害，但通常没必要）。
5. 每帧：
   - `xian_web_engine_views_acquire_frames(...)` 获取 `texture_id` + `producer_fence` + `slot`。
   - 若 `producer_fence != 0`，采样前等待（推荐 `glWaitSync`）；宿主不得删除该 fence（Rust 负责删除）。
   - 采样纹理后可选创建 `consumer_fence`（`glFenceSync`）并用 `xian_web_engine_views_release_frames(...)` 释放；所有权转移给 Rust，宿主不得删除。
6. 退出：先 `xian_web_engine_view_destroy(view)`，再 `xian_web_engine_destroy(engine)`（destroy 后不要再使用 view 指针）。

---

## 关键约定（必须看）

### 1) `struct_size/abi_version` 必须正确

- `xian_web_engine_create` / `xian_web_engine_view_create` 失败只返回 `NULL`，不提供错误码/错误字符串。
- 正确做法：永远先调用 init 函数填好头部与默认值：
  - `xian_web_engine_config_init(&cfg)`
  - `xian_web_engine_view_config_init(&vcfg)`

### 2) `EmbedderGlfwApi` 的所有函数指针必须非 0

- 引擎不会按名称动态加载 GLFW；只使用你提供的函数表。
- 任一字段为 0 都会导致创建失败（返回 `NULL`）。

### 3) 字符串必须是 NUL 结尾 UTF-8（C 字符串）

- 所有 `*const char` 均按 C 字符串解析：遇到第一个 `\0` 截断。
- 非 NUL 结尾属于越界读取（UB）；无效 UTF-8 会被拒绝。
- `resources_dir/config_dir`：`NULL` 或空字符串视为“不设置”。

### 4) 外部 vsync 模式必须周期性 `tick`（且只能单线程）

- `target_fps == 0`：Servo 会把 refresh 回调 push 到队列，等待宿主 `xian_web_engine_tick` 执行。
- 若不 `tick`：刷新会停；回调队列在压力下存在上限，超出会丢弃。
- `tick` 只能由单线程消费（不要并发调用）。

### 5) `acquire` 输出是“紧凑数组”，`release` 输入是“对齐数组”

- `xian_web_engine_views_acquire_frames`：只写入成功 acquire 的帧，输出 `out_frames[0..N)` 与 `out_view_indices[0..N)` 紧凑排列。
- `xian_web_engine_views_release_frames`：要求 `views[i]` / `slots[i]` / `consumer_fences[i]` 一一对应（长度就是你要 release 的数量）。

最小示例（多 view 时用 indices 重映射后再 release）：

```c
uint32_t n = xian_web_engine_views_acquire_frames(views, indices, frames, view_count);

XianWebEngineView* release_views[MAX_VIEWS];
uint32_t release_slots[MAX_VIEWS];
uint64_t release_fences[MAX_VIEWS]; // 或传 NULL 表示全 0

for (uint32_t i = 0; i < n; i++) {
  uint32_t input_index = indices[i];
  release_views[i] = views[input_index];
  release_slots[i] = frames[i].slot;
  release_fences[i] = 0; // 示例：不使用 consumer fence（需自行保证同步）
}

xian_web_engine_views_release_frames(release_views, release_slots, release_fences, n);
```

### 6) fence 同步与所有权

- `XianWebEngineFrame.producer_fence`（引擎提供）：
  - 非 0 时，采样前应等待；宿主不得删除（Rust 负责删除）。
- `consumer_fence`（宿主提供给 `release`）：
  - 采样后创建 `GLsync`，传给 Rust；宿主不得删除，Rust 会在确认 signal 后删除。
  - 若 fence 为 0，槽位会立即可复用；宿主必须确保 GPU 已不再使用该纹理。

性能/安全权衡 flags：
- `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_CONSUMER_FENCE`：忽略 consumer fence（最快，但你必须自行保证不覆盖仍在采样的纹理；且必须传 `consumer_fences = NULL` 或 fence=0，避免 GLsync 泄漏）。
- `XIAN_WEB_ENGINE_VIEW_FLAG_UNSAFE_NO_PRODUCER_FENCE`：不提供 producer fence（更低开销；你必须自行保证不会采样到未完成帧）。

### 7) 输入：可能“部分接受”，inactive 会“计数=全收但实际丢弃”

- `xian_web_engine_view_send_input_events` 返回“已接受数量”，队列满时会小于 `count`。
- view inactive：直接返回 `count`（视为已接受）但事件被丢弃（快路径）。
- `XIAN_WEB_ENGINE_VIEW_FLAG_INPUT_SINGLE_PRODUCER` 仅在你保证只有一个线程发送输入时才能开启；违反即 UB。

---

## ABI 版本与前向兼容

- 当前 ABI 版本：`xian_web_engine_abi_version() == 1`。
- `struct_size` 兼容策略：只要 `struct_size >= sizeof(Struct)` 即视为兼容（允许宿主传入更大的结构体以做前向扩展）。

---

## 附录：最小 C 声明 + 最小示例

> 说明：以下声明块可以直接复制到你项目里的 `.h`；示例块假定你已经包含了这些声明。

### C 声明（可做头文件）

```c
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

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
  void* glfw_shared_window;
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
  uint32_t modifiers;      // 宿主定义；在 Servo 线程映射到 Servo modifiers（未知 bit 会被忽略）

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

#define XIAN_WEB_ENGINE_FLAG_NO_PARK (1u << 0)

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

#ifdef __cplusplus
}
#endif
```

### 最小示例（外部 vsync）

```c
// 说明：示例省略 GLFW 初始化、OpenGL loader、以及你自己的渲染代码；
// 重点是 ABI 调用顺序与 fence 生命周期。
void example_create_and_loop(void* shared_glfw_window, EmbedderGlfwApi glfw_api) {
  XianWebEngineConfig cfg;
  xian_web_engine_config_init(&cfg); // 必须先 init：写入 struct_size/abi_version 与默认值
  cfg.glfw_shared_window = shared_glfw_window;
  cfg.glfw_api = glfw_api;
  cfg.default_width = 1280;
  cfg.default_height = 720;

  XianWebEngine* engine = xian_web_engine_create(&cfg);
  if (!engine) {
    return; // create 失败仅返回 NULL
  }

  XianWebEngineViewConfig vcfg;
  xian_web_engine_view_config_init(&vcfg); // 必须先 init
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

  for (;;) {
    // 外部 vsync 模式：每个 vsync 或每帧调用一次（单线程）
    xian_web_engine_tick(engine);

    XianWebEngineView* views[1] = { view };
    uint32_t indices[1];
    XianWebEngineFrame frames[1];

    uint32_t acquired = xian_web_engine_views_acquire_frames(views, indices, frames, 1);
    if (acquired == 0) {
      continue;
    }

    XianWebEngineFrame f = frames[0];

    // producer_fence 非 0 时，采样前建议等待（且宿主不得删除）
    // if (f.producer_fence != 0) glWaitSync((GLsync)f.producer_fence, 0, GL_TIMEOUT_IGNORED);

    // ...在你的 GL 上下文里采样 f.texture_id...

    // 安全释放：宿主在采样 draw 之后创建 consumer fence；所有权转移给 Rust，宿主不得删除
    // uint64_t consumer_fence = (uint64_t)(uintptr_t)glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
    // xian_web_engine_views_release_frames(views, &f.slot, &consumer_fence, 1);

    // 不提供 consumer fence：你必须自行保证 GPU 已不再使用该纹理后再 release
    xian_web_engine_views_release_frames(views, &f.slot, NULL, 1);
  }

  // 建议：先 destroy view，再 destroy engine（destroy engine 后 view 指针不可再用）
  // xian_web_engine_view_destroy(view);
  // xian_web_engine_destroy(engine);
}
```
