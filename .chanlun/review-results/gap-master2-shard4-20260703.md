# gap-master-2 分片4：策略/资金/对冲族 PDF 对照（7份）

工位：ws-gap2s4 | 任务 #154（→ #156 汇编） | 日期：2026-07-03
方法：7 份 PDF 全部有 dlpdf 系列（任务 #70-#74，2026-07-02 逐页全读）历史对照凭据——本分片不重读 PDF，以 dlpdf 提取的核心主张为基底，对 0702 之后实装浪潮（#112-#149、margin v2、f3 回测、裁定4）后的**现码状态**逐条刷新分类。抽查验证锚点：mutex.rs、mu_estimator.rs、ledger.rs、divergence.rs、risk.rs、margin-model-design §5.1、f3-fugue-backtest。
（完整的策略.pdf 已在 gap-master2-20260703.md 逐 16 节对照，本分片按任务描述不重做，仅引用。）

## 覆盖声明

| PDF | 逐页全读凭据 | 本分片刷新方式 |
|---|---|---|
| 缠论的全互斥定义策略.pdf (27p) | dlpdf-a §2.1/2.2（权威全本①） | 对 mutex.rs/mu_estimator.rs 现码验证 |
| 缠论的全互斥定义策略2.pdf (37p) | dlpdf-a（权威全本②最全） | 同上 + W-VERIFY 管线凭据 |
| 多空对冲.pdf (16p) | dlpdf-e §8 | f3 回测实证 + margin-design 对照 |
| 绝对资本.pdf (12p) | dlpdf-e §9 | margin-design §5.1 显式引用核验 |
| 经济正条件.pdf (19p) | dlpdf-e §5 | econ_positive.rs + gap-master2 A8 引用 |
| k的条件.pdf (13p) | dlpdf-d（κ=GAP3 严格定位） | ledger.rs RiskPolicy 现码验证 |
| 关于背驰.pdf (14p) | dlpdf-d（力度支配序+四条件） | ForceStateA5 现码验证（A2 精确化） |

## 逐 PDF 对照表

### 1. 缠论的全互斥定义策略.pdf

