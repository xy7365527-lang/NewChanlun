# #1604 第二轮非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1663，分支 `sandcastle/issue-1604`，范围 `git diff 35604dadf7..645600f4e8`（第一轮三条 MEDIUM 与 L1/L5/L6 的处置提交 `1466874397`、`c33f823696`、`d85a81cd52`、`e0943df381`、`742c489106`、`645600f4e8`）。
- 作者：本轮修补由 Codex 子代理（gpt 系）完成；审查者 `claude-fable-5-1` 经 `~/.local/bin/claude-proxy`（Claude Code CLI，本机 CLIProxyLocal 代理），非作者。
- 工具限制：Read/Grep/Glob 与只读 git/sed/grep/rg；未跑 cargo/pytest/node，未调 gh。审查提示原文：`prompt-r2.md`。日期：2026-10-21。
- 处置（作者侧，本记录之后）：LOW-3 已修（`scripts/segment_engine_diagnosis.py` 改读记录）；MEDIUM-1（`s-segment/4` 不升 schema/rule_revision 改必填值）、LOW-1、LOW-2、LOW-4 登记于 `.chanlun/review-results/issue1604/IMPLEMENTATION.md` §7，由用户裁定；`docs/spec/segment_rules_v1.md` 属推论层历史文档，按 ADR 0012 不追溯改写。

---

结论：PASS

## 三条 MEDIUM 处置

**M1（S 写者预算戳与常数）— 已处置。**
- 三处输出都来自本次所读记录：`tb02c.rs:255` SECOND `budget.policy` 写 `cursor.budget_policy.label()`，`:397` `scan_budget.policy` 写 `store.policy.label()`，K1 线形 `second_fold.rs:496` 写 `self.budget_policy.label()`；三者的源头都是 `SecondFoldStore::from_parse_config`（`second_fold.rs:619-621`）从 `ParseConfig::scan_budget` / `second_seq_scan_window` 取值，而段事实的 `SegmentPolicy::from_config`（`segment.rs:402-410`）读的是同一个 `ParseConfig`（`s_structure_session.rs:3080/3124/3134` 同一个 `config`）。
- `ROUND_BUDGET`、`SECOND_SEQ_SCAN_BUDGET_POLICY` 全仓（`rust/`、`s_session/`、`src/`、`scripts/`）已无代码定义；剩余的 `second-seq-scan-budget/1` 字面只在负控测试（`tb02w_tests.rs:1351`、`test_tb02c_turn_contract.py:202`）、`s-segment/3` 历史合同测试、历史 receipts 与评审记录里。`TEST_ROUND_BUDGET` 仍在但 `#[cfg(test)]`。
- 游标簿与段事实一致性：`tb02c.rs:203-209` 比对 `store.policy.label()` 与 `facts.policy.scan_budget`，不同即 `Err` 拒写，不互补。K1 跨版本：`from_wire`（`second_fold.rs:531-540`）把线形 `budget_policy` 与本次 `PolicyRef.label()` 对照，不同返回 `None` → `fold` 走 `SecondFoldCursor::new(candidate, self.round_budget, self.policy)`（`:665`），即自 b 笔按本次记录重折叠，不沿用旧线形的累计预算；按 K1 等价条件结果与无中断折叠相同，这是拒读后重算，不是回退常数。负控 `tb1604_k1_wire_with_other_budget_version_is_rejected` 三路（同版本可读 / 本次记录为 v1 拒 / 线形写 v1 拒）成立。
- `SecondFoldStore` 去掉 `Default` 后两处归还路径（`segment.rs:987-993`、`parser/mod.rs:316-320`）：我追了 `slot` 的生命周期——`divide_segments_core` 的两条早返回（`:1014`、`:1027`）发生在 `store.take()`（`:1039`）之前，主路径末尾 `*store = feat.take_second_store()`（`:1150`）必还；`parse_layer_from_facts` 的两处（`mod.rs:481-485`、`:530-534`）也都 `*store = Some(s)`。所以 `unwrap_or_else` 分支实际不可达，可达路径行为无变化；原 `unwrap_or_default()` 若可达反而会把 `round_budget` 清成 0 且无记录版本，新写法更对。
- 第二序列 `tail_cache_window` 改读 `second_seq_window`：逐位不变（签署值 0），常数确实消掉了，但语义上有保留意见（见 LOW-1）。
- Python 合同 / `q3-typed.js` / 重生成的 `tb01c-client.js` 三处 `_budget_policy(v4)` / `budgetPolicy(v4)` 分档对称：`s-segment/4` 只收 v2，`s-segment/3` 只收 v1，按 `schema_revision` 分派且各自严格，不是「严格判不过换宽松再判」的 if-else 兜底，不构成 #799 宽严两档。`test_tb02c_turn_contract.py:200-204` 对 v1/v3 Python 与 JS 同拒（`judge` 同时跑 Python 校验器和 node 捆绑）。历史兼容代价在 IMPLEMENTATION.md §7.2 与 PR 描述「兼容代价」如实写明，但由此引出一个新 MEDIUM（见下）。

