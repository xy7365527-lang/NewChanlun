# 663 判据：BTC 全级别×方向逐信号 μ̂>0 + 全历史长窗累积净值

**认识论等级**：L2（真实数据单标的逐信号确定性分解，可产否定性结果）。
**663 判据**：可交易性=μ(z,a)>0（正条件期望，出现就做不统计显著），**非** p<0.05 统计显著/跨品种符号检验。全级别 0..L 照报（级别是缠论全互斥定义构成部分，稀疏高级别不剔不判 Le Cam 硬墙——663 收窄 Le Cam 有效域至短窗单品种区分±Δ）。全历史长窗累积净值（O(n²) 已解锁 exp1.18，461万 bar ~2.6min）。
**664-Q3 修正**：旧 captured=Ab_rev−max(0,x)−max(0,y)−Ce 用 max(0,·) **丢有利滑移、全计不利滑移** ⟹ 系统性低估真实 PnL（captured−actual_pnl=min(x,0)+min(y,0)≤0）= **adverse-only 保守压力测试，非真实成交**。本报告补 signed Σx/Σy 与真实成交价差 actual_spread=δ(Pτout−Pτin)=Ab_rev−x−y。
**两口径**：x=δ(Pτin−Pλ_rev)（signed 入场滑移）、y=δ(Pρ_rev−Pτout)（signed 出场滑移）。
- **adverse-only captured** = Ab_rev−max(0,x)−max(0,y)−Ce（压力测试，保守，保留兼容旧口径）。
- **真实成交 actual_pnl** = actual_spread−Ce = δ(Pτout−Pτin)−Ce（含全部滑移，更接近真实成交）。
**复算**：`cargo test -p <crate> --release l2_btc_capturable_spread_diagnosis -- --ignored --nocapture`（确定性）。

## 数据
- 品种：BTC（btc_1m_full.json，全量 4613599 bar，2017-08→2026-05）
- **截断窗 [2025-11-04→2026-05-31]，bars=300960**（最后 300000 bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）
- untradable_ratio=0.0000
- 收集信号数 n_signals=720（无配对出场反转信号的入场信号被诚实跳过，不入此集——无 ρ_rev 不兜底）

## 三审计统计（codex #97 要求）
| 统计 | 值 | 说明 |
|---|---|---|
| n_rho_after_lambda | 720/720 | rho_rev_bar>lambda_rev_bar（codex Q1 不变量：出场 pivot 端点在入场 pivot 之后） |
| n_same_bar_opposite | 3 | 同 bar 出现反向信号（被 eb>entry_bar 排除的边界，codex Q2） |
| n_unpaired | 4 | 无配对出场反转信号（右删失诚实跳过，codex Q4） |

## P7 正规出场口径统计（接 sell.rs CloseRoot/ReduceCore，n=720）
| 出场决策 | 信号数 | 占比 | 说明 |
|---|---|---|---|
| CloseRoot（第一类顶背驰） | 0 | 0.0% | sell.rs SellDecision::CloseRoot |
| ReduceCore（第三类减核） | 719 | 99.9% | sell.rs SellDecision::ReduceCore |
| Type2Missing（第二类 still-MISSING） | 1 | 0.1% | sell.rs:35 诚实边界，不改账本口径 |
| Hold（无正规卖点 bit） | 0 | 0.0% | exit 信号无正规卖侧/买侧 bit |
（认识论 L0：口径结构变，不声明 alpha；alpha 待 W-VERIFY L2/L3。第二类闭环 still-MISSING 见 sell.rs:35 诚实边界，不碰 TW 三阶段/576。）

## 全局归因（双口径）
| 量 | 值 |
|---|---|
| ΣAb_rev（反转交易腿理想总价差） | 1.161503e5 |
| Σx（signed 入场滑移） | 7.044393e4 |
| Σy（signed 出场滑移） | 6.208958e4 |
| Σmax(0,x)=Σηin（adverse-only 入场） | 1.329613e5 (48.4%) |
| Σmax(0,y)=Σηout（adverse-only 出场） | 1.069181e5 (38.9%) |
| ΣCe（成本） | 3.478079e4 (12.7%) |
| Σactual_spread（真实成交价差 δ(Pτout−Pτin)） | -1.638323e4 |
| **Σcaptured（adverse-only 压力测试剩余）** | -1.585099e5 |
| **actual_pnl_proxy=Σactual_spread−ΣCe（真实成交 PnL 代理）** | -5.116402e4 |
| n_captured_positive/n（adverse-only 正占比） | 208/720 (28.9%) |
| n_actual_positive/n（真实成交正占比） | 245/720 (34.0%) |

## L2 判定（关键：两口径分歧 = codex Q3 核心）
- **adverse-only spread_eaten = true**（Σcaptured ≤ 0）
- **真实成交 actual_pnl_eaten = true**（actual_pnl_proxy ≤ 0）
- **「执行吃光」成立（真实成交口径）**：actual_pnl_proxy≤0 ⟹ 即便不丢有利滑移，真实成交价差减成本仍非正 ⟹ 该信号集真实亏（否定性结果，缩小有效域边界，161/formalization-validity-domain）。

