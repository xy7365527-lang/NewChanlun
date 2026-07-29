# 影子评审：票 #312 主缝锁两谓词（commit `e3b0d50c38`）

- 日期：2026-07-26｜评审人：Opus 5 主控（新上下文；实装为另一 claude lineage，禁自评满足）
- 对象：worktree `/tmp/kimi-nest-mainline` commit `e3b0d50c38`（6 文件 +309/−20）｜只读，唯一写入=本报告

## 0. 独立复跑（全部本 session 亲跑）

| 项 | 结果 |
|---|---|
| `cargo test --release --lib` | 1842 passed / **1 failed** / 132 ignored；唯一失败 `extract_signals_bit_exact_digest_guard`（#110 在案），digest `left=16618955402698307653 / right=10432481772907336594` 与 #276 报告逐位相同 → 未修未归因。计数漂移来自并行 session（#317 新增 `segment_tangency_tests.rs`），非本 commit |
| parity 四测试 | lean 11 + center 4 + classifier 19 + buy 10 = **44/44** |
| `check_fixture_drift.py` | exit=0，两 fixture 语义+字节级均一致 |
| **篡改抽验**（`segment.rs:141` `<=`→`<`） | 红 2 案并指认：`three_stroke_overlap_lean_fixture_bit_exact`（`tangent_a_high_eq_b_low…须 == Lean decide(Overlaps a b) left:false right:true`）+ `distinct_triple_tangent`；**同一篡改态下旧主缝 `theta_v0_lean_parity` 仍 11/11 全绿**（HIGH-1 属实，新缝是唯一守卫）；`cp` 恢复后 `git diff` 空、16/16 绿 |

## 1. Spec 轴（票 #312 + shadow-review-248 HIGH-1）

| # | 要求 | 判定 | 证据 |
|---|---|---|---|
| S1 | 取修法 (b) 并论证 | PASS | 新增 `gap_overlap_fixture.rs`（+213），两谓词仍无 `pub`（`feature_seq.rs:113`/`segment.rs:130`）；论证见 #312 Resolution |
| S2 | 篡改双向可分辨 | PASS | §0 抽验实证（three_stroke_overlap 侧）；feature_seq 侧红案由实装登记，未抽验 |
| S3 | fixture 读取点唯一性 | PASS | `src/` 下 `theta_v0_parity.json` 唯一 `include_str!` = `gap_overlap_fixture.rs:116`；两模块经 `super::super::gap_overlap_fixture::gap_overlap_cases()` 共用（`feature_seq.rs:506`、`segment.rs:811`） |
| S4 | 「禁手填」收窄 | PASS | 模块头 `:22-33` 三分登记：期望值禁手填 / 端点人工镜像（**不会红**）/ 本模块 tests 是 #246 裁定人工编码 |
| S5 | `mirrored_endpoints_match_kind` | PASS | `:189-211` 由端点几何反推 kind 再对真值，捕获「端点抄错但真值恰对」；`:194-200` 逻辑与 Tangent/Disjoint/Overlap 定义一致 |
| S6 | #248 reopen 闭合 | PASS | #248 CLOSED 2026-07-26T07:44:21Z；主缝注释 `theta_v0_lean_parity.rs:437-448` 补有效域「本测试绿 ≠ 那两谓词口径已锁」 |
| S7 | 无覆盖删弱 | PASS | 删除的 2 个 tangent 单测形态被新 fixture 测试的相切用例（Up/Down 双分支）吸收；`tangent_counts` 仅改名为 `distinct_triple_tangent`，断言体保留并加指认信息（`segment.rs:825-832`） |

## 2. Standards 轴（090 / 231）

