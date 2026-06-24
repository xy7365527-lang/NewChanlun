---
task: "task#40 [B-task] 9轨道操作分派 + add word(h⁺∘σ)实装"
topo_address: "swarm/orbit9-impl-rtas-40/B-dispatch"
parent_callback: "lead"
date: "2026-06-24"
worktree: "/tmp/orbit9-B-wt @ orbit9-B-dispatch-20260624 (从 facea 54279a503e 隔离)"
upstream: "#39 exhaustive-operation-classification(9 τ对称轨道 O1-O9 + S1-S8↦轨道分派) / 543(操作=word≠硬编码循环, OP_REBUY=h⁺∘σ) / 541(D∞) / #38 协调节点(recursion_decision载体)"
epistemic_level: "数据结构/会计/守恒=L0; add 原语=L0(543 word, 非新原子); 轨道分派=L0(#39 §3.2 Burnside 闭合); OFF bit-exact=L1(管线); 净收益/有效域=L3 未决(委托 #42/D-task)"
---

# task#40 B-task：9 轨道操作分派 + add word（O3 = 买回/卖回）实装

## 〇、一句话结论

route_bsp 现状 = **9 轨道的隐式二元坍缩**——已隐式分派 enter/ascend/flip/sink/recover/drain/no-op（O1/O2/O4/O5/O6/O7/O8/O9 的八个），**唯一结构缺口 = O3（add/add_short = 买回/卖回 = 不动 h 的同级别短差腿部分重建）**。当前同父向 BSP 在 j 持短差时是 `recover`（平**整条** m=u_sub 升回父），无短差时 `no-op`——**没有"加一段"那一腿**（部分买回）。本实装补 O3 add 原语 + route_bsp 显式 9 轨道判别（env `T_ORBIT9_DISPATCH`，缺省 OFF ⇒ 旧二元 bit-exact）。

## 一、概念定位（#39 §3.2 ↔ 引擎现状映射）

| 轨道 | #39 word | 引擎现状 | 本实装动作 |
|---|---|---|---|
| O1 flip | τ@φ=0 | route_bsp None 分支反向 → clear_all+enter | 复用（轨道标注）|
| O2 enter | e→建仓 | route_bsp None 分支首建 → enter | 复用 |
| **O3 add/add_short** | **OP_REBUY=h⁺∘σ（不动 h 同级别短差腿重建）** | **缺口**：同父向 j 持短差 → recover（平**整条**）/ 无短差 → no-op | **新增 add 原语 + route 分支** |
| O4 ascend/ascend_short | σ relabel riding | route_bsp None 同向 j>cc → ascend | 复用 |
| O5 emergence_upgrade | σ relabel level | step A' emergence_upgrade | 复用 |
| O6 sink | h⁻∘τ_sub | route_bsp Some(p) 反父向 → sink | 复用 |
| O7 recover | σ⁻¹∘τ | route_bsp Some(p) 同父向持短差 → recover | 复用 |
| O8 drain | τ@k | route_bsp Some(p) 反父向遗留同向 → drain | 复用 |
| O9 hold/no-op | e | route_bsp 各 no-op 分支 | 复用 |

**∴ B 任务的代码增量 = ① O3 add 原语（缺口轨道）② route_bsp 显式 9 轨道判别（OFF=旧二元 bit-exact）。其余八轨道是已有原语的轨道标注（无行为改变）。**

## 二、O3 add 的严格语义（543 + #39 §3.1b，非新原子）

### 2.1 add = 买回/卖回 = 不动 h 的同级别短差腿**部分**重建

#39 §3.1b 买回/卖回机制（同级别 k，买卖点，短差语境）：
- **多头核心仓**：次级别低点 type1买(k-1) → **买回**重建多仓（= O3 add）。
- **空头核心仓**：次级别高点 type1卖(k-1) → **卖回**重建空仓（= O3 add_short，τ 镜像）。

**与 recover（O7）的精确区分**（这是缺口所在）：
- **recover（O7，已实装）**：次级别走势**完成** ⇒ 平**整条**短差 m=u_sub 升回父向（全有/全无）。
- **O3 add（缺口）**：次级别**回调到买卖点（未走势完成）** ⇒ **部分**买回一段 m=quota(机动仓)，**不动 h**（同级别 k 内重建被 sink 减掉的腿的一段）。

