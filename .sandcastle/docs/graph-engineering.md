# Graph engineering 流程（wayfinder → 裁决 → Sandcastle → 合入）

本文定义 #1323 这类大图从决策票到合入 main 的自动化流程。wayfinder 规则以 `docs/agents/wayfinder-workflow.md` 为准；本文只补它 v1 明写未覆盖的一层：**决策票的代理裁决**（该文「v2 观察项 = 预授权影子模式」的落地）。

## 五个面

| 面 | 组件 | 节奏 | 管什么 |
|---|---|---|---|
| 控制面 | `scripts/wayfinder_engine.mts`（已有） | 4 分钟 | 扫图、给合格实施票挂 `sandcastle` |
| 裁决面 | `.sandcastle/decision-claimer.mts`（新） | 10 分钟 | grilling 票：分级 → 起草 → 独审 → 落地或待批 |
| 执行面 | `claim-loop.mts` + `main.mts`（已有） | 60 秒 | 实施票：实装工蜂 + 独审工蜂 |
| 验收面 | `harvest.sh`（已有）+ main 合入人闸 | 定时 | 封包验收、批量 PR |
| 观测面 | `health-monitor` + 进度页（新） | 5 分钟 / 1 分钟 | 停摆即 _red_、自愈、进度 |

人只守三闸：spec 批准、P 类决策、合入 main。

## 裁决流程（每张 grilling 票）

### 1. 分级

Fable 5.1 读票面与关联票，判一类，并在票上评论一行分级理由：

- **T 类（技术取舍）**：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。→ 自动裁决。
- **P 类（政策）**：资金与费用参数（本金、量档、cap、分配）、风险偏好、真实场所/账户/权限、关图与 #1279 去向、Destination 改写、产品 taste、本仓无先例的新取舍。→ 影子模式。

拿不准一律判 P。完成标准：票上有 `分级：T|P — 理由` 评论。

### 2. 起草

Fable 5.1 起草者在独立 worktree 里只读取证，产出固定结构的 resolution 草稿：问题、权威依据、候选、裁定、验收清单、受影响票与 blocking 边变化、授权引用。

权威依据逐条带可回查位置：`docs/chanlun/text/blog/<课>.md:<行>`、ADR 路径、票评论链接。引原文按 AGENTS.md 四步核行号、逐字、作者归属、行内注。验收清单每项必须是命令、测试名、文件存在或票状态之一。

完成标准：草稿文件落 `/Volumes/AgentStorage/decisions/<票号>/draft.md`，七段齐全。

### 3. 独审

Opus 5.5 审查者另起进程，只拿草稿和源文件，看不到起草推理。按 R1–R7 逐条给 PASS 或 BLOCK：

1. R1 每条引用回查成立。
2. R2 不与 `.chanlun/definitions/`、`docs/adr/`、#799 收敛通则、#804 总缝规则冲突。
3. R3 验收项全部可执行、判据二值。
4. R4 不越出 Destination；T 类草稿里没有夹带 P 类取舍。
5. R5 不新增同一判断的宽严两档或失败兜底。
6. R6 受影响票与 blocking 边清单完整。
7. R7 分级正确。

完成标准：`review.json` 落盘，七项各有结论与依据。BLOCK 时起草者升档为 Opus 5.5 带审查意见重做一次；再 BLOCK 转 P 类。

### 4. 落地

- **T 类 PASS**：正本写进 `decision/<票号>` 分支（`.chanlun/definitions/`、`docs/adr/` 或 `CONTEXT.md`），票上发 resolution 评论，首行写「本条依 #1323 Notes N-1 预授权判定」，并附草稿、审查路径。决策文档按日批量开 PR；PR 合入后关票，map 的 Decisions so far 加一行，接新 blocking 边。
- **P 类**：票上发草稿与审查结论，挂 `needs-approval`，不关票。用户批量批复后按 T 类落地。

### 5. 解锁

wayfinder_engine 下一轮重扫；FU blocker 全关的实施票被挂 `sandcastle`，进执行面。

## 授权记录

#1323 Notes 追加一条，comment 同步留痕：

> **N-1 技术类决策预授权**（用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」）：本图 T 类 grilling 票由代理起草、独立审查通过后直接裁定；P 类走影子模式待批。依此判定的 resolution 首行注明「本条依 #1323 Notes N-1 预授权判定」。

`wayfinder-workflow.md` 的「不碰决策票（v1）」只约束 wayfinder_engine 本体，裁决面是独立组件；该文同步加一行指向本文。

## 调用通道

`spawn_agent` 是会话内工具，launchd 脚本调不到。所有后台工蜂走 Claude CLI 经本机统一路由：

```bash
ANTHROPIC_BASE_URL=http://127.0.0.1:8318 \
ANTHROPIC_AUTH_TOKEN=<运行时从 CLIProxyLocal config 读取，不落日志> \
claude -p --model claude-fable-5-1 --output-format stream-json
```

2026-10-19 实测：`claude-fable-5-1`、`claude-opus-5-5` 两个无前缀名经自动路由均正常返回。模型分工：分级与起草用 Fable 5.1，独审与实施用 Opus 5.5，429 按退避重试。

执行面 `main.mts` 由 prime-agent 改为 Sandcastle `claudeCode` provider，走同一环境变量。沙盒后端用 noSandbox + 独立 worktree（判例：#1456 `codex-child.ts`）；Docker daemon 与 `sandcastle:newchanlun` 镜像就绪后再切回 docker。

## 宿主约束

Sandcastle 宿主 `NewChanlun-sandcastle-host` 开启 `SANDCASTLE_HOST_AUTO_UPDATE=1`：每次派工前 `reset --hard origin/main`，工作区脏则拒绝派工。因此执行面的任何改动都必须经 PR 合入 origin/main，宿主上不保留本地提交。

## 并行度

- 裁决面：同时处理至多 3 张互不阻塞的 grilling 票，claim 先行防重复。
- 执行面：当前单实例串行；接通后改为至多 3 个并发 main.mts，按票号加锁。
- 外盘不足时，长任务的 worktree 与日志可放 Windows 主机（Tailscale SSH）。

## 流程完成判据

1. 裁决面 dry-run：对一张 T 类票产出 `draft.md` + `review.json`，不写 GitHub。
2. 裁决面 live：一张 T 类票走完分级、起草、独审、分支落文档、PR。
3. 执行面：claim-loop 派出的实施工蜂实际产生输出（上下文非 0、有 commit 或明确 blocked 结论）。
4. health-monitor 发现停摆能变 _red_ 并自愈，进度页可看。
5. 以上改动经 PR 合入 main（人闸）。
