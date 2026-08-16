# .sandcastle/SECURITY.md —— 凭据与安全面说明

本文件说明 NewChanlun sandcastle 管线的凭据注入面与已知安全边界。依据：#996（脚手架）、#999（官方用法调研，报告 `chanlun/review-results/issue999-sandcastle-usage.md` §5）、#1004（kernel 预烤）、#1005（GH_TOKEN）、#1006/#1010（凭据搬迁与镜像扩容）。

## 1. `.env` 注入面

### 哪些 key

`.sandcastle/.env` 是全部运行时凭据的唯一入口，键面（按 `prebake-kernel.sh` 的 `--change` 清单，共 9 键）：

| 键 | 用途 | 来源 |
|---|---|---|
| `KIMI_API_KEY` | 主力 provider kimi-coding（工蜂默认模型 k3） | 宿主 `~/.prime/agent/auth.json` 静默迁移（#1006/#1010） |
| `DEEPSEEK_API_KEY` | 升档 provider deepseek | 同上 |
| `ZAI_API_KEY` | 升档 provider glm(zai) | 同上 |
| `MOONSHOT_API_KEY` | 升档 provider moonshot（**注意**：key 宿主侧已失效待刷新，#1010 卡点登记） | 同上 |
| `GH_TOKEN` | 沙盒内 `gh` CLI 读 issue / claim / 留评论 | 宿主 `gh` CLI 的 OAuth token（#1005） |
| `GIT_AUTHOR_NAME` / `GIT_AUTHOR_EMAIL` | 沙盒内 commit 身份（非密钥，走同一注入通道） | 编排侧配置 |
| `GIT_COMMITTER_NAME` / `GIT_COMMITTER_EMAIL` | 同上 | 同上 |

### 注入机制

sandcastle 自动从 `.sandcastle/.env` 与 `process.env` 解析环境变量注入沙盒；agent provider 与 sandbox provider 各自的 `env` 可覆盖同名键，但两者键面不得重叠（重叠即抛错）。本仓 `prime-agent-provider.ts` 的 provider env 默认为空，凭据全部走 `.env`。

### 为何 gitignore

豁免是双层的：

1. 仓根 `.gitignore`：`.env` / `.env.*` / `.env.bak-*`；
2. `.sandcastle/.gitignore`：`.env` / `logs/` / `worktrees/`。

`.env` 含 4 家模型 provider key 与一个 repo 全 scope 的 GitHub token，一旦入仓即等于凭据泄露且随 git 历史永久残留，故永不入仓（#1000 N-3 裁定原文：「`.sandcastle/.env` 永不入仓」）。验证口径：`git check-ignore` 豁免确认 + `git grep` 对已跟踪文件密钥零命中（#996 AC-6、#1010 复核均过）。

## 2. `GH_TOKEN` 权限面

- **形态**：`gh` CLI 的 OAuth token，`repo` 全 scope（#1005 关票）。
- **为什么是 repo 全 scope**：sandcastle 官方文档/envExample 只写细粒度 PAT「Issues RW + Metadata R」，但上游 issue #937 证实推分支、开 PR 还需 **Contents RW + Pull requests RW**；申领时按后者放全（#1005 权限修正）。本仓直接取 `gh` OAuth token，免人工申领 PAT，天然覆盖。
- **能力 vs 纪律的落差（如实标注）**：token 技术上具备本仓全部读写能力（push、合 PR、关票均可为）；「工蜂不关票、不 push、不合 main」是 **prompt 层硬性规则与编排层人工闸**，不是权限位强制。信任模型是「凭据够宽让流程跑得动，约束靠流程兜住」。
- **附带注入**：蜂群场景另注入 `GH_REPO=xy7365527-lang/NewChanlun`（clone 场景 origin 是本地路径，`gh` 靠它认仓，#1009）。

## 3. 沙盒网络面

