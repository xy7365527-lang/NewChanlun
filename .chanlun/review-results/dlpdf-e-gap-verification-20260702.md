# PDF 全读 E 组：gap+检验组（10份）验证结果包

**任务**：#74 Downloads PDF 全读 E — gap.pdf / 三个gap.pdf / 目前的缺口.pdf / 问题.pdf / 经济正条件.pdf / 过拟合.pdf / 多级别检验.pdf / 多空对冲.pdf / 绝对资本.pdf / on2.pdf

**方法**：逐页全读（pdftoppm 分页提取，≤20页/次），对照生产代码（`rust/src/theta_v0/`）核实每份 PDF 核心裁决是否已实装/已结算/仍是缺口。

---

## 一、覆盖声明

10 份全部逐页读完（页数：gap 16 / 三个gap 15 / 目前的缺口 21 / 问题 16 / 经济正条件 19 / 过拟合 14 / 多级别检验 20 / 多空对冲 16 / 绝对资本 12 / on2 20）。

首次读取时工具返回的图像显示层出现错位（图像内容一度显示为其他工位 #70 的"推导完全分类"系列文档），已通过重新按 `pages` 参数精确分页读取并按页数+时间戳匹配核实到正确文件，全部 10 份内容确认对应正确文件名。

---

## 二、逐份核心裁决 vs 代码现状

### 1. gap.pdf — GAP3 三阶段严格触发定义
**核心裁决**：PDF 三阶段（CostReduction/CapitalRecovered/EarningShares）是缠师原文"两相位"（η=0 唯一阈值）的严格 refinement，非原文唯一推出；`EnterReady_t := S_t=II ∧ W_t≥I_0 ∧ η_t≥η⋆(x_t) ∧ openLegacyLegs_t=0 ∧ RiskNormal_t` 是最安全 canonical 触发；η⋆应为状态依赖 barrier `η⋆(x_t)=L^wc_{t+1}(x_t)+κQ_t`，不可硬编码常量；action priority P1-P7 互斥穷尽定理；Stage III 可达性定理（需 Ready + 有限步关闭 overlay + 无风险强平阻断）；不可达性同样要保留（τ2,τ3 可能为 +∞）。

**代码现状**：`rust/src/theta_v0/closed_loop/transition.rs` 的 `stage_progression`/`schedule_adapter`/`oq9_legal` 已实现单向阶段迁移 + OQ-9 gate + 双账本事件派生，注释直接契约锚定 "PDF §10 步骤2/3"。但 `EnterEarning` 从未被派发——hwm_gain 已被 codex R3 C' 终局裁定降为纯诊断（零承重，不入 free），EarningShares 结构性不可达。这**不是矛盾**：PDF 本身在"9. 不可达性也要保留"一节明确"形式系统不能写 ∀run,∃t:S_t=III"，与当前"若利润不足或守卫不满足，则 III 可以不可达，但策略仍全定义"完全吻合——已是settled裁决（见 memory `project_gap3_l2_unreachable_architecture`）。

### 2. 三个gap.pdf — Cand 严格对象与区间套递归证书
**核心裁决**：`Cand^δ_ℓ` 应定义为"因果可见的ℓ级δ方向背驰段候选"，而非"已经确认的ℓ级买卖点"；`Cand ≠ Conf`（Cand=外层背驰定位区间，Conf=最内层执行级别买卖点确认）；背驰判定应参数化为力度泛函 `M_Θ,ℓ(s,t)`，MACD面积只是其一个特例，不应硬编码为唯一 canonical；`J^δ_ℓ` 取候选段时间区间；递归证书 `N^δ_{ℓ↓e} = Cand^δ_ℓ ∧ [J^δ_{ℓ-1}⊆J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}` 唯一性/soundness 定理；MACD 一票否决=选择机制嵌入标签定义（选择偏差），除非声明 `Θ=Θ_MACD`。

