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

## 在 Studio 里继续、恢复与中断一个已登记的 v2 run

服务启动时用 `PILOT1742_STUDIO_RUNS=<run_id>[,<run_id>]` 列出要绑定的 run，另设 `PILOT1742_STATE_ROOT` 与 `PILOT1742_APP_PRIVATE_KEY`（私钥路径，只由已审 App 客户端读取）。
每个绑定 run 对应一个确定的线程 `pilot1742.studio_binding.thread_for(<run_id>)`。`studio.sh` 在服务就绪后为每个绑定 run 建好这个线程（带 `metadata.graph_id=pilot1742`，`if_exists=do_nothing`），已有线程原样保留；已有线程缺 `graph_id` 时日志会报出来，状态读不到，换一个干净的 `STUDIO_STATE` 目录重启即可。在 Studio 线程列表里打开这个线程，看到的就是该 run 自己 `graph.sqlite` 里的 checkpoint，与 CLI 是同一条。

- 恢复。run 停在中断处（签批等待或操作员中断）时，点中断下方的 Resume。恢复载荷的内容从不当作批准，批准只看 root 保护的签批权威链。
- 继续。run 停在待执行的任务前（例如控制进程被杀后）时，点线程运行记录中的 **Continue**。只执行已有的待办任务，不开新一轮。不要点输入面板的 **Submit**。当 As Node 选为 `tools` 时，Submit 先请求改写线程状态，本接入返回 403。
- 中断。run 进行中点取消（cancel），改为写入该 run 的中断请求，图在下一次工具调用前停在 checkpoint，已派出的工蜂不被杀。下一次恢复时清除该请求。
- 已结束的 run 只读。Continue、Submit 与 Resume 都拒绝。首跑由执行器 CLI 发起，Studio 不创造任务、不开新一轮。
- 不分叉。Studio 请求里带的 `checkpoint`、`thread_ts`、`checkpoint_id` 必须就是线程当前的最新 checkpoint，入口核对后剥掉，执行时在 run 锁内再核一次，不一致即拒绝；选旧 checkpoint 运行一律拒绝。
- 忙时拒绝。Studio 发的 `multitask_strategy=rollback` 按 `reject` 执行：线程已有 run 在飞时新请求冲突拒绝，不取消已有任务、不回滚状态。CLI 持锁时 Studio 的 run 失败，Studio 持锁时 CLI 退出 75。

仍然拒绝（403）：未绑定的线程、新建或复制任意线程、改写线程状态、带新内容的输入、`checkpoint_during=false` 或 `durability=exit`、无状态 run、store 与 assistant 写入。建线程只接受 `thread_id`、`metadata`、`if_exists`；run 请求按字段白名单放行，并由入口归一化后才交给服务端。
checkpoint 写入只认持有该 run 锁的那次官方执行：写权随执行上下文签出，同进程里并发的其他请求拿不到，锁释放前作废。

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