| 核心主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|
| 状态扩展 z=(ℓ,δ,I_γ,r,σ_p,ω,β,κ,d,c,m) | **已装（主体）**：MuClass 13 维——level/delta/i_class(64 类非坍缩掩码)/parent_dir(=σ_p)/short_swing/position(=ω)/horizontal(=H)/force_state(=β 第8维)/sigma_higher(第9维 #132)/cand_channel(第10维 #138)/nest_depth(第11维)/origin_level(第12维，Jchain 代数派生非缺口，对照表论证在码注)/risk_mode(第13维 M0-M4=m) | rust/src/theta_v0/backtest/mu_estimator.rs:87-144 | — |
| 　z 残余维：TStage/ηBucket + TV/SubMovePower | **C 在办** #149 | gap-master2 B2/B1 归口 | 中 |
| 　z 残余维：c 成本 | **B 登记**（CostBucket 无 bar 级时变成本生产者） | gap-master2 B3 | 低 |
| 　z 残余维：d 结构止损距离 | **存疑**：未在 MuClass 与 gap-master2 A1/B 清单中找到对应物或登记——7 份中唯一无凭据的 z 维，交 #156 汇编判 A/B | mu_estimator.rs（无） | 低-中 |
| 全互斥解释器 C_j=P_j∧⋀_{k<j}¬P_k + C_0 兜底 | **已装**：P1..P10 固定优先级+C0，构造性互斥定理（最小成立索引），bar 级 P1-P4 从 TW 账本态独立重推导（oracle 第二实现） | rust/src/theta_v0/strategy/mutex.rs:90-215；#124；裁定4 f9333e21b2 | — |
| 证书 I_γ≠∅、零 bit→C_0 保持/L_t 记录不交易（dlpdf-a 矛盾-A） | **已消解=已装**：P10 record_struct_break=「记录不加仓」（codex-q2-d1 §4 加仓子动作不存在声明沿袭）——与原文 L_t 语义一致，StructBreak 不产生交易动作 | mutex.rs P10 注释 | — |
| AncOK 祖先闭包 A_{t+1}=AncOK[(A_t∖D_t)∪O_t] | **已装+A 残余**：persistent.rs 最小 overlay 已装（dlpdf-d 三链之身份链已修）；Stale 分支伪造 parent:None 的放宽偏差仍开 | strategy/persistent.rs；gap-master2 **A4**（recursive_tower.rs:56） | 中 |
| 短差语义（Sub∧δ=−σ_p，父仓保持+反向双开） | **已装**：short_swing 判定入 z（from_certificate：parent_dir≠0∧δ=−σ_p）+ 解释器 P7 close/P9 open ShortDiff + f3/f3c 反事实实证 | mu_estimator.rs from_certificate；mutex.rs P7/P9；#117 | — |
| alpha 定理（细分类优势 V(Z)≥V(Y)/鞅不可能/Eat 命题/元素数 O(n)） | **L0 理论层非实装项**；估计层对应药方已装（见 2.④）；Eat 覆盖定理=gap-master2 主文相邻（on2.pdf 族，dlpdf-e §10 移交项） | dlpdf-a §2.1 | — |

### 2. 缠论的全互斥定义策略2.pdf（①②同上，引 1；独有③④）

| 核心主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|
| ③ 背驰二值谓词 D_t^δ∈{0,1}，B=Context∧D；β 力度是细分非定义必要 | **部分装**：β 细分层已装（ForceStateA5 第8维，#112/#115 热路由 bit-exact）；D 作为定义层显式二值谓词的判定口径仍单一 MACD 面积——归 gap-master2 **A3**（Weak 判据仅 MACD area）族 | divergence.rs；gap-master2 A2/A3 | 中 |
| ④ 过拟合层：结构分类器不过拟合/μ̂ 估计层必过拟合；winner's curse；shrinkage+LCB+维数控制+cross-fit OOS；Le Cam 两点 n_z·d_z² 下界；跨品种 pooling 部分可交换 | **已装**：prereg_windows/perm_test/pooling_icc/mu_estimator/selector/l3_fullwindow/decontam 全管线（dlpdf-e §6 确认为方法论蓝本落地）；Le Cam 稀疏类⟹663/665/667 功效门（¬powered 桶）操作化；跨品种=pooling_icc+已结算判决（跨标的 L3 INCONCLUSIVE、正号 BTC 独有） | rust/src/theta_v0/backtest/{prereg_windows,perm_test,pooling_icc,mu_estimator,selector,decontam}.rs | — |

### 3. 多空对冲.pdf

| 核心主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|
| 净额 NAV 不可识别定理 + 分账本层分离（P^sep/Leg/Net 有损投影/Eat^sep） | **已装**：separate.rs 注释直引 PDF §一 页12（C25/C26/C29 R2），净额与分账本正交并置不强行统一 | rust/src/theta_v0/ledger/separate.rs；dlpdf-e §8 | — |
| §9 双开抵消定理（同标的同单位数双开价格 PnL 恒 0，扣成本 <0） | **已装+已证**：net_le_gross、hedged_gross_high_net_zero 性质测试 | strategy/risk.rs；margin-model-design §1.4 | — |
| 三层判定表：声部生成层/净额可见层/经济有效层 | **已装（实证已跑）**：声部层 ShortDiff 对冲腿 1.2%（42 条）；净额层 ‖ΔN‖₁ 驱动源极小⟹「b2 净额不可见」倾向成立；经济有效层 overlay E[HΔP−ΔC] 反事实 π 差分=#117 f3c | f3-fugue-backtest-20260703.md L37-38；#114/#117 | — |
| 　逐 bar 同时活跃声部数+overlay_net_delta 的 ‖ΔN‖₁ 逐 bar 分布采集器 | **B 登记**：f3 §4 ceiling 诚实声明（信号级占比已足判定，逐 bar 是精化非翻转）；overlay_net_delta 原语已在（f2 f7c60de5da） | f3-fugue-backtest §4 | 低 |
| §10 账户模型三分类（one-way/hedge/portfolio margin） | **已装 v0 前二**：one-way 净额 MM 低/hedge 毛额 MM 高对齐已证性质；portfolio margin（SPAN）**B 登记** L2 延后 | margin-model-design §1.4（L144） | 低 |
| 分账本载体 position_node 四元组 | **A 引用**：gap-master2 **A9**（条件触发型：多 active instance 出现即 binding） | nesting-fugue-conformance §126 | 中（条件） |

### 4. 绝对资本.pdf

| 核心主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|
| §1 不相容定理（非平凡尺度等变+固定绝对资本上限+全域策略等变不能同时成立） | **已装（作为设计约束显式消费）**：margin-design §5.1 引为「MM/buffer 不能用裸美元绝对值」的根因——裸绝对值=特权尺度破坏自相似 | margin-model-design §5.1（L317-321） | — |
| 方案 A：资本改协变量（U_ℓ 级别资本单位，无量纲化 B̄=B/U_ℓ） | **已装**：η⋆(x)=L^wc+κQ 状态依赖 barrier（非硬编码常量）+ B1/B2 协变缓冲归 Θ_risk + risk.rs:311 归一化等价注释（E<MM+B1 ⟺ Ē<MM̄+B̄₁ 两侧同除 U_ℓ 对消） | closed_loop/transition.rs；strategy/risk.rs:311 | — |
| 方案 C：B_2 保证金绑定 regime 互斥态 | **已装**：risk_mode M0-M4 五态穷尽互斥（Lean risk_mode_complete_unique Σ𝟙=1）+ MuClass.risk_mode 第13维入 z；#113/#120 真 MM 口径实装重冻结 | strategy/risk.rs:333；mu_estimator.rs 第13维 | — |
| §5 三阶段资本约束自相似写法（W̄<Ī₀ 归一化） | **已装**：risk.rs:311 注释即其原文依据（margin-design §5.1 L320 确认） | risk.rs:311 | — |
| （margin 未覆盖部分：M2/M3 接线扩展+ADL/资金费强平） | **A 引用**：gap-master2 **A10**（BTC 当前不 binding，接 IBKR/多品种即 binding） | codex-margin-ruling | 中（条件） |

### 5. 经济正条件.pdf

| 核心主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|
| 结构-价格正条件 ≠ 经济正条件层级区分；CEP 公理（q_e ε_e(P_τout−P_τin)−C_e≥η_e>0） | **已装**：decompose_capturable_spread/SpreadAttribution/spread_eaten/actual_pnl_proxy/actual_pnl_eaten——「可捕获价差是否被滑点/成本/确认滞后吃掉」正是层级区分工程化（dlpdf-e §5：与 PDF 第5节完全对应） | rust/src/theta_v0/backtest/econ_positive.rs | — |
| 弱统计版本 E[X(a)∣Z=z]>0 | **已装**：μ̂(z,a) 管线+χ selector（含 LCB/功效门） | mu_estimator.rs/selector.rs | — |
| 经济正条件⟹全互斥策略存在 alpha（定理） | L0 理论层；经验检验凭据=econ 系列结果包（OOS level0sell/walk-forward/663 全级别 μ） | econpositive-oos-level0sell、econpositive-walkforward、econ-663-full-level-mu | — |
| τ^reverse 旧口径拷贝残留（诊断路径） | **A 引用**：gap-master2 **A8**。注：本分片时点 econ_positive.rs 在 gap3-rework-codex9-fix 分支有未提交改动，grep tau_reverse 已无命中——A8 可能正被在办分支消化，最终状态交 #156 汇编以 merge 后码为准核验 | gap-master2 A8 | 低 |

### 6. k的条件.pdf

| 核心主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|
| κ 严格定位：barrier 缓冲系数（c^adj≤−κ ⟺ η≥κQ），η⋆=L^wc+κQ | **已装**：transition.rs barrier + RiskPolicy.kappa | closed_loop/transition.rs；strategy/ledger.rs:204-236 | — |
| 定理2：κ 对价格数据不可识别⟹κ_policy∈Θ_risk 须显式声明来源 | **已装**：ledger.rs 注释显式声明为风险政策参数 + κ≥0 constructor-only 类型不变量（try_new，非法不可构造）+ Lean kappa_nonneg 对齐 | ledger.rs:215-236 | — |
| 双 κ 消歧（PDF §10 barrier κ vs sizing κ） | **已装**：ledger.rs:208-209 显式注释「与 RiskConfig.kappa（成本倍数 2.0，sizing 用）同名不同义」——既往坑（RiskPolicy.kappa≠RiskConfig.kappa）已封 | ledger.rs:208；risk.rs:198 | — |
| κ↑⟹ΔQ_max↓；κ 太大⟹永不进增股（GAP3 可达性） | **已装语义**：默认 κ=0 最小基线（注释明示）+ GAP3 可达性已闭环（#139/#140 Realize/funded_campaign/barrier-gated 推进） | ledger.rs:205；#139/#140 | — |
| 　κ 敏感性网格 | **存疑**：margin-design §3.5 对 buffer1/2 强制敏感性网格，κ（barrier 缓冲）未见同等网格测试要求——κ 同样直接决定 EnterEarning 边界，交 #156 判是否补 B 登记 | margin-model-design §3.5（类比） | 低 |
| EnterReady/BuyCore 合法性不等式 | **已装**：oq9_legal/stage_progression（gap.pdf 同族，dlpdf-e §1 确认注释直接契约锚定 PDF §10） | transition.rs | — |

### 7. 关于背驰.pdf

| 核心主张 | 分类 | 锚点 | 严重性 |
|---|---|---|---|
| 力度=完整递归签名 𝔉_ℓ(s)，MACD 面积仅一个投影坐标（非充分统计量） | **A 引用（带精确化）**：gap-master2 **A2**（Θ_DOM/Θ_SCORE 判定口径未接入，三套 OOS 未跑）+ **A3**（Weak 判据仅 MACD area，force_conformance 未确认接入 judge_first） | divergence.rs:446-478；gap-master2 A2/A3 | 中 |
| 支配序 s≺_A s'（∀m≤ ∧ ∃m<）+ 无唯一全序（定理2）+ 三值判定含 Incomparable | **状态层已装——对 A2 的精确化**：ForceStateA5 四态 {Dominated(确定背驰)/Dominates/Tie/Incomparable(不作背驰确认，codex-beta ②)} 就是 𝒜₅ 5-proxy 支配序（码注「唯一支配序原语，beta-bucket-design v2 §6 路由⑤」），#112/#115 已入 z 第8维+生产热路由 bit-exact。**A2 的残余口径应收窄为**：支配序原语已在，缺的是 Θ_DOM 作为 D 判定口径接入 judge + 三口径 OOS 对照——非「支配序未实装」 | rust/src/theta_v0/classifier/divergence.rs:343-352 | 中→（残余收窄） |
| 背驰四条件（4.1 同级同向 / 4.2 Comparable_ℓ 同上级趋势语境 / 4.3 价格推进 / 4.4 力度弱化） | **部分装**：4.1/4.3 结构条件在 div_cand 路径；4.4=force_state 支配态；4.2 Comparable_ℓ **存疑**——sigma_higher 第9维提供上级方向语境，但「同一上级趋势语境可比性」是否作为四条件严格合取接入背驰判定未见显式凭据，交 #156 | divergence.rs；mu_estimator.rs 第9维 | 低-中 |
| Cand^δ=StructEligible（宽结构候选），非 Cand=MACDdiv——MACD 不得一票否决 | **已装**：MACD veto 已移除（P2）+ Xzd 不受一票否决（codex-f2 #1）；与 #62 StructBreak 第四类纤维方向一致（dlpdf-d 确认） | acc-p2-macd-veto-removed-20260701；f3-fugue-backtest L123 | — |
| （Type2 与背驰的分层关系——源头考古补充） | **已对照**：L1 存在性由走势不患保证/L2 回抽确认择一判据/L3 统计性质/L4 区间套精确定位四层原文基底已入档（不裁决，A/B 分叉归编排者） | source-audit-type2-divergence-20260702 | — |

## 分片增量小结（供 #156 汇编）

1. **新 A 候选**：仅 1 条——z 的 **d（结构止损距离）维**无实装、无登记、无在办凭据（策略1 表第4行）。其余 A 类全部是 gap-master2 主文已收录条目的引用（A2/A3/A4/A8/A9/A10），本分片零重复登记。
2. **对主文 A2 的精确化**：支配序原语（ForceStateA5 𝒜₅ 四态）已实装并入 z 第8维——A2 残余应收窄为「Θ_DOM 作为 D 判定口径接入 + 三口径 OOS」，非「支配序未实装」。建议汇编时更新 A2 表述。
3. **新存疑项 3 条**：d 维（上）、κ 敏感性网格（k的条件6）、背驰 4.2 Comparable_ℓ 严格合取（关于背驰7）。
4. **本族总体判定**：7 份中 4 份（多空对冲/绝对资本/经济正条件/k的条件）核心主张**已充分实装且有直接码注引用或性质测试**，是全部 36 份 PDF 中实装耦合度最高的一族（与 dlpdf-e「PDF 被代码引用为设计蓝本」模式一致）；2 份全互斥定义策略的主张主体已装（13 维 z+P1..P10 解释器），残余归口既有 A/B/C 条目；关于背驰是本族唯一 A 类残余集中地（A2/A3 力度公理层）。

## 结果包六要素

1. **结论**：如上逐 PDF 表与增量小结。
2. **定义依据**：dlpdf-a/d/e（0702 逐页全读）提取的各 PDF 核心主张；现码验证锚点 mutex.rs:90-215、mu_estimator.rs:87-144、ledger.rs:204-236、divergence.rs:343-352、risk.rs:311/333、margin-model-design §5.1。
3. **边界条件**：①本分片未重读 PDF 原文，若 dlpdf 提取有遗漏主张则本对照继承该遗漏（dlpdf 系列自声明逐页全读，风险低）；②A8 状态受本分支未提交改动影响，以 merge 后为准；③A2 精确化若 codex 认定 ForceStateA5 与 Θ_DOM 语义不同构则收窄不成立；④d 维若在 gap-master2 主文 A1「H 未入」的 H 被另读为止损距离（实为水平关系 Horizontal，已排除）之外另有登记，则新 A 候选撤销。
4. **下游推论**：#156 汇编应吸收：新 A 候选 1 条（d 维）、A2 表述更新 1 条、存疑 3 条；本族无需新增专项工位——A 类残余全部已在主文归口。
5. **谱系引用**：codex-q2-d1 §4（StructBreak 记录不加仓）、裁定4 f9333e21b2（解释器真统一）、#81（σ_higher 与 σ_p 不合并）、667（级别依赖调制器）、231（诚实 None 不伪造）、090（声明膨胀）；未发现需新开谱系的概念分离。
6. **影响声明**：新增本文件，零代码改动；对 gap-master2 主文的 A2 提出表述收窄建议（不直接改主文，归 #156）。
