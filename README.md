# Git Commit Template for OpenIDEA

面向 **OpenIDEA（v0.1.53+ / Plugin API `^1.16`）** 的 Git 提交信息模板插件。

挂载在 Git 提交（Commit）面板输入框上方的工具栏中，点击后直接在屏幕中央主区域弹出**完整卡片式可视化大表单**，点击 **【好的】**、**【取消】** 或按 `Esc` 即可自动关闭表单页，并内置 **Conventional Commits 提交前规范检查**。

## 功能特性

- **居中卡片式完整大表单 + 自动关闭**：
  - 点击 Commit 工具栏图标，或按下快捷键 `Ctrl+Alt+Shift+C`（macOS：`Cmd+Alt+Shift+C`）即可在主区域居中打开完整表单。
  - **【好的】 / `Ctrl+Enter`**：校验并一键填入 Git 提交消息输入框，随后**自动关闭表单页**。
  - **【取消】 / 右上角 `×` / `Esc`**：不修改提交框内容，**自动关闭表单页**。
- **经典完整中文表单**（自动适配 OpenIDEA 深色 / 浅色主题）：
  - **提交类型**：11 项经典单选列表（`feat` / `fix` / `docs` / `style` / `refactor` / `perf` / `test` / `build` / `ci` / `chore` / `revert`）
  - **变更范围**：自由输入 + 下拉选择，自动记忆最近 20 条常用范围（支持单条删除）
  - **简短说明**：必填校验、实时统计标题字符数与超长提醒
  - **详细说明**：支持多行描述，默认开启 72 字符自动换行（兼容中英文）
  - **重大变化**：自动添加 `BREAKING CHANGE:` 前缀，支持勾选标题 `!` 标记（如 `feat!:`）
  - **关闭 issues**：支持 `Closes` / `Fixes` / `Resolves` 关键字选择（如输入 `123, 456` 自动生成 `Closes #123, #456`）
  - **跳过 CI 流程**：勾选后自动附加 `[skip ci]`
  - **双向同步**：打开表单时自动读取并解析当前 Commit 输入框已有内容，预览统一由 Rust 后端实时生成。
- **Conventional Commits 提交前规范检查（`commitChecks`）**：
  - 可在 Commit 选项（Before Commit）中启用 **Conventional Commits 规范检查**，自动拦截空提交或不规范标题，放行 `Merge` / `Revert` / `fixup!` / `squash!` 等自动生成消息。

## 安装方法（OpenIDEA 0.1.53+）

1. 从 GitHub Actions 构建产物（Artifacts）下载打包好的 `.oiplugin` 文件。
2. 打开 OpenIDEA，进入 **Settings → Plugins**。
3. 点击右上角齿轮图标，选择 **Install Plugin from Disk…**，选中 `.oiplugin` 文件（若之前已安装旧版本，重装后确认授予新增的 `ide.commands` 权限用于自动关闭表单标签页）。
4. 重载后即可在 Git 提交面板工具栏点击图标使用。

## 从源码构建与测试

```bash
# 运行单元测试（基于官方 MockHost）
cargo test

# 编译 WASM 组件并执行官方清单与链接校验
openidea-plugin build .
```
