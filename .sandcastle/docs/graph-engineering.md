# Graph engineering 流程（wayfinder → 裁决 → Sandcastle → 合入）

本文定义 #1323 这类大图从决策票到合入 main 的自动化流程。wayfinder 规则以 `docs/agents/wayfinder-workflow.md` 为准；本文只补它 v1 明写未覆盖的一层：**决策票的代理裁决**（该文「v2 观察项 = 预授权影子模式」的落地）。

## 五个面

| 面 | 组件 | 节奏 | 管什么 |
|---|---|---|---|
| 控制面 | `scripts/wayfinder_engine.mts`（已有） | 4 分钟 | 扫图、给合格实施票挂 `sandcastle` |
| 裁决面 | `.sandcastle/decision-claimer.mts`（新） | 10 分钟 | grilling 票：分级 → 考古 → 起草 → 独审 → 落地或待批 |
| 执行面 | `claim-loop.mts` + `main.mts`（已有） | 60 秒 | 实施票：实装工蜂 + 独审工蜂 |
| 验收面 | `harvest.sh`（#1660 PR 模式）+ main 合入人闸 | 15 分钟 | 评审完成的分支推远端、基于 main 开 PR、票面登记；不合入不关票 |
| 观测面 | `health-monitor` + 进度页（新） | 5 分钟 / 1 分钟 | 停摆即 _red_、自愈、进度 |

人只守三闸：spec 批准、P 类决策、合入 main。

## 裁决流程（每张 grilling 票）

### 1. 分级

Fable 5.1 读票面与关联票，判一类，并在票上评论一行分级理由：

- **T 类（技术取舍）**：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。→ 自动裁决。
- **P 类（政策）**：资金与费用参数（本金、量档、cap、分配）、风险偏好、真实场所/账户/权限、关图与 #1279 去向、Destination 改写、产品 taste、本仓无先例的新取舍。→ 影子模式。

拿不准一律判 P。完成标准：票上有 `分级：T|P — 理由` 评论。

### 2. 考古

用户 2026-10-20 令：「所有的裁决票都要先考古」。考古是三条既有规矩在无人值守裁决面上的执行，不另立机制：

- `/grilling`：「If a *fact* can be found by exploring the environment … look it up rather than asking me. The *decisions*, though, are mine」——事实由代理查完，决定才问人，每问附推荐答案。
- `/grill-with-docs`（= `/grilling` + `/domain-modeling`）：对照 `CONTEXT.md` 与 `docs/adr/`，已裁的直接用，与词汇表或代码冲突的当场指出。
- AGENTS.md「开之前先搜（两路）」与 ADR 0012 裁定一、四：tracker 含已关闭票与评论，仓内含推论层；推论层主张可援引，推理链搬进票内重过。ADR 0012 代价第 4 条「每张裁定票前多一个 AFK 子代理」即本步。

无人值守下没有人能当场纠正「这个早定过了」，所以胶水只补一件事：把查过的证据落成可复核的文件。Fable 5.1 只读取证，逐个待决点（Q1、Q2…）两路各至少一条查询，归一类：

- **A 已裁**：正本层或已关票 resolution 明写。写出处，直接沿用，不再列为待决。
- **B 有先例未正式裁**：推论层、票评论、代码常量或夹具在用。写出处与推理链，作为默认候选，以「默认值，确认或否决」呈现。
- **C 确未设计**：列出已搜关键词，标缺口去向——`decision` 交用户，必带推荐答案及推导依据；`research` 是仓外事实，按 wayfinder 开 research 票由 `/research` 子代理解决；`probe` 缺读数推不出推荐值，按 `wayfinder-workflow.md` 反面判据③派探针票。

决定等事实（`/wayfinder` Ticket Types：research 票「surface a fact a decision waits on」）：考古有 `research`/`probe` 缺口的 T 类票转 P，不自动落地；缺口票由 `/graph-approve` 开。

机械校验：两路缺一、tracker 未覆盖已关闭票、A/B 缺出处、B 缺默认值、C 缺关键词或去向、C 类 `decision` 缺推荐答案，都带问题重跑一次（升档重做时随起草者升档重跑）；仍不过的问题交审查，草稿因此 BLOCK。

完成标准：`/Volumes/AgentStorage/decisions/<票号>/archaeology.md` 与 `archaeology.json` 落盘，每个待决点有查询词、命令、命中与归类。只验考古：`node --import tsx .sandcastle/decision-claimer.mts --archaeology-only --ticket <号> --out <目录>`（不看票是否已关，不写 GitHub）。

### 3. 起草

Fable 5.1 起草者在独立 worktree 里只读取证，产出固定结构的 resolution 草稿：问题、权威依据、候选、裁定、验收清单、受影响票与 blocking 边变化、授权引用。

权威依据逐条带可回查位置：`docs/chanlun/text/blog/<课>.md:<行>`、ADR 路径、票评论链接。引原文按 AGENTS.md 四步核行号、逐字、作者归属、行内注。验收清单每项必须是命令、测试名、文件存在或票状态之一。

