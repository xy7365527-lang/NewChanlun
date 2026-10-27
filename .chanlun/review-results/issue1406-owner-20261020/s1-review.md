APPROVE（复审 e14fed41ee：MEDIUM-1、LOW-1、LOW-2 均已关闭，生产码零改动；原审结论见下）

# #1406 S1 合链测试独立评审：codex/1406-s1-chain @ c793b4c9a5

基线 `6c8477ff6f`，`git diff --stat 6c8477ff6f c793b4c9a5`：只动 `rust/src/economic_session/mod.rs`（+2 行 `#[cfg(test)] mod tb03c_chain_tests;`）、新增 `tb03c_chain_tests.rs`（631 行）、`S1-REPORT.md`、`S1-oracle.py`。**生产码零改动属实。** 评审人非作者，只读。

## 实跑

`NC_TARGET=target-1406-review ~/.local/bin/nc-remote codex/1406-s1-chain rust cargo test --features s_session --lib economic_session` → sha c793b4c9a5，`157 passed; 0 failed; 8 ignored`（log `/tmp/nc-remote/20261021-000736-c793b4c9a5.log`）。两项新测试在 :851、:872，均 ok。

## (1) 正式入口

- 测试里所有业务请求都走 `Chain::send` → `service::handle(&mut db, &config, epoch, e_epoch, &json!({}), &envelope)`（tb03c_chain_tests.rs `fn send`）。`rg "persist\(|reduce|finance::"` 在测试文件里只命中文件头注释，**没有直调 persist、reducer 或 finance 内部函数**。
- service.rs:1428-1435：`PrepareDecision|CommitDecision|AbortDecision`（B 域）与 `ImportResponsibility|Reserve|CloseApplicants`（E 域）都分派到 `finance::write_with_ingress`；Capture 与 Resolve 走 service 自己的 `capture` / `resolve_evidence`，Apply 走 service 的应用路径。
- 测试另外直接调用的 `store::init/open/latest/recover/audit` 都是存储层的建库、读取与恢复入口，只用于开库、读图像和重开，不绕过写路径。
- S 信任根是 `/bin/sh` 桩加钉死的 sha256，经正式 reader 运输读取；报告与文件头都明写 TestOnly、不计真实 S。属实。

## (2) oracle 独立性与手算

`S1-oracle.py` 只读写死的 TestOnly 输入，不读任何产品输出；它复述的是产品口径，没有抄产品结果。实跑 `python3 S1-oracle.py` 输出 lock 30 / responsibility_total 50 / worst 6 / allocation {units 2, cash 26, commission 2} / K 80000→80028 / units 4→6 / cost_remaining 80028。逐项对产品口径手算：

- 锁额：finance.rs:426 `bound = quantity×limit_price + fee_tail` = 2×14+2 = **30**；不超权利 40。
- E 责任：baseline 20 + grant 30 = **50** ≤ cap 100 → Reserved（finance.rs:1083-1086 `total <= cap`）。
- worst：开局 4 + 计划 2 = 6 ≤ fixed_q 12。
- 成交：from 0 → to 2，2 股 × 单价 13 = 现金 **26**，佣金 **2**；单参与重 alpha 独得全部增量。
- K：tb03c_book.rs:316-335 `K = acquisition_cash + commission + fee_only − disposal − dividend ± transfer`；开局 acquisition_cash 80000（book.rs:487 `book::tests::document` 的 known_cost_components）→ 80000+26+2 = **80028**。
- 仓：4 → **6**。
- cost_remaining：tb03c_book.rs:372-375 多头返回 `Some(K)`，与 ADR 0036「五、多空统一 cost_remaining：多头 = K」一致 → Known 80028。
- 估值观察：tb03c_book.rs 中 apply 对 `valuation_observation` 返回 `SchemaUnsupported：估值观察只记 E…`，K 不变。

全部成立。

## (3) 负控与重开

