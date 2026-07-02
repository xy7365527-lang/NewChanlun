# A0 克隆簇 profile — YAGNI 重开门证据（2026-07-02）

**工位**：泳道 A / ws-a0（algo-opt-plan-20260702.md §二泳道A A0）
**认识论等级**：**L1**（CPU 耗时度量，确定性可复现，零信息增量——不验证 Θ 市场有效，formalization-validity-domain 231号）
**驱动**：`classify_with_tower_incremental` 逐 bar 跑 BTC（btc_1m_full.json）前 N bar，`THETA_PROFILE_STAGES=1` 全 15 阶段计时。
**复算**：
```
THETA_PROFILE_STAGES=1 A0_PROFILE_BARS=1000000 cargo test --release -p newchan_rust \
  --lib backtest::incremental::profile::profile_clone_cluster_a0 -- --ignored --nocapture
```

---

## 1. 结论（headline）

克隆簇 = `{00b_l0_units_clone, 04_cached_units_copy, 07c_bsp_memo_clone, 08_levels_centers_clone, 10_projected_units_clone}`（A1 的 Rc/借用 目标段）。

| 窗口 | 克隆簇 | 占分类器插桩总耗时 | 占墙钟 | compose_resume(05) 占插桩 |
|------|--------|------|------|------|
| **400K bar** | 1775.8 ms | **19.3%** | 13.1% | 65.9% |
| **1M bar** | 11369.1 ms | **21.8%** | 15.5% | 60.0% |

**克隆簇是真实 O(n²) 成本，占比随规模上升（19.3%→21.8%），清过 YAGNI 重开门槛——但历史量级声称（60–70% memcpy / 5–10x）被本测证伪。**

- 克隆簇缩放指数 exp≈**2.03**（400K→1M ×2.5 bar，耗时 ×6.40）= 教科书式「每 bar 全量拷贝累积结构」的 O(n²) 签名。
- 但克隆簇**不是**主导成本：`05_compose_resume`（中枢检测 resume，非克隆）单段 = 60% 分类器时间、exp≈1.79，是 A3/A4 域，**A1 完全不触及**。
- A1（整簇→O(1)/O(tail)）端到端分类器加速**上限 ≈ 1/(1−0.218) ≈ 1.28x**（1M），与三份 verdict 校准的「A1 ~1.5–2x 封顶」一致（本实测更紧）。绝非 5–10x。

**裁定建议**：A1 **可重开但须缩窗**到 ~1.28x 校准上限，不按旧 5–10x 立项。若目标是分类器最大加速，A3/A4（compose_resume）优先级 > A1。

## 2. 定义依据（1M bar 全阶段拆解，ms）

分类器插桩总耗时 = 15 段之和 = **52107.5 ms**（墙钟 73.38s；未插桩 21.3s/29% = parser append + `classify_move_incremental` + bsp miss 路径 sort/extend + Vec 分配 + profiling Instant 开销）。

克隆簇逐段（1M）：

| 阶段 | ms | %插桩 | A1 处置（plan §二泳道A A1） | 消除后 |
|------|-----|------|------|------|
| 04_cached_units_copy | 4010.9 | 7.7% | frontier 比较快照 copy（A1 acceptance 明列 04） | O(tail) |
| 00b_l0_units_clone | 2688.8 | 5.2% | l0_units 借用/`mem::take`（须 TowerCache 解构拆借，波及 ~28 消费点） | ~O(1) |
| 07c_bsp_memo_clone | 1797.1 | 3.4% | cached_bsp memo 命中 → `Rc::clone` | **O(1)** |
| 08_levels_centers_clone | 1583.9 | 3.0% | centers → `Rc`+make_mut | ~O(1) |
| 10_projected_units_clone | 1288.3 | 2.5% | projected_units 借用/`mem::take` | ~O(1) |
| **克隆簇Σ** | **11369.1** | **21.8%** | | |

非克隆大段（对照，A1 不触及）：
- 05_compose_resume 31281.2 ms（60.0%）— 中枢检测 resume 续扫，A3/A4 域。
- 07b_extract_second 5330.2 ms（10.2%）、07a_extract_signals_l0 1264.0 ms（2.4%）— BSP 提取（memo miss 路径，B 泳道）。
- 03_frontier_compare 2446.0 ms（4.7%）— frontier 投影**比较**（非克隆，A3 证书半边域）。

