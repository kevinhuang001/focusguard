# FocusGuard · 专注监控（原型）

一个基于 **Tauri 2** 的跨平台（Windows / macOS / Linux）桌面应用原型：通过**屏幕 / 摄像头**采集画面，在后台调用**本地小视觉语言模型**判断你是否在专注当前任务；一旦判定「开小差」，可用**系统通知**或**语音**提醒你回到正轨。

> ⚠️ 这是一个**原型**（prototype）：核心链路完整可跑，但打包、权限、边界情况等还有打磨空间。

## ✨ 功能特性

- 📺 **屏幕监控**：按设定的间隔截取屏幕（Windows / macOS / Linux-X11）。
- 📷 **摄像头监控**：按设定的间隔抓取摄像头帧（Windows / macOS / Linux-V4L2）。
- 🧠 **本地小模型判断**：接入任意 **OpenAI 兼容**的视觉模型服务（本地 Ollama、vLLM、LocalAI，或云端 OpenAI/DeepSeek 等），只需填一个兼容 URL。**所有画面只发给该服务，应用不代理模型的下载/安装**。
- 🎨 **分别配置提示词**：屏幕与摄像头可以同时开启，并各自配置独立的「任务提示词」。
- 🔔 **双提醒方式**：系统通知（Windows 通知中心 / macOS 通知 / Linux libnotify）与语音播报（Windows SAPI / macOS say / Linux speech-dispatcher），可二选一或都要；支持提醒冷却与「连续 N 次开小差才提醒」。
- 🖥 **GPU 检测与参数推荐**：自动检测显卡（nvidia-smi / system_profiler / PowerShell / lspci），按显存推荐模型与检测间隔（可手动修改）。
- 🧪 **演示模式**：没有可用模型时，勾选即可体验完整监控流程（结果由程序模拟）。
- 📋 **实时状态**：专注/开小差指示、持续时长、连续开小差次数、检测历史、模型耗时。

## 🏗 技术架构

```
┌─────────────────────────────┐
│  前端 (React + TypeScript)   │  ── invoke / event ──►
│  src/                       │                        │
└─────────────────────────────┘                        ▼
                                          ┌──────────────────────────────┐
                                          │  Rust 后端 (Tauri 2)          │
                                          │  · capture.rs   屏幕/摄像头采集 │
                                          │  · model.rs     Ollama/模拟后端│
                                          │  · monitor.rs   检测循环+提醒逻辑│
                                          │  · reminder.rs  通知 + TTS     │
                                          │  · gpu.rs       GPU检测+推荐    │
                                          │  · config.rs    JSON 配置持久化  │
                                          └──────────────┬───────────────┘
                                                         │  HTTP (本机)
                                          ┌──────────────▼───────────────┐
                                          │  Ollama serve (127.0.0.1:11434)│
                                          │  qwen3-vl / llava / …          │
                                          └──────────────────────────────┘
```

监控循环：每个检测间隔 → 采集画面 → 缩放/JPEG → base64 → OpenAI 兼容 `/chat/completions`（多模态，要求输出严格 JSON）→ 解析 `{focused, reason}` → 连续 N 次开小差且过冷却期 → 触发提醒。

## 📁 目录结构

```
focus-guard/
├── src/                    # 前端（React + TS + Vite）
│   ├── App.tsx             # 主界面：三个标签页
│   ├── api.ts              # Tauri invoke / event 封装
│   ├── types.ts            # 前后端共享类型
│   └── components/         # StatusTab / SettingsTab / ModelsTab
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs / lib.rs
│   │   ├── config.rs       # 配置读写（app_config_dir/config.json）
│   │   ├── gpu.rs          # GPU 检测 + 参数推荐
│   │   ├── capture.rs      # xcap(屏幕) + nokhwa(摄像头) + JPEG 编码
│   │   ├── model.rs        # Ollama 客户端 / 模拟后端
│   │   ├── monitor.rs      # 监控循环、提醒判定
│   │   ├── reminder.rs     # 系统通知 + TTS 语音
│   │   └── commands.rs     # Tauri 命令
│   ├── capabilities/       # Tauri 2 权限（通知等）
│   ├── tauri.conf.json
│   └── Info.plist          # macOS 摄像头权限文案
├── scripts/gen-icon.mjs    # 生成应用图标（纯 Node）
└── package.json
```

