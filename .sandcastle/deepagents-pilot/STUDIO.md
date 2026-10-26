# 在官方 Studio 查看试点执行图

启动本地只读服务后，打开 [LangSmith Studio](https://smith.langchain.com/studio/?baseUrl=http://127.0.0.1:2024)，选择 `pilot1742` 查看图。
网站登录由你本人完成。本地服务不需要 LangSmith API key。

## 启动服务

```sh
sh /Users/silencehan/.codex/worktrees/deepagents-pilot-1742/NewChanlun/.sandcastle/deepagents-pilot/studio.sh
```

终端显示 API 为 `http://127.0.0.1:2024` 后，打开上面的 Studio 链接。
保持该终端运行。按 `Ctrl-C` 停止服务。
服务只监听本机回环地址，不建立公网隧道。

首次在独立环境安装附加依赖时运行以下命令。既有试点依赖受 `requirements.lock` 约束。

```sh
uv pip install --python /Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv/bin/python \
  -r /Users/silencehan/.codex/worktrees/deepagents-pilot-1742/NewChanlun/.sandcastle/deepagents-pilot/requirements-studio.txt
```

## 查看什么

Studio 展示 `build_agent` 为 `real4` 冻结任务构建的真实 Deep Agent 图，包括 `model`、`tools` 和技能中间件。
设计、裁判、作者和独审是模型通过 `run_worker` 调用的工具职责，不是此图上五个独立节点。
此入口不加载或导入 `real4` 的 SQLite checkpoint，因此不会显示历史 `real4` 对话或运行轨迹。
历史原始记录仍在试点证据目录的 `runs/real4/`，查看运行结果继续使用 README 中的 `status` 命令。

## 保持只读

此入口用于看图和 schema。执行、恢复、状态更新和批准登记继续走现有 CLI。
Studio 的提交、运行、修改 assistant 等操作会收到 `403`，不会调用模型、工蜂或受管提交。
官方 `disable_runs` 关闭运行接口和 v2 事件执行通道，`disable_threads` 与 `disable_store` 关闭状态及存储接口。
只读 HTTP 中间件另外拦截官方仍开放的 assistant 创建、修改、删除和版本切换。
查询用的 `POST /assistants/search`、`count`、`versions` 保留。

启动脚本关闭 LangSmith tracing、OpenTelemetry 导出和 CLI analytics，并移除进程里的 LangSmith key。
Agent Server 的开发状态单独写入 `/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/studio/server-state/`。
它不打开 `runs/real4/graph.sqlite`，也不认领原 run 的执行锁。

## 官方依据

- [Studio 快速开始](https://docs.langchain.com/langsmith/quick-start-studio)。本地服务与网页直连，支持关闭 tracing。
- [LangGraph CLI](https://docs.langchain.com/langsmith/cli)。`langgraph dev`、图 factory 与 HTTP 配置。
- [Studio 概览](https://docs.langchain.com/langsmith/studio)。图模式与交互能力。

## 浏览器连接提示

Studio 顶部的 `Missing LANGSMITH_API_KEY` 提示针对云端追踪。只读本地图不需要添加该 key。
本地服务已关闭追踪上传，保留该状态即可看图。
如果浏览器显示 `Failed to fetch`，先确认启动终端还在运行，再刷新 Studio。
本接入在外层使用 Starlette 的 `CORSMiddleware`，只允许官方 Studio origin 的本机私网预检。
`langgraph-api 0.16.7` 的 CORS schema 尚未接收 `allow_private_network`，仅写入 JSON 会被过滤。
附加依赖固定 `starlette 1.10.0`，满足 API 的 `starlette>=1.3.1` 要求。没有修改第三方包代码。
图形界面已由主控在登录后的 Codex 内置浏览器验收，图加载和节点渲染通过。
