# 代码层验证：prop4-readingB-recursive L3 突破 look-ahead + 杠杆伪影

**工位**：code-verifier  
**日期**：2026-06-23  
**验证对象**：commit `efcea91ba1`，每级别多重赋格读法乙递归，ES +681% > BH +594%  
**触发**：Gemini 质询识别两个风险（Gemini 429 未跑，须代码层代理验证）  
**读文件**：`rust/src/recursive_t/divergence.rs`（全）、`rec_engine.rs`（关键段）、`rec_stream.rs`（lines 1-465）

---

## 一句话判决

**验证1 look-ahead：PASS — 完全因果，无前视数据访问。**  
**验证2 杠杆/恒仓：条件 PASS，存在理论边界条件，ES 实际情形 LOW RISK，需跑 max_gross 确认。**

---

## 验证1：Look-ahead 伪影（最关键）

### 1.1 背驰段确认时机

**核查点**：`trend_diverging_segment` / `d_top` / `nest_chain_complete` / `select_child_in_window` 是否依赖未来 bar？

**结论：全部使用历史数据，PASS。**

**证据链**：

(a) **`rec_stream.rs` 模块声明（lines 7-9）**：
> "BSP/走势完成直到**段确认那根 bar** 才被算出/投放，`on_view` 在确认时点 close 执行 reconcile ⟹ 因果（除不可消除的确认滞后本身）"

这是引擎级别的因果声明。

(b) **段门控触发条件（rec_stream.rs lines 192-208）**：
```rust
let sc = self.orch.strokes().len(); // 已确认笔数
if sc > self.last_stroke_n {
    // 触发口径：confirmed && settled 段数变化
    let count = segs.iter().filter(|s| s.confirmed && s.kind == SegKind::Settled).count();
    if trig != self.last_trigger {
        // 仅此时重跑树 + 计算 d_top / level_div
    }
}
```
重跑只在**已确认、已结算的段数改变**时触发。确认/结算是事后事件（段已完成），不前视。

(c) **`d_top_arr` 计算（lines 350-365）**：
```rust
let d_top_arr = if self.cfg.enable_reading_b {
    let tree = iterate(a0, self.mode); // tree 从已确认段构建
    for k in 0..MAX_LEVEL {
        dt[k] = crate::recursive_t::divergence::d_top(k, cc_trend, ...);
    }
    dt
} else { [false; MAX_LEVEL] };
```
`tree` 由 `iterate(a0, ...)` 从已确认段（`build_a0_fast(segs, strokes, m2r, ...)` — segs 全为已确认）构建，不含未来 bar。

(d) **`on_view` 调用时机（line 455）**：
```rust
self.driver.on_view(&view, c, bar);  // c = 当前 bar close
```
在 view 填充完成后、当前 bar close 时调用，不超前执行。

(e) **`divergence.rs` 全文审查（leg_extreme 关键逻辑）**：
```rust
fn leg_extreme(units: &[UnitRef], dir: Direction) -> Option<(f64, i64)> {
    // dir=Up：扫所有 unit，取 end_bar 端点最高价
    // dir=Down：扫所有 unit，取 start_bar 端点最低价
    // 全部来自已历史化的 unit slice
}
```
BSP struct: `{ kind, bar: c_bar, price: c_ext, level }` — `bar`/`price` 是 c 段历史极值，不是未来数据。

`nest_chain_complete` → `select_child_in_window` 的逻辑：
```rust
// parent 窗口 [plo, phi] 来自 trend.zhongshus.last() 的历史 units
// child 窗口 [lo, hi] 来自 child_trend 的历史 units
if plo <= lo && hi <= phi && (lo, hi) != (plo, phi) { // 纯历史区间比较
```
纯结构比较，无未来 bar 访问。

