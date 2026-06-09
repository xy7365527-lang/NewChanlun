# P1v2 验证实验报告：L2/L3 走势级别检验 ⋆ = 缠论 D 算子

## 命题（承自 P1v1）

J = D(F) = 残差曲率经缠论递归后的**方向性力度**。正交分解：
- `persistence` = |力度| = **ker(D)** 成分（无向幅度）
- `direction` = 方向 = **image(D)** 成分（±1）
- `J = persistence × sign(direction)` = image(D) 在有向价格空间的投影

P1v1 在 **L1 退化 bar** 上否证 ⋆=D（persistence≡amplitude → J 退化为『带随机符号的振幅』，方向一致率≈50%），但留下有效域口子：**『L2/L3 走势跨多 bar，persistence 与 amplitude 可能分离，结论可能改变』**。P1v2 关闭这个口子。

- 引擎：`newchan_rust.RecursiveOrchestrator max_levels=6 stroke_mode=wide`
- 样本：2,028,891 bar，2018-12-26 01:00:00+00:00 → 2026-06-05 20:59:00+00:00，214,466 笔
- 残差：`r = log(DX) - 0.576*log(USD6E) = log(DX)+0.576*log(EURUSD)`
- L2/L3 bar 锚定：L>=2: comp_start/comp_end 递归解析 settled-(L-1) move 子集 → stroke → bar（精确，无轮询）

## 级别涌现与走势数

| 级别 | 走势数 |
|------|--------|
| L1 | 1359 |
| L2 | 115 |
| L3 | 10 |
| L4 | 1 |

## 步骤4：persistence 与 amplitude 是否分离？（L0 代数恒等式）

**任务前提**：L2/L3 走势跨多 bar，persistence（力度）与 amplitude（振幅）应天然分离，从而 J 的方向投影携带超出纯振幅的独立信息。

**实测**：
| 级别 | 走势数 | persistence≠amplitude 的走势 | 最大相对差 |
|------|--------|------------------------------|-----------|
| L1 | 1359 | **0** | 0.00e+00 |
| L2 | 115 | **0** | 0.00e+00 |
| L3 | 10 | **0** | 0.00e+00 |
| L4 | 1 | **0** | 0.00e+00 |

**前提被证伪。** 所有级别 persistence ≡ amplitude（逐位全等）。原因是**代数恒等式**，非退化 bar 偶然：

- 引擎 persistence = 1D sublevel-set **H0 persistence**（`rust/src/ph.rs:37` `compute_move_persistence`），其闭式为走势内所有中枢中心 (dd,gg) 的 **max−min**。
- 走势 high/low（`rust/src/level.rs:242` `group_to_move`）= 同一组中枢的 **max(gg) / min(dd)**。两者取自**同一批中枢**，故 persistence = max(gg)−min(dd) = high−low = amplitude，**恒等**。
- 拓扑根因：**1D 信号的 H0 persistence 恒等于其极差**（sublevel-set 只有一个连通分量，从全局极小持续到全局极大）。这在**每一级别**都成立，与 bar 是否退化无关。
- 引擎 `ph.rs:36` 自标：persistence 的**认识论等级 = L0（代数恒等式，零信息增量）**。

**推论**：P1v1 把否证归因于『L1 退化 bar』并保留『L2/L3 可能分离』的口子，**这个口子在引擎层不存在**。J = persistence×direction = amplitude×direction 在**所有级别**都成立——J 相对纯振幅的唯一新增信息**始终**是方向符号，L2/L3 不改变这一点。

## 步骤6-7：L2/L3 翻转点 J 方向 vs 残差后续方向（L2 实证）

即便 persistence 不携带超出振幅的信息，**方向符号本身**在 L2/L3 较粗尺度（接近月级 regime 尺度）是否携带预测内容？两个互补检验：

- **T1 位置一致性**（承 P1v1 全样本）：走势翻转起点是否为残差局部极值且与新方向一致（down-flip 起于残差顶 / up-flip 起于残差底）。
- **T2 前瞻动量**（无前视，纯样本外）：从新走势 **settle bar**（方向已确认）起，残差在前瞻日历窗口内是否沿 J 方向移动。

### L2（114 settled 走势，58 次方向翻转）

**T1 位置一致性**：9/16 翻转起点为残差局部极值且方向一致，一致率 **56.2%**（二项 p=0.804）。

