# 经济正条件⑤ L2 重测：BTC signed 滑移分解（adverse-only vs 真实成交，664-Q3 codex 口径修正）

**认识论等级**：L2（真实数据单标的逐信号确定性分解，可产否定性结果）。
**664-Q3 修正**：旧 captured=Ab_rev−max(0,x)−max(0,y)−Ce 用 max(0,·) **丢有利滑移、全计不利滑移** ⟹ 系统性低估真实 PnL（captured−actual_pnl=min(x,0)+min(y,0)≤0）= **adverse-only 保守压力测试，非真实成交**。本报告补 signed Σx/Σy 与真实成交价差 actual_spread=δ(Pτout−Pτin)=Ab_rev−x−y。
**两口径**：x=δ(Pτin−Pλ_rev)（signed 入场滑移）、y=δ(Pρ_rev−Pτout)（signed 出场滑移）。
- **adverse-only captured** = Ab_rev−max(0,x)−max(0,y)−Ce（压力测试，保守，保留兼容旧口径）。
- **真实成交 actual_pnl** = actual_spread−Ce = δ(Pτout−Pτin)−Ce（含全部滑移，更接近真实成交）。
**复算**：`cargo test -p <crate> --release l2_btc_capturable_spread_diagnosis -- --ignored --nocapture`（确定性）。

## 数据
- 品种：BTC（btc_1m_full.json，全量 4613599 bar，2017-08→2026-05）
- **截断窗 [2025-08-27→2026-05-31]，bars=400320**（最后 400000 bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）
- untradable_ratio=0.0000
- 收集信号数 n_signals=1110（无配对出场反转信号的入场信号被诚实跳过，不入此集——无 ρ_rev 不兜底）

## 三审计统计（codex #97 要求）
| 统计 | 值 | 说明 |
|---|---|---|
| n_rho_after_lambda | 1108/1110 | rho_rev_bar>lambda_rev_bar（codex Q1 不变量：出场 pivot 端点在入场 pivot 之后） |
| n_same_bar_opposite | 34 | 同 bar 出现反向信号（被 eb>entry_bar 排除的边界，codex Q2） |
| n_unpaired | 4 | 无配对出场反转信号（右删失诚实跳过，codex Q4） |

## 全局归因（双口径）
| 量 | 值 |
|---|---|
| ΣAb_rev（反转交易腿理想总价差） | 2.204726e5 |
| Σx（signed 入场滑移） | 1.200523e5 |
| Σy（signed 出场滑移） | 1.017315e5 |
| Σmax(0,x)=Σηin（adverse-only 入场） | 2.151211e5 (48.2%) |
| Σmax(0,y)=Σηout（adverse-only 出场） | 1.715922e5 (38.5%) |
| ΣCe（成本） | 5.928827e4 (13.3%) |
| Σactual_spread（真实成交价差 δ(Pτout−Pτin)） | -1.311200e3 |
| **Σcaptured（adverse-only 压力测试剩余）** | -2.255290e5 |
| **actual_pnl_proxy=Σactual_spread−ΣCe（真实成交 PnL 代理）** | -6.059947e4 |
| n_captured_positive/n（adverse-only 正占比） | 309/1110 (27.8%) |
| n_actual_positive/n（真实成交正占比） | 364/1110 (32.8%) |

## L2 判定（关键：两口径分歧 = codex Q3 核心）
- **adverse-only spread_eaten = true**（Σcaptured ≤ 0）
- **真实成交 actual_pnl_eaten = true**（actual_pnl_proxy ≤ 0）
- **「执行吃光」成立（真实成交口径）**：actual_pnl_proxy≤0 ⟹ 即便不丢有利滑移，真实成交价差减成本仍非正 ⟹ 该信号集真实亏（否定性结果，缩小有效域边界，161/formalization-validity-domain）。

## 失血三源对比（ΣAb_rev 为反转腿结构上限）
- 反转腿价差：ΣAb_rev=2.204726e5（664 号测对了对象——可正可负，非触发段同义反复）
- 执行吃光：Ση=3.867133e5（入场+出场滞后）
- 成本：ΣCe=5.928827e4

**ΣAb_rev>0（反转交易腿）：** 664 号修复后反转腿理想价差为正 ⟹ 结构给了可捕获价差（与旧触发段 ΣAb=−1.8e4
形成对照——后者是测错对象的伪否证）。剩余 alpha = ΣAb_rev − Ση − ΣCe（执行/成本是否吃光见上表 Σcaptured）。

## per-class (level, δ) 分桶（双口径）
| level | δ | n | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | actual_pnl | n_cap+/n | n_act+/n |
|---|---|---|---|---|---|---|---|---|---|
| 0 | -1 | 464 | 1.9835e5 | 1.7367e5 | 2.4572e4 | 1.0733e2 | 7.0862e4 | 137/464 (30%) | 161/464 (35%) |
| 0 | +1 | 503 | 1.7822e4 | 1.6690e5 | 2.7141e4 | -1.7622e5 | -1.0753e5 | 134/503 (27%) | 157/503 (31%) |
| 1 | -1 | 46 | 2.4145e4 | 1.7874e4 | 2.4756e3 | 3.7957e3 | 1.1498e4 | 14/46 (30%) | 15/46 (33%) |
| 1 | +1 | 62 | -2.3847e4 | 1.8543e4 | 3.3211e3 | -4.5710e4 | -3.3694e4 | 13/62 (21%) | 16/62 (26%) |
| 2 | -1 | 12 | 4.0136e3 | 4.5897e3 | 6.1274e2 | -1.1889e3 | 5.4464e2 | 6/12 (50%) | 6/12 (50%) |
| 2 | +1 | 15 | 3.3584e3 | 2.7416e3 | 7.7971e2 | -1.6297e2 | 2.8879e3 | 5/15 (33%) | 8/15 (53%) |
| 3 | -1 | 2 | -7.0925e2 | 1.6498e2 | 1.1714e2 | -9.9137e2 | -4.7681e2 | 0/2 (0%) | 1/2 (50%) |
| 3 | +1 | 3 | -2.2390e3 | 1.4724e3 | 1.2754e2 | -3.8389e3 | -3.6218e3 | 0/3 (0%) | 0/3 (0%) |
| 4 | -1 | 2 | -9.5367e2 | 4.6350e1 | 9.2887e1 | -1.0929e3 | -8.4608e2 | 0/2 (0%) | 0/2 (0%) |
| 5 | +1 | 1 | 5.3284e2 | 7.1204e2 | 4.8673e1 | -2.2787e2 | -2.2787e2 | 0/1 (0%) | 0/1 (0%) |

## 663 原生口径（真实成交 actual_pnl>0 状态类）
actual_pnl>0 的 (level,δ) 类 = 该类真实成交逐信号路径级净正（出现即做，真实成交口径，非 adverse-only）：
- (level=0, δ=-1): actual_pnl=7.0862e4, n_act+/n=161/464
- (level=1, δ=-1): actual_pnl=1.1498e4, n_act+/n=15/46
- (level=2, δ=-1): actual_pnl=5.4464e2, n_act+/n=6/12
- (level=2, δ=+1): actual_pnl=2.8879e3, n_act+/n=8/15
