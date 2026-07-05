# prereg-rev4｜q_Θ v0→v1 升级消费 σ_higher 分级权重预注册修订（g1 冻结）

**工位**：swarm/ws-qthetapreg ｜ topo_address: swarm/ws-qthetapreg ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix ｜ 基线：780451c503
**parent_callback**：main ｜ **下游消费者**：g2（实装）/ g3（跑数），依赖本文件 commit 冻结哈希
**认识论等级约定**（231号）：本 prereg 每节标注 L0（纯定义/原文推导，零信息增量）/ L1（代码静态核对，管线正确性）。涉及 alpha 有效性的陈述属 L2/L3，全 INCONCLUSIVE，不在本 prereg 范围。
**编排者令**：推进 q_Θ v0→v1 扩展（唯一合法修复 kernel，ws-solresearch commit 77006b82dc 确认 + loss-research-final-summary-20260705 §三背书）。

---

## 〇、元结论

**本 prereg 冻结 q_Θ 的 v0→v1 扩展形式**：从仅消费 `depth`（`s_e = base_units × w_depth(depth)`）升级为消费 `σ_higher`（父级别方向）的**分级符号权重表**。σ_higher 经 **q_Θ / SizeΘ 通道**（腿单位 → p̃ 构造层）入策略，**不经** J_Θ（§15 三项作用于净持仓 p，已结算封闭）/ K_Θ（§11 九项全资本约束，无趋势方向）——后者两条通道已在 loss-solution-proposals §S1/§S2 三重违反 formal-chain 否决。

**权重函数形式（本 prereg 冻结的严格修复形式）**：**分级符号查表**（discrete sign-graded lookup），不是线性、不是连续幅度。依据 §6 page4 明文「σ_higher 的收益符号会随级别翻转」——原文给的是**符号级、按级别 ℓ 分行**的信息，连续幅度响应无 formal-chain 支持（选连续形式 = 声明膨胀，090号）。

**ShortDiff 合法性保护的精确 formal-chain 锚（本 prereg 核心贡献，超出 loss §S1-legit 的细化）**：买卖点.pdf §7.5 page6 明文「同股数短差要求：`s_g = s_α`」。此约束**直接蕴含** ShortDiff 腿必须豁免 σ_higher 权重——`w_dir(role=ShortDiff) ≡ 1.0`。这不是设计选择，是 §7.5 的代数推论（§五 定理 1）。loss §S1-legit 仅说"w_dir 不得归零 ShortDiff，只能缩放"——**订正**：§7.5 `s_g=s_α` 比"不归零"更强，要求 ShortDiff 腿的 w_dir **恒等于 1**（任何 ≠1 的缩放都破坏同股数）。

**bit-exact 不保留**（v0→v1 扩展）：q_Θ 改 ⟹ s_e 改 ⟹ p̃ = Σ ε·s_e 改 ⟹ p\* = LexArgmin J_Θ(p, p̃) 改 ⟹ 订单 ⟹ ledger ⟹ 信号集改 ⟹ 需新预注册（135号）。唯一例外是 Θ_dir_neutral 预注册套（w_dir ≡ 1 退化回 v0），作为 bit-exact 对照基线。

**三套预注册**（对齐 关于背驰.pdf §9.2「不能先看结果再选」+ loss §S1-legit 第4点）：Θ_dir_follow（顺势保权）/ Θ_dir_neutral（v0 对照）/ Θ_dir_adversary（逆势保权，假设 §6 page4「L1 是主要超 beta 来源」）——三套并行 OOS，事后选 = 数据窥探。

---

## 一、修订范围

### 1.1 v0 现状（L1）

q_Θ v0 = `leg_target`（coverage.rs:1316-1333）：
```rust
pub fn leg_target(elements, e_idx, base_units, config) -> LegTarget {
    let depth = element_depth(&ElementView::new(elements), e_idx);
    let w = depth_weight(depth, config);              // w=[0.60,0.30,0.10] by depth
    let role = operation_role(elements, e_idx);
    LegTarget { e_idx, side: e.eps, units: base_units * w, role }
}
```
- `units = base_units × w_depth(depth)`——**仅消费 SizeΘ 10 参数中的 `depth` 维**（第 4 参数 N^depth）。
- **未消费** σ_higher（第 5 参数）/ I_γ / role（ sizing 维，role 当前只用于 side 判定与 ShortDiff 标记，未入 units）/ TStage / RiskMode / CostBucket / MarginState（其余 8 维）。
- 注释 coverage.rs:1288 自述「units：目标单位数 s_e（按 role/depth 权重 w_depth × 基准，M28 深度权重）」——诚实声明了 v0 的 depth-only 有效域（未膨胀，符合 090号）。

### 1.2 v1 修订目标

升级 `leg_target`（+ 双段变体 `leg_target_two_segment` coverage.rs:1338-1356，二者须同步以保 bit-exact 一致性）的 units 计算：

```
v0:  s_e = base_units × w_depth(depth)
v1:  s_e = base_units × w_depth(depth) × w_dir(ℓ_e, δ_e, σ_higher(e), role(e))
```

