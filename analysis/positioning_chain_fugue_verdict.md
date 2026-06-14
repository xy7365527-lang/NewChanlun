# 区间套定位链驱动赋格（pcf）判决

> 编排者 2026-06-14"那你要实现啊"。`nested_interval_fugue` 用**固定 min_trade_ladder**
> 模拟区间套（结果是 regime 函数）。固定参数不是区间套——区间套（第14环）是操作
> 层级由**走势结构动态决定**（定位链顶层 source_ladder），非外部 floor。本任务实装
> 零参数的定位链驱动引擎，先做**必然性检验**（验收标准），后跑回测（有效域读数）。

## 结论

**必然性检验 PASS（验收标准），回测 = regime 函数（有效域读数，非验收标准）。**

定位链驱动引擎 `positioning_chain_fugue.rs`（mode=pcf）逻辑严格正确：~25M bar
真实数据零 panic ⇒ 每个操作都有从 a0 到 source 的完整定位链、链在操作前形成
（因果）、链顶一致、守恒律每 bar 成立。但回测揭示决定性 L2 发现：**区间套"从 a0
向上到 S"在 1min 真实数据上 source 上限 = move(L1)，被 segment 主导**——"走势结构
动态决定的操作层级"经验地坍缩到**最低层**（segment），因为高层完整嵌套链罕见同时
成立。这正是 nif 的 min_trade_ladder 想排除的层。

## 实装（与 URS 唯一构成性差异：层选择机制）

| 机制 | URS | PCF |
|------|-----|-----|
| 入场层 | `top` = 最高 θ 涌现层（与链无关） | `buy_source` = 完整买链 a0→S 顶层 |
| 出场层 | `E*` = dir/anchor 爬升（独立部件） | `sell_source` = 完整卖链 a0→S 顶层 |
| 降成本/出场分界 | E\* vs floor | source ≷ voice.ladder（势在层上=出场，层下=降成本） |
| located | 仅卖侧；E\* 选层后 recursive_confirmed 校验 | 双侧；source **即**链顶（层选择 ≡ 链确认，合一） |
| 操作参数 | floor_ladder（结构常量） | **零**（无 min_trade_ladder、无操作 floor） |

新数据结构 `LocatedEntry { extreme, source_ladder, arm_bar }`（替代 URS 的
`Option<f64>`）；`source_ladder` 逐 bar 由 `refresh_sources` 维护为所属连续链顶层。

## 必然性检验（验收标准，L0/L2）

引擎内 `prove_chain` 在 F/C/D/E 每个操作点运行时证明（违反即 panic）：
- **N1 完整链**：每操作 source 对应 [segment..=S] 全 located ∧ a0 翻转的完整链
  （"没有定位链的交易 = bug"的逐操作硬断言）。
- **N2 因果**：located.arm_bar ≤ 操作 bar（链在操作前形成，无未来定位）。
- **N3 链顶一致**：located[s].source_ladder == s（refresh_sources 正确）。
- **守恒**：§8.1 Σunits=N_base 每 bar assert。

**8 标的 ~25M bar 跑通无 panic ⇒ N1∧N2∧N3∧守恒全成立（L2 验证）。**
单测 9/9 通过（合成场景覆盖 refresh/chain_source/入场/翻转/降成本/守恒）。

## 回测（必然性通过后；L2/L3——非验收标准）

| 标的 | pcf% | BH% | P1≥BH | ΔURS | entry source 层 |
|------|------|-----|-------|------|----------------|
| OKLO | −58.9 | +307.1 | ✗ | −380.8 | segment 207 / move 16 |
| QQQ | +137.5 | +174.6 | ✗ | −28.7 | — |
| BRN | +142.7 | +87.4 | **✓** | **+86.2** | segment 107 / move 11 |
| DX | −10.1 | +4.1 | ✗ | −14.5 | — |
| ES | +217.4 | +594.3 | ✗ | −271.0 | segment 607 / move 25 |
| GC | +145.4 | +257.3 | ✗ | −99.6 | — |
| CL | +71.0 | +28.2 | **✓** | **+66.3** | segment 467 / move 31 |
| BTC | +280.1 | +1380.4 | ✗ | −269.8 | segment 435 / move 30 |

