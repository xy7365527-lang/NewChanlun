# 亏损修复方案 S1/S2/S3 严格调研 + 数学证明（持仓管理方向）

**工位**：swarm/ws-solresearch ｜ topo_address: swarm/ws-solresearch ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix
**输入**：opsem-redo-20260705.md（诊断：亏损=持仓管理失败，两充分特征：持仓>5000bar + 逆势；4 族 A/B/C/D）+ strict-fix-proposals-20260705.md（R1-R5 信号侧，本报告是其持仓管理侧后续）
**认识论等级约定**（231号）：每节标注 L0（纯定义/证明，零信息增量）/ L1（代码静态核对，管线正确性）/ L2（真实数据单标的假设检验）。涉及 alpha 有效性的陈述属 L2/L3，全部 INCONCLUSIVE，不在本报告范围。
**方法**：亲读 formal-chain 4 PDF 关键页（完整的策略.pdf / 买卖点.pdf §7.4-7.5 / 区间套.pdf）+ grep/Read 逐行核实代码锚点（interp.rs / coverage.rs / exit.rs / risk.rs / runner.rs）+ 对每个 S 给出"现状锚点 / formal-chain 原文 / 严格修复形式 / 数学证明 / bit-exact 影响 / 边界条件"六要素。

---

## 〇、元结论（先于分节）

**三个持仓管理修复方向全部为否定性发现**——formal-chain 不要求其中任何一个，且实装会**违反** formal-chain 已结算的定义。核心否定性发现照 161 号（否定性照实）+ 231 号（有效域<定义域）+ 090 号（声明膨胀禁止）报告：

| S | 任务描述前提 | 核实后真实状态 | 严格修复性质 |
|---|---|---|---|
| **S1** | "J_Θ 加 parent_dir 一致性约束（顺父候选优先 / 逆势候选加惩罚）" | J_Θ 形式定义 = `‖p−p̃‖²_W + λ·Cost(p) + ν·RiskPenalty(p)`（§15 line 740）三项作用于**净持仓 p**，**不含** parent_dir 项；σ_higher 经 **q_Θ/SizeΘ**（§6 line 867 状态参数）入策略，**不经** J_Θ；ShortDiff **定义上即逆势**（§7.5 `δg=−σα(g)`），J_Θ 加 parent_dir 项 ⟹ 抑制全部 ShortDiff ⟹ 违反 §7.5 | **否定**：formal-chain 违反（J_Θ 定义封闭 + §7.5 ShortDiff 合法性 + §6 明文禁"全局顺/逆上级一刀切"） |
| **S2** | "K_Θ 加逆势降仓/禁开约束" | K_Θ = 保证金/强平/净毛上限/双开/OQ-9/TW 三阶段/成本滑点/交易所（§11 line 873-888）全资本约束，**不含** 趋势方向；AncOK（§13）**已实装**裸逆势门控（coverage.rs:1753-1774「ShortDiff 未持父则剔除，不开 naked 逆势仓」） | **否定**：K_Θ 排除逆势 ⟹ 消灭 ShortDiff（§7.5 违反）+ p̃ 被推出 K_Θ ⟹ X^cover 静默偏离（§16）；裸逆势门控 AncOK 已装 |
| **S3** | "持仓时长上限（超 5000bar 强制减仓）" | §9 typed exit 五枚举 `{CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}`（line 656）**无时长型**；closePred（exit.rs:85）= ¬ParentValid ∨ reverse ∨ Stop ∨ RiskClose 全信号/价格触发，**无时长项** | **否定**：formal-chain §9 无时长出场型；"持仓过久"根因是 typed exit 未触发（Stop 太远/反向 BSP 未识别/ReduceCore 未发），非缺时长上限 |

**唯一合法 kernel**（三方向共通）：σ_higher（父级别方向）**确实是** formal-chain q_Θ/SizeΘ 的状态参数（§6 line 867 `SizeΘ(ℓ, δ, Iγ, N^depth, σ_higher, role, TStage, RiskMode, CostBucket, MarginState)`），但当前 v0 q_Θ（`leg_target` coverage.rs:1316）**仅消费 depth**（`s_e = base_units × w_depth(depth)`），**未消费** σ_higher。这是 v0 简化（v0 SizeΘ 只实装 10 参数中的 depth 维），升级 q_Θ 消费 σ_higher 是 **v0→v1 扩展**（需新预注册，135 号），**必须经 q_Θ 通道**（p̃ 构造层），**不经** J_Θ（S1）/K_Θ（S2）。且须遵守 §6 line 468-469 明文「不能用全局'顺上级/逆上级'一刀切」+ §7.5 ShortDiff 合法性。

**核心诊断修正**（对 opsem-redo §3 的关键补正）：诊断标记的 10 笔"逆势"亏损（含最大单笔 3765，role=`SameReverse|ShortDiff|Minus`）**全部是 ShortDiff 对冲腿**（V=ShortDiff），**非裸逆势投机**。ShortDiff 按 §7.5 定义 `δg=−σα(g)` 即逆父方向——这是**形式化定义的合法对冲操作**，其亏损是**对冲成本**（父仓保持，建反向子声部抵消），单腿 P&L 为负不蕴含 campaign（父+子）亏损。AncOK（§13）已保证这些 ShortDiff 腿的父 carrier 必在 A_t（coverage.rs:1753-1774），故无 naked 逆势。诊断的"逆势是亏损充分特征"是**逐腿口径**观察（L2），非 campaign 口径因果（L3）——此补正不翻转"持仓管理是主因"结论，但精化了"逆势"的语义。

---

## §S1 J_Θ 加 parent_dir 一致性约束

### 1. 现状代码锚点（L1 静态核对）

