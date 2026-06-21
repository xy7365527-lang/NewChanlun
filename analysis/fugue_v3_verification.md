# 赋格引擎 v3（递归嵌套多重赋格）实装与 L3 验证报告

> 日期：2026-06-17
> 引擎：`rust/src/fugue_v3/`（从头写，不改 unn/spiral v2）
> 理论：`docs/recursive_fugue_necessity_proof.md`（定理 RF/RF-cyc/RF-alg/RF-exh）+
> `docs/spiral_physical_reinterpretation.md` v2 + `analysis/chanlun_original_operation_structure.md`

---

## 0. 速览（三句话）

1. **结构 L0/L2 验收 PASS（8 标的 ~28M bars 零 panic）**：守恒（Σunits=n_base）/四步循环闭合/
   穿 ε=−1（非平凡 H¹）/σ-不变配额/NAV 中性/核心仓恒持，全部 prove 守卫每 bar panic 检查零触发，
   流式与批量逐位等价（bit-exact PASS）。
2. **激活 P1（strat≥BH）仅 1/8 PASS（CL）**：赋格的**演奏**是 regime 函数——机动仓穿 ε=−1 在强牛
   踏空（6/8 机动仓已实现为负），MACD 面积背驰门减轻但未消除。精确印证必然性证明 RF-NR2。
3. **MDD 8/8 优于 BH**：核心仓 H⁰ 恒持 + 顶背驰 C 清仓的鲁棒风险收益；CL 唯一 P1 PASS 靠 core_clear
   避开油崩（−94% BH_MDD），非机动短差。

---

## 1. 架构（层结构 ≠ voice forest）

| | unn / spiral（voice forest） | fugue_v3（层结构） |
|---|------------------------------|--------------------|
| 拓扑 | 树：root + child 借 units | **层数组**（每 ladder 一账户），机动仓相邻级别流动 |
| 操作 | 五事件 F/C/D/E/A spawn 独立 voice | **四步 1-cycle**（平多→开空→平空→做多，不可分） |
| 仓位 | 恒仓满仓 + 子借贷 | **核心仓 H⁰(2/3 恒持) ⊕ 机动仓 H¹(1/3 循环)** |

**七文件**：`mod.rs`（常量+认识论标注）/`layer.rs`（Layer+CyclePhase+FugueResult）/`cycle.rs`
（四步循环执行）/`accounting.rs`（NAV/σ-配额/守恒）/`prove.rs`（守卫+10 反证）/`engine.rs`
（A→C→D→E→F 驱动）/`ffi.rs`（FugueV3Stream + run_fugue_v3）。**信号层复用** `spiral::signal`
（向心 confirm + nf_sell/nf_buy；`enable_macd_divergence=True` 已开）。

### 守恒模型（L0，prove_conservation 守）

```
total_units = Σ core_units(H⁰@core_ladder, 2/3) + mobile_pool_long(H¹预算, 1/3) + Σ mobile_units(各层做空敞口)
```

四步循环穿 ε=−1（真做空，非平凡 H¹，§2.5.D，2× alpha 几何）：
- 开仓①②：pool−=m，free+=m·c（平多），机动仓 cash=m·c（空头 proceeds），MtM=m·c−m·c=0 ⟹ ΔNAV=0。
- 平仓③④：cover + rebuy ⟹ ΔNAV=0；全循环净 free += 2m(basis−c)。

---

## 2. L0/L2 验收（prove 守卫，每 bar panic = 验收标准）

`cargo test --lib fugue_v3`：**21/21 通过**（11 正向 + 10 `should_panic` 反证，守卫非重言）。

8 标的真实数据全程**零 panic**（守恒/四步闭合/穿 ε=−1/σ-配额/NAV 中性/核心仓恒持 L0/L2 成立）：

| 守卫 | 断言 | 等级 |
|------|------|------|
| prove_cross_level_closure | 四步循环 Δr=−1（H¹ 生成元） | L0 |
| prove_mobile_chiral_short | 机动仓穿 ε=−1（非平凡 H¹，非纯多平凡环路） | L0 |
| prove_sigma_quota | 配额 m=f×pool 级别无关（σ-不变，T18） | 形式 L0 |
| prove_cycle_closure | 四步相位↔敞口一致 | L0 |
| prove_conservation | Σunits=n_base（T48 Casimir） | L2 |
| prove_nav_neutral | 同价操作 NAV 中性 | L2 |
| prove_core_unchanged | 核心仓恒持（H⁰ σ-不变，026:80） | L2 |
| prove_no_double_act / prove_recursive_consistency | per-layer 互斥 / 局部依赖 | L2 |

bit-exact（L1 管线）：300K BTC 流式 `finish()` 与批量 `run_fugue_v3` 逐键 **PASS**。

---

## 3. L3 交叉验证（8 标的，可否证）

