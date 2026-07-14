# 缠论多级别 Rust 回测引擎严格验证方法论 — 深度研究报告 v2

- **运行**: workflow `wf_17f87a97-c03`（deep-research harness），106 agents / 654 tool calls / 3,137,256 subagent tokens
- **方法**: 问题分解为 5 个检索角度（primary/preregistration、academic/risk-normalized、hierarchical/power、practitioner/crypto-leverage、formal/quantifier-scope）→ 抓取 24 个来源 → 抽取 106 条 claim → 对 top 25 做三人对抗式验证（≥2/3 否证即杀）→ 23 确认、2 否证、0 未验证
- **日期**: 2026-07-06
- **前作**: 无（前一工位因模型区域限制零产出；本文件即首份落盘报告，归档路径 `.chanlun/review-results/deep-research-validation-methodology-20260704.md`）

## 总览

业界最佳实践以 **Bailey & López de Prado 的 Deflated Sharpe Ratio（DSR）** 与 **Harvey & Liu 的 haircut Sharpe** 为锚：评估目标必须在跑数前冻结为带哈希的不可变对象；完整试验次数 N 是回测报告中最关键的缺失信息——未控制搜索广度的回测"无论报告业绩多优秀都是无价值的"。风险归一化仓位（q∝1/σ、q∝1/d）的**有效域严格小于定义域**；混桶估计会触发 Simpson 式符号反转，且条件反转率在特定数据配置下可超 20%。低功效裁决应采用 **Three-Sided Testing（TST）** 四态互斥框架，INCONCLUSIVE 明确等于"数据不足、需补样"。

**覆盖声明**：领域 (1)–(4) 由同行评审一手来源充分覆盖（多数 3-0 投票）；领域 (5)(6)(7) 本轮检索**未被同行评审来源直接覆盖**，无新增可引用结论，需按文末方向补充检索。

---

## 领域 1：策略对象冻结 / 评估目标规格版本化

**存活结论（3-0）**

1. **冻结 + 哈希 + 版本化**：Keystone 框架在 OOS 运行前冻结并哈希 `ExperimentConfig`，任何参数变更创建新实验；`TrialRegistry` 累计整个研究项目的每次试验（非仅当前运行），DSR 据此惩罚完整搜索历史；物理日期锁定的 holdout 仅在 IS 参数选择后触及一次且永不复查。
   注意：Keystone 是个人开源项目而非同行评审框架，"enforces" 宜读作 "documents as a control"。
2. **子集外推的形式论证**：零假设（真实 SR=0）下 N 次独立试验的期望最大 Sharpe 经极值理论公式机械膨胀：
   `E[max{ŜR}] = E[ŜR] + √Var[ŜR]·((1−γ)·Φ⁻¹(1−1/N) + γ·Φ⁻¹(1−1/(N·e)))`，γ≈0.5772。
   López de Prado & Lewis (2018) 用层次聚类推导有效试验数 E[K]∈[1,N]。
3. **重要限定（来自否证桶）**：上述校正的铁证部分针对**正结论**的选择偏差。"负结论也必须有完整试验日志才有效"的强形式被 0-3 否证——familywise false-positive 校正验证的是正结论；负 alpha 子集结论不可外推至 Π_full 的论证要走**功效不足/覆盖不全**路线（子集未测 ⇒ 无证据），而非多重检验校正路线。相关 claim 一条 2-1 分裂存活，置信度中等。

**可操作建议**

- 为每次评估目标（裁决桶定义、样本过滤器、绩效度量）生成内容哈希并写入试验注册表；任何变更 = 新实验 ID，禁止原地修改。
- 维护跨运行的全局 TrialRegistry（含被放弃的试验），DSR/haircut 按累计 N 计算。
- 对 Π_tested⊊Π_full 的负结论，报告措辞限定为"在已测子集上未证实"，并附子集覆盖率与功效声明；不得写成对 Π_full 的否证。

