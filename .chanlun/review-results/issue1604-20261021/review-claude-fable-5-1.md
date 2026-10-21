# #1604 非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1663，分支 `sandcastle/issue-1604`（`703e3d1c7f` 实装 + `64af6270e8` 评审工蜂），base `02b5b571eb`。
- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318），模型 `claude-fable-5-1`；实装与评审工蜂均为 `claude-opus-5-5`，分支内 `issue1604/REVIEW.md` 为同模型子代理所做，本审查非作者。
- 工具限制：Read/Grep/Glob 与只读 git；未跑 cargo/pytest。审查提示原文：`prompt.md`。日期：2026-10-21。

---

结论：PASS

PR #1663（票 #1604）非作者独立审查报告。审查范围 `git diff 02b5b571eb...HEAD`（实装 703e3d1c7f + 评审工蜂 64af6270e8），只读，未跑 cargo / pytest / gh。

## 总评

验收硬条件均满足：运行时唯一读取点是具名签署记录（`second-seq-scan-budget/2`、`segment-second/2`），生产/回放路径里没有残留的尾窗或相切常量，缺记录/字段非法走 `PolicyError::{RecordMissing, AwaitingPolicy, Invalid}`，`impl Default for ParseConfig` 只是读记录失败即 panic 而非回退旧值；负控与「常量回退变异体必红」的测试存在且逻辑成立；尾窗截断标记 `INCOMPLETE_RESOURCE_BOUND` 与正常确认可区分，全量≡增量有测试锁；旧线回退干净、逐字节同 base。未发现 HIGH。下列 MEDIUM 是「记录 id+版本写进每份输出」在 S 写者与分类器两处只做了一半，以及 Python v0 口径变更越出票面字面范围——都已在 IMPLEMENTATION.md §7 登记或属后续票合理拆分，不构成阻断。

## HIGH

无。

## MEDIUM

**M1. S 写者 TB-02-C 输出里记录戳与实际所读版本不一致，且仍保留预算代码常量**
`rust/src/bin/s_session_v2/tb02c.rs:22`、`:34`、`:196`、`:209`、`:240`，`rust/src/theta_v0/parser/feature_seq.rs:190`
- `tail_cache_window`（:196、:209）现在确实从 `facts.policy` 读到的是 v2 记录的 `tail_window`，但同一份输出里 `"budget":{"policy":...}` / `"scan_budget":{"policy":...}` 仍盖 `SECOND_SEQ_SCAN_BUDGET_POLICY = "second-seq-scan-budget/1"`；`parse_second_seq_scan_budget` 又明确把 v1 判为 `Invalid`。于是一份输出同时宣称「读的是 v1」和「值来自 v2」。
- `const ROUND_BUDGET: u32 = 0`（:22，生产恒用）与第二序列的 `"tail_cache_window":0` 字面量（:240）仍是代码常量，没有改读记录的 `second_seq_window`（当前恰好也是 0，所以逐位相同）。票面「生产/回放/准入不得有代码常量」在 S 写者侧未闭合。
- IMPLEMENTATION.md §7 登记了「S token 不升」，但没有登记 `ROUND_BUDGET` 常量这一条。考虑到 `s_session/s_tb02_contract.py:567`、`s_session/browser/*.js` 校验器把 `second-seq-scan-budget/1` 和 `tail_cache_window === "0"` 写死，升版本需要同时改契约与浏览器端，拆票合理，但应在报告里把常量一条也写明并给出后续票号。

**M2. Python v0 口径由严格 `<` 改为记录 closed（`<=`），影响生产桥接，越出票面字面范围**
`src/newchan/a_segment_v0.py:166-168`，调用方 `src/newchan/ab_bridge_newchan.py:108`
- 票面写的是「Python P1 同读该字段」，并未要求把 v0 的既有口径改掉。改后 v0 在恰好相切的输入上段划分会变，`ab_bridge_newchan.py` 是生产路径，等价性验收「输出逐位相同」在 Python v0 侧对相切语料不成立。
- 已在 IMPLEMENTATION.md §7 第 1 条登记为有意偏离（理由是 #804「各层同读一值」），`tests/test_segment_policy_1604.py` 用 monkeypatch 覆盖了 closed/open 两档。但没有 Python 侧「改前 vs 改后」语料对拍数据来量化影响面，报告中「等价」措辞对 Python v0 应加限定。