其中 `w_dir` 是新引入的**分级符号权重函数**（§四 形式化）。σ_higher 经此进入 q_Θ，对齐 §11 page7 `q_Θ(z,a) = SizeΘ(ℓ,δ,Iγ,N^depth,σ_higher,role,...)` 第 5 参数。

### 1.3 通道选择（为什么是 q_Θ，不是 J_Θ/K_Θ）

| 通道 | formal-chain 锚 | 消费 σ_higher？ | 裁决 |
|---|---|---|---|
| **q_Θ / SizeΘ**（腿单位层） | §11 page7 line 867 `SizeΘ(...,σ_higher,...)` 第 5 参数 | **是**（定义性） | **合法**（本 prereg） |
| J_Θ（净持仓代价函数） | §15 line 740 `J_x = ‖p−p̃‖²_W + λ·Cost + ν·RiskPenalty` 三项作用于净持仓 p | 否（p̃ 折叠后 parent_dir 信息已丢失，loss 定理 S1-3） | 否决（loss §S1 三重违反） |
| K_Θ（风险可行集） | §11 page7 line 873-888 九项全资本/保证金/交易所约束 | 否（定义域无趋势方向） | 否决（loss §S2 三重违反） |

σ_higher 是腿级属性（每条腿的父级别方向），p̃ 是账户级标量（`net_target_units` 折叠后，coverage.rs:1426）。腿级属性只能在**腿级** q_Θ 消费，不能在账户级 J_Θ/K_Θ 消费（231号 有效域：腿级变量的有效域 ⊊ 账户级函数的定义域）。

---

## 二、formal-chain 原文依据（亲读，准引用）

### 2.1 完整的策略.pdf page 4——σ_higher 是完整状态分量 + 禁全局一刀切（pdftotext -f 4 -l 4 亲读）

**完整状态 z 含 σ_higher**（page4 verbatim）：
> 「完整分类状态至少应为：z = (ℓ, e, δ, Iγ, N^depth, CandType, ForceState, Jchain, **σ_higher**, σ_p, role, posState, shortDiff, H, TStage, ...)」

σ_higher 是 z 的第 9 分量（入场时上级方向）。

**禁全局一刀切**（page4 verbatim，本 prereg 分级权的直接依据）：
> 「你们已有实验说明，**σ_higher 的收益符号会随级别翻转**，L1 是主要超 beta 来源，**不能用全局'顺上级/逆上级'一刀切**。所以完整状态必须含：...(ℓ, δ, σ_higher)。**否则会把'低级别逆上级短差'和'高级别逆上级接飞刀'混在一起。**」

——三重信息：
1. σ_higher 收益**符号**（非连续幅度）随级别翻转 ⟹ 权重须按 (ℓ, δ, σ_higher) 三元组分级（§四 w_dir 设计依据）。
2. L1（执行级别之上的第一超级别）是主要超 beta 来源 ⟹ Θ_dir_adversary 预注册套的实证动机（§六）。
3. 「低级别逆上级短差」（合法 ShortDiff）vs「高级别逆上级接飞刀」（裸逆势投机）须区分 ⟹ ShortDiff 豁免 + SameReverse 不豁免的分层依据（§五）。

### 2.2 完整的策略.pdf page 7——SizeΘ 定义 + q_Θ 入策略通道（pdftotext -f 7 -l 7 亲读）

**§11 仓位与风险投影**（page7 verbatim）：
> 「活动声部目标头寸：`p_{t+1} = Σ_{v∈A_{t+1}} q_Θ(z_v, x_t) · e^{σ_v} · v`
> 仓位函数：**`q_Θ(z, a) = SizeΘ(ℓ, δ, Iγ, N^depth, σ_higher, role, TStage, RiskMode, CostBucket, MarginState)`**
> 风险可行集：K_Θ(x_t) 必须包含：保证金；强平；最大净头寸；最大毛头寸；多空双开；OQ-9；TW 三阶段合法性；成本和滑点；交易所约束。
> 最终策略：`π_Θ(x_t) = Schedule_Θ[LexArgmin_{p∈K_Θ(x_t)} J_Θ(p, p_{t+1}) − p_t]`」

——四处亲读确认：
1. `SizeΘ` 第 5 参数 = σ_higher（✓ 与 loss §6 line 867 引用一致）。
2. q_Θ 输出进入 `p_{t+1} = Σ q_Θ · e^σ · v`（腿单位 → 净持仓 p̃ 构造层）⟹ σ_higher 经 q_Θ 入策略的通道确认。
3. K_Θ 九项约束**全资本/保证金/交易所**，无趋势方向（✓ loss §S2 否定依据）。
4. J_Θ 消费 `p_{t+1}`（= p̃，q_Θ 的输出聚合），**不直接消费** σ_higher（✓ loss 定理 S1-3）。

### 2.3 买卖点.pdf page 6——§7.5 ShortDiff 定义 + 同股数要求（pdftotext -f 6 -l 6 亲读）

