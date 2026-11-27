# Git Commit Template for OpenIDEA

面向 **OpenIDEA（v0.1.53+ / Plugin API `^1.3`）** 的 Git 提交信息模板插件。

挂载在 Git 提交（Commit）面板的提交信息输入框上方工具栏中，点击即可打开可视化表单，快捷生成并一键填充符合 Conventional Commits 规范的提交信息。

## 功能特性

- **Commit 工具栏快捷挂载**：通过 `commit.message` 扩展点直接显示在 Commit 工具窗口 / Commit 对话框输入框上方工具栏，并支持快捷键 `Ctrl+Alt+Shift+C`（macOS：`Cmd+Alt+Shift+C`）。
- **完整中文可视化表单**（自动适配 OpenIDEA 深色 / 浅色主题）：
  - **提交类型**：`feat` / `fix` / `docs` / `style` / `refactor` / `perf` / `test` / `build` / `ci` / `chore` / `revert`
  - **变更范围**：支持自由输入 + 下拉选择，自动记忆最近 20 条常用范围（支持单条删除）
  - **简短说明**：实时统计标题字符数，支持 `Ctrl+Enter` / `Cmd+Enter` 快速填入
  - **详细说明**：支持多行描述
  - **超过 72 字符换行**：默认开启，兼容中英文自动换行
  - **重大变化**：自动添加 `BREAKING CHANGE:` 前缀
  - **关闭 issues**：自动添加 `Closes` 前缀（如输入 `123, 456` 自动生成 `Closes #123, #456`）
  - **跳过 CI 流程**：勾选后自动附加 `[skip ci]`
- **双向同步**：
  - 点击工具栏图标时，自动读取当前 Commit 输入框中的已有内容并解析回填表单，方便二次修改。
  - 点击 **好的** 后直接写入 Commit 输入框。

## 安装方法（OpenIDEA 0.1.53）

### 方式一：直接从本地目录安装（最简单）

1. 将本项目目录（需已包含编译好的 `dist/plugin.wasm`）复制到你的电脑。
2. 打开 OpenIDEA，进入 **Settings › Plugins**。
3. 点击右上角齿轮图标，选择 **Install Plugin from Disk…**，选择本项目文件夹（或打包好的 `.oiplugin` 文件）。
4. 确认授予插件所需权限（`vcs.read`、`vcs.write`、`storage`），重启或重载后即可在 Git 提交面板工具栏看到图标。

### 方式二：开发模式链接（`openidea-plugin link`）

```bash
openidea-plugin build /path/to/openidea-git-commit-template
openidea-plugin link /path/to/openidea-git-commit-template
```

修改代码后重新执行 `openidea-plugin build`，在 OpenIDEA 的 **Settings › Plugins** 中点击 **Reload** 即可生效。

## 从源码构建与测试

```bash
# 运行单元测试（基于官方 MockHost）
cargo test

# 编译 WASM 组件并执行官方清单与链接校验
openidea-plugin build
```
