# 形式化链 B 组（买卖点+alpha 8 份）× wverify-alpha-retest 对照审计

**工位**：swarm/ws-dlB | task #71 | 纯只读，零 git
**对照基线**：`wverify-alpha-retest-20260702.md`（PASS 终局）+ `acc-alpha-estimand-prereg-20260701.md`（预注册冻结 v0）
**源**：`docs/formal-chain/`（唯一权威）8 份逐页全读
**认识论**：本报告 L0（文档对照，零 L2 信息增量）；对照对象的实跑是 L2 单标的 BTC

---

## 0. 覆盖声明（8/8 全读）

| PDF | 页 | 内页标题/性质 | alpha 相关核心 |
|---|---|---|---|
| 买卖点.pdf | 15 | 推导完全分类（结构覆盖版语法/状态机） | §7 覆盖净收益 G_e>0 端点定向恒真 |
| 买卖点2.pdf | 14 | 推导完全分类（事件状态机版） | §18 择时 alpha 判定 E[N^bsp ΔP−C]>0 |
| 买卖点alpha.pdf | 17 | 推导完全分类（π^cov vs π^bsp 分离） | 定理3 前视禁止；b1 否证 π^cov 非 π^bsp |
| 买卖点alpha2.pdf | 38 | 推导完全分类（**本组权威 alpha 判定链**） | §12 μ_{ℓ,δ,I}>0；§10-11 ΔN 增量；§14/§16 严格/鞅定理 |
| alpha.pdf | 33 | Eat(e) 覆盖定理 + |E|=O(n) 复杂度 | 全 L0/结构，soundness 非 performance |
| 严格alpha.pdf | 27 | V*=E[max(0,max_a μ)]>0 + 全定义管线 | LCB 门控 χ_t、no-trade a_0、鞅不可能 |
| **alpha检验.pdf** | 20 | **统计判定四定理（检验流程原文）** | **n_eff/功效/三态/block bootstrap/删尾/方向不对称** |
| **alpha分离.pdf** | 18 | **alpha/beta 分离（去污流程原文）** | **残差 Y_i/分层δ置换/跨标的/σ_higher/确认滞后** |

**关键定位**：`alpha检验.pdf` + `alpha分离.pdf` 才是 wverify retest 应实装的**原始检验/去污规格**。retest 的估计量、三态判据、分层置换、n_eff、功效门都可逐条溯源到这两份——这是本次对照的重心。

---

## 1. 结论（PASS 是否满足原文严格标准）

**⚠ 最高优先级：retest 的 PASS 未满足 `alpha检验.pdf`/`alpha分离.pdf` 定义的 "confirmed structural alpha" 标准。**

retest 主桶 = **L0 / type3 / δ+1(买) / σ^H=0**（μ̂=+262.49, perm_p=0.000, LCB=+183.86, 全5窗正），全局裁决 PASS 唯一挂此桶。按原文严格标准，此桶**尚未达 confirmed，介于 inconclusive 与 confirmed 之间**——四条硬缺口：

1. **未做 beta 残差减法（去污流程核心违背）**。`alpha分离.pdf §1/§4.1` **强制**："所有后续 alpha 检验**只看残差** `Y_i=δ_i(H_i−B̂_i)−C_i`，不是 `X_i`"，`B̂_i`=持有窗市场漂移积分。retest 估计量 `actual_pnl=δ·(P_out−P_in)−C` **就是原始 `X_i`，未减 `B̂_i`**。retest 依赖"分层内 δ 置换吸收 beta"（665 路径 A），但原文 §4.2 的置换检验本身是在**残差** `T^res=∑δ_i(H_i−B̂_i)−∑C_i` 上做的——原文是"残差减法 + 残差上分层置换"双重，retest 只有后半。

2. **分层置换的分层维不全 → beta 吸收不完整**。`alpha分离.pdf §4.2` 分层键 = **`(ℓ, h bucket, time block, σ_higher)`**。retest 分层键 = `(ℓ, bsp_class, σ^H)`——**缺 h(持有期桶) + 缺 time block**（retest §4 已诚实标注 h 未接通=#51 G-A2 缺口）。分层缺 h/time block ⟹ 同分层内 `B̂_i` 不同质 ⟹ 若买点系统性落在高 `B̂` 段（上涨段），置换 δ 会让"δ 与 B̂ 相关"冒充结构 alpha，**beta 从分层漏进 perm_p**。故 retest `perm_p=0.000` **不能排除 BTC 上涨 beta**。