**"不动 h" = 操作不涉及级别跃迁**（#39 §3.1b 关键群论事实）：add 在父级 k 内把被 sink 减掉的核心 units 部分恢复，sub 级短差对应减一段。这是 **σ-不变配额 m=quota(mob)** 的同级别重建，**非** 543 表述的 `h⁺∘σ`（径向递归）——#39 §3.4 修正：「O3（加仓=买回/卖回）= 不动 h 的同级别短差腿重建（**非**径向递归）」。

**543 表 `OP_REBUY=h⁺∘σ` 与 #39 §3.4「不动 h」的张力 → 已由 #39 终版裁决**：543 的 word 标注是 path 层（操作路线）的一种表达；#39 §3.4 在群作用层精化为「O3 不动 h」（莫比乌斯 τhτ⁻¹=h⁻¹ 仅作用 h，对 O3 平凡对称）。本实装遵从 #39 终版——**add 不引入新 Σ 原子（仍是 {e,h⁺,h⁻,τ} 的 word，任务 #40 硬约束「非新 add 原语」），其行为 = 同级别短差腿部分重建**。

### 2.2 add 不污染 long 方向（任务 #40 单测「add 加仓不污染 long」）

add 极性不变（pdir）：在父级 k 加 m 个 pdir 方向 units，sub 级减 m 对应短差。`rec_add` 已有 `assert_eq!(inst.direction, dir)` 层内单一方向守卫——add 永不向 Long 实例加 Short（反之亦然）。τ 镜像（add_short）走父空分支，对称。

### 2.3 τ 镜像对称（任务 #40 单测「O3 τ 镜像」）

add（父多买回）↔ add_short（父空卖回）= τ 镜像，**纯群论对称**（#39 §3.1b：不动 h ⇒ 莫比乌斯不适用 ⇒ 平凡对称双射）。代码层：add 用 `pdir`（父向），父多则买回多、父空则卖回空，**单一方法 τ 对称**（无 if 多空硬编码，pdir 参数化）。

## 三、route_bsp 9 轨道分派（env T_ORBIT9_DISPATCH）

### 3.1 硬约束遵守

1. **账本过滤是 route 之后独立 gate，非 route 内分支**（任务 #40 硬约束，禁重犯 #35 C7/初版 NL7）：route_bsp 9 轨道分派**只做操作语义**（按 Σ_state 分派），**不含任何 if regime / if account / 空头损益判断**。空头镜像（add_short/sink/ascend_short）操作语义层一律合法执行——是否过滤由**独立账本 gate**（route 返回后，本任务不实装账本 gate，委托部署层；route 内零账本分支）。
2. **按 Σ_state↦轨道分派，非 if regime/level 硬编码**：分派依据 = `nearest_active_parent(j)`（子级/核心级，= Σ_state 的 rel 轴）+ `highest_active`（方向态，= τ_t 轴）+ is_buy（bsp 轴）——这些是**结构涌现**的 Σ_state 投影，非硬编码 regime。
3. **OFF bit-exact**：`T_ORBIT9_DISPATCH` 缺省 OFF ⇒ route_bsp 走旧二元（O3 add 分支不激活，同父向无短差仍 no-op）⇒ 与 facea 54279a503e 逐字一致。

### 3.2 O3 接入点

O3 add 接入 route_bsp **Some(p) 同父向**分支（当前 `buy_noop`/no-op 点）：
- 旧（OFF）：同父向 BSP，j 持短差 → recover（整条）；j 无短差 → no-op。
- 新（ON）：同父向 BSP，j 持短差 → recover（整条，走势完成）**或** add（部分买回，回调未完成）。**判别"走势完成 vs 回调"= C 任务（区间套 H¹ 定位）接口**——本实装按接口 spec 占位：`is_sub_trend_done(j)`（C 提供，缺省 fallback = 现状 recover）。

## 四、接口需求（B 需要 A/C 提供什么）

| 接口 | 提供方 | B 的用途 | 占位 fallback |
|---|---|---|---|
| `is_sub_trend_done(sub)` → bool | **C（区间套 H¹ 定位）** | 区分 O7 recover（走势完成，整条升回）vs O3 add（回调未完成，部分买回） | fallback=true ⇒ 全走 recover = 现状 bit-exact |
| `confirmed_flip_dir(j)` → Option<Polarity> | **A（H⁰ 方向骨架）** | O1 flip 的 confirmed 门（A 的 flip confirmed 门接口） | fallback=现状 route_bsp None 反向即 flip |