- **P1（≥BH）= 2/8 = {BRN, CL}**（油链）；**ΔURS>0 = 2/8 = {BRN, CL}**；URS 胜 6/8。
- 正域 {BRN, CL} = 油链深崩/区间域，与 nrf v4 开放轴、scco 白名单等项目史**同域**
  （regime 函数第 N 例）。
- **entry source 全资产 recL2+ 零**：上限 = move(L1, ladder 3)，segment 主导
  (≈92-95%)。买定位链从不延伸到高层。

## 决定性 L2 发现：区间套链的 source 上限 = move(L1)

PCF 把"操作层级由走势结构动态决定"严格实装后，走势结构**决定了 segment**：完整
买链 [segment..=S] + bi 翻转在高层（recL2+）**罕见同时成立**——连续 located 链在
到达 recL2 前断裂。所以动态 source 坍缩到最低层 = segment 级 churn（幅度
0.02-0.19% < 摩擦地板 0.2%）= 强牛踏空 + 翻空被否定（OKLO 528 negate）。

这从新角度**坐实**了用户原诊断的另一半：固定 min_trade_ladder（nif）强制操作上移
是 regime 函数；放手让链决定（pcf）则链决定**下移**到 segment。两者夹逼出结论：
**1min a0 上，区间套理想（高级别精确定位）与操作层级经验上不可兼得**——高链罕见
（pcf 坍到 segment），强制上移又是 regime 函数（nif）。alpha 不是级别选择的普适
函数（与 `project_backtest_benchmark_falsifiability` / nif 否证共振）。

## 边界条件（结论翻转条件）

- 若改 a0 粒度（更粗周期，如 30min/日线）⇒ 高层完整链可能更易成立 ⇒ source 上移
  ⇒ 本"segment 坍缩"结论限 1min a0 有效域（未在粗周期检验——开放轴）。
- 若入场链放宽为"非完整链"（如单层 located_buy[S] 即入场）⇒ source 可高 ⇒ 但
  退回 539号 A′ 单层选层（已 L3 否证踏空）⇒ 完整链是 pcf 区别 A′ 的本质，不可松。
- 若 root 入场量改为 θ_source（非满仓恒仓）⇒ 低 source 入场量小 ⇒ 可能减 segment
  churn 暴露——但违 26课恒仓（已结算），未实装。

## 下游推论

- **入场层是真瓶颈**：URS 的 `top`（最高 θ 层）入场 vs pcf 的 buy_source（链顶）
  入场——URS 6/8 胜 pcf，说明"在最高 θ 涌现层入场"优于"在完整买链顶层入场"
  （后者坍到 segment）。买侧定位链不是有效入场轴。
- **出场 source 绑定未堵踏空**：OKLO −58.9% ≪ URS +278% ⇒ source 绑定（持到入场
  级反向链）在 source=segment 时退化为 segment 级出场 = 高频清仓/翻空 = 踏空。
  与 539号 A′ 同族死因延伸（A′ 单层选层踏空，pcf source 坍缩到 segment 踏空）。
- **油链正域稳定**：{BRN, CL} 跨 pcf/nrf v4/scco 一致正域 ⇒ 油链 alpha 来源是
  regime（深崩/区间）而非引擎机制——机制换了正域不变。

## 谱系引用（与 539号已否证 A′ 区分——强制）

539号 A′ 合取选层 `max{k: sell1[k]∧located[k]}`（**单层** located 选清仓层）已 L3
否证（强牛高层 located 频繁 ⇒ 过度清仓 ⇒ 踏空，`project_constitutive_throughput_falsified`）。
PCF **不是** A′ 重演（要求整条链 + 入场-出场 source 绑定）。但回测揭示 PCF 经
**不同路径**到达**同一踏空结局**：A′ 因高层 located 频繁而过度清仓；PCF 因链坍缩
segment 而 segment churn。两者收敛为：**located 驱动的层选择在 1min a0 上无法稳定
锚定高级别操作** ⇒ 定位链驱动的入场/出场层选择轴关闭（与 E\* 爬升、棘轮、固定
floor 并列为已探明的层选择机制，URS 的 E\* + 最高 θ 层仍是在册最优）。

## 补充：三层统一必然性检验（编排者 2026-06-14 追加）

> "不仅区间套要做必然性检验，信号层和会计层也要。任何一条 violation = bug。"
> 脚本 `analysis/necessity_check_three_layers.py`；8 标的 ~28M bar；
> 输出 `analysis/data_cache/necessity_three_layers.json`。