## 失血三源对比（ΣAb_rev 为反转腿结构上限）
- 反转腿价差：ΣAb_rev=1.161503e5（664 号测对了对象——可正可负，非触发段同义反复）
- 执行吃光：Ση=2.398794e5（入场+出场滞后）
- 成本：ΣCe=3.478079e4

**ΣAb_rev>0（反转交易腿）：** 664 号修复后反转腿理想价差为正 ⟹ 结构给了可捕获价差（与旧触发段 ΣAb=−1.8e4
形成对照——后者是测错对象的伪否证）。剩余 alpha = ΣAb_rev − Ση − ΣCe（执行/成本是否吃光见上表 Σcaptured）。

## per-class (level, δ, bsp_class) 三键分桶（W4 P4：消除 buy1/2/3 混合池稀释）
μ̂(z,a)=Σactual_pnl/n = 逐信号正条件期望估计（663 判据：>0 即可交易，不需统计显著/不判稀疏硬墙）。
bsp_class=u8 位掩码（bit0=buy1,bit1=buy2,bit2=buy3,bit3=sell1,bit4=sell2,bit5=sell3）。
| level | δ | bsp_class | n | μ̂=Σactual_pnl/n | Σactual_pnl | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | n_act+/n |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | -1 | 0x20 | 349 | 1.7074e2 | 5.9587e4 | 1.4365e5 | 1.2158e5 | 1.6818e4 | 5.2500e3 | 128/349 (37%) |
| 0 | +1 | 0x04 | 359 | -3.0980e2 | -1.1122e5 | -3.1124e4 | 1.1418e5 | 1.7330e4 | -1.6264e5 | 111/359 (31%) |
| 1 | -1 | 0x10 | 8 | 3.1632e1 | 2.5305e2 | 2.5530e3 | 2.4013e3 | 4.0828e2 | -2.5658e2 | 4/8 (50%) |
| 1 | +1 | 0x02 | 3 | -1.5617e2 | -4.6852e2 | -1.1653e2 | 1.2649e3 | 1.6715e2 | -1.5486e3 | 1/3 (33%) |
| 2 | -1 | 0x10 | 1 | 6.8140e2 | 6.8140e2 | 1.1926e3 | 4.5402e2 | 5.7204e1 | 6.8140e2 | 1/1 (100%) |

## 663 判据：全级别×方向×类型 μ̂(z,a)>0（正条件期望，出现就做不统计显著）
涌现最高级别 L=2（全级别 0..2 均列；稀疏高级别照报不判硬墙，663）。
**有效域**：本窗 bars=300960（2025-11-04→2026-05-31）。663 要求全历史长窗——
若 bars<461万，高级别 L3+ 仍稀疏（n=个位数），累积净值是**本窗**结论非全历史（ECON_L2_MAX_BARS=5000000 跑全量，~6-7min）。
μ̂>0 类 = 该 (level,δ,bsp_class) 逐信号正条件期望——出现即做累积正期望（非 p<0.05 统计显著）：
- (level=0, δ=-1, cls=0x20) ✓μ̂>0: μ̂=1.7074e2, Σ=5.9587e4, n_act+/n=128/349
- (level=0, δ=+1, cls=0x04) ✗μ̂≤0: μ̂=-3.0980e2, Σ=-1.1122e5, n_act+/n=111/359
- (level=1, δ=-1, cls=0x10) ✓μ̂>0: μ̂=3.1632e1, Σ=2.5305e2, n_act+/n=4/8
- (level=1, δ=+1, cls=0x02) ✗μ̂≤0: μ̂=-1.5617e2, Σ=-4.6852e2, n_act+/n=1/3
- (level=2, δ=-1, cls=0x10) ✓μ̂>0: μ̂=6.8140e2, Σ=6.8140e2, n_act+/n=1/1
**3/5 类 μ̂>0**（663 判据：正期望类可交易，不因稀疏判 inconclusive）。

## 全历史累积净值曲线（长窗，按 entry_bar 时间序）
- **全级别累积净值终值 = -5.1164e4**（720 信号，min 累积=-9.2507e4 曲线最低点）
| level | 该级别累积净值 | 正期望? |
|---|---|---|
| 0 | -5.1630e4 | ✗ |
| 1 | -2.1546e2 | ✗ |
| 2 | 6.8140e2 | ✓ |

累积净值曲线采样（10 等分点，全级别）：
- [72/720] entry_bar=30135 累积=-2.5643e4
- [144/720] entry_bar=62480 累积=-6.7356e4
- [216/720] entry_bar=90015 累积=-8.0102e4
- [288/720] entry_bar=120168 累积=-8.1273e4
- [360/720] entry_bar=155155 累积=-1.7030e4
- [432/720] entry_bar=184134 累积=-9.9017e3
- [504/720] entry_bar=215781 累积=-3.8341e4
- [576/720] entry_bar=243826 累积=-6.5377e4
- [648/720] entry_bar=270195 累积=-6.4432e4
- [720/720] entry_bar=299915 累积=-5.1164e4
## 666 号 σ_higher 分布（上级方向态 vs δ）
- 顺上级（δ==σ_higher）：477/720 (66.2%)
- 逆上级（δ==−σ_higher）：240/720 (33.3%)
- 无上级（σ_higher==0）：3/720 (0.4%)

