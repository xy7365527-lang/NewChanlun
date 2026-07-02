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
- **截断窗 [2025-08-27→2026-05-31]，bars=400320**（最后 400000 bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）
- untradable_ratio=0.0000
- 收集信号数 n_signals=985（无配对出场反转信号的入场信号被诚实跳过，不入此集——无 ρ_rev 不兜底）

## 三审计统计（codex #97 要求）
| 统计 | 值 | 说明 |
|---|---|---|
| n_rho_after_lambda | 985/985 | rho_rev_bar>lambda_rev_bar（codex Q1 不变量：出场 pivot 端点在入场 pivot 之后） |
| n_same_bar_opposite | 5 | 同 bar 出现反向信号（被 eb>entry_bar 排除的边界，codex Q2） |
| n_unpaired | 4 | 无配对出场反转信号（右删失诚实跳过，codex Q4） |

## P7 正规出场口径统计（接 sell.rs CloseRoot/ReduceCore，n=985）
| 出场决策 | 信号数 | 占比 | 说明 |
|---|---|---|---|
| CloseRoot（第一类顶背驰） | 0 | 0.0% | sell.rs SellDecision::CloseRoot |
| ReduceCore（第三类减核） | 978 | 99.3% | sell.rs SellDecision::ReduceCore |
| Type2Missing（第二类 still-MISSING） | 7 | 0.7% | sell.rs:35 诚实边界，不改账本口径 |
| Hold（无正规卖点 bit） | 0 | 0.0% | exit 信号无正规卖侧/买侧 bit |
（认识论 L0：口径结构变，不声明 alpha；alpha 待 W-VERIFY L2/L3。第二类闭环 still-MISSING 见 sell.rs:35 诚实边界，不碰 TW 三阶段/576。）

## 全局归因（双口径）
| 量 | 值 |
|---|---|
| ΣAb_rev（反转交易腿理想总价差） | 2.541008e5 |
| Σx（signed 入场滑移） | 1.107439e5 |
| Σy（signed 出场滑移） | 9.369023e4 |
| Σmax(0,x)=Σηin（adverse-only 入场） | 1.929116e5 (47.2%) |
| Σmax(0,y)=Σηout（adverse-only 出场） | 1.626916e5 (39.8%) |
| ΣCe（成本） | 5.272879e4 (12.9%) |
| Σactual_spread（真实成交价差 δ(Pτout−Pτin)） | 4.966671e4 |
| **Σcaptured（adverse-only 压力测试剩余）** | -1.542311e5 |
| **actual_pnl_proxy=Σactual_spread−ΣCe（真实成交 PnL 代理）** | -3.062081e3 |
| n_captured_positive/n（adverse-only 正占比） | 285/985 (28.9%) |
| n_actual_positive/n（真实成交正占比） | 332/985 (33.7%) |

## L2 判定（关键：两口径分歧 = codex Q3 核心）
- **adverse-only spread_eaten = true**（Σcaptured ≤ 0）
- **真实成交 actual_pnl_eaten = true**（actual_pnl_proxy ≤ 0）
- **「执行吃光」成立（真实成交口径）**：actual_pnl_proxy≤0 ⟹ 即便不丢有利滑移，真实成交价差减成本仍非正 ⟹ 该信号集真实亏（否定性结果，缩小有效域边界，161/formalization-validity-domain）。

## 失血三源对比（ΣAb_rev 为反转腿结构上限）
- 反转腿价差：ΣAb_rev=2.541008e5（664 号测对了对象——可正可负，非触发段同义反复）
- 执行吃光：Ση=3.556032e5（入场+出场滞后）
- 成本：ΣCe=5.272879e4

**ΣAb_rev>0（反转交易腿）：** 664 号修复后反转腿理想价差为正 ⟹ 结构给了可捕获价差（与旧触发段 ΣAb=−1.8e4
形成对照——后者是测错对象的伪否证）。剩余 alpha = ΣAb_rev − Ση − ΣCe（执行/成本是否吃光见上表 Σcaptured）。

