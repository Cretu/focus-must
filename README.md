# Focus Must

一个面向 macOS 的专注管理桌面应用：
先写下任务，再开始专注；支持白名单应用、休息计时、历史记录与托盘控制。

## 项目定位

- 平台：macOS（使用 macOS 原生 API 监控前台应用）
- 形态：Tauri 2 + Vue 3 的桌面应用
- 目标：减少无意识切换应用，强化「先计划、后执行」

## 核心功能

- 专注会话：填写任务描述 + 选择允许使用的 APP 后开始专注（⌘/Ctrl + Enter 可快速开始）
- 白名单拦截：切到非白名单 APP 时，弹出主窗口提示并阻断分心
- 多显示器遮罩：副屏显示专注/休息遮罩窗，并轮播励志语录
- 休息模式：可选 5/10/15/30/45 分钟或自定义时长
- 统计分析：总专注/休息时长、会话数、近 30 天趋势与专注时段分布
- 历史记录：支持「全部 / 专注 / 休息」筛选，分页加载，时间与时长精确到秒
- 任务复用：自动提供最近专注任务，点击即可快速填充
- 托盘控制：显示计划窗口、结束专注、结束休息、打开设置、退出
- 开机启动：可选开机自动启动
- 启动体验：应用启动时显示加载动画，主界面初始化完成后进入计划页

## 技术栈

- 前端：Vue 3 + TypeScript + Vite
- 桌面容器：Tauri 2
- 后端：Rust
- macOS 原生能力：`objc2` / `objc2-app-kit`（运行中应用、前台应用、图标）

## 架构总览

### 前端（`src/`）

- `src/main.ts`：应用入口
- `src/App.vue`：根组件，区分主窗口与副屏遮罩窗（overlay）
- `src/stores/appStore.ts`：Pinia 全局状态与核心动作编排
- `src/components/`：界面组件
  - `PlanningView.vue`：计划页（任务、APP 选择、休息、历史）
  - `FocusSessionCard.vue`：专注进行中视图
  - `OverlayView.vue`：副屏遮罩（专注/休息/计划提示 + 语录）
  - `BlockedAppPanel.vue`：分心拦截提示
  - `SettingsView.vue`：设置页
  - `AnalyticsView.vue`：统计分析
  - `AppGrid.vue`：APP 网格选择
  - `HistoryList.vue`：历史记录展示与筛选
- `src/composables/`：可复用逻辑（`useHistory`、`useSnowEffect`）
- `src/i18n.ts` + `src/locales/`：国际化（简体中文 / English）
- `src/styles/main.css`：全局样式与布局

### 后端（`src-tauri/src/`）

- `src-tauri/src/lib.rs`：Tauri 命令、托盘菜单、应用状态、线程启动
- `src-tauri/src/app_monitor.rs`：前台 APP 监控、白名单判断、图标提取
- `src-tauri/src/storage.rs`：本地设置与会话历史存储

### 前后端契约

前端调用的主要命令：

- `get_running_apps` / `get_app_icon` / `get_app_info`
- `get_state`
- `unlock_session` / `lock_session`
- `start_free_activity`
- `switch_to_app`
- `allow_app_temporarily` / `dismiss_distraction`
- `update_settings` / `set_locale`
- `get_history_page`
- `get_analytics`

后端推送的主要事件：

- `state-changed`
- `blocked-app`（分心提示，携带被收起的 App 名称/包名）
- `show-view`

## 数据与本地存储

默认存储目录：`~/.focusmust`

- `settings.json`：用户设置（默认白名单、语言）
- `history.db`：会话历史（SQLite，含专注/休息记录与统计查询）
- `sessions.jsonl`：旧版历史格式；首次启动会自动迁移进 `history.db` 并重命名为 `sessions.jsonl.migrated`。仅当 SQLite 不可用时才作为回退写入。

## 开发环境要求

- macOS 14+
- Node.js 18+
- Rust stable
- Xcode Command Line Tools

推荐 IDE：VS Code + Vue (Volar) + rust-analyzer + Tauri 插件

## 快速开始

```bash
npm install
```

### 启动前端开发

```bash
npm run dev
```

### 启动 Tauri 开发模式

```bash
npm run tauri dev
```

## 常用命令

```bash
# 前端类型检查 + 打包
npm run build

# 预览前端构建结果
npm run preview

# Tauri 构建（含 .app / .dmg）
npm run tauri build

# 按 tag 同步版本后构建（例如 v0.2.0）
npm run tauri:build:tag -- v0.2.0
```

Rust 侧检查（在 `src-tauri/` 下执行）：

```bash
cargo check
cargo test
```

## 打包与产物

执行：

```bash
npm run tauri build
```

如果希望产物版本号跟发布 tag 一致，使用：

```bash
npm run tauri:build:tag -- v0.2.0
```

这会把以下文件的版本统一为 tag（支持 `v1.2.3` 或 `1.2.3`）：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

默认产物路径：

- `src-tauri/target/release/bundle/macos/Focus Must.app`
- `src-tauri/target/release/bundle/dmg/Focus Must_<version>_aarch64.dmg`

## GitHub 上传建议

推荐上传（源码与配置）：

- `src/`
- `src-tauri/src/`
- `src-tauri/icons/`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `package.json` / `package-lock.json`
- `tsconfig.json` / `tsconfig.node.json`
- `vite.config.ts`
- `index.html`
- `public/`
- `README.md`

不要上传（构建产物与本地文件）：

- `node_modules/`
- `dist/`
- `src-tauri/target/`
- `src-tauri/gen/schemas/`
- `*.dmg` / `*.app`
- `.env*`

本仓库已配置对应忽略规则：`.gitignore`

## 平台说明

本项目当前为 macOS 优先实现，核心能力依赖 macOS API（运行中应用与前台监控）。
非 macOS 平台即使能编译，也无法保证功能完整可用。

## 后续发布（可选）

若你要自动发布安装包，可在 GitHub 上配置 Actions：

- 触发：`push` tag（如 `v0.1.1`）
- 执行：`npm run tauri:build:tag`
- 上传：产物到 GitHub Release

如需对外分发，建议增加 macOS 签名与公证流程。
