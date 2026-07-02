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
- **截断窗 [2026-05-18→2026-05-31]，bars=20160**（最后 20000 bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）
- untradable_ratio=0.0000
- 收集信号数 n_signals=47（无配对出场反转信号的入场信号被诚实跳过，不入此集——无 ρ_rev 不兜底）

## 三审计统计（codex #97 要求）
| 统计 | 值 | 说明 |
|---|---|---|
| n_rho_after_lambda | 47/47 | rho_rev_bar>lambda_rev_bar（codex Q1 不变量：出场 pivot 端点在入场 pivot 之后） |
| n_same_bar_opposite | 1 | 同 bar 出现反向信号（被 eb>entry_bar 排除的边界，codex Q2） |
| n_unpaired | 4 | 无配对出场反转信号（右删失诚实跳过，codex Q4） |

## P7 正规出场口径统计（接 sell.rs CloseRoot/ReduceCore，n=47）
| 出场决策 | 信号数 | 占比 | 说明 |
|---|---|---|---|
| CloseRoot（第一类顶背驰） | 0 | 0.0% | sell.rs SellDecision::CloseRoot |
| ReduceCore（第三类减核） | 45 | 95.7% | sell.rs SellDecision::ReduceCore |
| Type2Missing（第二类 still-MISSING） | 2 | 4.3% | sell.rs:35 诚实边界，不改账本口径 |
| Hold（无正规卖点 bit） | 0 | 0.0% | exit 信号无正规卖侧/买侧 bit |
（认识论 L0：口径结构变，不声明 alpha；alpha 待 W-VERIFY L2/L3。第二类闭环 still-MISSING 见 sell.rs:35 诚实边界，不碰 TW 三阶段/576。）

## 全局归因（双口径）
| 量 | 值 |
|---|---|
| ΣAb_rev（反转交易腿理想总价差） | 2.337656e4 |
| Σx（signed 入场滑移） | 3.175030e3 |
| Σy（signed 出场滑移） | 6.194100e2 |
| Σmax(0,x)=Σηin（adverse-only 入场） | 5.401900e3 (52.9%) |
| Σmax(0,y)=Σηout（adverse-only 出场） | 2.674070e3 (26.2%) |
| ΣCe（成本） | 2.132396e3 (20.9%) |
| Σactual_spread（真实成交价差 δ(Pτout−Pτin)） | 1.958212e4 |
| **Σcaptured（adverse-only 压力测试剩余）** | 1.316819e4 |
| **actual_pnl_proxy=Σactual_spread−ΣCe（真实成交 PnL 代理）** | 1.744972e4 |
| n_captured_positive/n（adverse-only 正占比） | 17/47 (36.2%) |
| n_actual_positive/n（真实成交正占比） | 19/47 (40.4%) |

## L2 判定（关键：两口径分歧 = codex Q3 核心）
- **adverse-only spread_eaten = false**（Σcaptured > 0）
- **真实成交 actual_pnl_eaten = false**（actual_pnl_proxy > 0）
- **两口径一致正**：adverse-only 与真实成交均 >0 ⟹ 反转腿价差未被吃光（保守口径都过 ⟹ 真实更宽松）。仅 BTC 单标的 L2，非跨品种功效。

## 失血三源对比（ΣAb_rev 为反转腿结构上限）
- 反转腿价差：ΣAb_rev=2.337656e4（664 号测对了对象——可正可负，非触发段同义反复）
- 执行吃光：Ση=8.075970e3（入场+出场滞后）
- 成本：ΣCe=2.132396e3

**ΣAb_rev>0（反转交易腿）：** 664 号修复后反转腿理想价差为正 ⟹ 结构给了可捕获价差（与旧触发段 ΣAb=−1.8e4
形成对照——后者是测错对象的伪否证）。剩余 alpha = ΣAb_rev − Ση − ΣCe（执行/成本是否吃光见上表 Σcaptured）。