3. **主桶是买方向 + 单标的，正撞原文两处明确警告**。
   - `alpha分离.pdf §9`："BTC 全历史强上涨，**做多方向正收益不能自动算 alpha**……只在 BTC 上正，可能是 BTC 特定 beta。" retest 主桶恰是**做多（δ+1）**。
   - `alpha检验.pdf §7-§8` 实测：level0 **买腿两窗巨亏**（跌窗 −57,200 / 涨窗 −50,900），**卖腿为正**。原文预注册重心是 **level0 卖**（H1:μ_{0,sell}>0, H2:μ_sell−μ_buy>0）。retest 靠 `bsp_class+σ^H` 细分从"整体巨亏的买腿"里挑出一个正子桶——符合 §13 细分类 V(Z)≥V(Y)，但正踩 §2 反复警告的 selection bias。
   - 原文两份均强制**跨标的 L3**（BTC,ETH,ES,NQ,CL,GC,DX,BRN + 层级模型 μ_{asset,z}=μ_z+u_{asset,z}）才算 confirmed structural alpha。retest 是 L2 单标的（报告已诚实标注"M1前必须L3复验"）。

4. **未做删尾稳健性 + 方向不对称检验（原文明确要求，retest 缺失）**。`alpha检验.pdf §6`："删除前 3 个最大赢家后转负"=尾部依赖诊断。retest 主桶 **CV=7.278 极高**（高波动/疑似尾部主导），却无删尾检验。`§7-§8/Step1 H2` 要求 `μ_sell−μ_buy>0` 回归（block bootstrap/cluster SE），retest 逐桶独立未做。

**公平陈述**：在 retest **自声明的有效域内**（L2 单标的 + 四元组桶 + walk-forward 5窗），PASS 是自洽的，且报告已诚实标注全部边界（单桶单标的、不可外推、M1前L3复验）。**但 retest 的"PASS"= L2 条件性 PASS ≠ 原文 `alpha检验.pdf` 的 "confirmed structural alpha"**。后者要求：残差 Y_i + 完整分层置换 + 跨标的 + 删尾稳健 + 方向不对称，四者 retest 均未满足。原文对本类结果的精确标签是 **"positive point estimate, statistically inconclusive / beta 未分离"**，非 confirmed。

---

## 2. 定义依据（原文 alpha 定义 vs 实跑口径）

### 2.1 已对齐项（retest 正确溯源原文）

| 维度 | 原文规定 | retest 实跑 | 判定 |
|---|---|---|---|
| 估计量语义 | `X_γ=δ(P_τ−P_t)−C`，τ=出场证书时刻（买卖点alpha2 §12） | `actual_pnl=δ·(P_out−P_in)−C` | ✓ 择时语义（非 π^cov 覆盖净额），对齐 |
| 三态判据 | LCB>0⟹confirmed；UCB<ε_econ⟹rejected；else inconclusive（alpha检验 §3/§6） | VALIDATED/FALSIFIED/INCONCLUSIVE | ✓ 三态对齐（差异见 2.2） |
| 功效门槛 | d_sig=1.645/√n_eff ⟺ n_eff>(1.645·CV)²（alpha检验 §4-§5） | powered⟺n_eff≥(1.645·CV)² | ✓ **完全对齐**（retest 667 门槛=原文 d_sig 反解） |
| n_eff 口径 | n_eff=n_raw/(1+2∑ρ_k) 自相关校正（alpha检验 §4） | decontam::effective_n 自相关校正 | ✓ 方法一致 |
| 分层 δ 置换 | 分层内只 shuffle δ 标签，判据 T_obs>Q_0.95(T_π)（alpha分离 §4.2） | 200次 stratified_delta_perm_p | ✓ 方法对齐（分层维不全，见 1.2） |
| 显著阈值 | p_perm<0.05；单侧 z=1.645（alpha检验 §3/alpha分离 Step3） | perm_p<0.05；z_α=1.645 | ✓ 对齐 |
| 预注册 | Step1 看结果前冻结假设（alpha检验 §13） | acc-alpha-estimand-prereg 冻结 v0 | ✓ 对齐（重心不同，见 2.2） |

### 2.2 差异项（口径不一致）

