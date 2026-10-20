# #1406 TB-03-C 切片 S1 报告：FIN/1 同重合链

- 分支 `codex/1406-s1-chain`，起点 `6c8477ff6f`（`origin/codex/1406-owner-20261020`）。
- 写集：新增 `rust/src/economic_session/tb03c_chain_tests.rs`，`mod.rs` 加一行 `#[cfg(test)] mod tb03c_chain_tests;`。独立手算 oracle 在 `.chanlun/review-results/issue1406-owner-20261020/S1-oracle.py`。**生产码零改动。**
- 本报告只陈述证据，不自评通过。

## 结果

同一重 alpha 已在一个 E/B 会话里走完 Prepare → Reserve → Commit → Capture → Resolve → Apply 全链。全部请求都经 `service::handle` 分派。财务三步走 `finance::write_with_ingress`，包括正式 S reader 运输与跨域 Prepared/grant 证明读取；没有直调 `persist` 或 reducer。随后关库，用新 store 实例 `recover` + `audit` 重开，公共查询逐值相同。

两条链能接上，但接法暴露了一个关键缺口（下文 F1）：Commit 之后的成交能否 Resolve，与这次 Commit 产生的 grant 责任无关，只取决于冻结的 FIN 输入有没有事先在 `baseline.covered_facts` 里列出该成交。

## 版本选择

- **书账**：`tb03c-book/1`（`book::TB03C_BOOK_V1`）。
- **财务**：`tb03b-finance/1`。main 上现行有两版（`finance.rs:17` 的 REV=/1，`:19` 的 REV_V2=/2）。选 /1 有三个原因：
  1. /2 必须配 `economic-session/2` 域和真实 `s-financial-basis/3` 读源，后者受 #1552/#1488 阻塞；
  2. /2 在 macOS 上要求固定容量卷；
  3. #1394 现有的 `c4a_fin1_*` 与 `tb03b_runtime.py` 都用 /1。
- **资金数值**全部是 TestOnly 夹具，与 `finance_tests.rs` 同族：
  - cap 100，既有责任 20；
  - 计划：quantity 2 / limit 14 / fee_tail 2；
  - 私有现金权利 40。
- 本例未使用 ADR 0040 的 P0 签署值。本金批没有授权，测试断言 `principal`、`growth_enabled` 均为 `NotEvaluable`，没有填任何默认值。

## 测试覆盖

### 主测试

主测试为 `tb03c_chain_fin1_prepare_reserve_commit_capture_resolve_apply_same_chong_and_reopen`。

**开局**：真实 Capture → Apply → BindVoice，之后 ImportResponsibility。全程没有手工设仓。

**Prepare / Reserve / Commit**：
- Prepare：locked 30，worst 6；
- Reserve：Reserved，retained 50，一个 grant 归 alpha、额 30；
- Commit：`LocalCommitReceipt`，状态 Committed，`grant_id` 与该 grant 一致；
- Commit 本身不动仓（仍为 4）。

**成交与 Resolve**：Capture 的成交为 netted，from 0 → to 2，单价 13，佣金 2。之后经 FIN 的已保留责任界完成 Resolve。

**归属**（限定：Commit 与成交的对应由夹具构造（成交预列于冻结 baseline），产品不强制，见 F1/F2，归 TB04-F #1425；`command_id == plan_id` 一条只核夹具值经 Resolve 原样传递不变，不是产品绑定）：
- 唯一分配为 `chong_id=alpha`；
- `command_id` 等于被 Commit 决定的 `prepared.plan.plan_id`；
- B 端 `QueryFact` 的 `resolved_allocation` 回指 alpha / plan-alpha；
- B 的决定为 Committed，`prepared.voice.binding_id` 和 B 现存 Voice 都是 `voice-alpha`。

**oracle 对拍**（Python 独立推导，数值写死在测试里）：

| 项 | 值 |
|---|---|
| 分配 | units 2 / cash 26 / commission 2 |
| B 数量 | 4 → 6 |
| K | 80000 → 80028 |
| cost_remaining | Known 80028 |
| E 责任总额 | Resolve/Apply 前后始终 50，不下降 |

**负控**：

| 情形 | 结果 |
|---|---|
| 同事实重送 Capture | 原收据 |
| 同证据重送 Resolve | 原收据 |
| 同分配重送 Apply | 原收据；`applied` 只多 1 条 |
| 异内容冲突：同来源身份改价重送 Capture | `IdentityConflict`，无提交 |
| 另一 `evidence_id` 覆盖已分配事实 | `IdentityConflict` |
| 错 `expected_fact_hash` 的 Apply | 完整文案 `IdentityConflict：实读E事实/金额/归属与请求不匹配`，无提交 |
| 缺前件：累计前沿缺口成交（2 → 3） | Resolve 得 `MissingDependency`（含「前沿缺口」），无提交；B Apply 得完整文案 `AwaitingDependency：净额事实尚无 E 持久分配`；该成交 Pending 保留，无分配行 |
| Resolve 之前 Apply | 完整文案 `AwaitingDependency：净额事实尚无 E 持久分配`，B 不变 |
| 估值观察（14 → 9） | E 记 Recorded；B Apply 得 `SchemaUnsupported`，B 不变，K 仍为 80028 |