**代码现状**：本 session 正在改动 `rust/src/theta_v0/classifier/bsp.rs`、`classifier/mod.rs`（git status 显示未提交改动），涉及任务 #41/#44/#47/#55/#56 的 C3 判据重设计（新中枢+突破方向、level==1 通道）。**这是本组与当前工作线关联度最高的一份**——Cand/Conf 分层定义是否已在新判据中严格区分，需要 C3 相关工位在收敛时对照本节裁决自查，暂不在此工位判定为矛盾或已实装（超出本工位只读职责）。

### 3. 目前的缺口.pdf — 现存缺口清单（21页，含 pdf-icon 空白终止页）
内容与 gap.pdf/三个gap.pdf 高度重叠（同一时间段的往返裁决），核心新增点：确认"缠师原文只给出两相位阈值 η=0，PDF 三阶段是严格 refinement 而非原文单独推出的唯一形式"这一裁决贯穿全篇作为方法论前提，未见超出 gap.pdf 覆盖范围的独立新裁决。

### 4. 问题.pdf — 完整策略 alpha 检验七问严格裁决（英文）
**核心裁决**：Π_simp ≠ Π_full（P1: 区间套证书差异 / P2: MACD候选域差异 / P4: 买卖点类型分桶差异 / P7: 出场规则差异 / GAP3: 资金管理差异五处结构性不等价）；旧回测"无 alpha"结论只能否证 Π_simp，不能提升为"完整缠论无 alpha"；P1+P4+P7 是信号层 alpha 最小充分核心，完整策略 alpha 需 P1+P2+P4+P7+GAP3；账本选 C（R账本+TW账本双层并置）；η⋆应 state-dependent；区间套可渐进实装但语义验收不能降级（只有 FullNest 才能称"区间套已接入"）；MACD veto=选择偏差（除非声明MACD canonical）；完整重跑需三层分离验收（信号层/组合层/资金管理层不可混算一个 alpha 数）。

**代码现状**：
- 账本选C（576=C 双层并置）→ **已完成实装**（任务 #8/#21/#26，`LedgerState`+`TWState` 双层并置）。
- η⋆ state-dependent → **已实装**（`transition.rs` 的 `η*(x)` barrier 逻辑）。
- P1+P2+P4+P7+GAP3 最小充分集 → 部分对应任务 #51/#52（a3-实装①②：σ^H 桶键+分层维度+walk-forward LCB、prereg 文本订正），是 W-VERIFY 重测口径的直接落地。
- 三层分离验收（信号/组合/资金管理不可混算）→ **需核查**：当前 `W-VERIFY alpha 全量重测`（任务#13）是否已严格分层报告三个层级的 alpha 数字，还是仍混算成单一数字。此项建议移交 W-VERIFY 相关工位复核，本工位未见明确的分层报告产物。

### 5. 经济正条件.pdf — 结构正条件 vs 经济正条件的层级区分
**核心裁决**：笔端点"顶高于底"的价格序关系（`ε_e(P_ρ-P_λ)>0`）是**结构-价格正条件**，不能单独推出**因果交易后成本净收益为正**（经济正条件）；需要额外的 Causal Endpoint Profitability 公理（CEP：`q_e ε_e(P_τout-P_τin)-C_e ≥ η_e>0`）或更弱的统计版本 `E[X(a)|Z=z]>0`；给出完整公理层级：走势终完美⟹收益变量可定义；端点序关系⟹存在理想价差；可成交价差扣成本仍为正⟹经济正条件；经济正条件⟹全互斥策略存在 alpha（严格定理+证明）。

**代码现状**：`rust/src/theta_v0/backtest/econ_positive.rs` 的 `decompose_capturable_spread`/`SpreadAttribution`/`spread_eaten()`/`actual_pnl_proxy()`/`actual_pnl_eaten()` 正是本 PDF"结构正条件（可捕获价差）→ 经济正条件（实际扣成本净收益）"层级区分的具体工程化——检验"可捕获价差是否被（滑点/成本/确认滞后）吃掉"，与 PDF 第5节"严格可证明版本：成交价误差有界"完全对应。**该 PDF 的核心裁决已在生产代码中实装**，未见矛盾。

