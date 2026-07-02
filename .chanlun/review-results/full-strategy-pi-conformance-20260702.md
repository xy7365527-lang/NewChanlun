# 完整策略层 π 逐节对照定案（full-strategy-pi conformance）

- **工位**：ws-pi1（goal g-20260702T2330Z-full-strategy-pi 首工位）
- **日期**：2026-07-02
- **HEAD**：9e3e8ab066（纯只读审计，零 git 改动）
- **对照原文**：`docs/formal-chain/缠论的全互斥定义策略.pdf`（27 页，逐页读）+ `缠论的全互斥定义策略2.pdf`（37 页，逐页读）
- **代码锚点根**：`rust/src/theta_v0/{strategy,backtest,closed_loop,nautilus}/`
- **认识论**：本报告的对照判定是 **L0/L1**（结构存在性 + 接线核对——「代码里有没有这个构造、它是否被回测消费」），**不是** L2/L3（不判定 π 是否在真实数据上产生 alpha）。alpha 有效性由既有 wverify/L3 线（project_wverify_alpha_retest_pass、project_stheta_v1_fullwindow_l3_falsified）承载，不在本工位范围。

---

## 0. 前置结论：两条平行世界 + 对 goal 前提的订正

编排者 goal 前提是「已完成的只是**逐信号 alpha 测量**（μ̂(z) 判定），不是**完整策略 π:Z→A**」。逐节盘点后，此前提需**部分订正**：

代码里存在**两条平行路径**，都已实装：

| 路径 | 入口 | 做什么 | 对齐 PDF |
|------|------|--------|----------|
| **(甲) 完整策略 π** | `runner::run_theta_v0_pi_inner` → `coverage::pi_theta_step_prebuilt` | 每 bar：Γ 候选集 → ≺_Θ 三桶 → AncOK 活动集 → 净持仓账本 p̃ → LexArgmin J over K_Θ → `Schedule(p*−p_t)` 订单 → fill/MtM → equity 曲线 | PDF1 五、策略定义 + 十三最终策略形式 `π=Schedule[LexArgmin J−p]` |
| **(乙) 逐信号 μ̂(z) 测量** | `backtest::econ_positive::decompose_capturable_spread` / `mu_estimator::MuEstimator` | 逐买卖点收因果信号，配对退出，估 μ̂(z) / 残差 Y_i、LCB、shrinkage、ICC | PDF1 三~九（μ/收益/稀疏/收缩/LCB）+ PDF2 后半（winner's curse/估计层） |

**关键接线**：两条并非割裂——(乙) 的 μ̂ 选择器经 `χ_t` 门（`run_theta_v0_pi_chi` / `run_theta_v0_pi_chi_shrink`）**注入 (甲) 的 π 回测**（`ChiFilterCtx`，`Γ_t^trade={γ:LCB(μ)>θ}`）。故 PDF1 十三「`χ_t(z,a)=1[LCB>θ∧RiskOK∧ConflictOK]`」的选择器已接进完整 π。

**订正**：「只有逐信号测量、没有完整策略」不准确。完整 π 状态机（声部/AncOK/净账本/LexArgmin/K_Θ/χ 选择器）**已实装且被回测消费**。真实缺口不在「π 不存在」，而在**三处具体不完备**（详见 §缺口分级）：
1. PDF Part B 四「8 谓词全互斥解释器 I_Θ=(D_t,O_t,L_t)」有**两个实装**——`interp::interpret`（已接线，用 ≺_Θ 全序三桶）与 `mutex::mutex_class`（P1..P8 固定优先级，**无生产消费者，未接线**）。二者是同一 spec 的两种构造，**未统一**。
2. 选择器状态 z 的**非拓扑细分维度不全**：β（背驰强度连续量）、κ（中枢位置细粒度）未进 μ̂ 桶键（当前 `MuClass` 只到二值背驰 + Horizontal 粗桶）。
3. **RiskMode μ_t 五态退化**为 equity≤0 单分量（maint_margin/buffer/liq_flag 未建模）。

---

## A. PDF1 前半「推导完全分类」（页 1–12，第 1–14 节）