**来源**：github.com/pancakes9798/Keystone；Bailey & López de Prado 2014, *The Deflated Sharpe Ratio*, JPM 40(5):94-107, DOI 10.3905/jpm.2014.40.5.094；López de Prado & Lewis 2018（codemacher.com PDF 镜像）

---

## 领域 2：风险归一化期望 E[X/d|state] 与 Simpson 式符号反转

**存活结论（3-0）**

1. **有效域严格小于定义域**：波动率缩放（f^{σ}_{t+1}=(c/σ²_t)·f_{t+1}）在风险资产（股票/信用/60-40/风险平价）上改善 Sharpe（市场因子年化 alpha≈4.9%、appraisal ratio=0.33、Sharpe +≈25%），但对债券/外汇/商品可忽略。机制 = 杠杆效应（收益-波动率变化负相关），等价于引入短期动量叠加（R²=45–60%）。
   **OOS 警告**：Cederburg et al. 2020 JFE——103 例中 72 例样本外劣于未管理组合；本报告引用的 Moreira & Muir 数字为 in-sample。Xu 2024 指出方差缩放系数存在 look-ahead bias。
2. **机制同构**：条件方差高度可预测但不预测未来收益 ⇒ 高波动期均值-方差权衡弱化 ⇒ w*_t∝E_t[f_{t+1}]/σ²_t。这与止损距离感知仓位 q∝1/d 同构——正是 E[X/d|state] 的形式。
3. **符号反转形式定义**：beta_1·gamma_1<0（简单回归斜率 vs 含交互项全模型系数）；完整 Simpson 悖论是更强条件 |gamma_1|>|gamma_3| 且 beta_1·gamma_1<0。（HDSR 2025, MIT Press, DOI 10.1162/99608f92.47359565）
4. **边际独立变量也能翻转符号**：与响应及所有其他预测变量均边际独立的 A_2，仍可经强交互效应 + 非平衡分布使 A_1 斜率反号。⇒ 即便按状态分桶看似正交，混合时仍可能翻转。
5. **反转发生率作为稳健性检查**：WMW 评分下无条件反转率仅 0–1.74%，但**条件于特定初始序列可超 20%**（2×7 案例最大 ~22.14%）。反转发生率可当作类 p 值的稳健性指标：发生反转 ⇒ 结果对聚合选择不稳健。（Boudreau et al. 2023, Frontiers）

**可操作建议**

- E[X/d|state] 必须按 state 分桶输出，主裁决禁止用混桶（pooled）估计；对每个主裁决桶跑一次"反转检查"：比较分桶符号与混桶符号，反号即挂 INCONCLUSIVE 并溯源交互项。
- q∝1/d 仓位规则先限定资产域（BTC 高波动风险资产在有效域内，但需本地验证），并在预注册中声明有效域假设。
- 力度/背驰特征进入主裁决桶时，检查其与止损距离 d 的交互项系数 gamma_3——|gamma_3| 接近 |gamma_1| 即为反转高危区。

**来源**：Moreira & Muir 2017, *Volatility-Managed Portfolios*, JF, DOI 10.1111/jofi.12513；Harvey et al. 2018 / Man Group, man.com/insights/the-impact-of-volatility-targeting；HDSR 2025, hdsr.mitpress.mit.edu/pub/bywbvnfy；Boudreau et al. 2023, Frontiers Appl. Math. Stat., 10.3389/fams.2023.1169164

---

## 领域 3：预注册纪律 — 冻结裁决桶、多重检验校正、LCB vs p 值

**存活结论**

1. **平价 haircut 是严重错误（3-0）**：正确 haircut 非线性、须作为 N 的解析函数计算（Harvey & Liu 2015, SSRN 2345489）。Šidák：p_M=1−(1−p_S)^N；p_S=0.05、N=10 ⇒ p_M=0.401。实例：N=200、T=240、Sharpe 0.75（p_S=0.0008）→ haircut Sharpe 0.32（p_M=0.15），≈60% haircut。强策略（SR>1.0）haircut≤25%，弱策略（SR<0.4）haircut>50%——一刀切 50% 双向皆错。
2. **DSR 作为可预注册的可证伪决策规则（2-1 分裂 + 3-0）**：拒绝 H0:SR=0 于 5% 显著性 ⇔ DSR>0.95，阈值可在跑数前冻结。DSR 纳入 N、T、偏度、峰度、跨试验方差。技术注记：DSR 是 PSR 经选择偏差校正的变换 CDF，"预注册工具"定位为 2-1 分裂投票——DSR 本质是事后多重检验校正，可冻结的是其**判据阈值**，不是免除事后校正。