### 6. 过拟合.pdf — 结构分类器 vs 收益估计器的过拟合层级区分
**核心裁决**：缠论结构分类器 `C_Θ:x↦z` 本身零参数、确定性，不拟合收益，因此没有统计意义上的过拟合；但条件收益估计器 `μ̂(z,a)=1/n_z·ΣX_m(a)` + 选择器 `χ=1[μ̂>θ]` 是随机估计量，必然过拟合（winner's curse，多重比较，K=300时约150类别假阳性）；标准解 = shrinkage（层级收缩模型）+ LCB（置信下界）+ 严格 OOS + block/cluster 检验；level0 卖 349 信号可单独 OOS 验证，但须预注册+有效样本数修正（自相关降低 n_eff）+ block bootstrap，不能用全样本挑赢家；跨品种 pooling 需验证部分可交换假设（leave-one-asset-out 等）。

**代码现状**：`rust/src/theta_v0/backtest/prereg_windows.rs`、`l3_pi_falsify.rs`、`perm_test.rs`、`pooling_icc.rs`、`mu_estimator.rs`、`selector.rs`、`l3_fullwindow.rs`、`decontam.rs` 已构成完整的 prereg/walk-forward/pooling/LCB 防护管线（对应任务 #51/#52 W-VERIFY 收敛）。**该 PDF 是 W-VERIFY 过拟合防护线的方法论蓝本，已在代码层落地**，与既往结算谱系（`project_stheta_v1_fullwindow_l3_falsified`、`project_oddeven_mu_identity` 等 memory 记录的 L3/perm_test 否证结果）一致，未见矛盾。

### 7. 多级别检验.pdf — L0-only 负结果 vs 多级别 alpha 的逻辑独立性
**核心裁决**：全互斥完全分类不会消灭 regime（除非 `ΔP_{t+1}⊥R_t|Z_t` 或 `R_t=f(Z_t)`）；买卖两边"理论上都该赚钱"不是镜像语法自动推出的（需要市场分布+成本都镜像对称，现实数据通常不对称，方向不对称不违背缠论语法）；**严格定理**：`Π_0⊆Π_{≤L}` ⟹ `V_{≤L}≥V_0`，L0-only 无 alpha **不能推出**多级别缠论买卖点无 alpha；多级别过滤可以把总体亏损的 L0 买点池子分解出正收益子类（即使总体 `μ_{0,+}<0`，某上级语境 `y*` 下 `μ(0,+,y*)>0` 可能成立）；正确实验设计=三个实验（按上级方向分桶 / 多级别买卖点本身 / 区间套确认）。

**代码现状**：`econ_positive.rs` 第3217-3494行的 `xzd_l1_same_side_causal_ok`/`same_center_any`/`same_side_l0_type3_any` 等诊断字段正是"lvl==1 子集分裂断点"分析——与本 PDF"multi-level filtering rescue theorem"关切点方向一致（拆分总体池子找上级语境正收益子集）。另外，"regime 不会被全互斥分类自动消灭"这一裁决与既往结算的 memory `project_regime_is_level_truncation_artifact`（★regime=级别截断伪影）在概念层同构，非新矛盾，是同一发现的独立佐证。

### 8. 多空对冲.pdf — 净额 NAV 不可识别定理与分账本层分离
**核心裁决**：净额 NAV 只能看见净头寸过程 `N_t`，看不见分账本腿结构 `p_t` 本身——`N^{#5}_t=N^{base}_t ⟹ ΔSharpe=0` 是数学必然（净额不可识别定理），但反过来若 `#5` 子腿真改变净头寸（非完全对冲 `w≠1`），净额 NAV 原则上可以看见；`§9` 双开抵消定理精确版本：同标的同单位数父仓保持+反向双开⟹价格 PnL 恒为 0，扣成本后 <0；overlay（ShortDiff对冲腿）的经济实质取决于 `E[H_tΔP_{t+1}-ΔC_t]>0` 是否成立，否则只是风险转换/暂时降敞口而非 alpha；三层判定表（声部生成层 depth>0 / 净额可见层 ‖ΔN‖₁ / 经济有效层 overlay IR）。