| 节 | 原文要求（页码） | 代码锚点 | 判定 |
|----|-----------------|----------|------|
| 1. 状态扩展 z=(ℓ,δ,I_γ,r,σ_p,ω,β,κ,d,c,m)（p1-2） | 完整 11 维状态 | `mu_estimator::MuClass{level,delta,i_class,parent_dir,short_swing,position(ω),horizontal}`；strategy 侧 `VoiceDecision{depth(≈ℓ),root_side(δ),bsp(I_γ),...}` + `coverage::operation_role`(r=18类 H×V×δ) + σ_p(attached_dir) | **部分**：ℓ/δ/I_γ/r/σ_p/ω/κ(粗) 到位；**β/d/c/m 未进桶键**（d/c 进 sizing/risk 但非 μ̂ 桶维，β 只二值） |
| 2. 仓位大小=状态函数 q_Θ=LotFloor(min{ρW/D, ΓW/MP})；D=M\|P−S\|+Cost+GapBuffer；多空不镜像 ρ_{+}≠ρ_{-}（p2-3） | sizing 三路 min + 结构止损 D + 多空 profile | `risk::size_position`（三路 min）、`risk::structural_stop`（S）、`config.sizing_profile.resolve(level,side_key)`→(ρ,Γ,gap_buffer)（多空/级别 override，注释 §3「多空不强行镜像」）、`coverage::feasible_net_cap`（Γ̄ 绝对资本协变，CovariantCapital.lean） | **已实装** |
| 3. 级别化收益 X=q·side·(P_τ−P)−C；μ(z,a)=E[X\|z]；风险归一化 Y=X/R, R=q·D（p3-4） | 逐信号 X + 残差 Y | `mu_estimator::marginal_return`(X)、`ResidualTrade{resid_base=δ(H−B̂),cost}`(Y=X−δB̂)、`MuEstimator`(μ̂) | **已实装**（Y 用残差减 beta 口径，alpha分离.pdf §1） |
| 4. 多空拆成 μ_{ℓ,+}/μ_{ℓ,−}（p4-5） | δ 进桶键分开估 | `MuClass.delta` 是桶键维 ⟹ 多空独立桶 | **已实装** |
| 5. 级别拆（p5） | ℓ 进桶键 | `MuClass.level` / `level_bucket` | **已实装** |
| 6. 细分优势定理 V(Z)≥V(Y)（p5-6） | oracle 单调性（数学命题） | 无实装对象（L0 定理，桶键扩维 #83 是其操作化启发但非「实装此定理」） | **N/A**（codex-p2 改判：数学命题非实装对象，不入三态） |
| 7. 样本稀疏 n_z≳6.19/d²（p6-7） | 功效阈 | `prereg_windows`（预注册窗口 + 功效）、`mu_estimator` 的 n_z 计数 | **已实装** |
| 8. 层级收缩 μ̂_shrink=w·X̄+(1−w)·pooled, w=n/(n+σ²/τ²)（p7-8） | shrinkage | `run_theta_v0_pi_chi_shrink(tau_sq)` + `mu_estimator` shrink 分量、`pooling_icc`（ICC 分解） | **已实装** |
| 9. 交易选择器 χ=1[LCB>θ∧RiskOK∧ConflictOK], LCB=μ̂−c·se（p8-9） | 置信下界门 | LCB=`ChiFilterCtx{theta,z_alpha}`+`mu_estimator` LCB（**已实装**）；ConflictOK=`interp::interpret` ≺_Θ 冲突互斥（**已实装**）；RiskOK=`k_theta_risk_gate` 判定逻辑全，但**账户层输入退化**——maint_margin/buffer/liq_flag 喂占位 0/false，实际只判 equity≤0 | **部分**（codex-p2 订正）：LCB/ConflictOK 齐，但 **RiskOK 输入退化**（五态风控只 discharge Insolvent 子集，见 P0-2） |
| 10. 多空双开状态 σ_p；短差 σ_v=−σ_p；短差入场 δ=−σ_p 出场 δ=σ_p（p9） | 父声部方向 + 短差反向 | `coverage::Vertical::{Ambient,FollowParent,ShortDiff}`（ShortDiff=δ_g=−σ_{p(g)}）、`attach_bsp_to_tree`(σ_p=hostOf.attached_dir) | **已实装** |
| 11. 级别多空仓位后 alpha 定理：∃π E[X(π)]>0 ⟺ P(max_a μ>0)>0；不交易 a_0（p9-11） | 存在性定理 + a_0 | 无需代码（L0 定理）；不交易 = K_Θ 安全锚 0（`lex_argmin` 恒返 Some(0)） | **已实装（原则+锚 0）** |
| 12. 鞅不可能定理 E[ΔN(P_{t+1}−P)]=0（p10-11） | 数学限制 | 无需代码（L0 命题，指导 L3 否证解读） | **已实装（作为原则）** |
| 13. 最终策略形式 π=Schedule[LexArgmin_{p∈K} J(p,p̃)−p]；p̃=Σ_γΣ_a χ·q·e^side；选择器 χ（p11） | 完整 π 复合 | `coverage::pi_theta_step`（`pi_theta_position`=LexArgmin over K_Θ → `schedule_order(p*−p_t)`）+ ChiFilterCtx 过滤 Γ_t^trade | **已实装** |
| 14. 最终结论：必须引入级别/多空/角色/仓位（p11-12） | 结论 | 桶键扩维已兑现（#83） | **已实装** |

