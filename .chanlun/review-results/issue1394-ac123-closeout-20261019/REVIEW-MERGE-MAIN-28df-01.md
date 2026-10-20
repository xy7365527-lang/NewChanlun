# REVIEW-MERGE-MAIN-28df-01：#1394 合入 main 28df886697 的 merge 提交 2a38b56fb0

- 审查者：独立审查（非作者），只读；未改代码、未推送、未合入、未关票。
- 被审：`2a38b56fb0`（父1 `7e914f1e55` = 已审 #1394 head；父2 main `28df886697`；merge-base `5baeb030a6`）。
- 工作副本：自建 `git worktree add --detach /tmp/rv1394 2a38b56fb0`（`/Volumes/AgentStorage/issue1394-b46-closeout-wt` 仅只读查阅，HEAD 亦为 2a38b56fb0）。
- 限制：本机禁 cargo，**Rust 部分全部为静态审查，未编译、未跑 cargo test**；JS/Python 跑了轻量测试（见 §F）。

## 0. 方法

1. 对 `git diff 5baeb030a6 28df886697` 中的每个代码文件分类：若 #1394 分支未动（br-untouched），核合并结果与 main 逐字节相同；若两侧都动，分别对照 main 侧 hunk（`5baeb030a6..28df886697`）与合入分支侧 hunk（`7e914f1e55..2a38b56fb0`）的 +/- 行集合差。
2. 对作者的每处冲突解法，逐条复核语义。

文件分类结果：

| 文件 | merged 与 main | 分支是否改过 |
|---|---|---|
| formal/Phase2/Claim10_SegmentV1.lean、rust/src/bin/s_session_v2/tb02c.rs、theta_v0/backtest/incremental.rs、theta_v0/config.rs、parser/{feature_seq,second_fold,second_kind,segment,tb02w_tests}.rs、s_session/tests/{tb02w_browser.cjs,tb02w_runtime.py,test_tb02w_contract.py} | 逐字节相同 | 否 |
| .github/workflows/ci.yml、rust/src/bin/s_structure_session.rs、parser/mod.rs、browser/index.html、browser/tb01c-client.js、s_query_integrity.py、s_tb02_contract.py | 不同（逐项见下） | 是 |
| s_session/tests/test_tb02w_contract.cjs | 不同（见 §C） | 否（合入时改） |

## A. #1420 / #1571 行为是否保留

### A-总：s_structure_session.rs 的 hunk 对账
main 侧 `+` 行在合入侧缺失的只有：旧 `RULE_REVISION` 注释、`open_historical_readonly` 的单规则判定、`advance_core` 中单路径解析的那几行、hot-wal 列表中的 `"s2-axis-quantifiers+new-bi/2"`。合入侧多出的正是作者说明里的四处改写，此外没有别的增删。#1571 的 5 个测试（`tb1405_*`）、#1420 的 `design_rows/…/tb02w_killed_case_two_scan_budget_resume_and_recover`、`RULE_REVISION = …+new-bi/2+segment-second/2`、`cmd_pragma` 注释、`recover_core` 不建库门都已原样进入。`tb02c::build(..., &second_store)?` 调用形状与 main 一致。

### A(1) open_db / open_historical_readonly 把 v2 `RULE_REVISION` 与 v3 `V3_RULE_REVISION` 都视作当前规则：**正当**
- v3 写者确实经 `crate::open_db` 开库：`s_session_v3/audit_owner.rs:125 AuditConnection::open → open_db`，`driver::open_*` 与 `driver::recover` 都走这条路。如果只认 v2 规则，v3 当前库会拿到只读连接，S3 serve/advance/recover 写入都会失败，所以必须豁免 v3 规则。
- 豁免是**精确匹配两条字符串**（`matches!(rule, Some(RULE_REVISION) | Some(V3_RULE_REVISION))`），没有前缀匹配。
- 真旧规则库不会因此拿到写连接：查过 v2 `RULE_REVISION` 的全部历史取值（`git log -G'const RULE_REVISION'`）：`s2-axis-quantifiers` → `+new-bi/1` → `+new-bi/1+segment-first/1` → `+new-bi/1+segment-second/1` → `+new-bi/2+segment-second/1` → `+new-bi/2+segment-second/2`。**v2 写者从来没有写过 `+new-bi/2`**，所以不存在“v2 旧库恰好等于 V3_RULE_REVISION”的情况。`LEGACY_V3_RULE_REVISION`（`+new-bi/1`）以及上面所有旧值都走只读连接。
- v2 入口打开 v3 当前库时会拿到写连接，accept/advance/recover 随后用 `!= RULE_REVISION` 拒绝（VersionMismatch）。这和合入前分支的行为相同；v3 库属于活库，不在 #1571“旧规则库字节不变”的承诺范围内。
- 对 v3 旧规则库的副作用：`driver.rs historical_bi_rule_is_readable_but_writer_and_recover_leave_it_unchanged` 现在会拿到只读连接。`driver::recover` 走 `begin(Immediate)`。我用 Python sqlite3（3.53.4）验证过：只读连接上 `BEGIN IMMEDIATE` 成功，只有真正写入时才报 readonly；而 `require_current_writer_rule` 在任何写入之前触发。`startup` 走 Deferred。按静态推理，测试断言（含 `VersionUnsupported`）仍成立。**未经 cargo 实证**，属残余风险。
- #1571 hot-wal 测试列表把 `+new-bi/2` 换成 `+new-bi/2+segment-second/1`：**正当，而且更贴题**。在合入后的语义里，`+new-bi/2` 是 v3 当前规则，按上一条它应该拿写连接，留在“旧规则库不得改写”列表里会自相矛盾。`+new-bi/2+segment-second/1` 是 v2 真实的上一代规则，main 原列表反而漏了它。代价是 v2 入口在 v3 当前库上的拒绝行为不再有测试覆盖（见 L1）。