具体判据到位的：缺口成交 Resolve 用 `err_has("MissingDependency","前沿缺口")`，命中 tb03c_allocation.rs:300 `MissingDependency：净额累计前沿缺口、倒退或重复冲突`。由于缺口成交已预列在 baseline，它能通过覆盖核验，失败只能落在前沿判据上，隔离成立。反证测试用 `err_has("MissingDependency","未被已核责任界覆盖")`，命中 finance.rs:442。冲突 Capture、二次 evidence 走 `IdentityConflict` 前缀，估值 Apply 走 `SchemaUnsupported` 前缀。每个负控后都断言 `store::latest` 整体不变（无提交）。

**LOW-1**：错 `expected_fact_hash` 的 Apply 只断言 `kind == "Error"`，没有锁类型和文案；产品应得 `IdentityConflict：实读E事实/金额/归属与请求不匹配`（service.rs:1049-1053）。紧接着的正确请求成功，差分上能说明失败来自 hash，但单看这条断言，任何错误都能让它通过。两处 Resolve 前 Apply 只锁了 `AwaitingDependency` 前缀，实际文案是 tb03c_allocation.rs:393/396 的「净额事实尚无 E 持久分配」，建议加 `contains`。这两处都与 1fc8453540「负控断言收紧到具体失败原因」的自述不完全相符。

重开：`c.dbs.clear()` 关掉全部旧连接，然后 `store::recover(&config)`（断言 `kind=Recovered`，writer_epoch 严格递增），再用 `store::open` 开新连接并 `store::audit`。公共查询的前后快照分别取自旧连接与新连接，逐值相等（ReadView 只去掉 writer_epoch）。重开后用新消息身份重送 Apply、旧 epoch 的 Commit 和 Capture，都只返回原收据，`applied` 数与 E 责任 50 不变。**确属新实例读持久层。**

## (4) F1 / F2 事实

**F1 属实。**

- tb03c_e.rs:198-210：先用请求自带的 covered 跑一遍 `covered_pending`；FIN 域再对 `responsibilities[*].source.covered_facts` 汇总后跑一遍。
- finance.rs:923：只有 ImportResponsibility 推入的责任带 `"source": d["baseline"]`。
- finance.rs:1093：Reserve 推入的 grant 责任只有 `{responsibility_id, owner, amount, grant}`，没有 `source` 字段，因此被 `filter_map` 跳过。
- 反证测试里请求自带的 covered 与 Pending 匹配，第一遍会通过，失败只来自第二遍的已保留责任集。它锁住的是现状本身，没有掩盖：E 责任 50 断言证明 grant 30 确实在 E 里，Resolve 仍被拒。测试注释写明「记录现状，若日后接线改变应同步改此测试」。

**F2 属实。** tb03c_allocation.rs:97-110 对 participant 的 `command_id` 只做 `required`；:349 原样复制进 allocation。`rg "decision|grant|finance" tb03c_allocation.rs` 零命中，即没有任何与决定或 grant 的绑定，也没有成交量对 Commit 数量的比较。报告注明「未单测」，属实。

**MEDIUM-1（夸大措辞，需改）**：报告正文对 F1/F2 的陈述准确，但测试文件里有三处措辞会让后来者把这条测试当成「Commit 引出成交、成交回指 Commit」的产品证据：

- tb03c_chain_tests.rs:2 文件头写 `Capture(该 Commit 对应成交)`；
- :405 写 `// ---- 该 Commit 对应的成交：E.Capture ----`；
- 归属段注释写「command_id 即被 Prepare/Commit 的计划身份」，并断言 `allocation.command_id == committed.prepared.plan.plan_id`。

