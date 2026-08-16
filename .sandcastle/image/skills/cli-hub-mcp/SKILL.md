---
name: cli-hub-mcp
description: CLI-Hub（CLI-Anything 注册表）MCP 工具——按关键词搜索/安装/启动 agent-native CLI。触发：需要操作外部软件（图像/视频/3D/办公应用等）、查找可用 CLI、安装或运行 CLI-Hub 里的工具。
---

# cli-hub-mcp — CLI-Hub registry MCP（沙盒镜像版，#1010）

stdio 本地 server：`~/.agents/mcp/cli-hub-mcp/.venv/bin/python ~/.agents/mcp/cli-hub-mcp/server.py`
（server.py 薄包装 `cli_hub` 包 = 公共 PyPI 的 `cli-anything-hub`）。

## 内核调用

```python
import cli_hub_mcp
tools = await cli_hub_mcp.list_tools()        # 发现工具
r = await cli_hub_mcp.search(query="image")   # 搜索 CLI
r = await cli_hub_mcp.list_clis()             # 全量列表
```

工具集（以 `list_tools()` 为准）：search / list_clis / info / installed /
install / uninstall / update / launch。注意：install/launch 需要目标 GUI
应用或上游 CLI 已存在——沙盒容器里多数外部应用不存在，这两个工具可能
返回错误属预期。