**§7.4 SubFollow（顺向，对照）**（page6 verbatim）：
> 「Role(g) = SubFollow ⟹ ℓ_g < ℓ_{α(g)}, δ_g = σ_{α(g)}。语义：次级别顺父方向，可以作为同向子腿；不是短差。」

**§7.5 ShortDiff（逆向，本 prereg 保护目标）**（page6 verbatim）：
> 「Role(g) = ShortDiff ⟹ ℓ_g < ℓ_{α(g)}, **δ_g = −σ_{α(g)}**。语义：**父仓保持不变，建立独立反向子声部**。
> 形式上：p_before = s_α · e^{σ_α}。短差开启后：p_after = s_α · e^{σ_α} + s_g · e^{−σ_{ν(g)}}。
> 父仓保持：Δs_α^child = 0。
> **同股数短差要求：s_g = s_α**。
> 因此：父级多头 Q_α^+ → Q_α^+ + Q_ν^+；父级空头 Q_α^− → Q_α^− + Q_ν^−。」

——§7.5 三条对 w_dir 设计的决定性约束：
1. `δ_g = −σ_{α(g)}`：ShortDiff 定义上即逆势（不需 w_dir 判方向，role.v==ShortDiff 已标记，coverage.rs:1313 注释自述）。
2. 「父仓保持不变，建立独立反向子声部」：ShortDiff 不改父腿，只加反向子腿。
3. **`s_g = s_α`（同股数）**：ShortDiff 腿单位 = 父腿单位——这是 §7.5 对 w_dir 的硬约束（§五 定理 1）。

---

## 三、当前实装锚点（L1 逐行核实）

### 3.1 leg_target 主路径（coverage.rs:1316-1333）

见 §1.1。`units: base_units * w`，`w = depth_weight(depth, config)`。**无 σ_higher 读出**。

### 3.2 leg_target_two_segment 双段变体（coverage.rs:1338-1356）

热循环 `strategy_target_legs`（coverage.rs:1375）调双段版以复用 base 兄弟索引缓存（O(n²) 消除，注释 :1381-1385）。其 units 计算（:1353）`base_units * w` 与主路径**完全一致**（w 同为 depth_weight）。**v1 升级须同步两处**，否则双段/单段 bit-exact 分裂（注释 :1337 自述"bit-exact == leg_target"须保）。

### 3.3 σ_higher 的可得性（L1 核实：v1 输入是否就绪）

σ_higher(e) = 元素 e 的父级别方向 = `σ_{α(e)}`（§7.5 符号）。实装中：
- `operation_role`（coverage.rs，loss §S1 引）已判 `role.v==Vertical::ShortDiff`，即已隐式读出"父方向 vs 腿方向"关系。
- `role` 结构（coverage.rs:1303 `OperationRole`）含 (H, V, δ)——V=ShortDiff 标记 + δ=腿方向 ⟹ σ_higher 可从 role 与父子关系恢复（若 role.v==ShortDiff 则 σ_higher = −δ_e；若 role.v==SameFollow/SubFollow 则 σ_higher = +δ_e；RootDir 则 σ_higher=0/None）。
- **L1 结论**：σ_higher 在 v0 已隐式可得（经 role 解码），v1 升级**不需要新数据源**，只需在 leg_target 内解码 role → σ_higher 并查 w_dir 表。具体解码细节（role → σ_higher 映射）属 g2 实装范畴，本 prereg 冻结的是 w_dir 函数形式与 ShortDiff 豁免约束。

### 3.4 net_target_units 折叠（coverage.rs:1426，下游影响锚）

`p̃ = Σ_{leg} ε_e · s_e`（标量净持仓）。v1 改 s_e ⟹ p̃ 变 ⟹ 进入 J_Θ/LexArgmin 的 p̃ 变 ⟹ p\* 变（loss 定理 S1-2 同链）。bit-exact 不保留的传导起点。

---

## 四、严格修复形式：分级符号权重函数 w_dir（L0 + 设计）

### 4.1 w_dir 函数定义（本 prereg 冻结形式）

```
w_dir(ℓ_e, δ_e, σ_higher(e), role(e)) → ℝ⁺:

  CASE 1 (ShortDiff 豁免, §7.5 s_g=s_α):
    role.v == ShortDiff                         ⟹  return 1.0

  CASE 2 (根级/无上级方向, 无 σ_higher 可消费):
    σ_higher(e) ∈ {None, 0}  (RootDir)          ⟹  return 1.0

  CASE 3 (分级符号查表, §6 page4 按 (ℓ,δ,σ_higher) 分级):
    otherwise                                   ⟹  return Θ_dir[ℓ_e][sign(δ_e · σ_higher(e))]

其中 sign(δ · σ_higher):
  +1  ⟺  δ 与 σ_higher 同号  ⟺  "顺上级"（SameFollow / SubFollow 同向子腿）
  -1  ⟺  δ 与 σ_higher 反号  ⟺  "逆上级"（SameReverse 同级反向，非 ShortDiff）
```

**v1 units 完整式**：`s_e = base_units × w_depth(depth) × w_dir(ℓ_e, δ_e, σ_higher(e), role(e))`

### 4.2 形式选择依据（为什么是查表，不是线性/连续）

