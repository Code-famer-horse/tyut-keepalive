# Tauri 2 跨平台桌面化设计

## 目标

把现有 Rust 保活工具改造为 Tauri 2 桌面应用，只保留三个用户可感知目标：开机自启、后台留存、界面显示正确。平台为 Windows、macOS、Linux。

## 范围与不变项

- 保留并复用现有 Rust 核心：`config`、`portal`、`dns`、`watch`。登录、保活、退避逻辑不变。
- 现有 eframe/egui UI 层（`src/main.rs`、`args.rs`、`service.rs`、`install.rs`、`bootstrap.rs`、`logging.rs`、`paths.rs`、`state.rs`）由 Tauri 壳替换。当前工作区中未提交的界面卡顿/字体修复继续保留为未提交改动，不在 Tauri 中迁移；Tauri WebView 使用系统字体，无 egui 中文字体问题。
- 凭证仍为安装目录下的 `config.enc`（XOR 0x77 协议混淆，非安全加密）。

## 架构

- 根目录：最小前端（原生 HTML/CSS/JS，无框架），仅状态展示与按钮。
- `src-tauri/`：Tauri 2 Rust 侧。
  - `core/`：从现有实现迁移的登录与保活核心，不依赖 UI。
  - `commands.rs`：`invoke` 入口（重连、重置凭证、自启开关、退出）。
  - `state.rs`：运行状态封装，`emit` 给前端。
  - `lib.rs`：注册插件、启动后台任务、管理窗口与托盘。
- 前后端边界：前端只读状态与调用命令；后台任务不依赖前端生命周期。

## 关键行为

- 开机自启：使用 `tauri-plugin-autostart`，登录时启动，界面提供开关。
- 单实例：使用 `tauri-plugin-single-instance`；二次启动时显示并聚焦已有窗口。
- 后台留存：`setup` 阶段由 Rust 独立线程启动保活循环；关闭主窗口执行隐藏而非退出；托盘菜单「显示」「退出」控制可见性与彻底退出。
- 托盘缺失（Linux）：托盘创建失败不阻断启动；关闭窗口改为最小化；二次启动由单实例插件唤回窗口；彻底退出使用界面按钮。
- 界面自适应：CSS flex/grid 与百分比尺寸；支持窗口拖动缩放；高 DPI 由 WebView 处理；中文字体由系统字体回退。
- 状态事件：`status` 事件包含在线状态、脱敏账号、最近检查、最近登录、最近日志摘要。

## 构建与发布

- GitHub Actions 使用 `tauri-apps/tauri-action`，按 `windows-latest`/`macos-latest`/`ubuntu-latest` 矩阵构建，生成 `.msi`/`.dmg`/`.deb`/`.AppImage` 并上传 GitHub Release。
- macOS 未配置签名证书时使用 ad-hoc 签名，避免下载后被系统标记为损坏。
- Linux 构建需安装 Tauri 官方列出的 WebKitGTK 等系统依赖。

## 验证

- Windows 本机做功能验证：首次运行、凭证生成、自启开关、关窗隐藏、托盘显示/退出、单实例、窗口缩放与中文显示。
- macOS/Linux 本机无图形与打包环境，功能验证只能在 GitHub Actions 产物上进行结构检查；交付说明中必须明确未在本机做三平台功能验证，不以编译成功代替功能验证。
- 最小验证命令：Rust 侧 `cargo test`；前端 `npm run build`；CI 构建成功后检查 Release 资产清单。