| 标的 | bars | strat_pct | BH | P1 | MDD | BH_MDD | 机动仓已实现 | 四步(开/平) | 强平 | 声部 |
|------|------|-----------|-----|----|-----|--------|-------------|------------|------|------|
| BTC | 4.63M | +772.9% | +1380.4% | ✗ | −71.9% | −84.0% | **−79063** | 182/166 | 3 | 3 |
| OKLO | 448K | +305.9% | +307.1% | ✗ | −67.5% | −76.8% | −27533 | 22/12 | 3 | 2 |
| QQQ | 728K | +71.6% | +174.6% | ✗ | −11.5% | −25.6% | −1534 | 9/7 | 0 | 2 |
| DX | 2.05M | +3.0% | +4.1% | ✗ | −15.3% | −16.8% | +200 | 3/3 | 0 | 1 |
| GC | 5.54M | +161.7% | +257.3% | ✗ | −37.5% | −45.6% | −13643 | 79/76 | 1 | 3 |
| ES | 5.59M | +432.6% | +594.3% | ✗ | −27.7% | −36.0% | −38668 | 52/48 | 2 | 3 |
| BRN | 2.42M | +3.9% | +87.4% | ✗ | −59.1% | −78.8% | +590 | 61/52 | 0 | 3 |
| **CL** | 5.53M | **+100.2%** | **+28.2%** | **✓** | −74.9% | −94.0% | −12792 | 129/116 | 1 | 3 |

### 判读（诚实，否定性结果价值高）

1. **P1 = 1/8（CL）= regime 函数**：正域 = 下跌/震荡（CL 油崩 −94% BH_MDD，减仓+顶背驰清仓胜出）；
   负域 = 强牛（BTC/ES/QQQ/OKLO/GC，踏空）。与全部 MEMORY 一致（清仓频率=regime 函数，
   `project_constitutive_throughput_falsified`/`project_unn_btc_spawn_throwback`）。
2. **机动仓穿 ε=−1 已实现 6/8 为负**：四步循环的空头腿在上涨中亏（理论预言 reinterp §6.4）。
   MACD 面积背驰门**减轻**踏空（182 次 BTC 循环 vs unn 333 子空头）但**未消除**。CL P1 PASS
   **靠 core_clear（顶背驰避油崩）而非机动短差**（CL 机动仓仍 −12792）——赋格 H¹ 在多数 regime 是拖累。
3. **MDD 8/8 优于 BH**：核心仓 H⁰ 恒持 + 顶背驰 C 清仓的鲁棒风险收益（结构性，跨 regime）。
4. **多声部 L6 实证**：max_concurrent_voices 达 3（GC/ES/BRN/CL/BTC），赋格并发声部经验成立。

### 必然性证明的 L3 兑现（RF-NR2 印证）

> 赋格的**谱**（所有可能声部 = 四步 1-cycle 结构）是 L0 必然且完整（零 panic 验收）；
> 赋格的**演奏**（哪些声部此刻发声）是 regime 决定的 L2/L3 读数。

本 L3 精确兑现：**结构 L0 完整（8/8 零 panic）∧ 激活 regime-gated（P1 仅 CL）**。混淆二者 = 有效域膨胀。
MACD 面积门是结构性 regime 门但在强牛**不充分**——这是否定性增量（缩小"赋格激活必然"的有效域）。

---

## 4. 开放轴（不打补丁续命）

- **更强 regime 门**：MACD 面积背驰单独不足以在强牛抑制机动仓穿 ε=−1。候选：主级别走势类型门
  （趋势态禁机动仓，026:80 字面）——但 MEMORY `project_constitutive_throughput_falsified` 已证
  "走势类型门 L3 否证（A′ 解耦坍塌）"，须谨慎预注册。
- **核心仓 vs 机动仓贡献分离**：MDD 改善来自 core_clear（H⁰），alpha 拖累来自机动短差（H¹）。
  消融实验：关闭机动仓（纯 H⁰ core + C 清仓）的 P1/MDD 基线，量化 H¹ 净贡献符号。
- **CL 白名单**：CL（油崩 regime）是唯一 P1 正域，与 MEMORY `project_constitutive_throughput_falsified`
  的 CL 白名单收敛。

## 5. 影响声明

- 新增 `rust/src/fugue_v3/`（7 文件）+ `lib.rs` 注册（2 处，加性）+ `trading_system/backtest_fugue_v3.py`。
- **不改** unn/spiral v2/任何已结算定义/守恒律/谱系。
- 结果：`trading_system/data_cache/fugue_v3_{BTC,OKLO,QQQ,DX,GC,ES,BRN,CL}.json`。
- 认识论：结构 L0（定理 RF + prove 零 panic）；激活 L3（8 标的，P1=1/8，否定性=regime 函数）。