**可操作建议**

- 跑数前冻结：裁决桶定义、SESOI、判据（DSR≥0.95 或 LCB>0）、试验预算 N_max；写入带哈希的预注册文件。
- 逐笔直接输出裁决键（trade → bucket key 在引擎内完成），禁止事后离线合并/重分桶——这正是本项目审计发现的"预注册逐笔裁决被离线聚合补丁替代"缺口的直接补救。
- 每次运行向 TrialRegistry 追加 (config_hash, N_effective, 结果摘要)；最终报告必须披露累计 N 并给出 haircut 后判据。

**来源**：Harvey & Liu 2015, *Backtesting*, SSRN abstract_id=2345489；Bailey & López de Prado 2014, DOI 10.3905/jpm.2014.40.5.094；López de Prado & Lewis 2018

---

## 领域 4：低功效场景的分层/嵌套设计与三态裁决

**存活结论（3-0）**

1. **Three-Sided Testing（TST）**：同时检验劣效（效应<Δ_L）/等效（Δ_L≤效应≤Δ_U）/优效（效应>Δ_U）三个互斥假设，相对预注册 SESOI 产出四态互斥裁决：**实践显著负 / 等效于零 / 实践显著正 / INCONCLUSIVE**。INCONCLUSIVE 明确意味着数据不足（CI 与 SESOI 边界相交），标准补救为补样（复制与合并）。
2. **TST 是 TOST 的一致改进**：等效成分与 TOST 完全相同（全水平 α、同功效、无惩罚）；Bonferroni α/2 仅施于新增的方向性检验。采用 TST 无成本、只增益。

**可操作建议**

- 将现行 VALIDATED/FALSIFIED/INCONCLUSIVE 三态升级为 TST 四态：VALIDATED(+) / EQUIVALENT-TO-ZERO / FALSIFIED(−) / INCONCLUSIVE——"等效于零"与"数据不足"是不同结论，当前三态把二者混在 INCONCLUSIVE/FALSIFIED 里会导致误判补救方向。
- 高级别信号作方向/体制过滤、低级别入场供样本量的嵌套设计：对每个高级别桶预注册 SESOI 与最小有效样本量功效门；门未过 ⇒ 自动 INCONCLUSIVE，禁止降级为 FALSIFIED（这正是"高级别信号桶样本饥饿"的正确出口）。
- INCONCLUSIVE 的补救动作写死为"补样/延长窗口/合并复制"，不允许改判据。

**来源**：Fitzgerald (Isager & Fitzgerald), *Three-Sided Testing to Establish Practical Significance: A Tutorial*, Tinbergen Institute DP 24-077, papers.tinbergen.nl/24077.pdf（OSF: 10.31234/osf.io/8y925）；Goeman, Solari & Stijnen 2010, Statistics in Medicine, DOI 10.1002/sim.4002

---

## 领域 5：杠杆加密期货的保证金/强平/ADL/资金费建模

**本轮检索未被同行评审来源直接覆盖——无存活结论。**

抓取到的相关来源（nautilus_trader PR #4077、arxiv 2512.01112、Crypto-Perps-Backtest-Engine、cv5capital basis-trade insight）产出的 claim 未进入验证前 25 或未存活。

**补充检索方向**（预注册为下一轮 deep-research 的角度）：
- BitMEX/Perp 协议级白皮书（强平引擎、ADL 队列、资金费公式的一手规格）
- 交易所事后爆仓复盘（2020-03-12、2021-05-19 类事件的强平级联分析）
- 关键问题：无保证金净额结算假设下哪些结论仍有效？哪些 alpha 会在引入逐 bar 维持保证金/强平/资金费后被否证？