## per-class (level, δ, bsp_class) 三键分桶（W4 P4：消除 buy1/2/3 混合池稀释）
μ̂(z,a)=Σactual_pnl/n = 逐信号正条件期望估计（663 判据：>0 即可交易，不需统计显著/不判稀疏硬墙）。
bsp_class=u8 位掩码（bit0=buy1,bit1=buy2,bit2=buy3,bit3=sell1,bit4=sell2,bit5=sell3）。
| level | δ | bsp_class | n | μ̂=Σactual_pnl/n | Σactual_pnl | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | n_act+/n |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | -1 | 0x20 | 464 | 2.6328e2 | 1.2216e5 | 2.4226e5 | 1.7624e5 | 2.4556e4 | 4.1469e4 | 166/464 (36%) |
| 0 | +1 | 0x04 | 503 | -2.4648e2 | -1.2398e5 | 8.9767e3 | 1.7427e5 | 2.7136e4 | -1.9243e5 | 159/503 (32%) |
| 1 | -1 | 0x10 | 11 | -1.7279e2 | -1.9007e3 | 8.3615e2 | 2.7749e3 | 6.0641e2 | -2.5451e3 | 4/11 (36%) |
| 1 | +1 | 0x02 | 6 | -4.1360e0 | -2.4816e1 | 8.3312e2 | 1.8692e3 | 3.7259e2 | -1.4087e3 | 2/6 (33%) |
| 2 | -1 | 0x10 | 1 | 6.8140e2 | 6.8140e2 | 1.1926e3 | 4.5402e2 | 5.7204e1 | 6.8140e2 | 1/1 (100%) |

## 663 判据：全级别×方向×类型 μ̂(z,a)>0（正条件期望，出现就做不统计显著）
涌现最高级别 L=2（全级别 0..2 均列；稀疏高级别照报不判硬墙，663）。
**有效域**：本窗 bars=400320（2025-08-27→2026-05-31）。663 要求全历史长窗——
若 bars<461万，高级别 L3+ 仍稀疏（n=个位数），累积净值是**本窗**结论非全历史（ECON_L2_MAX_BARS=5000000 跑全量，~6-7min）。
μ̂>0 类 = 该 (level,δ,bsp_class) 逐信号正条件期望——出现即做累积正期望（非 p<0.05 统计显著）：
- (level=0, δ=-1, cls=0x20) ✓μ̂>0: μ̂=2.6328e2, Σ=1.2216e5, n_act+/n=166/464
- (level=0, δ=+1, cls=0x04) ✗μ̂≤0: μ̂=-2.4648e2, Σ=-1.2398e5, n_act+/n=159/503
- (level=1, δ=-1, cls=0x10) ✗μ̂≤0: μ̂=-1.7279e2, Σ=-1.9007e3, n_act+/n=4/11
- (level=1, δ=+1, cls=0x02) ✗μ̂≤0: μ̂=-4.1360e0, Σ=-2.4816e1, n_act+/n=2/6
- (level=2, δ=-1, cls=0x10) ✓μ̂>0: μ̂=6.8140e2, Σ=6.8140e2, n_act+/n=1/1
**2/5 类 μ̂>0**（663 判据：正期望类可交易，不因稀疏判 inconclusive）。

## 全历史累积净值曲线（长窗，按 entry_bar 时间序）
- **全级别累积净值终值 = -3.0621e3**（985 信号，min 累积=-4.4406e4 曲线最低点）
| level | 该级别累积净值 | 正期望? |
|---|---|---|
| 0 | -1.8180e3 | ✗ |
| 1 | -1.9255e3 | ✗ |
| 2 | 6.8140e2 | ✓ |

累积净值曲线采样（10 等分点，全级别）：
- [98/985] entry_bar=39696 累积=-9.4961e3
- [196/985] entry_bar=76594 累积=6.3074e4
- [294/985] entry_bar=112559 累积=3.7322e4
- [392/985] entry_bar=153330 累积=7.0570e3
- [490/985] entry_bar=192037 累积=-2.5542e4
- [588/985] entry_bar=236735 累积=2.8777e4
- [686/985] entry_bar=279463 累积=3.8961e4
- [784/985] entry_bar=320753 累积=7.9965e3
- [882/985] entry_bar=358255 累积=-2.0555e4
- [980/985] entry_bar=397437 累积=-2.5601e3
- [985/985] entry_bar=399275 累积=-3.0621e3
## 666 号 σ_higher 分布（上级方向态 vs δ）
- 顺上级（δ==σ_higher）：649/985 (65.9%)
- 逆上级（δ==−σ_higher）：334/985 (33.9%)
- 无上级（σ_higher==0）：2/985 (0.2%)

