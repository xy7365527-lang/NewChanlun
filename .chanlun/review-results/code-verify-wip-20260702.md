# 代码验证报告 20260702（干净context重跑）

## 1. cargo build
通过，exit 0，仅 warning（未用变量/dead_code/非snake_case函数名等，无 error）。

## 2. cargo test --lib
1353 passed / 2 failed / 92 ignored

失败测试（同一 debug_assert 触发，均在 `src/theta_v0/classifier/mod.rs:1020`）：
- `theta_v0::classifier::tests::incremental_tower_per_segment_append_matches_full`
- `theta_v0::classifier::tests::incremental_tower_scaling_dominates_full_synthetic`

panic 信息：`had_emitted_window ⟹ 至少一个已产出中枢可回退`

**根因定位**：`git diff` 确认 `had_emitted_window` / `resume_start` 整套逻辑是本次未提交改动**新增**的（注释标注"★frontier 修复（task #47/#21，区间套.pdf 六~十节裁决②）"）。断言声称"若 `resume_from < consumed`（存在已产出窗口），则 `lc.centers` 非空、可 pop 回退最后一个 frontier 中枢"。测试实测中该不变量不成立——`resume_from < consumed` 但 `lc.centers`/`lc.upper_moves` 为空，说明"已产出窗口"与"已产出中枢"两个条件在增量扫描的某些路径下（per-segment append / scaling dominates 两个合成场景）脱钩。

**定性**：不是随机噪声或环境问题——两个失败用例在同一断言、同一新增逻辑块内。是否为"实现 bug"还是"定义冲突"需要判断：断言本身编码的是一条**充分性假设**（resume_from<consumed ⟹ centers非空），如果这条假设在 anc.pdf §16 / 区间套.pdf 六~十节的原始定义下并不成立（即"产出窗口"允许在不产出中枢的情况下推进 resume_from），则是定义层面的断言过强，需上浮；如果只是实现遗漏了某个"consumed 推进但未产出中枢"的分支（如仅推进 scan cursor 而未 emit），则是实现 bug。**本工位职责为验证，不修复，此判断需上游持有该改动上下文者（frontier 修复 task #47/#21 的实施者）确认**，标注为待上浮项。

## 3. pytest scripts/tests/test_goal_reducer.py
29 passed / 1 failed：
- `test_ready_details_empty_when_no_decompose` — `AssertionError: assert [{'desc': 'c', 'id': 'c'}] == []`
- 与 rust WIP 无关联，独立缺陷，goal_reducer.py 的 wiring 改动引入或暴露。

## 4. goal 4条 acceptance 对照（wiring 修复验证）
`ceremony_scan.py` 输出 `goal_ready_workstations` 长度 = **4**，与预期一致。

逐条对照（基于 memory 索引 4 条 acceptance 主题）：
| acceptance | 判定 |
|---|---|
| level塔空洞（H1门滤空） | goal_ready 计数达标，wiring 层面已接通；机制本身此前已被"确证但归因未定"（见 `project_level_hole_window_dependence`），本次验证范围仅确认 wiring，非重新验证机制 |
| P2 MACD veto→feature | wiring 已接通（goal_ready=4 含此项） |
| GAP3 EarningShares 可达 | wiring 已接通；但本次 rust 全量测试新增 2 个 frontier 相关失败，需确认与 GAP3/EarningShares 路径是否共享 `theta_v0::classifier::mod.rs` 增量扫描代码（**共享同一文件**，需排查是否互相影响） |
| alpha | wiring 已接通 |

## 5. commit 建议
**拆分为两个独立 commit**（rust WIP 与 goal_reducer wiring 修复无关联）：
1. **rust/src/theta_v0/ WIP**：暂不 commit——2 个测试失败源于本次新增的 frontier 修复逻辑本身，断言可能定义过强或实现遗漏分支，需先上浮确认后再决定"收紧断言"还是"补分支"，不能带着已知失败 commit。
2. **scripts/goal_reducer.py + scripts/tests/test_goal_reducer.py**：wiring 修复本体（+15行）验证通过（ceremony_scan goal_ready=4 符合预期），但其测试文件本身有 1 个失败（`test_ready_details_empty_when_no_decompose`），需先修复该测试后再 commit——否则视为"带着已知失败提交"，违反 testing-override 标准要求。

## 结果包六要素
1. **结论**：build✓；rust test 1353/1355通过2失败；pytest 29/30通过1失败；wiring goal_ready=4符合预期。两个rust失败共享同一新增断言（frontier修复），1个pytest失败独立于rust改动。均不建议直接commit。
2. **定义依据**：`.claude/rules/testing-override.md`——"测试失败揭示的是定义冲突而非实现错误时，不修改实现来让测试通过，而是走矛盾上浮流程"；本报告2号rust失败的定性（实现bug vs 定义冲突）无法仅凭本工位（验证工位）职责范围内的信息判定，故标注上浮而非擅自判定。
3. **边界条件**：若确认 `resume_from<consumed` 允许中枢空产出为定义内合法路径 → 断言需改写为"允许空pop"，测试保留失败信号价值直到断言修正；若确认是遗漏分支 → 实现修复后测试应转绿，此时结论翻转为"可commit"。
4. **下游推论**：rust WIP 不commit → GAP3/EarningShares 相关下游工位若依赖此文件（`theta_v0/classifier/mod.rs`）的当前HEAD版本（非WIP）不受影响；若下游已依赖WIP版本代码路径，需等frontier修复上浮结论后才能推进。
5. **谱系引用**：本报告未直接对应`.chanlun/genealogy`已结算条目；frontier修复引用的"区间套.pdf 六~十节裁决②"是否已谱系化，本工位未核实（超出验证工位职责，建议实施者补充）。
6. **影响声明**：本次仅验证，未修改任何代码/测试文件；仅新增本报告文件。

## wiring-test-fix (goal_ready 收口)
旧断言过时：`test_ready_details_empty_when_no_decompose` 假设"ready 只来自 DECOMPOSE"，与 630号开口②修复（open acceptance 直接派生 ready）冲突。判定为断言过时非实现误混——更新为新契约（有 open acceptance→ready 非空 / 无 acceptance→空），补真退化空 case。pytest 31/30 全绿。未 commit。