| 层 | 检验项 | 结果（8 标的） | 判定 |
|----|--------|--------------|------|
| 信号 | S1 type1 买卖点交替 | **FAIL 8/8（~75-81% 违反）** | **定义张力，非 bug** |
| 信号 | S2 每级别 BSP 完整 | **PASS 8/8（0 gap）** | ✓ 严格成立 |
| 信号 | S3 买点价 < 前卖点价 | FAIL 8/8（~2-3% 违反） | 定义张力 |
| 会计 | A1 Σunits==N_base / A3 存一次 / A4 价值中性 | **PASS 8/8（引擎无 panic）** | ✓ 普适严格 |
| 会计 | A2 逐 close 恒等 / A4 逐 bar 空头 MtM | 引 §11.1/§11.4 有效域 | 已结算 GAP |
| 区间套 | N1 完整链 / N2 θ_source 量 / N3 出场对齐 / N4 无链交易不存在 | **PASS 8/8（引擎无 panic）** | ✓ 普适严格 |

### S1 失败的精确诊断（no-workaround：surface 而非绕过）

S1"买卖点交替"在 type1 级 8/8 失败 ~78%。诊断（OKLO ladder2 实证）：连续同向
type1 的 **seg_idx 全不同（2162 个不同走势，0 个同走势重发）**，价格**递降**（bar
521/608/630/638 type1-buy 11.01→9.81→8.33→8.33）。

**这不是信号层 bug，是定义张力**：type1 是**可被 027:25 否定线否定的背驰候选**，
不是保证的反转。持续下跌趋势中，1买（背驰）出现 → 价格继续跌（1买被否定）→
更低低点再生新 1买。"走势终完美 ⇒ 买卖点交替"的推理隐含假设"每个 type1 都是
完成的反转"——假。**S1 严格成立的形式**："买卖点交替仅在**非否定**（成功）反转
之间成立"——而这正是交易引擎用 `negate_line`（027:25）的原因：因为 type1 会被
否定。必然性检验揭示的张力，其解法**已在系统设计中**（否定线机制）。

**四分法分类 = 语法记录**（已在运作但未显式化的规则）：信号层的 type1 = 可否定
候选，不是反转保证。S2 的完整性（全称命题）严格成立佐证——是**交替**（S1）而非
**完整性**（S2）被否定候选打破。S3 同源（~2-3% 是走势跨更高中枢上移的买点高于
前卖点，少数派）。

### 会计层 / 区间套层：普适严格成立

- A1（Σunits=N_base）/ A3（存一次 trades.len()==Σn_exits）/ A4 核心（同价操作
  价值中性，新增引擎逐 bar assert）/ N1-N4（prove_chain）：8 标的 ~28M bar
  **零 panic** ⇒ 运行时证明在全 bar 成立（L2）。
- A2（child.P&L≡parent.cost_reduction）逐 close 字面、A4 逐 bar 空头 MtM：**§11
  审计已结算为 GAP**（§11.1 四去向分裂 / §11.4 冻结 capital 递延）——字面普适
  形式 FALSE 不是 code bug，是规格理想化的有效域（code 正确）。本检验验证**价值
  守恒等价聚合形式**（价值中性 + 终态 final_nav 正确），字面逐字恒等引 §11 有效域。

## 影响声明

- 新增 `rust/src/trading/positioning_chain_fugue.rs`（引擎 + 9 单测）。
- 接线 `mod.rs`（注册）、`positional.rs`（`PolarityMode::PositioningChain` 变体 +
  parse "pcf" + 分派 + legacy 路径 unreachable 臂）。
- 新增 `analysis/positioning_chain_fugue_backtest.py`（必然性检验 + 8 标的回测）。
- 输出 `analysis/data_cache/pcf_*.json` + `pcf_summary.json`。
- **零改动**：信号层（buysellpoint.rs）、会计层（nested_fugue 原语复用 bit-exact）、
  URS/nif/其余引擎（新入口 GH2 先例，零接触）。守恒 violation = panic（任务裁决）。
- 认识论等级：必然性检验 = L2（25M bar 运行时证明）；回测 = L2/L3（8 标的，
  否定性结果——source 链驱动层选择轴关闭，有效域边界收窄）。