**T2 前瞻动量**（各前瞻窗口）：
| 前瞻(天) | 一致/可测 | 一致率 | p |
|---------|----------|-------|---|
| 5 | 58/114 | 50.9% | 0.925 |
| 10 | 53/114 | 46.5% | 0.512 |
| 20 | 52/114 | 45.6% | 0.399 |
| 40 | 51/114 | 44.7% | 0.303 |

### L3（9 settled 走势，6 次方向翻转）

**T1 位置一致性**：0/4 翻转起点为残差局部极值且方向一致，一致率 **0.0%**（二项 p=0.125）。

**T2 前瞻动量**（各前瞻窗口）：
| 前瞻(天) | 一致/可测 | 一致率 | p |
|---------|----------|-------|---|
| 5 | 6/9 | 66.7% | 0.508 |
| 10 | 7/9 | 77.8% | 0.18 |
| 20 | 5/9 | 55.6% | 1 |
| 40 | 8/9 | 88.9% | 0.0391 |

## Regime 反转点标注（L2/L3 精确 bar 索引）

用 L2/L3 走势的精确起止 bar（非天级日期）标注最近的方向翻转：

### L2

| regime 反转 | 最近 L2 flip 起点 bar | flip 日期 | 新方向 | 滞后(天) |
|------------|----------------------|----------|--------|---------|
| 2022-03-16 Fed 加息启动 | 872,382 | 2022-03-28 | down | +12 |
| 2022-09-28 DXY 见顶 114.78（内生反转） | 1,045,380 | 2022-10-21 | up | +23 |
| 2024-09-18 Fed 降息启动 | 1,555,937 | 2024-09-20 | down | +3 |

### L3

| regime 反转 | 最近 L3 flip 起点 bar | flip 日期 | 新方向 | 滞后(天) |
|------------|----------------------|----------|--------|---------|
| 2022-03-16 Fed 加息启动 | 975,327 | 2022-08-01 | down | +139 |
| 2022-09-28 DXY 见顶 114.78（内生反转） | 975,327 | 2022-08-01 | down | -57 |
| 2024-09-18 Fed 降息启动 | 1,535,544 | 2024-08-23 | down | -26 |

## 判决

**1. 前提层（L0，决定性）**：persistence ≡ amplitude 在 L2/L3 **恒等**（1D H0 persistence = 极差，代数恒等式）。P1v1 留下的『L2/L3 可能分离』有效域口子**被关闭——它在引擎层从不存在**。J 在所有级别都退化为『带符号的振幅』，方向是唯一新增位。

**2. 实证层（L2）多重比较校正**：共 10 个方向检验（L2/L3 × T1/T2×4 窗口）。Bonferroni 阈 = 0.05/10 = 0.0050。统计判决另要求可测样本 N≥20（否则为案例级）。

- **名义 p<0.05**：L3-T2@40d（89%, p=0.0391, N=9）。
- **校正后稳健**：无。T1 位置一致性与 T2 前瞻动量在 L2/L3 经多重比较校正后**均不显著偏离 50%**。唯一名义命中（L3-T2@40d 89%）N≈9 为**案例级**，且是 10 个检验中最小 p——family-wise 校正后 ≈0.33，**不显著**，是多重比较伪影，非方向预测信号。方向符号在较粗尺度上同样不携带稳定的残差方向预测——与 P1v1 的 L1 否证一致，**尺度升级未救回 ⋆=D**。

- **L2 位置错配**：58 次方向翻转中仅 16（28%）起点落在残差局部极值——72% 的翻转发生在残差中段，与曲率 F 极值在**位置上**就不对齐（承 P1v1 L1 同型发现）。
- **L3 位置错配**：6 次方向翻转中仅 4（67%）起点落在残差局部极值——33% 的翻转发生在残差中段，与曲率 F 极值在**位置上**就不对齐（承 P1v1 L1 同型发现）。

**3. 综合判断**：⋆=D 在 L1 与 L2/L3 **一致否证**。否证不再限定于『退化 bar』——根因是 persistence（1D H0）≡ amplitude 的拓扑恒等式贯穿所有级别，J 的信息增量（方向符号）既不来自 persistence/amplitude 分离（不存在），也不在 L2/L3 翻转点携带残差方向预测。这与谱系『残差→流量无免费桥梁』一致：缠论 D 算子不充当 Hodge star，残差曲率 F 到流量 J 仍缺金融度规。

## 边界条件（结论翻转条件）