### A(2) recover_core 规则门移到 ensure_writable 之前：**正当**
顺序为 `!db.exists()` → `open_db` → `ensure_initialized` → **规则门** → `ensure_writable` → `record_connection_pragmas` → 事务，与 main 的相对位置一致（main 没有 ensure_writable）。规则门放前面的好处：旧规则库（只读连接）先得到 VersionMismatch，与 #1571 测试断言一致；而且规则门只读，不碰写。`evidence_store::format` 也只读（带按连接的捕获缓存），放后面没有副作用。附带一点不一致：accept/advance 仍是 `ensure_writable` 在规则门之前（分支原状，不是这次合入引入的），见 L3。

### A(3) advance_core 让 K1 游标簿穿过 oracle 与共享证据两条路径：**同一段核，续扫/恢复语义一致**
- `SecondFoldStore::new(tb02c::round_budget())` 和 `tb02c::restore_cursors(&read_active_objects(&conn)?, …)` 只在 `is_ohlc` 时执行，与 main 相同（main 在 is_ohlc 分支内执行；合入版提到分支外，但同样受 is_ohlc 守卫，效果等价）。之后游标簿 move 进两条路径中实际执行的那一条，再取回传给 `tb02c::build`。
- `parser/mod.rs`：`parse_layer_with_group_evidence_store` → `parse_layer_with_group_evidence_inner(…, &mut Some(store))`。
  - 降级分支调 `parse_layer_from_facts(&mut facts, &config.parse, store)`，就是 main 的 `parse_layer_with_inclusion_facts_store` 内部用的同一函数。
  - 非降级分支调 `parse_layer_from_strokes(…, store)`，`match store.take()` 时调 `segment::divide_segments_with_facts_store(&strokes, config, s)` 并回填，和 main `parse_layer_from_facts` 中的 match 逐字同构。
  - 两条路径都只经过 `divide_segments_with_facts_store` 这一个段核，笔序列来自同一组完整新笔事实（既有测试 `group_evidence_matches_complete_facts_for_every_prefix` 锁定了非 store 版本下两路径的 layer、groups、stroke_facts、segment_facts 逐前缀相等）。
- 取回方式：main 用 `slot.unwrap_or_default()`，合入版用 `slot.expect("K1 段核保留游标簿")`。`divide_segments_with_facts_store` 总会归还游标簿，所以 expect 不会触发，而且比 default 更严（不会静默丢游标）。
- 续扫/恢复：游标只来自已发布对象（CC-056），lane 缓存只缓存包含/笔状态，段划分每 cut 全量重算，因此续扫和 recover 后的再推进同样经过“已发布游标 → store → 同一段核”。main 的 `tb02w_killed_case_two_scan_budget_resume_and_recover` 在合入版默认走增量（共享证据）路径，正好覆盖这条新接线（前提是 cargo 能跑通）。
- v3 `generation.rs:490` 仍用无 store 的 `parse_layer_with_group_evidence`。v3 只发布 TB-02-A/B，不消费段事实，没有影响。

### A-其它文件
- `parser/mod.rs`：main 的 `pub mod second_fold`、`parse_layer_with_inclusion_facts_store`、`parse_layer_from_facts(…, store)`、`mod tb02w_tests` 都在；#1394 的 `evidence`、`GroupParseEvidence`、`parse_layer_with_group_evidence` 及前缀对拍测试都保留。
- `ci.yml`：main 的两个 TB-02-W 步骤（python unittest、node --test）原样进入，只是位置放在 #1394 的 Q3 lock 之后；#1394 步骤也都保留。
- `index.html`：logical slot 加 `s-segment/3`，与 main 同义（分支 index.html 已整体重排格式，只改这一处）。
- `s_query_integrity.py`：`_published_axes` 加 `+new-bi/2+segment-second/2 → tb02.AXES`，与 main 同义。分支的“未声明规则即拒绝”保持不变。
- `s_tb02_contract.py`：main hunk 与合入 hunk 的 +/- 行集合**完全相同**（各 291 行 diff，集合差为空）。