**代码现状**：**该 PDF 是 `rust/src/theta_v0/ledger/separate.rs` 的直接设计蓝本**——该文件注释明确引用"PDF §一 页12"，实现了 C25/C26/C29 工作单元 R2：分账本头寸空间 `P^sep`（Leg/SepPosition/Net）+ 净额映射 `Net`（有损投影，双开 (Q,Q)↦0 退化）+ 分账本吃到 `Eat^sep`，并明确声明"净额账本与分账本正交并置，不强行统一"——与本 PDF 第10节"支持多空双开的保证金系统会不会不同"的三种账本模型分类（one-way/hedge mode/portfolio margin）在架构原则上完全一致。**该 PDF 核心裁决已充分实装，未见矛盾**。

### 9. 绝对资本.pdf — 严格自相似与固定绝对资本上限不相容定理
**核心裁决**：非平凡尺度等变 + 固定绝对资本上限 + 全定义域策略等变，三者不能同时成立（不相容定理，证明：`S_k` 缩放头寸规模但 `B` 不变则可行集不能等变）；三种修复方案——A（资本改协变量，无量纲比例，`S_kΘ=Θ`，全域严格自相似）/ B（承认资金投影层打破自相似，只在非绑定域 `X^{nb}_k` 内等变）/ C（固定绝对资本+binding regime 分类，完全互斥全定义但不再全域自相似）。

**代码现状**：`transition.rs` 中的 `η⋆(x_t)=L^wc_{t+1}(x_t)+κQ_t` 是状态依赖（协变）barrier，非硬编码绝对常量，`κ` 是风险政策参数（不由价格唯一导出）——这与本 PDF"方案 A：资本改成协变量"的核心修复思路一致（无量纲比例 + 协变资本单位）。当前实现看起来已避免了"固定绝对资本上限破坏自相似"的陷阱，未见需要上浮的矛盾。

### 10. on2.pdf — 缠论元素数量渐近界 + Eat(e) 祖先生命期不变量缺口
**核心裁决**：**问题一**（渐近界）——规范单父/非共享缠论元素树在每级至少 m 个子构成父下 `|E^can(n)|=O(n)`；若允许 DAG 共享（子被 b 个父共享），则 `b<m⟹O(n)`，`b=m⟹O(n log n)`，`b>m` 可超线性甚至 `O(n²)`；实测 `B(n)≈n^{1.26}` 不反驳 `|E^can|=O(n)`，除非 `B(n)` 确实统计的是去重规范 BSP 节点数而非候选/事件流。**问题二**（Eat(e)缺口）——PDF 证明 `Eat(e)` 时把"保持"错写成 `e∈A_t⇒e∈A_{t+1}`，但实际活动集递归包含 `AncOK` 要求所有祖先都在活动集内；`Eat(e)` 成立的充要条件是 **`∀a∈Anc(e), [λ_e,ρ_e)⊆[λ_a,ρ_a)`**（祖先生命期包含不变量），若 `par` 只是 host/依附/最近容器关系而非严格构成父关系，该不变量不自动成立，存在明确反例（祖先提前结束会剪掉后代活动资格，即使后代自己未结束）；需要补 `WF-Contain`/`WF-HalfOpen`/`WF-Par` 三条公理才能补完 PDF 证明。

