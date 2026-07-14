# goal 规格：S_Θ 完整闭合与生产化

> 复制下方「/goal 命令描述」整段，粘贴到 prompt 跑 `/goal <描述>`。Lead 据此 append GOAL_SET（acceptance）+ DECOMPOSE（sub_goals）。
> 或者让我直接 append JSON 到 events.jsonl（文末备好）。

## /goal 命令描述（复制这整段用）

```
S_Θ 完整闭合与生产化。Q4 确定性 ElementId 层（对齐 spec §13 结构映射 p:C_ℓ→C_{ℓ+1}，修 Stale 伪造 parent:None 的 AncOK 放宽——codex 发现 A）→ #5 alpha 坐实（ΔSharpe 非零，depth>0 对冲腿准入）→ L2 全窗 8 品种否证/确认（n_beats_random 双口径 + 分层诊断：引擎产不出信号 vs 产信号不盈利）→ O(n) 大规模验证（全引擎 per-bar exp≈1.0 @16K，非 n=1000 小窗）→ 接 NautilusTrader 生产引擎（数据流 Nautilus→S_Θ→订单 + CLI 生产入口，回测与实盘同一 S_Θ 引擎）。acceptance 五项全可证伪：(1) cargo test --lib 退出码0 + ElementId 层 bit-exact；(2) 至少 1/8 品种 depth>0 腿准入>0 + ΔSharpe≠0；(3) 8 品种 OOS 跑通 + n_beats_random + 分层诊断；(4) 全引擎 per-bar exp≈1.0 @16K；(5) nautilus 依赖 + 数据流贯通 + CLI 生产入口。sub_goals DAG：q4-id-layer→delta-sharpe-retest→l2-fullwindow-falsify→nautilus-integration，perf-fullwindow-verify 并行。
```

## acceptance（5 项，锋利可证伪，机器判定）

| # | check（机器可判定） | falsifiable | 怎么判 |
|---|---------------------|-------------|--------|
| 1 | Q4 确定性 ElementId 层：cargo test --lib 退出码0 + grep ElementId coverage.rs 命中 + bit-exact（增量==全量，spec §13 parent_id 闭包） | true | cargo test 退出码 + grep + bit_exact_per_bar |
| 2 | ΔSharpe 非零：至少 1/8 品种 depth>0 腿准入数 >0 + ΔSharpe ≠ 0.000 | true | l3_pi_depth_diag_cl_btc 输出 |
| 3 | L2/L3 全窗：8 品种 OOS 跑通 + n_beats_random 双口径 + 分层诊断（引擎产不出 vs 不盈利） | true | l3_pi_falsify_multi_symbol 输出 |
| 4 | O(n) 大规模：全引擎 per-bar exp ≈1.0 @16K（profile_incremental_tower_real_scaling 大规模，非 n=1000） | true | profile 输出 exp 字段 |
| 5 | Nautilus：nautilus 依赖集成 + 数据流 Nautilus→S_Θ→订单 + CLI 生产入口 | true | nautilus import + 端到端跑 + CLI main |

## sub_goals（DECOMPOSE，DAG）

| id | desc | blocked_by |
|----|------|-----------|
| q4-id-layer | 确定性 ElementId 贯穿 LeveledMove→CoverageElement→ActiveLeg；Stale 不伪造 parent:None（改 prune）；held_leg_tree_index 按 ElementId 跨 bar 匹配（spec §13 结构映射） | [] |
| delta-sharpe-retest | ΔSharpe 重测 CL/BTC 32K + 8 品种；验证 depth>0 腿准入 + ΔSharpe 非零 | [q4-id-layer] |
| l2-fullwindow-falsify | 8 品种 OOS 全窗否证/确认；n_beats_random 双口径 + 分层诊断（引擎产不出 vs 不盈利） | [delta-sharpe-retest] |
| perf-fullwindow-verify | 全引擎 per-bar exp 大规模验证 @16K（非 n=1000 小窗） | [] |
| nautilus-integration | nautilus 依赖 + 数据流 Nautilus→S_Θ→订单 + CLI 生产入口（回测与实盘同引擎） | [l2-fullwindow-falsify] |