## B. JS

- **q3-typed.js 的 s-segment/3 校验与 main `tb01c-client.js` 运行段逐条相同**：取 main `tb01c-client.js` 第 240–489 行（`SECOND_FOLD_CURSOR` 至 validateSegmentPayload 末尾）与 q3-typed.js 第 236–505 行，去掉行首缩进后 diff，唯一差别是 `function validateSegmentPayload` 改成了闭包里的 `return function validateSegmentPayload` 加闭包收尾，判据行零差异。闭包内的 `integer/signed/direction/keys/canonical/equal` helper 与 main 同义（`integer(v, positive, frontier)`、`signed` 返回 BigInt）。`RULE_CONTRACTS` 加了 `+new-bi/2+segment-second/2 → [s-new-bi/2, s-segment/3, KNOWLEDGE_DOMAINS[1]]`，与 main 一致。
- **q3-consumer.js 覆盖 main validateTB02Candidate 的改动**：main 对 validateTB02Candidate 共改三处：① byRule 加 segment-second/2 → TB02_AXES；② 段筛选扩到 `["s-segment/2","s-segment/3"]`，序列 schema 改为“等于段的 schema”；③ s-segment/3 下 KILLED/PENDING_CASE_TWO 绑 SECOND 终态。合入版分别落在 `RULE_AXES`（q3-consumer.js:715）和 `bindSegmentSequences`（:1420–1433），三处逐字对应；logical slot（:881）也加了 s-segment/3。
- **目录证据 scope 维持 TB-02-D**：main 写者 `tb02a_facts.rs` 写 `scope = "TB-02-D"`（main :445，合入 :1041），deferred 仍含 `general_scan_continuation`。main 的 Python（s_tb02_contract.py:398/417）与 JS（tb01c-client.js:726/735）也只认 TB-02-D。合入版 Python 判据同上，q3-consumer.js:759/801 要求 scoped=TB-02-D 并精确比对 delivered/deferred 子义务。三方一致，没有留下漏校验。CC-056 的**语义槽**在 v3 下为 `TB-02-W`（q3-typed.js:483、s_tb02_contract.py:966、tb02c.rs:361 三方一致），这是另一回事，不要和目录 scope 混淆。
- 参考线 `codex/1405-1420-combined-20261015` 的 d5b9d01814（“资源截断缺有限扫描预算”“SECOND游标消费超过扫描预算”）不在 main 中（`merge-base --is-ancestor` 为否），合入版与 main 同样没有。不采纳是对的，见 INFO-1。
- `node s_session/tests/build_tb01c_browser.cjs --check` → `"status": "passed"`（bundle_sha256 `5b114403…23a6`）。

## C. test_tb02w_contract.cjs 第 4 例改走公开 `API.Q3.validateState`：**断言强度保留（略增强）**
- 断言逐字未改：4 个正例、1 个 v2 正例，以及 5 个 `assert.rejects` 的 `message` 精确匹配（`段结果未绑定SECOND终态`×2、`段事实可知性未绑定SECOND扫描证据`、`段引用缺同cut序列对象`、`段合同与cut规则版本不符`）。
- 拒因来源唯一：在 q3-*.js 中 grep，前三条只出现在 `q3-consumer.js` 的 `bindSegmentSequences`（:1426/:1431/:1433），`段合同与cut规则版本不符` 只出现在规则→schema 绑定（:1377）。message 精确匹配加上来源唯一，可以确定拒绝确实由同 cut 绑定或规则绑定产生，而不是被别的早期校验截走。
- 正例现在要过完整 `validateState`（目录、计数、逐对象 typed 加跨对象绑定），比原来调私有 `validateTB02Candidate` 更严。新增的 `fullState` 是人工合同域，不当作生产证据，注释里已写明。
- `node --test s_session/tests/test_tb02w_contract.cjs`：5/5 pass。

## D. 旧规则 `s2-axis-quantifiers+new-bi/2+segment-second/1` 可读性：**一致**

