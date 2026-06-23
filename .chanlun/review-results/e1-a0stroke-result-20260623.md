---
trigger: "E1 实验：a0=Stroke 1min BTC+CL（任务 #31，565号 ghost-owner 孤儿重启）"
target: "信号层分辨率升级假设（主线方向#1）的最小可否证检验"
mode: "L2 真实数据 A/B 受控实验"
result: "P_bi 决定性否证 —— a0=Stroke 在当前操作层下 BTC/CL 双标的 strat≈−100%"
date: "2026-06-23"
epistemic_level: "L2（真实数据单标的×2 假设检验，产生否定性结果）"
depends_on: ["526", "531", "547(#32已完成)", "552", "556", "557"]
commit_base: "b01029b42a（prop4-nest-readingB-20260623，含 547 cascade 修复）"
---

# E1 实验结果：a0=Stroke 信号层分辨率升级假设否证

## 一、实验设计（受控 A/B，唯一变量 = a₀ 来源）

| 维度 | 控制 | 变化 |
|------|------|------|
| bar 粒度 | 1min（固定） | 不变 |
| 标的 | BTC + CL（固定） | 不变 |
| 三模式 | Structural/AND/OR（同一 a₀ clone 三份） | 不变 |
| **a₀ 来源** | **Segment（基线）** | **→ Stroke（唯一变量）** |
| 代码 commit | b01029b42a | 同一 commit（无漂移） |

`a0=Stroke` 与 `a0=Segment` 唯一差异：过滤口径。Stroke=`confirmed` 笔序列，
Segment=`confirmed && Settled` 线段序列（`backtest.rs::build_a0_from_strokes`
vs `build_a0_from_segments`，单元构造逐字一致，仅基序列不同）。操作层
（`apply_bsp` 长仓+减仓1/3+无回补）两路**完全相同** ⇒ A/B 干净。

## 二、结果矩阵（同 commit，同时段，同操作层）

| 标的 | 指标 | a0=Segment（基线） | a0=Stroke（变量） | Δ |
|------|------|-------------------|-------------------|---|
| **CL** | a₀ 单元数 | 36,374 段 | 327,900 笔 | **9.0×** |
| | r*（递归深度，S） | 5 | **6** | **+1** |
| | n_bsps（S） | 6,985 | 45,980 | 6.6× |
| | n_trades（S） | 2,875 | 17,398 | 6.05× |
| | **strat% Structural** | **+120.4%** | **−99.9%** | 💥 |
| | strat% AND | +92.5% | −99.9% | 💥 |
| | strat% OR | +65.1% | −99.8% | 💥 |
| | BH | +28.2% | +28.2% | — |
| **BTC** | a₀ 单元数 | 39,891 段 | 328,952 笔 | **8.2×** |
| | r*（递归深度，S） | 5 | **5** | **+0** |
| | n_bsps（S） | 6,628 | 45,601 | 6.9× |
| | n_trades（S） | 2,685 | 17,806 | 6.6× |
| | **strat% Structural** | **+68.3%** | **−100.0%** | 💥 |
| | strat% AND | +64.4% | −100.0% | 💥 |
| | strat% OR | +196.6% | −100.0% | 💥 |
| | BH | +1380.4% | +1380.4% | — |

（MAX_LEVEL=8，观测 r*=5/6 未触顶 ⇒ 递归深度是真实涌现值，非钳制。）

## 三、结果包六要素

### 1. 结论

**P_bi 决定性否证（L2）。** 命题「a0 从线段切换为笔，带来可测的捕获率提升」
在 1min × BTC/CL 受控实验下被真实数据否证：
- 否证条件 MET：BTC a0=Stroke strat = **−100.0% ≪** 基线 +68.3%（亦 ≪ spec 旧阈 +664%）。
- CL 同样 +120.4% → −99.9%。双标的、三模式、全部塌为 ≈−100%。

**两个附带 L0 声明同时被否证：**
- types.rs:64「a0=Stroke ⇒ 级别数 5-6→8-10 的主因」：**否证**。实测递归深度
  CL 5→6（+1）、BTC 5→5（**+0**）。9× 底座单元 **未**推到 8-10 级。
- 「递归深度增加是代数必然」：**否证**。BTC 底座单元 8.2× 而 r* 不变 ⇒
  涌现深度由**行情反复**决定，非 a₀ 单元数（尺度不变性第三次确认）。

### 2. 定义依据

- 第65课 `aₙ=f(aₙ₋₁)` 065:182「区别仅在 a₀」（526号）：a0=Stroke 是合法的
  底座下移，代数上成立（L0）。