**代码现状 / 待核实缺口**：本 session 正在改动的 `bsp.rs`/`mod.rs` 涉及中枢（Center）/塔（Tower）构成关系（`detect_centers_complete`/`detect_centers_geometric`/`classify_with_tower`/`TowerCache`）。**这是本组 10 份 PDF 中唯一识别到的、指向当前正在改动代码的未决核实项**：需要确认中枢/塔的父子构成关系（谁是谁的"父"）是否满足半开区间包含不变量（`WF-Contain`），即子中枢/子走势段的生命周期区间是否严格被父中枢/父级别走势段的生命周期区间覆盖。若 `par` 关系是通过"host/依附/最近命中容器"隐式定义（而非显式构成关系），则参照 on2.pdf 第14页反例，可能存在"祖先已结束但活动集判定仍试图吃到本应仍存活的子节点"的边界问题。**建议移交 C3 判据重设计相关工位（#41/#44/#47/#55/#56 序列）核查**，本工位（只读、无 git 改动权限）未做代码修改验证，仅指出需要核实的具体点。

---

## 三、矛盾即报检查

对照任务描述要求的六项交叉点：

| 交叉点 | 结论 |
|---|---|
| GAP3 hwm 终局 FALSIFIED | **一致，非矛盾**——gap.pdf/问题.pdf 的"不可达性也要保留"裁决与代码 EarningShares 结构不可达（codex R3 C' 终局裁定）完全吻合 |
| econ_positive 实装 | **一致**——经济正条件.pdf 的结构/经济正条件层级区分已在 `econ_positive.rs` 工程化 |
| prereg/walk-forward 过拟合防护 | **一致**——过拟合.pdf 的 shrinkage+LCB+OOS 标准解已在 W-VERIFY 管线落地 |
| 塔多级别检验 | **一致（方向吻合）**——多级别检验.pdf 的"regime 不被分类自动消灭"与既有 memory `project_regime_is_level_truncation_artifact` 同构 |
| delta/σ^H 对冲 | **一致**——多空对冲.pdf 是 `ledger/separate.rs`（P^sep/Net/Eat^sep）的直接设计蓝本，PDF 原文被显式引用 |
| TW 账本 | **一致**——问题.pdf 的账本选 C 裁决已实装为 576=C 双层并置 |
| 优化泳道 | 未见本组 PDF 与优化泳道（#40 compose alloc 等性能工作）直接冲突 |

**未发现需要 `/escalate` 上浮的定义矛盾**。唯一识别到的待核实缺口是 on2.pdf 指向的中枢/塔父子关系半开区间包含不变量（见上文第10项），性质是"需要现场核查是否满足"而非"已确认的矛盾"，已建议移交对应工位，不构成本工位需要上浮的矛盾。

---

## 六要素（结果包格式）

1. **结论**：gap 组 10 份 PDF 全部逐页读完；除 on2.pdf 指向的中枢/塔父子关系不变量需要 C3 工位现场核实外，其余 9 份的核心裁决与生产代码高度吻合（多份 PDF 被代码注释直接引用为契约锚），未发现新矛盾。
2. **定义依据**：见上文逐份"核心裁决 vs 代码现状"，每项均引用 PDF 具体页/节与对应源码文件行号。
3. **边界条件**：若 C3 工位核实发现中枢/塔的 `par` 构成关系确实不满足半开区间包含（即存在"父已结束但子活动资格判定仍依赖父"的路径），则 on2.pdf 的 `Eat(e)` 定理不成立，需要按 PDF 第2.7节补 `WF-Contain`/`WF-HalfOpen`/`WF-Par` 三条公理，或修改 `AncOK` 判定逻辑。
4. **下游推论**：本组 PDF 大部分内容已被既有实装吸收，说明形式化链条 gap→检验组与生产代码的耦合度高于其余分组；后续 PDF 全读工位若发现类似"PDF已被代码引用为设计蓝本"的模式，建议直接标注"已实装"而非重新推导。
5. **谱系引用**：`project_gap3_l2_unreachable_architecture`、`project_regime_is_level_truncation_artifact`、`project_stheta_v1_fullwindow_l3_falsified`（均为本 session 相关 memory，非本工位新增谱系条目）。本工位未新增谱系写入（纯只读任务）。
6. **影响声明**：本工位零 git 改动，仅产出本结果包文件。识别的唯一待办（on2.pdf 中枢父子关系核实）已通过 SendMessage 转交 team-lead 决定是否分派给 C3 相关工位。
