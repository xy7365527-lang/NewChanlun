# formal-chain 硬判据清单——entry/持仓/exit 合法性（供亏损逐笔重放机器核验）

**工位**：swarm/ws-critextract ｜ topo_address: swarm/ws-critextract ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix
**提取源**：`docs/formal-chain/` PDF（编排者钦定「最严格起点的形式化」，**项目形式化唯一权威**，严格性高于缠师博文——090号 formal-chain 为准）。亲读页码逐条标注。
**认识论等级约定**（231号）：本清单是判据**转写**（L0 定义层），非 alpha 声明。判据的经验有效性（满足即盈利）属 L2/L3，全 INCONCLUSIVE，不在本清单范围。

---

## 〇、可测性总原则（决定 bug 判据 vs 理论边界判据）

`关于背驰.pdf` §4 + `经济正条件.pdf` §3/§8/§10 联合确立的核心分野：

> **结构序关系 `ε(P_ρ−P_λ)>0` 是 ex-post（走势段端点 λ/ρ 确认后才知），不能推出因果成交正收益 `ε(P_τout−P_τin)>0`（τ_in≥λ, τ_out≥ρ 确认滞后）。**

由此，判据分两类，**这是 entry 时刻可测性列的判定依据**：

| 类别 | 定义 | 违反后果分类 | 核验方式 |
|------|------|------|---------|
| **ex-ante 可测** | entry 决策时刻输入已全部确定，判据可当场核验 | **违反=实装 bug**（语法非法交易被放行 π_Θ 全函数性破裂） | 逐笔重放当场断言 |
| **ex-post 才可判** | 判据依赖走势段/中枢**完成**（未来端点、确认滞后、力度比较段收尾） | **满足仍亏=理论边界候选**（缠论语法⇒端点正价差 ⇏ 成交正收益，需 CEP 经济公理补） | 只能事后归因，不作 bug |

**关键**：确认-bar 部署下（proofs D-3 CandType Live 轴），所有信号在**确认 bar** 触发，其结构判据（Rel/Role/AncOK/P1..P10/LexArgmin）在该 bar 已 ex-ante 可测；而**区间套 χ 的 Conf 终端确认、背驰的 Extreme/Comparable、经济正收益**依赖走势段收尾，属 ex-post。

---

## 一、C 系列——entry 合法性硬判据

### C1｜六类买卖点信号向量全定义单值
- **形式化条件**（`买卖点.pdf` p1）：`b_ℓ(x)=(B_{1,ℓ},B_{2,ℓ},B_{3,ℓ},S_{1,ℓ},S_{2,ℓ},S_{3,ℓ})∈{0,1}^6`，每谓词 `B_{i,ℓ}(x),S_{i,ℓ}(x)∈{0,1}` 全定义、单值、可判定、因果；`Σ_{u∈{0,1}^6} 1[b_ℓ(x)=u]=1`。不强制互斥（买卖点可重合，作为不同语法状态）。
- **entry 可测性**：**ex-ante 可测**（确认 bar 上六 bit 由结构塔确定）。
- **实装位置**（源 proofs-full-strategy-20260703.md 假设4，重放前须 Read 重验行号）：`rust/src/theta_v0/classifier/six_state.rs:155` `six_state_exhaustive_and_bot_dimension`；6-bit `BspBits` 掩码 64 类不压扁 `classifier/bsp.rs`。
- **违反后果分类**：违反=**实装 bug**（漏分类/双分类 ⟹ b_ℓ 非单值 ⟹ 语法状态歧义）。

### C2｜区间套方向确认证书递归全定义
- **形式化条件**（`买卖点.pdf` p2）：声部 v 操作级别 o_v ≥ 执行级别 e_v，方向 δ∈{+1,−1}：
  `χ_v^δ(x) = N^δ_{o_v↓e_v}(x) ∈ {0,1}`，递归 `N^δ_{ℓ↓e}(x) = Conf^δ_e(x)` (ℓ=e) / `Cand^δ_ℓ(x) ∧ [J^δ_{ℓ−1}(x)⊆J^δ_ℓ(x)] ∧ N^δ_{ℓ−1↓e}(x)` (ℓ>e)。终端确认 `Conf^+_e=⋁_{i=1}^3 B_{i,e}`、`Conf^−_e=⋁_{i=1}^3 S_{i,e}`。区间包含**非端点相等**（闭口径 `child.lo≥parent.lo ∧ child.hi≤parent.hi`）。级别链有限 `o_v>o_v−1>…>e_v` ⟹ 递归有限终止 ⟹ `∀v,δ,x, ∃!χ_v^δ∈{0,1}`。
