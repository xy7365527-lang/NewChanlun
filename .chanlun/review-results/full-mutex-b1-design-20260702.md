# 全互斥定义策略 b1+b2：逐维对照定案 + P0 缺维接入设计

- 工位: ws-fullmutex（task #79，goal g-full-mutex-impl 首工位）
- 日期: 2026-07-02
- 源: `docs/formal-chain/`（原文）+ A 组报告 `dlpdf-a-mutex-classification-20260702.md`（P0/P1/P2 索引）+ **当前代码核对**（rust/src/theta_v0/）
- 环境: 纯只读，零 git。代码锚点均 `文件:行` 核过。
- 交付: (1) 逐维对照定案表（含代码锚点）+ (2) P0 缺维接入设计稿（交 codex 审计后实装）

---

## 0. 顶层订正（对 A 组报告的关键翻转）

**A 组报告（#70，纯读 PDF 未核代码）的四条"原文有我们无"缺口中，三条已在代码实装，A 报告的"缺失"判定 stale。** 真缺口比 A 报告窄且更精确，正是编排者点破的**"回测跑在未完全实装的全互斥定义策略上"**——不是"没有角色维"，而是**"角色维 r/σ_p 已在分类层算出，但 alpha 回测路径 `econ_positive.rs` 的分桶键仍是粗投影 Y=(ℓ,δ,I_γ)，不消费它们"**。

核对结论（逐条见 §1）：
- **角色 r = R(g)=(H,V,δ) 18 类**：已完整实装于分类层 `coverage.rs:758-806`（`Horizontal`/`Vertical`/`OperationRole`，带 spec §7/§8 锚点 + 231 号 L0/L2 标注）。A 报告"缺整条角色轴"**订正为：已实装分类层，缺 alpha 桶键接入 + H 轴从未进任何桶键**。
- **父声部方向 σ_p**：已实装分类层（`coverage.rs:829 parent_sign` 从 V(g) 推）+ 状态层（`mu_estimator.rs:73 MuClass.parent_dir`）+ wverify/perm_test 桶键（`wverify_run.rs:83`、`perm_test.rs:29`）。A 报告"无此维"**订正为：三处已实装，独缺 `econ_positive.rs` alpha 桶键**。
- **短差 ShortDiff**：已实装（`coverage.rs:781 Vertical::ShortDiff`、`mu_estimator.rs:101 short_swing`、TW `runner.rs` schedule 派 ShortDiff）。A 报告"无短差概念"**订正为：已实装，同样独缺 alpha 桶键接入**。
- **64 类非坍缩 b_ℓ**：`BspBits.class_index()` 6-bit 掩码不压扁（`mu_estimator.rs:59-60/105`）——已实装，A 报告 P1-③ 对齐（τ 坍缩只在 gate 分派层，桶键用 class_index 未坍缩）。

**A 组仍成立的缺口**：H 轴（水平兄弟关系）计算了但**从未进入状态 z / 任何桶键**（`MuClass` 只投影 V 轴，见 §1 σ_p 行）；区间套 N^δ 严格嵌套（P1-④，实测 95% 退化，属 #77 在途，非本工位）；Eat 覆盖定理（P2-⑥，soundness 层，非 alpha 路径）。

---

## 1. 逐维对照定案表（原文维 → 分类层 / 状态层 / alpha 桶键 三处实装状态）

三处含义：**分类层**=候选构造时是否算出（coverage/interp）；**状态层**=是否进 `MuClass` z；**alpha 桶键**=`econ_positive.rs` 回测分桶是否消费。原文要求维进入**状态 z**（细分类优势定理 §13：z 是细状态，回测须按 z 分桶）。

