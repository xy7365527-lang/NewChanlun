## Serena（MCP）+ Claude Code / Cursor：Windows 配置速记

目标：让 Claude Code 在**不爆 prompt** 的前提下，按需读取/检索仓库里的资料（例如本仓库的 `docs/chanlun/` 与 `缠论知识库.md`）。

---

## Claude Code（推荐）

### 前置依赖（Windows）

本仓库通过 **Serena 官方插件** (`serena@claude-plugins-official`) 加载 Serena，无需额外依赖。

> **注意**：本仓库不再使用 `.mcp.json` 配置 Serena。之前同时存在 `.mcp.json` 和插件会导致工具名冲突（API 400 错误）。如果你的环境中仍有 `.mcp.json` 里的 serena 配置，请删除它。

### 插件方式（当前使用，推荐）

Serena 已在 `.claude/settings.json` 的 `enabledPlugins` 中启用：
```json
"serena@claude-plugins-official": true
```
Claude Code 启动时自动加载，无需手动配置。

### 手动配置方式（备选）

如果不使用插件，也可以手动添加：

```powershell
# 方式一：按项目配置
claude mcp add serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context claude-code --project "$($PWD.Path)"

# 方式二：全局配置
claude mcp add --scope user serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context=claude-code --project-from-cwd
```

手动方式需要 `uv`（含 `uvx`）和 `git`。**不要同时启用插件和手动配置，否则会导致工具名冲突。**

---

## 最大化 Token 效率（强烈建议）

Claude Code 支持“按需加载工具描述”（on-demand tool loading）。开启后启动时**不会把所有工具说明一次性塞进上下文**，能显著减少 token。

在 PowerShell 中（对当前会话生效）：

```powershell
$env:ENABLE_TOOL_SEARCH = "true"
claude
```

Windows CMD（单行）：

```bat
set ENABLE_TOOL_SEARCH=true && claude
```

---

## VSCode / Cursor（工作区 MCP 配置）

可在仓库内创建/编辑：`<repo>/.vscode/mcp.json`（示例来自 Serena 文档；Cursor 通常也能复用 VSCode 的工作区配置）。

```json
{
  "servers": {
    "oraios/serena": {
      "type": "stdio",
      "command": "uvx",
      "args": [
        "--from",
        "git+https://github.com/oraios/serena",
        "serena",
        "start-mcp-server",
        "--context",
        "ide",
        "--project",
        "${workspaceFolder}"
      ]
    }
  },
  "inputs": []
}
```

---

## 在本仓库里的推荐用法（避免 prompt 过长）

- **权威性**：两份思维导图是总结/讲解材料；如与原文出入，**以《股市技术理论》拆分原文 `docs/chanlun/text/chan99/` 为准**。
- 日常问答：优先引用 `缠论知识库.md`。
- 查《股市技术理论》原文：优先引用拆分后的 `docs/chanlun/text/chan99/`（入口 `docs/chanlun/text/chan99/INDEX.md`）。
- 查两份思维导图：优先引用 `docs/chanlun/text/mindmaps/`（入口 `docs/chanlun/text/mindmaps/INDEX.md`）。

---

## 常见问题

### `/mcp` 里 `serena` 显示 `failed`

如果使用插件方式，检查 `/mcp` 中 `plugin:serena:serena` 的状态。如果显示 `connected` 则正常工作。

如果使用手动配置方式，常见原因是 `uvx` 或 `git` 不在 PATH：

```powershell
where uvx
where git
```

如果找不到：安装依赖后**重启 Claude Code**。

### 工具名冲突（API 400 错误）

如果同时存在 `.mcp.json` 中的 serena 配置和 `serena@claude-plugins-official` 插件，会导致工具名重复注册，API 返回 400 错误。解决方法：只保留一种配置方式，删除另一种。

### Claude Code 提示词（不想记路径时）

把下面这段直接贴进 Claude Code 里再提问：

```text
请在本仓库内检索并按需引用缠论资料：优先使用 docs/chanlun/text/chan99/ 的原文小文件；思维导图仅作索引/补充，若与原文冲突以原文为准。为避免 prompt 过长，只打开最相关的 1-3 个小文件片段，不要整本读取 full.md/PDF。
```