- **entry 可测性**：**ex-post 才可判**（Conf 终端确认依赖执行级别走势段完成；J^δ 区间需子级别走势收尾定界）。
- **实装位置**：`rust/src/theta_v0/classifier/nest.rs:200` `NestCertificate::n_delta`；`nest.rs:205` `n_delta_rec`；`nest.rs:62` `is_sub`（子⊆父闭口径）；L2 parity `backtest/econ_positive.rs:4103` `acc_bottomup_nest_parity_probe`。
- **违反后果分类**：结构形式违反（is_sub 口径错/递归未终止）=**实装 bug**；χ=1 仍亏=**理论边界候选**（区间套定位对但成交滞后，见 X-econ）。**L2 事实**：BTC 全历史 95% 退化深度=1，深度≥2 几乎为 0（市场几何，非 bug）。

### C3｜级别关系三分互斥穷尽
- **形式化条件**（`买卖点.pdf` p2-3）：父声部选择器 `α_x(g)=`包含信号 g 的最深活动声部（无则 ∅）。`Rel_x(g)=Root` (α=∅) / `Same` (α≠∅ ∧ ℓ_g=ℓ_{α}) / `Sub` (α≠∅ ∧ ℓ_g<ℓ_{α})。**不允许模糊的"差不多次级别"，必须严格 `ℓ_g<ℓ_{α}`**。`Σ_{R∈{Root,Same,Sub}} 1[Rel_x(g)=R]=1`。
- **entry 可测性**：**ex-ante 可测**（活动声部集与信号级别在决策时刻已知）。
- **实装位置**：proofs 未单列独立锚点（并入 interp Role 判定）——**实装位置待核**（重放工位 grep `Rel`/`parent_voice`/`alpha_selector` 于 `strategy/interp.rs` 与 `strategy/coverage.rs`）。
- **违反后果分类**：违反=**实装 bug**（Rel 非单值 ⟹ Role 判定歧义 ⟹ 操作方向错）。

### C4｜操作角色五分互斥（做多做空 vs 短差严格区分）
- **形式化条件**（`买卖点.pdf` p3-4, p14 §15）：`Role_x(g)∈{RootDir,SameFollow,SameReverse,SubFollow,ShortDiff}`：
  - `RootDir` ⟺ Rel=Root
  - `SameFollow` ⟺ Rel=Same ∧ δ_g=σ_{α}
  - `SameReverse` ⟺ Rel=Same ∧ δ_g=−σ_{α}
  - `SubFollow` ⟺ Rel=Sub ∧ δ_g=σ_{α}
  - `ShortDiff` ⟺ Rel=Sub ∧ δ_g=−σ_{α}
  `Σ_R 1[Role_x(g)=R]=1`。**做多做空=绝对方向 δ_g；短差=相对父声部的次级别反向操作**（`短差 ⟺ Sub ∧ δ=−σ_parent`，非固定等于做空）。
- **entry 可测性**：**ex-ante 可测**（Rel + δ_g + σ_α 决策时刻已知）。
- **实装位置**：`rust/src/theta_v0/strategy/interp.rs:188` `reverse_exit_type`（ShortDiff 腿↔P7）；Role 完整判定**实装位置待核**（grep `Role`/`ShortDiff`/`SubFollow` 于 `strategy/`）。
- **违反后果分类**：违反=**实装 bug**（把次级别短差 ShortDiff 当本级别做多做空 ⟹ 在主升浪回调腿砍核心 = 踏空，bsp-opsem 报告 §五踏空根因④）。