## ready_workstations（reducer 算）

- q4-id-layer（blocked_by=[]）—— **当前 a33c7aaf 在跑**
- perf-fullwindow-verify（blocked_by=[]）—— 可并行

## 与当前 goal 的关系

当前 `g-strict-mutex-classification-strategy`（acceptance 0/3）聚焦 Lean L0 形式化 + rust 七链实装。本 goal **承接**它——Lean L0 已大部分 discharge（§3/§16/§18 步1/步2），rust 七链已实装 + O(n) 达成，本 goal 聚焦剩余的 **#5 alpha（Q4）+ L2 验证 + Nautilus 生产化**。

建议：用 SUPERSEDE 事件把 g-strict-mutex 标为被本 goal 替代（g-strict-mutex 的 acceptance[1]Lean/acceptance[2]rust 已基本达成，acceptance[3]结果包/reconcile 并入本 goal 的 L2 验证）。

---

## 备用：直接 append 的 JSON（如果让我设而非 /goal）

```jsonl
{"event":"SUPERSEDE","old_goal_id":"g-strict-mutex-classification-strategy","new_goal_id":"g-sigma-complete-l2-nautilus","ts":"2026-06-29T00:00:00Z"}
{"event":"GOAL_SET","goal_id":"g-sigma-complete-l2-nautilus","description":"S_Θ 完整闭合与生产化：Q4 确定性 ElementId 层→#5 alpha 坐实→L2 全窗否证/确认→O(n) 大规模验证→接 NautilusTrader 生产引擎","acceptance":[{"check":"Q4 确定性 ElementId 层：cargo test --lib 退出码0 + grep ElementId coverage.rs 命中 + bit-exact(增量==全量,spec §13 parent_id 闭包)","falsifiable":true},{"check":"ΔSharpe 非零：至少 1/8 品种 depth>0 腿准入数>0 + ΔSharpe≠0.000","falsifiable":true},{"check":"L2/L3 全窗 8 品种 OOS 跑通 + n_beats_random 双口径 + 分层诊断(引擎产不出 vs 不盈利)","falsifiable":true},{"check":"O(n) 大规模：全引擎 per-bar exp≈1.0 @16K(profile 大规模非 n=1000)","falsifiable":true},{"check":"Nautilus：nautilus 依赖集成 + 数据流 Nautilus→S_Θ→订单 + CLI 生产入口","falsifiable":true}],"base_head":"<当前 git HEAD>","ts":"2026-06-29T00:00:00Z"}
{"event":"DECOMPOSE","goal_id":"g-sigma-complete-l2-nautilus","ts":"2026-06-29T00:00:00Z","sub_goals":[{"id":"q4-id-layer","desc":"确定性 ElementId 贯穿 LeveledMove→CoverageElement→ActiveLeg;Stale 不伪造 parent:None(改 prune);held_leg_tree_index 按 ElementId 跨 bar 匹配(spec §13 结构映射)","blocked_by":[]},{"id":"delta-sharpe-retest","desc":"ΔSharpe 重测 CL/BTC 32K + 8 品种;验证 depth>0 腿准入 + ΔSharpe 非零","blocked_by":["q4-id-layer"]},{"id":"l2-fullwindow-falsify","desc":"8 品种 OOS 全窗否证/确认;n_beats_random 双口径 + 分层诊断(引擎产不出 vs 不盈利)","blocked_by":["delta-sharpe-retest"]},{"id":"perf-fullwindow-verify","desc":"全引擎 per-bar exp 大规模验证 @16K(非 n=1000 小窗)","blocked_by":[]},{"id":"nautilus-integration","desc":"nautilus 依赖 + 数据流 Nautilus→S_Θ→订单 + CLI 生产入口(回测与实盘同引擎)","blocked_by":["l2-fullwindow-falsify"]}]}
```
