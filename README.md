# Git Commit Template for OpenIDEA

面向 **OpenIDEA（v0.1.53+ / Plugin API `^1.3`）** 的 Git 提交信息模板插件。

提供**原生弹窗快速向导**（确认或按 `Esc` 自动关闭）与**侧边高级编辑面板**双模式，并内置 **Conventional Commits 提交前规范检查**，帮助快速生成并校验规范的提交信息。

## 功能特性

- **原生弹窗快速向导（主入口，自动关闭）**：
  - 点击 Commit 面板输入框上方的工具栏图标，或按下快捷键 `Ctrl+Alt+Shift+C`（macOS：`Cmd+Alt+Shift+C`）即可唤起弹窗流程。
  - **最快路径仅需四步回车**：选择类型（自动置顶当前/上次类型）→ 选择或跳过 Scope → 输入简短说明 → 回车直接填入提交框并自动关闭弹窗。
  - 确认菜单中可按需补充 **详细说明（Body）**、**关联 Issues**、**重大变化（`BREAKING CHANGE`）**、**标题 `!` 破坏性标记**、**`[skip ci]`**，或一键把已填内容带入侧边高级编辑面板继续编辑。
  - 任意一步按 `Esc` 即可取消关闭，不改动原有提交框内容。
- **侧边高级编辑面板**：
  - 可从菜单 **Tools → 打开提交消息高级编辑面板**、右侧工具窗口栏或弹窗向导中进入。
  - 紧凑式提交类型标签组（`feat` / `fix` / `docs` / `style` / `refactor` / `perf` / `test` / `build` / `ci` / `chore` / `revert`）。
  - 常用变更范围自动记忆（最多保留最近 20 条，支持单条删除）。
  - 支持标题 `!` 标记、`Closes` / `Fixes` / `Resolves` 关键字切换、正文 72 字符自动换行（兼容中英文）与标题超长提醒。
  - 预览结果统一由 Rust 后端实时格式化输出，支持双向回填解析。
- **Conventional Commits 提交前规范检查（`commitChecks`）**：
  - 在 Commit 选项（Before Commit）中可启用 **Conventional Commits 规范检查**，自动拦截空提交或不符合规范的标题，并自动放行 `Merge` / `Revert` / `fixup!` / `squash!` 等 Git 生成消息。

## 安装方法（OpenIDEA 0.1.53+）

### 方式一：直接安装 `.oiplugin` 插件包（推荐）

1. 从 GitHub Actions 构建产物（Artifacts）下载打包好的 `.oiplugin` 文件。
2. 打开 OpenIDEA，进入 **Settings → Plugins**。
3. 点击右上角齿轮图标，选择 **Install Plugin from Disk…**，选中 `.oiplugin` 文件。
4. 确认授予所需权限（`vcs.read`、`vcs.write`、`storage`），重载后即可使用。

### 方式二：开发模式链接（`openidea-plugin link`）

```bash
openidea-plugin build /path/to/openidea-git-commit-template
openidea-plugin link /path/to/openidea-git-commit-template
```

修改代码后重新执行 `openidea-plugin build`，在 OpenIDEA 的 **Settings → Plugins** 中点击 **Reload** 即可生效。

## 从源码构建与测试

```bash
# 运行单元测试（基于官方 MockHost）
cargo test

# 编译 WASM 组件并执行官方清单与链接校验
openidea-plugin build .
```
