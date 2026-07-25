# 消费端候选来源调研：fill loop 是否只从 L0 取 bsp

> **Issue**: #132  
> **日期**: 2026-07-21  
> **类型**: wayfinder:research（纯调研，零代码改动）  
> **Worktree**: `/tmp/kimi-nest-mainline`

---

## 结论速览

| # | 问题 | 结论 |
|---|------|------|
| 1 | fill loop 候选从哪个级别取 bsp？ | **全级别**。`newly_confirmed_step` + `assemble_gamma` + `recognize` / `recognize_nested` 均遍历 `classification.levels.iter()` |
| 2 | `extract_signals` 是否只从 L0 取？ | **否**。`extract_signals_with_hist`（L0 路径）和 `extract_first_third_for_level`（L1+ 路径）**逐级别调用**，外加 `extract_second_for_level`（B2/S2）跨级别遍历 |
| 3 | 如果只从 L0：设计决定还是历史遗留？ | **不适用**——代码从设计上就支持全级别。L0 占比 ~90% 是 tower 自然衰减（结构单元逐级递减）的结果，不是代码限制 |
| 4 | 改成全级别的影响面 | **无需改动**——代码已经是全级别。V4 数据证实 L1/L2 交易存在（wf7: L1=18, L2=6；p3fold: L1=33, L2=17） |

**核心裁定**：issue 前提"所有 trade 的 certificate.level=0" **不成立**。消费端已经是全级别覆盖，L0 高占比是数据驱动的结构性现象。

---

## 一、fill loop 候选来源——全级别遍历的证据链

### 1.1 `newly_confirmed_step`：增量切片遍历全部级别

**文件**: `rust/src/theta_v0/backtest/signal.rs:38-65`

```rust
pub(crate) fn newly_confirmed_step(
    classification: &classifier::Classification,
    seen: &mut std::collections::HashSet<(usize, usize, u8)>,
) -> classifier::Classification {
    classifier::Classification {
        levels: classification
            .levels       // ← 遍历所有级别
            .iter()
            .enumerate()
            .map(|(lvl, ls)| LevelState {
                // ...
                bsp: ls.bsp.iter()
                    .filter(|p| seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits))))
                    .cloned()
                    .collect::<Vec<_>>()
                    .into(),
                // ...
            })
            .collect(),
    }
}
```

`seen` 的 key 是 `(level, source_index, bits)` —— **level 是三元组的一环**，说明设计上就预期多级别共存。

### 1.2 `assemble_gamma`：候选集组装遍历全部级别

**文件**: `rust/src/theta_v0/strategy/interp.rs:275-282`

```rust
pub fn assemble_gamma(classification: &Classification) -> Vec<Candidate> {
    let mut elements: Vec<CoverageElement> = Vec::new();
    let mut raw: Vec<(...)> = Vec::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {  // ← 全级别
        let lvl = level_idx as u32;
        for point in level.bsp.iter() {                                    // ← 该级别全部 bsp
            let dir = candidate_dir(point);
            let cls = min_class(&point.bits, dir);
            // ...
            elements.push(CoverageElement { level: lvl, ... });
            raw.push((lvl, point.source_index, ...));
        }
    }
    // map raw → Candidate with gamma_index
}
```

`coverage_elements_and_gamma_with_tower`（`interp.rs:473`）委托同一逻辑的 tower 变体，行为一致。

### 1.3 `recognize` / `recognize_nested`：旧路径同样全级别

**文件**: `rust/src/theta_v0/strategy/mod.rs:465-469` / `674-678`

```rust
// recognize (mod.rs:465)
let points: Vec<&classifier::bsp::BspPoint> = classification
    .levels
    .iter()
    .flat_map(|level| level.bsp.iter())
    .collect();

// recognize_nested (mod.rs:674) — 完全相同的模式
let points: Vec<&classifier::bsp::BspPoint> = classification
    .levels
    .iter()
    .flat_map(|level| level.bsp.iter())
    .collect();
```

### 1.4 `theta_key` 排序：高级别优先

**文件**: `rust/src/theta_v0/strategy/interp.rs:1070-1088`

```rust
/// ≺_Θ 排序键（spec §12 line 609-613：平移不变全序）。
/// 1. `Reverse(level)`：**高 level 先**（spec:54 高 level 先处理）。
pub(crate) fn theta_key(c: &Candidate) -> (Reverse<u32>, u8, usize, u8, u8, u8, u8, usize) {
    (
        Reverse(c.level),  // ← 高级别排在前面
        c.bsp_class,
        c.source_index,
        // ...
    )
}
```

排序键第一维度是 `Reverse(c.level)` —— 高级别候选**优先**于 L0 被处理。如果代码意图是只跑 L0，不会做这种排序。

### 1.5 fill loop 调用链