这个「对应」完全由夹具构造：成交覆盖是预列在冻结 baseline 里的（F1），`command_id: "plan-alpha"` 是夹具写进 fill_manifest 的（F2）。产品不建立这层对应，所以该断言是同义反复。测试名本身没有「驱动」「反馈」字样，可以不改名。建议：把 :2 与 :405 改成「Capture 一笔预列于冻结输入、command_id 由夹具填为 plan-alpha 的成交（对应关系由夹具构造，产品不强制，见 S1-REPORT F1/F2，归 TB04-F）」，并在 command_id 断言旁注明「夹具自洽检查，非产品绑定（F2）」。报告「测试覆盖 → 归属」一条也宜加同样的限定。改动纯属注释和文档，不需要重跑。

## (5) 未签本金 / 增长

tb03c_book.rs:606-617：`principal.status` 只在存在授权本金批时为 Known，否则为 `NotEvaluable`，reason 为 `no authorized principal batch`。`growth_enabled` 在非 Eligible/NotEligible 时为 `NotEvaluable`（principal batches absent）。夹具没有任何本金批，也没有 θ 或增长政策字段，测试断言两者都是 `NotEvaluable`。**无默认填值，正确。** `cost_remaining` 断言 Known 80028 是多头 = K 的口径，与本金无关，同样正确。

## 其它

- LOW-2：worktree 根目录有一份未跟踪的 `S1-REPORT.md`，与入仓版逐字相同（`diff -q` 无输出），属残留副本，建议删除，避免两处漂移。
- 1c479a91b9 能编译但含一条错误断言，已由 872121783e 更正，报告已自陈；是否压扁历史由主控决定，本评审不作要求。

## 发现汇总

- HIGH：无。
- MEDIUM-1：测试注释「该 Commit 对应成交」与 command_id 归属注释夸大了产品语义（实为夹具构造），需按上文改措辞。
- LOW-1：错 hash Apply 只锁 `Error`；Resolve 前 Apply 只锁前缀。建议锁类型与文案。
- LOW-2：worktree 根有未跟踪的报告副本。

## 复审（e14fed41ee，2026-10-21）

结论：**APPROVE**。

范围：`git diff --stat c793b4c9a5 e14fed41ee` 只有 `S1-REPORT.md`（+4/−4）与 `tb03c_chain_tests.rs`（+14/−6）；`git diff --stat 6c8477ff6f e14fed41ee` 仍只有 mod.rs（+2 测试模块声明）、测试文件、报告、oracle 四个文件。**生产码零改动，无越界。**

- **MEDIUM-1 关闭**：文件头 :2 去掉「该 Commit 对应成交」，改为明写成交预列于冻结 FIN baseline、command_id 由夹具填为 plan-alpha、对应由夹具构造、产品不强制，见 F1/F2，归 TB04-F #1425；原 :405 段注释同样改写；`command_id == plan_id` 断言前注明「只核夹具值经 Resolve 原样传递不变（夹具自洽检查，非产品绑定…见 F2）」。报告「归属」小节标题加了同一限定。`rg "对应成交|该 Commit 对应"` 不再有未限定的表述。断言本身未改，没有删测或放宽。
- **LOW-1 关闭**：新增 `err_exact`（kind==Error 且 error 全文相等）。错 hash 的 Apply 锁 `IdentityConflict：实读E事实/金额/归属与请求不匹配`（与 service.rs:1053 逐字一致）；两处 Resolve 前 Apply 锁 `AwaitingDependency：净额事实尚无 E 持久分配`（与 tb03c_allocation.rs:393/396 逐字一致）。报告负控表同步更新。
- **LOW-2 关闭**：worktree `git status --short` 为空，根目录未跟踪的报告副本已删。

实跑：`NC_TARGET=target-1406-review ~/.local/bin/nc-remote codex/1406-s1-chain rust cargo test --features s_session --lib tb03c_chain` → sha e14fed41ee，`2 passed; 0 failed`（log `/tmp/nc-remote/20261021-001741-e14fed41ee.log`）。改动只涉测试断言与注释，未重跑 economic_session 全量（原审 c793b4c9a5 上 157/0/8）。