| 候选形式 | formal-chain 支持 | 裁决 |
|---|---|---|
| **分级符号查表**（本 prereg） | §6 page4「收益**符号**随级别翻转」——符号级、按 ℓ 分行 | **选**（信息粒度匹配原文） |
| 线性连续 `w_dir = a + b·(δ·σ_higher)` | 原文无连续幅度响应支持 | 否（声明膨胀，090号：声明原文未给的能力） |
| 分段连续（按 ℓ 分段 + 段内线性） | 同上，段内线性无依据 | 否（同上） |
| 全局符号二值 `{1.0, η}`（不分 ℓ） | 违反 §6 page4「不能一刀切」「须含 (ℓ,δ,σ_higher)」 | 否（§6 page4 明文违反） |

查表是最保守、最可证伪、信息粒度与原文严格匹配的形式。表的每一行（每个 ℓ）+ 每个符号槽（±1）是一个独立 Θ 参数，须冻结先于跑数（135号）。

### 4.3 三套预注册（对齐 关于背驰.pdf §9.2 + loss §S1-legit 第4点）

```
Θ_dir_follow（顺势保权假设）:
  对每个 ℓ:  Θ[ℓ][+1] = 1.0,  Θ[ℓ][-1] = η_adv(ℓ) < 1.0
  （顺上级腿保权，逆上级 SameReverse 腿降权）

Θ_dir_neutral（v0 对照基线）:
  对每个 ℓ:  Θ[ℓ][+1] = 1.0,  Θ[ℓ][-1] = 1.0
  （w_dir 恒等退化为 v0，bit-exact == v0，作对照）

Θ_dir_adversary（逆势保权假设，§6 page4 "L1 主要超 beta 来源"）:
  对每个 ℓ:  Θ[ℓ][+1] = η_same(ℓ) < 1.0,  Θ[ℓ][-1] = 1.0
  （逆上级 SameReverse 腿保权，顺上级腿降权）
```

- 三套**并行 OOS，不能事后选**（§9.2「不能先看结果再选」= 数据窥探禁令）。
- `η_adv(ℓ)` / `η_same(ℓ)` 是新 Θ 参数族（每级别一个），其**具体数值须在跑数前冻结**（135号：冻结先于跑数）。本 prereg 冻结的是**函数形式与三套结构**，数值冻结是 g3 跑数前的独立步骤（由编排者授权具体数值表，或采用标准网格如 {0.5, 0.7, 1.0}）。
- Θ_dir_neutral 不是"无用套"——它是 v0 bit-exact 对照，验证 w_dir 实装正确性（若 neutral 套 OOS ≠ v0 基线 = 实装 bug）。

### 4.4 与 depth_weight 的正交性

`w_depth(depth)`（coverage.rs:1325，[0.60,0.30,0.10] by 嵌套深度）与 `w_dir(ℓ,δ,σ_higher,role)` 是**两个正交维度**：
- w_depth 消费 N^depth（SizeΘ 第 4 参数，嵌套树深度）。
- w_dir 消费 σ_higher（SizeΘ 第 5 参数，父级别方向）。
- 二者乘积 `w_depth × w_dir` 是 v1 对 SizeΘ 第 4+5 参数的联合消费，符合 §11 page7 十参数定义（v0 只实装第 4，v1 扩展到 4+5）。

---

## 五、ShortDiff 合法性保护（§7.5 s_g=s_α 推论，本 prereg 核心）

### 5.1 定理 1（ShortDiff 腿 w_dir ≡ 1 是 §7.5 的必要条件，L0）

**命题**：`w_dir(role=ShortDiff) ≡ 1.0` 是 §7.5 同股数要求 `s_g = s_α` 的必要条件。

**证明**：
- §7.5 page6 明文：ShortDiff 腿 g 与其父腿 α 满足 `s_g = s_α`（同股数短差要求）。
- v1 units 式：`s_g = base_units × w_depth(depth_g) × w_dir(g)`，`s_α = base_units × w_depth(depth_α) × w_dir(α)`。
- ShortDiff 是次级别（ℓ_g < ℓ_{α(g)}，§7.5），depth_g ≠ depth_α 一般成立 ⟹ `w_depth(depth_g) ≠ w_depth(depth_α)` 一般成立。
- 为保 `s_g = s_α`，须 `w_depth(depth_g) × w_dir(g) = w_depth(depth_α) × w_dir(α)`。
- 最保守（不恶化既有张力，见 §5.2）且最简的形式：`w_dir(g) = w_dir(α) = 1.0`，此时 `s_g/s_α = w_depth(depth_g)/w_depth(depth_α)`——与 **v0 完全一致**（v0 的 depth_weight 已应用于 ShortDiff 腿）。
- 若 `w_dir(g) ≠ 1.0`：引入额外偏离因子 ⟹ `s_g/s_α` 偏离 v0 既有的 depth_weight 比例 ⟹ **额外恶化** s_g=s_α 张力 ⟹ 违反 §7.5（比 v0 更违反）。
- 故 `w_dir(role=ShortDiff) ≡ 1.0` 是 v1 升级**不额外恶化** §7.5 的充要条件。∎

