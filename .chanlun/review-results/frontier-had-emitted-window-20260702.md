# frontier 断言定性+修复：incremental_tower had_emitted_window（task #5）

工位：ws-frontier ｜ parent_callback：main ｜ 日期：2026-07-02

## 定性结论（一句话）

报告（code-verify-wip-20260702.md §2）记录的 2 个测试失败是**实现 bug**（`resume_from` 回退语义），**非定义冲突**；
根因修复**已存在于当前未提交 WIP**（recursive_tower.rs + mod.rs），无需新增改动。当前全量 `cargo test --lib` = **1363 passed / 0 failed**。

## 复现与矛盾

| 口径 | 结果 |
|------|------|
| 报告 code-verify-wip-20260702.md | 2 failed（`incremental_tower_per_segment_append_matches_full`、`incremental_tower_scaling_dominates_full_synthetic`），同一 debug_assert `mod.rs:1020`「had_emitted_window ⟹ 至少一个已产出中枢可回退」 |
| 本工位 `cargo test --lib incremental_tower` | 5 passed / 0 failed（含上述 2 个） |
| 本工位 无 filter `cargo test --lib` | **1363 passed / 0 failed / 92 ignored** |

报告与当前状态矛盾 → 报告捕获的是 WIP 的**中间态**，修复在报告之后、提交之前落入当前 WIP。

## 根因（实现 bug）

`detect_centers_windowed_resume`（recursive_tower.rs:340-365）扫描窗口，游标 `WindowScanCursor { consumed, resume_from }`。
调用方 mod.rs:1017 用 `had_emitted_window = resume_from < consumed` 判定「上次扫描产出过窗口」，为真则 pop 最后一个 center/upper_move（frontier 中枢，重扫会重新产出）。

**中间态 bug**：当一段扫描区间**无任何成立窗口**（全走 None 支，每次 `+1`）时，`consumed = i` 推进而 `resume_from` 若以 `start_i` 兜底，则 `resume_from(=start_i) < consumed(=i)` 为真 → 假阳性 `had_emitted_window` → 对**空 centers** 执行 pop 前的不变量断言失败。这正是报告描述的「`resume_from < consumed` 但 `lc.centers`/`lc.upper_moves` 为空」——"已产出窗口"与"已产出中枢"脱钩。

**当前 WIP 的严格修复**（不是特例分支，是语义对齐）：
- recursive_tower.rs:350 `let mut last_window_start: Option<usize> = None;`
- :353-355 仅在 `build()` 返回 `Some(c)`（真 push center 到 `out`）时 `last_window_start = Some(i)`
- :364 `resume_from: last_window_start.unwrap_or(i)`——无窗口回退到 `consumed(=i)` 而**非** `start_i`

修复后 `resume_from < consumed` ⟺「本次扫描至少 emit 一个窗口」为**精确等价**（`last_window_start.is_some()` ⟹ 至少一次 `out.push()`）。断言不变量恒成立：had_emitted_window 为真 ⟹ tail_centers 非空 ⟹ lc.centers 非空。cascade_reset 路径下 `scan_cursor=default`（resume_from=0=consumed）⟹ had_emitted_window 为假，断言不触达。

## testing-override 判据

- 不改任何定义即可修复（仅修正游标记账，使「窗口产出」与「中枢产出」正确耦合）→ **实现错误**，正常修复，不上浮。
- 未改变 "已产出窗口"/"已产出中枢" 任一概念的含义或边界 → 无定义冲突。
- no-patch-mentality：修复是 `resume_from` 语义的严格形式（`Option` + `unwrap_or(consumed)`），非在错误代码上加边界特例分支。断言保留为防御性不变量文档，未删未弱化。

## 本工位动作

定性 = 实现 bug（已修）。验证 = 全量绿。**无新增代码**——根因修复已在 WIP。ponytail：代码已含严格形式的根因修复，再加任何东西都是冗余。

## sub-swarm 分解评估

原子任务，不建子 DAG。理由：定性→（按需）修→验证 是严格串行链（每步输入是前步输出：不定性无法判断是否需修；不修无法验证），无 ≥2 独立并行子单元。

## 结果包六要素

1. **结论**：报告的 2 个 incremental_tower 失败是 WIP 中间态的实现 bug（`resume_from` 兜底到 `start_i` 致假阳性 had_emitted_window → pop 空 centers），根因修复已落入当前 WIP（`last_window_start: Option` + `unwrap_or(consumed)`）。当前全量 `cargo test --lib` = 1363 passed / 0 failed。无需新增改动。
2. **定义依据**：`.claude/rules/testing-override.md`——「不改任何定义即可修复→实现错误，正常修复」。本例修复仅修正 `WindowScanCursor.resume_from` 的记账语义，未触动 "成立窗口"/"中枢产出" 的定义（recursive_tower.rs:319-339 bit-exact 充要条件文档不变），故属实现错误而非定义冲突，不上浮。
3. **边界条件**：结论翻转条件——(a) 若未来发现 `resume_from < consumed` 在 `last_window_start.is_none()` 下仍可为真（即 `unwrap_or` 语义被改动或 `WindowScanCursor` 构造被绕过），断言会重新触达空 centers，需重新定性；(b) `incremental_tower_scaling_dominates_full_synthetic` 含**时间比断言** `ratio_at_max < 0.5`（recursive_tower/mod.rs:1870），重载机器下可能因 debug 计时噪声偶发 flake——此为该测试固有属性，**与本次 frontier 修复无关**（报告中该测试因 debug_assert 先触发而未走到计时判据），非本任务引入。
4. **下游推论**：GAP3/EarningShares 与本 frontier 修复共享 `theta_v0/classifier/mod.rs` 增量扫描路径（code-verify-wip §4 提出的排查项）——本次验证表明该文件当前 WIP 全量测试绿，共享路径不被 frontier 断言阻塞，GAP3 下游可安全依赖当前 WIP 版本。该 WIP（frontier 修复 + resume_from 语义）现无已知失败，具备 commit 前提（就 rust 侧 2 个 frontier 失败这一阻塞项而言已解除；goal_reducer pytest 失败是独立项，见 code-verify-wip §3/§5）。
5. **谱系引用**：同源于记忆 `project_frontier_resume_bt_too_late`（bar49291 末中枢多吸收段；consumed 停太晚；实现 bug 非定义冲突）。该记忆记录的根问题即「resume 用 `consumed` 停太晚 → 最后一个 frontier 中枢多吸收段」；本 WIP 正是其修复——resume 改从 `resume_from`（最后成立窗口起点）重扫，每 bar 重算 frontier 中枢。报告的 had_emitted_window 断言失败是实现该修复时引入的中间态回归（resume_from 兜底 bug），现亦已修。属同一 frontier resume / consumed 语义谱系族，均为实现 bug 而非定义冲突。`.chanlun/genealogy` 中 "区间套.pdf 六~十节裁决②" 是否已结算为独立条目本工位未核实（超出定性职责）。
6. **影响声明**：本工位**未修改任何代码/测试文件**——根因修复已存在于未提交 WIP（`recursive_tower.rs` `detect_centers_windowed_resume`/`WindowScanCursor`、`mod.rs` `classify_with_tower_incremental` had_emitted_window 回退块）。仅新增本报告文件，并将 code-verify-wip §2「待上游确认」的 frontier 上浮项定性关闭为「实现 bug，已修，验证绿」。
