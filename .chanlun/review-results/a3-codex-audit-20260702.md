# A3 设计稿 Codex 异质审查判决（ws-a3audit）

**审查对象**：`.chanlun/review-results/a3-design-20260702.md`（per-level dirty_from 证书 §2 + 04/03 证书化 §3 + project_to_units_resume truncate 修正 §2.5）
**审查方式**：本机 codex CLI（`newchan.codex review`），上下文含设计稿全文 + mod.rs 主循环 940-1136 实码 + recursive_tower.rs 380-471 实码 + 审计工位自行推导的一条验证链
**Codex 交互持久化**：`.chanlun/review-results/codex-review-20260702-181938-451b.md`

## 判决：需修改（不通过，退回设计稿补丁）

## 三个指定问题的回答

**(1) 正确性**——`dirty_from[L+1] = prefix_count[L]`（pop 后 extend 前取值）：**成立**。对照 `project_to_units_resume` 实码（recursive_tower.rs:462 `prev = moves[idx-1]`，仅 1 阶前驱），`idx < prefix_count` 只读 `moves[idx]`/`moves[idx-1]`，二者都在稳定前缀内；`idx == prefix_count` 才是首个可能脏点。无需 −1 偏移的论证审计工位已独立核实，Codex 复核一致。

**(2) 完备性**——证书失效边界：**不通过**。设计稿 §4 只列了"pop 恒单删末尾 / fold 仅 1 阶前驱"两条翻转条件，遗漏了第三条真实缺口：**早停级别的缓存血缘（cache lineage）未失效**。

- `mod.rs:956-958`（`if units.len() < min_parts { break; }`）与 `mod.rs:1127-1129`（`if units.is_empty() { break; }`）会跳过某个 level_idx 及其所有更高级别本 bar 的处理，但**不清空/截断它们的 `LevelCache`**（`cache.levels[level_idx]` 原样保留，含 `cached_units`/`scan_cursor`/`last_input_len`）。
- 当该级别 M 个 bar 后重新达到 `min_parts` 恢复处理时，它的 `lc.cached_units`/`lc.scan_cursor.consumed` 是 **M 个 bar 前的陈旧快照**，而 `dirty_from`（来自父级**本 bar**连续处理算出的 `prefix_count`）与这份陈旧快照之间没有血缘关系。
- **真实的坍缩路径**：`stable = dirty_from.min(scanned)`（§3.3），若父级已推进很远（`dirty_from` 大）而本级 `scanned`（受制于陈旧 `lc.scan_cursor.consumed+2` 和陈旧 `lc.cached_units.len()`）很小，`stable` 会被下压到 `scanned`，令比较区间 `[stable..scanned]` **坍缩为空**——`frontier_mutated` 无条件为 `false`（空切片必等），即使该级真实数据早已发生了 03 本应检出的变异，也会被静默放过，cascade 该触发而未触发 ⟹ 下游投影/BSP memo 复用错误数据。
- **对照现状**：当前生产代码 L1+ 恒 `stable=0`（`mod.rs:984`），`stable` 永远不会大于 `scanned`，此坍缩路径**不存在**——这是 A3 §3.3 引入 `dirty_from` 后**新增**的、当前代码没有的失效面，不是既有 bug 的重述。
- L0 的 `segments_confirmed_len`/`reuse` 语义本身不受影响（accept）；多级同 bar cascade 也不受影响（下级 reset 后 `prefix_count=0` 会正确传导）。缺口精确限定在"级别被跳过后恢复"这一路径。