---

## B. PDF1 后半「缠论作为全互斥定义策略的形式化对象」（页 14–27，第一–十四）＝PDF2 页 1–12（逐字重复，同一判定）

| 节 | 原文要求（页码 PDF1 / PDF2） | 代码锚点 | 判定 |
|----|-----------------|----------|------|
| 一. 形式化对象 x=(h,D,A,p,W)；元素 e=(I_e,ℓ_e,ε_e,α_e,ρ_e)，ε_e(P_ρ−P_λ)>0；BSP⊆E；证书 γ=(c,ℓ,δ,I_γ,t)，成立 ⟺ I_γ≠∅（p14-16 / p1-2） | 元素树 + 买卖点证书 | `coverage::extract_elements`(元素树)、`CoverageElement{dir(ε_e),lo/hi,rho/lambda}`、`classifier::bsp::BspPoint{bits(I_γ),source_index(t),center}`、`interp::Candidate{level(ℓ),dir(δ),...}` | **已实装** |
| 二. 全互斥分类函数 Z=C_Θ(x)；X=⊔C⁻¹(z)（p16 / p2-3） | 分划唯一性 | `classifier`（递归级别 C_Θ）；分划严格证明见 partition-proof（#43/#54，L0） | **已实装**（分划证明 L0；有效域见 a1-v2） |
| 三. 买卖点操作语义：多头/空头声部 δ 语义；**短差子声部** σ_u=−σ_p；ShortDiffEntry ⟺ a_{p,t}=1∧δ=−σ_p；活动集 A_{t+1}=AncOK[(A\D)∪O]；AncOK(S)={v:Anc(v)⊆S}（p16-18 / p3-4） | 声部方向 + AncOK 祖先闭合 | `voice::{voice_side,dir_of_depth,flip}`（σ 交替）、`coverage::Vertical::ShortDiff`、`coverage::active_set_step`/`ancestor_close`(AncOK)、`persistent`（跨 bar 持久元素，anc.pdf §4-9 修复 LiveDetached 误剪） | **已实装** |
| 四. 全互斥解释器：可重叠谓词 P_1..P_8（风险强平/平多/平空/开短差/开多/开空/加仓/保持）；C_j=P_j∧⋀_{k<j}¬P_k；C_0 兜底；Σ1[C_j]=1；I_Θ=(D_t,O_t,L_t)（p18-19 / p4-6） | 8 谓词固定优先级互斥化 | **两个实装**：① `mutex::mutex_class`（P1..P8 精确对齐，穷举 2^8 证 Σ=1）**但无生产消费者**；② `interp::interpret`（≺_Θ 全序 fold → 三桶 D(close)/O(open)/L(record)，**已接线** pi_theta_step） | **部分**：语义三桶 D/O/L 已接线（interp）；但**PDF 指名的 P1..P8 谓词互斥化（mutex.rs）未被 π 消费**，两实装未统一（见 §缺口 P0-1） |
| 五. 策略定义：A_{t+1}=AncOK[(A\D)∪O]；q_v=Q_Θ(Z_t(v),x)；p̃=Σ_{v∈A}q_v e^{σ_v}；净 Ñ=Σσ_v q_v；风险可行集 K_Θ≠∅；p*=LexArgmin J；O=Schedule(p*−p_t)；π=Schedule[LexArgmin J−p]（p19-20 / p6） | 完整策略复合 | `coverage::pi_theta_step`：`coverage_step_classification`(A_{t+1}+p̃ via `net_target_units`=Σσq) → `pi_theta_position`(LexArgmin J over K_Θ) → `schedule_order`(O)。K_Θ=`KThetaRiskGate`+安全锚 0 | **已实装** |
| 六. 策略全定义证明（9 假设 ⟹ ∃!O）（p20 / p6-7） | 唯一性 | `plan_orders`/`pi_theta_step` 确定性（`ConflictKey` 全序、`lex_argmin` 键单射）；测试 `plan_orders_*_deterministic`、`given_theta_total_unique` | **已实装** |
| 七. alpha 定义 ΔN=N^Θ−N^0；ΔR=ΔN(P_{t+1}−P)−ΔC；E[ΔR]>0（p20-21 / p7） | 增量收益 | `runner::RunResult{equity_curve(ΔR/nav0),trade_pnls(已实现),trade_pnls_with_forced(含浮盈)}`；成本扣进 equity | **已实装** |
| 八. alpha 定理（同 A-11，充要条件）（p21-22 / p7-8） | 定理重述 | 同 A-11 | **已实装（原则）** |
| 九. 说明什么：语法不产 alpha，正条件净收益才产（p22 / p8-9） | 结论 | L3 否证线兑现（μ≤0 桶 χ 滤掉） | **已实装（原则）** |
| 十. 非拓扑度量 β/d/c/m（p22-23 / p9-10） | 力度/风险/成本/保证金 | β：MACD 面积（`classifier::divergence` 有 area，但**未进 μ̂ 桶键**，只二值）；d：`structural_stop`；c：`ResidualTrade.cost`；m：**未建模** | **部分**：β 只二值化未连续分桶；m（保证金）缺（见 §缺口 P1） |
| 十一. 细化优势定理 V(Z)≥V(Y)（p23 / p10） | oracle 单调（同 A-6） | 无实装对象（同 A-6） | **N/A**（codex-p2 改判：数学命题非实装对象） |
| 十二. 鞅不可能定理（同 A-12）（p24 / p11） | 数学限制 | 同 A-12 | **已实装（原则）** |
| 十三. 元素数量 \|E\|=O(n) + Eat(e) 命题（结构层，非 alpha 层）（p24-25 / p11-12） | 复杂度界 + 覆盖 | Eat(e)：`persistent`(祖先命期不变量 I1-I5) **已实装**；\|E\|=O(n) 复杂度界：`coverage` element-coverage 有实装但**无 O(n) 复杂度证明**（PDF 亦承认实测 n^1.26 是事件流非规范节点数） | **部分**（codex-p2 降级）：Eat(e) 已实装；**\|E\|=O(n) 界无复杂度证明** |
| 十四. 最终结论：Z_t 状态空间 + 动作唯一性由缠论给出，alpha=正条件净收益（p25 / p12） | 结论 | π 全定义（六）+ χ 选择器 | **已实装（原则）** |