**B 不自造 C/A 的判别逻辑**（禁越界）：O3/O7 的"走势完成 vs 回调"判别**委托 C**；flip confirmed 门**委托 A**。本实装提供 add 原语 + 轨道分派骨架 + 接口占位，实际整合由 lead/协调节点（#38）。

## 五、结果包六要素

### 1. 结论
新增 `add(parent, sub, c)` 原语（O3 = 买回/卖回 = 不动 h 同级别短差腿**部分**重建，部分平 sub 短差 m=quota 升回父向 pdir，τ 镜像对称，极性不变不污染 long）+ route_bsp 显式 9 轨道判别（env `T_ORBIT9_DISPATCH`，OFF ⇒ 旧二元 bit-exact）。八轨道（O1/O2/O4-O9）复用已有原语（轨道标注，无行为改变），唯一结构增量 = O3。

### 2. 定义依据
- **#39 §3.2 轨道表**：O3 = add/add_short，来源态 S2（type1后回调,中枢上），op=开，极性不变加仓。
- **#39 §3.1b**：add = 不动 h 同级别短差腿重建 = 多头买回 ↔ 空头卖回 τ 镜像（莫比乌斯仅作用 h，对不动 h 操作平凡对称）。
- **543**：操作 = word（Σ\*={e,h⁺,h⁻,τ}\*），add 非新原子（仍 σ-不变配额 quota 的同级别重建）。
- **任务 #40 硬约束**：账本过滤是 route 之后独立 gate；O3 τ 镜像；OFF bit-exact；致命否证退回零残留。

### 3. 边界条件（结论翻转）
- 若 **add 改为 pyramid 加新仓（非买回被减掉的腿）**，则违反「不动 h 同级别重建」语义，退化为加杠杆（杠杆来源应是级别叠加 #579，非 add 内）。**反例：add 配额 m=quota(机动仓) 且 sub 须持反父向短差才 add（无短差→no-op，不凭空 pyramid）⇒ 不翻转。**
- 若 **route_bsp 9 轨道内含 if account/regime（空头过滤进 route）**，则重犯 #35 C7/NL7（账本当结构）。**反例：route 内零账本分支，空头镜像一律执行，账本 gate 独立于 route ⇒ 不翻转。**
- 若 **OFF 非 bit-exact**，则致命否证 ⇒ 退回零残留（任务 #40）。**待 D-task L1 验证（OFF vs facea 54279a503e）。**

### 4. 下游推论
- 9 轨道 = route_bsp 二元 is_buy 坍缩的完整 Ω 值域 ⇒ 承接 #35 下游（539 过/欠覆盖架构层消除，L3 净收益未决）。
- O3 接入需 C（区间套定位）判别 recover vs add ⇒ B/C 整合由协调节点（#38）。
- 空头镜像操作语义合法 + 独立账本 gate ⇒ #35 C7/NL7 双重错的正解（操作语义全函数 + 账本过滤分层）。

### 5. 谱系引用
- **#39（操作语义穷尽分类）**：9 τ对称轨道 + S1-S8↦轨道 + 账本⊥操作语义分离（本实装的直接依据）。
- **543（操作=word≠硬编码循环）**：add 非新原子，OP_REBUY；#39 §3.4 精化「O3 不动 h」（与 543 `h⁺∘σ` 的张力由 #39 终版裁决，path 层 word vs 群作用层不动 h）。
- **541（D∞/莫比乌斯/P2）**：τ 镜像对称的群论根（不动 h ⇒ 莫比乌斯不适用）。
- **概念分离领域（多空对称/空头非对称）**：本实装遵守 #39 分离线——操作语义（τ 对称）⊥ 账本（P&L 过滤）。route 内零账本，空头镜像合法执行。

### 6. 影响声明
- **新增**：`rec_engine.rs` add 原语（O3）+ route_bsp 9 轨道分派（env T_ORBIT9_DISPATCH 门控）+ 单测（add 不污染 long / O3 τ 镜像 / OFF bit-exact）。
- **接口占位**：`is_sub_trend_done`（C）/`confirmed_flip_dir`（A）按 spec 占位 fallback，实际整合由 #38 协调节点。
- **未改动**：八轨道原语（enter/ascend/flip/sink/recover/drain/no-op）行为 bit-exact。
- **谱系待动作**：建议结晶「O3 = 不动 h 同级别短差腿部分重建 ≠ recover 整条」（fold #39 或新号）。