- `A0Source::Stroke` 过滤口径 = `confirmed`（笔无 Settled 语义，bi.md:151）；
  `Segment` = `confirmed && Settled`（types.rs:62-67）。两路均处「a₀ 确认层」。
- 否定性结果价值（formalization-validity-domain）：L2 否证缩小有效域边界，
  信息增量 > 确认性结果。本实验把「信号层分辨率升级」的有效域**排除**了
  「当前单调减仓长仓操作层 + a0=Stroke」这一格。

### 3. 边界条件（结论翻转条件）

否证**条件于当前操作层**——`apply_bsp` 长仓 + 减仓1/3单调 + 无短差回补 + 无做空腿。
机制：a0=Stroke → 9× 底座 → 6.6× 交易（17,000+ vs 2,800）→ **过度交易死亡螺旋**。
强牛市（BTC BH+1380%）中，6.6× 卖点在次级别反复触发减仓而无回补，核心多腿被
sink 至近零；每笔确认滞后税 × 6.6× 笔数，乘性复利塌到 −100%（非 panic，引擎跑净，
test passed）。

**翻转条件**：若操作层协同进化为支持「短差回补（H¹ 配额释放）+ 做空腿（吃每级别
涨跌幅绝对值 = 多空双开目标，547号死锁谱系）」，stroke 密度信号**可能**被吸收。
本否证**不外推**到「a0=Stroke + 协进化操作层」（未测）。

### 4. 下游推论

- **信号层分辨率升级（主线方向#1）在 E1 最小单位上、当前操作层下被否证。**
  「更深递归→滤波器展开→总和趋近 Σ|涨跌幅|」的因果链断在第一环：a0=Stroke
  **未**显著加深递归（CL+1/BTC+0），且操作层无法消化密度爆炸。
- **操作层必须先于 a₀ 细化协进化。** 单调减仓长仓床位上，越细的 a₀ 严格越差。
  这与 MEMORY 整条 1s/细 a₀ 否证谱系收敛：`pcf_1s_a0_source_collapse`、
  `cl_1s_a0_verdict`、`1s_a0_nest_coverage_falsified`、`1min_resolution_irreducible`。
- **E2（1s bar + a0=Segment）失去与 a0=Stroke 的联合动机**，但 E2（bar 粒度轴）
  与 a₀ 轴正交独立（531号），E1 否证**仅杀 a0=Stroke 轴，不杀 1s-bar 轴**。
  是否单独跑 E2 = 选择类，留编排者/Lead 裁决（spec 原设 E2 触发条件为「E1 成立」，
  现 E1 否证 ⇒ E2 不自动触发）。

### 5. 谱系引用

- 526号（a₀ 来源载体）/525号（完成口径）/531号（bar 粒度正交）/547号（cascade，
  任务 #32 已完成）/552/556/557号。
- MEMORY：`project_pcf_1s_a0_source_collapse`、`project_cl_1s_a0_verdict`、
  `project_1s_a0_nest_coverage_falsified`、`project_recursive_level_emergence`
  （聚合非 bug，瓶颈是行情反复非 bar 数 = 本实验 r* 不变的同一机制）、
  `project_1min_resolution_irreducible`、`project_deadlock_dual_open_target`
  （多空双开 = 翻转条件指向的操作层目标）。

### 6. 影响声明

- **未改动任何引擎代码。** 纯回测产出（A/B 受控实验）。
- 复用现成参数化基建：`T_A0=stroke` 环境变量（backtest_run.rs:370）+
  `build_a0_from_strokes`（backtest.rs:190）——a₀ 已完全参数化，零新增代码。
- on-disk `analysis/data_cache/t_backtest_*.json`（untracked）已复跑 segment
  恢复为基线态（避免下游脚本误读 stroke 结果）。
- 否定 spec `filter-spec-a0-stroke-20260623.md` 中「+664%」否证阈：该值来自旧
  commit dc8c，当前基线 BTC Structural = +68.3%。受控 A/B 不依赖绝对阈，
  以**同 commit 段→笔退化**为否证判据，更严格。

## 四、子工作分解评估（任务 #31 (c)递归要求）

**不分解。** E1 是原子最小可否证单位，单次 A/B 给出决定性否证。识别出的两个
自然后继均不构成「≥2 独立可立即执行子单元」：
1. E2（1s-bar 单轴）= 选择类（spec 原设 E1 成立才触发，现否证），留编排者裁决；
2. a0=Stroke + 协进化操作层 = 越界（操作层 = 547号死锁/多空双开**独立工位**，非本工位 a₀ 轴）。

故无 TaskCreate 子任务（避免投机性分解 = gold-plating）。