**M3. 分类器层未扩展记录戳，也未消费尾窗截断标记**
`rust/src/theta_v0/classifier/pipeline.rs:441-455`（`TowerPolicyStamp { level_budget, macd }`）、`:554`（只按 `segments_parse_invalid_from` 门控）
- #1603 模式下 `ClassifyOutput.policy` 只带 tower-level-budget 与 macd 两条；本票新增的 `scan_budget`/`construction_rule` 没有写入分类器输出。「每份输出写出记录 id 与版本」目前止于 `SegmentFacts.policy` 与 `ParseLayer`，分类器下游输出看不到。
- `ParseLayer.segments_resource_bound_from` 全仓无消费者；`classify_incremental` 只对 `parse_invalid` 门控，尾窗截断不会让分类器进入等待/标记态。票面「可区分、上游可见」做到了 ParseLayer 这一层，再往上没有接。这与 #1603 当时的分层边界一致，建议登记为后续项而非本票范围。

## LOW

- **L1** `src/newchan/a_segment_v1.py:297`（`TAIL_WINDOW: int = 7`）、`:422` 使用处——Python v1 的尾窗常量未改读记录，也未在 §7 登记。Rust 侧已删 `TAIL_WINDOW`，Python 侧留着一个同名常量，与「全层同读一值」的目标不一致。
- **L2** `src/newchan/segment_policy.py:21-28`——`RECORD_PATH` 走 `parents[2]/rust/src/theta_v0/policy_records/segment-second.json` 的仓库相对路径，`pyproject.toml:50-51` 没有 package-data 把 JSON 打进包；安装到 site-packages 后会 `SegmentPolicyError`（fail-closed，方向正确），但这意味着 Python 包只能在 checkout 内运行，应写明。
- **L3** `rust/src/theta_v0/parser/feature_seq.rs`（`probe_tail_excluded`）——在签署值 7 下，探针对每个被挤出窗口的 case-two 候选都会再跑一次 `second_sequence(..., 0)`。我推导它在 tw ≥ 3 时是状态中立的（见下「看过什么」），但这是额外计算，报告与 REVIEW.md 建议 4 把它描述为「几乎无开销」缺量化。
- **L4** `rust/src/theta_v0/parser/segment.rs`（`TailWindowBound::new(seg_start, ..)`）——首段的 `seg_start` 与 `parse_invalid` 用的 `start` 在首段上取值不同；批量与增量两边一致，所以不是 bug，但两个标记的「起点」语义不同，上游消费时容易混淆，建议在类型文档注明。
- **L5** 提交 64af6270e8 的提交信息没有「编号声明」行（703e3d1c7f 有）。仓库惯例要求每个提交声明 `#1604` 仅指 GitHub Issue。
- **L6** `docs/reference-theta-v0.md` 未同步新增的 `SegmentFacts.policy` / `tail_window_bound` 与 `ParseLayer.segments_resource_bound_from` 字段（REVIEW.md 建议 2 已提，未执行）。

## 对六个重点的逐项结论

