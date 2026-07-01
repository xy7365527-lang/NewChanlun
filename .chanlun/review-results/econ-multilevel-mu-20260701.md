# acc-multilevel-sample + acc-highlevel-mu：BTC 全历史多级别 alpha 检验

**认识论等级**：L2（真实数据单标的 BTC 2017-2026 全历史，可产否定性结果，非 L3 跨品种）。
**目的**：同一次全量跑产出两条 acceptance 判定，避免重跑。
**复算**：`ECON_L2_MAX_BARS=5000000 cargo test --release acc_multilevel_highlevel_mu -- --ignored --nocapture`

## 数据
- 品种：BTC（btc_1m_full.json，全量 4613599 bar，2017-08→2026-05）
- 本次窗口：bars=4613599（2017-08-17→2026-05-31，max_bars=5000000）
- train_frac=0.6，切分 bar=2768159（2022-11-27），train=[0,2768159) / holdout=[2768159,4613599) 无重叠
- 全窗 n_signals=7742

## acc-multilevel-sample：level0-5 全历史信号数分布
Le Cam 硬墙判定（level3+）：n<30 ⟹ 结构性稀疏；n≥50 ⟹ 可 OOS 验。
| level | δ | n（全窗） | μ̂=Σactual_pnl/n（全窗，含选择偏差，仅参考） | Le Cam 判定（level3+）|
|---|---|---|---|---|
| 0 | -1 | 3648 | 2.1152e1 | — |
| 0 | +1 | 4093 | 5.8528e1 | — |
| 5 | -1 | 1 | -4.0744e1 | 结构性稀疏（<30，Le Cam 硬墙） |

**acc-multilevel-sample 判定**：level3+ 全历史 n 分布——
→ **level3+ 均 <30（Le Cam 硬墙成立）**：高级别结构性稀疏，全历史长窗也无足够样本做 OOS 验（有效域：高级别 alpha 可存在但不可功效检验）。

## acc-highlevel-mu：level2+ 逐级别 train-only holdout μ̂/LCB/p
train_frac=0.6，切分日 2022-11-27。每个 (level,δ) 独立评估（不从 train-only 挑类——无跨级别选择偏差）。
LCB=μ−1.645·se（单侧5%，se 用 neff），block bootstrap p（H0:μ≤0，block=20，B=2000，固定种子）。

| level | δ | train_n | train_Σpnl | holdout_n | holdout_Σpnl | μ̂_OOS | se | LCB | neff/nraw | drop_d5 | bootstrap_p | LCB>0? |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 4 | +1 | 0 | 0.0000e0 | 1 | -3.8737e2 | -3.8737e2 | inf | **-inf** | 1.00/1 | -0.0000e0 | 1.000 | ✗ |
  胜率=0/1 (0%)
| 5 | -1 | 1 | -4.0744e1 | 1 | -9.9277e0 | -9.9277e0 | inf | **-inf** | 1.00/1 | -0.0000e0 | 1.000 | ✗ |
  胜率=0/1 (0%)

**acc-highlevel-mu 判定**：
- level2+ holdout LCB>0 占比 = 0/2 (0%)
→ **否证（level2+ 全部 LCB≤0）**：高级别 holdout μ̂ 扣除不确定性后无一正 ⟹ 高级别无稳健 OOS alpha（否定性结果，缩小有效域边界，161/formalization-validity-domain）。
→ 否定性结果价值：高级别 alpha 不存在于 BTC 单标的 L2，仅低级别候选（level0/1 待 L3 验）。

## 结果包六要素
1. **结论**：
   - acc-multilevel-sample：level0-5 全历史信号数分布见上表。level3+ 全部 <30（Le Cam 硬墙，结构性稀疏）。
   - acc-highlevel-mu：level2+ holdout LCB>0 占比=0/2（0%）。全否证，高级别无稳健 OOS alpha（L2）。
2. **定义依据**：actual_pnl=δ(Pτout−Pτin)−Ce（664-Q3 真实成交口径）；LCB=μ−1.645se（PDF §7.3，se 用 neff_autocorr 修正）；train-only 每级独立评估（无跨级选择偏差）；Le Cam 硬墙判定（n<30）。
3. **边界条件**：单标的 BTC L2——结论翻转条件：(a) max_bars=5000000（若<461万则为截断窗结论）；(b) train_frac=0.6 改变切分点结论可能变化；(c) 高级别 n 过少（Le Cam 约束）时 LCB 由 se 主导而非 μ；(d) 不同 bootstrap 种子理论上可影响 p（本实现固定 LCG 种子确定性）。
4. **下游推论**：若 level2+ 全否证 ⟹ 策略信号层仅可依赖 level0/1（需 L3 跨品种升基座）。若存在 LCB>0 ⟹ 该 (level,δ) 可作 entry 候选（仍需 L3）。否定性结果意味着高级别配额应为 0 或纯结构性（不依赖 alpha 预期）。
5. **谱系引用**：663 econpositive（可交易性=μ(z,a)>0）；664-Q3（真实成交口径）；formalization-validity-domain（L2 有效域<定义域，L3 待 L2 完成后）；testing-override 生成态例外（测试目的=产出可证伪结果，非通过）；161（否定性结果照实报）。不确定是否有 level2+ alpha 的独立谱系记录，保守声明无关联已知谱系。
6. **影响声明**：新增 acc_multilevel_highlevel_mu 测试；复用 decompose_capturable_spread/class_actual_pnl/neff_autocorr/mean_se_lcb/drop_top_winners/block_bootstrap_pvalue；不改任何生产代码/TradeRecord/Order/信号收集逻辑。报告落盘 econ-multilevel-mu-20260701.md。
