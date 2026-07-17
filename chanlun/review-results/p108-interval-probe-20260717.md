# P108 区间套收敛探针（任务 #108，2026-07-17）

只读探针 `p108_interval_probe`（BTC 全量 4,613,599 bars，classifier 递归塔逐 bar 扫描，
(lvl, source_index, bits) 首见去重）。坐标基：`BspPoint.source_index` 与
`Center.start_index/end_index` 同为 L0 原始 K 序（bsp.rs / center.rs UnitRange 文档）。

## 输入与规模

- `P108_EVT`：lvl0=18,516 / lvl1=4,472 / lvl2=2,406 / lvl3=1,210 / lvl4=548；
  (src,side) 组 24,510，其中多级组 2,456（10.0%）；events_raw=28,417。
- `no_center3=0`：3 类 bit ⟹ center 必 Some，判据自洽（bsp.rs 文档承诺经验成立）。

## 检验 1：同点降级存在性（严格同 bar 同侧）——大面积不成立

| lvl | have | miss | miss% |
|----|------|------|-------|
| 1 | 1,308 | 3,164 | 70.75 |
| 2 | 294 | 2,112 | 87.78 |
| 3 | 91 | 1,119 | 92.48 |
| 4 | 49 | 499 | 91.06 |

「高级别点同时是低级别点」按**同 source_index 同侧**的严格口径 70–92% 缺失。
与 p107 结论一致（P107_L0NN/P107_MAT：lvl≥1 点到 lvl0 同侧点是**近邻**而非同点）。
⟹ 条款 9 实装（#106）不能用同点硬判据，否则大量误杀。

## 检验 2：嵌套收缩——窗口口径 100% 成立，中枢口径天然 disjoint

- `P108_NEST_W`（离开窗口 `[min(center.end,src), max(center.end,src)]`，相邻在场级对，
  两侧均有 center 的 86 对）：**contain/equal = 100%**（lvl1 32+5、lvl2 23、lvl3 20、lvl4 6；
  partial=disjoint=reversed=0）。低级窗口 ⊆ 高级窗口，收缩方向全数成立。
- `P108_NEST_C`（最后中枢区间）：几乎全 disjoint（81/86）。这**不是破例**：低级别最后中枢
  是高级别离开段内部对象，与高级别最后中枢本就不同段，结构上应当错开。
- `nocenter` 对数大（1,271/670/396/219）：1/2 类 BspPoint 的 `center` 字段可为 None，
  样本因此偏小——是字段可得性限制，非结构破例。

## 检验 3：lvl0 极值 bar 收敛——组内单调 100%，点≠极值是常态

- `P108_MONO`：多级窗口组 114/114 **单调收敛**（|极值bar−src| 随级别下降非增，100%）。
- `P108_CONV`：lvl0 exact0 仅 3.86%（547/14,155），主体 ≤1440 bar（日内）；
  lvl≥2 以 far 为主。原因：a) 2/3 类点本就不在极值；b) 窗口含中枢末端区间，极值常落在
  中枢内/前一极值处。⟹ 条款 9 不应要求「点 bar = 窗口极值 bar」，只应要求**逐级收敛**。

## 裁定建议（喂 #106 条款 9 实装）

1. 区间套必要条件应实装为**窗口式**：lvl k 点的确认要求存在 lvl k-1 同侧点，
   其 source_index 落在 lvl k 离开窗口内（而非同 bar）；窗口嵌套方向已 100% 经验成立。
2. 收敛检验用**单调性**（逐级 |极值距离| 非增），不用 exact0。
3. 覆盖率注意：多级组仅 10%，且两侧 center 可得的相邻级对仅 86——实装时应从
   `LevelState.centers`（每级全量中枢序列）取区间，而非依赖 `BspPoint.center` 字段，
   以消除 nocenter 空洞。

## 复现

```text
cargo run --release --features backtest_bin --bin p108_interval_probe
```