---

## 领域 6：多腿多空子账本核算与 overlay 陷阱

**本轮检索未被同行评审来源直接覆盖——无存活结论。**

**补充检索方向**：
- LTCM 等多腿对冲事后复盘中的子账本归因失败模式
- "只做诊断不驱动订单"的 overlay 经信号污染或仓位漂移间接驱动订单的路径（overlay-not-driving-orders 复盘，如 Argo/Scientific Beta 文献）
- 声部级 ledger 的基差收益 vs 短差收益归因的会计一致性检验

---

## 领域 7：序贯门控吸收态 bug 与量词作用域错误

**本轮检索未被同行评审来源直接覆盖——无存活结论。**

已抓取但未产出存活 claim 的相关一手来源（可作下轮种子）：Springer s00236-025-00504-z、arxiv 2011.03567、TCAD 2015 (Erb et al., Stuttgart)、Riener IVSW16 counterexample-guided diagnosis、arxiv 2209.00991。

**补充检索方向**：
- Model checking 中 AG/EG 路径算子的反例优先搜索与覆盖度测试模式——"单个早期反例永久锁死全局量词门"正是 AG 语义误用为吸收态的形态
- 量词作用域错误（∀/∃ 交换、作用域提升）的检测/变异测试文献
- 工程侧最小测试：对每个序贯门写"反例后恢复"用例——注入一个早期反例，断言门在后续窗口可重新打开（除非规格明确要求吸收态）

---

## 被否证的结论（方法论警示）

| 被杀 claim | 投票 | 教训 |
|---|---|---|
| 波动率缩放对美股 1927–2017 Sharpe 0.40→0.48–0.51、t=3.05 的"统计显著改善" | 1-2 | in-sample 数字被当作可交易结论；Cederburg 2020 OOS 证据相反 |
| "负结论也必须有完整试验日志（E[K]）才有效" | 0-3 | familywise 校正只管正结论；负结论不可外推要走功效/覆盖论证，不能借多重检验校正之名 |

## 总体 caveats

- 两条存活结论为 2-1 分裂投票（DSR 预注册定位、EVT 公式的负结论外推子句），置信度低于 3-0 项。
- 波动率管理 alpha 的样本外可交易性存在争议；引用数字为 in-sample。
- Keystone 为个人开源项目，其控制措施是"文档化的控制"而非经审计的强制。

## 开放问题

1. 无保证金净额结算假设下哪些结论仍有效？（→ 领域 5 补检索）
2. 只做诊断的 overlay 经哪些路径间接驱动订单？（→ 领域 6 补检索）
3. 吸收态门 bug 在 model checking 文献中的标准覆盖度测试模式？（→ 领域 7 补检索）
4. 嵌套设计中 SESOI 与功效门阈值如何用本项目 BTC 权威基线实测标定？（→ 本地实验，非检索）

## 来源清单（按质量）

**一手/同行评审**：DOI 10.3905/jpm.2014.40.5.094（Bailey & López de Prado 2014）· SSRN 2345489（Harvey & Liu）· DOI 10.1111/jofi.12513（Moreira & Muir 2017）· hdsr.mitpress.mit.edu/pub/bywbvnfy（HDSR 2025）· 10.3389/fams.2023.1169164（Boudreau 2023）· papers.tinbergen.nl/24077.pdf · DOI 10.1002/sim.4002（Goeman et al. 2010）· López de Prado & Lewis 2018（codemacher.com 镜像）· man.com/insights/the-impact-of-volatility-targeting

**二手/工程**：github.com/pancakes9798/Keystone · github.com/Aliipou/backtest-audit · github.com/nautech-systems/nautilus_trader/pull/4077 · arxiv 2512.01112 · github.com/metinmertyesilyurt/Crypto-Perps-Backtest-Engine

**下轮种子（领域 7）**：Springer 10.1007/s00236-025-00504-z · arxiv 2011.03567 · TCAD 2015 Erb et al. · Riener IVSW16 · arxiv 2209.00991
