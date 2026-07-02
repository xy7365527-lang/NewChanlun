# B2 SKIP — signal 提取 frontier-resume 增量：负结果结果包（2026-07-02）

**工位**：泳道 B / ws-b2（task #40，algo-opt-plan B 泳道 B2）
**裁定**：**SKIP**（Lead 2026-07-02 裁定 (1)；负结果 = 合格交付，161 照实 + formalization-validity-domain 231号）
**认识论等级**：**L1**（CPU 耗时度量，确定性可复现，零信息增量——不验证 Θ 市场有效）
**代码影响**：**零**——signal.rs 未改，GOLDEN digest `0x56ed_dd65_1c59_5733`（signal.rs:1666）原地保绿；无新文件（本记录除外）；既有 bit_exact 电池不受影响。

---

## 1. 结论（headline）

frontier-resume 增量提取**不落地**。理由：memo（mod.rs:1087 `cached_bsp_key`）已把 L0 BSP 重算坍缩到只在「段/中枢计数变」的 bar 触发（mod.rs:1080 注释实测：16K bar 仅变 ~120 次，「坍缩近常数」）。实测 07a 阶段 = **墙钟 0.91%**，远低于任何立项门槛；frontier-resume 的实际收益是其中一部分，代价却是 ~80 行**跨-bar 有状态、bit-exact 关键、digest 封门**的缓存（τ-flip 回退 + change-point + 前缀归并 tie 语义），且落在 ws-a1 正在改写的 mod.rs:1092 区。ROI 不成立。

---

## 2. 300K 重测表（当前 HEAD b875cc5dda，全文）

复算命令：
```
THETA_PROFILE_STAGES=1 A0_PROFILE_BARS=300000 cargo test --release -p newchan_rust \
  --lib backtest::incremental::profile::profile_clone_cluster_a0 -- --ignored --nocapture
```

墙钟 = **8.71s**（300000 bar 逐 bar `classify_with_tower_incremental`）。

| 阶段 | ms | %插桩 | 备注 |
|------|-----|------|------|
| 05_compose_resume | 4260.323 | 74.76% | ← 真热点（中枢检测 resume，A3/A4 泳道） |
| 07b_extract_second | 327.714 | 5.75% | 第二类提取（miss 全算路径） |
| 04_cached_units_copy | 313.592 | 5.50% | 克隆簇（A1） |
| 00b_l0_units_clone | 242.595 | 4.26% | 克隆簇（A1） |
| 03_frontier_compare | 206.028 | 3.62% | frontier 投影比较（A3） |
| 10_projected_units_clone | 134.513 | 2.36% | 克隆簇（A1） |
| **07a_extract_signals_l0** | **79.051** | **1.39%** | **← B2 目标（墙钟 0.91%）** |
| 06_extend_centers_upper | 39.475 | 0.69% | |
| 09_project_to_units_resume | 27.282 | 0.48% | |
| 08_levels_centers_clone | 22.192 | 0.39% | 克隆簇（A1） |
| 07c_bsp_memo_clone | 21.054 | 0.37% | memo 命中（已 Rc::clone O(1)） |
| 01_l0_tower_rebuild | 9.388 | 0.16% | |
| 00_update_closes_cache | 5.413 | 0.09% | |
| 02_macd_incremental | 5.170 | 0.09% | |
| 00_l0_units_build | 4.628 | 0.08% | |
| **插桩Σ** | **5698.418** | 100% | |

**07a = 79.051 ms = 插桩 1.39% / 墙钟 0.91%。05_compose_resume 大它 54 倍。**

---

## 3. scaling 核对

| 窗口 | 07a ms | %插桩 | 来源 |
|------|--------|------|------|
| 300K | 79.05 | 1.39% | 本次重测（HEAD b875cc5dda） |
| 1M | 1264.0 | 2.4% | A0 profile（a0-clone-profile-20260702.md §2） |

- 300K→1M：bar ×3.333，时间 ×15.99 ⟹ 缩放指数 exp ≈ ln(15.99)/ln(3.333) = **2.30**（次要 O(n²)，miss 路径每次 O(S)、S 随 n 增长）。
- 全历史 4.6M 外推：1264ms × 4.6^2.30 ≈ **~42s**（07a 绝对量）。
- **但 05_compose_resume 同样超线性**（A0 实测 exp≈1.79）**且绝对量始终大 40–54 倍**——全历史下 05 仍是 07a 的十几倍。若目标是「分类器最大加速」，05（A3/A4 泳道）价值远高于 07a。

