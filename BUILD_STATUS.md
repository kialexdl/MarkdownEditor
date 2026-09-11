# MarkdownEditor v0.1.0 交付状态

日期：2026-08-24

## 本次已完成

- Tauri 2 + Rust + Svelte 5 + TypeScript 项目结构。
- 多工作区注册、切换、移除和目录树懒加载；已移除添加根与新窗口入口及其后端命令。
- Markdown 编辑、预览、可拖动分屏与块级双向滚动同步；新文件默认模式可持久化设置。
- 当前工作区标签页一键全部关闭，以及编辑器/输入控件白名单式右键限制。
- markdown-it 渲染 Worker、DOMPurify、Mermaid 动态加载和缓存。
- Mermaid 固定使用纯 SVG 文字标签，修复二次安全清洗后图形存在但节点文字消失的问题。
- Markdown 代码围栏语言标识使用专用高优先级样式，修复暗色模式下 `mermaid` 字段难以辨认的问题。
- Mermaid 缓存键包含当前主题，切换明暗主题会自动重渲染已显示的图表。
- 相对 Markdown 链接、本地图片、标题锚点、可隐藏的右侧文档目录和受控外部链接。
- Markdown 后端写权限校验、原子保存、自动/手工保存、磁盘指纹冲突与恢复快照。
- 非 Markdown 文本/图片只读查看；PDF/未知文件使用系统应用。
- Git 必装检测、分支与状态、单文件分页历史、重命名追踪、历史内容和 CodeMirror Diff。
- Windows 11 bootstrap、Release EXE 便携 ZIP 打包、ZIP 结构/校验和脚本及 tag CI workflow。
- Rust 单元测试规格：Markdown 类型识别、Git 重命名历史解析和危险 revision/path 拒绝。

## 本环境实际完成的验证

- `node scripts/validate-source.mjs`：通过，核对全部前端 IPC 调用均已注册 Rust handler。
- JSON 配置语法：通过。
- Bash 脚本语法：通过。
- Windows PowerShell 脚本已改为纯 ASCII，并对原生命令增加非零退出码检查。
- Windows ICO 已核对 16/24/32/48/64/256 像素图层，32 像素图层位于首位。
- Tauri installer bundling 已禁用；打包固定使用 `tauri build --no-bundle`，避免 NSIS/WiX 下载并仅压缩 Release EXE。
- 已根据 Windows 首轮 `svelte-check` 结果修复 6 个 TypeScript 错误与 2 个无障碍警告。
- Windows 后续实测已达到 `svelte-check` 0 errors；剩余两个分隔条警告已改用原生按钮处理。
- 已应用 Rust 1.96 `cargo fmt --check` 输出的全部格式差异。
- 已修复 Rust 1.96 Clippy 报告的两处 `manual_pattern_char_comparison`。
- Svelte 模板控制块数量检查：通过。
- Git 版本检测：本环境为 Git 2.51.1。

## 尚未在本环境执行的验证

当前执行环境没有 Rust 工具链，且网络策略阻止访问 npm/crates.io，因此无法安装依赖，也无法实际运行：

- `pnpm check`
- `pnpm build`
- `cargo fmt` / `cargo clippy`
- `cargo test`
- `pnpm tauri build`

这不代表这些检查已经通过。请在 Windows 11 开发机执行：

```powershell
.\scripts\bootstrap.ps1
pnpm check
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
.\scripts\package.ps1
```

首次依赖安装会生成 `pnpm-lock.yaml` 与 `Cargo.lock`，应将两者提交后再建立正式版本 tag。