- **persistence 定义**：结论建立在『persistence = 1D sublevel-set H0 = 极差』上。若改用**多维 filtration**（如 (价格, 时间) 或 (价格, 成交量) 二维 PH），H0/H1 persistence 不再恒等于价格极差，persistence 与 amplitude 可能真正分离 → 结论可能改变。**当前否证限定于 1D 价格 persistence。**
- **方向检验参数**：残差局部极值窗口 ±30天、分位 0.8；前瞻窗口 (5, 10, 20, 40) 天。窗口/阈值改变会移动 T1/T2 可测样本与一致率。
- **L3 样本量**：L3 走势数少（见上表），二项检验功效低；L3 结论是案例级，不构成独立 L3 交叉验证。跨残差构造（多锚/多系数）的 L3 验证不在本实验范围。
- **退化 bar 与方向分离正交**：本结论的核心（persistence≡amplitude）**不依赖**退化 bar——即使非退化 bar，1D 价格 H0 persistence 仍≡极差。P1v1 的『非退化 bar 可能救回』推测在 1D persistence 下**不成立**（需多维 filtration 才可能）。

## 定义依据

- **残差**：`r = log(DX) − 0.576·log(USD6E)`，协整系数既定（memory: project_residual_to_flow_no_bridge，占 DX 方差 33.8%）。
- **走势/方向/力度**：缠论正典走势由中枢定义，方向=上行/下行，力度=persistence。由 newchan_rust.RecursiveOrchestrator 产出（与正典引擎 bit-exact）。
- **persistence = H0 闭式**：`rust/src/ph.rs:37` `compute_move_persistence`，走势内中枢中心 (dd,gg) 序列的 max−min，引擎自标 L0。
- **L2/L3 bar 锚定**：`rust/src/level.rs:265` first_seg_s0/last_seg_s1 = comp_start/comp_end = settled-(L-1) move 子集的 component 索引（`orchestrator.rs:426` `filter(settled).enumerate()`），递归解析至 stroke→bar，精确无轮询。
- **D 算子分解**：persistence∈ker(D)、direction∈image(D) 是本实验对 P1 命题的操作化映射假设（非缠师原文），受本检验约束。

## 下游推论

- **P1v1 的有效域口子关闭**：『L2/L3 可能分离』与『非退化 bar 可能救回』两个口子在 1D 价格 persistence 下均不成立。⋆=D 的否证从『限 L1 退化 bar』升级为『限 1D 价格 persistence，贯穿所有级别』。
- **唯一未关闭的口子是多维 filtration**：若 persistence 来自 (价格,时间) 或含成交量的多维 PH，H0/H1 才可能与价格极差分离。这是 ⋆=D 唯一尚存的潜在有效域，且需重新定义引擎的 persistence 层（当前为 1D 闭式）。
- **流量仍须独立源**：d⋆F=J 缺金融度规的判断被独立加固——缠论 D 算子（在 1D persistence 下）不提供残差曲率 F 到流量 J 的桥梁。

## 谱系引用

- **P1v1**（`analysis/p1_hodge_star_equals_D.md`）：L1 退化 bar 上 ⋆=D 否证 + 保留 L2/L3 / 非退化 bar 有效域口子。本实验是 P1v1 的直接续作与口子关闭。
- **残差→流量无免费桥梁**（memory: project_residual_to_flow_no_bridge）：残差=曲率（严格）但 d⋆F=J 缺金融度规。本实验确认 ⋆=D 不在 L2/L3 提供该桥梁。
- **PH 性能真瓶颈 / persistence≡max-min**（memory: project_ph_perf_streaming_oN2）：persistence 闭式 = max−min 已逐位等价落地——本实验把该工程事实提升为对 ⋆=D 的理论否证（H0≡极差贯穿所有级别）。
- **formalization-validity-domain**：persistence≡amplitude 是 L0 同义反复；本实验标注 L0（前提分离）与 L2（方向预测）等级，否定性结果缩小 ⋆=D 有效域至空集（1D persistence 下）。

## 影响声明

- 新增：`analysis/p1v2_hodge_star_L2L3.py`、本报告、`analysis/data_cache/_p1v2_l2l3_moves.json`（各级 move + 精确 bar 锚定缓存）。
- 不改动任何既有定义/模块/引擎。component→bar 递归解析是对既有引擎输出的**读取**，非修改。
- 改动的认知：P1v1 报告中『L2/L3 不可判定』『非退化 bar 可能救回』的有效域留口，经本实验在引擎层关闭（1D persistence 下 ⋆=D 否证贯穿所有级别）。