1. **残留常量 / 回退**：`ParseConfig {` 字面构造只在 `rust/src/theta_v0/config.rs:87`（`from_active_records`）；`impl Default` 读记录失败即 panic，无 `unwrap_or` 回退值；`Tangency::Closed/Open` 字面量只出现在 config.rs、policy_records.rs 和测试代码；`TAIL_WINDOW` 已从 `segment.rs` 删除并有源码守卫禁止在 parser 下重现。唯一残留是 M1 的 S 写者 `ROUND_BUDGET` 与 Python v1 `TAIL_WINDOW`（L1）。Python dataclass `SegmentConstruction` 无默认值，解析严格。
2. **尾窗截断标记**：`probe_tail_excluded` 的判定逻辑我逐条对照过——只对已定稿窗口 `(i-1, i, i+1)` 且 `i < tail_lo` 的候选探测，命中物质性候选（case-one 确认或 case-two 非 Killed）即置 `tail_truncated`，再由 `divide_segments_core` 写入 `facts.tail_window_bound`（只记首次）。漏标风险：Open/None 状态会先走 `unresolved_blocked` 返回，不会探测，但该状态下窗口也不会前移，所以不会漏。误标风险：探针只在 `from >= end` 之外的区间探测，不会把窗口内元素误判。增量侧 `IncrSegments::resource_bound_from` 的持久化/过滤逻辑与 `parse_invalid_from` 同形，`tb1604_tail_window_ge3_equals_unbounded_and_tw1_is_marked` 锁了 360 例 tw=1 全量≡增量且 `marked > 0`。标记通过 `SegmentFacts.tail_window_bound`（含 policy 戳）与 `ParseLayer.segments_resource_bound_from` 暴露，与正常确认段可区分。
3. **tangency**：theta_v0 下所有相切敏感谓词（`is_fractal_and_gap`、`gap_between`、`closed_by_c`、`Interval::overlaps/gap`、`three_stroke_overlap`、`end_gate`、`classify_termination`、`locate_apex_stroke`、`record_candidate` 的 `closure_operator`）都经 `Tangency::meets`；第二特征序列里剩下的 `<=`/`>=` 是包含关系与 case-one 突破判定，不是相切边界。open/closed 定点 fixture（`[5,10],[0,10],[0,5]` 在 5 相切：closed 为重合、open 为不重合）与定义一致。Python v1 用 `meets`/`separates` 两个互补谓词，closed ⇔ `<=`/`>`，open ⇔ `<`/`>=`，正确。v0 口径变更见 M2。
4. **等价性**：签署值 tw=7 下 `probe_tail_excluded` 为状态中立：索引 i 的窗口在标准化长度 i+3 时定稿，此时 `tail_lo = i+3-7 ≤ i`，i 在窗内已被主循环判过；非命中判定对已定稿的 a,b,c 稳定，Killed 在批量与增量下看到同一份 `strokes`，命中则 `reset`。探针对 `skipped_secondkind` 的副作用与主循环已置值一致。`tb1604_*` 用 360 例锁 tw=3/7 与 ∞ 除 `checks` 轨迹外逐位相同、closed 覆写 == 签署。`SegmentFacts` 新字段均 `skip_serializing_if none`，TB-02-C 逐字段消费不整体序列化，`tail_cache_window` 7→7、`resource_truncated` false→false。tb02/tb1493/tb1581 fixture 在 diff 中未改动。
5. **旧线回退**：`git diff --stat 02b5b571eb HEAD -- rust/src/segment.rs rust/src/bin/ s_session/ formal/ docs/` 只剩 `rust/src/bin/s_session_v2/tb02c.rs`，`rust/src/segment.rs` 与三个 `p94x` bin 逐字节同 base；grep 无残留 `separates`/`active_segment_construction_cached`/theta_v0 转接模块，无半截编辑。
6. **编号声明与报告**：703e3d1c7f 有编号声明，64af6270e8 缺（L5）。报告表格（tw=3 355 例、tw=7 91 例 checks 轨迹差、tw=1 360 例标记）与 `tb1604_tests.rs` 的 eprintln 口径一致，结论未超出测试所证；§7 偏离登记了 Python v0、S token、旧线、Lean #1626 四条，漏了 `ROUND_BUDGET`（M1）与 Python v1 `TAIL_WINDOW`（L1）。

## 看过什么

- 完整 diff：`config.rs`、`policy_records.rs` 及两份新 JSON 记录、`feature_seq.rs`、`segment.rs`、`second_kind.rs`、`parser/mod.rs`、`profile.rs`、`tb02c/d/w_tests.rs`、`tb1604_tests.rs`、`bin/s_session_v2/tb02c.rs`、`theta_v0_lean_parity.rs`、`segment_policy.py`、`a_segment_v0/v1.py`、`test_segment_policy_1604.py`、IMPLEMENTATION.md、REVIEW.md、两条提交信息。
- 通读（非仅 diff）：`feature_seq.rs` 600–1160、`segment.rs` 960–1230、`classifier/pipeline.rs` 385–576、`tb02c.rs` 1–60 与写出位置、`tb1604_tests.rs` 全文。
- 全仓 grep：`ParseConfig {` 构造点、`unwrap_or`、`Tangency::` 字面量、`TAIL_WINDOW`、`separates`、`segments_resource_bound_from` 消费者、s_session 契约/浏览器校验器对 `second-seq-scan-budget/1` 与 `tail_cache_window` 的要求。

## 未核实什么

- 未跑 `cargo test/fmt/clippy` 与 `pytest`，报告声称的 2895 pass / 181 pass 未复现。
- 未调 gh，票 #1604 原文仅凭用户转述；「Python P1 同读该字段」的确切措辞与 M2 的范围判断依赖该转述。
- `probe_tail_excluded` 在 tw ≥ 3 下状态中立是推导结论，有 360 例语料测试佐证，但未对真实行情数据做过对拍。
- 未实际执行 S 写者或 Python 桥接，TB-02-C 输出逐位不变是基于代码路径的静态推断。