A1 窄口径 `{04,08,10}`（plan A1 acceptance「砍掉 04/08/10 ≥70%」）= 6883.1 ms = **13.2%** 插桩 / 9.4% 墙钟。若砍 70% ⟹ 省 ~4.8s = 1.10x 分类器。

## 3. 边界条件（结论翻转条件）

- **克隆簇占比若 <10% ⟹ 撤 A1**：实测 21.8%（1M）且随 n 升，未触发撤项。
- **若 compose_resume(05) 可被 A1 顺带消除 ⟹ A1 收益翻倍**：不能——05 是中枢检测计算，非拷贝，Rc/借用无效，须 A3/A4 证书路径。
- **若 memo 命中率 <99% ⟹ 07c 占比降、07a/07b（miss 全算）升**：本 BTC 段命中率高（07c 1797ms 命中 vs 07a+07b 6594ms miss，miss 集中在段变化 bar），符合 memo 设计。
- **缩放外推**：克隆簇 exp≈2.03 ⟹ 全历史 4.6M 处克隆簇绝对量 ×~21（相对 1M），占比因 05 exp≈1.79 略慢而**继续微升**——1M 结论对全历史保守（占比只会更高，不会更低）。

## 4. 下游推论（对系统其他部分）

- **A1 立项**：重开合法，但目标从旧「5–10x」改校准「~1.28x 分类器上限」；ROI 排序：07c（Rc::clone，trivial O(1)、low risk）> 08（centers Rc）> 04（最大单段 4010ms 但与 frontier 比较缓存 A3 纠缠）> 00b/10（借用，须 ~28 消费点机械适配，边际 ROI）。
- **A3/A4 优先级**：若「分类器最大加速」是目标，05_compose_resume（60%，O(n^1.79)）> 整个克隆簇。A3（frontier L1+ 证书）打的是 05 相关的全量比较/快照，量级远大于 A1。
- **incremental 塔「exp≈1 amortized」文档声明（mod.rs:830-833）与实测冲突**：实测整分类器 exp≈1.90（1M/400K），克隆簇 exp≈2.03——「段账本单调追加 ⟹ O(n) amortized」的前提被逐 bar 全量克隆破坏。这是 A1 存在的根因，非文档错误（文档描述的是**扫描**O(n)，克隆是**扫描之外**的每 bar 全拷）。

## 5. 谱系引用

- 分类器增量 bit-exact 语义：mod.rs:806-839 硬契约（#93/#106）；插桩零行为改变由 `stage_profile::time` env 未启用时闭包直通保证（mod.rs:770-772）。
- YAGNI 暂缓裁定：Lead 2026-06-30（algo-opt-plan §全局约束2）。
- 有效域标注：L1 度量零信息增量（formalization-validity-domain 231号）——本报告只证「克隆簇 CPU 占比」，不证「A1 后市场表现」。
- 相关：`project_frontier_resume_bt_too_late`（05_compose_resume 的 frontier 不稳定性同源）；incremental.rs:466-500 `diag_classifier_resume_frontier_divergence`（bar-1464 发散，A3/A4 域根因）。

## 6. 影响声明

**改动（零行为改变，仅插桩 + profile 驱动）**：
- `rust/src/theta_v0/classifier/mod.rs:855` — `let l0_units` clone 包 `stage_profile::time("00b_l0_units_clone", …)`。
- `rust/src/theta_v0/classifier/mod.rs:1082` — bsp memo 命中 `lc.cached_bsp.clone()` 包 `stage_profile::time("07c_bsp_memo_clone", …)`。
- `rust/src/theta_v0/backtest/incremental.rs` — 新增 `#[ignore]` profile 测试 `profile_clone_cluster_a0`（BTC ≥1M bar 驱动 + dump）。
- **无生产路径行为变更**（`stage_profile::time` env-gated 直通，bit-exact 守卫不受影响）；`cargo build --lib` 绿；profile 测试通过。
- **未 commit**（Lead 唯一 commit 者）。