**M2（Python v0 相切 `<`→`<=`）— 已处置，且正本确实要求。**
- `xianduan.md:262` §6.3 默认值 `closed`，`:386` 明写「P1 与 Lean claim5 写死的 `<` 是未跟残留，改读字段归 §15 X-38」，`:524` X-38 的 Python 栏列出 `a_segment_v0.py:164`（P1 写死 `<`），处置「P1 与 claim5 的 `<` 改读字段」，分派「Rust 与 Python → #1604」。所以改 v0 在票面范围内，不是越界。
- 新锁 `test_v0_signed_record_counts_tangency_as_overlap`（`tests/test_segment_policy_1604.py:99-106`）不打桩：先断言生效记录 `tangency == "closed"`，再在 `TANGENT`（`[5,10]/[0,10]/[0,5]`，`max(lows)==min(highs)==5`）上要求 `v0.segments_from_strokes_v0` 给出恰好 `[(0,2,"up",10,0)]`。改回严格 `<` 的变异体给 0 段 → 红。同时锁住 `ab_bridge.segments_from_strokes_v0 is v0.segments_from_strokes_v0`。实现侧 `a_segment_v0.py:158/168` 确实经 `rule.meets(...)` 读记录。
- `/tmp/fix1604/pr-body.md` 的影响面描述（只在单点相切时变；`ab_bridge_newchan` 仅 `segment_algo="v0"` 受影响，默认 v1 不变；「逐位相同」对 v0 相切输入不成立且是裁定要的）与代码一致。

**M3（分类器记录戳与尾窗截断标记）— 已处置。**
- `pipeline.rs:496-511` `TowerPolicyStamp` 加 `scan_budget` / `construction_rule`，取自 `config.parse.scan_budget` / `construction_rule`，研究覆写时随 `PolicyVersion::ResearchOverride` 标 `…/research-override`（`tb1604_classifier_output_stamps_segment_records` 两路都锁）。
- `SegmentCoverage::of`（`:437-445`）读 `ParseLayer::segments_resource_bound_from`，`classify_incremental` 在拒读判定之前算好（`:616`），两条返回路径都带（`:627`、`:640`）；`classify` 是 `classify_incremental` 的薄包装（`:534-537`），所以全量/增量/拒读三路一致。「只标记不拒读」与 #1603 对 `termination: ResourceBound` 的处理同口径，`is_segment_resource_bound()` 与 `is_resource_truncated()` 并列，不算静默——段层本身已把截断段与正常确认区分开，classifier 把同一标记和起点带出即可。
- `tb1604_classifier_surfaces_tail_window_bound`：正控 1200 根 bar 逐根要求 `Complete`；注入标记后要求 `IncompleteResourceBound{from, budget}`、`code()=="INCOMPLETE_RESOURCE_BOUND"`、增量入口同一标记、叠加段端门标记的拒读路径仍保留标记，另锁「注入前后 `classification` 逐位相同」。改回「恒 `Complete`」的变异体在 `:289` 必红；把 `from` 丢掉的变异体也红（`from` 取中段起点）。
- 与 #1662 的冲突面：本 PR 在 `pipeline.rs:619-628` 这个拒读早返回结构体里加了 `segment_coverage` 字段，`termination` 行（`:625`）未动；若 #1662 把该行改成 `ParseInvalid`，文本冲突是同一结构体相邻行，语义正交，描述属实。**#1662 本身我无法核实**（本地 `main` 上没有，不能调 `gh`）。

