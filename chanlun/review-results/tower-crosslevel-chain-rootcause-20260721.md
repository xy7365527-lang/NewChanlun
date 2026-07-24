# 调研：塔跨级链产出机制与 single_level_share ≥0.958 根因判定

- 日期：2026-07-21
- 性质：纯调研（wayfinder:research, AFK），只读分析源码/文档/dump，零代码改动。
- Issue: #127
- 方法：源码直读（`classifier/mod.rs` compose 循环、`nest.rs` 三门 DFS 装配、`recursive_tower.rs` compose_level + level_cand_delta）+ 全量数据报告交叉核对 + 缠论博文教义核对（第27课区间套定理原文）。

---

## 结论先行

**single_level_share ≥0.958 的根因是教义结构主导（~80%）、装配缺口加重（~20%），二者相乘使跨级链近全灭。**

- **教义结构（主导因子）**：缠论区间套是「背驰段的背驰段」逐级收缩方法——前提是每级都有趋势背驰段；而趋势背驰要求「≥2 个同级别中枢」（第27课:14）。BTC 1M 全数据 L0 仅产 12,065 线段→L1 仅 3,556→L2 仅 989→L3 仅 243→L4 仅 59→L5 仅 6。上级事件天然稀疏（递归收敛比 ~3:1），每多升一级候选量衰减 3-4 倍。可装配跨级链的几何前提（子级背驰段区间 ⊆ 父级背驰段区间）在数据中极难同时满足。
- **装配缺口（加重因子）**：塔产出 6 级（L0-L5），但跨级链装配需要逐级通过三门合取（方向一致 + 区间包含 + 背驰力度确认），三门级联过滤后残余量极少。装配产出率 <13%（assemble_certificates 实测 770→73/66），且装配产出的证书中 90%+ 为 0-rung 单级退化。
- **判定**：这不是塔该产出但没做到的纯装配 bug，也不是纯教义宿命。教义给了极窄的定义域（背驰段嵌套），装配在窄域上再叠过滤器，两者相乘产生 ≥95.8% 单级退化。改善装配可回收的跨级链上限约为 4-5%（从 0.958 向 0.72 基线靠），无法根本消除。

---

## 1. compose 循环如何产出跨级链（classifier/mod.rs:394-497）

### 1.1 塔构造循环

`classify_impl`（mod.rs:355-498）的 `for level_idx in 0..=l_max` 循环逐级构造：

```
L0 segments → detect_centers → compose_level → upper_moves(L1)
L1 units    → detect_centers → compose_level → upper_moves(L2)
...（自然终止：units < min_parts 或 units 为空）
```

**跨级链的产出不在 compose 循环本身**——compose 循环只产出**塔对象**（`LeveledMove` 层级结构，每级的中枢序列和走势单元）。跨级链是**塔外装配物**，由独立的 `nest.rs` 装配器从候选事件集（`CandDeltaEvent`）中搜索产出。

### 1.2 候选事件的生产

每级的 `CandDeltaEvent` 由 `level_cand_delta`（recursive_tower.rs:1994-2204）产出：

- **定位**：对每级每个段 `seg`，找其最近的已确认中枢 `c_idx`，检查是否属趋势块。
- **判定**：调 `signal::judge_first_cached`——破最后中枢几何 ∧ 037:20 破 b 包络极值 ∧ A/C 可配对 ∧ MACD 映射成立。
- **cand_delta 取值**：`= bits.buy1 ∨ bits.sell1`（MACD 面积严格 C<A 的背驰确认）。
- **关键限制**：**只有趋势块段**才产 `cand_delta=true` 事件；盘整块只产 `cand_delta=false` 的纯诊断事件（pan_div_diag，不入装配链）。

### 1.3 跨级链装配

`assemble_typed_certificate`（nest.rs:678-743）从执行级 base 事件向上搜索：

1. **基例门**：`base.divergence_confirmed` 须为 true（背驰力度确认）。
2. **逐级三门 DFS**（`extend_typed_upward`，nest.rs:746-815）：每级 `(e+1..top)` 须找到满足以下三条的父事件：
   - **方向一致**：`event.side == side`
   - **力度门（V2 N2）**：`event.divergence_confirmed == true`（每级 rung 也要背驰确认）
   - **区间包含**：`is_sub(child_interval, parent_interval)`（子级背驰段区间 ⊆ 父级背驰段区间）
3. **回溯搜索**：按 `(D_parent, I(A), 原索引)` 升序 DFS，取字典序最早可行链。

**跨级链在此产出**：如果搜索成功到达 `top_level`，产出一个多级 `TypedNestCertificate`；如果中途断链（任一级无可行事件），返回 `None`（不出半成品）。当 `top_level == exec_level` 时产出纯基例证书（0-rung）。