**推论**：loss §S1-legit 第2点「w_dir 不得归零 ShortDiff，只能缩放」**订正**为「w_dir(ShortDiff) ≡ 1.0，不缩放」——§7.5 s_g=s_α 比"不归零"更强。

### 5.2 既有张力的诚实声明（v0 depth_weight vs §7.5 s_g=s_α）

v0 的 `s_e = base_units × w_depth(depth)` 应用于 ShortDiff 腿时，`s_g = base_units × w_depth(depth_g)` 而 `s_α = base_units × w_depth(depth_α)`——当 depth_g ≠ depth_α 时 `s_g ≠ s_α`，与 §7.5 同股数要求有张力。

**此张力是 v0 既存问题，非本 prereg（σ_higher 升级）引入**：
- 本 prereg 的 `w_dir(ShortDiff) ≡ 1.0` 保证 σ_higher 升级**不额外恶化**该张力（s_g/s_α 比例与 v0 一致）。
- depth_weight 是否应豁免 ShortDiff 腿（以严格满足 s_g=s_α）是**独立的既存问题**，超出 q_Θ v0→v1（σ_higher 维）范畴，登记供后续独立预注册审查（若编排者授权）。本 prereg 不处理。

### 5.3 ShortDiff vs SameReverse 的分层（§6 page4 区分依据）

§6 page4 明文须区分「低级别逆上级短差」（ShortDiff，合法对冲）vs「高级别逆上级接飞刀」（SameReverse 裸逆势投机）：
- **ShortDiff**（role.v==ShortDiff）：w_dir ≡ 1.0（§5.1 定理 1，§7.5 保护）。
- **SameReverse**（role.v≠ShortDiff 但 δ=−σ_higher，同级反向）：w_dir 走 CASE 3 查表，sign(δ·σ_higher)=−1 ⟹ 查 Θ_dir[ℓ][-1]（可降权/保权，由预注册套决定）。
- **SameFollow/SubFollow**（δ=σ_higher，顺向）：w_dir 走 CASE 3 查表，sign=+1 ⟹ 查 Θ[ℓ][+1]。

分层由 role + sign(δ·σ_higher) 自动实现，不需额外开关。ShortDiff 的保护是结构性的（CASE 1 优先于 CASE 3）。

---

## 六、bit-exact 声明（不保留，v0→v1 扩展）

### 6.1 不保留结论（定理 2，L0）

**定理 2**（q_Θ v1 改变信号集）：设 v0 信号集 S_0 = {(entry_i, exit_i)}，v1 信号集 S_1（某预注册套）。若 w_dir 不恒等 1.0（即非纯 Θ_dir_neutral 套），则 ∃ 腿 e 使 `s_e^v1 ≠ s_e^v0`。

**证明**：
- `s_e^v0 = base_units × w_depth(depth_e)`，`s_e^v1 = base_units × w_depth(depth_e) × w_dir(e)`。
- 若 w_dir(e) ≠ 1.0 ⟹ s_e^v1 ≠ s_e^v0。
- s_e 进入 `p̃ = Σ ε·s_e`（coverage.rs:1426）⟹ p̃^v1 ≠ p̃^v0。
- p̃ 进入 `p* = LexArgmin_{p∈K_Θ} J_Θ(p, p̃)`（§11 page7，coverage.rs:2416-2424）⟹ p\*^v1 ≠ p\*^v0（J_Θ 主键 `‖p−p̃‖²_W` 最优点随 p̃ 漂移）。
- p\* 进入 `schedule_order(p* − p_t)`（coverage.rs:2466）⟹ 订单 O_{t+1} 变 ⟹ 成交序列变 ⟹ ledger 变 ⟹ 信号集变 ⟹ **bit-exact 不保留**。∎

**推论**：v1 升级触发 135号冻结纪律——必须先冻结 w_dir 表（本 prereg）再跑 OOS，不能先看结果再选（§9.2）。

### 6.2 唯一 bit-exact 保留情形

**Θ_dir_neutral 预注册套**：w_dir ≡ 1.0（所有 CASE 返回 1.0）⟹ s_e^v1 = s_e^v0 ⟹ p̃/p\*/订单/ledger 全不变 ⟹ bit-exact == v0。

此套的 OOS 用途：**实装正确性自检**——若 Θ_dir_neutral 套 OOS 结果 ≠ v0 基线（af8910d062 的 Π_max-full OOS），则 w_dir 实装有 bug（如误把 ShortDiff 腿 w_dir 设为非 1，或 sign(δ·σ_higher) 解码错误）。这是 g2 实装验收的硬测试。

### 6.3 与既有冻结的关系

- v0 冻结（135号）的 bit-exact 基线 = af8910d062（Π_max-full 端到端 OOS，INCONCLUSIVE）。
- v1 三套预注册产生三个独立信号集（follow/neutral/adversary），各自独立 OOS，互不 bit-exact（除 neutral = v0）。
- 不存在"v1 是 v0 的微调"——v1 是 SizeTheta 维度扩展（depth → depth+σ_higher），是结构升级非参数微调。