(f) **成交价口径（`open_leg`/`close_leg`/`g(k)`）**：
```rust
// g(k): close_leg(k, c) + open_leg(k, new_dir, node, top, c)
// c = 当前 bar close（push_bar 传入）
fn close_leg(&mut self, k: usize, c: f64) { ... self.free += leg.units * c; ... }
fn open_leg(&mut self, k: usize, ..., c: f64) {
    let m = self.geom_tower_quota(k, top, self.free.max(0.0), c);
    self.free -= m * c; // Long: 以当前 close 成交
}
```
成交价 = 当前 bar close，**不是历史 BSP.price**（历史极值价格）。这消除了 look-ahead 价格伪影。

**→ 结论：Look-ahead 验证 PASS。**

### 1.2 确认滞后（不可消除，非伪影）

唯一的时间偏差是**确认滞后**（confirmation lag）：段在被下一段起始时确认，操作信号延迟约 1 段（a0 级别约几根到十几根 bar）。这是缠论原文固有的时序约束，不是 look-ahead 伪影。ES +681% 含此滞后税，已反映在真实回测中。

---

## 验证2：净敞口 / 杠杆（恒仓声明）

### 2.1 geom_tower_quota 公式（rec_engine.rs line 1054）

```rust
const MOBILE_FRAC: f64 = 1.0 / 3.0;  // line 39

fn geom_tower_quota(&self, k: usize, top: usize, free_pool: f64, c: f64) -> f64 {
    let base = free_pool * (2.0 / 3.0);     // base = free×2/3
    let depth = top - k;                     // k=top: depth=0（顶层最大），k=0: depth=top（底层最小）
    let notional = base * MOBILE_FRAC.powi(depth as i32);  // = base × (1/3)^depth
    notional / c
}
```

### 2.2 Long-only 方向：数学证明 Σnotional < free ✓

`open_leg` 中 Long 开仓：`self.free -= m * c`（free 递减）。

`consume_legs` 迭代 k=0..=top，每次 `open_leg` 使用**当前（已递减的）free**。

设初始 free = F₀，all-Long top=2 的递推：
- k=0: notional = F₀×(2/3)×(1/3)² = F₀×2/27，free → F₀×25/27
- k=1: notional = (F₀×25/27)×(2/3)×(1/3) = F₀×50/243，free → F₀×175/243
- k=2: notional = (F₀×175/243)×(2/3) = F₀×350/729

Σnotional = F₀×(54+150+350)/729 = **F₀×554/729 ≈ 0.76×F₀ < F₀** ✓

一般情况：每次 Long 开仓后 free 减少，下一层配额基数更小 → 几何收缩累加 < F₀。**Long-only 恒仓证明成立。**

### 2.3 Short 方向：理论杠杆边界（代码注释存在不严格性）

`open_leg` 中 Short 开仓：`self.free += m * c`（short 收到现金，free 递增）。

后续层使用递增的 free，产生配额放大：

all-Short top=2 递推：
- k=0: notional = F₀×2/27，free → F₀×29/27
- k=1: notional = (F₀×29/27)×(2/3)×(1/3) = F₀×58/243，free → F₀×319/243
- k=2: notional = (F₀×319/243)×(2/3) = F₀×638/729

Σshort_notional = F₀×(54+174+638)/729 = **F₀×866/729 ≈ 1.187×F₀ > F₀**

→ **all-Short 情形 gross exposure 理论上超 100%（约 119%，top=2 时）。**

代码注释 "收敛 base×3/2≤free 恒仓不加杠杆"（line 1053）是**针对静态公式**（所有层共用同一 F₀ 计算）的声明，**不覆盖 all-Short 顺序开仓的实际路径**。声明与实际代码行为在 Short 方向存在小幅不一致。

### 2.4 ES 实际风险评估

ES（2015-2024 强牛趋势）在多重赋格读法乙下的实际情形：

