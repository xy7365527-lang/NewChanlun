---
name: serena
description: Serena 语义代码工具（LSP 支撑的符号检索/引用追踪/模式搜索/代码编辑）。触发：找符号定义与引用、按名称/正则找代码、查类与函数的文档与签名、改符号实现。首次调用会自动拉起本地 Serena MCP daemon（127.0.0.1:8017，LSP 冷启动约 30-60 秒），之后复用。
---

# Serena — 语义代码工具（MCP）

本地 Serena MCP daemon（streamable-http，127.0.0.1:8017，端口可用 `SERENA_MCP_PORT` 覆盖）。
daemon 在首次调用时自动拉起（从 cwd 向上找 `.serena/project.yml`），日志 `~/.serena/mcp-daemon.log`，进程随 OS 常驻——重启机器后下次调用会自动再拉起。

> 沙盒镜像版（#1010）：`.serena/` 在 NewChanlun 被 gitignore，worktree 里没有 project.yml；本版 skill 首次调用时自动在 git root 生成最小 project.yml（rust+python）再拉起 daemon。

## 内核调用

```python
import serena
tools = await serena.list_tools()          # 发现工具（含 inputSchema）
help(serena.find_symbol)                    # 每个工具的签名/文档
r = await serena.find_symbol(name_path="ElementId", relative_path="rust/src")
```

工具名与参数以 `list_tools()` 为准。常用：
- `find_symbol` — 按名/路径找符号（LSP 索引，比 grep 精确）
- `find_referencing_symbols` — 找引用者（含定义跳转）
- `get_symbol_details` / `get_symbols_overview` — 签名、文档、结构
- `search_for_pattern` — 正则模式搜索
- `replace_symbol_body` / 编辑类工具 — 改代码（daemon 在项目根运行）

## 注意

- daemon 一次只服务一个项目（启动时激活）；换项目先 `pkill -f "serena start-mcp-server"` 再调用。
- Rust 项目依赖 rust-analyzer（本机已装 /opt/homebrew/bin/rust-analyzer）。
- 索引：`serena project index <repo>` 预建 LSP 缓存，查询更快。
