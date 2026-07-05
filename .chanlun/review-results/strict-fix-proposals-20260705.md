# 严格修复方案 + 数学证明（5 个 spec-acknowledged v0 简化）

**工位**：swarm/ws-fixresearch ｜ topo_address: swarm/ws-fixresearch ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix
**输入**：opsem-loss-diagnosis-20260705.md（B1-B5 真实 bug）+ loss-diagnosis-combined-20260705.md（synthesis 修正：spec-acknowledged 简化非 bug）+ formal-criteria-20260705.md（C1-C8/H1-H3/X1-X3+X-econ 15 条硬判据）
**认识论等级约定**（231号）：本报告每节标注 L0（纯定义/证明，零信息增量）/ L1（代码静态核对，管线正确性）。涉及 alpha 有效性的陈述属 L2/L3，全部 INCONCLUSIVE，不在本报告范围。
**方法**：亲读 formal-chain 6 PDF 关键页 + grep/Read 逐行核实代码锚点 + 对每个 R 给出"现状锚点 / formal-chain 原文 / 严格修复形式 / 数学证明 / bit-exact 影响 / 边界条件"六要素。

---

## 〇、元结论（先于分节）

**5 个修复方向中，3 个的"bug"前提不成立**——这是本次研究的核心否定性发现，照 161 号（否定性照实）+ 231 号（有效域<定义域）报告：

| R | 任务描述前提 | 核实后真实状态 | 严格修复性质 |
|---|---|---|---|
| **R1** | "divergence 默认 MacdArea 单指标" | §9.2 三套预注册 Θ 全实装（MacdArea/ThetaDom/ThetaLex/ThetaScore）；但 **per-rung Cand^δ_k 含 Weak（div_cand 条件4）违反 §9.1**；SubMovePower 缺（已诚实声明） | **混合**：§9.1 偏差（有效域扩展，需新预注册）+ 真实缺口（SubMovePower） |
| **R2** | "Type2/3 占 97.8% 跳过 per-rung divergence" | Type2/3 非 divergence 候选（Conf 终端确认 ≠ 背驰候选）；per-rung cand=true 是 codex 673-fix 裁决① | **非 bug**：当前实装符合 formal-chain，误读 |
| **R3** | "level 0 占 89.6% 免锚定门" | lvl==0 免门是递归 base case（空真）；89.6% 是市场几何 | **非 bug**：第29课L396 原文语义，有效域收窄非实装缺陷 |
| **R4** | "K_Θ 不 wire 三阶段资本约束（runner.rs:2130 v0 未建模）" | 三阶段已 wire（方案A协变缩放 + stage_progression + KThetaRiskGate）；**任务锚点 runner.rs:2130 错误**（实为 t_stage_str 辅助函数） | **非 bug**：任务描述锚点与事实不符 |
| **R5** | "3 语义字段全 null" | **真实插桩缺口**（lex_argmin_top3 硬编码 null / divergence_input 仅一类 Some / nest_depth π 路径恒 None） | **真 bug**（纯诊断，env-gated，生产 bit-exact 不变） |

**唯一可立即修的真 bug = R5**（纯诊断插桩，零生产语义改动）。R1 是有效域扩展（需编排者授权新预注册）。R2/R3/R4 当前实装已满足 formal-chain 或基于误读。

---

## §R1 背驰支配序实装

### 1. 现状代码锚点（L1 静态核对）

背驰判定在**两层**实装，需分别核对：

**(a) per-rung 区间套 Cand^δ_k（生产 π 路径）**：
- `rust/src/theta_v0/classifier/cand_predicate.rs:107-151` `div_cand` 四条件合取：
  - 条件1 Dir（:122-129）、条件2 Comparable（:131-136）、条件3 Extreme（:138-145）、**条件4 Weak = MACD 面积 `is_divergence(prev_area, curr_area)`（:147-150）**
- 调用链：`econ_positive.rs:789 cand_delta_type1_extreme` → `cand_predicate.rs:204 div_cand` → 被 `econ_positive.rs:951 build_nest_certificate`（生产区间套证书构造）消费
- **核实**：`cand_delta_type1_extreme` 注释自述「跑 `div_cand`（Extreme+Weak 四条件）。bit-exact 复用旧 Type1 分支逻辑」（econ_positive.rs:787-788）——**生产 π 路径的 per-rung Cand 门含 Weak**

**(b) buy1 judge 层 D 判定**：
- `rust/src/theta_v0/classifier/divergence.rs:481-499` `confirm_divergence(gauge, macd_c_lt_a, force)` 四口径：
  - `MacdArea`（默认，:494）/ `ThetaDom`（:495，`force_state()==Dominated`）/ `Conjunction`（:496）/ `ThetaLex`（:497，`weak_theta(Lex)`）
- `divergence.rs:371-389` `ForceProxies::force_state()` 已实装 5-proxy 支配序（macd_area/dif_peak/price_amplitude/price_speed/tv），返回 `ForceStateA5::{Dominated,Dominates,Tie,Incomparable}`
- 调用链：`signal.rs:349 confirm_divergence(gauge, macd_c_lt_a, force.as_ref())` ← `judge_first_cached`（buy1 一类买卖点背驰确认）
- `config.rs:281 divergence_gauge: DivergenceGauge`（默认 `MacdArea`，bit-exact 冻结，135号）

**(c) 真实缺口诚实声明**：
- `divergence.rs:340-349`：「𝒜_ℓ 第6成员 SubMovePower（`Σ_{次级别同向段} m_{ℓ-1}(u)`）仍缺，数据源未达 signal 抽取层，登记诚实缺口——裁定见 codex-a4-force-20260704.md」

### 2. formal-chain 原文要求（亲读 关于背驰.pdf）