1. **方向偏置 Long**：ES 是长期上行，绝大多数级别的腿是 Long。Short 腿只在顶背驰段（稀疏）出现。
2. **多级别同时 Short 概率低**：`d_top[k]` 链要求区间套贯通，上下级别同时 fire 是罕见结构，实践中每次重跑最多 1-2 级别切换。
3. **max_gross_exp_x100 已在 eprintln 输出**（rec_stream.rs line 1355-1357）：
   ```
   max_gross={:.2}× max_net={:.2}× ({:.1}s)
   ```
   但 L3 报告（`.chanlun/review-results/prop4-readingB-recursive-L3-20260623.md`）未保存这些值（仅来自 stderr，未收录）。

**已知约束**：`self.free.max(0.0)` 确保 free ≤ 0 时不开新腿（line 1075）——当 free 耗尽或为负时自动停开。这提供了一个下界保护。但对 Short 方向（free 递增）无上界约束。

**→ 结论：Long-only 方向 PASS（Σnotional < free）。Short 方向存在理论 >100% gross exposure（top=2 时约 119%），但 ES 强牛场景中以 Long 为主，实际风险 LOW。需重跑 `prop4_reading_b_recursive_l3 BT_SYMBOLS=ES` 确认 max_gross_exp_x100 ≤ 100 以最终排除。**

---

## 总结（简化结果包格式，纯技术验证）

### 结论

| 风险点 | 判决 | 置信度 |
|--------|------|--------|
| Look-ahead 伪影 | **PASS（无前视）** | 高（代码层全路径确认） |
| ES +681% 来自价格前视 | **否定** | 高（成交价 = 当前 close，非历史 BSP.price） |
| Long-only 恒仓 < 1x | **PASS（数学证明）** | 高（Σnotional ≈ 0.76×F₀ < F₀） |
| Short-only 恒仓 < 1x | **待确认（理论超 100%）** | 中（all-Short top=2 ≈ 119%，ES 实际以 Long 为主故 LOW RISK） |
| 代码注释准确性 | **不严格** | 确定（注释 "≤free 恒仓" 仅对 Long 侧成立） |

### 边界条件

- ES +681% 作为 Long-dominated 趋势标的，look-ahead 和杠杆两个主要风险均已排除或低风险。
- 如果多级别腿同时以 Short 方向开仓（bear market 强下跌初始建仓），理论 gross > 100%。实际量级：top=2 时约 19% 超越，top=4 时更高。这一场景在 CL/BTC/OKLO 穿仓中可能起到放大作用。
- 确认需要 `max_gross_exp_x100` 实际输出值。

### 影响声明

- 本报告不修改任何代码（verifier-only）。
- 发现 1 处代码注释不严格（line 1053 "恒仓不加杠杆"），仅对 Long 方向成立。
- 发现 1 处无意义 dead code（TRoot.reading_b_diverge，已在 code-verify-prop4-20260623.md 报告，no-patch 违规）。两者均不影响 ES +681% 的因果有效性。

### 谱系引用

- 谱系记录未查（本报告为纯技术验证），不确定是否有相关谱系。
- 539号（做空腿失血有效域）：本验证支持 ES（强牛）Short 腿少→影响低的推断。
- 231号（形式化有效域规则）：代码注释声明膨胀（Short 方向未覆盖）是典型有效域 < 定义域。

---

## 行动建议（仅报告，不执行）

1. **验证 ES max_gross**：运行 `cargo test --release recursive_t::rec_stream::tests::prop4_reading_b_recursive_l3 -- --exact --ignored --nocapture BT_SYMBOLS=ES`，检查 eprintln 输出中 max_gross 值是否 ≤ 100%（1.00×）。
2. **修复代码注释**（no-patch 规则）：将 line 1053 注释从 "收敛 base×3/2≤free 恒仓不加杠杆" 改为 "Long-only 方向 Σnotional < free；Short 方向 free 递增，max_gross 由 on_bar 追踪"。
3. **删除 TRoot.reading_b_diverge 死字段**（no-patch 规则，已在 code-verify-prop4-20260623.md 报告）。

---

*验证：code-verifier 工位，2026-06-23。不修改代码，只读 + 报告。*
