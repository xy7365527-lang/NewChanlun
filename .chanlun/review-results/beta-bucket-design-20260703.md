# β^div 背驰强度进桶键——设计稿 v2（阶段1 纯设计，只读）

- 工位：ws-beta（task #104 v1 / task #110 v2，编排者令「候选后续全做」）
- 日期：2026-07-03（v2）
- 规格来源：`docs/formal-chain/关于背驰.pdf`（14 页，标题「推导完全分类」，力度=支配序主论证）+ `docs/formal-chain/alpha分离.pdf` p2 方框 `Z_i=(ℓ,δ,I_γ,σ_higher,r,ω,β^div,d,c)` + `z-bucket-impl-20260702.md`（九维对照，β^div 行标「数据源部分在，未接入 z」）
- 认识论等级：**L0**（本稿是纯定义/结构设计，不含数据验证，零信息增量）——分箱边界的 alpha 有效性是 L2/L3，须 OOS，本稿不声明
- 状态：**v2 已落 codex 4 必修，可进实装排期**（`codex-beta-ruling-20260703.md` conditional→按 4 项修复）。

## v2 修订记录（codex-beta-ruling-20260703.md 逐条落地）

| # | codex 裁定 | v2 修复 | 落点 |
|---|-----------|---------|------|
| ① | 定理2「非法」措辞 fail | 「非法」→「非 canonical，需 Θ 预注册」 | §1 |
| ② | ForceState 命名 conditional | 定名 **`ForceStateA4`**（4-proxy sufficient state，非完整 𝒜_ℓ）；补 TV 后可升名 `ForceState` | §2.2 |
| ③ | δ-共线证明力度 conditional | 构造级措辞改 mirror-invariant；mix 加最小双侧样本阈值 + Cramér's V / exact test | §3 |
| ④ | full-z 接入不完整 conditional | `stratified_delta_perm_p_fullz` 六元组 base 加 `force_state` + 报告维 + fill-rate 断言 | §5.3（新）|
| ⑤ | 路由方案 (b) 就地重算 fail | 改 `divergence.rs` 单一原语 `ForceProxies::force_state()` + 透传 `Option<ForceProxies>` 到 Candidate + `z_of_candidate_with_force` | 实装可行性 |

---

## 1. 结论（核心：β^div 无 canonical 标量，原文最忠实进桶键是 ForceStateA4 四值支配序）