---

## C. PDF2 后半（PDF1 无对应）

### C1. 背驰强度专题（PDF2 页 14–21，第 1–9 节）

| 节 | 原文要求（页码） | 代码锚点 | 判定 |
|----|-----------------|----------|------|
| 1. 背驰可严格定义成二值谓词 D_t^δ∈{0,1}；B_{i,ℓ}=Context∧D^+（p14-15） | 二值背驰足以定义买卖点 | `classifier::bsp::BspBits{buy1/2/3,sell1/2/3}`（二值位）、`divergence.rs`（Context∧背驰） | **已实装**（二值背驰） |
| 2. 二值背驰不等于赚钱；证书 γ=(ℓ,δ,i,c,t)；μ(z,a)；∃z,a μ>0 才赚（p15-16） | 赚钱=正条件收益 | `mu_estimator`（μ̂ per z）+ χ 门 | **已实装** |
| 3. 定理1：二值背驰产 alpha 充要条件（p16-17） | 定理（同 A-11 结构） | 同 A-11 | **已实装（原则）** |
| 4. 背驰强度不作定义买卖点必要条件，但可作状态细分变量 K；Z'=(Z,K)（p17-18） | 强度 K 细分 | **未实装**：`MuClass` 无背驰强度桶维（只二值 divergence bool） | **缺口 P1**：β 连续量未进 z 细分 |
| 5. 定理2：细化 V(Z')≥V(Z)；μ(D=1)≈0 但 μ(D=1,K=强)>0 ⟹ 强度是识别可交易子类的必要条件（p18-19） | 强度细分优势 | 同上，未实装强度桶 | **缺口 P1** |
| 6. 坚持背驰有/无仍能赚（若二值状态本身正收益）（p19） | 二值充分性 | 二值桶已有 | **已实装** |
| 7. 背驰有/无 vs 非拓扑量：M_t(s) 力度函数、D_t=1⟺TopContext∧M(s2)<M(s1)∧创新高/低（p20） | 力度比较内蕴 | `divergence.rs`（MACD 面积/斜率力度比较，产二值 D） | **已实装**（力度比较在二值判定内部；连续 M 未外露为桶维） |
| 8. 背驰赚钱=条件收益命题（p20-21） | 同 2 | 同 2 | **已实装** |
| 9. 最终结论：强度非语法必要，是识别 alpha 细化（p21） | 结论 | — | **部分**（细化路径未实装，见 4/5） |

### C2. 估计层 / winner's curse 专题（PDF2 页 22–37，第 1–18 节）

| 节 | 原文要求（页码） | 代码锚点 | 判定 |
|----|-----------------|----------|------|
| 序. 确定性结构层不过拟合，因果选择器 μ̂ 估计层会过拟合（p22） | 两层对象分离 | `classifier`(C_Θ 确定) vs `mu_estimator`(μ̂ 统计) 分离 | **已实装** |
| 1. C_Θ:x→z 确定；μ̂ 统计估计器；过拟合在 μ̂−μ（p22-23） | 两层对象 | 同上 | **已实装** |
| 2. 估计误差分解 μ̂=μ+ε, Var=σ²/n（p23-24） | 误差分解 | `mu_estimator`（se=σ/√n） | **已实装** |
| 3. Winner's curse 严格证明 E[max ε]>0（p24-25） | 多重选择偏差 | `prereg_windows`（预注册 + walk-forward OOS 防选择污染）、cross-fit（#42 注释） | **已实装** |
| 4. 稀疏类别更严重（n_z=1,2,3）（p25） | 稀疏 | shrinkage + LCB 抑制稀疏假赢家 | **已实装** |
| 5. 多重比较严格形式 1−(1−α)^K（p25-26） | FWER | LCB/Bonferroni（§9 c_α=z_{1−α/K}）；`chi_z_alpha` | **已实装** |
| 6. 过拟合是估计器问题非缠论问题（p26） | 自由度来自 z 选择+μ̂ 噪声 | 同序 | **已实装** |
| 7. 层级收缩数学解 μ̂_shrink（p26-27） | shrinkage | `run_theta_v0_pi_chi_shrink`、`pooling_icc` | **已实装** |
| 8. Shrinkage 的 MSE 解释（Bayes 最优）（p27-28） | MSE 最优 | 同 7（w=τ²/(τ²+σ²/n)） | **已实装** |
| 9. LCB 数学解 LCB=μ̂_shrink−c_α·ŝe（p28-29） | 置信下界 | `ChiFilterCtx`（LCB 门）+ shrink | **已实装** |
| 10. Shrinkage 与 LCB 互补非替代（p29） | 互补 | 两者都在（shrink 降方差 + LCB 控选择） | **已实装** |
| 11. 更本质解：限制有效维数 φ:Z→U 压缩映射（p29-30） | 维数控制 | `MuClass` 桶键=有限低维投影（level/δ/type/role/σ_p/H/ω，非全 z）——是一个有限投影 | **部分**（codex-p2 降级）：桶键是有限投影，但**未证它是 φ 的压缩最优选择**（PDF §11「不要把不能可靠估计的维度放进 z」需按维度增益/OOS value 逐维验证——当前是既定桶键，非经证明的最优压缩，声明「就是 φ 压缩」属声明膨胀） |
| 12. 正则化视角 L1/L2 β̂（p30） | 正则化 | 未见显式 L1/L2 回归模型（μ̂ 是分桶均值非参数回归） | **部分**：正则化经「分桶+shrinkage」实现，非显式 Lasso/Ridge（等价路径，非 PDF 字面 L1/L2） |
| 13. 高级别稀疏不可避免下界 T≳6.19/(λd²)（p30-31） | 信息论下界 | `prereg_windows`（功效/样本量约束）；跨标的 pooling（#86） | **已实装** |
| 14. Le Cam 两点论证 n d²≪1 不可区分（p31-32） | 不可能性 | L3 INCONCLUSIVE 判据（`mu_estimator` LCB≤0<UCB ⟹ 无检出力，对应 Le Cam） | **已实装（判据层）** |
| 15. 跨品种 pooling 严格条件 τ²=0 同分布（p32-33） | pooling 条件 | `pooling_icc`（跨品种 τ² 估计）、#86 多品种 L3 | **已实装** |
| 16. 检验 pooling：ICC=τ²/(τ²+σ²) + leave-one-asset-out（p33） | ICC 检验 | `pooling_icc::ICC`、跨标的 OOS（#86 wverify 多品种循环） | **已实装** |
| 17. 三问严格回答（过拟合来源/shrink+LCB 正确性/跨品种 pooling 可信性）（p33-34） | Q1-Q3 | 同序综合 | **已实装** |
| 18. 最终结论（p34-35） | 结论 | — | **已实装（原则）** |

---

## Q1 原文考古：单时刻动作空间 = 多候选并行 or 单动作？（决定 D1 重设计）

**结论：原文可判——多候选并行合规。Part B 四（候选/声部级裁决）与 五+十三（组合级持仓向量优化）是分层关系，不是矛盾。** 生产 `interp::interpret` 多候选并行是**合规**的。逐字依据：

**(a) Part B 四「全互斥解释器」（PDF1 页 18-19 / PDF2 页 4-6）**——P1..P8 是**逐冲突分类**，不是「每时刻裁决出唯一一个全局动作」：
- 原文：「同一时刻可能出现**多个买卖点、多个级别、多个声部冲突**。因此定义原始谓词 P_1,…,P_m。」→ 谓词是为「多个并存冲突」设的。
- 原文解释器输出：「`I_Θ(A_t, Γ_t) = (D_t, O_t, L_t)`……D_t = 要关闭的声部，O_t = 要开启的**声部**，L_t = 记录但不交易的**证书**。」→ **D_t/O_t/L_t 是集（多个声部/证书），不是单一动作**。故 `Σ_{j=0}^m 1[C_j]=1` 的互斥是**逐声部/逐候选**「每个冲突恰归一个动作类」，不是「整个 book 每 bar 只出一个动作」。

**(b) PDF1 十三 π=Schedule[LexArgmin J−p]（页 11）+ Part B 五（PDF1 页 19-20 / PDF2 页 6）**——LexArgmin 在**持仓向量空间**上选，内在允许多 (level,δ) slot 同时变动：
- 五：`A_{t+1}=AncOK[(A_t\D_t)∪O_t]`；`p̃_{t+1}=Σ_{v∈A_{t+1}} q_v·e^{σ_v}`——目标账本对**多个 active voice 求和**（多 slot 同时持仓）。
- 十三：`p̃_{t+1}=Σ_{γ∈Γ_t} Σ_{a∈A(Z_t(γ))} χ_t·q_Θ·e^{side(a)}`（对**所有候选 γ** 求和）；`π=Schedule_Θ[LexArgmin_{p∈K_Θ(x_t)} J_Θ(p,p̃)−p_t]`——LexArgmin 的变量是**持仓向量 p∈K_Θ**（可行持仓集），不是单一动作标量。故内在允许多 (level,δ) slot 并行变动。

**分层关系（非矛盾）**：四 = **候选级裁决**（每个候选/声部经 P1..P8 固定优先级归入 D/O/L 桶之一）→ 五/十三 = **组合级优化**（把逐候选桶归属聚合成目标账本 p̃，再 LexArgmin 在 K_Θ 上求唯一 p*，Schedule 成订单）。两层在原文里明确串接（四产 (D,O,L)，五消费 D/O 组装 p̃）。

**对 D1 的影响**：D1 **不需要**把 π 收敛为「单 bar 单动作」——多候选并行是原文规定。D1 的等价性证明范围收窄为**逐声部/逐候选级**：证 `interp::interpret` 的**逐候选 ≺_Θ 三桶归属** ⟺ `mutex::mutex_class` 的**逐候选 P1..P8 优先级归属**（对同一候选的冲突输入，两者归入同一 {close,open,record} 桶）。`mutex_class`（单 Predicates→单 C_j）天然是**逐候选**粒度，与此收窄一致。残留裁定点仍是：≺_Θ 的候选间排序是否与 P1..P8 谓词优先级在**同候选多谓词命中**时一致（如一候选同时满足「平多」与「加仓」时，两实装是否都取 P2 平多）。

---

## 缺口分级

### P0（策略闭环必需——无它 π 语义不完整或有双实装漂移风险）

- **P0-1｜8 谓词全互斥解释器双实装未统一**（PDF Part B 四）。
  - 现状：`mutex::mutex_class`（精确 P1..P8 固定优先级，L0 穷举证 Σ1[C_j]=1）**无任何生产消费者**（grep 确认：仅 interp.rs 注释引用，无 `mutex_class(` 调用点）；被 π 消费的是 `interp::interpret`（≺_Θ 全序三桶）。
  - 矛盾性质：**不是概念矛盾，是双实装**。interp 的三桶 D/O/L 与 mutex 的 C_j 都对齐 PDF「I_Θ=(D_t,O_t,L_t)」，但 interp 用「候选平移不变全序 ≺_Θ」裁决，mutex 用「谓词固定优先级 P1≻…≻P8」裁决。PDF 四**明确用 P1..P8 谓词优先级**（风险强平≻平多≻平空≻开短差≻开多≻开空≻加仓≻保持）。当前接线的 interp **不保证**与这个谓词优先级序等价——尤其「风险强平 P1 最高」在 interp 里由 `KThetaRiskGate.force_flat` 单独兜（K_Θ 收窄），「平仓先于开仓」由 `ConflictKey.exit_first` 兜，是**分散实现**，未证与 mutex 单一优先级链等价。
  - 为何 P0：若 interp 的 ≺_Θ 序与 PDF P1..P8 优先级在某状态分叉，则 π 在冲突时选的动作 ≠ PDF 定义的动作——策略语义不合规。需**证等价**（interp 三桶结果 == mutex_class 派生动作）或**统一为单实装**。

- **P0-2｜RiskMode μ_t 五态退化为 equity≤0**（PDF Part B 五 K_Θ / A-9 RiskOK）。
  - 现状：`risk::risk_mode` 定义五态（Insolvent/Liquidation/Deleverage/CloseOnly/Normal），但 `exit::exit_decision_for` 与 `k_theta_risk_gate` 喂入 `maint_margin=0,buffer=0,liq_flag=false` 占位 ⟹ μ_t 实际只 discharge `equity≤0`（Insolvent 子集）。Liquidation/Deleverage/CloseOnly 触发条件（维持保证金/缓冲/强平旗）**未建模**。
  - 为何 P0：PDF「K_Θ≠∅ + RiskClose」是策略闭环的风险可行集约束。风险层只有 equity≤0 ⟹ 杠杆/保证金驱动的强平（真实盘中最常见的爆仓）不触发 ⟹ π 的 K_Θ 在真实约束下不完整。与 memory project_gap3_l2_unreachable（TW 账本接价格无关事件流）同源——账户层输入流缺口。

### P1（增强——细分优势定理意义上提升 alpha 识别力，非 well-definedness 必需）

- **P1-1｜背驰强度 β 连续量未进 z 细分**（PDF1 十 + PDF2 §4/§5）。当前只二值背驰（BspBits）。PDF2 明证「μ(D=1)≈0 但 μ(D=1,K=强)>0」时强度是识别可交易子类的必要条件。`divergence.rs` 已算 MACD 面积（连续力度），但未外露为 `MuClass` 桶维。接入 = 桶键加 K∈{弱,中,强} 分箱。
- **P1-2｜κ 中枢位置细粒度、d/c/m 状态维**（PDF1 一 + 十）。`MuClass.horizontal` 是 κ 粗桶；d（止损距离）/c（成本）已用于 sizing/risk 但非 μ̂ 桶维；m（保证金）完全未建模。d/c 是否需进桶键取决于是否影响 μ 符号（PDF 把它们放 z 主要为 sizing 与 Y=X/R 归一，非分类）——**非必需**，标注为可选增强。
- **P1-3｜显式正则化回归缺**（PDF2 §12）。当前 μ̂=分桶均值 + shrinkage，无显式 L1/L2 β̂ 回归。分桶+收缩是等价的维数控制路径，PDF §11 亦承认「限制有效维数」是更本质解——故 L1/L2 是**可选替代**，非缺口。

---

## p2 实装设计草案（P0 优先，复用存量，供 codex 审计）

### 设计 D1：统一 8 谓词解释器（P0-1）——**证等价，不重写**

- **不新增第三实装**（no-patch）。方案：为 `interp::interpret` 的三桶输出补一个**桥接函数** `mutex_class_of_moment(gamma_x, active) -> MutexClass`，把当前时刻的（风险门 + 候选 + 活动集）投影为 `mutex::Predicates`，断言 `interp::interpret` 的分桶结果 ⟺ `mutex_class` 派生的动作。
  - `P1 risk_liquidate` ← `KThetaRiskGate.force_flat`；`P2 close_long/P3 close_short` ← D 桶中反向平仓腿；`P4 open_short_diff` ← O 桶中 Vertical::ShortDiff 候选；`P5 open_long/P6 open_short` ← O 桶 Ambient/FollowParent 候选按 δ；`P7 same_dir_record_add` ← L 桶或同向加仓；`P8 hold` ← 空动作。
  - 交付一个 property test：对随机 (gamma_x, active, gate)，`interp` 三桶归属的**动作** == `mutex_class` 派生动作。**若不等价 ⟹ 暴露真矛盾（走 escalate），不 workaround**。
- 最小 diff：新增 `strategy/mutex.rs::bridge_from_interp`（~40 行）+ 一个 test。不改 interp/coverage 主链（若等价证成立，接线现状即合规，mutex.rs 从「未接线」升为「等价见证」）。
- **风险/裁定点**：≺_Θ 全序与 P1..P8 优先级是否真等价，是 codex 审计焦点。若发现分叉（如 ≺_Θ 在同级多候选时的裁决 ≠ 谓词优先级），需编排者裁定「以哪个为准」（PDF 四写的是谓词优先级 ⟹ 倾向 mutex 为规范，interp 需对齐）。

### 设计 D2：RiskMode 账户层输入接线（P0-2）——**补输入流，不改判定逻辑**

- `risk::risk_mode` 判定逻辑已全（五态齐），缺的是**真实 maint_margin/buffer/liq_flag 输入**。方案：在 `AccountState`（或 runner 的 equity 循环）补 `maint_margin`/`margin_buffer` 字段，由 sizing 的名义仓位 × 保证金率派生（`m` 参数，PDF z 的第 11 维）。`liq_flag` 由 venue（生产）或模拟撮合（回测）产。
- 最小 diff：`AccountState` 加 2 字段 + `k_theta_risk_gate`/`exit_decision_for` 用真值替占位。与 memory project_gap3_earning_shares（RiskPolicy.kappa≠RiskConfig.kappa）对齐——账户层输入是 GAP3 线共同缺口，可与 TW 账本注资流合并设计。
- **诚实边界**：这引入 `m`（保证金参数），把 P1-2 的 m 维一并补上。回测侧需一个保证金率 config（Θ_risk 参数，非缠论可导——明确标注）。

### 设计 D3（P1，可后置）：β 强度分箱进桶键

- `MuClass` 加 `divergence_strength: Option<u8>`（弱/中/强，从 `divergence.rs` 的 MACD 面积比分箱）。桶键扩维 = 复用 #83 的扩维模式（role/σ_p 已进）。walk-forward μ̂ 自动按新桶估。**先做 P0，此项待 P0 收口后按细分优势定理增益评估再接**。

---

## 结果包六要素

1. **结论**：完整策略 π（声部/AncOK/净账本/LexArgmin/K_Θ/χ 选择器）**已实装且被回测消费**（`run_theta_v0_pi_inner`→`coverage::pi_theta_step`）；订正 goal 前提「只有逐信号测量」。逐节判定（共 56 节/行，含 codex-p2 四项修订）：PDF1 前半 14 节（11 已实装/2 部分/1 N/A）、后半 14 节（10 已实装/3 部分/1 N/A）、PDF2 背驰强度 9 节（6 已实装/1 部分/2 缺口P1）、估计层 19 行（17 已实装/2 部分）。合计 **44 已实装 / 8 部分 / 2 缺口 / 2 N/A**；跨认识论轴另计 **2 个 P0 + 3 个 P1**（P0：P0-1=后半四解释器双实装、P0-2=A-9/K_Θ 风控输入退化）。Q1 原文考古已判：**多候选并行合规**（四=候选级裁决，五/十三=组合级持仓向量优化，分层非矛盾）。
2. **定义依据**：逐节表引 PDF 页码/节号 + 代码锚点（文件:符号），三态判定按「构造存在 ∧ 被 π 消费」= 已实装、「存在但未接线/退化」= 部分、「无构造」= 缺口。
3. **边界条件**：判定翻转条件——(a) 若 `interp::interpret` ≺_Θ 序被证 ≠ PDF P1..P8 谓词优先级，则 P0-1 从「双实装」升为「合规矛盾」（interp 接线不合 PDF）；(b) 若账户层保证金输入接入，P0-2 与 P1-2(m) 同时闭合；(c) 本报告 L0/L1，若 L3 重测证 β 强度桶无增益，P1-1 可判为无需实装。
4. **下游推论**：p2 应先做 D1（证等价/统一解释器）+ D2（RiskMode 输入流），二者是 π 语义合规的必需；D3 后置。D2 与 GAP3 账本注资流（memory project_gap3_earning_shares）应合并设计，共享账户层输入缺口。
5. **谱系引用**：GAP3 线（project_gap3_l2_unreachable_architecture、project_gap3_earning_shares_reachable——账户层输入缺口同源）；#78（StructBreak 门拒=C_0/L_t 记录不交易，已结算，佐证「每可达 z 有定义动作」）；#83（role/σ_p 进桶键，佐证 A-1 部分实装的扩维模式）；#43/#54（partition-proof，佐证 B-二分划唯一）；project_wverify_alpha_retest_pass / project_stheta_v1_fullwindow_l3_falsified（alpha L3 线，本报告不重复其 L2/L3 判定）。
6. **影响声明**：纯只读审计，未改任何代码/git。产出本报告文件一份。下游：p2 实装工位消费 D1/D2/D3 设计；codex 审计消费 P0 缺口分级 + 等价性裁定点。
