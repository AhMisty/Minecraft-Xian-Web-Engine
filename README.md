# xian_web_engine C ABI（单上下文 / 最高性能取向）

## 中文

`xian_web_engine` 输出一个 `cdylib`（库名：`xian_web_engine`），对外只暴露 C ABI。

C ABI 版本：`1`（`xian_web_engine_abi_version()`）。

本版本已“完全重写”工作模型：

- 宿主只需要提供一个 **GLFW OpenGL 上下文**（`GLFWwindow*`）
- Servo **直接使用该上下文**，并把每个 WebView 渲染到该上下文内创建的 **纹理（texture）**
- 不再创建共享/离屏 GLFW window，不再做跨线程上下文共享/三缓冲/同步 fence
- 对外 API 采用 **单线程模型**：所有函数必须在“该 GLFW 上下文已 current 的同一线程”调用（通常是渲染线程）

---

## TL;DR（最小调用顺序）

1. 初始化配置：`xian_web_engine_config_init(&cfg)`
2. （可选）安装 Servo 内置资源目录：`xian_web_engine_set_resources_dir("...")`（应用启动时调用一次）
3. （可选）设置 Servo 配置目录：`xian_web_engine_set_config_dir("...")`（创建 engine 之前调用）
4. （可选）设置 Servo 工作线程上限：`xian_web_engine_set_thread_pool_cap(n)`（创建 engine 之前调用）
5. 填 `cfg.glfw_window` + `cfg.glfw_api.glfw_get_proc_address`（可选 `glfw_make_context_current`）
6. 创建引擎：`engine = xian_web_engine_create(&cfg)`
7. 创建 view：`xian_web_engine_view_config_init(&vcfg)` -> `vcfg.engine = engine` -> `view = xian_web_engine_view_create(&vcfg)`
8. 每帧调用：`xian_web_engine_tick(engine)`（同一线程；且 `cfg.glfw_window` 的上下文已 current）
9. 获取纹理：`tex = xian_web_engine_view_texture_id(view)` 并在宿主渲染
10. 输入事件：`xian_web_engine_view_send_input_events(view, events, count)`
11. 释放：先 `xian_web_engine_view_destroy(view)`，再 `xian_web_engine_destroy(engine)`

---

## 关键约定（必须看）

### 1) 线程与上下文

- **所有 API 必须在同一线程调用**；不得跨线程使用 `XianWebEngine* / XianWebEngineView*`
- 调用前必须保证 `cfg.glfw_window` 的 OpenGL 上下文已经 **current**
  - 默认 `cfg.assume_context_current = 1`：引擎不会调用 `glfwMakeContextCurrent`（最高性能）
  - 若你希望库内部显式调用 `glfwMakeContextCurrent`：设置 `cfg.assume_context_current = 0`，并提供 `glfw_make_context_current` 指针

### 2) 自动绘制（AUTO_PAINT）

- 默认 `cfg.auto_paint = 1`
- 启用时：`xian_web_engine_tick` 会在 `Servo::spin_event_loop` 后自动绘制所有 dirty view
- 你也可以用：
  - `xian_web_engine_view_needs_paint(view)` 查询
  - `xian_web_engine_view_paint(view)` 手动触发绘制

### 3) 纹理生命周期

- 每个 view 内部持有一个 FBO + `GL_TEXTURE_2D`（RGBA8）
- `xian_web_engine_view_texture_id(view)` 返回的纹理 ID 在 resize 后保持不变（仅重新分配纹理存储）

### 4) 配置作用域

- `xian_web_engine_set_resources_dir`：进程全局（建议在创建 engine 前调用）
- `xian_web_engine_set_config_dir / xian_web_engine_set_thread_pool_cap`：进程全局（必须在创建 engine 前调用；Servo 初始化后不可再改）

---

## C 声明（可做头文件）