| 原文维 | 原文定义（页/§） | 分类层 | 状态层 MuClass | alpha 桶键 econ_positive | 定案 |
|--------|-----------------|--------|---------------|--------------------------|------|
| **ℓ 级别** | z 分量，去根化前沿 ℓ_max | ✅ `Candidate.level` | ✅ `MuClass.level:70` | ✅ 桶键 `(level,..)` `econ_positive.rs:2070` | **全通** |
| **δ 方向** | δ∈{±1} 绝对方向 | ✅ `Candidate.dir` | ✅ `MuClass.delta:71` | ✅ 桶键 `(..,delta,..)` | **全通** |
| **I_γ 一二三类** | {0,1}⁶ 非坍缩 64 类 | ✅ `Candidate.bits`/`BspBits` | ✅ `MuClass.i_class:72`（`class_index()` 6-bit 不压扁） | ✅ 桶键 `(..,bsp_class)` `2070` | **全通**（gate 分派层坍缩 τ，桶键不坍缩） |
| **σ_p 父声部方向** | ∈{-1,0,+1}，0=Ambient | ✅ `coverage.rs:829 parent_sign`（从 V(g) 推） | ✅ `MuClass.parent_dir:73` | ❌ **`econ_positive.rs` 桶键 `(level,delta,bsp_class)` 不含**（`2070/2318/2729`） | **缺 alpha 接入**（wverify 桶键含 `wverify_run.rs:83`，但 alpha 回测路径不含——**编排者点破的真缺口**） |
| **角色 r: V 轴** (Ambient/FollowParent/ShortDiff) | R(g) 垂直分量，短差=Sub∧反父 | ✅ `coverage.rs:775 Vertical` | ✅ 折叠为 `parent_dir+short_swing:74+position:75` | ❌ 同 σ_p，不进 alpha 桶键 | **缺 alpha 接入**（与 σ_p 同一接入点，V 轴已在 z，随 parent_dir 一并接入） |
| **角色 r: H 轴** (First/SameFollow/SameReverse) | R(g) 水平分量，同父前兄弟方向关系 | ✅ `coverage.rs:849 horizontal_relation` | ❌ **`MuClass` 不含 H**（只投影 V 轴） | ❌ 不进 | **真缺口·选择类**（算出即弃；补 H 进 z=升完整 R(g)18 类，vs H 对 μ 贡献待验——交 codex 裁，见 §2.3） |
| **短差 ShortDiff 操作** | 父仓保持+次级别反向双开 | ✅ `Vertical::ShortDiff` | ✅ `short_swing:74` + TW `runner.rs` schedule | ❌ 不进 alpha 桶键 | **缺 alpha 接入**（同 σ_p 接入点） |
| **门通道 γ 区间套 N^δ** | [J^δ_{ℓ-1}⊆J^δ_ℓ] 严格嵌套 | ✅ `build_gate_certificate`（`econ_positive.rs:314`） | — （γ 是准入门非 z 分量） | ✅ `gate_pass()` 消费（`318`） | **部分**（Nest 接入，但实测 95% 退化 base-case=P1-④，#77 在途，非本工位） |
| **β 力度/背驰二值 D** | D∈{0,1} 定义足够，β=K 分箱细分 | ✅ 力度 proxy（#10/#19） | — （β 是细分变量非定义必要） | 部分（wverify σ^H 桶键） | **部分**（P2-⑧，二值谓词 D 未形式化，非 P0） |
| **κ 对象同一性** | 单一判别 seen-key | ✅ `(ℓ,s,bsp_disc)` | ✅（seen 去重 `econ_positive.rs:237`） | ✅ | **全通** |

### 1.1 对 task 特定问题的精确回答

> **"σ_p 已在桶键层（mu_estimator.rs parent_dir）——原文要求分类层还是桶层？精确缺什么？"**

原文要求 σ_p 进入**状态 z**（`z=(ℓ,δ,I_γ,r,σ_p,…)`，权威全本①②§状态；细分类优势定理 §13 要求回测按 z 分桶）——即**同时**是分类层判别量和细分桶键。核对：

1. **分类层**：✅ 已实装。`coverage.rs:833 parent_sign(attached_dir)` 从父容器方向算 σ_p∈{-1,0,+1}，`vertical_relation` 据此定 V(g)。
2. **状态层 z**：✅ 已实装。`MuClass.parent_dir`（`mu_estimator.rs:73`）是 z 的第 4 分量。
3. **桶键层（两条路径分叉）**：
   - **统计路径**（perm_test/wverify）：✅ 桶键 `(level, bsp_class, delta, parent_dir)`（`wverify_run.rs:83/138`、`perm_test.rs:29`）——σ^H 已分层。
   - **alpha 回测路径**（`econ_positive.rs` = 编排者所指"回测"）：❌ **桶键仍是 `(level, delta, bsp_class)`**（`2070/2318/2729/2823` 全部三元键），**不含 parent_dir**。

