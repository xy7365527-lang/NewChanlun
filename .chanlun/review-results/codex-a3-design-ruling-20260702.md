# A3 设计稿逐条裁决（codex diagnose 模式，ws-a3audit）

审查对象：`.chanlun/review-results/a3-design-20260702.md`。证据来源：本机 codex CLI review 交互
（`.chanlun/review-results/codex-review-20260702-181938-451b.md`，完整 prompt+response）+ 审计工位对
`mod.rs` 954-1136 / `recursive_tower.rs` 380-471 生产代码的直接核对。第7条未涉及 bit-exact 正确性
（纯 env-gated 测量插桩），不需二次调用 Codex，直接读设计文本判定。

**总裁决：partial（有 refuted 子项，未解锁阶段2）**——核心公式成立，但存在一个设计稿未列出的完备性缺口，
需修订后重审。

---

## 1. dirty_from[L+1]=prefix_count[L] 是否为可证不可变前缀（无需 -1 偏移）

**verdict: confirmed**

对照 `project_to_units_resume` 实码（recursive_tower.rs:462 `prev = moves[idx-1]`）：`idx < prefix_count`
时只读 `moves[idx]`、`moves[idx-1]`，二者均在稳定前缀 `[..prefix_count]` 内 ⟹ `projected[..prefix_count]`
不变；`idx == prefix_count` 首次读到可能新的 `moves[prefix_count]`，是首个可能脏点。Codex 复核一致
（stance `dirty_from_prefix_count_correctness: accept`、`fold_prev_dependency_no_minus_one: accept`）。

## 2. per-level dirty_from 是否完全替代 did_extend、覆盖 had_emitted_window pop 的全部尾部改写路径

**verdict: partial**

- 对**连续每 bar 都处理**的级别：confirmed。pop 只删末 1 个（mod.rs:1031-1032 `.pop()`），
  `prefix_count = lc.upper_moves.len()`（pop 后 extend 前取值，mod.rs:1044）精确定位不可变前缀，
  did_extend 全局布尔漏判的 bar-1464 型场景（tail 空但 frontier 值已变）被 per-level 下标消除。
- 对**曾被 `min_parts`/`units.is_empty()` 早停跳过、之后恢复处理**的级别：refuted，见第4条。
  这不是 did_extend 类漏判的重现，是 dirty_from 方案本身在早停恢复路径上引入的**新**缺口。

## 3. 04 truncate+extend 终长约束在 units 回缩时是否恒成立；三重 min 是否遗漏边界

**verdict: confirmed**

`keep = dirty_from.min(lc.cached_units.len()).min(units.len())` ⟹ `keep <= units.len()` 恒成立。
`truncate(keep)` 后 `extend_from_slice(&units[keep..])` 补齐到 `units.len()`，故
`cached_units.len() == units.len()` 恒成立，含 units 回缩情形（keep 同步被 `.min(units.len())` 夹住）。
Codex 交互未反驳此条，审计工位独立代数核验通过。

## 4. 03 stable 从 0 抬到 dirty_from（L1+）是否漏检 cascade 触发的 frontier mutation

**verdict: refuted**（在早停恢复边界条件下）

真实坍缩路径：当某级别被跳过 M bar 后恢复，其 `lc.scan_cursor.consumed`/`lc.cached_units.len()` 是
M bar 前的陈旧值 ⟹ `scanned = (consumed+2).min(units.len()).min(cached_units.len())` 被陈旧值压得很小；
而 `dirty_from`（父级**本 bar**连续处理算出的 `prefix_count`）可能远大于该陈旧 `scanned`。
`stable = dirty_from.min(scanned)` ⟹ `stable` 被下压到 `scanned` ⟹ `[stable..scanned]` 坍缩为**空区间**
⟹ `frontier_mutated` 无条件 `false`（空切片必等），即使该级真实数据已发生 03 本应检出的变异也不会触发
cascade。

对照现状：当前生产代码 L1+ 恒 `stable=0 <= scanned`，比较区间永不为空，此坍缩路径**不存在**——
这是 A3 §3.3 引入 dirty_from 后**新增**的失效面。Codex 交互 stance
`completeness_table_missing_cache_lineage_and_skipped_levels: needs_work`，
`min_parts_early_break_stale_higher_cache: reject`（即设计稿未处理该问题的隐含假设被拒绝）。

**根因定位**：`mod.rs:956-958`（`if units.len() < min_parts { break; }`）与 `mod.rs:1127-1129`
（`if units.is_empty() { break; }`）跳过某 `level_idx` 及更高级别本 bar 处理时，不清空/截断其
`LevelCache`（`cache.levels[level_idx]` 原样保留陈旧 `cached_units`/`scan_cursor`/`last_input_len`）。

**修复建议**（Codex 给出）：早停处显式截断不可达级别缓存——
`units.len() < min_parts` 时 `cache.levels.truncate(level_idx)`；
`units.is_empty()` 投影后 `cache.levels.truncate(level_idx + 1)`。
级别恢复处理时 `cache.levels.len() <= level_idx` 触发 `LevelCache::default()`（全量重扫，bit-exact
退化路径），消除跨 skip 的缓存血缘问题。

## 5. project_to_units_resume truncate(prefix_count) 加固：append-only 隐式依赖 cascade 兜底的
   soundness 分析是否成立；证书化是否引入回归

**verdict: partial**

- 核心 bit-exact 论证（pop 非 cascade 时，`frontier_mutated=false` ⟹ `[resume_from..consumed]` 数据
  未变 ⟹ 重扫确定性复现 `T>=1`，且首个 tail 元素与被 pop 元素逐字段相同，含 `id.ordinal` 复用不会
  错位）：confirmed。Codex stance `audit_id_ordinal_reuse: accept`。
