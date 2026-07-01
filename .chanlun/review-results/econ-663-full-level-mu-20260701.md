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
- **截断窗 [2017-08-17→2026-05-31]，bars=4613599**（最后 5000000 bar；全量 461万 OOM 不可行 ⟹ 截断窗=显式有效域边界，非全窗结论，l3 同纪律）
- untradable_ratio=0.0052
- 收集信号数 n_signals=7742（无配对出场反转信号的入场信号被诚实跳过，不入此集——无 ρ_rev 不兜底）

## 三审计统计（codex #97 要求）
| 统计 | 值 | 说明 |
|---|---|---|
| n_rho_after_lambda | 7742/7742 | rho_rev_bar>lambda_rev_bar（codex Q1 不变量：出场 pivot 端点在入场 pivot 之后） |
| n_same_bar_opposite | 37 | 同 bar 出现反向信号（被 eb>entry_bar 排除的边界，codex Q2） |
| n_unpaired | 3 | 无配对出场反转信号（右删失诚实跳过，codex Q4） |

## P7 正规出场口径统计（接 sell.rs CloseRoot/ReduceCore，n=7742）
| 出场决策 | 信号数 | 占比 | 说明 |
|---|---|---|---|
| CloseRoot（第一类顶背驰） | 0 | 0.0% | sell.rs SellDecision::CloseRoot |
| ReduceCore（第三类减核） | 7742 | 100.0% | sell.rs SellDecision::ReduceCore |
| Type2Missing（第二类 still-MISSING） | 0 | 0.0% | sell.rs:35 诚实边界，不改账本口径 |
| Hold（无正规卖点 bit） | 0 | 0.0% | exit 信号无正规卖侧/买侧 bit |
（认识论 L0：口径结构变，不声明 alpha；alpha 待 W-VERIFY L2/L3。第二类闭环 still-MISSING 见 sell.rs:35 诚实边界，不碰 TW 三阶段/576。）

## 全局归因（双口径）
| 量 | 值 |
|---|---|
| ΣAb_rev（反转交易腿理想总价差） | 1.588863e6 |
| Σx（signed 入场滑移） | 5.663648e5 |
| Σy（signed 出场滑移） | 5.259839e5 |
| Σmax(0,x)=Σηin（adverse-only 入场） | 8.631650e5 (47.2%) |
| Σmax(0,y)=Σηout（adverse-only 出场） | 7.854587e5 (43.0%) |
| ΣCe（成本） | 1.798366e5 (9.8%) |
| Σactual_spread（真实成交价差 δ(Pτout−Pτin)） | 4.965146e5 |
| **Σcaptured（adverse-only 压力测试剩余）** | -2.395969e5 |
| **actual_pnl_proxy=Σactual_spread−ΣCe（真实成交 PnL 代理）** | 3.166781e5 |
| n_captured_positive/n（adverse-only 正占比） | 2591/7742 (33.5%) |
| n_actual_positive/n（真实成交正占比） | 2844/7742 (36.7%) |

## L2 判定（关键：两口径分歧 = codex Q3 核心）
- **adverse-only spread_eaten = true**（Σcaptured ≤ 0）
- **真实成交 actual_pnl_eaten = false**（actual_pnl_proxy > 0）
- **翻案（codex Q3 坐实）**：adverse-only 判「执行吃光」但真实成交 actual_pnl_proxy>0 ⟹ 之前「执行滞后吃光」(Σcaptured=−2.33e5) 是 **adverse-only 伪结论**——有利滑移被 max(0,·) 丢弃所致。真实成交口径下反转腿可捕获。

## 失血三源对比（ΣAb_rev 为反转腿结构上限）
- 反转腿价差：ΣAb_rev=1.588863e6（664 号测对了对象——可正可负，非触发段同义反复）
- 执行吃光：Ση=1.648624e6（入场+出场滞后）
- 成本：ΣCe=1.798366e5

**ΣAb_rev>0（反转交易腿）：** 664 号修复后反转腿理想价差为正 ⟹ 结构给了可捕获价差（与旧触发段 ΣAb=−1.8e4
形成对照——后者是测错对象的伪否证）。剩余 alpha = ΣAb_rev − Ση − ΣCe（执行/成本是否吃光见上表 Σcaptured）。