**精确缺什么**：不是"σ_p 没实装"（分类层+状态层+统计桶键三处都有），而是**`econ_positive.rs` 的 alpha 分桶/信号元组丢弃了 σ_p（及整个 role）**——`collect_signals`（`222`）产出的 signal 元组是 `(bar, side, source_index, u32, δ:i8, bsp_class:u8)`，`assemble_gamma_with_tower` 返回的 `c.role`（`291` 处可读）被就地丢弃。结果：**alpha 回测跑在粗投影 Y=(ℓ,δ,I_γ) 上，而非原文细状态 Z=(ℓ,δ,I_γ,σ_p,…)**——这是 A 报告 §3 "我们声明的完全互斥分类实为 Y 粗投影" 的**代码级坐实**，也是编排者"回测跑在未完全实装策略上"的字面机制。

---

## 2. P0 缺维接入设计稿（交 codex 审计后实装）

**P0 不是"实装 18 类判别函数"**（`coverage.rs` 已实装，带 spec 锚点）。P0 是**把已算出的 role/σ_p 接入 `econ_positive.rs` alpha 回测的信号元组 + 分桶键**，使回测状态从 Y 升到 Z。判别函数来源、接入点、与既有类型关系、dx 守恒扩展如下。

### 2.1 判别函数来源（原文页引用 + 已实装锚点）

| 判别量 | 原文定义 | 已实装函数（复用，不重写） |
|--------|---------|---------------------------|
| σ_p ∈{-1,0,+1} | 权威全本 §状态 z；`级别和sigma` PDF p3-4/p8/p11 `z=(ℓ,δ,σ^H)` | `coverage.rs:833 parent_sign` |
| V(g) (Ambient/FollowParent/ShortDiff) | 递归完全分类买卖点.pdf §7.2 / P6-P7 | `coverage.rs:775 Vertical` + `vertical_relation` |
| H(g) (First/SameFollow/SameReverse) | 递归完全分类买卖点.pdf §7.1 / P6 | `coverage.rs:849 horizontal_relation` |
| R(g)=(H,V,δ) 18 类 | 递归完全分类买卖点.pdf §8 / P7（3×3×2） | `coverage.rs:799 OperationRole` |
| σ_p→MuClass 桥 | 权威全本 §16 | `selector.rs:150-155`（Ambient→0/FollowParent→δ/ShortDiff→−δ） |

**关键**：`selector.rs:145-155` 已有 `Candidate.role.v → (parent_dir, position)` 的规范映射，`MuClass::from_certificate`（`mu_estimator.rs:92`）已能吃 `(level,delta,bits,parent_dir,position)` 产完整 z。接入 = **让 `econ_positive.rs` 复用这两个已存在的桥**，而非造新逻辑。

### 2.2 接入点（最小 diff，复用既有类型）

`econ_positive.rs` 两处：

1. **`collect_signals`（`222`）signal 元组扩展**：现元组 `(usize, VoiceSide, usize, u32, i8, u8)` → 加 `parent_dir:i8` + `position`（或直接携带 `OperationRole`）。`291` 处 `for c in &assemble_gamma_with_tower(...)` 已有 `c.role`，按 `selector.rs:150` 同一 match 算 `parent_dir`——**代码已在别处存在，此处调用**。
2. **分桶键升级**：所有 `(d.level, d.delta, d.bsp_class)` 三元键（`2070/2318/2729/2823` 等）→ 改用 `MuClass::from_certificate(...)` 作 key（`MuClass` 已 `Eq+Hash`，`mu_estimator.rs:68`）。**推荐方案 (a)**：直接以 `MuClass` 为桶键（canonical z），而非手工加 `parent_dir` 到元组——`MuClass` 是原文 z 的既有载体，复用它=最小且正确（ponytail：不重造 z 的 Hash 键）。