**§9.1 两层架构（p9-10）**——verbatim：
> 定义 `StructEligible^δ_ℓ(s,s')` 作为宽结构候选：`dir(s)=−δ ∧ Comparable(s',s) ∧ Extreme^δ(s',s)`。然后定义力度状态 `ForceState_ℓ(s',s)={Dominated,Dominates,Incomparable,Tie}`。最后：**`Cand^δ_ℓ = StructEligible^δ_ℓ` 而不是 `Cand = MACDdiv`**。是否交易由 `Z=(ℓ,δ,I_γ,ForceState,MACD,DIF,Amplitude,Speed,…)` 进入 `μ(z,a)` 或选择器决定。**这样没有候选会因为 MACD 被提前删除。**

**§5-6 支配序（p5-6）**——verbatim：
> 定义强度支配序 `s ⪯_𝒜 s' ⟺ ∀m∈𝒜_ℓ, m(s)≤m(s')`。定义严格弱化 `s ≺_𝒜 s' ⟺ s ⪯_𝒜 s' ∧ ∃m∈𝒜_ℓ, m(s)<m(s')`。
> **定理1（支配弱化稳健性）**：若 `s ≺_𝒜 s'`，则任何允许力度函数 `m∈𝒜_ℓ` 都不会认为 s 强于 s'。
> **定理2（不存在无额外公理的唯一全序力度）**：若 `∃m1,m2,s,s'` 使 `m1(s)<m1(s') ∧ m2(s)>m2(s')`，则不存在原文强制的唯一布尔背驰判定。

**§9.2 三套预注册 Θ（p10）**——verbatim：
> 如果必须二值背驰，定义三套预注册 Θ：`Θ_DOM`（支配序，只判最稳背驰）/ `Θ_LEX`（结构>DIF>面积）/ `Θ_SCORE`（标准化多特征得分）。分别 OOS 回测，**不能先看结果再选**。

**§10 形式化定理（p10-11）**——`macd_is_projection`（MACD 是完整签名投影）/ `dominance_sound`（`weak_dom s s'=yes → ∀m∈admissible, m s ≤ m s'`）/ `no_canonical_total_order`。

### 3. 严格修复形式

当前实装存在**两层不一致**，需分别处理：

#### 修复 R1-a（§9.1 偏差，per-rung Cand 含 Weak）—— 有效域扩展

**问题**：`div_cand`（cand_predicate.rs:107）四条件含 Weak（条件4 MACD 面积），违反 §9.1「`Cand=StructEligible` 而不是 `Cand=MACDdiv`」。生产 π 路径每个 rung 的 Type1 Cand^δ_k 都用 MACD 面积一票否决——这正是 §9.1 警告的"候选因为 MACD 被提前删除"。

**严格修复（§9.1 路线，力度完全交 Z）**：
```rust
// cand_predicate.rs:107 div_cand 改为三条件 = StructEligible
pub fn div_cand(input: &DivCandInput<'_>) -> bool {
    // 条件1 Dir + 条件2 Comparable + 条件3 Extreme（原 :122-145 不动）
    // 删除条件4 Weak（:147-150 is_divergence 调用）
    // 力度判定移出 Cand 门 → ForceState 进 Z 第8维（#159 已透传，已实装）
}
```
**不保留旧 Weak 分支**（no-patch-mentality：兼容性垫片=保留已知错误作 fallback，禁止）。`is_divergence` 原语不删（close_pred 契约锚保留先例），仅移除 `div_cand` 内的消费点。

**严格修复（§9.2 路线，per-rung Cand 参数化）—— 备选**：
若编排者裁定 per-rung 需保留二值背驰（与 buy1 judge 层一致），则 `div_cand` 接受 `gauge: DivergenceGauge` 参数，与 `confirm_divergence` 同口径（复用 `ForceProxies::force_state`，不重算）。但此路线下 per-rung 仍需 `ForceProxies`（A/C 段 5-proxy），当前 `div_cand` 签名只接 `hist`——需 #115/#159 透传链扩展到 per-rung 层。

#### 修复 R1-b（真实缺口 SubMovePower）

**问题**：𝒜_ℓ 第6成员 `SubMovePower = Σ_{次级别同向段} m_{ℓ-1}(u)`（关于背驰.pdf p5 原文列出）缺，数据源未达 signal 抽取层。

**严格修复**：在 `signal.rs` 抽取层接入次级别塔段（`recursive_tower` 已有 `sub_moves`），为每个 A/C 段累加其次级别同向段的 `m_{ℓ-1}(u)`，填入 `ForceFeatures` 新字段 `sub_move_power`。补齐后 `ForceStateA5` 升名 `ForceState`（完整 𝒜_ℓ）。

**优先级**：R1-a 改 Cand 门 ⟹ 信号集变 ⟹ ledger 变 ⟹ **需新预注册重跑（135号冻结纪律）**。R1-b 补 SubMovePower ⟹ ForceState 升级 ⟹ 同样需新预注册。两者都不是"修 bug"，是有效域扩展。

### 4. 数学证明

**定理 R1-1（支配序单调性，divergence.rs:348-349 已述）**：补维只会把 `Dominated`/`Dominates`/`Tie` 变 `Incomparable`，反向不会。

*证明*：设当前力度族 𝒜₅，补维后 𝒜₆=𝒜₅∪{m₆}。
- 若 `s ≺_{𝒜₅} s'`（即 ∀m∈𝒜₅, m(s)≤m(s') ∧ ∃m∈𝒜₅, m(s)<m(s')），补 m₆ 后：
  - 若 m₆(s)≤m₆(s') ⟹ `s ≺_{𝒜₆} s'`（保持 Dominated）
  - 若 m₆(s)>m₆(s') ⟹ ∃weaker(m₆) ∧ ∃stronger(m₆) ⟹ `Incomparable`（降级）
- 反向：若 `s ≺_{𝒜₅} s'` 不成立（∃m∈𝒜₅, m(s)>m(s')），补维不会消除该 stronger ⟹ 不会变 Dominated。 ∎

**推论**：`ForceStateA5::Dominated` 是完整 𝒜_ℓ `Dominated` 的**超集（宽判背驰）**——当前 5-proxy 判定的背驰 ⊇ 完整 6-proxy 判定的背驰。补 SubMovePower 只会把部分 Dominated 降级为 Incomparable，不会引入新的 Dominated ⟹ **保守侧**（不会把非背驰误判为背驰，只会把部分背驰判为不可判）。