`关于背驰.pdf` 的定理 2（p7）**证明了**：允许力度族 𝒜_ℓ 上不存在无额外公理的唯一全序力度 / 唯一二值判定。存在 s,s' 与两个力度函数 m₁,m₂ 使 `m₁(s)<m₁(s')` 但 `m₂(s)>m₂(s')`——PDF 原文结论（p7 逐字）：任意 AND、OR、词典序、加权和，**都是额外选择 Θ，不是原文唯一推出**。因此：

> **把「背驰强度 β^div」实现为单一 proxy 的连续标量（MACD 面积比、DIF 比、振幅比之一）不是原文强制推出的 canonical β 定义**——它是**合法但需预注册的工程选择**（Θ_SCORE / Θ_LEX 路线），不是原文唯一 β。（v2：codex ① 复核确认原文未用「非法」措辞——单 proxy/词典序/加权和是需预注册的 Θ，非非法；删除 v1「非法」表述。）

原文自己给出的 z 力度分量（§9.1，p10）是：

```
ForceState_ℓ(s,s') = {Dominated, Dominates, Incomparable, Tie}
Z = (ℓ, δ, I_γ, ForceState, MACD, DIF, Amplitude, Speed, …)  → 进入 μ(z,a) 或选择器
```

**故 β^div 的最忠实进桶键 = `ForceStateA4`（四值支配序，4-proxy 近似完整 𝒜_ℓ 支配序，见 §2.2 命名理由），不是连续量 K 分箱。** 这直接细化了现有 `UClass.divergence: bool`：
- 现 `divergence bool` = C 段在**冻结 MACD 面积**判据上弱于 A 段（`z.i_class & 0b001001 != 0`，即 I_γ 含一类点 B1/S1）。这是单 proxy 二值（Θ=MACD 面积）。
- `ForceStateA4` 把它细化为**4-proxy 支配四态**：`Dominated`（C 在所有 4 proxy 上 ≤ A 且至少一个 <，=确定背驰）/`Dominates`（C 在所有 proxy 上 ≥ A，=确定非背驰/力度延续）/`Tie`（全等）/`Incomparable`（口径冲突，第17课「黄白线弱但面积强」类）。（原文记号 `ForceState_ℓ` 指完整 𝒜_ℓ 支配序；本稿实装类型定名 `ForceStateA4` 诚实标注只用现有 4 proxy，§2.2。）

**「候选后续全做」的诚实回应**：任务标题写「连续量进桶键」，但源文档的形式化结论表明连续标量非 canonical β 定义。本稿不 workaround（不硬造一个标量塞进去假装是 canonical β，no-patch §5），而是：**主键 = ForceStateA4（原文支配序的 4-proxy 忠实近似，无额外公理）；连续标量 K 分箱作为 Θ_SCORE 预注册候选之一（§9.2 三套 OOS 之一），不作默认**。

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

**β^div := `ForceStateA4`（4 值），𝒜₄ = {macd_area, dif_peak, price_amplitude, price_speed}**（现有四 proxy）。

### 2.3 命名裁决（v2，codex ② 二选一）——定名 `ForceStateA4`

codex 给二选一：改名 `ForceStateA4` 诚实标注四 proxy 近似，**或**先补 `segment_total_variation` 进 `ForceFeatures` 再定名 `ForceState`。**本稿选改名 `ForceStateA4`**，理由：

1. **命名诚实零成本，且不阻塞实装**——改名是纯符号决定，立即消除「完整 𝒜_ℓ 支配序」的声明膨胀（231号）；补 TV 是实装工位的代码工作（改 `ForceFeatures` + `force_features` + 全部构造点），把它前置到设计稿反而拖慢实装排期。
2. **A4 后缀是可升级契约**：`ForceStateA4` 显式携「4-proxy sufficient state」语义，升级路径清晰——补齐 TV + 递归次级别力度后，𝒜₄→𝒜_ℓ，类型可重命名 `ForceState`（届时支配序覆盖完整口径族）。
3. **单调性保证（codex ② 认定）**：补 TV/递归后，现 `Dominated`/`Dominates`/`Tie` **只可能**变 `Incomparable`（新增口径可能冲突），反向不会（已有冲突不因加维消失）。故 `ForceStateA4` 的 `Dominated` 是完整支配序 `Dominated` 的**超集**（宽判背驰）——用作 μ 分层安全，但**须知它可能把完整口径下的 `Incomparable` 误判为 `Dominated`**。

**诚实有效域标注（formalization-validity-domain 231号）**：`ForceStateA4` = 𝒜₄ 上的支配序，是完整 𝒜_ℓ 支配序的**宽近似**（有效域 < 定义域）。原文 §5（p6）完整 𝒜_ℓ 还含 `TV=Σ\|P_{t+1}−P_t\|`（全变差）和 `Σ_{次级别同向段} m_{ℓ-1}(u)`（递归力度），现有 `ForceFeatures` 缺此二者。不虚报为原文完整 𝔉_ℓ 支配序。

**Incomparable 用法约束（codex ②）**：`Incomparable` 单列一态进 μ 分层（不并入 `Dominated`），但**不得作为背驰确认**（口径冲突≠力度衰减）。

---

## 3. δ-共线检查设计

### 3.1 构造级（结构论证）——mirror-invariant，非「⊥δ by construction」（v2，codex ③）

见 §2.1 表：四 proxy 全绝对量（`.abs()`）⟹ 每个 proxy **不直接编码方向符号**（sign-agnostic / mirror-invariant：把整段价格镜像翻转，proxy 值不变）。故 `ForceStateA4` 的支配比较**不由 δ 的符号直接决定**。

**v2 订正（codex ③ 复核）**：`.abs()` 只证明「proxy 不 sign-coded」，**不能**证明「支配序输出统计独立于 δ」。数据生成过程仍可能让涨/跌两方向的支配态**经验分布**产生相关（非逻辑必然，但经验可能——如上涨段普遍力度更强 ⟹ `Dominates` 在 δ=+1 富集）。故 v1「β ⊥ δ by construction」是过强声明，删除；正确表述：**`ForceStateA4` 对方向镜像不变（构造级），经验独立性须 §3.2 数据验证**。

### 3.2 经验级（L2 验证设计，实装后跑）——加样本阈值 + Cramér's V/exact（v2，codex ③）

构造级 mirror-invariance 是必要不充分——须核实**经验分布**上 `ForceStateA4` 与 δ 在桶内混合（记忆 [[project_oddeven_mu_identity]]：共线维在置换检验上自毁）。设计：

1. **联合分布交叉表**：对每个 `(ℓ, i_class)` 层，构造 `ForceStateA4 × δ` 2×4 列联表（δ∈{+1,−1} × 4 态）。
2. **混合度判据（v2 加最小双侧样本阈值）**：定义 `mix(fs) = min(n_{fs,+1}, n_{fs,−1}) / max(1, n_{fs,+1}+n_{fs,−1})`。v1 的「mix>0」判据太弱（1 vs 999 也通过，codex ③）。**v2 强化**：通过条件 = `min(n_{fs,+1}, n_{fs,−1}) ≥ n_min` **且** `mix(fs) ≥ mix_min`，冻结 `n_min = 5`、`mix_min = 0.1`（双侧至少 5 笔且弱侧占比 ≥10%）。`min(n_{fs,+1}, n_{fs,−1}) < n_min`（含 δ-纯，弱侧=0）⟹ 该桶不足以支撑 δ-置换，**降级为仅 μ 分层不作置换单元**（不是「自毁」而是「证据不足禁用」，与 [[project_oddeven_mu_identity]] 的 underpowered-非证伪同精神）。
3. **关联强度检验（v2 替换裸卡方）**：卡方在小样本（期望频数 <5）功效与适用条件不足（codex ③）。改用 **Cramér's V**（关联强度，`V = √(χ²/(n·min(r-1,c-1)))`，值域 [0,1]，与样本量解耦）+ **Fisher exact / permutation test**（小样本精确 p，替代卡方的渐近近似）。判据：`V ≤ V_max`（冻结 `V_max = 0.2`，弱关联）**且** exact p > 0.05（不拒独立）⟹ 非共线（期望）；否则共线警告，回查 proxy 符号污染。
4. **报告**：每 `(ℓ,i_class)` 层输出 `{mix 向量, min 双侧样本, Cramér's V, exact p}`。任一桶不过阈值 ⟹ 该层该 β 桶降级仅 μ 分层；低样本桶按 §3.2.2 合并或禁用置换。
5. **低样本桶合并规则（v2）**：若 `(ℓ,i_class)` 层内某 `ForceStateA4` 桶双侧样本不足 `n_min`，先尝试与语义相邻态合并（`Tie`→并入较近的 `Dominated`/`Dominates` 前须人工核，`Incomparable` 不合并）；合并后仍不足则该桶整体禁用 δ-置换（只报 μ，不做统计推断）。

**对称化方案（若检查失败）**：若某 proxy 事后发现带符号污染（Cramér's V 高 + exact p<0.05），方案 = 取绝对值 + 分箱对称化。**当前四 proxy 已全绝对量，构造级无需对称化**——但经验共线（若出现）不能靠对称化消除，须回查是否力度本身随方向系统性偏移（那是真结构，non-tradeable beta，记忆 [[project_oddeven_mu_identity]] 的机制 C）。

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

  允许力度族 𝒜₄ = {macd_area, dif_peak, price_amplitude, price_speed}   # 冻结：4 proxy（缺 TV/递归，ForceStateA4 诚实近似 §2.3）

  Θ_DOM（主，推荐默认）:
    β^div := ForceStateA4(C, A) ∈ {Dominated, Dominates, Tie, Incomparable}   # divergence.rs ForceProxies::force_state() 唯一原语
    判据:
      Dominated    = ∀m∈𝒜₄, m_C ≤ m_A  ∧  ∃m, m_C < m_A       # 确定背驰（C 力度衰减）
      Dominates    = ∀m∈𝒜₄, m_C ≥ m_A  ∧  ∃m, m_C > m_A       # 确定力度延续
      Tie          = ∀m∈𝒜₄, m_C = m_A
      Incomparable = 否则（口径冲突）                              # 单列一态，不并入 Dominated，不作背驰确认
    进桶: MuClass.force_state = Some(ForceStateA4)（真候选路径 z_of_candidate_with_force）; None（裸证书口径）
    selection: 默认不进（UClass 保 divergence bool）
    fullz 置换: force_state 进 base 前逐层过 §3.2 δ-共线检查（谱系 iclass-delta-collinearity；不无条件全进）

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

现 `MuClass` 的 estimand（含 `horizontal` 7 维）已冻结（codex #81）。加 `force_state` 是**第 8 维扩展**——须作 estimand 版本升级（`z-bucket-impl` 已有先例：加 `horizontal` 时 `from_certificate` 40+ 调用点填 None 零改动）。`force_state: Option<ForceStateA4>` 默认 None ⟹ `from_certificate` 全部调用点 + pi_bsp_timing 从 Voice 构 z **零改动**，只有真候选路径 `z_of_candidate_with_force` 显式填 Some。

**v1 声明订正（codex ④，非零改动路径）**：v1 称「现有全部路径零改动」**不成立**。`perm_test.rs:206 stratified_delta_perm_p_fullz` 的 δ-free base key 已显式列 **五元组** `(c.level, c.i_class, c.parent_dir, c.position, c.horizontal)`（perm_test.rs:216 坐实）——这是 full-z 置换路径。`force_state` 若不进这个 base，不同背驰支配态被并入同一置换层，**稀释检验力度**。故 fullz base 必须与第 8 维同步扩到六元组（见 §5.3）。

### 5.3 full-z 置换路径同步（v2，codex ④ 必修）

`force_state` 进 canonical Z 后，以下三处**必须同步改动**（否则 fullz 置换检验静默失真）：

1. **base key 扩六元组**：`stratified_delta_perm_p_fullz`（perm_test.rs:216）的 δ-free base key 从
   `(level, i_class, parent_dir, position, horizontal)` → `(level, i_class, parent_dir, position, horizontal, force_state)`。
   `force_state` 是 δ-free（力度支配态由绝对量算，置换 δ 时恒定，§3.1 mirror-invariant）⟹ 合法进 base。
2. **报告格式加 force 维**：fullz 报告表加 `force_state` 列（同 `z-bucket-impl` 加 role/H 列）。
3. **fill-rate 断言（防静默）**：加断言「fullz 置换路径的 `ResidualTrade.class.force_state` 非全 `None`」——生产路径若因路由未接（实装可行性节）导致全 `None`，六元组退化回五元组，测试须 fail-fast 暴露，不静默通过。

**⚠ δ-共线前置门（谱系 [[project_iclass_delta_collinearity_perm_degeneracy]]，2026-07-03 坐实）**：该记忆记录 **i_class 进 fullz base 与 δ 共线导致 δ-置换退化**（主 root buy 子桶 n=1612 大样本，perm_p 从 4 元组的 0.005 退化到六元组口径的 1.000——加样本救不了，比碎裂更根本）。`force_state` 进 base **前必须过 §3.2 δ-共线检查**：若某 `(ℓ,i_class)` 层内 `force_state` 桶 δ-纯（min 双侧样本 < n_min 或 Cramér's V > V_max），该桶**不进 fullz base 的置换单元**（降级仅 μ 分层，报告展示但不做 δ 推断）。这是 §5.3.1 与 §3.2 的强耦合——**force_state 是否进 fullz 置换 base，由 §3.2 检查结果逐层决定，不是无条件全进**。

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

**v2 路由裁决（codex ⑤ 必修，废除 v1 方案 (b)）**：v1 推荐的「collect_signals 就地重算 ForceState」被 codex 裁 **fail（致命）**——它会在 `collect_signals` 独立重实现支配序比较，构成**第二套力度比较逻辑**（`divergence.rs` 已有 `ForceFeatures`/`weak_theta` 一套原语），是 no-patch-mentality 明确禁止的重复实装，且诊断/dx 手写循环须同步维护，两套逻辑漂移风险高。

**v2 唯一路由（单一原语 + 数据透传，不重算）**：

1. **`divergence.rs` 新增单一支配序原语**：
   - `enum ForceStateA4 { Dominated, Dominates, Tie, Incomparable }`。
   - `impl ForceProxies { pub fn force_state(&self) -> ForceStateA4 }`——`𝒜₄` 上支配序比较的**唯一实现**（C=`seg_c` vs A=`seg_a`，四 proxy 全 ≤/≥ 判 Dominated/Dominates，混合判 Incomparable，全等 Tie）。
   - `force_state()` 与既有 `weak_theta` **不互相替代但共享同一 `ForceProxies` 数据结构**（`weak_theta` 是 Θ 布尔判据=单口径/词典序弱化；`force_state` 是 DOM 四态=多口径支配。二者是 §5.1 prereg 的 Θ_LEX vs Θ_DOM，各自独立方法，同一份 proxy）。
2. **数据透传到候选**（codex ⑤ 认定的正确路由，即 v1 方案 (a) 的坐实）：`Option<ForceProxies>`（或更小的 `Option<ForceStateA4>`）从 `signal.rs` 的 `Vec<(BspPoint, Option<ForceProxies>)>` 透传进 `Candidate`/`RawSignal`。
   - **前置修复**：`signal.rs:543 extract_signals` 生产路径现传空 `dif`/`closes_tick` ⟹ `force` 恒 `None`（注释自陈「生产 BspPoint 入口不消费 force」）。实装须让生产路径**真透传 dif/closes**，否则 `force_state` 恒 None，第 8 维退化（§5.3 fill-rate 断言正是防此静默）。
3. **`z_of_candidate_with_force(c, fs)` 构 z**：新构造器在 `z_of_candidate` 基础上 struct-update `force_state: Some(fs.force_state())`——**调 divergence.rs 唯一原语，不在此重算比较**。

**本稿声明**：数据源在（ForceProxies 已算），路由未接（z 构造点拿不到 + 生产 extract_signals 传空 dif/closes）。实装 = 单一原语 `ForceProxies::force_state()` + 透传，非重算。改动面大于 v1 (b) 但消除 no-patch 违规（team-lead 已裁 v2 直接排实装）。

---

## 结果包六要素

1. **结论**：β^div 的原文忠实进桶键 = `ForceState`（4 值支配序，𝒜=4 proxy），**不是连续标量 K 分箱**（定理 2 否定单一标量作 β 定义）。ForceState 进 canonical `MuClass`（第 8 维，Option 诚实），默认不进 selection/UClass。连续标量 K 分箱降为 Θ_SCORE 预注册候选之一（§9.2 三套 OOS）。

2. **定义依据**：`关于背驰.pdf` §9.1（p10，`Z=(…,ForceState,MACD,DIF,Amplitude,Speed)`——原文 z 力度分量是 ForceState 非标量）；定理 2（p7，无唯一全序力度，标量=额外 Θ）；§5-6（p6，支配序 `s≺_𝒜s'`）；§9.2（p10，三套 Θ 预注册 OOS）；§8（p8，级别 ≻ DIF ≻ 面积）。代码：`divergence.rs` `ForceFeatures`（4 proxy 全绝对量）/`ForceProxies`（A/C 并置）；`mu_estimator.rs:79 MuClass`（加 `force_state`）/`:154 UClass.divergence`（β 细化对象）；`selector.rs:144 z_of_candidate`（进 z 点）。

3. **边界条件（结论翻转）**：
   - (a) 若 codex 裁定「工程必须连续标量 β」⟹ 主键从 ForceState 退到 Θ_SCORE（`β_norm`，K=3），但须承担定理 2 的额外 Θ 警告 + 边界冻结纪律。
   - (b) 若 §3.2 δ-共线检查发现某 proxy 带符号污染 ⟹ β 变 δ 代理，置换自毁，须取绝对值/对称化（当前四 proxy 已全绝对量，不触发）。
   - (c) 若 OOS-value-gated 证明 ForceState 4 态比 `divergence bool` 无额外 μ 区分度 ⟹ 不进 UClass，甚至从 canonical Z 删维（退回 7 维）。
   - (d) 若补齐 TV/递归力度进 𝒜 ⟹ ForceState 支配序改变（更多口径 ⟹ 更多 Incomparable），有效域从 4-proxy 扩到完整 𝔉_ℓ。

4. **下游推论**：`MuClass` 6→7（H 轴，已做）→8（`force_state: Option<ForceStateA4>`）；`from_certificate` + pi_bsp_timing 路径填 None 零改动；**但 `stratified_delta_perm_p_fullz` 五元组 base 须扩六元组 + fill-rate 断言（v2 §5.3，非零改动）**；force_state 进 fullz 置换 base 前逐层过 §3.2 δ-共线检查（谱系 iclass-delta 覆辙）；实装须解路由（透传 `Option<ForceProxies>` 到 Candidate + `signal.rs:543` 生产路径真传 dif/closes，非 collect_signals 重算）。

5. **谱系引用**：[[project_oddeven_mu_identity]]（β 用绝对量 proxy 规避 δ-共线）；[[project_iclass_delta_collinearity_perm_degeneracy]]（**v2 新增，2026-07-03 坐实 i_class 进 fullz base 致 δ-置换退化 0.005→1.000——force_state 进 base 前必过 §3 检查的直接依据**）；231/formalization-validity-domain（`ForceStateA4`=4-proxy 支配序是完整 𝔉_ℓ 宽近似，有效域<定义域）；codex #81（H 轴 accept in Z / conditional in selection 同构，ForceStateA4 沿用）；`z-bucket-impl-20260702.md`（β^div 行「数据源部分在未接入 z」，本稿=接入设计）；no-patch §5（路由不重算=单一原语 `ForceProxies::force_state()`）；`codex-beta-ruling-20260703.md`（本 v2 的 4 必修来源）。

6. **影响声明**：本稿 v2 是纯设计（L0），**不改任何代码**。设计对象（实装工位的直接改动清单）：`divergence.rs`（加 `enum ForceStateA4` + `ForceProxies::force_state()` 唯一支配序原语；可选补 `segment_total_variation` 升 𝒜₄→𝒜_ℓ）、`interp.rs`/`selector.rs`（`Candidate`/`RawSignal` 携 `Option<ForceProxies>` + `z_of_candidate_with_force` + `signal.rs:543` 生产路径真传 dif/closes）、`MuClass`（加 `force_state: Option<ForceStateA4>` 第 8 维）、`perm_test.rs:216`（`stratified_delta_perm_p_fullz` base 扩六元组 + 报告 force 列 + fill-rate 断言）、`UClass.divergence`（升级路径 OOS-gated）、新预注册 `prereg-beta-div-20260703`（三套 Θ）。**v2 已落 codex 4 必修，team-lead 裁定不再送审，可直接排实装。**