## 新发现

**MEDIUM-1：`s-segment/4` 合同在不升 `schema_revision` / `rule_revision` 的情况下改了必填值。** `s_session/s_tb02_contract.py:569-574`、`q3-typed.js:247`：同一个 `s-segment/4` 对象，改前合法的 `second-seq-scan-budget/1` 现在非法。`s_structure_session.rs:102-103` 的纪律是「旧规则会话只读（AsKnown），同一 rule_revision 下不混两代笔/段对象」，而 #1493 的先例是合同变动即升 `s-segment/3→/4` 并给 rule_revision 加 token。现在一个已有库里会出现同一 `rule_revision` 下两代 `s-segment/4` 对象（旧二进制写的带 v1、新二进制写的带 v2），旧对象再经新校验器即红。作者在 IMPLEMENTATION.md §7.2 与 PR「兼容代价」里如实登记并说明「活动对象每代重算、预算 0 下 K1 检查点本就为空」，这把实际爆炸面压得很小，但这是纪律层面的偏离，应由用户裁定是接受登记、还是升 `s-segment/5`（或给 rule_revision 加预算 token）。本条不阻塞，与第一轮对 M1 的处理一致。

**LOW-1：SECOND 的 `tail_cache_window` 写 `second_seq_window` 是语义借用。** `tb02c.rs:252`。FIRST 的 `tail_cache_window` 是主序列尾窗（标准元素缓存），SECOND 没有尾窗，`second_seq_window` 是每候选扫描预算，已经以 `budget.granted` 出现在同一对象里；并且 `s_tb02_contract.py:879` 仍硬要求 SECOND 的 `tail_cache_window == "0"`——记录一旦签成非 0，S 写者会写 N、合同当场拒绝（fail-loud，不是静默，但合同与「读记录」的说法互相打架）。建议后续要么改字段名/口径说明，要么合同改读记录。

**LOW-2：`TEST_ROUND_BUDGET` 覆写只改值不改版本标签。** `tb02c.rs:33-38`：测试覆写把 `store.round_budget` 改成 w，但 `store.policy` 仍是签署的 `second-seq-scan-budget/2`，输出会以签署版本之名写出非记录值；与 `ParseConfig::research_override_*` 的「改值即标 research-override」约定不一致。同时 `:203` 的一致性检查只比标签不比值，所以这种标签同、值异的状态能通过。仅 `cfg(test)`，不影响生产，但是条约定上的小口子。

**LOW-3：删 `_FeatureSeqState.TAIL_WINDOW` 留下悬空引用。** `scripts/segment_engine_diagnosis.py:686/688/696/713` 仍读 `_FeatureSeqState.TAIL_WINDOW`，一跑到那段就 `AttributeError`（248 谱系诊断脚本，依赖 yfinance，不在 CI）；`docs/spec/segment_rules_v1.md:155/161` 仍把它写成类常数。

**LOW-4：提交 `c33f823696` 的主题以 `fix(segment):` 开头。** 关键字后面紧跟的是 `(segment):` 而不是 `#1604`，不会触发 GitHub 自动关票；但票面明说不写 fix/close/resolve，换 `refactor`/`feat` 更稳。其余五个提交与所有提交的编号声明均齐全（`742c489106` 把 #1662 标为 PR）。

