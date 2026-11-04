# #1742：正式独审 attempt 的启动前预检

每次启动正式 attempt（`ExternalReview.review_pr`）之前运行一次。预检只回答“现在的环境能不能让两位 reviewer 跑完”，**不是审阅结果**：报告固定写 `"is_review_result": false`，不写 App 状态、不进 `external-review/runs`、不占 attempt 号。预检拦下时修环境，修好后再预检，通过了才启动新 attempt。

## 用法

在要启动正式 attempt 的 controller checkout 根目录，用启动 attempt 的同一宿主账户运行（Python ≥ 3.11，只用标准库）：

```sh
python3 -I .sandcastle/deepagents-pilot/review_preflight.py \
  --source . \
  --base <base_sha> --head <head_sha> --merge-base <merge_base_sha> \
  --checkout-root /Users/silencehan/.sandcastle-host/external-review-checkouts
```

- `--checkout-root` 填正式 attempt 将传给 `ExternalReview(checkout_root=...)` 的同一路径。
- 三个 SHA 与 attempt 的 target 相同；`--source` 是 controller 读候选对象的仓库。
- 其余参数有默认值：`--image sandcastle:newchanlun`、`--proxy-config ~/Library/Application Support/CLIProxyLocal/config.yaml`（只读）、`--proxy-url http://127.0.0.1:8318`、`--proxy-log-dir ~/Library/Application Support/CLIProxyLocal/auth/logs`、`--codex-home $CODEX_HOME 或 ~/.codex`、`--deadline 55`。

退出码：`0` 可以启动；`2` 拦截（看 `checks` 里 `ok:false` 的项）；`3` 用法错误。标准输出是一行 JSON 报告；并发执行的检查（空间、挂载、两家探测）带 `elapsed_s`，两项凭据检查在它们之前串行完成。

## 检查什么

| 检查 | 拦下的历史故障 | 做法 |
|---|---|---|
| `credential:review-worker` | 正式 attempt 的 worker 缺 key（只认进程环境与 launchctl，见 `codex-child.ts` 的 `providerEnvironment`） | 先读进程环境，再读 `launchctl getenv CLIPROXY_LOCAL_KEY`。缺失即拦，且不发任何探测；两家往返探测都用这把 key |
| `credential:codex-host` | #1323：宿主缺 `CLIPROXY_LOCAL_KEY`，回合在调模型前报 `Missing environment variable` | 去掉该环境变量后，按 `$CODEX_HOME/config.toml` 的 `model_providers.cliproxy-local` 取 key（`auth.command` / `env_key`），`auth` 与 `env_key` 并存也拦（Codex 拒绝启动）。宿主 `base_url` 必须就是预检代理（`localhost`/`127.0.0.1` 视为同一端点）。key 与 worker 相同时由两家探测一并证明；不同时 Sol 那一次探测改用宿主 key，两把 key 各由一次真实往返证明。宿主凭据失败不阻止探测 |
| `checkout_root_free_space` | PR1866 A1/A2、PR1868 A3（f9）：快照需 2.60GB，HOME 只剩 1.40–1.65GB | 逐字节复刻 `prepare_review` 的快照字节公式，要求 checkout_root 所在卷剩余 ≥ 2 份（两位 reviewer；colima 可能仍占着已删快照的 inode） |
| `checkout_root_container_mount` | PR1868 A1：checkout 在未共享进 colima 的路径，容器里 `/review` 是 VM 本地空目录 | 在 checkout_root 写一次性 nonce，用审阅镜像 `--network none --read-only` 只读 bind 挂载 checkout_root 读回；读不到或不一致即拦。探针目录随即删除 |
| `route:<model>` | PR1883 A1：claude-opus-5-5 新会话被选到 Antigravity | 按本次唯一 `Session-Id` 在 CLIProxy request-log 里读每次上游尝试的 `Auth: provider` 与上游主机；要求 Sol=`codex@chatgpt.com`，Opus=`claude@api.anthropic.com`。日志缺失或行数不齐也拦；CLIProxy 配置 `observability.logs.request-log` 为 false 时无法核路由，立即拦 |
| `identity:<model>` | 留出变体：proxy 给每个 Claude OAuth 请求注入 `fallbacks:[claude-opus-4-8]`，上游可能静默改由 4.8 服务 | 从同一份 request-log 的上游响应读实际服务模型（`message_start.message.model` / `response.model`），与请求不一致即拦；日志里没有响应体时记 `verified:false` 不拦 |
| `roundtrip:<model>` | 同上，上游 400 `tool_choice ... not supported` | 每家一次最小 `/v1/responses` 往返，带 tools、`tool_choice=auto`、JSON 输出格式、新会话，必须读到 `response.completed`/`incomplete` |

模型探测每家只发一次、提示极短（effort=low），这是预检唯一的模型花费。凭据检查先串行完成，其余检查随后并发；所有阶段共用同一个总时限（默认 55 秒，其中 min(3 秒, 总时限/2) 留给回收挂载探针），超时的检查判失败。

不要把 effort 降到 `minimal`：2026-11-05 实测 Sol 在 `minimal` 下返回 400（codex 后端注入的 `image_gen` 工具不能与 `minimal` 同用），预检会因此误报。真实环境单次墙钟约 3–5 秒，下限是 Sol 一次完整往返；路由要读 request-log，而 proxy 在响应完成后才写日志，所以不能提前截断流。实测日志在响应结束后 14–85ms 落盘；预检在往返成功后最多等 2 秒找日志，往返已失败时只等 0.3 秒（日志仅作诊断）。

## 局限

- 预检是某一时刻的快照。CLIProxy 是 fill-first + 会话亲和，正式 attempt 的新会话仍可能在几秒后被选到别的凭据；路由的根治在共享 proxy 配置（当前已用 `oauth-excluded-models.antigravity` 排除两个 Claude 模型），预检只负责发现回归。
- JSON 围栏类失败需要真实模型完整输出才能复现，预检不覆盖。
- 双快照阈值约 5.20GB（随 PR 变化）。空间不足时报告会附 `vm_deleted_open_bytes`：colima/lima 的 VM 进程仍打开着的已删除文件字节数（2026-11-05 实测 142.67GB，是 HOME 耗尽的主因）。这部分要先停掉 VM 里的容器再重启 colima 才能回收；colima 是共用的，重启前确认其他线程没有在跑的容器。

## 测量

拦截率与误报按冻结语料在隔离环境注入同样故障测得：小容量卷模拟 HOME、VM 内同名空目录、本地假 proxy 返回同样的 400、修前形态的 Codex provider 配置。不碰共享 proxy 与真实账号。结果见 PR 描述。