## per-class (level, δ, bsp_class) 三键分桶（W4 P4：消除 buy1/2/3 混合池稀释）
μ̂(z,a)=Σactual_pnl/n = 逐信号正条件期望估计（663 判据：>0 即可交易，不需统计显著/不判稀疏硬墙）。
bsp_class=u8 位掩码（bit0=buy1,bit1=buy2,bit2=buy3,bit3=sell1,bit4=sell2,bit5=sell3）。
| level | δ | bsp_class | n | μ̂=Σactual_pnl/n | Σactual_pnl | ΣAb_rev | Ση(adv) | ΣCe | Σcaptured(adv) | n_act+/n |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | -1 | 0x20 | 3648 | 2.1152e1 | 7.7163e4 | 6.5199e5 | 7.6772e5 | 8.2274e4 | -1.9800e5 | 1307/3648 (36%) |
| 0 | +1 | 0x04 | 4093 | 5.8528e1 | 2.3956e5 | 9.3690e5 | 8.8090e5 | 9.7560e4 | -4.1555e4 | 1537/4093 (38%) |
| 5 | -1 | 0x10 | 1 | -4.0744e1 | -4.0744e1 | -3.0320e1 | 8.2600e0 | 2.1644e0 | -4.0744e1 | 0/1 (0%) |

## 663 判据：全级别×方向×类型 μ̂(z,a)>0（正条件期望，出现就做不统计显著）
涌现最高级别 L=5（全级别 0..5 均列；稀疏高级别照报不判硬墙，663）。
**有效域**：本窗 bars=4613599（2017-08-17→2026-05-31）。663 要求全历史长窗——
若 bars<461万，高级别 L3+ 仍稀疏（n=个位数），累积净值是**本窗**结论非全历史（ECON_L2_MAX_BARS=5000000 跑全量，~6-7min）。
μ̂>0 类 = 该 (level,δ,bsp_class) 逐信号正条件期望——出现即做累积正期望（非 p<0.05 统计显著）：
- (level=0, δ=-1, cls=0x20) ✓μ̂>0: μ̂=2.1152e1, Σ=7.7163e4, n_act+/n=1307/3648
- (level=0, δ=+1, cls=0x04) ✓μ̂>0: μ̂=5.8528e1, Σ=2.3956e5, n_act+/n=1537/4093
- (level=5, δ=-1, cls=0x10) ✗μ̂≤0: μ̂=-4.0744e1, Σ=-4.0744e1, n_act+/n=0/1
**2/3 类 μ̂>0**（663 判据：正期望类可交易，不因稀疏判 inconclusive）。

## 全历史累积净值曲线（长窗，按 entry_bar 时间序）
- **全级别累积净值终值 = 3.1668e5**（7742 信号，min 累积=-3.6130e3 曲线最低点）
| level | 该级别累积净值 | 正期望? |
|---|---|---|
| 0 | 3.1672e5 | ✓ |
| 5 | -4.0744e1 | ✗ |

累积净值曲线采样（10 等分点，全级别）：
- [774/7742] entry_bar=453254 累积=7.4779e4
- [1548/7742] entry_bar=950140 累积=1.0214e5
- [2322/7742] entry_bar=1406375 累积=1.4189e5
- [3096/7742] entry_bar=1860286 累积=2.2533e5
- [3870/7742] entry_bar=2305605 累积=1.6721e5
- [4644/7742] entry_bar=2785853 累积=1.6974e5
- [5418/7742] entry_bar=3278541 累积=1.9546e5
- [6192/7742] entry_bar=3721518 累积=2.0850e5
- [6966/7742] entry_bar=4155029 累积=3.9242e5
- [7740/7742] entry_bar=4611609 累积=3.1710e5
- [7742/7742] entry_bar=4612554 累积=3.1668e5
## 666 号 σ_higher 分布（上级方向态 vs δ）
- 顺上级（δ==σ_higher）：5537/7742 (71.5%)
- 逆上级（δ==−σ_higher）：2202/7742 (28.4%)
- 无上级（σ_higher==0）：3/7742 (0.0%)