**J_Θ 代价函数实装**（coverage.rs:2385-2406 `j_theta_key`）：
```rust
fn j_theta_key(p, p_tilde, p_t, weights, grid_index) -> JThetaKey {
    JThetaKey {
        tracking_err: scale_key(weights.w * (p - p_tilde).powi(2)),  // 主键 ‖p−p̃‖²_W
        trade_cost:    scale_key(weights.lambda * (p - p_t).abs()),    // 次键 λ·|p−p_t|
        risk_penalty:  scale_key(weights.nu * p.abs()),                 // 三键 ν·|p|
        turnover: 0,                                                     // §15 无独立换手项
        grid_index,                                                       // 平局键（升序位次）
    }
}
```
- 权重（coverage.rs:2237-2253 `PiThetaWeights`）：`w=1.0`（范数正定，§15 line 744 `w_a>0`）、`λ=RiskConfig.kappa`（成本倍数 κ）、`ν=RiskConfig.rho`（风险罚 ρ）。三权重全是 Θ_risk 参数。
- **J_Θ 的定义域**：净持仓 `p ∈ 𝒦_Θ(x)`（有限 lot 网格，coverage.rs:2366 `feasible_candidates` 返 `{floor_pt, ceil_pt, hi, lo, 0.0, anchor_pt}`）。J_Θ 是 **net position p 的函数**，**不是**候选方向选择器。
- **p\* 选择**（coverage.rs:2416 `pi_theta_position`）：`p* = LexArgmin_{p∈𝒦_Θ} J_x(p)`——在净持仓网格上选字典序最小 p，**不选**候选方向。
- **候选方向选择**在 `interpret`（interp.rs:1161 `interpret_with_close_triggers`）的 fold：按 ≺_Θ 序处理候选 g，规则2 反向关闭 / 规则3 开启 / 规则4 记录——**买卖点 bits + reverse_signal 驱动**，**不读 J_Θ**。
- **opsem dump 的 `lex_argmin_top3`**（coverage.rs:2643 `StepTrace.lex_top3`）：暴露本步 LexArgmin 的 top-3 **(JThetaKey, control=p)** 候选——control 是**净持仓值**（如 -1/0/+1 lot），**不是**方向选择器。

**关键澄清（对诊断 trade 3765 的语义校正）**：诊断称"解释器按 J_Θ 选了 Short，因 tracking_err=128 最小"——这是对 J_Θ 作用层的**误读**。真实链路：
1. ShortDiff 候选 g（dir=Short, role.v=ShortDiff）经 `interpret` 规则3 进入 ℬ_x（open 桶）——此步**不读 J_Θ**，只读 bsp bits + reverse_signal + slot。
2. ShortDiff 腿入 A_{t+1}（AncOK 准入，父 carrier 必在场，§13）。
3. `strategy_target_legs` 算腿单位 `s_e = base_units × w_depth`（coverage.rs:1330）。
4. `net_target_units` 折叠腿为 p̃（coverage.rs:1426）——ShortDiff 空腿使 p̃ 变负。
5. `pi_theta_position` 在 𝒦_Θ 上 `LexArgmin J_x` 投影 p̃ → p\*（coverage.rs:2424）——**此步才读 J_Θ**，选 p\*=负值因 p̃ 为负（跟踪主键 `w(p−p̃)²` 最小）。

故 J_Θ 选 p\*=Short 是 p̃ 已为负的**投影结果**，非 J_Θ 主动选 Short。逆势决策发生在**步骤1**（interpret fold，bsp bits 驱动），不在步骤5（J_Θ 投影）。

### 2. formal-chain 原文要求（亲读 完整的策略.pdf）

**§15 P12 line 740 J_x 方框**（pdftotext /tmp/strategy.txt:890 邻近，coverage.rs:2230 引）：
> `J_x = ‖p−p̃‖²_W + λ·Cost_x(p) + ν·RiskPenalty_x(p)`

**精确三项**，作用于净持仓 p：(1) 加权跟踪误差（主键），(2) 成本（次键），(3) 风险罚（三键）。**无 parent_dir / σ_higher / 趋势一致性 项**。

**§15 P12 line 748 LexArgmin 方框**（coverage.rs:2408 引）：
> `p* = LexArgmin_{p∈𝒦_Θ(x)} J_x(p)`

J_x 的定义域是 𝒦_Θ（净持仓可行集），**非候选方向集 Γ(x)**。J_Θ 与候选方向选择（interpret fold）是**两个正交层**。

**§6 line 867 Q_Θ 仓位函数方框**（σ_higher 在此入策略，非 J_Θ）：
> `q_Θ(z, a) = SizeΘ(ℓ, δ, I_γ, N^depth, σ_higher, role, TStage, RiskMode, CostBucket, MarginState)`

σ_higher 是 **SizeΘ 的第 5 参数**——经 q_Θ（腿单位 → p̃=Σq·σ·e）入策略，**不经** J_Θ。J_Θ 消费的是 p̃（q_Θ 的输出），不直接消费 σ_higher。

**§6 line 468-482 明文禁"全局顺/逆上级一刀切"**（pdftotext /tmp/strategy.txt:468-482，**最关键否决依据**）：
> 「你们已有实验说明，**σ_higher 的收益符号会随级别翻转**，L1 是主要超 beta 来源，**不能用全局'顺上级/逆上级'一刀切**。所以完整状态必须含：`(ℓ, δ, σ_higher)`。否则会把'低级别逆上级短差'和'高级别逆上级接飞刀'混在一起。」

**§7.5 ShortDiff 定义**（买卖点.pdf /tmp/bsp.txt:596-602）：
> `Role(g) = ShortDiff ⟹ ℓ_g < ℓ_{α(g)} ∧ δ_g = −σ_{α(g)}`

ShortDiff **定义上即逆势**（δ_g = −父方向）。其语义（line 607-608）：「父仓保持不变，建立独立反向子声部」——这是 formal-chain **定义的合法对冲操作**。

### 3. 严格修复形式

**否定性发现（照实，161 号）**：S1 任务描述"J_Θ 加 parent_dir 一致性约束"在 formal-chain **三重违反**，无法给"严格修复形式"——任何形如 `J_x += η·Penalty(dir(g)≠σ_higher)` 的改动都破坏以下不变量：

1. **违反 J_Θ 定义封闭性**（§15 line 740）：J_Θ 是 p（净持仓）的函数，候选方向 dir(g) 不在其定义域。J_Θ 消费 p̃（已折叠），p̃ 已丢失"哪个候选贡献"信息——对 p̃ 加 parent_dir 项在代数上无意义（p̃ 是标量，无 dir 字段）。
2. **违反 §7.5 ShortDiff 合法性**：ShortDiff 定义 `δ_g = −σ_{α(g)}` 即逆父。J_Θ 加逆势惩罚 ⟹ ShortDiff 腿的 p̃ 分量被罚 ⟹ LexArgmin 偏向压扁 ShortDiff ⟹ 消灭短差对冲 ⟹ 违反 §7.5「父仓保持，建反向子声部」。
3. **违反 §6 line 468-469 明文**：「不能用全局顺上级/逆上级一刀切」——J_Θ 加 parent_dir 项恰是"全局逆势惩罚"的一刀切，正是 formal-chain 明文禁止的混淆（低级别逆上级短差 vs 高级别逆上级接飞刀）。