| 位置 | 取值 |
|---|---|
| tb02a_facts.rs `CatalogAxisTable::for_rule_revision` | SegmentSecond（17 轴） |
| tb02b.rs `contract_for_rule` | CURRENT_CONTRACT（s-new-bi/2） |
| source_pending.rs `equal_is_evidence`（经 contract_for_rule）及其测试 | true（新笔 /2 同价为证据） |
| s_tb02_contract.py `RULE_CONTRACTS` | (s-new-bi/2, s-segment/2, KNOWLEDGE_DOMAIN_V2) |
| s_query_integrity.py `_published_axes` | tb02.AXES（17 轴） |
| q3-typed.js `RULE_CONTRACTS` | [s-new-bi/2, s-segment/2, KNOWLEDGE_DOMAINS[1]] |
| q3-consumer.js `RULE_AXES` | TB02_AXES（17 轴） |

七处在“17 轴 / 新笔 /2 / 段 /2 / 域 V2”上全部一致；新规则 `…segment-second/2` 在各处同样对应 17 轴 / s-segment/3。

## E. HIGH / MEDIUM
**无。**

LOW / INFO（不阻断）：
- **L1（测试覆盖）**：hot-wal 列表去掉 `+new-bi/2` 后，“v2 入口（accept/advance/recover）在 v3 当前规则库上返回 VersionMismatch”不再有直接测试。行为本身由 `!= RULE_REVISION` 门保证，建议补一个小例锁住 V3 豁免的边界（v3 可写、v2 入口仍拒绝）。
- **L2（测试覆盖）**：`parse_layer_with_group_evidence_store` 与 `parse_layer_with_inclusion_facts_store` 的逐前缀对拍没有 store 版本（只有非 store 的 `group_evidence_matches_complete_facts_for_every_prefix`）；有限预算下 oracle 路径也没有端到端用例。静态上两者是同一段核，建议补一个带 `SecondFoldStore::new(Some(1))` 的前缀对拍。
- **L3（一致性，分支原状）**：recover 先规则门、后 `ensure_writable`；accept/advance 先 `ensure_writable`、后规则门。旧规则且为 evidence-graph/2 的库，在 accept/advance 上报的是格式只读错，而不是 VersionMismatch。两种都是拒绝，不影响安全。
- **INFO-1**：d5b9d01814 的两条预算校验 main 也没有，属于 main 侧合同缺口，不是本次合入引入的，也不在本次合入范围内。如需要可在 #1420 另行跟进。
- **INFO-2（残余风险）**：Rust 未编译、未跑测试。合入前须由 CI 或作者在允许 cargo 的环境跑 `s_structure_session` 二进制测试（重点：`tb1405_*`、`tb02w_killed_case_two_scan_budget_resume_and_recover`、`driver::historical_bi_rule_is_readable_but_writer_and_recover_leave_it_unchanged`、`parser::tests::group_evidence_matches_complete_facts_for_every_prefix`、`tb02w_tests`）。`gh run list --commit 2a38b56fb0` 目前没有运行记录。

## F. 实跑记录（/tmp/rv1394 @ 2a38b56fb0）

| 命令 | 结果 |
|---|---|
| `node s_session/tests/build_tb01c_browser.cjs --check` | passed |
| `node --test s_session/tests/test_tb02w_contract.cjs` | 5 pass / 0 fail |
| `python3 -B -m unittest test_tb02w_contract`（cwd s_session/tests，Python 3.14.6） | 14 OK |
| `unittest discover -p test_tb02b_contract.py` | 15 OK |
| `unittest discover -p test_tb02d_contract.py` | 14 OK |
| `unittest discover -p 'test_tb02a_*.py'` | 9 OK |
| `test_tb02d_validate_once.py` / `test_tb02d_key_reuse.py` | 9 OK / 6 OK |
| `PYTHONPATH=s_session unittest test_s_evidence_graph_integrity` | 22 OK |
| `node s_session/tests/test_tb01c_browser.cjs` | failed: [] |
| `node --test` test_tb02b_contract.cjs / test_tb02c_contract.cjs / test_q3_v3_validate_once.cjs / test_q3_complete_client.cjs | 9/6/1/23 pass，0 fail |
| `test_tb02c_*.py` | 1 个导入错误（本机缺 `ijson`，环境问题，CI 会先 pip install） |
| test_tb02d_strict_clone.cjs / test_tb02d_snapshot_validate_once.cjs / test_tb02d_projection_cache.py | 需要 baseline 或运行时参数（不在 CI 中），未计入 |
| cargo | 未运行（本机禁止） |

## 结论
#1420（PR #1569）与 #1405 恢复 v4（PR #1571）的行为在合并结果中逐项保留，没有丢失。作者的四处适配（双写者当前规则豁免、recover 规则门前移、K1 游标簿穿过两条解析路径、hot-wal 列表换成真实上一代 v2 规则）在语义上都正当。JS 段合同与 main 运行段逐条相同，旧规则 `+new-bi/2+segment-second/1` 在七处一致可读。未发现 HIGH/MEDIUM。附带条件：Rust 部分须经 CI 或作者跑过 cargo test 后再合入 main（INFO-2）。

VERDICT: PASS
