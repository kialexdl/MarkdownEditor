# MarkdownEditor

面向本地项目与 Git 工作区的轻量级 Markdown 桌面编辑器。Windows 11 为首发平台，Git for Windows 是必装依赖。

## 已实现能力

- 多工作区注册、切换和移除；每个新工作区对应一个文件夹，并在主窗口中切换。
- 目录树懒加载与常见构建目录过滤。
- Markdown 编辑、渲染预览和源码/渲染分屏；可持久化设置新文件的默认打开模式。
- 标签页支持一键关闭当前工作区全部文件，未保存内容会统一确认。
- CodeMirror 6 搜索、撤销、行号、Markdown 高亮与只读查看。
- markdown-it + Mermaid 按需渲染（使用可安全清洗的 SVG 文字标签，并随明暗主题重渲染）、相对链接、本地图片和标题锚点；预览右侧提供可隐藏的标题目录。
- 自动保存、`Ctrl+S`、全部保存、原子替换、外部修改冲突和崩溃恢复。
- 非 Markdown 文件后端强制只读；文本、图片内置查看，PDF/其他文件交给系统查看。
- 本机 Git 必装检测、分支/状态、单文件提交历史、重命名跟踪和历史版本 Diff。
- WebView 默认右键菜单只在编辑器和输入控件中开放。
- Windows Release EXE 便携 ZIP 一键打包脚本和 tag 自动发布工作流。

## 技术栈

- Tauri 2 / Rust
- Svelte 5 / TypeScript / Vite
- CodeMirror 6
- markdown-it / DOMPurify / Mermaid

## Windows 11 环境准备

1. 安装 [Git for Windows](https://gitforwindows.org/)，并确认 `git --version` 可用。
2. 安装 Node.js 20.19 或更高版本，启用 Corepack，并安装项目指定的 pnpm。
3. 通过 rustup 安装 stable Rust，选择 MSVC 工具链。
4. 安装 Visual Studio 2022 Build Tools，包含“使用 C++ 的桌面开发”和 Windows 11 SDK。
5. 保持 Microsoft Edge WebView2 Runtime 可用；Windows 11 通常已经安装。

首次准备：

```powershell
corepack enable
.\scripts\bootstrap.ps1
```

当前交付环境无法访问 npm/crates.io，因此源码包不包含机器生成的 `pnpm-lock.yaml` 和 `Cargo.lock`。首次运行 bootstrap 会生成前者，首次 Cargo 构建会生成后者；生成后应立即提交两个锁文件，此后的脚本会自动切换为 frozen install。

开发启动：

```powershell
pnpm tauri dev
```

## 检查与测试

```powershell
pnpm check
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Windows 一键打包

```powershell
.\scripts\package.ps1
```

脚本会检查环境、锁定安装依赖、执行检查和测试，再通过 `tauri build --no-bundle` 生成 Release 应用。最终在 `artifacts/<version>/windows-x64-portable/` 写入一个便携 ZIP 和 `SHA256SUMS.txt`；ZIP 根目录只包含 `MarkdownEditor.exe`，解压后直接运行，不再生成或下载 NSIS/MSI。

如只想验证打包流程而暂时跳过测试：

```powershell
.\scripts\package.ps1 -SkipTests
```

打包脚本与 CI workflow 都保存在仓库中。推送 `v*` tag、手动运行工作流，或向 `main` 推送提交消息包含 `[release]` 的提交，会在 Windows Runner 上完成检查、测试和便携 ZIP 构建后创建正式 Release。tag 必须与 `package.json` 版本一致；已存在的 Release 不会被自动覆盖。普通提交和 PR 只检查、构建并保留 Actions 产物。

发布下载：[GitHub Releases](https://github.com/kialexdl/MarkdownEditor/releases)。Actions 产物同时保留本次构建生成的依赖锁文件。

便携版仍依赖系统 WebView2 Runtime，Windows 11 通常已内置；Git 历史功能仍要求本机预先安装 Git for Windows。

## 安全边界

- Rust 核心会规范化真实路径，并拒绝工作区根以外的文件访问。
- 只有 Markdown 扩展名可写，非 Markdown 写请求即使绕过 UI 也会被拒绝。
- Git 通过参数数组执行固定只读命令，不调用 Shell、不执行网络操作。
- Markdown 内嵌 HTML 默认关闭；预览经过 DOMPurify，Mermaid 使用严格安全级别。
- `javascript:` 等危险链接被阻止；外部图片默认不加载。
- 文件树、标签栏、工具栏和预览区禁用 WebView 默认右键菜单；CodeMirror 编辑/只读文本区和表单控件保留系统菜单。

## 当前产品边界

不包含所见即所得、Git 写操作、云同步、多人协作、插件市场、内置终端或 AI Agent。PDF 的内置渲染取决于系统能力，无法内置时使用系统默认应用。

详细架构和验收标准参见 [MarkdownEditor-设计方案-v1.md](./MarkdownEditor-设计方案-v1.md)。