### C5｜背驰结构候选三条件合取（Cand^δ = StructEligible^δ）
- **形式化条件**（`关于背驰.pdf` p10 §9.1，**严格订正版**）：
  `Cand^δ_ℓ = StructEligible^δ_ℓ(s',s) := dir(s)=−δ ∧ Comparable_ℓ(s',s) ∧ Extreme^δ_ℓ(s',s)`
  **三条件全为结构条件**：
  - **Dir**：背驰段方向 `dir(s)=−δ`（买入方向 δ=+1 ⟹ 判下跌段衰竭 dir(s)=−1）
  - **Comparable**（§4.2 同一上级趋势语境）：`∃ 上级容器 C, s',s⊑C ∧ s' 与 s 是 C 中可比的前后同向推进段` ⟹ `Comparable_ℓ(s',s)=1`。**排除"拿两个完全不在一个结构语境里的走势段比较力度"**。
  - **Extreme**（§4.3 价格推进）：买入 δ=+1 ⟹ `minP(s)<minP(s')`；卖出 δ=−1 ⟹ `maxP(s)>maxP(s')`。
- **★关键订正**（相对任务描述「四条件合取含 Weak」）：**力度弱化 Weak 不进 Cand 门**。`关于背驰.pdf` §9.1 明文：`Cand^δ_ℓ = StructEligible^δ_ℓ` **而不是** `Cand = MACDdiv`。力度状态 `ForceState_ℓ(s',s)∈{Dominated,Dominates,Incomparable,Tie}` 进入状态向量 `Z=(ℓ,δ,I_γ,ForceState,MACD,DIF,Amplitude,Speed,…)` 交 `μ(z,a)` 或选择器决定——**这样没有候选会因为 MACD 被提前删除**（宽结构候选 = 超集，力度在选择层收窄非在候选门一票否决）。
- **entry 可测性**：**ex-post 才可判**（Comparable 需上级容器成型；Extreme 需比较段 s 完成才知 minP/maxP）。
- **实装位置**：`rust/src/theta_v0/classifier/divergence.rs:358` `ForceProxies::force_state → ForceStateA4`；`divergence.rs:340` 四态枚举；`divergence.rs:474` `weak_theta`。
- **违反后果分类**：三结构条件违反=**实装 bug**（非可比段被判背驰）；Cand 满足仍亏=**理论边界候选**（背驰是走势完成的必要非充分，located≠done）。

### C6｜力度弱化支配序（无 MACD 一票否决，三值判定）
- **形式化条件**（`关于背驰.pdf` p2-3 §2, p5-6 §5-6, p8 §8）：完整力度=递归签名 `𝔉_ℓ(s)`（非标量）。最低级 `𝔉_0(s)=(|ΔP_s|,T_s,|ΔP_s|/T_s,TV_s,Vol_s,MACD_s,DIF_s)`；高级 `𝔉_ℓ(s)=(|ΔP_s|,T_s,TV_s,(𝔉_{ℓ−1}(u_1),…,𝔉_{ℓ−1}(u_m)),PathShape_ℓ(s),MACD_s,DIF_s)`。允许力度族 `𝒜_ℓ`（每个 `m:𝒮_ℓ→ℝ_{≥0}`：|ΔP|、|ΔP|/T、TV、MACDarea、DIFdistance、Σ次级别同向段…）。**支配序**：`s ≺_𝒜 s' ⟺ ∀m∈𝒜_ℓ, m(s)≤m(s') ∧ ∃m, m(s)<m(s')`（当前段在所有允许力度维度上都不强于前段，且至少一个更弱）。判定三值 `D^δ_ℓ(s,s')∈{Yes,No,Undetermined}`：Weak（所有口径一致弱化）/NotWeak（有或关键口径不弱）/Incomparable（口径冲突）。
- **entry 可测性**：**ex-post 才可判**（需比较段 s 完成计算全部力度维度）。
- **实装位置**：`rust/src/theta_v0/classifier/divergence.rs:303` `ForceFeatures`；`divergence.rs:474` `weak_theta`（预注册 Θ 词典序比较，三套冻结 `Θ_DOM/Θ_LEX/Θ_SCORE`）。
- **违反后果分类**：用 MACD 面积单判据一票否决 = **实装 bug**（`M_MACD` 不是 𝔉_ℓ 的充分统计量，`关于背驰.pdf` §3 定理）；Incomparable 强判 Weak/NotWeak = 语法非法（须交预注册 Θ 或标 Undetermined）。

