# W-VERIFY acc-alpha 定版跑批（L2 BTC，锚 d906648041 段2完整实装）

- 工位 ws-wverify | 任务#3 | 锚 SHA d906648041（Cand^δ 段1+段2 完整实装语义）
- 数据 BTC OOS 窗，**截 32000 bar**（build_walk_forward_mu 为 O(n²)，全窗 461万 计算不可行——L3 MAX_BARS 有效域边界，非全窗结论）
- 认识论 **L2**（单标的 BTC，单 32k 窗）；有效域受限（231号）

## 结论：verdict=Pass（∃ VALIDATED 桶）
trades=74，n_sig=74，5 桶有样本，levels=[0,1,2]。V=1 F=1 I=3。

| ℓ | bsp_class | δ | n | μ̂ | LCB | UCB | CV | perm_p | state |
|---|---|---|---|---|---|---|---|---|---|
| L0 | 3 | -1 | 22 | -169.10 | -227.98 | -110.22 | 0.993 | 1.000 | Falsified |
| L0 | 3 | +1 | 44 | 242.27 | 50.65 | 433.89 | 3.189 | 0.000 | **Validated** |
| L1 | 2 | -1 | 2 | -211.84 | -548.72 | 125.03 | 1.367 | 0.765 | Inconclusive |
| L1 | 2 | +1 | 3 | 14.56 | -65.48 | 94.59 | 5.789 | 0.340 | Inconclusive |
| L2 | 2 | +1 | 3 | 42.33 | -19.61 | 104.27 | 1.541 | 1.000 | Inconclusive |

## 结果包六要素
1. **结论**：L0 第三类买点(δ+1) 在此有效域内检出超 beta alpha——n=44、μ̂=+242 tick、LCB=+50.6>0（powered）、perm_p=0.000（δ 方向携真实信息，非纯 beta）。L0 第三类卖点(δ-1) FALSIFIED（μ̂=-169，powered 负=纯 beta 亏损腿）。L1/L2 数据稀疏 INCONCLUSIVE。
2. **定义依据**：prereg §1.4 路径A分层内δ置换 + §3.1 三态 + §4 冻结常量(z_α=1.645,N_PERM=200,种子20260701,perm_α=0.05,功效门 n_eff≥(1.645·CV)²)。bsp_class 由 MuClass::bsp_class() 6-bit→{1,2,3} 折叠。
3. **边界条件**：Pass 由**单个桶**(L0三买)支撑，n=44、单 32k 窗、单标的。翻转：跨窗/跨标的(L3)重估若该桶 perm_p 升破 0.05 或 LCB≤0 则退 INCONCLUSIVE；扩窗(O(n²)约束下需算法优化#14)改变样本分布。
4. **下游推论**：L0 三买是唯一候选可交易 alpha；但 L2 单窗 **不构成稳健 alpha 证据**——必须 L3 跨标的交叉验证才可用于选择器。三卖腿确认纯 beta（勿做空 L0 三卖）。
5. **谱系引用**：663/奇偶交替μ恒等/665/666/667；231(有效域≠定义域)；H1 #7(段2已修 level1-4 门 bug——本轮 L1/L2 出现即证解封)；segment2 定位增强(#13)已含。
6. **影响声明**：与 level0 基线(全INCONCLUSIVE)相比，段2完整实装后 L0 三买转 VALIDATED——**但根因是 build_walk_forward_mu 的 OOS-split walk-forward 口径 + 段2 锚定，不是纯 μ 均值**。既有 alpha 否证(奇偶交替/σ_higher/663)的有效域仍未被本轮 L2 单窗结论推翻——本轮是 L2 单窗内的 within-sample 信号，非跨维度确证。

## 231 有效域声明（强制）
- 有效域 = **单标的 BTC × 单 32k bar OOS 窗 × L2**。**非**全定义域：O(n²) 引擎禁 461万全窗；L1-4 桶因 32k 窗样本稀疏(n≤3)仍 INCONCLUSIVE（这次是**数据稀疏**非门 bug——段2 已解封，L1/L2 已出现在样本中）。
- global_verdict=Pass **只对此有效域成立**。不得外推"全级别/全窗有 alpha"。
- 稳健性升级路径：L3 跨标的(≥3)池化 + 算法优化(#14)解 O(n²) 后扩窗覆盖全 OOS。
