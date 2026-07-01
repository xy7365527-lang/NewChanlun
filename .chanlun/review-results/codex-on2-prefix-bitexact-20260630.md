# Codex 异质审计：#106 前缀投影缓存 bit-exact 语义

- **模式**: review（代码层异质否定审查）
- **审计对象**: `project_to_units_resume` (recursive_tower.rs:417) + `projected_units` 缓存机制 (mod.rs:390/949/1046)
- **日期**: 2026-06-30
- **异质源**: OpenAI Codex (codex-cli 0.125.0), sandbox read-only
- **审计 Task**: #107（独立于 on2-hotspot 工位的真热点定位）

## 1. 结论

**bit-exact 成立**（生产路径上 `project_to_units_resume` == `project_to_units` 任意输入）。
Codex 未找到生产反例。我对该否定性结论执行反质询后确认其与代码一致。**#106 落地改动可 commit。**

### 三问裁定

| 问题 | Codex 裁定 | 代码核实 |
|------|-----------|----------|
| Q1 前缀真不可变？ | 是。`upper_moves` 只 cascade 时 clear、否则尾部 extend，无原地/深层改写路径。`Rc<Vec<>>` 是共享不可变存储非内部可变性 | mod.rs:944/980 核实：`Rc::make_mut().clear()` (cascade) / `Rc::make_mut().extend()` (append)。无 RefCell/Cell |
| Q2 深层 subs 改写漏投影？ | 否。投影值只依赖 `{start_index, end_index, lo, hi, prev.hi}`；`lo()/hi()` 把整棵子树约简为 min/max 聚合。深层改写保持聚合不变 ⟹ 全量投影本身也 bit-identical（resume 不漏）；聚合变 ⟹ 低级别结构可见 ⟹ 触发 cascade | recursive_tower.rs:79-92 (lo/hi 递归 min/max)、164-177 (fold_direction 只读 hi)、393-408 (project_to_units 只读这 5 量) 核实 |
| Q3 cascade 覆盖所有前缀投影变？ | 是。`projected_units.clear()` 在 cascade_reset 块内与 `upper_moves.clear()` 同步 ⟹ resume 退化全量。已有反例测试 `cascade_reset_on_frontier_interior_rewrite` 正是验证此路径 | mod.rs:949 (clear)、1046 (resume)、1660 (反例测试) 核实 |

### 关键论证（Q2，否定的核心）

之前 codex 发现的 `cascade_reset_on_frontier_interior_rewrite` 反例是：前缀内点改写使**上级投影 bit-identical 但底层 sub_moves 变**。本审计确认这对**投影缓存本身不构成漏洞**——因为投影值只读子树聚合 min/max，深层改写若保持聚合则全量投影也相同（resume 正确），若改变聚合则在产生该 upper_move 的下级触发 frontier_mutated ⟹ cascade 向上传播 ⟹ projected_units.clear()。投影缓存的失效完全绑定 cascade，无独立失效路径。

## 2. 边界条件（在什么条件下结论翻转）

1. **契约违反翻转**：若调用方填 cache 为 `[A,B]` 后原地改写 A/B 再 resume `[A',B,C]` 而不 clear → resume 从 `cache.len()` 续投影，与全量发散。但这是 resume 的**契约前条件违反**（`debug_assert!(cache.len() <= moves.len())` + cascade_reset 守卫），唯一生产调用点 (mod.rs:1046) 前必经 cascade_reset，**不可达生产**。
2. **若未来引入 RMove 内部可变性**（RefCell/Cell 包裹 subs）→ Q1 前提崩塌，前缀可被静默原地改写而不经 extend/clear → 翻转。当前无此模式。
3. **若 cascade_reset 块未来移除 `projected_units.clear()`** → Q3 翻转，前缀重排后投影缓存陈旧。当前 line 949 在位。

## 3. 影响声明

- **审计范围**：read-only，未改任何代码。
- **被审改动**：`recursive_tower.rs` +52 行（resume 函数 + test）、`mod.rs` +13 行（projected_units 字段 + cascade clear + 调用点）。
- **涉及模块**：`theta_v0::classifier`（recursive_tower / mod）。下游 runner/interp/l3 消费 tower_snapshots，bit-exact 保证其不受性能优化影响。
- **测试覆盖核实**：
  - `project_to_units_resume_matches_full`（recursive_tower.rs:765）：三批增量追加 + cascade clear 退化，逐字段对比 full。覆盖 append 单调增长 + clear 退化。
  - `cascade_reset_on_frontier_interior_rewrite`（mod.rs:1660）：覆盖"前缀深层 subs 改写"边界——这是 Lead 问的关键边界，**已被现有测试覆盖**（非仅测 append 单调增长）。
  - `bit_exact_synthetic` 2000bar 逐 bar 增量 == legacy 已过；cargo --lib 1303 passed。
- **未验证项**：Codex 未重跑 cargo（read-only sandbox）。这是源码审计而非运行时验证；运行时由已过的 1303 tests + bit_exact_synthetic 兜底。

## 完整 codex 交互
prompt: /tmp/on2_audit/ctx.md（三层代码：投影函数 + lo/hi/fold_direction 依赖 + cascade_reset 机制）
codex 自主导航补读了 segment_to_unit、TowerCache::clear、cascade 反例测试，裁定无生产反例。