- 显式 `truncate(prefix_count)` 修正本身**不引入**回归（把隐式"陈旧值=新值"假设改为显式重算，只会
  更安全，不会更不安全）：confirmed。
- 但该修正**继承**第4条的同一根因——若不先修复"早停跳过级别的缓存血缘"，`lc.upper_moves`/
  `lc.projected_units` 在恢复处理时同样可能与陈旧缓存不匹配。Codex stance
  `audit_implicit_append_only_invariant: needs_work`：若不落地 §2.5，当前隐式不变量需要补内容级
  debug/test 锁；若落地 §2.5，也要补 `projected_units == project_to_units(upper_moves)` 的定向锁。
  故整体判 partial，不是纯 confirmed。

## 6. 稀疏变异定向 oracle 设计（§5.3）是否足以锁住 pop-and-rescan 覆盖弱点

**verdict: refuted**

现有 `project_to_units_resume` 测试（recursive_tower.rs:803 附近）只覆盖 append-only/clear 两种路径，
未覆盖 §2.5 新提议的 `truncate(prefix_count)+append`；bar-1464 相关测试
（incremental.rs:520/1071）多为 ignored/诊断性质，非 always-run 锁。§5.3 描述的"稀疏变异事件对拍"
方向正确但未具体到必需的分支：`had_emitted_window` 下 `T==1`（重扫仅复现被 pop 窗口）与 `T>1`
（重扫产出更多）两个分支、"长度回缩后再次达到 min_parts/级别跳过后重入"路径。
Codex stance `project_to_units_truncate_test_coverage: needs_work`。

**修复建议**：新增 always-run 合成 oracle，逐 bar 断言 `cached_units == units`、
`projected_units == project_to_units(upper_moves)`、证书路径输出与强制全量路径 bit-identical，显式
构造上述缺失分支（尤其级别跳过后重入，这是当前设计和现有测试都未覆盖的新增风险面）。

## 7. 任务(a) 05 拆解 profile-first 定性方案：三子标签+重扫跨度累加器是否足以区分
   H-detect（A4域）/H-detect-bounded/H-clone 三假说

**verdict: confirmed**（设计层面；未含运行时数据验证，L0 认识论等级，符合本阶段"只读+设计"性质）

三子标签（05a_detect_windowed / 05b_tail_centers / 05c_tail_upper_build）分别对应
`compose_level_resume` 内三个构成操作（recursive_tower.rs:409-424：纯检测循环 / centers collect /
subs clone+compose），足以在时间维度上区分"耗时落在检测循环"还是"耗时落在 tail_upper 构建"两大类。
新增第四维（`05_span_sum`/`05_span_max`/`05_call_count`，重扫跨度 `units.len()-start_i`）是区分
H-detect 与 H-detect-bounded 的必要且充分手段：跨度随窗口规模线性增长 ⟹ H-detect（O(n²)续扫，转 A4
域）；跨度 O(1) 而 05a 时间仍高 ⟹ H-detect-bounded（不可约）；05a 低而 05c 高 ⟹ H-clone（可 A3 延伸压缩）。
三个假说两两互斥、插桩设计的观测量（时间×3 + 跨度分布）恰好张成能分辨这三个假说的最小观测空间，
无冗余也无遗漏。此项未经 Codex 复核（本次 Codex review 聚焦 §2/§3，§1 未纳入 prompt），标注为审计
工位独立设计层判定，非 codex-confirmed。

---

## 汇总

| 项 | verdict |
|---|---|
| 1 dirty_from 核心公式 | confirmed |
| 2 did_extend 替代完备性 | partial（早停恢复路径 refuted） |
| 3 04 终长约束 | confirmed |
| 4 03 stable 抬升 soundness | **refuted**（早停恢复场景比较区间坍缩为空） |
| 5 project_to_units_resume truncate 加固 | partial（继承第4条根因，测试未锁） |
| 6 稀疏变异 oracle 覆盖 | **refuted**（未覆盖 T==1/T>1/级别重入） |
| 7 05 拆解插桩设计 | confirmed（设计层，独立判定，未经 Codex） |

**阶段2 实施解锁条件（未满足）**：第4条为 refuted 核心声明 ⟹ 未过审。A3 工位需修订设计稿：
(i) §2/§3 补"级别早停缓存血缘"失效边界 + 采纳 `cache.levels.truncate(...)` 修复；
(ii) §5 补 always-run 合成 oracle 覆盖 T==1/T>1/级别重入路径。修订稿需再次过 codex 审。

## 结果包（简化版）

1. **结论**：7 项审查点中 1 项 refuted（03 stable 抬升在级别早停恢复场景下比较区间坍缩为空，漏检
   frontier mutation）、1 项测试覆盖 refuted（稀疏变异 oracle 未覆盖新增风险分支）、2 项 partial（继承
   同一根因）、3 项 confirmed。总裁决 partial，未解锁阶段2实施。
2. **边界条件**：若 A3 工位采纳早停缓存截断修复（`cache.levels.truncate`）并补齐 oracle 分支，第4/6条
   转为 confirmed，可解除阻断；若能提供生产数据证明"某级别在实测数据分布下从不会被跳过后又恢复"，
   该缺口的**触发概率**可能趋零，但这不构成不修的理由——形式化上仍是证书失效路径，且当前无此类经验
   证据。
3. **影响声明**：零代码改动、零 git 操作。本文件是既有 Codex 交互
   （`.chanlun/review-results/codex-review-20260702-181938-451b.md`）与审计工位独立代数核验的整理落盘，
   未新增 Codex API 调用。