| # | 标准 | 判定 | 证据 |
|---|---|---|---|
| T1 | GapOverlapKind 消 Primitive Obsession | PASS | `:69-77` 三值枚举 + `#[derive(PartialEq, Eq, Debug)]`；`kind` 字段强制新增用例显式归类，不按名字前缀猜 |
| T2 | 失败信息指认性 | PASS | 实证（§0）：用例名+谓词名+口径方向全在 panic 文本内 |
| T3 | 端点镜像忠实 | PASS | `gap_overlap_fixture.rs:120-152` 五组端点 == 导出器 `ParityFixtureExport.lean:187-193` `feOf` 参数 == 主缝 `:456-489`，逐字核对一致 |
| T4 | 构造正确性 | PASS | `stroke_interval`（`segment.rs:105-111`）排序 → `stroke(_,_,_,lo,hi)` 得 `[lo,hi]`；退化 `(a,b,b)` 使 `max/min` 塌缩为两区间式，与 `Overlaps a b` 同式；feature_seq 侧 `assert!(f_up/f_dn)` 先坐实分型前件，杜绝断言退化 |
| T5 | L0 标注收窄 | PASS | 模块头 `:41-45` 纯 L0，明写「不构成任何盈利/实盘有效声明」 |
| T6 | 剥离对照真实性（+2） | PASS | 净增算术自洽：feature_seq −2+1、segment ±0+1、fixture 模块 +2 = **+2** |
| T7 | 诚实边界 | PASS | 退化三笔不冒充覆盖互异三笔（`segment.rs:805-807`）；CI 腿未闭合登记在 `:35-39`；`ordered()` 塌缩实得 3 组互异输入照实登记 `:98-101` |

## 3. 问题清单（无 HIGH，不回票）

### MED-1｜端点镜像仍是三处人工对照，转录漂移未消除
`gapOverlapJson` 只导出真值、不导出输入端点，故同一组端点手抄于三处（`ParityFixtureExport.lean:187-193`、`gap_overlap_fixture.rs:120-152`、`theta_v0_lean_parity.rs:456-489`）。Lean 端点若改而 rust 侧漏改，两侧测试**都不会红**——真值仍是原形态的，rust 却在测另一组端点。#312 消除的是「真值」转录漂移，「端点」转录漂移原样留存。实装已在两处显式登记（`:26-30`、`:446-448`），故非声明膨胀，但缺口客观存在。**建议（票外）**：导出器 `gapOverlapJson` 一并输出 `a_lo/a_hi/b_lo/b_hi`，两处改读，缺口即闭。

### LOW-1｜本缝只在 `cargo test --lib` 生效
仓内 CI 只跑 pytest 与 `fixture-drift`，不跑 `cargo test` ⟹「口径回改即红」在自动化层未闭合。已在 `:35-39` 如实登记，票 #312 未要求 CI 腿，登记合规。

### LOW-2｜`ordered()` 对 `a.lo == b.lo` 的输入静默塌缩
`:104-110` 以 `<=` 定序，两区间 lo 相等时「下方/上方」失去意义，分型前件退化。当前 5 用例均严格不等，且 `assert!(f_up/f_dn)` 会红而非静默通过，故实效风险为零；新增此类用例时才需处理。

## 4. 结果包六要素

1. **结论**：两轴 14 项**全 PASS**，无 FAIL。#248 reopen 的 HIGH-1（主缝对两个私有谓词零覆盖）已被实质闭合——本 session 独立复现「篡改后旧主缝仍全绿、新缝红并指认」的双向证据。3 项问题均为 MED/LOW，不构成回票。
2. **定义依据**：#312 票体修法 (b) 与验收三条；shadow-review-248 HIGH-1「次缝接上机器耦合链」补法；`no-patch-mentality.md`（090 声明膨胀禁止）；`formalization-validity-domain.md`（231 号认识论等级）。
3. **边界条件**（结论翻转）：若 MED-1 的端点漂移实际发生（Lean 侧改 `feOf` 参数而 rust 两处漏改），S3/T3 的 PASS 翻为 FAIL 且两侧静默；若 #246 裁定按其 §3.1 重议触发条件翻转，`mirrored_endpoints_match_kind` 与 `distinct_triple_tangent` 须同步改，届时其红的含义是「裁定已变」而非「Lean 漂移」（`:181-186` 已预登记）。
4. **下游推论**：两个生产谓词的相切口径自此有机器见证守卫（crate 内 `--lib` 层）；`theta_v0_lean_parity` 绿不再被误读为该口径已锁（主缝注释已加有效域）；#276 MED-1（三笔互异相切 Lean 无见证）仍开口，本 commit 明确不冒充覆盖。
5. **谱系引用**：#84 点3 → #246 supersede 链在 `segment.rs:117-129` 完整；631 机器耦合模式（消转录漂移）在真值维度落地、端点维度未落地（MED-1）。
6. **影响声明**：只读评审，未改动任何被评审文件（篡改抽验后 `cp` 恢复，`git diff` 已验为空）。写入仅本报告。工作区 `M rust/src/theta_v0/strategy/coverage.rs` 与 ~370 个 ` D` 为并行 session 所有，未触碰。