### C7｜安全可行集非空且有限（K_Θ≠∅）
- **形式化条件**（`买卖点.pdf` p9 §12）：`K_Θ(x)⊆𝒫^sep`，含手数约束/同股数双开约束/祖先闭合约束/杠杆约束/保证金约束/短差可执行性/三阶段资本约束/订单执行约束。要求 `K_Θ(x)≠∅`。
- **entry 可测性**：**ex-ante 可测**（约束在决策时刻可核）。
- **实装位置**：`rust/src/theta_v0/strategy/coverage.rs:2155` `feasible_candidates`（含安全锚 `0.0` 构造性非空见证）；`coverage.rs:2105` `KThetaRiskGate`。
- **违反后果分类**：`K_Θ=∅` = **实装 bug**（π_Θ 全函数性破裂，无可行订单）。安全锚 0 保证恒非空。

### C8｜最终目标头寸唯一（LexArgmin 固定平局）
- **形式化条件**（`买卖点.pdf` p9-10 §12）：`J_x(p)=‖p−p̃_{t+1}‖²_W + λ·Cost_x(p) + ν·RiskPenalty_x(p)`，`p*_{t+1}=LexArgmin_{p∈K_Θ(x)} J_x(p)`。字典序键单射 ⟹ 平局由固定规则（末键 grid_index 升序）裁决 ⟹ p* 唯一。
- **entry 可测性**：**ex-ante 可测**。
- **实装位置**：`rust/src/theta_v0/strategy/intent.rs:197` `JThetaKey`；`intent.rs:214` `lex_le`；`intent.rs:270` `lex_argmin`。
- **违反后果分类**：键非单射/平局非确定 = **实装 bug**（p* 非唯一 ⟹ ∃! 订单破裂）。

---

## 二、H 系列——持仓合法性硬判据

### H1｜祖先闭合活动集（AncOK：子声部存在则父声部必存在）
- **形式化条件**（`买卖点.pdf` p9 §10-11；proofs 假设7）：`AncOK(A)={e∈A : Anc(e)⊆A}`；活动集递归 `A_{t+1}=AncOK[(A_t∖D_x)∪B_x]`（先平后开+祖先闭合）。不变量 `e∈A_{t+1} ⟹ Anc(e)⊆A_{t+1}`。生命期包含 `∀a∈Anc(e), [λ_e,ρ_e]⊆[λ_a,ρ_a]`。
- **持仓可测性**：**ex-ante 可测**（活动集在每 bar 可核）。
- **实装位置**：`rust/src/theta_v0/strategy/coverage.rs:740` `active_set_step`；`coverage.rs:698` `ancestor_close_by_id`；不变量 I1-I5 `strategy/mod.rs:57-58`。
- **违反后果分类**：违反=**实装 bug**（子声部漂浮在已结束父上；Q4「LiveDetached 误判 Stale ⟹ depth>0 腿被系统性剪掉」修复史）。**已知加固缺口**（proofs §D 加固清单4）：「该剪未剪」方向（去激活滞后，元素过 ρ 仍留 A_t）无 property test 看守。