| 维度 | 原文规定 | retest 实跑 | 缺口性质 |
|---|---|---|---|
| **桶键第3维** | I_γ⊆{1,2,3} = **同时触发类别集合**（7种非空子集 {1},{2},{3},{1,2},{1,3},{2,3},{1,2,3}）（买卖点alpha2 §12/严格alpha §3） | bsp_class **单值**∈{1,2,3} | I_γ 集合语义被压成单值，**重合类别信息丢失** |
| **rejected 阈值** | UCB<**ε_econ**（经济意义最小门槛，可能>0）（alpha检验 §6） | FALSIFIED 用 UCB≤**0** | retest 更保守（不轻判无 alpha）；但 ε_econ 未引入 |
| **beta 去污** | 残差 Y_i=δ(H−B̂)−C，只看 Y_i（alpha分离 §1/§4.1） | 原始 X_i + 分层置换吸收 | **B̂_i 减法缺失**（见 1.1） |
| **置换分层维** | (ℓ, h bucket, time block, σ_higher)（alpha分离 §4.2） | (ℓ, bsp_class, σ^H) | **缺 h + 缺 time block**（见 1.2） |
| **认证范围** | 跨标的 8 品种 + 层级模型（两份均强制 L3） | L2 单标的 BTC | retest 诚实标注为边界；原文要求 L3 才 confirmed |
| **稳健性诊断** | 删尾（去前3赢家）+ block bootstrap（alpha检验 §6/§11） | 无删尾；perm_p 代 block bootstrap | 删尾缺失；block bootstrap 未做（perm 非等价） |
| **方向不对称** | H2:μ_sell−μ_buy>0 回归 + cluster SE（alpha检验 §7-§8） | 逐桶独立，无 δ 对比 | 缺失（原文认为"更有希望"的假设） |
| **σ^H 语义** | σ_higher=上级方向（"第一优先级"新变量，alpha分离 §c3） | σ^H=parent_dir 父声部方向 | ✓ 语义对齐（retest R2 已标残余差） |

---

## 3. 缺口清单（原文有而实装无的检验步骤）

按优先级（阻塞 confirmed 判定的在前）：

1. **[阻塞] beta 残差减法 B̂_i**：实装 `Y_i=δ_i(H_i−B̂_i)−C_i`，`B̂_i=∑_{s=t_i}^{τ_i−1}ĝ_s`，`ĝ_s` 只用 train/过去信息估计（滚动均值/因子模型）。所有 alpha 检验改看 Y_i。（alpha分离 §1/§4.1）
2. **[阻塞] 置换分层补 h + time block**：分层键补齐 `(ℓ, h bucket, time block, σ_higher)`，否则 beta 从分层漏进 perm_p。（alpha分离 §4.2；与 retest #51 G-A2 h 缺口同源）
3. **[阻塞] 跨标的 L3**：8 品种 + 层级模型 `μ_{asset,z}=μ_z+u_{asset,z}`，"μ_z 跨资产为正"才算结构 alpha。（两份均强制）
4. **[高] 删尾稳健性**：去前 3/前 k 最大赢家后重估符号（尾部依赖诊断）。主桶 CV=7.278 尤需。（alpha检验 §6）
5. **[高] 方向不对称检验**：回归 `X_i=α+βD_i+γR_i+η(D_iR_i)+ε_i`，检验 β=μ_sell−μ_buy>0，用 block bootstrap/cluster-robust SE。（alpha检验 §7-§8/§13 Step1 H2）
6. **[中] block bootstrap 推断**：全信号策略推断用 block bootstrap（非 iid SE，非仅 perm），原文实测 level0 卖 p=0.371。（alpha检验 §11）
7. **[中] I_γ 集合桶键**：桶键第3维保留同时触发类别集合（7子集），而非单值 bsp_class，或显式声明边际化对齐。（买卖点alpha2 §12）
8. **[中] 确认滞后分解**：`X=A^ideal−η^in−η^out−C`，分离"结构无 edge" vs "edge 被确认滞后/出场口径吃掉"；配 7 条因果出场规则反事实。（alpha分离 §6-§7）
9. **[中] LCB 门控选择器**：交易选择器用 `χ_t=1[LCB_t(μ)>θ]` 而非裸 `μ̂>θ`（防高维过拟合）。retest 已用 LCB 判据，但生产选择器是否 LCB 门控待确认。（严格alpha §12）
10. **[低] UCB<ε_econ 判 rejected**：引入经济门槛 ε_econ，UCB<ε_econ 才判 confirmed negative。（alpha检验 §6）
11. **[低] 多窗口 walk-forward 窗层推断**：rolling walk-forward 后对**窗口**做 block/cluster inference（非拼接后单一 n_eff）——retest §8 已诚实标注拼接口径的"日历相邻冒充统计相邻"残余。（alpha检验 §13 Step2）

---

## 4. 前视 / 因果可测性（原文明确规定 vs retest 状态）