## per-class 完整状态 Z=(level, δ, I_γ, σ_p, 短差, 仓位态, H) 分桶（b2 #83：Y 粗投影→Z 细状态）
μ̂(z,a)=Σactual_pnl/n = 逐信号正条件期望估计（663 判据：>0 即可交易，不需统计显著/不判稀疏硬墙）。
I_γ=u8 位掩码（bit0=buy1,bit1=buy2,bit2=buy3,bit3=sell1,bit4=sell2,bit5=sell3，未压缩 6-bit=z.i_class）。
role=σ_p|短差(sw/tr)|仓位态(R/C)|H(F/SF/SR)——R(g)=(H,V,δ) 完整角色（codex #81 H 进 canonical Z）。
| level | δ | I_γ | role(σ_p\|sw\|pos\|H) | n | μ̂=Σactual_pnl/n | Σactual_pnl | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | n_act+/n |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | -1 | 0x20 | -1|tr|C|SR | 4 | 1.6341e3 | 6.5365e3 | 7.2910e3 | 6.9785e2 | 1.8254e2 | 6.4106e3 | 3/4 (75%) |
| 0 | -1 | 0x20 | +0|tr|R|F | 19 | 7.8020e2 | 1.4824e4 | 1.8088e4 | 4.0759e3 | 8.5547e2 | 1.3157e4 | 10/19 (53%) |
| 0 | -1 | 0x20 | +0|tr|R|SR | 1 | -7.1936e2 | -7.1936e2 | -5.9288e2 | 1.9840e2 | 4.6011e1 | -8.3729e2 | 0/1 (0%) |
| 0 | +1 | 0x04 | +0|tr|R|F | 20 | -1.7510e2 | -3.5020e3 | -1.9761e3 | 2.6438e3 | 9.1014e2 | -5.5300e3 | 4/20 (20%) |
| 0 | +1 | 0x04 | +1|tr|C|SR | 1 | 6.6596e2 | 6.6596e2 | 7.4755e2 | 1.5166e2 | 4.5774e1 | 5.5012e2 | 1/1 (100%) |
| 2 | -1 | 0x10 | +0|tr|R|SF | 1 | -8.9283e2 | -8.9283e2 | -5.3817e2 | 3.0839e2 | 4.6269e1 | -8.9283e2 | 0/1 (0%) |
| 2 | +1 | 0x02 | +0|tr|R|SR | 1 | 5.3769e2 | 5.3769e2 | 3.5691e2 | 0.0000e0 | 4.6190e1 | 3.1072e2 | 1/1 (100%) |

## 663 判据：全级别×方向×类型 μ̂(z,a)>0（正条件期望，出现就做不统计显著）
涌现最高级别 L=2（全级别 0..2 均列；稀疏高级别照报不判硬墙，663）。
**有效域**：本窗 bars=20160（2026-05-18→2026-05-31）。663 要求全历史长窗——
若 bars<461万，高级别 L3+ 仍稀疏（n=个位数），累积净值是**本窗**结论非全历史（ECON_L2_MAX_BARS=5000000 跑全量，~6-7min）。
μ̂>0 类 = 该完整状态 z 逐信号正条件期望——出现即做累积正期望（非 p<0.05 统计显著）：
- (level=0, δ=-1, I_γ=0x20, role=-1|tr|C|SR) ✓μ̂>0: μ̂=1.6341e3, Σ=6.5365e3, n_act+/n=3/4
- (level=0, δ=-1, I_γ=0x20, role=+0|tr|R|F) ✓μ̂>0: μ̂=7.8020e2, Σ=1.4824e4, n_act+/n=10/19
- (level=0, δ=-1, I_γ=0x20, role=+0|tr|R|SR) ✗μ̂≤0: μ̂=-7.1936e2, Σ=-7.1936e2, n_act+/n=0/1
- (level=0, δ=+1, I_γ=0x04, role=+0|tr|R|F) ✗μ̂≤0: μ̂=-1.7510e2, Σ=-3.5020e3, n_act+/n=4/20
- (level=0, δ=+1, I_γ=0x04, role=+1|tr|C|SR) ✓μ̂>0: μ̂=6.6596e2, Σ=6.6596e2, n_act+/n=1/1
- (level=2, δ=-1, I_γ=0x10, role=+0|tr|R|SF) ✗μ̂≤0: μ̂=-8.9283e2, Σ=-8.9283e2, n_act+/n=0/1
- (level=2, δ=+1, I_γ=0x02, role=+0|tr|R|SR) ✓μ̂>0: μ̂=5.3769e2, Σ=5.3769e2, n_act+/n=1/1
**4/7 类 μ̂>0**（663 判据：正期望类可交易，不因稀疏判 inconclusive）。

## 全历史累积净值曲线（长窗，按 entry_bar 时间序）
- **全级别累积净值终值 = 1.7450e4**（47 信号，min 累积=-7.5771e3 曲线最低点）
| level | 该级别累积净值 | 正期望? |
|---|---|---|
| 0 | 1.7805e4 | ✓ |
| 2 | -3.5514e2 | ✗ |

累积净值曲线采样（10 等分点，全级别）：
- [4/47] entry_bar=3047 累积=-7.3961e2
- [8/47] entry_bar=5294 累积=-3.1552e3
- [12/47] entry_bar=6842 累积=-1.7441e3
- [16/47] entry_bar=7921 累积=-4.9270e3
- [20/47] entry_bar=9434 累积=-5.2173e3
- [24/47] entry_bar=10869 累积=-7.2653e3
- [28/47] entry_bar=11875 累积=2.7456e3
- [32/47] entry_bar=12786 累积=1.3322e4
- [36/47] entry_bar=14952 累积=1.7120e4
- [40/47] entry_bar=16925 累积=1.7283e4
- [44/47] entry_bar=18170 累积=1.8186e4
- [47/47] entry_bar=19115 累积=1.7450e4
## 666 号 σ_higher 分布（上级方向态 vs δ）
- 顺上级（δ==σ_higher）：32/47 (68.1%)
- 逆上级（δ==−σ_higher）：12/47 (25.5%)
- 无上级（σ_higher==0）：3/47 (6.4%)