**降维分叉（抗过拟合，原文 §29-30 已实装 `UClass`）**：细化桶键→每桶样本 n̄ 降→winner's curse。原文药方 `ϕ:Z→U`（`mu_estimator.rs:161 project_to_u`）已实装。接入后若高维 z 桶稀疏，回测统计层走 `UClass`（`level_bucket,δ,VoiceRole,divergence`）折叠——**这条路径已存在**，接入 z 桶键不需新建降维，只需在 alpha 聚合处二选一（Z 桶=oracle alpha 上界 / U 桶=抗过拟合估计），交 codex 定 alpha 报告用哪层。

### 2.3 H 轴是否进 z（选择类，明确上浮 codex 裁定）

`MuClass` 现含 V 轴（parent_dir/short_swing/position）但**不含 H 轴**（Horizontal）。原文 R(g)=18 类含 H，故严格补维应把 H 进 z。但：

- **补 H 进 z 论据**：原文 18 类完全分类含 H；细分类优势定理 §13——补维不降 oracle alpha。
- **不补 H 论据**：H（同级前兄弟顺/反）是**结构关系**非**操作极性**，`UClass` 降维（§30）本就把 role 折叠掉 H（只留 Root/ChildTrend/ChildSwing=V 轴语义）；补 H 使 |Z| ×3，加剧稀疏（§29 winner's curse），而 H 对 μ 的贡献 spec **未给 L2 证据**（`coverage.rs:796` 明标"18 类经验全可达性 spec 未覆盖"）。

**这是选择类**（价值判断：完备性 vs 样本经济，`no-unnecessary-escalation` 四分法=选择，允许上浮）。设计稿建议：**P0 先接入 V 轴+σ_p（已在 z，零结构改动，直接闭合编排者点破的缺口）；H 轴进 z 作为独立决策交 codex 裁**——若裁定补，H 进 `MuClass` 加一字段 `horizontal:Horizontal`，`from_certificate` 加参，桶键自动升 54 类（18×δ 已含在 role，实为 3H×3V×2δ×|I_γ|×|ℓ|）。

### 2.4 与既有 GateCertificate / BspCandType 关系

- **正交，不冲突**。`GateCertificate`（γ 通道，`econ_positive.rs:314 build_gate_certificate`）是**准入门**（区间套 Nest/小转大 Xzd/Reject），决定候选是否成交易；role/σ_p 是**状态分类维**（成交后按哪个 z 桶记 alpha）。两轴独立：门控不变，只是桶键从 Y 细化到 Z。
- `BspCandType::StructBreak`（#62 第四类纤维，零 bit I_γ=∅）：与 σ_p 接入正交。StructBreak 的交易语义边界是矛盾-A（#78 在途 codex 裁），本工位不触碰——σ_p 接入对 StructBreak 候选同样适用（它也有 role/σ_p），但是否 emit 交易由 #78 裁定，不由本接入决定。

### 2.5 dx 守恒式扩展

`econ_positive.rs` 有"captured 分解恒等式"（`2292`：`captured == recomputed`）与 per-bucket Σpnl 一致性断言（`2318/2729/3113`）。**桶键从 Y 细化到 Z 是分划细化（partition refinement）**：Z 桶是 Y 桶的不相交细分（同一 (ℓ,δ,I_γ) 按 σ_p 再分）。因此：

- **逐笔 dx 守恒不变**：`captured 分解恒等式` 是**逐笔**（per-decomp）恒等式，与分桶无关——细化桶键不触碰单笔分解，断言自动继续成立。
- **聚合守恒扩展**：`Σ_{Y桶} pnl` 断言 → 扩展为 `Σ_{Z桶} pnl == Σ_{Y桶} pnl`（细分桶求和 = 粗桶求和，分划细化的求和守恒）。**新增一条断言**：`decomps.filter(z).sum() 按 σ_p 分组再合 == 按 (ℓ,δ,bsp_class) 分组`——这是分划细化保 Σ 的可测试守恒扩展（`no-patch`：不是加 workaround，是把既有守恒律的定义域从 Y 扩到 Z）。

---

## 3. 结果包（六要素）