```
pi_theta_fill_loop_overlay (fill.rs:392)
  └─ classify_at(i) → full Classification (全级别)
  └─ newly_confirmed_step(classification_i, &mut seen_bsps) (fill.rs:688)
       └─ classification.levels.iter().enumerate() ← 全级别 bsp
  └─ interp::coverage_elements_and_gamma_with_tower_cached_gen(step_classification, ...) (fill.rs:770)
       └─ assemble_gamma 逻辑 ← 全级别
  └─ step_gamma_trade → coverage::pi_theta_step_traced (fill.rs:930)
```

---

## 二、`extract_signals` 是否只从 L0 取——否，逐级别提取

### 2.1 分类器的级别塔构建

**文件**: `rust/src/theta_v0/classifier/mod.rs:355-498`

```rust
for level_idx in 0..=l_max {
    if units.len() < min_parts { break; }  // 自然终止

    let is_l0 = level_idx == 0;
    // L0: classify_level(is_l0=true) → detect_centers_complete（方向交替+线段方向）
    // L1+: classify_level(is_l0=false) → detect_centers_geometric（几何区间重叠）
    let (centers, moves) = classify_level(&units, is_l0);

    // BSP 提取（逐级别调用）：
    let (mut bsp, pan_div) = if is_l0 {
        // L0 路径：extract_signals_with_hist on segments
        signal::extract_signals_with_hist(
            &centers, &l0.segments, &hist, &dif, &closes_tick, &close_src,
            config.divergence_gauge,
        )
    } else {
        // L1+ 路径：extract_first_third_for_level on geometric units
        extract_first_third_for_level(
            &centers, &units, &units_anchors, &hist, &dif, &closes_tick, &close_src,
            config.divergence_gauge,
        )
    };
    // 全级别：B2/S2 从递归 compose subs 提取
    bsp.extend(extract_second_for_level(&upper_moves, &hist, &close_src));

    for point in &mut bsp {
        point.level_origin = level_idx as u32;  // ← 标记来源级别
    }
}
```

三层 BSP 提取，**每一层都逐级别执行**：

| 层 | 函数 | 适用级别 | 提取内容 |
|----|------|----------|----------|
| B1/S1（趋势背驰） | `extract_signals_with_hist` / `extract_first_third_for_level` | L0 / L1+ | 同谓词，不同单元类型 |
| B3/S3（结构几何） | 同上 | L0 / L1+ | 同谓词 |
| B2/S2（盘整背驰） | `extract_second_for_level` | **全级别** | 从 `upper_moves` 的 `sub_moves` 递归提取 |

### 2.2 `extract_signals` 单元函数

**文件**: `rust/src/theta_v0/classifier/signal.rs:895`

```rust
pub fn extract_signals(
    centers: &[Center],
    segments: &[Segment],
    closes: &[f64],
    close_src: &[usize],
    macd_cfg: &MacdConfig,
) -> Vec<BspPoint> {
    let hist = compute_macd(closes, macd_cfg).hist;
    extract_signals_with_hist(centers, segments, &hist, &[], &[], close_src, DivergenceGauge::default()).0
}
```

这是一个**单级别**提取函数——接收某个特定级别的 centers + segments。多级别调用由上层 `classify_impl` 的 `for level_idx` 循环负责。

### 2.3 L1+ 的提取谓词与 L0 一致

`extract_first_third_for_level`（`signal.rs:2276` 附近）委托 `extract_signals_with_hist_anchored`，使用与 L0 **完全相同的背驰/几何谓词**，只是输入单元从 `Segment` 换成 `Unit`（几何区间）。这意味着高级别 BSP 的提取标准不低于 L0。

---

## 三、为什么 L0 占比 ~90%——tower 自然衰减

### 3.1 数据实证（V4 A2）

| 文件 | 总交易数 | L0 | L1 | L2 |
|------|---------|-----|-----|-----|
| `wf7/trades.jsonl` | 257 | 233 (90.7%) | 18 (7.0%) | 6 (2.3%) |
| `p3fold/trades.jsonl` | 479 | 429 (89.6%) | 33 (6.9%) | 17 (3.5%) |

`nest_depth` 分布也跨 0-4，不是全 0：

| nest_depth | wf7 | p3fold |
|------------|-----|--------|
| 0 | 85 | 153 |
| 1 | 92 | 159 |
| 2 | 44 | 89 |
| 3 | 36 | 70 |
| 4 | 0 | 8 |

### 3.2 Tower 事件确认塔递归深度

| Level | wf7 事件数 | p3fold 事件数 |
|-------|-----------|--------------|
| L0 | ~2000+ | ~3000+ |
| L1 | 938 | — |
| L2 | 500 | — |
| L3 | 258 | — |

### 3.3 衰减机制

