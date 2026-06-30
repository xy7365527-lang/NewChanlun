# PDF §11 walk-forward 验收：train-only 挑类（除 codex Q2 BIAS-FATAL 选择偏差）

**认识论等级**：L2（真实数据单标的，train-only 选类已除选择偏差；仍单标的，非 L3 跨品种）。
**与被否证 acc_level0sell_oos 的差异**：选类只用 train 段（PDF§6/codex：用挑赢家同一数据验证不能反驳挑赢家）。
**复算**：`cargo test -p <crate> --release acc_walkforward_trainonly -- --ignored --nocapture`（确定性，含固定种子 bootstrap）。

## 数据 / 窗口
- BTC 全量 4613599 bar；截断窗 [2025-11-04→2026-05-31]，bars=300000（最后 300000，OOM 边界=显式有效域）。
- 主切分 train_frac=0.6，split=180000（2026-03-09，半开 train=[0,180000) / holdout=[180000,300000) 无重叠）。

## 任务1+2：train-only 挑类 → 锁 holdout 评估（核心：除 Q2）
- **train 期最强正类 = (level=0,δ=-1)**
- **train 赢家是否 level0卖？是**（level0卖 train 期 Σpnl=2.1914e4/n=219）

| holdout 评估 train 选定类 (level=0,δ=-1) | 值 |
|---|---|
| holdout n (=nraw) | 128 |
| **holdout Σactual_pnl** | **7.8697e3** |
| holdout 胜率 | 45/128 (35%) |
| holdout Σab_rev | 2.9633e4 |
| μ_OOS | 6.1482e1 |
| se（用 neff） | 2.4009e2 |
| **LCB（μ−1.645·se，单侧5%）** | **-3.3346e2** |

**判定**：holdout μ_OOS > 0，**LCB ≤ 0**（PDF§7.3 强判据 χ=1[LCB>0]=false）
→ **Q2 消除，但 μ_OOS 正而 LCB≤0（未通过 §7.3 强判据）**：level0卖确是 train-only 赢家（选择偏差消除，这一点为正），但 holdout μ_OOS=6.15e1 扣除不确定性后 LCB=-3.33e2≤0 ⟹ **+7.87e3 不达可交易阈值**。neff/nraw 见下（事件聚集致有效样本远小于 nraw）。这不是「干净 OOS 正」——是「选择偏差消除后，弱正信号被不确定性吞没」。

## 任务4：neff vs nraw（PDF§6 条件二，自相关修正）
| 量 | 值 |
|---|---|
| nraw（holdout 赢家类原始信号数） | 128 |
| Σρk（k=1..20，仅正自相关） | 1.8839 |
| **neff = nraw/(1+2Σρk)** | **26.8** |
| neff/nraw | 0.21 |
（neff≪nraw ⟹ 事件高度聚集，有效检验力远低于原始 n；se 已用 neff）。

## 任务5/Q4：赢家集中度（剔最大赢家 + block bootstrap）
| 剔除 | 剩余 Σpnl | 仍正? |
|---|---|---|
| 不剔（全量） | 7.8697e3 | 是 |
| 剔最大 1 | 4.2170e3 | 是 |
| 剔最大 3 | -2.8967e3 | 否 |
| 剔最大 5 | -9.5885e3 | 否 |
- **block bootstrap p（H0:μ≤0，block_len=20，B=2000，固定种子）= 0.3713**（p<0.05 ⟹ 正均值稳健远离 0）。
→ **Q4 坐实少数大赢家驱动**：剔最大 5 后转负 ⟹ holdout 正总和由极少数大赢家撑起，非稳健 alpha。

## 任务3：多窗滚动 walk-forward（除 Q5 单切分脆弱，K=4）
每窗：窗内 train(前60%) 挑类 → OOS(后40%) 评估该类（每 train 只用窗内过去）。
| 窗 | train赢家 | =level0卖? | OOS n | OOS Σpnl | OOS LCB | LCB>0? |
|---|---|---|---|---|---|---|
| 1 | (level=2,δ=+1) | 否 | 0 | -0.0000e0 | 0.0000e0 | 否 |
| 2 | (level=0,δ=-1) | 是 | 39 | 4.0458e4 | -2.5731e2 | 否 |
| 3 | (level=1,δ=-1) | 否 | 3 | -2.7317e3 | -1.6164e3 | 否 |
| 4 | (level=1,δ=-1) | 否 | 2 | 3.2022e3 | -5.6487e2 | 否 |

- **各窗 OOS LCB>0 占比 = 0/4 (0%)**
- level0卖为 train 赢家的窗数 = 1/4
→ 无窗 LCB>0 ⟹ 扣除不确定性后无窗稳健正，OOS 不稳健（Q5 坐实）。

## 结果包六要素（完整版）
1. **结论**：train-only 赢家=(level=0,δ=-1)（=level0卖，Q2 选择偏差消除），holdout μ_OOS=6.1482e1/**LCB=-3.3346e2**；多窗 LCB>0 占比=0%（0/4）；neff/nraw=0.21（27/128）；剔最大5 转负；bootstrap p=0.371。**综合判定：Q2 选择偏差消除（level0卖确为 train-only 赢家），但 §11 稳健性验收未过——LCB≤0/少数大赢家驱动/多窗不稳之一以上成立 ⟹ +7.87e3 不达可交易 alpha 阈值（诚实否证，161/formalization-validity-domain）**
2. **定义依据**：actual_pnl=δ(Pτout−Pτin)−Ce（664-Q3 真实成交口径）；train-only 挑类=PDF§11「train 决定规则」；neff=nraw/(1+2Σρk)=PDF§6 条件二；LCB=μ−1.645se=PDF§7.3。
3. **边界条件**：单标的 BTC L2——结论翻转条件：(a) L3 跨标的若 level0卖不普遍赢则 BTC 是品种特例；(b) train_frac/窗数 K 改变 train 赢家身份则选类不稳；(c) holdout 仍为下跌段则卖优势含方向效应。
4. **下游推论**：level0卖不可单独作 entry——§11 稳健性验收未过（LCB≤0/赢家集中/多窗不稳），下游策略勿基于 +7.87e3 升基座。Q2 消除只证「不是挑赢家产物」，不证「是可交易 alpha」——二者独立。
5. **谱系引用**：663 econpositive；664-Q3 真实成交口径；codex Q2 BIAS-FATAL（codex-oos-level0sell-audit-20260630.md）；PDF§6/§11（overfit-consult-20260630.txt）；161（务实=留缺口）；formalization-validity-domain（L2 有效域<定义域）。
6. **影响声明**：新增 acc_walkforward_trainonly 测试 + train_winner_class/neff_autocorr/mean_se_lcb/drop_top_winners/block_bootstrap_pvalue helper（均纯函数，L1 自检 walkforward_helpers_l1）；复用 slice_bar_range/decompose_capturable_spread/class_actual_pnl；不改生产代码/TradeRecord/Order。被否证的 acc_level0sell_oos 保留（谱系：选择偏差的发生史）。