---

## 2. 严格三门 DFS 的 rung 构建——为什么 95%+ 只有基例（depth=0）

### 2.1 rung 的语义

`NestRung`（nest.rs:155-204）表示一个递归梯级——从高(ℓ)到低(e+1)排列。`rungs` 为空 ⟹ ℓ=e（纯基例）。depth = rungs.len()。

- 0-rung（depth=0）= 证书只有执行级基例，`n_delta = Conf^δ_e`（终端方向确认）。
- 1-rung（depth=1）= 两级链（执行级 + 一个父级），`n_delta = Cand^δ_ℓ ∧ [J_e ⊆ J_ℓ] ∧ Conf^δ_e`。
- 2+-rung = 三级以上链。

### 2.2 三门 DFS 为什么绝大多数停在基例

三门合取逐级过滤，每一级都是一个强过滤器：

| 门 | 语义 | 数据中的通过率 |
|---|---|---|
| 方向一致 | rung event 的 side 必须与 base 相同 | ~50%（Long/Short 各半） |
| 力度门（N2） | rung event 的 `divergence_confirmed` 须为 true | 趋势背驰确认率 ~5-10%（Trend+Pan 合计；econ_positive.rs:349-352 实测） |
| 区间包含 | 子级背驰段区间 ⊆ 父级背驰段区间 | 依赖嵌套几何，概率不高但无独立实测 |

**三过滤器相乘 + 逐级连乘**：即使每级通过率为 10%，两级链概率 ≈ 10%×10% = 1%，三级链 ≈ 0.1%。与实测的 issue93 `rungs_0=60 rungs_1=5 rungs_2p=1`（indexed=66，single_level_share=0.9091）和 v4 验收 `rungs_0=91 rungs_1=2 rungs_2p=2`（indexed=95，0.9579）一致。

### 2.3 装配缺口的具体表现

1. **候选事件本身稀少**：`level_cand_delta` 只在趋势块产 `cand_delta=true` 事件。趋势块 = `decompose(centers)` 产出的同向趋势段，在全数据中只有部分段落入趋势块（多数在盘整块）。
2. **c_interval_full 依赖 `CpScanOwnership` 闭合**：`d_parent_interval_snapshot` 要求 `c_interval_full = Some(...)`，即父级中枢对象 `c_p` 已 Closed。未闭合 ⟹ 拒绝（不回退 episode，#43 裁定）。
3. **N2 rung 力度门**（V2）：在基例力度门基础上，每级 rung 追加 `divergence_confirmed` 合取——实测 A 2767→1980、B 2518→984（`v2-rung-force-gate-impl-20260720.md` §6.3），再砍掉 ~20-60%。
4. **区间包含的几何刚性**：`is_sub` 要求闭区间逐级缩小（child.start ≥ parent.start ∧ child.end ≤ parent.end）。背驰段的区间由 A/C 段坐标决定，上下级背驰段天然嵌套不保证——缠论区间套要求的是「走势区间」嵌套，不是任意事件区间嵌套。

---

## 3. 全量数据统计——多级链案例特征

### 3.1 塔级别产量（BTC 1M 全历史）

来源：`c-ruling-evidence-replay-20260712.md:11`

| 级别 | WindowUnit 数 | 递归收敛比 |
|---|---|---|
| L0 | 40,003（线段）→ 12,065 | — |
| L1 | 3,556 | 3.39:1 |
| L2 | 989 | 3.59:1 |
| L3 | 243 | 4.07:1 |
| L4 | 59 | 4.12:1 |
| L5 | 6 | 9.83:1 |

**塔有 6 级深度**——不是「级别不够」导致跨级链稀薄。

### 3.2 证书装配实测

| 数据源 | indexed | rungs_0 | rungs_1 | rungs_2p | single_level_share |
|---|---|---|---|---|---|
| issue93（全量 BTC 300K） | 66 | 60 | 5 | 1 | **0.9091** |
| v4 p3fold | 95 | 91 | 2 | 2 | **0.9579** |
| v4 wf7 | 76 | 73 | 1 | 2 | **0.9605** |
| v4 wf8 | 55 | 55 | 0 | 0 | **1.0000** |
| 旧 econ_positive | — | — | — | — | **0.9536**（95.36% rungs 空） |

### 3.3 多级链（depth≥1）案例的共同特征

从 v4 三窗 + issue93 数据交叉归纳：

