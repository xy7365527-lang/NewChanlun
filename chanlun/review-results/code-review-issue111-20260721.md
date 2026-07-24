# Code Review — Issue #111 身份桥多级查询扩展（expand）

- 日期：2026-07-21
- 审查基线：工作区 diff（HEAD `640609071d`，32 files changed, 6400 insertions(+), 4253 deletions(-)）
- 参照：issue #111 / SPEC #109 / `chanlun/review-results/issue110-impl-20260721.md`
- 审查方式：只读双轴（Standards + Spec）。未跑 cargo（审查约束禁止），测试绿与否见条件 C1。

## 结论：PASS WITH CONDITIONS

新多级查询路径实现正确、并存不接线、因果护栏成立；条件为测试绿证据缺失（C1）与 anchor 坐标系类型弱约束（C2）。

## 关键验收点核验

### 1. 旧固定 lvl+1 路径零变化（expand 并存）✓
- `admission.rs:588` `typed_lookup`：仍走 `by_end` 固定键（lvl+1, source_index, is_long），rungs 计算 `cert.judge_at().len() - 1`（:603）语义未动。
- `admission.rs:687` `admit`：门仍**只**消费 `typed_lookup`，未触碰 multi 路径。
- `admission.rs:572` `has_bridge_key`：未改动。
- 并存回归测试：`runner.rs:2686` `nest_chain_multi_level_lookup_coexists_with_fixed_path`。

### 2. 新多级路径未被门消费（留给 #112）✓
- `admission.rs:248/:261` `MultiLevelTypedHit` / `MultiLevelTypedLookup` 与 `:627` `typed_lookup_multi` 均标注 `#[allow(dead_code)] // expand 阶段：门消费 #112 接线前无调用方`。
- 全仓 grep：生产侧零调用方；全部调用位于 `runner.rs:897–898` `#[cfg(test)] mod tests` 内（:2640/:2669/:2686 等）。
- `econ_positive.rs:682` `build_multilevel_nest_cert` 为 #105/#106 W1 返工的判定核，非本 issue 查询路径，不构成第二消费点。

### 3. 因果护栏（090 无未来函数）✓
- `admission.rs:648`：`cert.judge_at().iter().any(|&t| t > anchor_index)` ⟹ 整证剔除（文档 :622 明示"任一越界即整证剔除"）。
- `admission.rs:512–513` `absorb_exts`：身份去重保首次观察、`judge_at` 不后移——索引内容本身因果单调。
- 测试：`runner.rs:2669` `nest_chain_multi_level_lookup_causal_guard`（:2678 anchor=99 拒、:2680 anchor=100 收）。

### 4. 禁第二查法 ✓
- 多级键域 `by_end_multi:(source_index, is_long)`（`admission.rs:317`）唯一写点在 `absorb_exts`（:531），与 `by_end` 同源同批吸收，无旁路索引。
- 值桥语义逐字复用旧路径（文档 :617–618：`source_index==seg_c_full.1`、side 同向）；新增的仅是级别轴扫描 `ℓ ≥ level_origin+1`（:643 `<=` 剔除，测试 :2660 "只向更深级别扫描"、:2662 side 同向、:2663 值桥）。未引入新的桥判据。

## 发现

| # | 严重度 | 位置 | 内容 |
|---|--------|------|------|
| C1 | MEDIUM（条件） | 全局 | #111 无落盘实施报告，`cargo test --release --lib` 全绿证据缺失（#110 基线为 `1824 passed; 0 failed; 132 ignored`）。审查方被禁跑 cargo，无法代验。**条件：补跑并归档测试输出，确认零变红。** |
| C2 | MEDIUM（条件） | admission.rs:628–648 | `anchor_index: usize` 为裸值，`judge_at` 与 anchor 的坐标系（合并K线索引 vs source index）仅靠文档约定。#112 接线若误传坐标，因果护栏静默失效。**条件：#112 接线时以 newtype 或断言固定坐标系，并加跨坐标回归测试。** |
| F1 | LOW | 工作区 diff | #111 与 #105/#106、#97-supersede（nest.rs rung 门谓词收紧）等混在同一未提交 diff 中，回归归因困难。建议分 issue 提交。 |
| F2 | INFO | admission.rs:353/:376 | `by_end_multi` 在两个构造点均初始化，无遗漏。 |
| F3 | INFO | runner.rs:2651–2663 | 边界测试覆盖：级别下限、side、值桥、深层可达（:2640）、rungs 归因（:2702）。覆盖面充分。 |

## 判定

**PASS WITH CONDITIONS**（C1 测试绿证据、C2 anchor 坐标类型约束随 #112 落实）。expand 阶段设计目标——旧路径零变化、新路径并存不接线、因果与桥语义不放宽——全部有代码与测试证据支撑。