**(3) 测试覆盖**——三层锁对 pop-and-rescan 的覆盖：**不通过**。现有 `project_to_units_resume` 测试（recursive_tower.rs:803 附近）只覆盖 append-only/clear 两种路径，未覆盖设计稿 §2.5 新提议的 `truncate(prefix_count)+append`；bar-1464 相关测试（incremental.rs:520/1071）多为 ignored/诊断性质，不是 always-run 锁。§5 验收清单需要补一条 **always-run 合成 oracle**，显式覆盖：`had_emitted_window` 下 `T==1`（重扫仅复现被 pop 的窗口）与 `T>1`（重扫产出更多）两个分支、以及"长度回缩后再次达到 `min_parts`/级别跳过后重入"路径，逐 bar 断言 `cached_units == units`、`projected_units == project_to_units(upper_moves)`、证书路径输出与强制全量路径 bit-identical。

## 审计工位自行推导链的复核结果

审计工位在审查前独立推导了"pop-and-rescan 下 `project_to_units_resume` 当前为何不炸"的证明链（数据不变 ⟹ 重扫必然复现同一窗口 ⟹ `T>=1` ⟹ debug_assert 数值关系不违反，但陈旧值与新值内容相等属于隐式不变量非显式锁）。Codex 复核：
- (a) `scanned < consumed` 在正常同源、未 shrink 路径下不是缺口，shrink 会触发 cascade——**accept**；但真正的缺口是"级别被跳过后 `cached_units` 不是上一 bar 快照"（即上面的完备性缺口）。
- (b) `id.ordinal` 复用无并发错位（循环串行，cascade 先 clear，`prefix_count` 会是 0）——**accept**。
- (c) 若不落地 §2.5 的 truncate 修正，当前"陈旧值=新值"的隐式不变量必须补内容级 debug/test 锁；若落地 §2.5，也要补 `projected_units == project_to_units(upper_moves)` 的定向锁——**needs_work（真实缺口）**。

## 附带发现（建议级，不阻断）

设计稿 §4/§5 把 `debug_assert!` 称为"release 保留的护栏"，与 Rust 语义矛盾——`debug_assert!` 在 release profile 下按定义被编译期剥离，不可能作为 release 层的正确性锁。这与设计稿自身 §3.2 "release 跳过（这就是省下的 4010ms 的来源）"的表述其实一致，只是 §4/§5 的措辞把它错误地叫成"release 保留"。修改建议：改称"debug/test 护栏"，若要在 release 下也校验，需用 env-gated `assert!` 或采样校验，不能寄望于 `debug_assert!`。

## 结果包（简化版：结论/边界条件/影响声明）

1. **结论**：A3 设计稿的核心证书公式（`dirty_from[L+1] = prefix_count[L]`）正确，但 §3.3（`stable = dirty_from.min(scanned)`）在"级别被跳过后恢复处理"场景下存在真实的比较区间坍缩为空的 soundness 缺口，且 §5 验收清单对 `project_to_units_resume` 的新 truncate 路径覆盖不足。判决：**退回，需补两处修改后重审**——(i) 在 §2/§3 补"级别早停缓存血缘"失效边界，实现上须在 `min_parts`/`units.is_empty()` 早停处截断/清空不可达级别的 `LevelCache`（Codex 建议的具体修复：`cache.levels.truncate(level_idx)` / `cache.levels.truncate(level_idx+1)`）；(ii) 在 §5 补 always-run 合成 oracle 覆盖 `T==1`/`T>1`/级别重入路径。
2. **边界条件**：若设计稿采纳 Codex 建议的早停截断修复（消除跨越 skip 的缓存血缘），完备性缺口即解除；若额外证明"某级别在生产数据分布下永不会被跳过后又恢复"（即 `min_parts` 早停在实测数据上从不发生跨 bar 恢复），则该缺口的**实际触发概率**可能为零，但设计稿当前未提供此类经验证据，不能作为不修的理由（形式化上仍是缺口）。
3. **影响声明**：本次审查零代码改动、零 git 操作。判决结果需 A3 工位据此修订设计稿（补完备性表格第三条 + 补 oracle 用例），修订稿再次过 codex 审后方可进入阶段2实施（recursive_tower.rs 插桩 + mod.rs 主循环 03/04/09 证书化）。