## 🔧 环境要求

| 组件 | 版本 | 说明 |
|---|---|---|
| Node.js | ≥ 18 | 前端构建 |
| Rust | stable (≥ 1.77) | 后端编译 |
| Tauri CLI | 2.x | `npm i` 自带（`@tauri-apps/cli`） |
| 模型服务 | — | **可选**。任意 OpenAI 兼容服务（本地 Ollama/vLLM/LocalAI 或云端），只需填一个兼容 URL |

### Linux 系统依赖（Ubuntu/Debian）

```bash
sudo apt install -y build-essential curl wget file pkg-config libssl-dev \
  libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev javascriptcoregtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev libxdo-dev \
  libv4l-dev libpipewire-0.3-dev libspa-0.2-dev \
  speech-dispatcher libspeechd-dev
```

- **语音提醒**依赖 `speech-dispatcher` 服务：`sudo systemctl start speechd`（或安装后重启）。
- **屏幕捕获**需要 **X11 会话**（Wayland 暂不支持，启动监控时会给出提示）。

### macOS

- 屏幕捕获：首次启动监控时系统会弹出「屏幕录制」授权，请在 **系统设置 → 隐私与安全性 → 屏幕录制** 中允许本应用。
- 摄像头：`Info.plist` 已声明 `NSCameraUsageDescription`，首次使用会弹出授权。
- 语音：系统自带 `say`，无需额外安装。

### Windows

- 摄像头：在 **设置 → 隐私和安全性 → 相机** 中允许桌面应用访问相机。
- 屏幕捕获：Windows 10 1903+ 通过 Graphics Capture API，无需额外授权。
- 语音：系统 SAPI，无需额外安装。

## 🚀 构建与运行

```bash
# 1. 安装前端依赖
npm install

# 2. 开发模式（热更新）
npm run tauri dev

# 3. 生成安装包（deb/rpm/AppImage/dmg/msi… 取决于当前平台；也可只打一种：
#    npm run tauri build -- --bundles deb）
npm run tauri build
```

### Windows 本地打包与安装

> Windows 安装包只能在 **Windows 环境**里构建（Tauri 打包器依赖 Windows 原生工具链，Linux/WSL2 里无法交叉打包）。推荐二选一：本地构建 或 GitHub Actions 自动构建。

**方式 A：本地构建（一次配置，之后随时打包）**