### H2｜三阶段资金单向迁移 + TW 事件合法性（OQ-9 门）
- **形式化条件**（proofs 假设8/§C.5）：`TStage∈{CostReduction→CapitalRecovered→EarningShares}`（单向）；TW 事件 `{ShortDiff,RecoverCapital,Withdraw,EnterEarning,BuyCore,Realize(d_pi)}` 经 OQ-9 门在当前 stage 合法性判定。**非 Realize 七构造子逐事件保 `tw()=free+holding+withdrawn` 守恒；`Realize(d_pi)` 是唯一 TW 漂移构造子**（d_pi 可正可负）。stage 驱动字段**白名单**（free/holding/withdrawn/notional_in/open_legacy_legs/risk_mode）/**黑名单**（hwm_gain/当前 MTM equity/forced_pnl/未平仓浮盈）。
- **持仓可测性**：**ex-ante 可测**（stage 状态机 + OQ-9 门在决策时刻可核）；但 EarningShares **可达性**分域：L0 同价无盈亏不可达 / L1 生产可达（已实现利润经 Realize 入 free）。
- **实装位置**：`rust/src/theta_v0/closed_loop/transition.rs:319` `stage_progression`；`transition.rs:268` `Oq9Illegal`；`strategy/ledger.rs:364` `TwEvent::Realize`；白名单硬边界 `transition.rs:309-316`。
- **违反后果分类**：OQ-9 非法事件被放行（如 EarningShares 阶段开 legacy 腿）=**实装 bug**；黑名单字段（hwm_gain 等未实现浮盈）驱动 stage =**实装 bug**（利润棘轮，codex A' 裁定边界条件 b）。

### H3｜毛头寸约束（gross exposure cap，条件激活）
- **形式化条件**（proofs §D-7）：`K_Θ` 含最大毛头寸约束，strict §11 毛杠杆 `L^G≤L̄^G`（单标的 units 空间 `base_units=E/px` 精确等价）。缩放语义：legs 折叠成净标量**之前**逐根子树 KKT water-filling 投影 `c_r=max(0,1−μ/t_r)`。
- **持仓可测性**：**ex-ante 可测**。
- **实装位置**：`rust/src/theta_v0/config.rs:162` `RiskConfig.enforce_gross_cap`（default false）；`strategy/risk.rs:563` `gross_units_cap`；`strategy/coverage.rs:1329` `apply_gross_cap`。
- **违反后果分类**：default false ⟹ **当前生产无毛 cap（诚实声明，非 bug——#122「约束未配置=不激活」）**；**#135 π^full 收口跑批须显式 `enforce_gross_cap=true`，否则 §11 一致性声明不成立**。激活后违反=实装 bug。

---

## 三、X 系列——exit 合法性硬判据

### X1｜typed 出场五枚举全定义
- **形式化条件**（proofs §D-4，PDF §9）：`ExitType∈{CloseRoot,ReduceCore,CloseShortDiff,RiskExit,Hold}`。归因单源 `reverse_exit_type`：ShortDiff 腿→P7 / 触发类3→P6 / 一二类→P5（二类归 CloseRoot）。**统计层出场用 τ^typed 非 τ^reverse**（τ^reverse 平行简化状态机整体删除，不留 fallback）。
- **exit 可测性**：**ex-ante 可测**（出场类型由触发候选角色确定）。
- **实装位置**：`rust/src/theta_v0/strategy/interp.rs:162` `ExitType`；`interp.rs:188` `reverse_exit_type`；`interp.rs:1004` `interpret_with_close_triggers`（close 桶第 k 腿↔第 k 触发候选 fold 内同步）。
- **违反后果分类**：用 τ^reverse 简化状态机 = **实装 bug**（proofs D-4，#134 已删除）；close 桶归因错配 = 实装 bug。
- **★μ 样本坍缩语义**（proofs D-4 诚实必读，非缺口）：μ 观测对象=生产 π 真正开腿的信号（record 桶/slot 冲突/AncOK 被剪候选不再入 μ），样本量降约两个数量级。旧口径 μ̂/W-VERIFY/L3 数字失效为历史证据。

### X2｜解释器固定优先级 P1..P10 全序无二义
- **形式化条件**（`买卖点.pdf` p7-8 §8-9；proofs §D-5）：候选按固定全序 `≺_Θ` 裁决。原始谓词（可重叠）P1..P10：
  - `P1`：强平或全局风险退出
  - `P2`：有未完成订单需处理（CloseOverlay）
  - `P3`：同级别反向信号（Withdraw）
  - `P4`：短差子声部需要平仓（EnterEarning）
  - `P5`：根方向开仓信号（CloseRoot）
  - `P6`：可执行短差开仓信号（ReduceCore）
  - `P7`：次级别顺向信号（CloseShortDiff）
  - `P8`：有信号但不可执行（Open Root）
  - `P9`：三阶段资本动作（Open ShortDiff）
  - `P10`：保持（Record）
  互斥化 `A_1=P_1, A_i=P_i∧⋀_{j<i}¬P_j, A_0=⋀¬P_j` ⟹ `Σ_{i=0}^m 1[A_i(x)]=1`（取最小成立索引 r）。生产两级结构：bar 级 P1..P4 组合层 + 候选级 P5..P10 fold。
- **exit 可测性**：**ex-ante 可测**（谓词向量在决策 bar 可核）。
- **实装位置**：`rust/src/theta_v0/strategy/interp.rs:990` `interpret`；`strategy/mutex.rs:143` `mutex_class`；`mutex.rs:296` `mutex_total_exhaustive_2pow10`（2^10 穷举）；bar 级 P1..P4 `coverage.rs:2449/2472/2510`；生产分支测试 `coverage.rs:4217/4278/4327/4377`。
- **违反后果分类**：`Σ≠1`（漏裁/双裁）=**实装 bug**；interp `≺_Θ` 序 ≠ mutex P1..P10 序 = 实装 bug（`shadow_fold_bucket_equivalence` 12 场景看守）。

### X3｜分账本每元素被对应腿覆盖（G_e^sep>0）
- **形式化条件**（`买卖点.pdf` p11-13 §11/§16）：目标头寸 `p̃_{t+1}=Σ_{e∈A_{t+1}} Leg(e)`，`Leg(e)=s_e·e^+_{ν(e)}` (ε_e=+1) / `s_e·e^−_{ν(e)}` (ε_e=−1)。分账本毛收益 `G_e^sep=s_e·ε_e·(P_{ρ_e}−P_{λ_e})`；由 `ε_e(P_{ρ_e}−P_{λ_e})>0 ∧ s_e>0` ⟹ `G_e^sep>0`。
- **exit 可测性**：**ex-post 才可判**（`G_e^sep>0` 依赖元素方向端点价差，即走势段完成后的结构序关系）。
- **实装位置**：`rust/src/theta_v0/backtest/runner.rs:1165` `TypedTrade`；`runner.rs:1044` `typed_ledger_from_bars`。
- **违反后果分类**：结构毛收益 `G_e^sep≤0` 于合法元素 = **实装 bug**（元素方向定义违反）；`G_e^sep>0` 仍实际亏 = **理论边界候选**（结构毛收益 ≠ 因果净收益，见 X-econ）。

### X-econ｜因果净收益正条件（理论边界闸门，非 bug 判据）
- **形式化条件**（`经济正条件.pdf` p2-8）：结构序 `ε_b(P_{ρ_b}−P_{λ_b})>0` ⇏ 因果成交正收益。真实收益 `X_b^causal=q_b·ε_b·(P_{τ_out}−P_{τ_in})−C_b`（τ_in≥λ_b, τ_out≥ρ_b 确认滞后）。**正收益需下列任一额外条件**：
  - **强公理版**（Endpoint Execution Axiom）：底分型端点买入/顶分型端点卖出 ∧ `C_b<q_b·ε_b·(P_{ρ_b}−P_{λ_b})` ⟹ `X_b>0`
  - **弱公理版**（工程化）：`端点价差 A_b > 确认滞后损失 η_in + 滑点 η_out + 交易成本`，即 `q_b(A_b−η_in−η_out)>C_b` ⟹ `X_b^causal>0`
  - **统计版**（CEP 公理）：`𝔼[q_e·ε_e·(P_{τ_out}−P_{τ_in})−C_e | Z_t=z] ≥ η_z>0` ⟹ alpha 定理 `𝔼[R]>0`
- **exit 可测性**：**ex-post 才可判**（τ_out 出场后才知实际成交价）。
- **实装位置**：成本函数 `strategy/exec.rs:73` `apply_fees`；方向感知 PnL `backtest/metrics.rs` `trade_abs_pnl`。滑点/确认滞后模型 η **实装位置待核**（重放工位 grep `slippage`/`eta_in`/`eta_out`/`confirm_lag`）。
- **违反后果分类**：**这是理论边界判据，不是 bug 判据**。缠论语法 ⇒ 端点正价差（C1-C8 全满足）后仍亏 ⟹ 落在「结构对但成交滞后/成本超端点价差」的经济边界，须由 CEP 经济公理（L2/L3 未决，全 INCONCLUSIVE）解释，**不修实装**。**亏损逐笔重放的核心分诊闸门**：先核 C/H/X 系列 ex-ante 判据（有违反=bug 先修），全通过再落此闸门（=理论边界，归 μ/选择器/成本模型）。

---

## 四、与缠师博文层判据的对照声明（formal-chain 为准，090号）

> deep-research 博文层报告（第37课趋势背驰前提/第53课三类硬条件）产出文件尚未落地（仅 prompt `.deep-research-prompt-20260704.txt`）。以下对照依据博文层结构（第17/19/20/21/24/37课，经 `bsp-opsem-classification-20260624.md` 提取）与 formal-chain 严格化关系。

| 博文层判据 | formal-chain 严格化/修改 | 判据编号 |
|-----------|------------------------|---------|
| 第20课中枢=次级别走势重叠 [ZD,ZG] | 形式化为区间套 `J^δ_{ℓ−1}⊆J^δ_ℓ` 闭口径包含（非端点相等） | C2 |
| 第21课买卖点仅三类（升跌完备定理） | `b_ℓ∈{0,1}^6` 六 bit 不压扁；三类买卖点 ⊊ 转折节点（E0 成型节点非买卖点，bsp-opsem §四） | C1 |
| 第24课 MACD 对背驰的辅助判断 | **严格化**：MACD 面积单判据 → 允许力度族 `𝒜_ℓ` 支配序（`M_MACD` 非充分统计量，§3 定理）；MACD 降为 𝔉_ℓ 一个投影坐标 | C6 |
| 第37课趋势背驰前提（a+A+b+B+c，A/B 须同级别中枢构成趋势语境） | **严格化**：`Comparable_ℓ(s',s)=1` ⟺ ∃上级容器 C, s',s⊑C 且为可比前后同向推进段——排除非同结构语境比较 | C5 |
| 第37课「离开级别不谈趋势背驰」 | 形式化为 `ℓ(s)=ℓ(s') ∧ dir(s)=dir(s')=−δ`（同级同向，§4.1） | C5 |
| 第19课力度/背驰（延异二值） | **严格化**：二值 → 三值 `{Yes,No,Undetermined}`（支配序口径冲突时不强判，交预注册 Θ） | C6 |
| 缠论「按买卖点交易赚钱」 | **修改（收窄有效域）**：缠论语法 ⇒ 端点正价差，**⇏** 因果成交正收益；赚钱须额外 CEP 经济公理（强/弱/统计版），非缠论语法本身可证（§8） | X-econ |

**核心对照结论**：formal-chain 相对博文层的两处实质严格化——(1) 背驰力度从「MACD 面积一票否决」升为「允许力度族支配序 + 三值判定」，Weak 移出 Cand 门进状态向量；(2) 引入 ex-ante/ex-post 分野与 CEP 经济公理，明确「缠论语法⇒端点正价差 ⇏ 净收益」的严格边界——这是博文层未显式化的、决定 bug 判据 vs 理论边界判据的关键。

---

## 五、结果包（简化版）

**结论**：从 formal-chain 6 份 PDF（完整的策略/买卖点/递归完全分类买卖点/关于背驰/区间套/经济正条件，亲读页码逐条标注）提取 entry/持仓/exit 合法性硬判据 15 条（C1-C8 entry / H1-H3 持仓 / X1-X3+X-econ exit），每条含形式化条件（保留 PDF 原符号+页码）、entry 时刻可测性（ex-ante 可测=bug 判据 / ex-post 才可判=理论边界候选）、实装位置（复用 proofs-full-strategy-20260703.md 已 Read 验证锚点，标注待重验）、违反后果分类。产出 `.chanlun/review-results/formal-criteria-20260705.md`。

**边界条件**（结论翻转）：
- 若 proofs-full-strategy-20260703.md 锚点行号漂移（今日 07-04，proofs 为 07-03 commit）⟹ 实装位置列须逐条 Read 重验，找不到照实改「实装位置待核」（本清单已标注 C3/C4 Rel/Role 完整判定、X-econ 滑点 η 模型为待核）。
- 若 deep-research 博文层报告落地后与第37/53课原文有出入 ⟹ 第四节对照表须按博文原文修正（formal-chain 为准原则不变）。
- ex-ante/ex-post 分类翻转条件：若确认-bar 部署改为 Live 候选部署（proofs D-3 CandType Live 轴升级）⟹ 部分 ex-ante 判据（Rel/Role）在未确认候选上变 ex-post。

**影响声明**：新增 `.chanlun/review-results/formal-criteria-20260705.md`（本清单）；零代码/定义/测试改动；引用 formal-chain 6 PDF + proofs-full-strategy-20260703.md 现有锚点（标注重放前须 Read 重验）。下游消费：**亏损逐笔重放工位**——按 C/H/X 系列 ex-ante 判据逐笔当场核验（有违反=实装 bug 先修），全通过后落 X-econ 理论边界闸门（归 μ/选择器/成本模型，不修实装）。
