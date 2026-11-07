# Wayfinder 工作流

Wayfinder 用图处理尚不清楚的依赖与决定。已知步骤的任务直接实施；只需整理实施方案时走 `/to-spec` → `/to-tickets` → `/implement`。tracker 命令见 [issue-tracker.md](issue-tracker.md)，自动化入口见 [graph-engineering.md](../../.sandcastle/docs/graph-engineering.md)。

## 开图与走图

1. 搜 tracker 全部状态及仓内相关材料，复用已有票与裁定。开图写清 Destination、Notes 和 Not yet specified；先列 frontier，再接真正的 blocking 边。
2. 默认决策与实施同图。纯决策图在 Notes 使用精确声明「本图为纯决策图，不带实装——理由：…」，并写明图外承接位置；长驻台账写明不预期关闭，否则到达目的地即关图。
3. 每轮先核子票与 Decisions so far，补缺失索引。查询子票、评论及 blocker 使用 `gh api --paginate`，不能把第一页当全集。
4. 选择 open、无 open blocker、未认领的票，先 claim 再处理。已有 owner 的票不重复派工；有写入的并行任务用独立 worktree，并明确写区。GitHub assign 不是互斥锁，写入前仍要核 owner。
5. 按票型取得结果，在 resolution comment 留结论、依据、验证与产物位置，再关闭已完成的票。图正文只补一句 gist 和链接；因本票而明确的雾转为具体任务。

blocking 边只表达必要依赖。可以独立开始的票不串成链；优先级写在 Notes，不伪装成 blocker。

## 票型与代理裁定

- **grilling**：由既有证据和推理决定“该是什么”。裁定及必要正本文档一起落地。
- **research**：补充仓外事实或源材料，结论供决策票使用。
- **prototype**：答案必须造出来才知道。一次性原型保存在 `proto/<票号>` 分支，票内留指针；长期测试 fixture 和可复用探针保留在产品测试链。
- **task**：实施、测量或解阻塞。有已知方案的工作直接执行；不属于图目标的欠账另走 triage。

T/P 两类决策均经过考古、起草和非作者独审，全 PASS 后由代理裁定（#1909 / PR1915）。不再要求“agent 不得替人答”、`needs-approval` 或 `/graph-approve`。起草者提供推荐答案与推导，独审核对真实材料，不以撤标签代替通过。

`decision-claimer` 负责决策票，`wayfinder_engine` 不替它裁决。BLOCK 用尽、`fact_gap` 或 `invalid_landing` 进入 held：事实缺口开 research/探针票并接 blocking 边；材料补齐后在原票发「退回：…」，整链重跑。旧 shadow 产物先原件归档，旧 PASS 不复用。旧 `needs-approval` grilling 票由 claimer 按新流程处理；实施票逐张核依赖、owner 和 SPEC 裁定，不批量摘标签。

## 从决策到实施

图内一批决策已解决、下游工作不再依赖未决问题时，按 `/to-spec` → `/to-tickets` → `/implement` 交棒，不等长驻图全部结束。终点本身就是一份文档时，票内明确的产出要求可直接作为实施输入。

SPEC 由非作者独审。有效放行记录为编排者账号发布的结构化「SPEC 独审裁定：PASS」，绑定正文摘要、作者、不同于作者的 reviewer 及证据。使用 `npx tsx scripts/wayfinder_engine.mts --spec-ruling <号> --review-file <独审原件.json>` 校验并生成记录；原件必须包含 specNumber、digest、author、reviewer、verdict，票号和所审正文须匹配，发布时不得重算原件 digest，格式和切换规则见 graph-engineering「SPEC 裁定」。自由文本“批准”不替代结构化记录；正文变更须重新绑定独审。BLOCK 交回起草者返修后重审。

开发流程裁定不授权真实资金、交易、账户或凭据操作；这些能力仍受 ADR 0041 及具体任务授权约束，图的 Notes 不代签。

有效 SPEC 裁定后拆 tracer-bullet 实施票、接 blocking 边并逐票执行。只有符合票型、未认领、无 open blocker 且满足所属 SPEC/DAG 条件的票获得 `sandcastle` 标签；不把历史 `ready-for-agent` 全部放行。SPEC 裁定沿子树递归，控制面从 tracker 重算 ready 集。

Sandcastle 使用既有隔离执行与独审入口；LangChain Deep Agents 队列沿 `pilot1742 executor serve` 与 ClaimStore 运行，不转交旧 swarm，也不重复认领已有任务。模型和路由以各入口配置为准。合入须非作者独审、适用 CI 和当前任务的合入授权；服务与发布变更仍按相应任务范围执行。

过程证据落 `/Volumes/AgentStorage/<任务>/`，票与 PR 记录精确 head、实际结果及证据位置。产品文档、源代码、许可证和实际测试输入入 Git。SPEC 随对应交付完成而关闭，长期语义由代码、ADR 与教义正本维护。

## 图正文的写入协议

决策正本在子票的 append-only resolution comment；`## Decisions so far` 只是可重建索引。保留整份正文视图，并按以下步骤保护并发写入：

1. 要写时才读最新 body 和 `updated_at`，不要复用开场缓存。
2. PATCH 前重查 `updated_at`；变化则重读、重拼。该检查不是原子 compare-and-swap，仍有 TOCTOU 残窗。冲突重试一次，再冲突则报告冲突并交由当前图写入者处理，不盲目循环覆盖。
3. 每轮开场将已关子票与索引对账，恢复漏行。决策详情留子票，索引只留一句 gist 和链接。GitHub issue body 上限 262,144 字节；逼近限制时压缩索引或拆图。
4. Not yet specified 只改本票实际解决的雾。并发产生重复票时，保留先开的票，关闭重复票并指向原票。

## 关图与交接

到达 Destination 后，每条残雾须有具体去向：新票、Out of scope，或明确搁置。写“归 #N”前核实 #N 真有承接条目；没有就补条目。图完成而实施尚未完成时，保留真实 owner 与后续票，不以关图冒充交付。

标题前缀和 `wayfinder:<type>` 标签同时对齐票的实际工作。历史已关票不重贴、不重判。旧工作流与纪律原件归档于 `/Volumes/AgentStorage/restart-20261107/slim/docs-archive/before-3d61fb/docs/agents/`。
