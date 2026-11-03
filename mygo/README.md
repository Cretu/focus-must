# Focus Must · MyGo 版

用 [MyGo](https://mygo.egoist.dev/) 原生 UI 重写的 Focus Must：**纯 Go、无 WebView、无 cgo**，界面由 MyGo 在 GPU 上直接绘制（macOS 为 Metal）。

与仓库根目录的 Tauri 版并行存在，功能一一对应；数据目录相同（`~/.focusmust`），两版可以互相读取对方的设置与历史。

## 快速开始

需要 Go 1.27+（`go.mod` 已声明，旧版 Go 会自动下载对应工具链）。

```bash
cd mygo
go tool mygo dev        # 开发：热重载
go test ./...           # 测试：视图测试无需窗口，Linux 上也能跑
go tool mygo build      # 打包：.app + .dmg（dmg 与签名需在 macOS 上执行）
```

在 Linux 上也能交叉编译出 macOS 应用：`go tool mygo build -platform darwin/arm64`。

> 前台监控、收起应用等能力依赖 macOS（NSWorkspace）；在其他系统上应用可运行，但不会拦截任何应用。

## 架构

| 职责 | Tauri 版 | MyGo 版 |
|---|---|---|
| 会话状态机（开始 / 暂停 / 继续 / 结束、休息、白名单判定） | `src-tauri/src/state.rs` + `commands.rs` | `internal/core` |
| 设置与历史（settings.json + SQLite，JSONL 兜底与迁移） | `src-tauri/src/storage.rs`（rusqlite） | `internal/storage`（MyGo SQLite 插件，purego 加载） |
| 前台应用 / 运行中应用 / 收起应用 | `app_monitor.rs`（objc2） | `internal/macos`（purego 调 AppKit） |
| 托盘、主窗口、多显示器遮罩、轮询 | `lib.rs` + `tray.rs` | `desktop.go` |
| 界面 | Vue 3 + Nuxt UI（`src/components`） | `view_*.go`（MyGo `ui` 包） |
| 状态管理 | Pinia（`stores/appStore.ts`） | `app.go`（`focusApp`，全部在主线程） |
| 国际化 | vue-i18n + `src/locales` | `internal/i18n`（复用同一份语言文件，补充托盘与通知文案） |
| 图标 | 构建期打包的 Lucide | `internal/icons`（由同一份 Lucide 数据生成） |
| 品牌标记 / 托盘图标 | `BrandMark.vue` / 应用图标 | `internal/brand`（同一几何，矢量绘制，托盘为模板图） |

要点：

- **单线程模型**：视图、托盘菜单回调、每 500ms 的前台监控都在主线程执行（`mygo.RunOnMain`），状态无需加锁。
- **可测试**：`app.go` 通过 `system` 接口访问桌面能力，测试用假实现驱动真实视图（`ui.NewTester`），覆盖专注全流程、休息、提醒、设置、统计与遮罩。
- **截图自检**：`FOCUS_SCREENSHOTS=<目录> go test -run Screenshots .` 会把各界面（浅色 / 深色）渲染成 PNG。

## 与 Tauri 版的差异

| 项目 | 说明 |
|---|---|
| 启动 | 无开屏动画：原生 UI 首帧即可交互，不再需要 HTML 开屏遮挡加载 |
| 外观偏好 | 改为写入 `settings.json` 的 `appearance` 字段（原为 WebView 的 localStorage） |
| 下雪开关 | 同上，写入 `snow` 字段 |
| 暂停时长 | 结束暂停中的会话时，暂停时间不计入专注时长（原版会计入） |
| 提醒后去休息 | 从休息提醒进入休息时，之前的专注会话会被记录（原版会丢失这条记录） |
| 系统语言检测 | 使用 `mygo.App.PreferredLanguages()`，不再调用 `defaults read` |

## 目录

```
mygo/
├── main.go            入口
├── app.go             状态与动作（focusApp）
├── desktop.go         窗口、托盘、监控（system 的真实实现）
├── theme.go           设计令牌与通用组件
├── view_*.go          各界面
├── snow.go            下雪效果与遮罩语录
├── internal/          core · storage · macos · i18n · icons · brand
├── scripts/           gen-icons.mjs（重新生成图标）
├── resources/         应用图标
└── mygo.json          打包配置（LSUIElement：不显示 Dock 图标）
```