**若编排者仍要求 parent_dir 感知**（合法 kernel，经正确通道）：走 **q_Θ/SizeΘ 升级**（§S1-legit 见下），**不经** J_Θ。

### 4. 数学证明

**定理 S1-1（J_Θ 加 parent_dir 项破坏 ShortDiff 合法性，L0）**：

*证明*：设形式化改动 `J'_x(p, g) = J_x(p) + η·𝟙[δ_g ≠ σ_{α(g)}]`（η>0 为逆势惩罚）。对 ShortDiff 候选 g，由 §7.5 `δ_g = −σ_{α(g)}`，当 `σ_{α(g)} ≠ 0` 时 `δ_g ≠ σ_{α(g)}` 恒真 ⟹ `𝟙[δ_g ≠ σ_{α(g)}] = 1` ⟹ `J'_x(p, g) = J_x(p) + η`。
对 FollowParent 候选 g'，`δ_{g'} = σ_{α(g')}` ⟹ `𝟙 = 0` ⟹ `J'_x(p, g') = J_x(p)`。
故 `J'_x(p, g_{ShortDiff}) > J'_x(p, g'_{FollowParent})` 恒成立（同 p）⟹ LexArgmin 在同 tracking_err 下**永不选** ShortDiff 腿的 p̃ 分量 ⟹ ShortDiff 对冲腿被净额折叠消灭 ⟹ §7.5「父仓保持+建反向子声部」的净额体现（net_target_units 部分抵消，coverage.rs:1419）失效。 ∎

**推论**：S1 改动等价于**功能上删除 ShortDiff**——违反 §7.5 定义。

**定理 S1-2（J_Θ 加 parent_dir 项改变 p\* ⟹ 信号集变 ⟹ 需新预注册，L0）**：

*证明*：设原 `p* = LexArgmin_{p∈𝒦_Θ} J_x(p)`，改动后 `p*' = LexArgmin_{p∈𝒦_Θ} J'_x(p)`。由定理 S1-1，存在 ShortDiff 活动腿的状态 x 使 `J'_x(p) > J_x(p)` 对该腿贡献的 p̃ 分量 ⟹ p̃ 的 LexArgmin 最优点漂移 ⟹ `p*'(x) ≠ p*(x)`。由 `schedule_order(p*−p_t)`（coverage.rs:2466）是 p\* 的函数 ⟹ 订单 O_{t+1} 变 ⟹ 成交序列变 ⟹ ledger 变 ⟹ 信号集变 ⟹ **bit-exact 不保留，需新预注册**（135 号冻结纪律）。 ∎

**定理 S1-3（J_Θ 的 parent_dir 信息在 p̃ 折叠后已丢失，L0）**：

*证明*：`p̃ = net_target_units(legs) = Σ_{leg} ε_e · s_e`（coverage.rs:1426）是**标量净持仓**。折叠后 p̃ 不携带"哪条腿是 ShortDiff / 哪条是 FollowParent"信息（信息论不可恢复，对齐 231 号 net→gross 不可逆）。故 J_Θ(p, p̃) 在代数上**无法**消费 parent_dir——parent_dir 是腿级属性，p̃ 是账户级标量，两者不在同一抽象层。parent_dir 的合法消费点是**腿级** q_Θ（SizeΘ 第 5 参数 σ_higher），非**账户级** J_Θ。 ∎

### 5. bit-exact 影响分析

| 改动 | 影响路径 | bit-exact？ |
|---|---|---|
| S1 J_Θ 加 parent_dir 惩罚项 | LexArgmin p\* 漂移 ⟹ 订单 ⟹ ledger | **否**（信号集变，需新预注册，定理 S1-2） |
| S1-legit q_Θ 升级消费 σ_higher（经 SizeΘ） | 腿单位 s_e 变 ⟹ p̃ 变 ⟹ p\* 变 | **否**（v0→v1 扩展，需新预注册） |
| 不改（默认） | — | **是**（v0 冻结，135 号） |

### 6. 边界条件

- **修复无效条件**：若市场所有候选 dir(g) = σ_higher（全顺势，无 ShortDiff 生成），则 S1 惩罚项 `𝟙[δ_g ≠ σ_higher] = 0` 恒，J_Θ 实际不变——但此条件下 ShortDiff 本就不存在，S1 是空操作。
- **结论翻转条件**：若编排者裁定"v0 禁用 ShortDiff"（即策略层决定不做短差对冲），则 S1 的 ShortDiff 抑制效果成为**预期行为**——但这属策略选择（选择类，/escalate），且应用 `config.disable_shortdiff`（coverage.rs:2096 已实装的 f3 反事实开关）显式表达，**非** 经 J_Θ 隐式抑制。
- **不翻转**：J_Θ 定义封闭性（§15 line 740 三项）+ ShortDiff §7.5 定义 + §6 line 468-469 禁一刀切，三者都是 formal-chain 已结算定义，不随数据/配置翻转。

---

## §S1-legit σ_higher 经 q_Θ 通道的合法升级（v0→v1 扩展）

**此节是 S1/S2 共通的合法 kernel**，照 161 号（否定性照实）+ 231 号（有效域）记录。

### 现状（L1）

当前 v0 q_Θ = `leg_target`（coverage.rs:1316）：
```rust
let w = depth_weight(depth, config);  // w=[0.60, 0.30, 0.10] by depth
LegTarget { units: base_units * w, ... }  // s_e = base_units × w_depth
```
**仅消费 depth**（SizeΘ 10 参数中的 1 维），**未消费** σ_higher / I_γ / N^depth / role / TStage / RiskMode / CostBucket / MarginState（其余 9 维）。

### formal-chain 要求（§6 line 867）

`q_Θ(z, a) = SizeΘ(ℓ, δ, I_γ, N^depth, σ_higher, role, TStage, RiskMode, CostBucket, MarginState)`——σ_higher 是第 5 参数，**应**经 q_Θ 入策略。

### 严格升级形式（须经编排者授权 + 新预注册）

```rust
// leg_target / leg_target_two_segment 升级签名（概念，非现装）：
fn leg_target(..., sigma_higher: Option<VoiceSide>) -> LegTarget {
    let w_depth = depth_weight(depth, config);
    // σ_higher 维：按 (ℓ, δ, σ_higher) 三元组查分级权重表
    // ⚠ 须分级（§6 line 468-469）：不同 ℓ 下 σ_higher 的收益符号不同
    let w_dir = sigma_higher_weight(level, dir, sigma_higher, config);
    LegTarget { units: base_units * w_depth * w_dir, ... }
}
```

**严格性约束**（no-patch + §6 line 468-469）：
1. **分级**：`sigma_higher_weight` 必须按 (ℓ, δ, σ_higher) 三元组参数化，**不可**全局一刀切（§6 明文）。
2. **保 ShortDiff 合法性**：w_dir 不得归零 ShortDiff（§7.5），只能**缩放**（对冲规模调整，非消灭）。
3. **新预注册**：w_dir 是新 Θ 参数 ⟹ 触发 135 号冻结纪律 ⟹ 先冻结 w_dir 表再跑 OOS，不能先看结果再选（§9.2「不能先看结果再选」同源）。
4. **三套预注册**（对齐 关于背驰.pdf §9.2）：可定义 `Θ_dir_follow`（顺势加权）/ `Θ_dir_neutral`（不区分）/ `Θ_dir_adversary`（逆势降权），分别 OOS，不能事后选。

### bit-exact 影响

**不保留**（v0→v1 扩展）：w_dir 新参数 ⟹ s_e 变 ⟹ p̃ 变 ⟹ p\* 变 ⟹ 信号集变 ⟹ 需新预注册。

### 边界

- **有效域**：σ_higher 收益符号"随级别翻转"是 L2 实证（§6 line 468「实验说明」），故 w_dir 表的分级结构须 L2/L3 标定，L0 无法推导。
- **与诊断的关系**：此升级**不直接修** opsem-redo 的 4 族亏损——族 A（超大持仓）根因是 typed exit 未触发（§S3），族 B（震荡择时）是短差止损择时，族 C（背驰持仓久）是 ReduceCore 未发，族 D（减仓时机）是减仓点错。σ_higher 分级 sizing 是**正交的 v0→v1 增强**，非 4 族亏损的直接修复。

---

## §S2 逆势开仓风险门控（K_Θ 降仓/禁开）

### 1. 现状代码锚点（L1 静态核对）

**K_Θ 约束门实装**（coverage.rs:2316-2348 `KThetaRiskGate` + `caps`）：
```rust
pub struct KThetaRiskGate {
    pub force_flat: bool,        // GlobalRiskClose ⟹ 𝒦_Θ={0}
    pub stop_long: bool,         // 结构止损 ⟹ 禁净多（hi_cap=0）
    pub stop_short: bool,        // 结构止损 ⟹ 禁净空（lo_cap=0）
    pub no_increase_cap: Option<f64>,  // M2/M3 禁增仓
}
```
四态全是**资本/风控约束**（强平/止损/去杠杆），**无**趋势方向项。

**K_Θ wire 点**（runner.rs:606 `k_theta_risk_gate`，runner.rs:1009 消费）：
- `force_flat = global_risk_close(risk_mode(equity))`（equity≤0 ⟹ Insolvent/Liquidation）
- `stop_long/stop_short = close_pred(structural_stop 触及)`（per 活动腿结构止损）
- **无** parent_dir / σ_higher / 逆势 读出

**裸逆势门控（AncOK，§13）已实装**（coverage.rs:1753-1774 + 2089-2091）：
- `ancestor_close_by_id`（coverage.rs:2091）剔除真 Compose 父不在 raw 的孤儿腿。
- 注释（coverage.rs:1771-1774）：「父有向但**未持父仓**的逆向次级候选 = ShortDiff ⟹ 父不在 raw ⟹ AncOK **剪枝**该 ShortDiff ⟹ **不开仓**（防 garbage trade）。持父 ⟹ 父在 raw ⟹ ShortDiff **准入**。」
- **即**：裸逆势（无父持仓的 ShortDiff）**已被 AncOK 门控剔除**，当前实装已满足"禁开 naked 逆势仓"。

### 2. formal-chain 原文要求

**§11 line 873-888 K_Θ 风险可行集**（pdftotext /tmp/strategy.txt:873-888）：
> 风险可行集 K_Θ(x_t) 必须包含：保证金；强平；最大净头寸；最大毛头寸；多空双开；OQ-9；TW 三阶段合法性；成本和滑点；交易所约束。

**九项全是资本/保证金/交易所约束**，**无**"趋势方向一致性"约束。

**§13 AncOK 持仓准入**（pdftotext /tmp/strategy.txt:637-639）：
> `A_{t+1} = AncOK[(A_t ∖ D_t) ∪ O_t]`

AncOK = 祖先闭合（每条子声部腿的操作父 live）。ShortDiff 子声部腿的准入条件 = 父 carrier 在 A_t（§8 父子关系 + §13）。**这正是"裸逆势禁开"的 formal-chain 机制**——已实装。

**§7.5 ShortDiff 合法性**（同 §S1）：ShortDiff 定义即逆势，是合法对冲。K_Θ 排除逆势 = 排除 ShortDiff。

### 3. 严格修复形式

**否定性发现（照实，161 号）**：S2 任务描述"K_Θ 加逆势降仓/禁开约束"在 formal-chain **三重违反**：

1. **违反 K_Θ 定义域**（§11 line 873-888）：K_Θ 九项约束无趋势方向。加"逆势禁开"是向 K_Θ 注入 formal-chain 未定义的第 10 项 = 声明膨胀（090 号）。
2. **违反 §7.5 ShortDiff 合法性**：K_Θ 排除逆势净持仓 ⟹ ShortDiff 腿的 p̃ 分量无可行点 ⟹ 消灭短差对冲（同定理 S1-1）。
3. **违反 §16 单一决策出口**（同 strict-fix-proposals §R4 引）：K_Θ 收窄 ⟹ p̃ 被 clamp ⟹ p\*≠p̃ ⟹ X^cover 静默偏离（§16）——诊断可见性问题（K_Θ 静默 clamp）正是 R5 范畴（诊断插桩），非机制缺口。

**裸逆势门控已装**：AncOK（§13）已剔除无父 ShortDiff（coverage.rs:1753-1774）。诊断的 10 笔"逆势"全是**有父 ShortDiff 对冲腿**（§〇 已校正），非裸逆势——S2 的"禁开"目标（裸逆势）已达成，"降仓"目标（对冲规模）应经 q_Θ（§S1-legit），非 K_Θ。

### 4. 数学证明

**定理 S2-1（K_Θ 加逆势排除消灭 ShortDiff，L0）**：

*证明*：设改动 `K'_Θ(x) = K_Θ(x) ∩ {p : sign(p) = σ_higher 或 p = 0}`（排除逆势净持仓）。ShortDiff 腿使 p̃ 的分量 `ε_e · s_e = (−σ_{α(g)}) · s_e`（coverage.rs:1313，δ_g = −σ_{α(g)}）——若 σ_higher = +1（父多），ShortDiff 空腿贡献负 p̃ 分量。若父仓（FollowParent 多腿）+ ShortDiff 空腿共存，p̃ = s_父 − s_子。
- 若 s_父 > s_子 ⟹ p̃ > 0 ⟹ p̃ ∈ K'_Θ（顺势）⟹ p\* > 0 ⟹ ShortDiff 分量被净额保留（对冲生效）——但此非"禁开 ShortDiff"，是"净额保留"。
- 若 s_父 < s_子 ⟹ p̃ < 0 ⟹ p̃ ∉ K'_Θ（逆势）⟹ p̃ 被 clamp 到 0 ⟹ p\* = 0 ⟹ 父多腿也被平 ⟹ **父子同平**——违反 §7.5「父仓保持不变」。

故 K_Θ 排除逆势净持仓在 s_父 < s_子 时强制平父仓 ⟹ 违反 §7.5。 ∎

**定理 S2-2（K_Θ 加逆势降仓破坏覆盖不变量 X^cover，L0）**：

*证明*：§16 X^cover 不变量要求 p\* 是 p̃ 在 K_Θ 上的 LexArgmin 投影（覆盖不变量：p\* 尽可能贴近 p̃）。加逆势降仓约束 ⟹ K_Θ 收窄 ⟹ 对逆势 p̃，投影 p\* 被**静默推离** p̃（p\* 被推向 0 或顺势）⟹ 覆盖误差 `‖p*−p̃‖_W` 非零且无诊断输出（除非 R5 插桩）⟹ §16 X^cover 静默偏离（strict-fix-proposals §R4 引 F8 claim）。这正是 strict-fix-proposals §R4 已述的"K_Θ 静默 clamp"——机制是 K_Θ 约束的合法效果，诊断可见性是 R5 范畴。 ∎

**定理 S2-3（AncOK 已满足"禁开 naked 逆势"，L1 静态核对）**：

*证明*：`coverage_step_from_buckets_sep`（coverage.rs:1905）步骤2 调 `ancestor_close_by_id(&work, &raw)`（coverage.rs:2091）剔除父不在 raw 的孤儿腿。ShortDiff 候选元素 parent_id（coverage.rs:2079）指向父 carrier——若父不在 raw（未持仓），ShortDiff 被剪（coverage.rs:1771-1774 注释明文）。故**无父 ShortDiff 不入 A_{t+1}** = 不开 naked 逆势仓。此为 §13 AncOK 的实装兑现，L1 核对通过。 ∎

**推论**：S2 的"禁开逆势"目标（naked 逆势）**已由 AncOK 达成**。"降仓"目标（对冲规模调整）应经 q_Θ（§S1-legit），非 K_Θ。

### 5. bit-exact 影响分析

| 改动 | 影响路径 | bit-exact？ |
|---|---|---|
| S2 K_Θ 加逆势排除 | p̃ 被 clamp ⟹ p\* 漂移 ⟹ 订单 ⟹ ledger | **否**（消灭 ShortDiff + X^cover 偏离，定理 S2-1/S2-2） |
| S2 K_Θ 加逆势降仓系数 | K_Θ 收窄 ⟹ p\* 漂移 | **否**（同上） |
| 不改（AncOK 已门控 naked 逆势） | — | **是**（v0 冻结） |

### 6. 边界条件

- **修复无效条件**：若所有 ShortDiff 候选的父 carrier 都不在 A_t（无父持仓），则 AncOK 已全剪 ⟹ 无逆势仓位 ⟹ S2 空操作。诊断的 10 笔"逆势"不满足此条件（父 carrier 在场，是合法对冲）。
- **结论翻转条件**：若编排者裁定"ShortDiff 对冲腿的净持仓分量也属裸逆势，须 K_Θ 禁"——则与 §7.5「父仓保持+建反向子声部」直接冲突，属定义扩展（选择类，/escalate），非 bug 修复。
- **不翻转**：K_Θ 九项约束（§11 line 873-888）+ §7.5 ShortDiff 定义 + AncOK 已门控 naked 逆势（L1 核对），三者不随数据/配置翻转。

---

## §S3 持仓时长上限（超阈值强制减仓）

### 1. 现状代码锚点（L1 静态核对）

**typed exit 五枚举**（interp.rs:223-235 `ExitType`，对齐 §9）：
```rust
pub enum ExitType {
    CloseRoot,        // P5：一类反向点=根清仓
    ReduceCore,       // P6：三类反向点=核心仓减仓
    CloseShortDiff,   // P7：短差反向确认=关子声部
    RiskExit,         // P1：保证金/强平（KThetaRiskGate.force_flat）
    Hold,             // P0：无出场
}
```
**无时长型 / 超时型 exit**。

**closePred 关闭谓词**（exit.rs:85-128 `exit_decision_for`，对齐 `Origin.SubVoiceOpenClose.closePred` line 552-562）：
```rust
X_{v,t} = ¬ParentValid ∨ χ^{σ_p}（反向信号）∨ Stop（结构止损触及）∨ RiskClose
```
四析取项**全是信号/价格/风控触发**，**无时长项**。`held` 台账（exit.rs:37 `HeldVoice`）不记录开仓 bar / 持仓 age。

**K_Θ RiskGate**（coverage.rs:2316，§S2 已述）：四态（force_flat/stop_long/stop_short/no_increase_cap）无时长项。

**runner fill_loop**（runner.rs:1009 邻近）：逐 bar `k_theta_risk_gate` → `pi_theta_step` → `schedule_order`，**无**持仓 age 计数 / 时长阈值检查。grep `duration|max_hold|holding|age|超时|timeout` 在 exit.rs/risk.rs/runner.rs **零命中**（§S3 任务"grep exit.rs/runner.rs fill_loop 找持仓时长有无上限"——答：无）。

### 2. formal-chain 原文要求

**§9 正规出场层**（pdftotext /tmp/strategy.txt:653-667，**最关键否决依据**）：
> 「完整策略不能只用'下一个反向信号'出场。出场应是 typed exit：`Exit_Θ(v, x_t) ∈ {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}`。例如：第一类反向点：根仓清仓；第三类反向点：核心仓减仓；短差反向确认：关闭子声部；保证金/强平：风险优先退出。收益函数必须使用正规出场。」

**五枚举无时长型**。formal-chain 对"持仓过久"的回答是 **typed exit 正确触发**（CloseRoot/ReduceCore/CloseShortDiff/RiskExit），**非** 加时长上限。

**§9 line 667-670 收益函数**：
> `X_i^{full} = δ_i(P_{τ_i^{typed}} − P_{t_i}) − C_i`

出场时间 `τ_i^{typed}` 由 typed exit 决定（反向点/止损/强平），**非** 时长阈值。

### 3. 严格修复形式

**否定性发现（照实，161 号）**：S3 任务描述"超阈值强制减仓"在 formal-chain **双重违反**：

1. **违反 §9 ExitType 枚举封闭性**：五枚举 `{CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}` 无 `TimeExit` / `DurationCap`。加第 6 型 = 声明膨胀（090 号）+ 破坏 §13 策略全定义证明（line 1001 `∀x_t ∃!O_{t+1}` 的 typed exit 枚举封闭前提）。
2. **违反 §9 line 654 设计意图**：「不能只用下一个反向信号出场」⟹ 答案是**用 5 型 typed exit**（结构化的反向点/减仓/短差关/强平），**非** 加非信号型时长 cap。时长 cap 是 formal-chain **明确否定**的"非 typed 出场"路径。

**持仓过久的真实根因**（诊断 §3，非缺时长上限）：
- **族 A（3765/3756/4036，持仓 12775-201909 bar）**：ShortDiff 对冲腿（3765）或顺势腿（3756/4036）的 `Stop`（结构止损）未触发——止损价太远或 ShortDiff 腿 stop_hit 判据未覆盖子声部腿。根因是 **Stop 触发缺陷**，非缺时长。
- **族 C（2031，持仓 51107 bar）**：背驰做空正确，但 `ReduceCore`（三类反向点）未触发——大级别趋势延续突破背驰段，三类反向点未出现。根因是 **ReduceCore 触发条件**（三类反向点识别），非缺时长。
- **族 D（7 笔 ReduceCore）**：减仓点落在大级别反弹中段——`ReduceCore` 触发了但**时机错**（三类反向点识别在反弹中段）。根因是 **三类买卖点识别质量**（R1/R2 信号侧），非缺时长。

**严格修复方向**（非 S3 时长 cap，而是 typed exit 触发质量）：
- 族 A：检查 ShortDiff 腿的 `structural_stop` 是否正确设置（exit.rs:56 `record_held_voice` 是否覆盖子声部腿）+ stop_hit 触及判据。
- 族 C/D：检查三类买卖点（Type3）识别 + ReduceCore 触发（interp.rs:250 `reverse_exit_type` 的 trigger_class=3 分支）。
- 这些是 **R1/R2 信号侧 + exit 触发质量** 问题，已在 strict-fix-proposals 范畴，**非** S3 时长 cap。

### 4. 数学证明

**定理 S3-1（§9 ExitType 枚举封闭，时长 cap 不在其中，L0）**：

*证明*：§9 line 656 定义 `Exit_Θ(v, x_t) ∈ {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}`——有限枚举，封闭。设时长 cap `T_max` 触发强制减仓为新型 `Exit_Time`。则 `Exit_Time ∉ {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}` ⟹ 枚举非封闭 ⟹ 违反 §9 定义。又 §13 策略全定义证明（line 994-1001）假设6「解释器 I_Θ 使用固定优先级」依赖 P1..P10 谓词（line 495-554，对应五 typed exit），加 `Exit_Time` 须加新谓词 `P_Time` ⟹ 破坏互斥化 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`（line 561-567）的优先级链 ⟹ ∃!O_{t+1} 唯一性可能破裂（两谓词同触时优先级未定义）。 ∎

**定理 S3-2（时长 cap 截断既有持仓 ⟹ bit-exact 不保留，L0）**：

*证明*：设持仓腿 v 在 bar t_entry 开仓，时长 cap `T_max=5000`。若 `t − t_entry > T_max` 且 typed exit（CloseRoot/ReduceCore/Stop/RiskClose）均未触发，则 S3 强制 `Exit_Time(v)` 平仓。
- 原（无 S3）：腿 v 持续至 typed exit 触发 bar `τ_typed`（可能 `τ_typed − t_entry > T_max`）。
- 改后（有 S3）：腿 v 在 `t_entry + T_max` 被强平。
- 故 `τ'_exit = t_entry + T_max < τ_typed` ⟹ 成交序列变（提前平仓）⟹ ledger 变 ⟹ 信号集变 ⟹ **bit-exact 不保留，需新预注册**（135 号）。 ∎

**推论**：S3 改动截断所有 `持仓 > T_max` 的既有腿——族 A（3 笔）+ 族 C（1 笔）的持仓全 > 5000 bar ⟹ 全被提前平 ⟹ ledger 显著变。

**定理 S3-3（时长 cap 阈值 T_max 无 formal-chain 依据，L0）**：

*证明*：formal-chain §9 typed exit 五型的触发条件全部**结构化**（一类/三类反向点 = 买卖点 bit；短差反向确认 = 子声部反向信号；强平 = equity≤0；止损 = 结构止损价触及）——**无时间维度**。`T_max=5000` 是经验参数（诊断 §3 观察到"持仓>5000bar 是亏损充分特征"），**非** formal-chain 可导（§15 J_Θ / §11 K_Θ / §9 Exit_Θ 均无 T_max）。故 T_max 是 Θ_T_max 新参数 ⟹ 触发 135 号冻结纪律 ⟹ 须先冻结 T_max 再 OOS，且 T_max 的选择本身是**数据窥探（data snooping）**风险（从亏损样本反推阈值 = 过拟合，过拟合.pdf 警告）。 ∎

### 5. bit-exact 影响分析

| 改动 | 影响路径 | bit-exact？ |
|---|---|---|
| S3 加 T_max 强制减仓 | 族 A/C 长持仓被截断 ⟹ ledger | **否**（信号集变 + 新 Θ_T_max 参数，定理 S3-2/S3-3） |
| 不改（默认，typed exit 触发） | — | **是**（v0 冻结） |

### 6. 边界条件

- **修复无效条件**：若所有持仓均在 T_max 内被 typed exit 触发平仓（无持仓 > T_max），则 S3 空操作。诊断族 A/C 不满足（持仓 12775-201909 bar 远超 5000）。
- **结论翻转条件**：若编排者裁定"v0 须加非 typed 时长止损作为安全网"（如实盘风控要求最大持仓时长），则属**定义扩展**（新增 Exit_Time 第 6 型 + Θ_T_max 参数，选择类，/escalate）——但这与 §9 五枚举冲突，须编排者显式授权扩展 §9，并接受 §13 ∃!O 唯一性须重证（加 P_Time 后互斥化优先级链）。
- **不翻转**：§9 ExitType 五枚举封闭性 + typed exit 触发条件全结构化（无时间维）+ T_max 数据窥探风险，三者不随配置翻转。
- **与 R5 的关系**：若编排者要"持仓时长可观测"（诊断可见性，非强制平仓），则属 R5 范畴（opsem dump 增 `holding_bars` 字段，env-gated 只读，bit-exact 不变）——这与 S3（强制减仓机制）正交，可独立授权。

---

## 结果包六要素

### 1. 结论

三个持仓管理修复方向**全部为否定性发现**（formal-chain 不要求，实装会违反）：
- **S1（J_Θ 加 parent_dir 项）**：否定——J_Θ = `‖p−p̃‖²_W + λ·Cost + ν·RiskPenalty`（§15 line 740）三项作用于净持仓 p，不含 parent_dir；σ_higher 经 q_Θ/SizeΘ（§6 line 867）入策略；ShortDiff 定义即逆势（§7.5），J_Θ 加逆势惩罚消灭 ShortDiff（定理 S1-1）；§6 line 468-469 明文禁"全局顺/逆上级一刀切"。
- **S2（K_Θ 逆势降仓/禁开）**：否定——K_Θ 九项约束（§11 line 873-888）全资本/保证金/交易所，无趋势方向；AncOK（§13）**已实装**裸逆势门控（coverage.rs:1753-1774，定理 S2-3）；K_Θ 排除逆势消灭 ShortDiff（定理 S2-1）+ 破坏 X^cover 覆盖不变量（定理 S2-2）。
- **S3（持仓时长上限）**：否定——§9 typed exit 五枚举（line 656）无时长型；closePred（exit.rs:85）全信号/价格触发；"持仓过久"根因是 typed exit 触发质量（族A Stop 缺陷/族C ReduceCore 未发/族D 三类点时机错），非缺时长 cap；T_max 是数据窥探参数（定理 S3-3）。

**唯一合法 kernel**：σ_higher 是 formal-chain q_Θ/SizeΘ 第 5 参数（§6 line 867），当前 v0 q_Θ（`leg_target` coverage.rs:1316）仅消费 depth（`s_e = base_units × w_depth`），未消费 σ_higher——升级 q_Θ 消费 σ_higher 是 v0→v1 扩展（须新预注册 135 号 + 分级权重表 + 保 ShortDiff 合法性，§S1-legit）。

**核心诊断补正**（对 opsem-redo §3）：诊断的 10 笔"逆势"亏损全是 ShortDiff 对冲腿（V=ShortDiff，§7.5 定义即逆势），**非裸逆势投机**；AncOK 保证父 carrier 在场；单腿 P&L 为负是对冲成本，不蕴含 campaign（父+子）亏损。"逆势是亏损充分特征"是逐腿口径 L2 观察，非 campaign 口径 L3 因果。

### 2. 定义依据

- **完整的策略.pdf**（pdftotext /tmp/strategy.txt 亲读）：§6 line 443-482（完整状态含 `(ℓ,δ,σ_higher)` + **明文禁全局顺/逆上级一刀切** line 468-469）；§6 line 867（Q_Θ=SizeΘ 含 σ_higher 第 5 参数）；§9 line 653-667（typed exit 五枚举 `{CloseRoot,ReduceCore,CloseShortDiff,RiskExit,Hold}`，无时长型）；§11 line 873-888（K_Θ 九项约束全资本/保证金/交易所）；§15 line 740（J_Θ 三项 `‖p−p̃‖²_W + λ·Cost + ν·RiskPenalty`）；§13 line 994-1001（策略全定义证明，ExitType 枚举封闭前提）。
- **买卖点.pdf**（pdftotext /tmp/bsp.txt 亲读）：§7.4 line 577-591（SubFollow 顺父，合法同向子腿）；§7.5 line 596-677（ShortDiff `δ_g=−σ_{α(g)}` 定义即逆势，合法对冲，同股数要求 s_g=s_a）；§7 line 713（关闭优先级：风险≻订单≻同级反向≻短差平≻根开≻短差开≻顺向≻保持）。
- **代码锚点**（全部 L1 逐行核实）：`coverage.rs:1316-1356`（leg_target，q_Θ v0 仅 depth_weight）；`coverage.rs:2230-2406`（J_Θ/j_theta_key，三项+grid 平局）；`coverage.rs:2316-2383`（KThetaRiskGate 四态+feasible_candidates 安全锚 0）；`coverage.rs:1753-1774,2091`（AncOK 裸逆势门控已实装）；`coverage.rs:2416-2424`（pi_theta_position LexArgmin）；`coverage.rs:250-258`（reverse_exit_type typed exit 判据）；`interp.rs:1161-1224`（interpret fold，候选方向选择层，bsp bits 驱动非 J_Θ）；`exit.rs:85-128`（closePred 四析取，无时长项）；`runner.rs:606-696`（k_theta_risk_gate，四态 wire，无 parent_dir/时长）。

### 3. 边界条件（结论翻转条件）

- **S1 结论翻转**：若编排者裁定"v0 禁用 ShortDiff"（策略层决定不做短差对冲），则 S1 的 ShortDiff 抑制成预期——但应用 `config.disable_shortdiff`（coverage.rs:2096 已实装 f3 反事实开关）显式表达，非经 J_Θ 隐式抑制。
- **S2 结论翻转**：若编排者裁定"ShortDiff 对冲腿净持仓分量也属裸逆势须 K_Θ 禁"——与 §7.5「父仓保持+建反向子声部」直接冲突，属定义扩展（选择类，/escalate）。
- **S3 结论翻转**：若编排者裁定"v0 须加非 typed 时长止损作实盘安全网"——与 §9 五枚举冲突，须显式授权扩展 §9（加 Exit_Time 第 6 型 + Θ_T_max），并重证 §13 ∃!O 唯一性（加 P_Time 谓词后互斥化优先级链）。
- **不翻转**：J_Θ 三项定义封闭（§15 line 740）+ K_Θ 九项约束（§11 line 873-888）+ §9 ExitType 五枚举（line 656）+ §7.5 ShortDiff 定义 + §6 line 468-469 禁一刀切 + AncOK 已门控 naked 逆势（L1 核对 coverage.rs:1753-1774）——六者皆 formal-chain 已结算定义，不随数据/配置翻转。

### 4. 下游推论

- **对 opsem-redo 诊断的影响**：4 族亏损的根因**不在 S1/S2/S3 范畴**——族 A（Stop 触发缺陷）/ 族 B（短差止损择时）/ 族 C（ReduceCore 未发）/ 族 D（三类点时机）全是 **typed exit 触发质量**问题，修复方向是 exit 触发链（structural_stop 覆盖子声部腿 + 三类买卖点识别质量），非 J_Θ/K_Θ/时长 cap。此结论与 strict-fix-proposals §R5（诊断插桩）+ §R1/R2（信号侧）同向。
- **对 §16 策略的影响**：S1/S2/S3 全不改 ⟹ Π_max-full 端到端 OOS 的 INCONCLUSIVE 结论（af8910d062）不变——本报告是否定性研究，识别 0 项可立即修的真 bug（持仓管理侧）。
- **对 #135 冻结纪律的影响**：S1/S2/S3 全不触发新预注册（否定性发现，不改信号集）；唯一合法 kernel（§S1-legit q_Θ 升级消费 σ_higher）触发新预注册（v0→v1 扩展，新 Θ_dir 参数）。
- **对 AncOK 的确认**：S2 核实过程确认 AncOK（§13）**已实装**裸逆势门控（coverage.rs:1753-1774）——这是 639 号（σ_p 来源修正）的实装兑现，"naked 逆势 garbage trade" 在当前 v0 已被门控。
- **对 ShortDiff 单腿 P&L 口径的影响**：诊断的"逆势亏损"逐腿口径可能误导——建议 R5 opsem dump 增 campaign 口径 P&L（父+子合并），避免将对冲成本误读为裸投机亏损（此属 R5 诊断增强范畴，非本报告范围）。

### 5. 谱系引用

- **无新谱系**：本报告是否定性研究合成，未检测到需结晶的稳定信号。
- **相关已结算谱系**：
  - 090号：严格性语法规则（声明膨胀禁止——S1 加 parent_dir 项/J_Θ 扩三定义/S2 加 K_Θ 第 10 项/S3 加 Exit_Time 第 6 型全声明膨胀）
  - 161号：否定性照实（S1/S2/S3 三重否定）
  - 231号：有效域<定义域（J_Θ 定义域=净持仓 p，parent_dir 不在其中；K_Θ 定义域=资本约束，趋势方向不在其中；§9 ExitType 定义域=五 typed 型，时长不在其中）
  - 135号：冻结先于跑数（§S1-legit q_Θ 升级 σ_higher 须新预注册）
  - 639号：σ_p 来源修正（S2 核实确认 AncOK 裸逆势门控已实装，639 的实装兑现）
  - formalization-validity-domain（净→毛不可逆，231号同源）：S1 定理 S1-3 引（p̃ 折叠后 parent_dir 信息已丢失）
  - strict-fix-proposals §R4/§R5（K_Θ 静默 clamp = §16 X^cover，机制已装诊断待补；R5 是诊断插桩范畴）
- **不确定是否有相关谱系**：σ_higher 分级 sizing（§S1-legit）是否在某谱系中标为"v0 未实装的 SizeΘ 维度"——未检索到明确谱系。若编排者认为"v0 SizeΘ 仅实装 depth 是诚实有效域声明"值得结晶，可走 knowledge-crystallization（参考 coverage.rs:1308 注释已诚实声明 v0 depth-only）。

### 6. 影响声明

- **代码改动**：零。本报告是否定性研究综合，未改动任何代码。
- **文档改动**：新建 `/Users/silencehan/Projects/NewChanlun/.chanlun/review-results/loss-solution-proposals-20260705.md`（本文件）。
- **对实装的影响**：识别 0 项可立即修的真 bug（持仓管理侧 S1/S2/S3 全否定）；识别 1 项 v0→v1 扩展候选（§S1-legit q_Θ 升级消费 σ_higher，须编排者授权 + 新预注册）。下游消费：编排者可授权 §S1-legit 跑批；S1/S2/S3 无需行动。
- **对诊断流程的影响**：建议 R5 opsem dump 增 campaign 口径 P&L（父+子合并）+ holding_bars 字段（env-gated 只读），以精化"逆势亏损"的逐腿 vs campaign 口径区分 + 持仓时长可观测性——此属 R5 诊断增强，非本报告范围。
- **诚实声明**：S1/S2/S3 的"否定"判定基于 formal-chain 原文（4 PDF 亲读）+ 代码逐行核实（L1）。若编排者对 formal-chain 有不同解读（如 J_Θ 应含 parent_dir、K_Θ 应含趋势、§9 应加时长型），结论可翻转——这些属定义扩展（选择类，/escalate），留编排者裁决。本报告**不**声明"持仓管理无问题"——只声明 S1/S2/S3 三方向非 formal-chain 要求的修复路径；真实持仓管理问题（typed exit 触发质量）在 exit/signal 侧，已超出 S1/S2/S3 范畴。

---

**报告结束**。三个持仓管理修复方向（S1/S2/S3）全部否定性发现——formal-chain 不要求，实装会违反已结算定义（J_Θ 三项封闭/K_Θ 九项资本约束/§9 五 typed exit）。唯一合法 kernel 是 σ_higher 经 q_Θ/SizeΘ 通道升级（v0→v1 扩展，§S1-legit）。核心可交付：每个 S 的严格修复形式 + 数学证明 + bit-exact 影响 + 边界条件，认识论等级 L0/L1 全标注。