---

## 七、fail 条件 + 认识论等级标注

### 7.1 fail 条件（准入/实装失败）

| fail 情形 | 实装行为 | 合法性 |
|---|---|---|
| w_dir(ShortDiff) ≠ 1.0 | 破坏 §7.5 s_g=s_α（§5.1 定理 1） | **非法**（§7.5 违反，g2 实装 bug） |
| w_dir 表非分级（全局二值，不分 ℓ） | 违反 §6 page4「不能一刀切」「须含 (ℓ,δ,σ_higher)」 | **非法**（§6 page4 明文违反） |
| w_dir 用线性连续形式 | 声明膨胀（原文无连续幅度支持） | **非法**（090号） |
| σ_higher 经 J_Θ 或 K_Θ 通道 | loss §S1/§S2 三重违反 | **非法**（通道错误，本 prereg §1.3 否决） |
| 三套预注册事后选其一 | §9.2 数据窥探 | **非法**（§9.2 违反） |
| Θ_dir_neutral 套 OOS ≠ v0 基线 | w_dir 实装 bug（sign 解码错/ShortDiff 未豁免） | **实装 fail**（g2 验收硬测试，须修） |
| σ_higher=0（RootDir）腿 w_dir ≠ 1.0 | 对无上级方向的腿强加方向权重 | **非法**（无 σ_higher 可消费，CASE 2 豁免） |

### 7.2 认识论等级标注（231号）

| 陈述 | 等级 | 依据 |
|---|---|---|
| σ_higher 是 SizeΘ 第 5 参数（须经 q_Θ 入策略） | **L0** | §11 page7 亲读 + §6 page4 状态分量 |
| §6 page4 禁全局一刀切（须按 (ℓ,δ,σ_higher) 分级） | **L0** | page4 亲读 verbatim |
| §7.5 ShortDiff s_g=s_α ⟹ w_dir(ShortDiff)≡1（定理 1） | **L0** | page6 亲读 + 代数推论 |
| w_dir 查表形式（非线性/连续） | **L0** | §6 page4 信息粒度匹配（符号级） |
| leg_target v0 仅消费 depth（未消费 σ_higher） | **L1** | coverage.rs:1316-1333 静态核对 |
| leg_target_two_segment 须同步升级 | **L1** | coverage.rs:1338-1356 静态核对 |
| σ_higher 经 role 隐式可得（不需新数据源） | **L1** | coverage.rs:1303 OperationRole 结构核对 |
| q_Θ v1 改变信号集（定理 2，bit-exact 不保留） | **L0** | p̃→p*→订单 传导链（coverage.rs:1426/2416/2466） |
| σ_higher 收益符号"随级别翻转"（分级数值依据） | **L2** | §6 page4「实验说明」（实证，非 L0 可导） |
| 三套预注册 OOS 结果（哪套优） | **L2/L3** | **不在本 prereg 范围**，全 INCONCLUSIVE |
| η_adv(ℓ)/η_same(ℓ) 具体数值 | **L2** | 须 g3 跑数前冻结，本 prereg 只冻结构 |

---

## 八、边界

### 8.1 有效域（σ_higher 权重的适用范围）

- **§6 page4 「收益符号随级别翻转」是 L2 实证**（「实验说明」），非 L0 可导 ⟹ w_dir 表的**分级结构**（按 ℓ 分行）是 L0（§6 page4 要求），但每行的**具体数值**（η_adv(ℓ)/η_same(ℓ)）须 L2/L3 标定。本 prereg 冻结结构，不冻结数值。
- **σ_higher = 0/None（RootDir，根级无上级方向）**：无 σ_higher 可消费 ⟹ w_dir = 1.0（CASE 2）。这是有效域的自然边界——q_Θ v1 只对有父级别方向的腿生效。
- **BTC/单标的 L2 有效域**：三套预注册的 OOS 结论首先在 BTC（loss 诊断同标的）有效；跨标的（L3）须独立验证，不外推。

### 8.2 与 opsem-redo 4 族亏损的关系（正交，非直接修复）

loss-research-final-summary §三 明确：q_Θ σ_higher 升级是**正交的 v0→v1 增强**，非 4 族亏损的直接修复：
- 族 A（超大持仓）：根因 Stop 触发缺陷（typed exit 触发质量，§S3 范畴）。
- 族 B（震荡择时）：短差止损择时（exit 侧）。
- 族 C（背驰持仓久）：ReduceCore 未发（typed exit 触发质量）。
- 族 D（减仓时机）：三类买卖点识别（R1/R2 信号侧）。

σ_higher 分级 sizing 不直接修上述任何一族。若 v1 三套 OOS 全 INCONCLUSIVE（无方向 alpha），则 q_Θ v1 与 signal 层无 alpha（loss 双根因①）一致——不翻转 INCONCLUSIVE。

### 8.3 结论翻转条件