```c
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct XianWebEngine XianWebEngine;
typedef struct XianWebEngineView XianWebEngineView;

// --- constants ---

#define XIAN_WEB_ENGINE_ABI_VERSION 1u

#define XIAN_WEB_ENGINE_GL_API_GL   1u
#define XIAN_WEB_ENGINE_GL_API_GLES 2u

#define XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_MOVE   1u
#define XIAN_WEB_ENGINE_INPUT_KIND_MOUSE_BUTTON 2u
#define XIAN_WEB_ENGINE_INPUT_KIND_WHEEL        3u
#define XIAN_WEB_ENGINE_INPUT_KIND_KEY          4u

#define XIAN_WEB_ENGINE_MOD_SHIFT   (1u << 0)
#define XIAN_WEB_ENGINE_MOD_CONTROL (1u << 1)
#define XIAN_WEB_ENGINE_MOD_ALT     (1u << 2)
#define XIAN_WEB_ENGINE_MOD_META    (1u << 3)

// --- structs ---

typedef struct XianWebEngineGlfwApi {
  uintptr_t glfw_get_proc_address;     // required
  uintptr_t glfw_make_context_current; // optional (required if assume_context_current is 0)
} XianWebEngineGlfwApi;

typedef struct XianWebEngineConfig {
  void* glfw_window; // GLFWwindow* (required)
  XianWebEngineGlfwApi glfw_api;
  uint32_t gl_api;   // XIAN_WEB_ENGINE_GL_API_*
  uint32_t assume_context_current; // bool (0/1), default 1
  uint32_t auto_paint;             // bool (0/1), default 1
  uint32_t _reserved0;             // must be 0
} XianWebEngineConfig;

typedef struct XianWebEngineViewConfig {
  XianWebEngine* engine; // required
  uint32_t width;
  uint32_t height;
  float hidpi_scale_factor; // currently ignored (reserved)
  const char* initial_url;  // optional, NUL-terminated UTF-8
} XianWebEngineViewConfig;

typedef struct XianWebEngineInputEvent {
  uint32_t kind;
  float x;
  float y;
  uint32_t modifiers;

  uint32_t mouse_button;
  uint32_t mouse_action; // 0=down, non-zero=up

  double wheel_delta_x;
  double wheel_delta_y;
  double wheel_delta_z;
  uint32_t wheel_mode;   // 0=pixel, 1=line, 2=page

  uint32_t key_state;    // 0=down, non-zero=up
  uint32_t key_location; // 0=standard, 1=left, 2=right, 3=numpad
  uint32_t repeat;       // 0=false, non-zero=true
  uint32_t is_composing; // 0=false, non-zero=true
  uint32_t key_codepoint;
  uint32_t glfw_key;
} XianWebEngineInputEvent;

// --- functions ---

uint32_t xian_web_engine_abi_version(void);

void xian_web_engine_config_init(XianWebEngineConfig* cfg);
bool xian_web_engine_set_resources_dir(const char* resources_dir);
bool xian_web_engine_set_config_dir(const char* config_dir);
bool xian_web_engine_set_thread_pool_cap(uint32_t thread_pool_cap);
XianWebEngine* xian_web_engine_create(const XianWebEngineConfig* cfg);
void xian_web_engine_destroy(XianWebEngine* engine);

bool xian_web_engine_needs_tick(const XianWebEngine* engine);
uint32_t xian_web_engine_tick(XianWebEngine* engine);

void xian_web_engine_view_config_init(XianWebEngineViewConfig* cfg);
XianWebEngineView* xian_web_engine_view_create(const XianWebEngineViewConfig* cfg);
void xian_web_engine_view_destroy(XianWebEngineView* view);

bool xian_web_engine_view_load_url(XianWebEngineView* view, const char* url);
void xian_web_engine_view_resize(XianWebEngineView* view, uint32_t width, uint32_t height);

uint32_t xian_web_engine_view_texture_id(const XianWebEngineView* view);
bool xian_web_engine_view_needs_paint(const XianWebEngineView* view);
bool xian_web_engine_view_paint(XianWebEngineView* view);

uint32_t xian_web_engine_view_send_input_events(
  XianWebEngineView* view,
  const XianWebEngineInputEvent* events,
  uint32_t count);

#ifdef __cplusplus
}
#endif
```

---

## English

`xian_web_engine` builds a `cdylib` (library name: `xian_web_engine`) and exposes a pure C ABI.

C ABI version: `1` (`xian_web_engine_abi_version()`).

This version is a full rewrite with a new high-performance model:

- The embedder provides one **GLFW OpenGL context** (`GLFWwindow*`)
- Servo renders **directly in that context** into per-view **textures**
- No shared/offscreen GLFW windows, no cross-thread context sharing, no triple-buffer fences
- Single-threaded public API: all calls must happen on the thread where the context is current

See the C header block above for the full ABI.

Notes:

- `xian_web_engine_set_config_dir` / `xian_web_engine_set_thread_pool_cap` are process-global and must be called before creating the engine.