**定理 R1-2（§9.1 Cand 移除 Weak 的候选集扩张）**：移除 div_cand 条件4 后，Cand 集严格扩张。

*证明*：设原 Cand₄ = Dir∧Comparable∧Extreme∧Weak，新 Cand₃ = Dir∧Comparable∧Extreme。
- Cand₄ = Cand₃ ∧ Weak ⟹ Cand₄ ⊆ Cand₃。
- ∃候选满足 Cand₃ 但不满足 Weak（MACD 面积未衰减但结构推进的段）⟹ Cand₄ ⊊ Cand₃。 ∎

**推论**：Cand 集扩张 ⟹ 进入 Z 的候选增多 ⟹ μ/选择器在更大候选集上决策 ⟹ 信号集可能变 ⟹ ledger 变 ⟹ **需新预注册**（135号）。这是有效域扩展不是 bug 修复。

**定理 R1-3（§9.2 三套预注册满足 dominance_sound）**：当前 `ThetaDom` gauge 实装满足 §10 `dominance_sound` 定理。

*证明*：`confirm_divergence(ThetaDom,_,force) = (force.force_state()==Dominated)`（divergence.rs:495）。`force_state()==Dominated` ⟺ ∀m∈𝒜₅, m(C)≤m(A) ∧ ∃m, m(C)<m(A)（divergence.rs:384）。这正是 §5 `s ≺_𝒜 s'` 的实装 ⟹ `weak_dom s s'=yes → ∀m∈admissible, m s ≤ m s'`（定理1）成立。 ∎

### 5. bit-exact 影响分析

| 改动 | 影响路径 | bit-exact？ |
|---|---|---|
| R1-a 移除 div_cand 条件4 | 生产 π per-rung Cand ⟹ 信号集 ⟹ ledger | **否**（Cand 集扩张，需新预注册） |
| R1-a 备选（per-rung gauge 参数化） | 同上 | **否**（默认 MacdArea 可保 bit-exact，但切 ThetaDom 变） |
| R1-b 补 SubMovePower | ForceState 升级 ⟹ ThetaDom 信号集变 | **否**（需新预注册） |
| 默认 MacdArea 不动 | — | **是**（v0 冻结，135号） |

**不受影响路径**：buy1 judge 层 `confirm_divergence` 四口径已实装且默认 MacdArea bit-exact；`ForceProxies::force_state` 5-proxy 支配序已实装（ThetaDom 消费）。R1-a/b 仅扩展有效域，不破坏既有 bit-exact 不变量（默认路径输出不变）。

### 6. 边界条件

- **修复无效条件**：若市场数据 A/C 段的 5-proxy 全部同向（无冲突），则 ForceStateA5 与 ForceState（6-proxy）判定一致——SubMovePower 补维对这些候选无影响。
- **结论翻转条件**：若 R1-a 移除 Weak 后 OOS 显示 alpha 显著变化（L2/L3），则"MACD 面积一票否决是信号 edge 来源"的假设需重评——但这属 L2/L3，本报告不声明。
- **不翻转的部分**：§9.2 三套预注册已实装（MacdArea/ThetaDom/ThetaLex/ThetaScore），无论 R1-a/b 是否执行，这三套都可 OOS 对比（wverify_run.rs:763-768 已有 thetadom_three_gauge_oos 跑批入口）。

---

## §R2 Type2/3 per-rung divergence verification

### 1. 现状代码锚点（L1）

- `rust/src/theta_v0/backtest/econ_positive.rs:847-859` `cand_delta`（673-fix 薄 dispatcher）：
  - `Type1 → cand_delta_type1_extreme`（:855，调 div_cand 四条件含 Weak）
  - **`Type2 | Type3 → true`**（:856）
  - `StructBreak → false`（:857，门拒 codex 终局裁决A）
- `econ_positive.rs:861-879` `cand_delta_base_gate`：Type2/3 存在性锚（descend anchor）base gate 一次
- `econ_positive.rs:843-846` 注释自述（codex 裁决①）：「Type2/3 存在性已由 `cand_delta_base_gate` 门控（base 一次），上级 rung 载上级语境（第17课L60 完备性保证 Type2/3 存在）⟹ cand=true」

### 2. formal-chain 原文要求

**关于背驰.pdf §9.1（p9）**：`Cand^δ_ℓ = StructEligible^δ_ℓ` 的定义域是**趋势背驰候选**（Type1，dir(s)=−δ 的衰竭段）。§4.3 明文：「若处理盘整背驰，可以…另设 `PanDivContext`，不要和趋势背驰混为一个谓词」——**divergence 只对趋势背驰（Type1），不对 Type2/3**。

**买卖点.pdf §4 / formal-criteria C2**：`Conf^+_e=⋁_{i=1}^3 B_{i,e}`、`Conf^−_e=⋁_{i=1}^3 S_{i,e}`——Type2/3 是 **Conf 终端确认**（执行级别买卖点 bit），不是趋势背驰候选。区间套递归 `N^δ_{ℓ↓e}` 的终端 `Conf^δ_e` 对 Type2/3 只需"买卖点 bit 置位"，不需"背驰力度衰减"。

**formal-criteria C5**：`Cand^δ_ℓ = StructEligible^δ_ℓ = dir ∧ Comparable ∧ Extreme`——**这是 Type1 背驰段候选的门**，Type2/3 无对应 Cand 门（它们走 Conf 终端确认 + 存在性锚）。

### 3. 严格修复形式

**否定性发现（照实）**：R2 任务描述「Type2/3 占 97.8% 跳过 per-rung divergence」是对 formal-chain 的**误读**。Type2/3 **不是 divergence 候选**——divergence（背驰力度衰减）只对 Type1（趋势背驰）。Type2/3 的 per-rung Cand 是"上级语境存在性"（第17课L60 完备性保证），已由 base gate（descend anchor）+ rung 载上级语境满足。