- **本 prereg 形式（w_dir 查表 + ShortDiff 豁免）翻转**：仅当编排者裁定采线性连续形式（须接受 090号声明膨胀代价），或裁定 ShortDiff 也缩放（须接受 §7.5 s_g=s_α 违反代价）。两者皆选择类决断（/escalate），非 bug。
- **不翻转的部分**：σ_higher 须经 q_Θ 通道（§11 page7 定义性）/ 禁全局一刀切（§6 page4 明文）/ ShortDiff 定义即逆势（§7.5 page6）/ s_g=s_α（§7.5 page6 明文）——四者皆 formal-chain 已结算定义，不随数据/配置翻转。

---

## 九、结果包六要素

### 1. 结论

- **冻结 q_Θ v0→v1 升级形式**：`s_e = base_units × w_depth(depth) × w_dir(ℓ,δ,σ_higher,role)`，w_dir 为分级符号查表（CASE 1 ShortDiff 豁免 / CASE 2 根级豁免 / CASE 3 按 (ℓ, sign(δ·σ_higher)) 查表）。
- **ShortDiff 合法性保护订正 loss §S1-legit**：§7.5 page6 明文 `s_g=s_α` ⟹ w_dir(ShortDiff) ≡ 1.0（定理 1），比 loss 所述"不归零只缩放"更强——不缩放。
- **通道确认**：σ_higher 经 q_Θ/SizeΘ 第 5 参数（§11 page7）入策略，不经 J_Θ（§15 三项封闭）/K_Θ（§11 九项资本约束）——后两者 loss §S1/§S2 已三重违反否决。
- **形式选择**：分级符号查表（§6 page4 符号级信息匹配），非线性/连续（声明膨胀）/全局二值（一刀切违反）。
- **三套预注册**：Θ_dir_follow / Θ_dir_neutral（v0 bit-exact 对照）/ Θ_dir_adversary，并行 OOS 不能事后选（§9.2）。
- **bit-exact 不保留**（定理 2），唯一保留情形 = Θ_dir_neutral 套（作 g2 实装自检）。

### 2. 定义依据

- **完整的策略.pdf page 4**（pdftotext -f 4 -l 4 亲读）：完整状态 z 含 σ_higher（第 9 分量）+「σ_higher 收益符号随级别翻转，不能用全局顺/逆上级一刀切，完整状态须含 (ℓ,δ,σ_higher)，否则把低级别逆上级短差和高级别逆上级接飞刀混在一起」（§2.1）。
- **完整的策略.pdf page 7**（pdftotext -f 7 -l 7 亲读）：`q_Θ(z,a)=SizeΘ(ℓ,δ,Iγ,N^depth,σ_higher,role,TStage,RiskMode,CostBucket,MarginState)` 第 5 参数 + `p_{t+1}=Σ q_Θ·e^σ·v`（腿单位→p̃ 通道）+ K_Θ 九项资本约束 + `π_Θ=Schedule[LexArgmin J_Θ(p,p̃)−p_t]`（§2.2）。
- **买卖点.pdf page 6**（pdftotext -f 6 -l 6 亲读）：§7.5 `Role(g)=ShortDiff ⟹ δ_g=−σ_{α(g)}` +「父仓保持不变，建立独立反向子声部」+ **`s_g=s_α`（同股数短差要求）**（§2.3，定理 1 依据）+ §7.4 SubFollow 对照。
- **代码锚点**（全部 L1 逐行核实）：coverage.rs:1316-1333（leg_target v0 depth-only）、:1338-1356（双段变体须同步）、:1303（OperationRole 结构，σ_higher 隐式可得）、:1426（net_target_units p̃ 折叠，bit-exact 传导起点）、:2416-2424（pi_theta_position LexArgmin）、:2466（schedule_order）。
- **先例**：loss-solution-proposals-20260705.md §S1-legit（commit 77006b82dc，唯一合法 kernel）+ loss-research-final-summary-20260705.md §三（双根因 + 唯一合法修复背书）+ prereg-rev3（f1 冻结格式先例）。

### 3. 边界条件（结论翻转）

- **本 prereg 形式翻转**：仅当编排者裁定采线性连续 w_dir（090号声明膨胀代价）或 ShortDiff 也缩放（§7.5 s_g=s_α 违反代价）——皆选择类（/escalate），非 bug。
- **bit-exact 翻转**：仅 Θ_dir_neutral 套保留（w_dir≡1 退化 v0）；follow/adversary 套必不保留（定理 2）。
- **不翻转**：σ_higher 经 q_Θ 通道（§11 page7）/ 禁一刀切（§6 page4）/ §7.5 ShortDiff 定义 + s_g=s_α——四者皆 formal-chain 已结算，不随数据/配置翻转。
- **有效域边界**：σ_higher=0（RootDir）腿 w_dir=1.0（无 σ_higher 可消费）；w_dir 数值（η_adv/η_same）须 L2 标定，L0 只定结构。

### 4. 下游推论