1. 安装 [Rust](https://rustup.rs)（默认 MSVC 工具链）与 [Visual Studio Build Tools](https://visualstudio.microsoft.com/zh-hans/downloads/)（勾选「使用 C++ 的桌面开发」——Rust 链接需要）；
2. 安装 [Node.js LTS](https://nodejs.org/)；
3. WebView2 运行时：Windows 10/11 已内置，无需安装；
4. 把本项目拷到 Windows（`git clone` 或直接拷贝）；
5. 在项目根目录执行：
   ```powershell
   npm install
   npm run tauri build -- --bundles nsis
   ```
6. 产物在 `src-tauri\target\release\bundle\nsis\FocusGuard_0.1.0_x64-setup.exe`，双击安装即可（也可加 `--bundles msi` 生成 MSI）。

**方式 B：GitHub Actions 自动构建**

项目已带 `.github/workflows/build.yml`。把仓库推到 GitHub 后：

1. 仓库页 → **Actions** → 左侧 **Build installers** → **Run workflow**（手动触发）；
2. 跑完在对应 job 的 **Artifacts** 里下载 `bundle-windows-latest`，解压即得 `FocusGuard_0.1.0_x64-setup.exe`；
3. 以后打 `git tag v0.1.0 && git push --tags` 也会自动构建。

**Windows 上首次运行注意**

- 摄像头：Windows 设置 → 隐私和安全性 → 相机，允许桌面应用访问相机；
- 屏幕捕获：Windows 10 1903+ 无需额外授权；
- 装 Ollama（Windows 版）：[ollama.com/download](https://ollama.com/download)，装好后直接 `ollama pull qwen3-vl:4b`；
- 没装 Ollama 时，先在「设置」里把后端切到**模拟模式**体验完整流程。

### Windows WSL2（Ubuntu）快速跑起来

```bash
# 1. 运行时依赖
sudo apt update && sudo apt install -y libwebkit2gtk-4.1-0 libgtk-3-0 libv4l-0 speech-dispatcher

# 2. 安装应用
sudo dpkg -i FocusGuard_0.1.0_amd64.deb

# 3. 运行（需要 WSLg：Windows 11 自带，或自己配 X server）
focus-guard

# 4.（可选）安装 Ollama 使用真实模型
curl -fsSL https://ollama.com/install.sh | sh
ollama serve
ollama pull qwen3-vl:4b
```

WSL2 注意事项：

- **屏幕捕获**：WSLg 默认提供 X11（XWayland），`DISPLAY=:0` 下可直接截屏；若截图为黑屏，检查 `echo $DISPLAY` 是否有值。
- **摄像头**：WSL2 默认拿不到 Windows 摄像头，需用 `usbipd-win` 把摄像头设备绑定/附加进 WSL；没有摄像头时可只用「屏幕」或「模拟模式」。
- **语音**：Linux 端走 speech-dispatcher，先 `sudo systemctl start speechd`。
- **没装 Ollama / 没 GPU**：在「设置」里把推理后端切到「模拟模式」，即可先体验完整的监控-提醒流程。

## 📖 使用指南

1. **准备模型服务**（可选，或用演示模式跳过）：本地装 [Ollama](https://ollama.com/download) 并拉取模型，例如：
   ```bash
   ollama serve          # 启动本地服务
   ollama pull qwen3-vl:4b
   ```
   或使用任意 OpenAI 兼容云端服务。
2. 打开应用 → **设置**：
   - 勾选采集源（屏幕/摄像头，可都开），分别填写**任务提示词**；
   - 在「模型服务」填写 **服务 URL / API Key / 模型名**（本地 Ollama 填 `http://localhost:11434/v1`，可点「测试连接」验证）；
   - 查看 GPU 推荐（已自动填入模型名与检测间隔，可手动改）；
   - 选择提醒方式（系统通知 / 语音 / 两者），可自定义语音内容。
3. **模型**页：测试连接、选用可用模型、或对当前配置做一次「测试检测」。
4. 回到 **监控状态**，点「开始监控」（配置不完整时按钮会禁用并提示）。

## 🖥 GPU 推荐逻辑

| 显存 | 推荐模型 | 推荐间隔 |
|---|---|---|
| ≥ 8 GB | `qwen3-vl:8b` | 10 s |
| ≥ 4 GB | `qwen3-vl:4b` | 15 s |
| ≥ 2 GB | `moondream` (1.8B) | 20 s |
| 无独显 / 显存未知 | `moondream` | 30 s |

> 均为建议值，可手动修改。推理越快/显存越紧张时，把「图片最大宽度」调小（如 384px）可显著提速。

## 🔒 隐私说明

- 截图与摄像头帧**仅在本地处理**，经 base64 发送给你配置的模型服务（本地服务则不离开本机；云端服务则按你选择的平台策略处理）。
- 检测后图片立即丢弃，不落盘、不记录画面内容，仅保留文字化的检测结果（时间、专注与否、模型原因）。
- 配置与检测历史保存在系统应用配置目录（`app_config_dir/config.json`）。

## ⚠️ 已知限制（原型）

- Linux 屏幕捕获仅支持 X11（Wayland 需走 Portal 方案，暂未实现）。
- 每次检测都会重新打开摄像头，首次帧延迟约数百毫秒；后续可优化为常驻采集线程。
- 本地模型服务首次加载模型到显存需要数秒到数十秒，首个检测结果会偏慢。
- 提醒语音（Linux）依赖 speech-dispatcher 服务运行。
- 模型可能误判：提示词写清楚、画面角度稳定时效果最好；判定倾向于「无法确定=专注」，降低误报。