- **默认全通**：`docker()` 不指定 `network` 时使用 Docker 默认 bridge，容器可自由出网——装包、调模型 API、`gh` 都依赖此。**无内置 egress 白名单**；收窄只有 `network: "<自定义网络>"` 一条路，官方未提供开箱的出口白名单机制（#999 报告 §5.3）。
- **隔离强度**：普通容器、共享宿主内核，比 microVM 弱一档（上游 #682 在案）；要更强隔离须换 isolated provider（如 Firecracker microVM），当前未采用。
- **prompt 注入面（在案风险）**：上游 #870——issue 评论注入是 open 安全洞（攻击者等 issue 被打上触发 label 后塞恶意评论）。本仓缓解：仓为 private + 拾取闸是专属 label `sandcastle`（#1008），但这不是硬门，读评论即承担注入面。
- **推论**：「网络全通 + repo 全 scope token + 自动执行的工蜂」三者叠加意味着——**prompt 内容（含 issue 正文/评论）即攻击面**。本条不作整改结论，只登记事实；收窄动作若要做，走 #1000 的例外举证通道。

## 4. 密钥不落盘纪律

1. `.env` 永不入仓（双层 gitignore，见 §1）；
2. key 从宿主 `auth.json` **静默迁移**：不打印值、不写任何已跟踪文件（#1006 裁定原文）；
3. **Dockerfile 与镜像资产不含任何 key**——provider 凭据只在运行时经 env 注入（`image/prime-agent-settings.json` 只登记 MCP server，凭据字段为零）；
4. 工蜂 prompt 硬性规则「密钥永不写入任何文件」；commit 前评审面含密钥扫描；
5. 任何会话导出/备份/工具残留按 AGENTS.md 名分四态禁入仓。

## 5. prebake 的 `--change` 清 Env 机制

`prebake-kernel.sh`（#1004）把 prime-agent 的 IPython kernel venv 预烤进镜像（首次 ipython 调用 1-2 分钟 → 秒级）。其中与凭据相关的环节：

1. **起一次性容器**：`docker run --env-file .sandcastle/.env`，触发一次真实 kernel bootstrap——**key 仅在此刻以运行时 env 存在**，不写镜像层文件；
2. **同进程清理运行时残留**：`rm -rf /tmp/prime-agent-* ~/.prime/agent/{daemon-workers,session-leases,logs,sessions}`——清理必须与 agent 同生命周期（`docker start` 重跑会重新拉起 agent）；`/tmp` 残留若烤进镜像会导致新容器 daemon rename 撞 overlayfs（EXDEV，#1004 实测）；
3. **`docker commit` 固化时逐键清空 Env**：对 §1 的全部 9 键执行 `--change 'ENV <KEY>='`，防止密钥随 commit 进入镜像 config（镜像 config 的 Env 可被 `docker inspect` 读出，不清空即等于密钥落盘进镜像）；固化后 `docker inspect` 复核 Env「无」；
4. **同时恢复入口**：`--change 'ENTRYPOINT ["sleep", "infinity"]' --change 'CMD []'`——`docker commit` 会把一次性容器的入口覆盖烤进镜像（#1004 发现、#1009 定案的「137 假象/入口劫持」根因），必须显式还原。

**推论**：预烤后的镜像 `sandcastle:newchanlun` 是不含密钥的可分发物，前提是每次 `Dockerfile` 变更重建后都重跑一次 `prebake-kernel.sh`（脚本内置此口径）。

## 相关票

- #996：脚手架落地 + `.env` gitignore 豁免首验；
- #999：官方安全模型调研（本文件 §3 的来源）；
- #1004：prebake 脚本与 `--change` 清 Env 机制；
- #1005：GH_TOKEN 形态与权限修正（上游 #937）；
- #1006/#1010：凭据搬迁裁定与落地（moonshot key 失效卡点在案）；
- #1008/#1009：两段式与蜂群模板（拾取闸 label、`GH_REPO` 注入）。