**重开**：
- E、B 各自 `recover`，writer_epoch 递增，新连接 `audit` 通过；
- 以下公共查询逐值相同（ReadView 只去掉随 epoch 合法变化的 `writer_epoch`）：
  - E：ReadView、QueryFact（成交、缺口）、QueryGrant；
  - B：ReadView、QueryFact、QueryDecision、QueryAllocation、QueryVoice；
- 重开后用新消息身份重送 Apply、旧 epoch 的 Commit 和 Capture，均只返原收据，不多入，E 责任仍为 50。

### 反证测试

反证测试为 `tb03c_chain_fin1_grant_responsibility_alone_does_not_cover_post_commit_fill`。流程完全相同，只是 baseline 不预列成交。Commit 成功、E 已持有 alpha 的 grant 责任 30（总额 50），成交 Resolve 仍得 `MissingDependency：相关 Pending 未被已核责任界覆盖`，无提交。

### S 信任根

`finance.structure.binary` 指向测试目录里的一个 `/bin/sh` 桩，它只 `cat` 一份固定的 TestOnly FinancialBasis，sha256 已钉入 manifest。正式 S reader 运输（`finance_transport::reader`）与 `verify_structure` 都真实执行。这是 TestOnly 信任根，**不能计作真实 S 验收**。

## 发现

**F1（关键，需裁定）：Commit 产生的 grant 责任覆盖不了它自己引起的成交。**

- Resolve 在 FIN 下只认带 `source.covered_facts` 的责任（`tb03c_e.rs:206`）。
- 只有 ImportResponsibility 的 baseline 责任带这个字段（`finance.rs:923`，`source=d["baseline"]`）。
- Reserve 推入的 grant 责任形如 `{"responsibility_id","owner","amount","grant"}`（`finance.rs:1093`），没有 covered_facts。

所以要让 Commit → 成交 → Resolve 走通，唯一办法是在 Prepare 之前，把未来成交的 fact_key / fact_hash / token 预写进冻结输入（主测试正是这样做的，见夹具注释）。反证测试锁住了这一现状。这不算产品缺陷修复范围：它正是前沿分析 Q1 所说「fill 必须属于某个已 Committed 的 grant」的新语义，票面归 TB04-F，所以本片没有改生产码。

如要接线，候选的最小方案是：Resolve 在 FIN 下额外接受这样一笔 grant 责任——冻结投影参与重的 `command_id` 等于该 grant 的 `plan_id`，且累计成交量不超过 `grant.quantity`。这需要改 `tb03c_e.rs:200-210`，并给 grant 加覆盖判据。

**F2（同源，代码阅读，未单测）**：`command_id` 与决定、grant 之间没有任何绑定。
- `tb03c_allocation.rs:97-104` 只要求字段存在，`:349` 原样复制。
- 该文件里没有任何对 finance decisions 的引用，成交量也不和 Commit 数量比较。
- 因此主测试里的「`command_id` 回指被 Commit 的计划」只靠夹具构造成立，产品不强制。

## 远端运行

均为 `NC_TARGET=target-1406-s1 ~/.local/bin/nc-remote codex/1406-s1-chain rust …`。

| SHA | 命令 | 结果 | 日志 |
|---|---|---|---|
| ee113fdcd2 | `cargo test --lib --features s_session -- tb03c_chain --nocapture` | 2 passed | `/tmp/nc-remote/20261020-234655-ee113fdcd2.log` |
| 1fc8453540 | 同上，加 `economic_session` 全量 | 2 passed；157 passed / 0 failed / 8 ignored | `/tmp/nc-remote/20261020-234917-1fc8453540.log` |
| 1c479a91b9 | `tb03c_chain` | **1 failed**：`cost_remaining` 断言写错（多头应为 Known K），下一提交更正 | `/tmp/nc-remote/20261020-235417-1c479a91b9.log` |
| 872121783e | `tb03c_chain` | 2 passed | `/tmp/nc-remote/20261020-235724-872121783e.log` |
| 872121783e | `cargo test --lib --features s_session economic_session` | 157 passed / 0 failed / 8 ignored（两项新测试名在日志内） | `/tmp/nc-remote/20261021-000037-872121783e.log` |

本报告的提交只加 `.md` 和 `.py`，不触 Rust。

## 需主控裁定

1. F1/F2：是否把「成交须归属某个已 Committed grant，并由该 grant 覆盖」纳入 #1406，还是维持归 TB04-F。如纳入，需要根裁定判据（`command_id` ↔ `plan_id`/`grant_id`、超量成交怎么处理）。
2. 用 shell 桩替代 S 走正式 reader 这一 TestOnly 手法能否计入 AC1 的进程内基线。它比 `finance_tests` 的 `persist` 缝多覆盖了正式分派、S 运输和跨域证明读取，但不是真实 S。
3. 提交 1c479a91b9 能编译，但其中一条断言是错的。分支未改写历史，是否压扁由主控决定。