原文（两份+严格alpha）反复强制 **q_t/N_t ∈ F_t**、无未来函数/无端点后视/无重绘。具体：
- **端点定向前视**（核心）：`ε_e=sign(P_{ρ_e}−P_{λ_e})`，若 ρ_e 是未来完成端点则 ε_e 非 F_{λ_e}-可测——"腿不是用未来终点方向伪造的"。（alpha.pdf §5；买卖点alpha 定理3）
- **出场规则**：`r_i∈F_{t_i}`，**"不能用未来 pivot 最优出场"**；出场价 = `P_{τ(a)}`，τ=下一反向证书或风险事件触发时刻（事件驱动，非固定 horizon）。（alpha分离 §7；严格alpha §7）
- **训练封闭**：所有 B̂/μ̂/σ_higher 规则/阈值必须 train 内确定，OOS 只评估。（alpha分离 Step7）

**retest 状态**：`actual_pnl=δ·(P_out−P_in)−C` 的 `P_out`（exit）如何确定、是否 F_{t_i}-可测（无前视），retest 报告**未声明**（§4 仅说 trades() 不携带 entry/exit bar 差值）。**缺口**：需独立确认 exit 价用的是因果出场规则（下一反向证书触发时刻的价），非未来确认的笔/线段端点。项目另有"零前视回测"工位，应交叉核对。

---

## 5. 下游推论

- **acceptance a4=PASS 的效力域应收窄**：从"存在可交易 alpha"降为"L2 单标的 BTC 上、原始收益 X_i（未减 beta）、分层不全的 δ 置换下，L0 买子桶点估计为正且 perm 显著"——**未达原文 confirmed structural alpha**。M1 里程碑前，缺口 1-3（残差减法/分层补全/跨标的）为**硬前置**，非"M1后补"。
- **主桶买方向 + BTC 上涨 beta 未分离**是本次最实质风险：原文实测同级别买腿整体巨亏，retest 正结论靠细分挑桶获得，必须用残差 Y_i + 跨标的排除"这是 BTC 做多 beta 的细分投影"。
- retest §7 (R2) 的"多条件化 bsp_class 是 PDF estimand 细化"论证方向正确，但**与原文 I_γ 集合语义相反**（retest 压单值，原文保集合）——需在边际化声明中对齐。

---

## 6. 谱系引用

- 663（判据=μ̂>0 非显著性）/665（单标的 beta 退化 + 分层置换吸收 beta）/666（δ 置换 perm_p）/667（LCB underpowered≠证伪，n_eff 门槛）——retest 消费，本对照确认门槛公式 = 原文 d_sig 反解。
- 231（形式化有效域）：retest PASS 的有效域（L2/四元组/单标的）< 原文 confirmed 定义域（残差/跨标的/删尾/不对称），**声明须收窄**（缺口 1-4 前不得称 confirmed structural alpha）。
- #51（σ^H 桶键 + G-A2 h 缺口）：本对照坐实 h 缺口不仅是分层维缺失，更导致 §1.2 的 beta 泄漏，**从"诚实缺口"升级为"阻塞 confirmed 的实质缺口"**。
- 曾发生判据错误领域（660/663）：本对照新增"未减 B̂ 残差 + 分层不全 ⟹ perm_p 显著不排除 beta"一例。

---

## 7. 影响声明

- **产出**：本报告（review-results 审计交付物，task #71 指定）。零代码改动，零 git。
- **影响的声明**：`wverify-alpha-retest-20260702.md` 的 PASS 结论——本对照指出其"confirmed alpha"效力**不成立于原文严格标准**，应降为"L2 条件性正结果，beta 未分离，L3 未做"。retest 报告自身的边界标注（单桶单标的/M1前L3复验）方向正确但**低估了缺口的阻塞性**（残差减法/分层补全被原文列为 alpha 检验的**前置**，非可选增强）。
- **不改**：任何引擎/estimand/prereg 文件（本工位纯审计）。缺口清单 §3 供 Lead 分派实装工位。

---

## 附：认识论等级标注（231/formalization-validity-domain）

- 本报告：**L0**（文档对照，零 L2 增量）。
- 结论"retest PASS 未达原文 confirmed 标准"：**L0 逻辑推论**（从原文规格 vs retest 报告字面对照推出，不依赖新回测）。
- 缺口 1-4 是否真的翻转主桶符号：**待 L2 验证**（补残差减法/删尾后重跑才知），本报告不预判翻转，只指出"当前证据不足以 confirmed"。