- **对 g2（实装工位）**：升级 `leg_target`（coverage.rs:1316）+ `leg_target_two_segment`（:1338）签名，增 w_dir 函数 + 三套 Θ_dir 表；硬约束 w_dir(ShortDiff)≡1.0（§5.1 定理 1）；Θ_dir_neutral 套作实装自检（OOS 须 = v0 基线 af8910d062，否则 bug）。
- **对 g3（跑数工位）**：三套预注册（follow/neutral/adversary）并行 OOS，不能事后选（§9.2）；η_adv(ℓ)/η_same(ℓ) 数值表须跑数前冻结（135号）；首跑 Θ_dir_neutral 验证 bit-exact == v0。
- **对 loss 双根因**：q_Θ v1 是 signal 层 alpha 探索的 v0→v1 扩展，不直接修 execution 层 typed exit 触发质量（4 族亏损根因，loss §三）。若 v1 三套全 INCONCLUSIVE ⟹ 与 signal 层无 alpha 一致，不翻转 INCONCLUSIVE。
- **对 Π_max-full OOS（af8910d062）**：v0 基线 OOS 不变；v1 产生三个新 OOS（follow/neutral/adversary），各自独立 INCONCLUSIVE 判定。
- **对 135号冻结纪律**：本 prereg 冻结 w_dir 函数形式 + 三套结构（g1）；η 数值表冻结是 g3 跑数前独立步骤；g2 实装依赖本 prereg 的 commit 冻结哈希。

### 5. 谱系引用

- **无新谱系**：本 prereg 是 g1 冻结文档（研究+形式化），未检测到需结晶的稳定信号（权重函数设计是定义层推导，非经验发现）。
- **相关已结算谱系**：
  - **090号**：严格性语法规则——禁止声明膨胀（线性连续 w_dir 形式被否的依据：声明原文未给的连续幅度能力）；禁止补丁思维。
  - **161号**：否定性照实——loss §S1/§S2/§S3 三方向否定的照实记录，导出唯一合法 kernel（本 prereg 形式化）。
  - **231号**：有效域<定义域——σ_higher 收益符号翻转是 L2 实证（有效域），w_dir 表数值须 L2 标定非 L0 可导；本 prereg 每节标注 L0/L1。
  - **135号**：冻结先于跑数——本 prereg 是 g1 冻结文档，commit 后 g2/g3 才开始；η 数值表须 g3 跑数前独立冻结。
  - **639号**：σ_p 来源修正——loss §S2 核实确认 AncOK 裸逆势门控已实装（coverage.rs:1753-1774），本 prereq 的 SameReverse（非 ShortDiff 的逆势）走 CASE 3 查表与此一致（SameReverse 父在 raw，非 naked，不被 AncOK 剪）。
  - **698号**：M8 归因双修正——loss 双根因（signal 无 alpha + typed exit 触发质量），本 prereg 的 q_Θ v1 是 signal 层 v0→v1 扩展，正交于 execution 层第二根因。
- **不确定是否有相关谱系**：v0 depth_weight 应用于 ShortDiff 腿是否已在某谱系中标为"§7.5 s_g=s_α 张力的既存问题"——未检索到明确谱系（§5.2 登记为独立既存问题，待后续审查）。

### 6. 影响声明

- **代码改动**：零。本 prereg 是 g1 冻结文档（研究+形式化），未改动任何代码。代码升级在 g2。
- **文档改动**：新建 `/Users/silencehan/Projects/NewChanlun/.chanlun/review-results/prereg-rev4-qtheta-sigma-higher-20260705.md`（本文件）。
- **对实装的影响**：冻结 q_Θ v0→v1 升级的严格形式（w_dir 分级符号查表 + ShortDiff 豁免 + 三套预注册）。g2 依此实装；g3 依此跑数。ShortDiff w_dir≡1.0 是硬约束（§5.1 定理 1），Θ_dir_neutral 是实装自检基线。
- **对 loss 研究链的影响**：订正 loss §S1-legit 第2点（w_dir ShortDiff 从"不归零只缩放"→"≡1.0 不缩放"，依据 §7.5 s_g=s_α 亲读）；形式化 §S1-legit 的"分级权重表"概念为具体 w_dir 函数 + 三套预注册结构。
- **诚实声明**：本 prereg 的 w_dir 函数形式（查表 + 三套预注册 + ShortDiff 豁免）基于 formal-chain 原文（3 PDF 亲读 page4/page7/page6）+ loss 研究链（commit 77006b82dc）+ 代码 L1 核实。若编排者对 §6 page4「符号随级别翻转」有不同解读（如认为支持连续响应），或对 §7.5 s_g=s_α 的强度有不同判定（如认为同股数是软约束），结论可翻转——前者属形式选择（选择类，/escalate），后者属定义强度判定（选择类，/escalate）。η_adv(ℓ)/η_same(ℓ) 的具体数值不在本 prereg 范围，须 g3 跑数前独立冻结。

---

**prereg-rev4 冻结结束**。q_Θ v0→v1 升级 = w_dir 分级符号查表（ShortDiff §7.5 s_g=s_α 豁免 + 根级豁免 + (ℓ,sign(δ·σ_higher)) 分级）× 三套预注册（follow/neutral/adversary）。bit-exact 不保留（除 neutral=v0 对照）。下游 g2 实装 / g3 跑数依赖本文件 commit 冻结哈希。