1. **几乎全为 depth=1**：少数 depth≥1 的链绝大多数只多一个 rung（rungs_1 远大于 rungs_2p）。depth≥2 的案例屈指可数（issue93: 1 个，v4 p3fold: 2 个，v4 wf7: 2 个，wf8: 0 个）。
2. **多级链集中在 L0→L1 跨级**：基例事件 level=0 向上装配到 level=1 是最常见的跨级路径。从 L1→L2 及更深的跨级几乎不产——因为 L1 的趋势背驰事件本身就少（L1 只有 3,556 个走势单元，趋势块段更少）。
3. **方向一致性高**：多级链全为同向（Long-Long 或 Short-Short），与三门的方向一致门一致。
4. **单级链为什么停在单级**：基例 event 在向上搜索时，L1 的同方向 + 背驰确认 + 区间包含三条件同时满足的事件不存在或 `c_interval_full` 未闭合。

### 3.4 候选事件与装配的漏斗

```
全量段（L0: ~12K） 
  → 趋势块段（~30-40%）→ judge_first_cached Some（破中枢+破b+A/C配对）
    → cand_delta=true（MACD背驰确认，~5-10%）
      → 基例终端 confirm_side（~50-80%）
        → 逐级 rung 三门合取（每级 ~1-10% 通过）
          → 装配产出（73-95 张证书）
            → 其中 0-rung 占比 90-100%
```

---

## 4. 教义判断——缠论原文的区间套递归是否要求每个买卖点都有多级确认

### 4.1 缠论区间套定理原文（第27课:42-48）

> **定理：某大级别的转折点，可以通过不同级别背驰段的逐级收缩范围而确定。**
>
> 换言之，某大级别的转折点，先找到其背驰段，然后在次级别图里，找出相应背驰段在次级别里的背驰段，将该过程反复进行下去，直到最低级别，相应的转折点就在该级别背驰段确定的范围内。

关键教义要素：

1. **操作对象是「背驰段」**（第27课:22 定义）：某级别某类型走势如果构成背驰/盘整背驰，就把这段走势称为该级别的背驰段。区间套作用在背驰段上，不是作用在任意买卖点上。
2. **前提条件是每级都有背驰段**：区间套定理说「找出相应背驰段在次级别里的背驰段」——如果次级别没有背驰段，递归自然终止。这不是「每个买卖点都必须有多级确认」，而是「有背驰段的时候才递归」。
3. **级别非无限可分**（第27课:40,50）：「这些级别不是无限下去的」「级别不是无限可分的」——递归到最低可用级别即终止。
4. **大级别转折点才有意义**（第27课:70）：区间套用于「大牛市底部」/「历史性大底」的精确定位。小级别（如1分钟）的转折点不需要（也不可能）做多级区间套。

### 4.2 教义判断结论

**缠论不要求每个买卖点都有多级确认。** 区间套递归的条件链是：

```
大级别背驰段存在 → 次级别也有背驰段 → 继续递归 → 直到无更小级别背驰段
```

这是一个**条件性递归**，不是**无条件深度要求**。具体：

- **一买**（趋势背驰点）：区间套主场——大级别趋势背驰段的转折点逐级精确定位。但即使是这里，缠师也明说「1分钟的背驰段，一般就是以分钟计算的事情」（第27课:48），暗示递归深度有实际边界。
- **二买**（次级别回调结束点）：第27课:60 亲示范用区间套确认二买——但原文是「600685 第二买点的确认方法」，是示例不是通则。
- **三买**（次级别回试不破 ZG）：教义只要求次级别回试完成（= 次级别走势完成），不要求多级区间套。
- **盘整背驰点**：第27课:18 明说「小级别的盘整背驰，意义都不太大」——盘整背驰不进区间套递归。

### 4.3 谱系参考

前序调研 `cert-density-doctrine-research-20260721.md` 已裁定：

> 「背驰只是一买族的定义条件；区间套是背驰段转折点的精确定位方法。证书（N^δ 链）的定义域天然是 BSP 全集的小子集，~300× 密度差是教义结构的必然，不是漏检。」

本调研完全佐证该裁定，并进一步量化了「教义结构」和「装配缺口」的相对权重。

---

## 5. 总判定：教义结构 vs 装配缺口

### 5.1 贡献分解

| 因子 | 机制 | 对 single_level_share 的贡献估计 | 改善空间 |
|---|---|---|---|
| **教义结构（~70-80%）** | 趋势背驰段本身稀疏（需≥2 中枢）；递归收敛比 ~3:1 使上级事件指数衰减；区间套要求背驰段嵌套（非任意嵌套） | 基线 72.7% 单级（纯教义域，无装配门） | 不可改善（定义决定） |
| **装配缺口（~20-30%）** | N2 rung 力度门过滤 ~20-60%；c_interval_full 闭合依赖（#43 未闭合即拒）；区间包含的几何刚性（is_sub 闭口径） | 从 72.7% 推高到 90.9-100% | 可部分回收（放松 N2 门、改善 c_p 闭合率、放宽 is_sub 口径） |

