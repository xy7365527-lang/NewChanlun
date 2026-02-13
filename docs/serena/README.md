## Serena（MCP）+ Claude Code / Cursor：Windows 配置速记

目标：让 Claude Code 在**不爆 prompt** 的前提下，按需读取/检索仓库里的资料（例如本仓库的 `docs/chanlun/` 与 `缠论知识库.md`）。

---

## Claude Code（推荐）

### 前置依赖（Windows）

本仓库的 `.mcp.json` 使用 `uvx --from git+https://...` 启动 Serena，因此需要：

- `uv`（会自带 `uvx`）
- `git`（用于拉取 `git+https://github.com/oraios/serena`）

可用 winget 安装：

```powershell
winget install --id astral-sh.uv -e
winget install --id Git.Git -e
```

### 方式一：按项目配置（仅当前仓库生效）

在仓库根目录打开 PowerShell，执行：

```powershell
claude mcp add serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context claude-code --project "$($PWD.Path)"
```

### 方式二：全局配置（所有项目通用）

```powershell
claude mcp add --scope user serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context=claude-code --project-from-cwd
```

说明：

- `--context claude-code`：关闭一些在 Claude Code 里重复的工具描述，减少冗余。
- `--project-from-cwd`：Serena 会从当前目录向上寻找 `.serena/project.yml` 或 `.git`，自动把包含它们的目录当作项目根。

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

这表示你项目里的 `C:\Users\hanju\NewChanlun\.mcp.json` 里配置的 Serena 进程**没启动成功**（最常见原因：`uvx` 或 `git` 不在 PATH）。

在 PowerShell 里检查：

```powershell
where uvx
where git
```

- 如果找不到：先按上面的 winget 命令安装依赖，然后**重启 Claude Code**（或至少重开终端让 PATH 生效）。
- 如果你已经在 `/mcp` 里看到 `plugin:serena:serena` 是 `connected`：通常说明你已经有可用的 Serena 插件，此时可以选择**删除/移除**项目里的 `.mcp.json` 这条 `serena` 配置，避免重复与误报。

### Claude Code 提示词（不想记路径时）

把下面这段直接贴进 Claude Code 里再提问：

```text
请在本仓库内检索并按需引用缠论资料：优先使用 docs/chanlun/text/chan99/ 的原文小文件；思维导图仅作索引/补充，若与原文冲突以原文为准。为避免 prompt 过长，只打开最相关的 1-3 个小文件片段，不要整本读取 full.md/PDF。
```