1. **结论**：原文完全互斥分类的 P0 维（角色 r=R(g)18 类、父声部 σ_p、短差）**已实装于分类层**（`coverage.rs`，带 spec §7/§8 + 231 号 L0/L2 锚点）**+ 状态层 z**（`MuClass`，含 parent_dir/short_swing/V 轴）**+ 统计桶键**（wverify/perm_test `(ℓ,bsp_class,δ,σ^H)`）。**真缺口 = alpha 回测路径 `econ_positive.rs` 的信号元组与分桶键仍是粗投影 Y=(ℓ,δ,bsp_class)，丢弃已算出的 role/σ_p**——即编排者点破"回测跑在未完全实装策略上"的代码级机制。P0 设计 = 让 `econ_positive.rs` 复用既有 `selector.rs` role→parent_dir 桥 + `MuClass` 作桶键（最小 diff，零新逻辑），把回测状态从 Y 升到 Z。H 轴（Horizontal）算出即弃、从未进 z，是独立**选择类**决策（补维完备 vs 样本经济），交 codex 裁。
2. **定义依据**：原文 z=(ℓ,δ,I_γ,r,σ_p,…)（权威全本①②）、R(g)=(H,V,δ)18 类（递归完全分类买卖点.pdf §7-8/P6-P7）、细分类优势定理 §13、维数控制 §29-30；对照代码锚点 `coverage.rs:758-849`（role 判别函数全实装）、`mu_estimator.rs:68-110`（MuClass z 载体，缺 H 轴）、`selector.rs:145-155`（role→parent_dir 桥）、`econ_positive.rs:222/291/2070`（signal 元组+桶键丢弃 role 的实证）、`wverify_run.rs:83`（统计路径已含 σ^H，证明桥可用）。
3. **边界条件（结论翻转）**：(a) 若核实 `econ_positive.rs` 某处已按 role 分桶（本核对未见，但若 alpha 聚合另有隐藏路径读 role）⟹ P0 缺口消解为"仅文档滞后"；(b) 若 codex 裁定 H 轴对 μ 无 L2 贡献且应删 ⟹ z 定型为 6 维（现 MuClass），本设计的 H 分叉作废、只接入 V 轴+σ_p；(c) 若接入后 Z 桶普遍 n̄<Le Cam 两点下界（§大样本不足）⟹ alpha 报告须走 `UClass` 降维层，Z 桶键仅作 oracle 上界不作估计。
4. **下游推论**：(a) 任何"回测验证了完全互斥分类 alpha"的声明须核 `econ_positive.rs` 桶键——现为 Y 桶，故当前 alpha 数字是**粗状态 Y 的 alpha**，非原文 Z（按 §13 补维只增不减 oracle，但需 §29 shrinkage/LCB 抗稀疏，我们 W-VERIFY 已有）；(b) 接入后 W-VERIFY(#13/#51) 与 `econ_positive.rs` 两条路径的桶键统一到 Z=`MuClass`，消除"统计路径含 σ^H、alpha 路径不含"的分叉；(c) H 轴裁定影响 `MuClass` 维数（6 vs 7），是 goal 后续所有分桶断言的前置。
5. **谱系引用**：230/231（有效域≠定义域——alpha 回测跑 Y 桶=在 Y 有效域声明 Z 完备性，本工位坐实为代码实例）、A 组报告 §3-4（P0/P1/P2，本工位订正其三条"缺失"判定 stale）、G2（2B/3B 坍缩仅在 gate 分派层，桶键 class_index 未坍缩，订正 A 报告 P1-③）、#62/#78（StructBreak 与本接入正交，交易语义 #78 裁）、#77（区间套严格嵌套 P1-④ 在途，非本工位）、576（分账本 P2-⑦）。**建议 genealogist 新结晶**："判别维已在分类层实装 ≠ 回测消费它——alpha 桶键滞后于分类层是'有效域<定义域'（231）在实装路径的具体形态"。
6. **影响声明**：纯只读，零 git，零代码改动。产出=本设计稿。**待实装**（交 codex 审计后）：`econ_positive.rs` signal 元组 + 分桶键接入 role/σ_p（复用 `selector.rs`+`MuClass`）+ dx 守恒断言从 Y 扩到 Z + H 轴进 z 的 codex 裁定。触及决策：H 轴是否进 z（选择类，上浮 codex）；不改任何 settled 定理，不改 `coverage.rs`/`mu_estimator.rs`（已实装正确），只改 `econ_positive.rs` 消费侧。认识论如实：role/σ_p 分类层实装为真，alpha 桶键未接入为真，当前回测 alpha 是 Y 粗状态的（231 有效域实例）。
