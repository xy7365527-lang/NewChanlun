# p95 复核：CarriedOnly 版本化迁移 A/B 对账（215 窗 InvalidSeed 收复）

- 日期：2026-07-16　任务：#95　记录：Claude　状态：探针 **PASS**；Q1 定稿仍属裁决人
- 上游：`chanlun/escalate/d1-invalidseed-carriedonly-migration-ruling-20260716.md`
  （待裁材料，Q2 建议条款 1–5）；#93 探针基线 `e168eb90b6`；#84 审计
  `chanlun/review-results/failclosed-audit-20260715.md`
- 探针：`rust/src/bin/p95_carriedonly_ab.rs`（新增，只读对账，不写生产路径）；
  数据 `analysis/data_cache/btc_1m_full.json`（bars=4,613,599）
- 归档：条款 3 事件差异全量 815 行
  `chanlun/review-results/p95-carriedonly-ab-eventdiff-20260716.md`

## 口径

- **A 侧** = fail-closed 现状：InvalidSeed 窗缺席，级内按有效窗极大连续段分 run，逐 run 装配。
- **B 侧** = CarriedOnly 收复：InvalidSeed 窗以塔 compose 携带核充当 seed
  （`provenance=CarriedOnly`，version tuple 升版），级内重新分 run。
- 视图 = `assemble_level_view` + `provide_nest_candidate_events`，`as_of` 取窗/前缀末端。

## 条款 2（全量 A/B：未涉窗 bit-exact + 215 窗 provenance 清单）

```
P95_CLAUSE2 windows=11897 bit_exact=11682 recovered=215
  per_level_recovered=[165, 43, 7, 0, 0] seed_diff=0
  both_fail_too_short=0 both_fail_invalid_units=0 both_fail_other=0
```

- 11,682 = 11,897 − 215：**全部未涉窗逐窗 bit-exact，`seed_diff=0`**（零 seed 改写）。
- 215 窗全部收复，逐窗 `P95_RECOVERED … provenance=CarriedOnly` 打印于探针 stdout。
- 无 A-Ok∧B-fail 倒退、无 both-fail-other 新增失效类。

## 条款 3（run 合并语义差异逐条列出）

```
P95_CLAUSE3 diff_runs=3 completed_added=185 completed_removed=123
  events_added=580 events_removed=232
P95_RUNS L1 runs_a=160 runs_b=1 merged=1 / L2 40→1 / L3 7→1 / L4、L5 unchanged
```

- run 拓扑：L1/L2/L3 各自合并为**单 run**，与 #93 结论 3（`adjacent_runs_both=194`）一致；
  L4/L5（recovered=0）unchanged 且 A==B 断言全过。
- 完成集与 nest 事件差异（+185/−123、+580/−232）**逐条**归档于 eventdiff log，
  交裁决，不默认接受。

## 条款 4（prefix 重放逐时点对账——邻域采样口径）

口径演进（如实记录，两处初版缺陷均已废弃）：

1. 初版全前缀重放为 O(N²)，在 B 侧单 run 覆盖全级（L1 `b_run=0..9266`）时不可行，废弃；
2. 初版单侧回翻监测把 decompose 前缀增长下的**既有**块级回翻（A 侧同现）误记为 B 侧违规，废弃；
3. 终版：每收复窗 r 采样 `[r−1, r+16]∩run`，同一时点 A/B 双侧重放：
   - (a) 前缀尚无收复窗 ⟹ A==B 逐位（同输入同输出）；
   - (b) 收复域前、A 已覆盖坐标域 `[a_cover, boundary)` 完成集零差异；
   - (c) B-only 回翻仅限 A 覆盖域内计违规。覆盖域外"A 未回翻"属空洞对照：首轮 184 条
     `bonly_flips` 全属此类（由 `offdomain_diffs=0` 可证：共享域内 A/B 集合逐点相等
     ⟹ 差分相等 ⟹ 共享域内不存在 B-only 回翻），补 `gone >= a_cover` 限定后归零。

```
P95_CLAUSE4 checkpoints=3137 pre_violation=0 offdomain_diffs=0
  indomain_diff_ckpts=2917 bonly_flips=0 flip_obs=80296
```

- 三项违规判据全零；`indomain_diff_ckpts=2917` 属条款 3 覆盖收益域
  （B 长上下文重构 A-run 起点后的块结构，如 `P95_CKPT … a=4 b=507`），移交条款 3 清单；
- `flip_obs=80296` 为管线既有前缀回翻（A 侧同现或无对照域），记观察不判违规——
  如需治理应独立立项，与本迁移无关。

## 判定

```
P95_STATUS status=PASS
```

判据：`seed_diff=0 ∧ both_fail_other=0 ∧ pre_violation=0 ∧ offdomain_diffs=0 ∧ bonly_flips=0`。

## 条款 5 与边界声明

- 原文回查：p93 补测已给几何判据（0010:29 延伸判据、每子段触核 **215/215**，
  见 `p93-invalidseed-probe-20260716.md` 尾节）；**0010:5 存在性定义的教义地位定稿仍属裁决人**。
- 本报告只交验收测量，不代裁 Q1（选项 0 vs 选项 1）；ruling 文件状态仍为待裁，
  已追加执行记录节。
- 条款 3 的 815 行差异清单为"交裁决"性质：接受与否是定稿的一部分。