**当前实装 `Type2|Type3 → true` 符合 formal-chain**（codex 673-fix 裁决①）：
- Type2/3 的存在性锚定在 base gate 一次（`cand_delta_type2_completion`/`cand_delta_type3_retest`，:821/:840）
- per-rung 不重复判 divergence（Type2/3 无 divergence 语义）
- per-rung "cand=true" 表示"上级 rung 载上级语境，Type2/3 存在性已 base gate 门控"——这是 §C2 Conf 终端确认的递归承袭，不是"跳过 divergence"

**无需修复**。若强加 per-rung divergence 给 Type2/3 = 在非背驰候选上施加背驰判据 = **范畴错误**（混 Type1 背驰与 Type2/3 Conf 确认为同一谓词，§4.3 明文禁止）。

### 4. 数学证明

**定理 R2-1（Type2/3 无 divergence 定义域）**：divergence 谓词 `Weak^δ(s,s')` 的定义域是{趋势背驰段对 (s,s')}，Type2/3 候选不在其定义域。

*证明*：divergence 要求 `dir(s)=−δ`（背驰段方向反向，§9.1 StructEligible 条件1）。Type2（回抽走势完成）/Type3（离开中枢回抽）的候选段方向由买卖点 bit 决定，非"背驰段方向反向"语义。formal-chain 把 Type1 背驰与 Type2/3 Conf 确认为**不同谓词**（§4.3「不要混为一个谓词」）⟹ Type2/3 不入 divergence 定义域 ⟹ per-rung `cand_delta` 对 Type2/3 返回 true 是"无 divergence 可判"的空真，非"跳过"。 ∎

**推论**：任务描述的"97.8% 跳过 per-rung divergence"是真实现象但**不是 bug**——Type2/3 本就无 divergence 可判。把 Type1 divergence 强加给 Type2/3 反而是 bug（范畴错误）。

### 5. bit-exact 影响分析

**无改动**（非 bug）。当前实装符合 formal-chain，bit-exact 不变。

### 6. 边界条件

- **结论翻转条件**：若编排者裁定"Type2/3 也需力度验证"（超出 关于背驰.pdf §9.1 定义域），则需新立 `PanDivContext`（§4.3 原文）并定义 Type2/3 的力度判据——但这是**定义扩展**（选择类，/escalate），不是 bug 修复。
- **不翻转**：Type1 per-rung Cand 含 Weak（R1-a 已述）是另一问题，与本节 Type2/3 无关。

---

## §R3 level 0 锚定门

### 1. 现状代码锚点（L1）

- `rust/src/theta_v0/backtest/econ_positive.rs:812` 注释：「第29课L396「二三类精确点要下次级别以下找第一类」）。`lvl==0`（无次级别，递归底）⟹ 存在性」
- `econ_positive.rs:821` `cand_delta_type2_completion`：`lvl == 0 || descend_type1_anchor_depth(s, source_index, delta, hist).is_some()`
- `econ_positive.rs:840` `cand_delta_type3_retest`：同结构（`lvl == 0 || ...`）
- `econ_positive.rs:1098` 注释：「本级 lvl（小转大域恒 ≥1：lvl==0 时 base gate 免门走区间套，不入本通道）」

### 2. formal-chain 原文要求

**缠师第29课L396**（formal-criteria C2 引）：「二三类精确点要下次级别以下找第一类」——锚定谓词 `descend_type1_anchor_depth` 的定义域是{lvl≥1}（需次级别塔）。

**formal-criteria C2**：区间套递归 `N^δ_{ℓ↓e}` 终止于执行级别 e。lvl==0 时 e=0 是递归底，**无次级别可下沉**——定律一下沉的定义域前提（"下次级别"）物理不满足。

### 3. 严格修复形式

**否定性发现（照实）**：R3 任务描述"level 0 占 89.6% 免锚定门"是**有效域收窄事实**，不是实装 bug。

**当前实装 `lvl==0 || ...` 符合第29课原文**——递归 base case 的空真（vacuous truth）实装：
- `descend_type1_anchor_depth` 在 lvl==0 时定义域为空（无次级别塔层）
- 第29课"下次级别以下找第一类"的前提（存在下次级别）在 lvl==0 物理不满足
- 免门 = "前提不成立时规则不适用"，不是"绕过规则"

**无需修复**。89.6% 在 lvl==0 是**市场几何**（BTC 数据 lvl==0 走势段主导），非实装缺陷。这是 231 号（有效域<定义域）的典型例：定律一下沉在 lvl≥1 有效，lvl==0 退化为存在性免门（无锚定，但物理上无法锚定）。

### 4. 数学证明

**定理 R3-1（lvl==0 免门的空真合法性）**：`descend_type1_anchor_depth(s,source,delta,hist)` 在 lvl==0 时返回 None（定义域为空），`lvl==0 || x.is_some()` 在 lvl==0 恒真。

*证明*：
- `descend_type1_anchor_depth` 需读 `tower[lvl+1..]`（次级别塔层）。lvl==0 ⟹ tower[1..]，但 anchor depth 递归依赖"次级别走势段"，lvl==0 是递归底（formal-criteria C2「级别链有限 o_v>…>e_v ⟹ 递归有限终止」）。
- 第29课原文锚定谓词 `∃次级别一类点` 在 lvl==0 ⟹ ∄次级别 ⟹ `∃x∈∅.P(x) = false`（空集存在量化恒假）。
- 但"二三类精确点要下次级别以下找第一类"是**条件规则**（前提：存在下次级别）。前提不成立 ⟹ 规则不适用 ⟹ 免门是"规则空真"非"规则绕过"。
- 实装 `lvl==0 || anchor.is_some()` 把"前提不成立"显式化为免门分支，等价于`(∃next-level ∧ anchor) ∨ ¬∃next-level`——在 lvl==0 退化为 `¬∃next-level = true`。 ∎

**推论**：89.6% 在 lvl==0 = 89.6% 的交易在递归底（无次级别锚定）——这是 BTC 市场几何（小级别走势段密度远大于大级别），非实装可修。

### 5. bit-exact 影响分析

**无改动**（非 bug）。当前实装符合第29课原文，bit-exact 不变。

### 6. 边界条件

- **修复无效条件**：若强行 lvl==0 也锚定（移除 `lvl==0 ||`），则 `descend_type1_anchor_depth` 在 lvl==0 返回 None ⟹ 所有 lvl==0 Type2/3 候选被门拒 ⟹ **89.6% 交易消失**——这是把"市场几何主导 lvl==0"误当 bug 强行修复，反而破坏策略。
- **结论翻转条件**：若编排者裁定"lvl==0 需替代锚定"（如用本级背驰确认代替次级别一类点下沉），则超出第29课L396原文（原文明确"下次级别以下"，lvl==0 无下次级别）——属定义扩展（选择类，/escalate）。
- **显式有效域声明**：当前实装 lvl==0 免门已部分声明（econ_positive.rs:812 注释），可进一步在 `descend_type1_anchor_depth` docstring 显式标注"定义域 lvl≥1，lvl==0 调用方须免门"——这是文档加固非代码修复。

---

## §R4 K_Θ 三阶段资本约束 wire

### 1. 现状代码锚点（L1）

**任务描述锚点 "runner.rs:2130 诚实承认 v0 未建模" 错误**——核实 runner.rs:2122-2130 是 `t_stage_str` 辅助函数（TStage→字符串），非"v0 未建模"声明。

**三阶段资本约束实际已 wire 的锚点**：
- `rust/src/theta_v0/strategy/coverage.rs:2316-2348` `KThetaRiskGate`（force_flat/stop_long/stop_short/no_increase_cap 四态约束门）
- `coverage.rs:2350-2383` `feasible_candidates`（𝒦_Θ 有限网格，**安全锚 0 构造性非空见证**，C7 满足）
- `coverage.rs:2285` 注释：「**绝对资本特权尺度已消除，三阶段资本比例化完成**」（方案A协变缩放，2026-06-28-absolute-capital-equivariance-resolution §6 钦定）
- `coverage.rs:2681-2730` TW 谓词 P2/P3/P4（#124 裁定4「真统一」：TW 三阶段进 fold）+ `stage_progression` 单源（:2729）
- `rust/src/theta_v0/backtest/runner.rs:606-688` `k_theta_risk_gate`（构造 KThetaRiskGate）
- `runner.rs:1009` wire 进 main loop：`let (gate, risk_mode_i) = k_theta_risk_gate(...)`
- `rust/src/theta_v0/closed_loop/transition.rs:319` `stage_progression`（OQ-9 门，H2 满足）
- `rust/src/theta_v0/strategy/ledger.rs:364` `TwEvent::Realize`（TW 漂移唯一构造子）
- `runner.rs:439-443`：「M8 treasury 层终态：三阶段资金账本 `TwState`」

### 2. formal-chain 原文要求

**formal-criteria C7（K_Θ≠∅）**：`K_Θ(x)⊆𝒫^sep`，含手数/同股数双开/祖先闭合/杠杆/保证金/短差可执行性/**三阶段资本约束**/订单执行约束。要求 `K_Θ(x)≠∅`。

**formal-criteria H2（三阶段单向迁移 + OQ-9 门）**：`TStage∈{CostReduction→CapitalRecovered→EarningShares}`（单向）；TW 事件经 OQ-9 门；非 Realize 七构造子逐事件保 `tw()=free+holding+withdrawn` 守恒。

**formal-criteria C8（LexArgmin）**：`p*_{t+1}=LexArgmin_{p∈K_Θ(x)} J_x(p)`——单一决策出口（§16）。

### 3. 严格修复形式

**否定性发现（照实）**：R4 任务描述"K_Θ 不 wire 三阶段资本约束"与代码事实**不符**。三阶段资本约束已通过方案A协变缩放 wire（coverage.rs:2285 注释明文「三阶段资本比例化完成」）：
- `KThetaRiskGate` 承载 stop/risk 风控约束（close_pred 折入 𝒦_Θ，Q2 编排者裁定，coverage.rs:2296-2313）
- `stage_progression` 承载三阶段单向迁移 + OQ-9 门（H2）
- `feasible_candidates` 含安全锚 0（C7 𝒦_Θ≠∅）
- `pi_theta_position` 在收窄 𝒦_Θ 上 LexArgmin（C8 单一决策出口）

**任务锚点错误**：runner.rs:2130 是 `t_stage_str` 辅助函数（TStage 枚举→字符串），非"v0 未建模"声明。整个 runner.rs 无"三阶段资本 v0 未建模"的诚实声明——grep `未建模|v0|placeholder|not.*modeled` 在 runner.rs 无三阶段相关命中。

**无需修复**。若 loss-diagnosis-combined §1.1 synthesis 说的"K_Θ 静默 clamp（p̃∉K_Θ 时）对应 GAP3 count=0 上游"是真实问题，那是 §16 X^cover 覆盖域（F8 claim：K_Θ 把 p̃ 推出时风险投影静默偏离）——但这是 K_Θ 约束的**合法效果**（p̃ 被 clamp 到可行集，p*≠p̃ 是约束生效），不是"未 wire"。诊断可见性可加固（R5 范畴），但机制已实装。

### 4. 数学证明

**定理 R4-1（三阶段 wire 的不变量）**：当前实装满足 C7（𝒦_Θ≠∅）+ C8（单一决策出口）+ H2（三阶段单向 + OQ-9）。

*证明*（L1 静态核对）：
- **C7 非空**：`feasible_candidates`（coverage.rs:2366）返回 `vec![floor_pt, ceil_pt, hi, lo, 0.0, anchor_pt]`——恒含 0.0 ⟹ 𝒦_Θ≠∅（安全锚构造性见证，coverage.rs:2357 注释「spec line 725 硬前提 / Lean `feasible_nonempty`」）。
- **C8 单一出口**：`pi_theta_position`（coverage.rs:2422）在 `feasible_candidates` 上调 `lex_argmin`（intent.rs）选 p*——退出经 𝒦_Θ 收窄（force_flat→{0}，coverage.rs:2336）非独立 exit 订单（coverage.rs:2300-2303 注释「§16 单一决策出口」）。
- **H2 三阶段**：`stage_progression`（transition.rs:319）返回 `Option<TwEvent>`（P3 RecoverCapital / P4 EnterEarning），TStage rank 单调不可逆（runner.rs:3769 注释「advance_to rank 单调」）；OQ-9 门 `Oq9Illegal`（transition.rs:268）拒非法事件。 ∎

**推论**：三阶段资本约束已完整 wire，R4 任务描述前提不成立。

### 5. bit-exact 影响分析

**无改动**（任务描述与代码不符）。三阶段已 wire，bit-exact 不变。

### 6. 边界条件

- **结论翻转条件**：若 loss-diagnosis synthesis 的"K_Θ 静默 clamp"指 p̃ 被 clamp 时**无诊断输出**（而非机制未装），则属 R5 范畴（语义插桩），不属 R4（机制 wire）——R4 机制已装，R5 诊断可见性待补。
- **不翻转**：runner.rs:2130 是 `t_stage_str` 辅助函数（事实），任务描述锚点错误不随任何条件翻转。
- **M7 L2 witness**：`runner.rs:3787 m7_l2_witness_treasury_reach_real_btc`（真实 BTC 数据三阶段可达性检验）已证 TStage 单调推进——三阶段机制在生产路径真实数据上可达（GAP3 "∃t TStage=III" 从 FALSIFIED → 生产可达，runner.rs:3442 测试）。

---

## §R5 语义插桩 3 字段（唯一真 bug，纯诊断）

### 1. 现状代码锚点（L1）

**opsem-dump 框架**（`runner.rs:1791-1813`，env-gated 只读外化）：
- 触发：`env OPSEM_DUMP_DIR=<dir>`，未设 ⟹ 全部方法 no-op（runner.rs:1828-1848 `from_env`）
- 产物：`<dir>/trades.jsonl` + `<dir>/tower_events.jsonl`
- 认识论：**L1（纯只读外化，零信息增量）**——不进 μ 桶键/J_Θ 排序/χ 门控（runner.rs:1796）

**3 字段缺席实装**：
- **`lex_argmin_top3`**：`runner.rs:1905` 硬编码 `"lex_argmin_top3":null`，附 `"lex_argmin_absent_reason":"J_Θ keys consumed inside coverage_step_from_buckets_sep; not exposed"`
- **`divergence_input`**：`runner.rs:1916` 字段已存在 `"divergence_input":{...}`，但仅一类候选 Some（opsem 诊断 210/214 笔 null，4/214 Some）
- **`nest_depth`**：`runner.rs:1899` 字段已存在 `opt_u8_str(t.entry_z.nest_depth)`，但 π 路径恒 None（runner.rs:1027 注释「π 路径候选不经 Nest/Xzd 准入门，cand_channel/nest_depth 本管线恒 None」）

### 2. formal-chain 原文要求

formal-chain 不直接要求 opsem-dump（这是诊断基建，非策略判据）。但**编排者方法论令**「用内在语法操作语义诊断」要求语义状态可观测——3 字段缺席 ⟹ 诊断不可观测（opsem-loss-diagnosis §3 元层结论：故事族全否决的根因）。

间接 formal-chain 依据：
- C8 LexArgmin：`p*=LexArgmin_{p∈K_Θ}J_x(p)`——lex_argmin_top3 暴露选择/拒绝拓扑理由（J_Θ 键 top3）
- C5/C6 divergence：`divergence_input` 暴露 A/C 段力度签名（ForceProxies）
- C2 区间套：`nest_depth` 暴露跨级塔链深度（N^δ 递归层数）

### 3. 严格修复形式

**修复 R5-a（lex_argmin_top3 暴露）**：
```rust
// coverage.rs: coverage_step_from_buckets_sep 返回值增补 lex_argmin 候选键 top3
// 当前 lex_argmin(&candidates)（intent.rs）仅返回 p_star，J_ThetaKey 被丢弃
// 修复：lex_argmin 增补返回 sorted candidates top3（J_ThetaKey + grid_index）
// 透传至 OpsemDump.write_trade 写入 "lex_argmin_top3"（替换 runner.rs:1905 硬编码 null）
```
具体：`intent.rs:lex_argmin` 增 `lex_argmin_top3(&candidates, k=3) -> Vec<(JThetaKey, f64)>`（已在内部排序，只需保留前3）。`coverage_step_from_buckets_sep` 把 top3 经 `StepTrace` 透传至 runner，runner 写 dump。

**修复 R5-b（divergence_input 透传至非一类候选）**：
```rust
// 当前 ForceProxies 仅一类候选 Some（#115 已收进 BspPoint.force，#159 Candidate.force 纯透传）
// 修复：二类/三类候选的 BspPoint.force 也填 ForceProxies（若 A/C 段可识别）
// 透传至 TypedTrade.entry_z.divergence_input，写入 dump
```
边界：二类/三类候选的 A/C 段定义需明确（二类=一类点后回抽走势；三类=离开中枢回抽）——若 formal-chain 未定义二类/三类的 divergence 语义（R2 已证 Type2/3 非 divergence 候选），则 `divergence_input` 对 Type2/3 恒 None 是**合法缺席**（非 bug），仅一类候选需暴露。

**修复 R5-c（nest_depth 接入 π 路径）**：
```rust
// 当前 π 路径候选不经 Nest/Xzd 准入门（runner.rs:1027）⟹ nest_depth 恒 None
// 修复：π 路径候选调 build_nest_certificate（econ_positive.rs:889，与生产门同源）填 nest_depth
// 写入 TypedTrade.entry_z.nest_depth
```
这与 z 维 #11 缺口同源（opsem-loss-diagnosis §5 下游推论）。

### 4. 数学证明

**定理 R5-1（env-gated no-op 的 bit-exact 不变量）**：`OPSEM_DUMP_DIR` 未设时，R5-a/b/c 改动对生产路径零影响。

*证明*：
- `OpsemDump::from_env()`（runner.rs:1829）`let dir = std::env::var("OPSEM_DUMP_DIR").ok().filter(|s|!s.is_empty())?`——未设 ⟹ 返回 None ⟹ `OpsemDump` 不构造。
- 所有 `write_trade`/`mark_entry`/`mark_exit` 调用点经 `if let Some(dump)=&mut self.opsem_dump`（runner.rs 既有模式）守卫 ⟹ None 时 no-op。
- R5-a 的 `lex_argmin_top3` 返回值经 StepTrace 透传——StepTrace 是 Debug/诊断字段，不进 𝒦_Θ/J_Θ 排序/χ 门（runner.rs:1796「dump 不进 μ 桶键/J_Θ 排序/χ 门控」）⟹ 生产 p_star 计算不变。
- R5-b 的 ForceProxies 透传只写 dump 字段，不改 Candidate.force 消费链（#159 已纯透传，μ 层 z 第8维仍读 c.force）。
- R5-c 的 build_nest_certificate 调用是**只读构造**（econ_positive.rs:889 已证 L0 纯结构构造），不产 side effect，nest_depth 仅写 dump。
- ∴ OPSEM_DUMP_DIR 未设 ⟹ 生产路径逐字节不变（bit-exact）。 ∎

**推论**：R5 是**唯一可立即修的真 bug**——纯诊断插桩，零生产语义改动，无需新预注册（135号不适用，因为不改信号集/ledger）。

**定理 R5-2（R5-b 的合法缺席边界）**：Type2/3 候选的 `divergence_input=None` 是合法缺席（R2 已证 Type2/3 非 divergence 候选），仅 Type1 候选需暴露。

*证明*：R2-1 已证 divergence 谓词定义域是 Type1 趋势背驰段对。Type2/3 不在定义域 ⟹ 无 ForceProxies 可算 ⟹ `divergence_input=None` 对 Type2/3 是"定义域外诚实缺席"（非 bug）。R5-b 修复仅扩展 Type1 候选的 divergence_input 覆盖率（从 4/214 提升到全部 Type1 候选）。 ∎

### 5. bit-exact 影响分析

| 改动 | 影响路径 | bit-exact？ |
|---|---|---|
| R5-a lex_argmin_top3 | OPSEM_DUMP_DIR 设时 dump 内容；未设时 no-op | **未设：是（生产零改动）；设：仅 dump 文件变** |
| R5-b divergence_input 透传 | 同上（仅 Type1 候选扩展） | 同上 |
| R5-c nest_depth 接入 π 路径 | 同上（build_nest_certificate 只读构造） | 同上 |

**不受影响路径**：μ 桶键/J_Θ 排序/χ 门控/𝒦_Θ 约束/p_star 计算全不动（opsem-dump 是只读外化，runner.rs:1796 铁律）。

### 6. 边界条件

- **修复无效条件**：若 Type1 候选的 A/C 段在 signal 抽取层无法识别（dif/closes 无源），则 R5-b 透传失败——但这与 #115/#159 透传链同源，#115 已实装 BspPoint.force 收进 signal 抽取层，前置已满足。
- **结论翻转条件**：若 R5 补齐后在可观测语义状态上重做 opsem 诊断（opsem-loss-diagnosis §3 元层建议），可能识别出真实的操作语义失败模式——但这属 L2 诊断重做，不翻转 R5 本身（插桩机制正确）。
- **不翻转**：env-gated no-op 是结构性不变量（runner.rs:1829 `?` 早返回），不随任何条件翻转。

---

## 结果包六要素

### 1. 结论

5 个修复方向核实后：
- **R1**：混合——§9.2 三套预注册 Θ 已全实装（MacdArea/ThetaDom/ThetaLex/ThetaScore，默认 MacdArea 是 v0 冻结 bit-exact）；**per-rung Cand^δ_k 含 Weak（div_cand 条件4）违反 §9.1**（Cand 应=StructEligible 不含力度），是有效域扩展需新预注册；SubMovePower（𝒜_ℓ 第6成员）真实缺口已诚实声明。
- **R2**：**非 bug**（否定性发现）——Type2/3 非 divergence 候选（Conf 终端确认 ≠ 背驰候选），per-rung cand=true 是 codex 673-fix 裁决①，任务描述误读。
- **R3**：**非 bug**（否定性发现）——lvl==0 免门是递归 base case 空真（descend_type1_anchor_depth 定义域为空），89.6% 是 BTC 市场几何。
- **R4**：**非 bug + 任务锚点错误**（否定性发现）——三阶段资本约束已 wire（方案A协变缩放 + stage_progression + KThetaRiskGate + 安全锚 0）；runner.rs:2130 是 t_stage_str 辅助函数非"v0 未建模"声明。
- **R5**：**唯一真 bug**（纯诊断）——3 字段缺席（lex_argmin_top3 硬编码 null / divergence_input 仅一类 Some / nest_depth π 路径恒 None），env-gated 生产 bit-exact 不变，可立即修。

### 2. 定义依据

- **关于背驰.pdf**（pdftotext 亲读 p5-11）：§5-6 支配序 `s ≺_𝒜 s' ⟺ ∀m∈𝒜_ℓ,m(s)≤m(s') ∧ ∃m,m(s)<m(s')`；定理1/定理2；§9.1「`Cand^δ_ℓ=StructEligible^δ_ℓ` 而不是 `Cand=MACDdiv`」「力度进 Z 交 μ/选择器」「没有候选会因为 MACD 被提前删除」；§9.2 三套预注册 Θ_DOM/Θ_LEX/Θ_SCORE「分别 OOS，不能先看结果再选」；§4.3「盘整背驰另设 PanDivContext，不要和趋势背驰混为一个谓词」。
- **买卖点.pdf / formal-criteria C2/C5/C7/C8/H2**：Conf 终端确认（Type2/3）≠ 背驰候选（Type1）；Cand=StructEligible；𝒦_Θ≠∅（安全锚 0）；LexArgmin 单一决策出口；三阶段单向迁移 + OQ-9。
- **缠师第29课L396**：「二三类精确点要下次级别以下找第一类」（锚定定义域 lvl≥1）。
- **代码锚点**（全部 L1 逐行核实）：`divergence.rs:371-499`（force_state/confirm_divergence 四口径）、`cand_predicate.rs:107-151`（div_cand 四条件含 Weak）、`econ_positive.rs:789-975`（cand_delta dispatcher + build_nest_certificate）、`econ_positive.rs:821,840`（lvl==0 免门）、`coverage.rs:2285-2383`（方案A + KThetaRiskGate + feasible_candidates 安全锚）、`coverage.rs:2681-2730`（TW P2/P3/P4 + stage_progression）、`runner.rs:606-688,1009`（k_theta_risk_gate wire）、`runner.rs:1791-1916`（opsem-dump + 3 字段缺席）、`runner.rs:2122-2130`（t_stage_str 辅助函数，证任务锚点错误）。

### 3. 边界条件

- **R1 结论翻转**：若 R1-a（移除 div_cand Weak）OOS 后 alpha 显著变化 ⟹ "MACD 一票否决是 edge 来源"假设重评（L2/L3，本报告不声明）。
- **R2 结论翻转**：若编排者裁定 Type2/3 需力度验证 ⟹ 需新立 PanDivContext（§4.3），属定义扩展非 bug 修复。
- **R3 结论翻转**：若编排者裁定 lvl==0 需替代锚定 ⟹ 超出第29课L396原文，属定义扩展。
- **R4 结论翻转**：若 synthesis "K_Θ 静默 clamp"指诊断可见性（非机制）⟹ 转入 R5 范畴，R4 机制已装不翻转。
- **R5 结论翻转**：补齐后重做 opsem 诊断可能识别真实失败模式（L2 诊断），不翻转插桩机制本身。
- **不翻转**：R2（Type2/3 非 divergence 候选，§4.3 原文）、R3（lvl==0 递归底，第29课原文）、R4（三阶段已 wire，方案A 已钦定）的形式化判定不随数据/配置翻转。

### 4. 下游推论

- **对 §16 策略的影响**：R5（唯一真 bug）补齐后可在可观测语义状态上重做 opsem 诊断（opsem-loss-diagnosis §3 元层建议），可能识别真实操作语义失败模式。R1-a/R1-b 是有效域扩展，需编排者授权新预注册（135号）。
- **对 #135 冻结纪律的影响**：R5 不触发新预注册（纯诊断，不改信号集/ledger）；R1-a/R1-b 触发新预注册（Cand 门/ForceState 变 ⟹ 信号集变）。
- **对 z 维 #11 缺口的影响**：R5-c（nest_depth 接入 π 路径）与 z 维 #11 缺口同源，修复路径同向。
- **对 codex 673-fix 裁决的影响**：R2 确认 673-fix 裁决①（Type2/3 per-rung cand=true）符合 formal-chain，非 bug。
- **对 M8 终报告的影响**：本报告不改 Π_max-full 端到端 OOS 的 INCONCLUSIVE 结论（af8910d062）——5 个修复方向中 3 个非 bug，1 个纯诊断（R5），1 个有效域扩展（R1）。

### 5. 谱系引用

- **无新谱系**：本报告是研究合成，未检测到需要结晶的稳定信号。
- **相关已结算谱系**：
  - 090号：严格性语法规则（声明膨胀禁止，divergence.rs 注释已诚实声明 SubMovePower 缺口）
  - 161号：否定性照实（R2/R3/R4 的"非 bug"判定）
  - 231号：有效域<定义域（R1 ForceStateA5 是完整 𝒜_ℓ 的超集；R3 lvl==0 有效域收窄）
  - 135号：冻结先于跑数（R1-a/R1-b 需新预注册）
  - 671号：力度=feature 不作 selector 一票否决（R1 的 Weak canonical 归属在 judge/per-rung Cand，非 filter_gamma）
  - 673-fix 裁决①：Type2/3 per-rung cand=true（R2 确认符合 formal-chain）
  - #115/#159：ForceProxies 透传链（R5-b 的前置已满足）
  - 124裁定4：TW 三阶段进 fold（R4 已 wire 证据）
- **不确定是否有相关谱系**：div_cand 四条件含 Weak 是否在某谱系中标为"v0 冻结 bit-exact"——未检索到明确谱系，若存在则 R1-a 属该谱系的有效域扩展。

### 6. 影响声明

- **代码改动**：零。本报告是研究综合，未改动任何代码。
- **文档改动**：新建 `/Users/silencehan/Projects/NewChanlun/.chanlun/review-results/strict-fix-proposals-20260705.md`（本文件）。
- **对实装的影响**：识别 1 项可立即修的真 bug（R5，纯诊断插桩，零生产语义改动）；1 项有效域扩展（R1，需新预注册）；3 项否定性发现（R2/R3/R4 当前实装符合 formal-chain 或任务锚点错误）。下游消费：编排者可授权 R5 立即修复 + R1 新预注册跑批；R2/R3/R4 无需行动。
- **对诊断流程的影响**：R5 补齐后 opsem 诊断可在可观测语义状态上重做（opsem-loss-diagnosis §3 元层建议落地）。
- **诚实声明**：R2/R3/R4 的"非 bug"判定基于 formal-chain 原文 + 代码逐行核实，若编排者对 formal-chain 有不同解读（如 Type2/3 需力度验证、lvl==0 需替代锚定、三阶段需诊断可见性），结论可翻转——这些属定义扩展（选择类），留编排者裁决。

---

**报告结束**。5 个修复方向中 R5 是唯一可立即修的真 bug（纯诊断），R1 是需授权的有效域扩展，R2/R3/R4 是基于 formal-chain 误读或任务锚点错误的否定性发现。核心可交付：每个 R 的严格修复形式 + 数学证明 + bit-exact 影响 + 边界条件，认识论等级 L0/L1 全标注。