分类器塔构建（`classify_impl:398-495`）每级别做 `compose_level` 投影：
- 3+ 个子级别线段 → 1 个父级别 move
- `units = project_to_units(&upper_moves, ...)`
- 当 `units.len() < min_parts`（默认 3）时 break

典型 OHLCV 数据：
- L0 产数百段 → 几十个中枢 → 数十个 BSP
- L1 需 L0 moves 组合成上级中枢 → 数量骤减
- L2+ 极少，需长窗口

**L0 高占比不是代码限制，是结构单元自然递减的统计结果。**

---

## 四、改成全级别取候选的影响面——无需改动

### 4.1 代码现状

代码已经是全级别覆盖：
1. **候选提取**：`classify_impl` 逐级别提取 BSP，tag `level_origin`
2. **候选组装**：`assemble_gamma` / `recognize` / `recognize_nested` 遍历 `levels.iter()`
3. **增量切片**：`newly_confirmed_step` 用 `(level, source_index, bits)` 三元组去重
4. **排序**：`theta_key` 用 `Reverse(level)` 让高级别**优先**处理
5. **执行**：`pi_theta_step` 按 `(level, direction)` slot 管理持仓，同一级别同方向只开一条腿
6. **证书**：`certificate.level` = `candidate.level` = `bsp.level_origin`，忠实传播

### 4.2 所有 `.levels[0].bsp` 硬编码位置

| 文件 | 行 | 上下文 |
|------|-----|--------|
| `strategy/mod.rs:1529` | 测试断言 | `classification.levels[0].bsp.iter().any(\|p\| p.bits.buy3)` |
| `backtest/nest_gate.rs:1506-1507` | 测试夹具 | `classification.levels[0].bsp.push(...)` |
| `backtest/nest_gate.rs:1611` | 测试 | `&cls_b3.levels[0].bsp` |
| `backtest/runner.rs:2385` | 测试 | `&cls.levels[0].bsp` |

**所有 `.levels[0].bsp` 直接访问均在 `#[cfg(test)]` 或诊断路径中**，生产 fill/recognize 路径零硬编码 L0。

### 4.3 唯一接近 L0 特殊处理的逻辑

| 位置 | 机制 | 影响 |
|------|------|------|
| `econ_positive.rs:1104` | `lvl==0` 免 base gate | L0 候选过门更宽松，但不阻塞高级别 |
| `nest_gate.rs` | nest gate 默认关闭（`THETA_NEST_CERT_GATE` 未设） | 不影响任何级别 |
| `config.max_depth=3` | voice tree 深度权重 `[0.60, 0.30, 0.10]` | 仅影响旧 `recognize` 路径的 sizing，不影响是否开仓 |

### 4.4 如果要增加高级别交易占比

**这不是代码修改问题，而是数据/参数问题**：

1. **增加数据量**：更长的窗口让高级别结构（L1 中枢、L2 走势）充分形成
2. **降低 `min_parts_per_level`**：从 3 降到 2 可以让高级别在更少结构单元下产 BSP（但有降低信号质量的副作用）
3. **开启 nest gate**（`THETA_NEST_CERT_GATE=1`）：对高级别候选取到过滤/背书作用，理论上可提升高级别信号的可信度

---

## 五、Issue 前提修正

> Issue 原文："V4 数据实证：所有 trade 的 certificate.level=0、nest_depth=0"

**此前提不成立**。V4 A2 数据明确包含 L1 和 L2 交易，nest_depth 跨 0-4。可能的混淆来源：

- 观察的是旧版 V4 数据（tower 递归深度不足）
- 查询口径有误（如只看了 `via_anc_ok_prune=true` 的子集）
- 数据文件路径错误

建议 ticket 报告人核对具体数据文件和查询语句。

---

## 附：关键文件索引

| 文件 | 关键函数 | 作用 |
|------|---------|------|
| `backtest/signal.rs:38` | `newly_confirmed_step` | 增量切片（全级别） |
| `strategy/interp.rs:275` | `assemble_gamma` | 候选组装（全级别） |
| `strategy/interp.rs:1077` | `theta_key` | 排序键（高级别优先） |
| `strategy/mod.rs:456` | `recognize` | 旧路径（全级别） |
| `strategy/mod.rs:663` | `recognize_nested` | nest 路径（全级别） |
| `backtest/fill.rs:392` | `pi_theta_fill_loop_overlay` | π 主 fill loop |
| `backtest/fill.rs:1693` | `plan_and_fill_mtm` | mtm fill（旧路径） |
| `backtest/fill.rs:2063` | `plan_and_fill_mtm_dual` | dual fill（旧路径） |
| `classifier/mod.rs:355` | `classify_impl` | 级别塔构建 + 逐级别 BSP 提取 |
| `classifier/signal.rs:895` | `extract_signals` | 单级别 BSP 提取 |
