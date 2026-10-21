# #1604 实施报告：线段尾窗与相切边界并入具名记录（#1586 C-04 / A-12）

实施者：Claude Code（sandcastle 工位，分支 `sandcastle/issue-1604`，单提交）。独审记录见同目录 `REVIEW.md`。
Rust 验证全部经远端 `nc-remote`（本地未跑 cargo/rustc/rustup/rustfmt），日志 `/tmp/nc-remote/20261021-173133-*.log`。

## 1. 记录

| 记录 | 版本 | 字段 | 值 | 来源 |
|---|---|---|---|---|
| `second-seq-scan-budget` | 2 | `second_seq_window` | 0 | v1 原值（#1344/#1420） |
| `second-seq-scan-budget` | 2 | `tail_window` | 7 | 原常数 `parser/segment.rs` `TAIL_WINDOW = 7` |
| `segment-second` | 2 | `tangency` | `closed` | #246 裁定的生产口径 |

文件：`rust/src/theta_v0/policy_records/{second-seq-scan-budget,segment-second}.json`。Rust 用 `include_str!` 编入；Python `src/newchan/segment_policy.py` 读同一份 `segment-second.json`，不另存副本。

## 2. 读取路径与输出戳

- `ParseConfig::from_active_records()`（`config.rs`）从两份记录给出 `tail_window`、`second_seq_scan_window`、`tangency` 和两个 `PolicyRef`。
- 新线 Rust 各级都只读 `ParseConfig.tangency`，包括特征序列、`construction`、第二种情况的 `resolve_second_kind`、Lean 对拍测试和 S 会话 TB-02-C。
  - 旧线 `rust/src/segment.rs` 不在本票（issue「不在本票」节：`:410/:413` 归 TB-10），保持改前写死的 `<=`／`>`（等于签署值 `closed`）。评审修正（`review: #1604`）已撤回初版对旧线的接线。
  - Python v0 和 v1 经 `active_segment_construction()` 读同一记录。
- 输出戳：
  - `SegmentFacts.policy` 写 `scan_budget`/`construction_rule` 的 `id/version`，以及 `tail_window` 和 `tangency` 的值。
  - 研究覆写一律戳 `…/research-override`。
- 尾窗标记：
  - `SegmentFacts.tail_window_bound` 带 `INCOMPLETE_RESOURCE_BOUND`、首个受影响段起点和所读记录。
  - 在 `ParseLayer.segments_resource_bound_from` 和 `ParseLayerIncr::segments_resource_bound_from()` 上层可见，四个构造入口都已接上。
  - S 会话 TB-02-C 按记录值取 `tail_cache_window`，并为每个候选写 `resource_truncated`。

## 3. C-04：7 vs ∞ 实测

测试 `parser/tb1604_tests.rs::tb1604_tail_window_ge3_equals_unbounded_and_tw1_is_marked`。语料为 LCG 锯齿：seed 0..120 × amp {3,6,15}，共 360 例，每例 60 笔。

| 尾窗 | 段 + 尾部（pending） | 段事实（除 `checks`） | 诊断轨迹 `checks` 与 ∞ 不同 | 尾窗标记 |
|---|---|---|---|---|
| 3 | 360/360 逐位相同 | 360/360 逐位相同 | 355 例 | 0 |
| 7（签署值） | 360/360 逐位相同 | 360/360 逐位相同 | 91 例 | 0 |
| 1 | 360/360 不同 | — | — | 360/360 带标记（凡不同处必带标记） |

**差异说明**：
- tw ≥ 3 时，段、尾部和判定结果都与 ∞ 相同。
- 唯一的差异是 `FeatureSequenceFacts.checks` 这条诊断轨迹。全扫会对已判过、已失效的峰在之后每次扫描里重复求值，`checks` 只去掉相邻的重复项，所以多出几条内容重复的检查条目，判定状态不变。
- 结构论证见 `feature_seq.rs` `probe_tail_excluded` 文档：
  - 标准序列只有末元素可变；
  - 下标 i 的窗口在长度达到 i+3 时定型，那次扫描时 i 一定在窗内。
  - 记录 `options` 已据实改写。

**本轮修掉的漏标**：
- 第一版探查只看窗口已定型的下标。
- 但全扫在窗口仍含末元素时，也会按这个未定型窗口确认下标 `n-2`（确认时才冻结，#1493）。
- tw=1 时这一格既不在窗内，也不在探查范围内，导致语料第 330 例结果变了却没有标记。
- 现在 `n-2` 每次都重探，已定型的下标仍只探一次（`probed_upto` 只推进到定型边界）。
- 增量与全量的标记、段已逐例对拍一致。

## 4. A-12：相切

- 谓词 `Tangency::meets(lo, hi)`：closed 为 `<=`，open 为 `<`。Python 侧另有 `separates(lo, hi)`：closed 为 `>`，open 为 `>=`；非 NaN 输入上它等于 `!meets`。它直接保留改前的比较符号，所以在 NaN 上与改前逐位一致。（Rust 侧同名方法随旧线接线一并撤回。）
- 每个取值一个 fixture：
  - Rust `tb1604_tangency_fixture_seed_per_value` 和 `segment.rs::tangency_fixture_per_value_flips_only_tangent_case`；
  - TB-02-C 的 `tb02c_tests` 加了 open 例；
  - Python `tests/test_segment_policy_1604.py` 覆盖 v0 重合、v1 重合和 v1 缺口三处，各两档。
- 跨级对拍锁：
  - `tb1604_tangency_open_changes_output_closed_reproduces_signed`：closed 覆写与签署记录逐位相同；open 在语料上至少一例改变段划分；open 下增量等于全量。
  - Python `test_active_record_matches_rust_signed_value`：断言 Python 读到的 token 与 Rust 正本文件一致。