---

## 4. skip 推导链（定理类，计划内规则推导，无需上浮）

1. **计划 A0 段 YAGNI 门先例**：占比低 → 缩水/撤项（a0-clone-profile-20260702.md §3「克隆簇占比若 <10% ⟹ 撤 A1」）。07a=0.91% 墙钟远低于该门槛。
2. **quality-guard 防膨胀锁**：target 按实测降准（Lead 裁定）；实测把计划的「8.7s/10x」证伪——「8.7s」是整分类器墙钟，signal 阶段本身仅 79ms，计划把两者混淆。
3. **275 局部依赖**：05 属 A3/A4 泳道，B2 不该越泳道去修 05（附庸的附庸不是我的附庸）。
4. **Ponytail-ULTRA rung-1**：profiler 说 0.9% ⟹ 有状态缓存 = bug farm with a hit rate。memo（O(1) Rc clone 命中）已捕获 99.2% 的赢面。
5. **Rc 区协作风险**：frontier-resume 落点 mod.rs:1092 = ws-a1 正在改写区（Rc 化 07c/08/…）；落地会与 A1 撞车，且 resume state 若持 lc.centers 的 Rc clone 会破坏 mod.rs:1052 原地 extend。

---

## 5. 边界条件（结论翻转条件）

以下任一成立则重开（最小版 frontier-resume 设计已存 task #40 metadata：新增 `extract_signals_resume` + `BspL0Resume{points,prev_segs_len,prev_tau}`，pure `extract_signals` 零改保 GOLDEN，新增「段 1..=N 增量喂 == 全量」单测锁 bit-exact）：

- 全历史 4.6M 实测 07a > ~5% 墙钟；或
- 05_compose_resume 被 A3/A4 优化掉，使 07a 上位为顶部热点；或
- Lead 为全历史跑批明确要 07a 的 O(n²)→O(n) 线性化（不管占比）。

---

## 6. 下游推论 + 谱系 + 影响

- **下游推论**：05_compose_resume（75%，A3/A4 泳道）是分类器真热点；Lead 已用本 05 数据立 A3 任务（A1 后继）。B 泳道剩余价值在 B4（#39，mod.rs:248 消 2×MACD，**删除类**非缓存，Ponytail 友好，等 A1 commit 后接）。
- **谱系引用**：formalization-validity-domain 231号（L1 度量零信息增量，负结果缩小有效域边界，比确认性结果更有价值）；161号（照实非务实，不 ship 也是合格交付）；no-patch-mentality 090号（声明与实际一致——零改代码零声明膨胀）。无概念分离谱系（纯性能）。
- **影响声明**：无代码变更、无生产路径行为变更、无既有测试受影响。仅新增本负结果记录。task #40 已 TaskUpdate completed。

---

## 7. 给 B3（#44）工位的关键界定（开工前必读）

B3 = MACD 面积 `(start,end)→f64` memo（禁前缀和差分——EMA state 依赖使区间面积非前缀和可差分，故需 (start,end) 键 memo）。

**热度上限（本次 300K 实测界定）**：MACD 面积计算是 `07a_extract_signals_l0`（79ms）+ `07b_extract_second`（328ms）内部的**子集**——B3 的收益天花板 ≤ 这两段之和 ≈ **407ms = 插桩 7.1% / 墙钟 4.7%**（且实际是其中一部分，非全部）。`02_macd_incremental`（增量 MACD state 推进）本身仅 5.17ms，不在 B3 域。

**强制纪律（B2 教训）**：**开工前先 profile 确认 MACD 面积是否真热点**，不得按计划预估直接立项——B2 已证计划基线陈旧（把整分类器墙钟当成单阶段耗时）。用 `THETA_PROFILE_STAGES` 或针对性 profile 测出 MACD 面积在 07a/07b 内的实际占比，占比低于立项门槛则同样 SKIP（负结果照实，161）。profile-first 主张已写进 #44 任务卡。

**谱系承接**：本 SKIP 判据（YAGNI 门 + 防膨胀锁 + 275 局部依赖 + Ponytail rung-1）对 B3 同样适用。B3 若立项，须 pure 路径零改保 GOLDEN、新增 bit-exact 单测锁 memo 命中==全算。
