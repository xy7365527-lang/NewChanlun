# β^div 背驰强度进桶键——设计稿（阶段1 纯设计，只读）

- 工位：ws-beta（task #104，编排者令「候选后续全做」）
- 日期：2026-07-03
- 规格来源：`docs/formal-chain/关于背驰.pdf`（14 页，标题「推导完全分类」，力度=支配序主论证）+ `docs/formal-chain/alpha分离.pdf` p2 方框 `Z_i=(ℓ,δ,I_γ,σ_higher,r,ω,β^div,d,c)` + `z-bucket-impl-20260702.md`（九维对照，β^div 行标「数据源部分在，未接入 z」）
- 认识论等级：**L0**（本稿是纯定义/结构设计，不含数据验证，零信息增量）——分箱边界的 alpha 有效性是 L2/L3，须 OOS，本稿不声明
- 状态：**设计冻结待 codex 审计**。实装排期由 team-lead 送审后定。

---

## 1. 结论（核心：β^div 不是标量，原文忠实形态是 ForceState 四值支配序）

`关于背驰.pdf` 的定理 2（p7）**证明了**：允许力度族 𝒜_ℓ 上不存在无额外公理的唯一全序力度。存在 s,s' 与两个力度函数 m₁,m₂ 使 `m₁(s)<m₁(s')` 但 `m₂(s)>m₂(s')`——任意 AND/OR/词典序/加权和都是**额外选择 Θ**，非原文唯一推出。因此：

> **把「背驰强度 β^div」实现为单一 proxy 的连续标量（MACD 面积比、DIF 比、振幅比之一）在原文上是非法的**——它是 Θ_SCORE 一条预注册路线，不是 β^div 的定义。

原文自己给出的 z 力度分量（§9.1，p10）是：

```
ForceState_ℓ(s,s') = {Dominated, Dominates, Incomparable, Tie}
Z = (ℓ, δ, I_γ, ForceState, MACD, DIF, Amplitude, Speed, …)  → 进入 μ(z,a) 或选择器
```

**故 β^div 的原文忠实进桶键 = `ForceState`（四值支配序），不是连续量 K 分箱。** 这直接细化了现有 `UClass.divergence: bool`：
- 现 `divergence bool` = C 段在**冻结 MACD 面积**判据上弱于 A 段（`z.i_class & 0b001001 != 0`，即 I_γ 含一类点 B1/S1）。这是单 proxy 二值（Θ=MACD 面积）。
- `ForceState` 把它细化为**多 proxy 支配四态**：`Dominated`（C 在所有允许口径上 ≤ A 且至少一个 <，=确定背驰）/`Dominates`（C 在所有口径上 ≥ A，=确定非背驰/力度延续）/`Tie`（全等）/`Incomparable`（口径冲突，第17课「黄白线弱但面积强」类）。

**「候选后续全做」的诚实回应**：任务标题写「连续量进桶键」，但源文档的形式化结论否定了连续标量作为 β^div 定义。本稿不 workaround（不硬造一个标量塞进去假装是 β^div，no-patch §5），而是：**主键 = ForceState（原文直接支持，无额外公理）；连续标量 K 分箱作为 Θ_SCORE 预注册候选之一（§9.2 三套 OOS 之一），不作默认**。

---

## 2. β 定义选型（多 proxy 中选哪个作 β^div）

### 2.1 现有力度 proxy 盘点（`divergence.rs` `ForceFeatures`）

| proxy | 代码字段 | 值域 | 方向性（δ-共线风险） | 原文对应（§5 p6 允许力度族 𝒜_ℓ） |
|-------|---------|------|-------------------|-------------------------------|
| MACD 段面积 | `macd_area = Σ\|hist\|` | ≥0 | **非方向**（绝对值和） | `m = MACDarea` |
| DIF 峰值 | `dif_peak`（up→max.max(0)/down→\|min\|） | ≥0 | **非方向**（`segment_dif_peak` 取绝对峰） | `m = DIFdistance` |
| 价格振幅 | `price_amplitude = \|端价差\|` | ≥0（i64 tick） | **非方向**（`.abs()`） | `m = \|ΔP\|` |
| 价格速度 | `price_speed = \|端价差\|/Δbar` | ≥0 | **非方向** | `m = \|ΔP\|/T` |

**关键核实（任务硬前提）**：四个 proxy **全部是绝对量**（`segment_macd_area` 用 `h.abs()`，`segment_dif_peak` 对 down 段取 `\|min\|`，`price_amplitude/speed` 用 `.abs()`）。**β 由这些绝对量构造 ⟹ 结构上非方向 ⟹ β ⊥ δ（构造级非共线）**。这满足记忆 [[project_oddeven_mu_identity]] 的硬前提：`i_class` 之所以与 δ 共线是因为 i_class 的 6-bit 把买/卖点编码进去了（bit0-2 买、bit3-5 卖，δ 可由 i_class 恢复）；β 的 proxy 不含符号，不重蹈此覆辙。**若未来引入带符号 proxy（如有向 hist 差），必须先取绝对值/对称化**，否则 β 变成 δ 的代理，置换检验自毁。

### 2.2 选型：不选单一 proxy，选支配序（ForceState）

原文（关于背驰.pdf 全文主论证 + 第17课「黄白线主，面积次，均线/MACD 都是辅助」+ 第34课）的裁决：

- **不用 MACD 面积作唯一判据**（§3 p3-4：`M_MACD 不是 𝔉_ℓ 的充分统计量`，存在 s₁≠s₂ 使 `M_MACD(s₁)=M_MACD(s₂)` 但结构不同）。
- **不用任何单一 proxy 作 β 标量**（定理 2 p7）。
- **用支配序**（§5-6 p6）：`s ≺_𝒜 s' ⟺ ∀m∈𝒜, m(s)≤m(s') ∧ ∃m, m(s)<m(s')`。三值 `{Weak, NotWeak, Incomparable}`（§7.2 p8），工程二值化须引入 Θ 处理 Incomparable（§7.2）。

**β^div := ForceState（4 值），𝒜 = {macd_area, dif_peak, price_amplitude, price_speed}**（现有四 proxy）。

**诚实有效域标注（formalization-validity-domain）**：原文 §5（p6）的完整 𝒜_ℓ 还含 `TV=Σ\|P_{t+1}−P_t\|`（全变差）和 `Σ_{次级别同向段} m_{ℓ-1}(u)`（递归力度）。现有 `ForceFeatures` **缺 TV 和递归力度**。故本 ForceState 是**四 proxy 上的支配序，是完整支配序的近似（有效域 < 定义域）**。补 TV 是低成本（`segment_total_variation` 一个纯函数），补递归力度需塔的次级别段力度递归（大工程）。**本稿诚实声明：ForceState = 4-proxy 支配序，非原文完整 𝔉_ℓ 支配序**，不虚报（231号）。

---

## 3. δ-共线检查设计

### 3.1 构造级（结构论证，已完成）

见 §2.1 表：四 proxy 全绝对量 ⟹ ForceState 由方向无关的量比较得出 ⟹ β ⊥ δ **by construction**。无带符号分量渗入。

### 3.2 经验级（L2 验证设计，实装后跑）

构造级非共线是必要不充分——须核实**经验分布**上 ForceState 与 δ 在桶内混合（记忆的教训：共线维在置换检验上自毁）。设计：

1. **联合分布交叉表**：对每个 `(ℓ, i_class)` 层（固定级别与买卖点类型），构造 `ForceState × δ` 2×4 列联表（δ∈{+1,−1} × 4 态）。
2. **混合度判据**：每个 ForceState 值内，δ=+1 与 δ=−1 的观测数都须 >0（非 δ-纯）。定义混合度 `mix(fs) = min(n_{fs,+1}, n_{fs,−1}) / max(1, n_{fs,+1}+n_{fs,−1})`。`mix=0`（δ-纯桶）⟹ 该 ForceState 桶完全由单方向占据 ⟹ 与 δ 共线 ⟹ 在该桶上对 δ 做置换检验自毁（S_perm 退化）。
3. **卡方独立性**：χ²(ForceState ⊥ δ | ℓ, i_class)。p 值大（不拒独立）= 非共线（期望结果）；p 小（拒独立）= 共线警告，须回查 proxy 是否漏了符号。
4. **报告**：每 `(ℓ,i_class)` 层输出 mix 向量 + χ² p 值。任一 δ-纯桶 ⟹ 该 β 分桶在该层不可用于 δ-置换，降级到「仅作 μ 分层不作置换单元」。

**对称化方案（若检查失败）**：若某 proxy 事后发现带符号污染，方案是**取绝对值 + 分箱对称化**——把 β 值域折叠为 `\|β_signed\|` 或对 δ=+1/−1 分别中心化后合并。但**当前四 proxy 无需对称化**（已全绝对量）。

---

## 4. 分箱方案

### 4.1 ForceState（主键）——无连续分箱，4 个离散态

ForceState 是**分类量**（4 值枚举），不是连续量，**不需要 K 分箱与边界选择**。这是选它作主键的额外优势：避开了「K 的选择」与「分箱边界冻结」的自由度（每个边界都是一个 Θ 选择，会引入过拟合自由度，§29-30 winner's curse）。

进桶后果（样本量）：现有 canonical Z 已高维稀疏（`z-bucket-impl` 主桶实测 n=4/19/1，记忆 [[project_oddeven_mu_identity]] 载 12 类主桶已 winner's curse）。ForceState ×4 **进 canonical Z 使桶数 ×4** ⟹ n̄ = M/(K_Z·4) 进一步缩小。故：

- **ForceState 进 canonical Z**（`MuClass` 加维，作 oracle 上界 / 报告，§13 细分类优势定理的可计算落点）。
- **ForceState 不进默认 selection/UClass**（降维层保持现有 `divergence: bool`，抗 winner's curse，同 codex #81 对 H 轴的 `h_axis_in_default_selection: conditional` 裁定）。
- 升级路径：`UClass.divergence: bool` → `UClass.force_state: ForceState`（4 态替 2 值）**仅当 OOS-value-gated（§11）证明 4 态比 2 值多出的 μ 区分度 > 稀疏代价**。否则保 bool。

### 4.2 Θ_SCORE 连续标量（预注册候选之一，非默认）——K 分箱

若 codex/编排者裁定必须有连续量 β（如为了平滑分层或做连续回归），则走 Θ_SCORE。设计：

- **归一化标量**：`β_norm = (m_A − m_C) / (m_A + m_C) ∈ (−1, 1)`，其中 m = 指定单一 proxy。背驰域 = `β_norm > 0`（C 弱于 A）；`β_norm ≤ 0` = 非背驰。用**归一化差**而非**比 `m_C/m_A`**：有界、对称、避免 m_A→0 的爆炸。
- **m 的选择**：`dif_peak`（第17课「黄白线最重要」，`WeakThetaMode::Dif` 的 proxy）。理由：原文层级 `级别结构 ≻ DIF ≻ 面积`（§8 p8），级别已在 ℓ 维，故 DIF 是层级中最高的**单指标**力度。
- **K 分箱**：`K=3`（弱化/中性/强化），冻结边界 `{β_norm ≤ 0: 非背驰, 0 < β_norm < 0.33: 弱背驰, β_norm ≥ 0.33: 强背驰}`。**K 的选择与样本量权衡**：
  - 细分类优势定理（§13）：更细分箱在 oracle 口径下不降 alpha（补维单调）。
  - 稀疏代价（§29-30）：K 大 ⟹ n̄=M/K 小 ⟹ winner's curse。现有主桶 n=4-19 ⟹ **K≤3**，再大（K=5/10 分位箱）在当前样本量下每箱 <5 笔，估计=噪声。
  - 裁决：K=3 是「够细以分弱/强背驰、不至稀疏到无估计力」的折中。**边界 0.33 是 Θ 选择，须预注册冻结，不得事后调**（§5 纪律）。

### 4.3 进 MuClass 加维 vs 独立分层键

| 方案 | 做法 | 取舍 |
|------|------|------|
| **A. 进 MuClass 加维**（推荐 ForceState） | `MuClass` 加 `force_state: Option<ForceState>`（Option 因 `from_certificate`/`Candidate` 现无力度源，诚实标 None，同 `horizontal` 模式） | 进 canonical Z，作 oracle 上界；桶数 ×4；默认不进 selection |
| **B. 独立分层键**（Θ_SCORE 备选） | β 不进 Z，作 `ResidualTrade` 的分层维（alpha分离 §4.2 分层置换键 `s(i)=(ℓ,h_bucket,time_block,σ_higher)` 加一维 β_bin） | 不放大 Z 桶数；用于残差置换的层内控制；但须先过 §3.2 δ-共线检查（分层键 δ-纯会使层内 δ 置换自毁） |

**推荐**：**ForceState 走方案 A**（进 `MuClass`，Option 诚实），**Θ_SCORE β_bin 走方案 B**（若启用，作分层键）。两者不互斥——A 是 canonical Z 的力度分类维，B 是残差置换的分层控制维，服务不同管线。

---

## 5. 新预注册草案（§5 纪律：改 estimand 须新冻结）

改分桶键 = 改 estimand μ(z)，须新冻结（预注册），桶键/分箱边界/判据**写死**、**先注册后看结果**（关于背驰.pdf §9.2「不能先看结果再选」）。

### 5.1 三套 Θ 预注册（§9.2 p10，OOS 分别回测）

```
prereg-beta-div-20260703（草案，待 codex 审计后冻结）:

  允许力度族 𝒜 = {macd_area, dif_peak, price_amplitude, price_speed}   # 冻结：4 proxy（缺 TV/递归，诚实近似）

  Θ_DOM（主，推荐默认）:
    β^div := ForceState_ℓ(C, A) ∈ {Dominated, Dominates, Tie, Incomparable}
    判据:
      Dominated    = ∀m∈𝒜, m_C ≤ m_A  ∧  ∃m, m_C < m_A        # 确定背驰（C 力度衰减）
      Dominates    = ∀m∈𝒜, m_C ≥ m_A  ∧  ∃m, m_C > m_A        # 确定力度延续
      Tie          = ∀m∈𝒜, m_C = m_A
      Incomparable = 否则（口径冲突）                              # 工程处理：单列一态，不并入 Dominated
    进桶: MuClass.force_state = Some(ForceState)（真候选路径）; None（裸证书口径）
    selection: 默认不进（UClass 保 divergence bool）

  Θ_LEX（备选）:
    β^div := WeakLex(C, A) ∈ {Weak, NotWeak}
    判据: 级别结构 ≻ DIF ≻ 面积（现 WeakThetaMode::Lex 扩结构层）:
      DIF 可判（dif_peak_C ≠ dif_peak_A）→ 用 DIF; 否则退 macd_area   # 二值

  Θ_SCORE（备选，连续→K 分箱）:
    m := dif_peak                                                # 冻结：单 proxy=DIF（第17课黄白线主）
    β_norm := (m_A − m_C) / (m_A + m_C) ∈ (−1, 1)
    K := 3, 边界冻结 { ≤0: 非背驰, (0,0.33): 弱, ≥0.33: 强 }        # 边界不得事后调
    进桶: 分层键方案 B（先过 §3.2 δ-共线检查）

  纪律:
    - 三套各自 OOS 回测，事前注册，不 peek。
    - δ-共线检查（§3.2）是准入门：任一 Θ 的桶出现 δ-纯 ⟹ 该 Θ 在该层降级为「仅分层不置换」。
    - 认识论：分桶接入=L1（管线正确）; β^div 桶的 μ̂ 值=L2（可否证）; 「哪个 Θ 有 alpha」=L2/L3（OOS，本稿不声明）。
```

### 5.2 与已冻结 estimand 的关系

现 `MuClass` 的 estimand（含 `horizontal` 7 维）已冻结（codex #81）。加 `force_state` 是**第 8 维扩展**——须作 estimand 版本升级（`z-bucket-impl` 已有先例：加 `horizontal` 时 `from_certificate` 40+ 调用点填 None 零改动）。`force_state: Option<ForceState>` 默认 None ⟹ 现有全部路径（perm_test/wverify 按 (ℓ,bsp,δ,σ_p) 4 维手取键、pi_bsp_timing 从 Voice 构 z）**零改动**，只有真候选路径 `z_of_candidate` 显式填 Some。

---

## 6. 与 UClass 降维的关系（β 分箱是 divergence bool 的细化）

- **现状**：`UClass.divergence: bool = z.i_class & 0b001001 != 0`（I_γ 含一类点 B1/S1）。这是**单 proxy（MACD 面积经 buy1 冻结判据）二值背驰**——buy1 判据 `segments_diverge`=MACD 面积（class_index 语义冻结，护栏3）。
- **β^div 是它的细化**：`divergence bool`（背驰有/无，单口径）→ `ForceState`（背驰的多口径支配态，4 值）。二者关系：
  - `divergence=true`（有一类点）⟹ MACD 面积口径判 C<A ⟹ **通常** `ForceState ∈ {Dominated, Incomparable}`（MACD 支持弱化，其他口径可能冲突）。
  - `divergence=false` ⟹ 无一类点 ⟹ ForceState 未定义（无 A/C 背驰对）或 `{Dominates, Tie}`。
- **降维层裁决**：`project_to_u` 默认**保 `divergence: bool`**（不升 ForceState），理由同 §4.1——4 态进 selection 放大 winner's curse。ForceState 只在 canonical Z（`MuClass.force_state`）承载，作 oracle 上界与报告。升级 `UClass.divergence → force_state` 须 OOS-value-gated（§11）证明区分度 > 稀疏代价。这与 codex #81「H 进 canonical Z（accept）但不进 default selection（conditional）」同构。

---

## 实装可行性（下游推论，供 codex 审计与排期）

**数据源路由缺口（诚实标注，非本稿修复）**：`z_of_candidate(c)` 的 `c: Candidate`（`interp.rs`）**不携带 ForceProxies**——字段仅 `{level, source_index, bits, dir, bsp_class, role, nest_confirmed, gamma_index}`。ForceProxies 在 `signal.rs` 的 `Vec<(BspPoint, Option<ForceProxies>)>` 已并置（段坐标透传，#10/#19），但**未透传到 collect_signals 的 z 构造点**。这与 `divergence.rs` §A2 记录的同一结构缺口（「接通需 Candidate 携 A/C 段力度 feature，但 assemble_gamma 候选构造作用域只有 BspPoint」）。

实装 ForceState 进 z 需二选一（**实装工位定，非本设计稿定**）：
- **(a) Candidate 携 ForceProxies**：`interp.rs`/`assemble_gamma` 把 A/C 段力度透传进 `Candidate.force`，`z_of_candidate` 从 `c.force` 算 ForceState。改动面大（改 Candidate 构造管线）。
- **(b) collect_signals 就地重算**：在 `collect_signals` 用 `entry_bar` 从并置的 BspPoint ForceProxies vec 取 A/C 段，算 ForceState 后 struct-update 进 z。改动面小（局部），但 collect_signals 需能访问 ForceProxies vec（当前用 Candidate 不用并置 vec）。

**推荐 (b)**（局部、低耦合），但最终由实装工位在 codex 审计后定。本稿只声明：**数据源在（ForceProxies 已算），路由未接（z 构造点拿不到）**——这是实装成本，不是定义缺口。

---

## 结果包六要素

1. **结论**：β^div 的原文忠实进桶键 = `ForceState`（4 值支配序，𝒜=4 proxy），**不是连续标量 K 分箱**（定理 2 否定单一标量作 β 定义）。ForceState 进 canonical `MuClass`（第 8 维，Option 诚实），默认不进 selection/UClass。连续标量 K 分箱降为 Θ_SCORE 预注册候选之一（§9.2 三套 OOS）。

2. **定义依据**：`关于背驰.pdf` §9.1（p10，`Z=(…,ForceState,MACD,DIF,Amplitude,Speed)`——原文 z 力度分量是 ForceState 非标量）；定理 2（p7，无唯一全序力度，标量=额外 Θ）；§5-6（p6，支配序 `s≺_𝒜s'`）；§9.2（p10，三套 Θ 预注册 OOS）；§8（p8，级别 ≻ DIF ≻ 面积）。代码：`divergence.rs` `ForceFeatures`（4 proxy 全绝对量）/`ForceProxies`（A/C 并置）；`mu_estimator.rs:79 MuClass`（加 `force_state`）/`:154 UClass.divergence`（β 细化对象）；`selector.rs:144 z_of_candidate`（进 z 点）。

3. **边界条件（结论翻转）**：
   - (a) 若 codex 裁定「工程必须连续标量 β」⟹ 主键从 ForceState 退到 Θ_SCORE（`β_norm`，K=3），但须承担定理 2 的额外 Θ 警告 + 边界冻结纪律。
   - (b) 若 §3.2 δ-共线检查发现某 proxy 带符号污染 ⟹ β 变 δ 代理，置换自毁，须取绝对值/对称化（当前四 proxy 已全绝对量，不触发）。
   - (c) 若 OOS-value-gated 证明 ForceState 4 态比 `divergence bool` 无额外 μ 区分度 ⟹ 不进 UClass，甚至从 canonical Z 删维（退回 7 维）。
   - (d) 若补齐 TV/递归力度进 𝒜 ⟹ ForceState 支配序改变（更多口径 ⟹ 更多 Incomparable），有效域从 4-proxy 扩到完整 𝔉_ℓ。

4. **下游推论**：`MuClass` 6→7（H 轴，已做）→8（force_state）；`from_certificate` 全部路径填 None 零改动；perm_test/wverify 4 维手取键不读 force_state 不受影响；实装须先解数据源路由缺口（Candidate 不携 ForceProxies，推荐 collect_signals 就地重算方案 b）。

5. **谱系引用**：[[project_oddeven_mu_identity]]（i_class×δ 共线=置换自毁，β 用绝对量 proxy 规避的直接依据）；231/formalization-validity-domain（ForceState=4-proxy 支配序是完整 𝔉_ℓ 的近似，有效域<定义域，诚实标注缺 TV/递归）；codex #81（H 轴 accept in Z / conditional in selection 的同构裁决，ForceState 沿用）；`z-bucket-impl-20260702.md`（β^div 行「数据源部分在未接入 z」，本稿=接入设计）；no-patch §5（不硬造标量假装 β，直面定理 2）。

6. **影响声明**：本稿是纯设计（L0），**不改任何代码**。设计对象：`MuClass`（提议加 `force_state: Option<ForceState>` 第 8 维）、`UClass.divergence`（提议升级路径，OOS-gated）、新预注册 `prereg-beta-div-20260703`（三套 Θ）、`divergence.rs`（提议加 `ForceState` 枚举 + `force_state(seg_a, seg_c, 𝒜)` 纯函数 + `segment_total_variation` 补 TV proxy）。实装排期待 codex 审计。