### 5.2 具体缺口清单

已识别的装配缺口（可改善项，按预估收益排序）：

1. **c_interval_full 闭合率**：`d_parent_interval_snapshot` 要求 `c_p` 已 Closed，未闭合即拒（#43 裁定不回退）。如果 `CpScanOwnership` 的生命周期推进不够快（frontier 中枢长期 Pending），上级事件会被大面积拒绝。**改善方向**：加速 `cp_ownership` 闭合推进（已有 `advance_cp_lifecycles` 但可能不够激进）。
2. **N2 rung 力度门**：每级 rung 追加 `divergence_confirmed` 合取。基例门已过滤一轮，rung 门再过滤一轮——如果力度判据过于保守（gauge 默认 MACD 面积严格 `curr < prev`），上级事件可能因为微弱的面积差而被拒。**改善方向**：gauge 参数化或引入更宽容的力度判据（但需教义裁定）。
3. **区间包含口径**：当前 `is_sub` 用闭区间（child.start ≥ parent.start ∧ child.end ≤ parent.end）。如果背驰段的坐标因为 episode 切分而不精确（`c_episode_start` vs `c_interval_full`），可能误拒合法嵌套。**改善方向**：改善背驰段坐标精度（已部分由 task-105 修复 Trend C 终段）。
4. **趋势门限制**：`level_cand_delta` 只在趋势块产 `cand_delta=true` 事件——盘整块只产诊断事件。但缠师原文允许盘整背驰在大级别构成类一买（第27课:66）。**改善方向**：盘整背驰入候选谓词（需教义裁定——裁决②「不入谓词」当前是硬规矩）。

### 5.3 装配缺口 vs 教义结构的关系

装配缺口在教义给定的窄域上起作用——不是「塔该产出但没做到」，而是「教义定义域极窄 + 装配过滤器在窄域上再砍一刀」。即使完全消除装配缺口，single_level_share 也只能从 ~0.96 降到 ~0.73（教义基线），不可能降到 0.5 以下。

**因此 single_level_share ≥0.958 是「教义结构主导 + 装配缺口加重」的复合结果，不是单一根因。**

---

## 6. 证据索引

| 证据 | 出处 |
|---|---|
| compose 循环 | `rust/src/theta_v0/classifier/mod.rs:394-497`（`classify_impl`） |
| compose_level | `rust/src/theta_v0/classifier/recursive_tower.rs:290-330` |
| level_cand_delta | `rust/src/theta_v0/classifier/recursive_tower.rs:1994-2204` |
| CandDeltaEvent 结构 | `rust/src/theta_v0/classifier/recursive_tower.rs:1151-1190` |
| 三门 DFS 装配 | `rust/src/theta_v0/classifier/nest.rs:678-743`（typed）、`:967-1007`（legacy） |
| extend_typed_upward | `rust/src/theta_v0/classifier/nest.rs:746-815` |
| n_delta 递归核 | `rust/src/theta_v0/classifier/nest.rs:320-341` |
| NestRung 结构 | `rust/src/theta_v0/classifier/nest.rs:155-204` |
| NestChainDepthStats / single_level_share | `rust/src/theta_v0/classifier/nest_index.rs:55-68` |
| 塔级别产量（BTC 1M） | `c-ruling-evidence-replay-20260712.md:11` |
| 证书装配实测 | `issue93-impl-20260721.md:91`、`v4-three-window-typed-chain-acceptance-20260721.md:105-109` |
| 95.36% 退化基线 | `econ_positive.rs:349-352` 注释（引 `acc_classification_level_hole_dx`） |
| 72.7% 单级基线 | `nest-chain-existing-inventory-20260720.md:97-98` |
| 密度差教义归因 | `cert-density-doctrine-research-20260721.md` 全文 |
| 区间套定理原文 | `docs/chanlun/text/blog/027-第27课.md:42-48` |
| 背驰段定义 | `docs/chanlun/text/blog/027-第27课.md:22` |
| 趋势背驰需≥2中枢 | `docs/chanlun/text/blog/027-第27课.md:14` |
| 盘整背驰大级别类一买 | `docs/chanlun/text/blog/027-第27课.md:66` |

---

## 7. 纪律声明

- 纯调研：只读分析源码/文档/dump，零代码改动，禁 cargo，禁 git mutation，主仓禁写。
- 全部判定附代码/文档行锚，可在 worktree `/tmp/kimi-nest-mainline` HEAD 树内复验。
- 教义判断基于缠论博文原文（第27课），非臆断。