- Lean 归 #1626，本票未动 `formal/`。

## 5. 无常数 / 不回退 / 负控 / 变异体

- `rg -n "impl Default for" rust/src/theta_v0/config.rs`：
  - 与本票有关的只有 `:119 ParseConfig`，实现为 `from_active_records().unwrap_or_else(|e| panic!("{e}"))`：读记录，读坏就停，没有第二个取值。
  - 其余 `TickConfig :28`、`LevelConfig :217`、`MacdConfig :243`、`VoiceConfig :291`、`RiskConfig :371`、`ExecConfig :469` 不属本票。
- 源码守卫（`policy_records.rs` 单测）：
  - `source_guard_no_constants_or_in_place_writes`：扩了 `tail_window: 7`、`second_seq_scan_window: 0`、`tangency: Tangency::` 字面量，以及新线 parser 中 `TAIL_WINDOW` 不再出现在代码里。
  - `source_guard_tangency_only_from_record`：扫 `src/**` 的生产代码，不得写 `Tangency::Closed/Open`。
- 负控 `negative_controls_scan_budget_and_tangency` 覆盖：
  - 两份记录缺记录（`RecordMissing`）；
  - 缺 `tail_window` 或缺 `tangency`，以及 `"AwaitingPolicy"`、`null`（`AwaitingPolicy`）；
  - 非法 token `half`、大小写错 `Closed`、数字值、裸字符串值、id 错、负数尾窗、裸数尾窗；
  - v1 预算缺新字段语义（`Invalid`）。
  - Python 侧同口径负控 11 例。全部报错，没有回退。
- 变异体：
  - 把相切谓词回退为任一常数时，open 覆写与 closed 相同，`tb1604_tangency_open…` 和逐档 fixture 必红。
  - 把尾窗回退为常数 7 时，`with_tail(1)` 不再挤掉候选，`marked > 0` 断言必红；`tb1604_signed_record…` 与 `scan_budget_and_construction_other_versions`（v3 记录）读不到记录值也会红。
- 等值对拍：记录值等于原常数，`cargo test --locked --lib` 全量 2895 passed、0 failed，既有 tb02*/Lean 对拍等锁不变。

## 6. 验证

- 远端：
  - `cargo fmt --check` 通过；
  - `cargo test --locked --lib` 2895 passed / 0 failed / 153 ignored；
  - `cargo test --locked --tests --bins` 全部 ok。
- **clippy 未验证**：远端工具链未装 `cargo-clippy`，CI 也没有 clippy job。
- Python：`tests/test_segment_policy_1604.py` 与 `tests/test_*segment*.py` 共 181 passed。全量 pytest 的既有环境性失败（bottle、asyncio 等）与基线逐条相同。

## 7. 偏离与登记

1. **Python v0 口径变化（正本要求，非偏离）**：`a_segment_v0.py` 改前写死严格 `<`（即 `open`），现读记录 `segment-second/2`（`closed`，`<=`）。依据：`.chanlun/definitions/xianduan.md` §6.3 `:262` 默认 `closed`（承 #246）、`:386` 与 §15 X-38 `:524`「P1 与 claim5 的 `<` 改读字段……Rust 与 Python → #1604」；#1600 A-12、#1604 票面「默认 `closed`」。影响面：只在三笔重叠核心恰为单点（`max(lows) == min(highs)`）时变，例 `[5,10]`、`[0,10]`、`[0,5]` 改前 0 段、改后 1 段；`ab_bridge_newchan` 仅 `segment_algo="v0"` 走此路（默认 `v1` 不受影响）。锁：`tests/test_segment_policy_1604.py::test_v0_signed_record_counts_tangency_as_overlap`。等值对拍「逐位相同」对 Python v0 的相切输入不成立，这是正本裁定的有意改变。
2. ~~S 写者 token 不升版~~ → 已做（#1604 第二轮，独审 M1）：TB-02-C 写本次所读记录 label（`second-seq-scan-budget/2`），删 `ROUND_BUDGET` 与 `SECOND_SEQ_SCAN_BUDGET_POLICY`；`s-segment/4` 校验器要求 v2，`s-segment/3` 历史合同保留 v1。`segment-second/2` 早已在 rule_revision 内，未另加字段。代价：本改动之前由旧二进制写出、`schema_revision = s-segment/4` 且带 v1 token 的对象不再过新校验器（同 rule_revision 未升）；S 每代重算全部活动对象，K1 游标 v1 线形被判不可读后自 b 笔重折叠（预算 0 下检查点本就为空）。
3. 旧线 `rust/src/segment.rs` 整体属 TB-10，未动：`:426` 的 `TAIL_WINDOW` 与相切谓词（`:133` 三笔重叠、`:410/:413` 缺口）仍是代码常数，取值等于签署值 `closed`。记录改签 `open` 时旧线不会跟随，由 TB-10 处理。
4. Lean 侧相切归 #1626。
5. Python v1 尾窗（独审 L1）：`_FeatureSeqState.TAIL_WINDOW = 7` 已删，构造时读记录 `tail_window`；Python 无尾窗截断标记，记录值为 1、2 时拒绝运行（不静默截断）。
6. classifier（独审 M3）：`TowerPolicyStamp` 加 `scan_budget` / `construction_rule`；`ClassifyOutput.segment_coverage` 显式带尾窗截断标记与起点（只标记不拒读，与 `termination` 正交，故不与 #1662 新增的 `TowerTermination::ParseInvalid` 争同一字段）。

## 8. 后续

- 远端补装 clippy 或 CI 加 clippy job，补验零新增告警。
- #1626 Lean 侧 tangency 参数化。