**观察（不计缺陷）**
- L1 的 Python 解析器 `segment_policy.py:122-150` 与 Rust `parse_second_seq_scan_budget` / `parse_header` / `read_u32` 口径逐条对过：v1 拒、`version` 缺/null/非正整数拒、字段缺/`null`/`"AwaitingPolicy"`/非对象/非 u32（含 bool、负数、超 u32）拒、id 不符拒；都不给默认值。负控参数化 8 例覆盖这些路径。
- Python v1 拒绝 `tail_window ∈ {1,2}`（`a_segment_v1.py:297-305`）是能力限制下的 fail-closed（Python 没有截断标记），阈值 3 来自记录 `options` 里「N ≥ 3 逐位同全扫」的论证，不是回退值；Rust 对同值是标记、Python 是拒绝，行为不对称但都不静默，且已登记。
- `tests/test_segment_tail_window_scan.py` 改写仍测原意：对照臂从「`TAIL_WINDOW=1000`」改为「记录 `tail_window=0`（不设上限）」，对 ≤31 笔语料等价；`start = max(1, last_checked, n - tw if tw else 0)` 的优先级正确（`tw=0` → 全扫）。
- `docs/reference-theta-v0.md:23` 新增段里的字段名（`SegmentFacts.policy` / `tail_window_bound`、`ParseLayer.segments_resource_bound_from`、`ClassifyOutput.policy.scan_budget` / `.construction_rule`、`ClassifyOutput.segment_coverage`、记录文件名与 v2 = 7 / 0）与代码一致。IMPLEMENTATION.md §7 新增 2/5/6 三条与代码事实相符，§8 删掉的两条确已做掉；「`segment-second/2` 早已在 rule_revision 内」经 `s_structure_session.rs:103` 核实。
- 编译面：所有 `SecondFoldStore::new` / `SecondFoldCursor::new` / `from_wire` 调用点都已改签名；`PolicyRef` 是 `Copy`，`self.policy` 按值传没问题；`ClassifyOutput` 仅一处构造；测试用到的 `tb1581_bars`（`pub(super)`）、`TowerCache: Clone`、`ParseLayer: Clone`、`Bar: Copy`、`classifier::SegmentCoverage` 再导出都存在。没看出编译错误迹象。
- PR 描述「验证」一节如实写了本机 pytest 仍有环境项失败，没有夸大。

## 看过什么
`git log/diff 35604dadf7..HEAD` 全部 21 个文件的改动；`second_fold.rs`、`segment.rs`、`parser/mod.rs`、`feature_seq.rs` 中 store/slot 的完整生命周期；`policy_records.rs` 的 `PolicyRef`/解析函数与 `second-seq-scan-budget.json`；`config.rs` `ParseConfig`；`tb02c.rs` `build`/`restore_cursors`；`s_structure_session.rs` `advance_core` 与 `run_design`；`classifier/mod.rs` 再导出与 `pipeline.rs` 全文相关段；`xianduan.md` §6.3、:386、X-38；AGENTS.md #799 通则；`delivery-discipline.md` 编号声明条；`a_segment_v0.py` 相切读取；两个 Python 测试文件全文；`s_tb02_contract.py` 的 `tail_cache_window` 要求；`/tmp/fix1604/pr-body.md`；全仓 grep `ROUND_BUDGET` / `SECOND_SEQ_SCAN_BUDGET_POLICY` / `second-seq-scan-budget/1` / `TAIL_WINDOW` / `SecondFoldStore` / `ClassifyOutput {`。

## 未核实
未跑 cargo / pytest / node（只读审查）；未重算 `tb01c-client.js` 捆绑段的 SHA256 与 `q3-typed.js` 是否一致（命令需审批，依赖 PR 描述的 `build_tb01c_browser.cjs --check`）；PR #1662 的实际内容（本地无该分支、不调 gh）；MEDIUM-1 里「已有生产库中是否真有带 v1 token 的 `s-segment/4` 对象」；`docs/reference-theta-v0.md` 新段里引用的实测数字（91 例等）属第一轮已审内容，本轮未复核。