遇缠师原文无定论、有多个可行候选的待决点（`[新缠论:选择]` 或参数类），按 [ADR 0041](../../docs/adr/0041-degrees-of-freedom-configurable-not-excluded.md) 优先裁为「可调字段 + 默认值」：落进具名版本化记录，写默认值与出处、可选值及代价，改值即新版本；另一种合法取值保留为可选值。已裁定签署前不得代填的字段（资金、量档、容量、成本门 θ、风险阈值等），默认值写 `AwaitingPolicy`（不签不跑、无备用值），数值只以「推荐值＋出处」列进签署单待用户确认。违原文、违已裁推论、被实测否证、属 #799 兜底或与现行判据构成同一判断宽严两档的候选照旧排除。

起草按考古处置待决点：只有 C 类 `decision` 列为真正待决；B 类写成默认值待确认；C 类 `research`/`probe` 写成要开的票。草稿末尾附「考古对照」表，每个考古点号一行。

完成标准：草稿文件落 `/Volumes/AgentStorage/decisions/<票号>/draft.md`，七段与「引用自证」「考古对照」齐全。

### 4. 独审

Opus 5.5 审查者另起进程，只拿草稿、`archaeology.md` 和源文件，看不到起草推理。按 R1–R8 逐条给 PASS 或 BLOCK：

1. R1 每条引用回查成立。
2. R2 不与 `.chanlun/definitions/`、`docs/adr/`、#799 收敛通则、#804 总缝规则、ADR 0041 自由度通则冲突。把原文无定论的合法候选当错误排除或写成常数、未落为带默认值的可调字段，判 BLOCK；把原文语义或已裁推论改成可调、把同一判断的宽严两档做成可调字段、或让可调字段运行中换值，也判 BLOCK。
3. R3 验收项全部可执行、判据二值。
4. R4 不越出 Destination；T 类草稿里没有夹带 P 类取舍。
5. R5 不新增同一判断的宽严两档或失败兜底。
6. R6 受影响票与 blocking 边清单完整。
7. R7 分级正确。
8. R8 考古成立：抽查 `archaeology.md` 至少三条命中并复跑查询，另对任一 B/C 点换说法复搜。草稿把 A 类当待决、B/C 类缺查询证据、「考古对照」漏点、事实缺口交给用户，或 P 类问题要求用户凭空提供数值而在案先例未列出，判 BLOCK。

完成标准：`review.json` 落盘，八项各有结论与依据。BLOCK 时起草者升档为 Opus 5.5 带审查意见重做一次；再 BLOCK 转 P 类。

### 5. 落地

- **T 类 PASS**：正本写进 `decision/<票号>` 分支（`.chanlun/definitions/`、`docs/adr/` 或 `CONTEXT.md`），票上发 resolution 评论，首行写「本条依 #1323 Notes N-1 预授权判定」，并附草稿、审查路径。决策文档按日批量开 PR；PR 合入后关票，map 的 Decisions so far 加一行，接新 blocking 边。
- **P 类**：票上发草稿与审查结论，挂 `needs-approval`，不关票。影子评论开头是考古问表：只列 B 类确认与 C 类 `decision`，每问附出处与推荐值；A 类列为已沿用，事实缺口列为要开的票。用户批量批复后按 T 类落地。

### 6. 解锁

wayfinder_engine 下一轮重扫；FU blocker 全关的实施票被挂 `sandcastle`，进执行面。

放行判据（2026-10-20 图审计第 13–15 条修正）：扫描集是 open 的 `ready-for-agent` ∪ `wayfinder:impl`；实施票按 `[impl]` 标题或 `wayfinder:impl` 标签认（决策票除外）。批准态沿 sub_issues 子树递归：祖先链上有 SPEC 的票，链上每张 SPEC 都已批准才算批准；没有 SPEC 的 map 直挂票及其子票继承 map 直挂 SPEC 的批准态，且仍须带 `ready-for-agent` 才进放行队列。有任何 assignee、有 open blocker、带 `needs-approval`/`ready-for-human`/`needs-info`/`needs-triage`/`wontfix`、或仍有 open 子票的汇总票，一律不挂 `sandcastle`；挂之前重读单票复核一次。裁决面 decision-claimer 在 live 与 dry-run 下都跳过本地已到终态（`done`/`land_t`/`shadow_p` 且无断点）的票，以及已关闭、已有 resolution、或编排者账号首行「用户 … 批复」评论的票（之后的「退回/驳回/重写」评论会重新打开）。要重跑已到终态的票，先把 `/Volumes/AgentStorage/decisions/<票号>` 改名（先例 `1530.returned-20261020`）。

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

2026-10-19 实测：`claude-fable-5-1`、`claude-opus-5-5` 两个无前缀名经自动路由均正常返回。模型分工：分级与起草用 Fable 5.1，独审与实施用 Opus 5.5，429 按退避重试。Fable 安全拦截（`Fable 5's safeguards flagged this message`）不算临时错误：分级/考古/起草被拦时该步改用 Opus 5.5 重试一次（本票内该角色粘住），起草因此改 Opus 的那一轮独审改用 Fable 5.1，独审再被拦则改回 Opus 另起一次调用——两档模型下这一轮即 Opus 起草、Opus 独审的两次独立调用，与升档重做轮同口径（分离要求的是不同进程/调用，不是不同模型）；切换记入 `summary.json` 的 `modelSwitches` 与日志（#1785）。

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
