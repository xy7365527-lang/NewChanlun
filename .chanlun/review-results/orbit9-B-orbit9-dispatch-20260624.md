---
crystallization_node: "swarm/orbit9-B-cryst"
subject: "task#40 B-task：9轨道操作分派 + O3 add 实装"
commit: "faaa0d6d93"
date: "2026-06-24"
sources:
  - ".chanlun/review-results/orbit9-B-dispatch-20260624.md"       # B-task 自述
  - ".chanlun/review-results/orbit9-B-review-20260624.md"         # 同质审查（约束3，异工位）
  - ".chanlun/review-results/orbit9-B-hetero-20260624.md"         # 异质审计（codex gpt-5.5）
epistemic_level:
  dispatch_logic: "L0（#39 §3.2 Burnside 闭合，轨道穷尽）"
  add_word: "L0（543 word 非新原子，codex Q1 确认）"
  off_bit_exact: "L1（管线级，单测通过 + 逻辑分析）"
  on_net_return: "L3 未决（委托 D-task #42）"
verdict: "PASS"
critical: 0
high: 0
medium: 2   # MEDIUM-1（B-review：T_ORBIT9_DISPATCH/T_OFF_BASELINE 互斥注释）
             # MEDIUM-obs（B-hetero：add_short PnL 路由账本层不对称）
d_integration_unlocked: true
---

# B 子工作结晶：9 轨道操作分派 + O3 add 实装

## 一、定论摘要

**B 子工作通过同质审查 + 真异质审计（codex gpt-5.5）。**

### 核心结论

`rust/src/recursive_t/rec_engine.rs` commit `faaa0d6d93` 完成了 9 轨道分派的实装：

1. **结构增量**：`route_bsp` 二元坍缩（is_buy）→ 显式 9 轨道分派（`T_ORBIT9_DISPATCH` 门控）。
   - O1/O2/O4/O5/O6/O7/O8/O9：复用已有原语（轨道标注，零行为改变）。
   - **O3 add/add_short**：新增原语（唯一结构缺口）。`fn add` = 同级别短差腿**部分**买回（m=quota(u_sub)=u_sub/3，极性不变，τ镜像对称）。

2. **OFF bit-exact（L1）**：`T_ORBIT9_DISPATCH` 缺省 OFF → route_bsp 逐字走旧二元分支，与 facea 54279a503e bit-exact。单测 `off_bit_exact_orbit9未激活_全走recover` 验证通过。OFF bit-exact 的 L2 数据验证委托 D-task #42。

3. **操作层 τ 对称**：`fn add` 主体无任何 `if pdir == Long` 硬编码分支，pdir 参数化贯通 rec_add/rec_reduce，实现多头买回 ↔ 空头卖回 τ 镜像（codex Q2 PASS，单测 `add_short_τ镜像_父空卖回对称` 验证）。

4. **三禁令合规（#35 §3.6/§5.4）**：route 内零 account_reduce 调用，无 if regime/account/level 门，add 是部分买回不触发 on_recover 整条守卫（codex Q3/Q4 PASS）。

5. **TW 守恒**：两半步各自 TW 中性（rec_reduce/rec_add 各自 free 变化 + NAV 贡献对冲），prove_tw_neutral 守卫通过（codex Q4 推导完整）。

6. **测试**：5 个 orbit9 单测全部通过，全量 lib 560 测试 0 failed。

---

## 二、必须标注的 MEDIUM-obs（B-hetero 传导）

**add_short（pdir=Short，空头卖回）的 PnL 路由进 `core_cost_basis`（降成本账本），而非 `short_leg_pnl`。**

### 为何如此

`account_reduce(mob, realized, c)` 以**仓位方向**（mob = flip_pol(pdir)）分路：
- `mob=Short`（多头买回场景，pdir=Long）→ `short_leg_pnl` 累加
- `mob=Long`（空头卖回场景，pdir=Short）→ `core_cost_basis` 更新 + 可触发 RecStage 阶段跳转

**这是三阶段会计系统的结构性设计（Long reduce = 核心高位卖出 ≠ Short reduce = 短差腿），非 add 引入的新不对称**。recover（O7）同样调用 `account_reduce(mob, ...)`，同一路由逻辑——add 从 recover 继承了这一特性，并无独立越界。

### 分离点标注（操作层 τ 对称 vs 账本层方向不对称）

| 维度 | 对称性 | 说明 |
|------|--------|------|
| 操作层（fn add 主体） | τ 对称 | pdir 参数化，无多空硬编码 |
| 账本层（account_reduce） | 方向不对称 | Long reduce / Short reduce 语义不同，三阶段会计既有设计 |

**下游 D-整合 L3 跑 bear 数据时须注意**：add_short（pdir=Short）的买回收益归因走降成本账本（core_cost_basis），不在 short_leg_pnl 中体现。bear 数据回放时若按 short_leg_pnl 分析 O3 贡献，将漏计空头卖回收益。

---

## 三、MEDIUM-1（B-review 传导）

**`T_ORBIT9_DISPATCH` 与 `T_OFF_BASELINE` 同时设置时，`production()` 走 off() 覆盖，orbit9 不激活——当前无明文注释警示。**

**位置**：`rec_engine.rs:282-290`（`production()`）和 `rec_engine.rs:212`（`from_env` 读 `T_ORBIT9_DISPATCH`）。

**严重度**：MEDIUM（不影响 OFF bit-exact 正确性，但可能造成 D-task 受控实验配置混淆）。

**建议**：在 `from_env` 的 `T_ORBIT9_DISPATCH` 读取处补注释：「与 `T_OFF_BASELINE` 同时设置时，production() 走 off() 覆盖此字段——受控实验勿混用」。

不阻塞 D-task #42，但建议 D-task 在配置脚本中显式互斥检查（assert_ne! 或 eprintln!）。

---

## 四、谱系评估建议

**建议 genealogist 评估「add = 543 word `h⁺∘σ` 的同级别不动 h 特例，非新原子」是否需引 543 谱系单独记录。**

根据 codex Q1 确认：
- `fn add` 底层操作序列 = `rec_reduce(sub, m) + rec_add(parent, give, pdir)`
- 与 `fn recover` 完全同形，唯一差别是配额 `m = u_sub/3` 而非 `u_sub`
- 未引入 `{e, h⁺, h⁻, τ}` 以外的新不可约操作

**概念分离点**（值得谱系标注）：
- 543 表述 `OP_REBUY = h⁺∘σ`（path 层 word）
- #39 §3.4 精化「O3 = 不动 h 同级别短差腿部分重建（非径向递归）」
- 二者在群作用层有张力，已由 #39 终版裁决——path 层 word 是一种表达，群作用层的「不动 h」是精化

这一张力-裁决对如果在后续其他操作分类中还会复现（如 ascend vs h⁺∘σ 的边界），值得引为通案记录（折叠 #39 谱系或另立号）。

---

## 五、D-整合关键接口（B-review 传导）

### 接口冲突点

A 工位（commit `0190470722`）为 `route_bsp` 增加了 `flip_confirmed: bool` 参数。
B 工位基于 `facea 54279a503e`，`route_bsp` 签名无此参数。

**D-整合策略（B-review 建议）**：
- 以 A 签名为基座（`route_bsp(..., flip_confirmed: bool)`）
- 在其函数体内保留 B 的 orbit9 同父向分支
- 二者修改不同代码段（A 改 O1 flip 逻辑，B 改同父向分支内 add/recover 分叉）
- **无逻辑冲突**，git merge conflict 可能发生在语法层（函数签名），手工合并即可

### B 暴露的接口清单

| 接口 | 类型 | 供 D-task 使用 |
|------|------|----------------|
| `EngineConfig::enable_orbit9_dispatch` | pub field | ON/OFF 控制 |
| env `T_ORBIT9_DISPATCH` | 环境变量 | 受控实验开关 |
| `fn orbit9_sub_trend_done` | private 占位 | C-task 替换接口 |
| `n_adds: u64` | pub 观测 | O3 买回计数 |
| `add_pnl_by_level: [f64; MAX_LEVEL]` | pub 观测 | 各级别买回收益归因 |

---

## 六、结果包六要素

### 1. 结论
B 子工作 PASS（0 CRITICAL / 0 HIGH / 2 MEDIUM 不阻塞）。9 轨道分派 + O3 add 实装经同质审查（异工位约束3）+ 真异质审计（codex gpt-5.5，四质询点全过），OFF bit-exact（L1 管线级），操作层 τ 对称。

### 2. 定义依据
- **#39 §3.2 轨道表**：9 τ对称轨道完全穷尽，O3=add/add_short，来源态 S2（type1后回调中枢上），op=开，极性不变加仓
- **#39 §3.1b**：add = 不动 h 同级别短差腿重建（部分买回），与 recover（整条）的精确区分
- **#39 §3.4**：O3「不动 h」精化裁决（vs 543 `h⁺∘σ` path 层表述的张力解消）
- **543**：操作 = word（{e,h⁺,h⁻,τ}*），add 非新原子（codex 确认 rec_reduce+rec_add 复用）
- **#35 §3.6/§5.4**：三禁令 + 账本⊥操作语义分离（route 内零账本，原子内自带会计）

### 3. 边界条件（结论翻转）
- 若 O3 add 配额改用全量 u_sub（非 u_sub/3）→ add = recover，O3 与 O7 合并，9 轨道退化为 8（不翻转：MOBILE_FRAC=1/3 是硬常量，quota 函数明确）
- 若 `orbit9_sub_trend_done` 从 `return true` 改为 `return false` → ON 状态下同父向持短差全走 add，OFF bit-exact 失效（不翻转：当前占位明确标注 fallback，D 整合替换时须保留 OFF 门）
- 若 route_bsp 引入账本状态前置条件（if pair_long_pnl > 0 等）→ 重犯 #35 C7（不翻转：当前 route 内零账本调用，codex Q3 验证）
- 若 rec_reduce/rec_add 中 free 的符号约定与 nav 中 sign(d) 不一致 → TW 双计（不翻转：两处约定一致，prove_tw_neutral 守卫兜底）

### 4. 下游推论
- 9 轨道是 route_bsp 二元坍缩的完整 Ω 值域，承接 #35（539 过/欠覆盖架构层消除）
- O3 接入需 C（区间套 H¹ 定位）提供 `is_sub_trend_done`，整合由 #38 协调节点
- D-task #42 现可解锁（B 是 #42 的最后一个 blockedBy，本结晶完成后）
- add_short（pdir=Short）收益归因走 core_cost_basis（降成本账本），L3 bear 数据须对此账本归因做专项分析

### 5. 谱系引用
- **#39**（操作语义穷尽分类）：9 τ对称轨道 + S1-S8↦轨道分派 + 账本⊥操作语义分离
- **543**（操作=word，OP_REBUY）：add 非新原子，#39 §3.4 精化与 543 张力的裁决来源
- **541**（D∞/莫比乌斯/P2）：τ 镜像对称的群论根（不动 h → 莫比乌斯不适用 → 平凡对称双射）
- **#35**（三禁令谱系；C7/NL7 正解）：route 内零账本分支的依据
- **不确定**：「recover vs add 判别时序」是否已有结算谱系（B-review 审查期间未找到相关结算，建议 genealogist 核查）

### 6. 影响声明
- **已完成（commit faaa0d6d93）**：`rec_engine.rs` 新增 fn add（36行）/ fn orbit9_sub_trend_done（占位）/ EngineConfig::enable_orbit9_dispatch / route_bsp 同父向 orbit9 分支（+6行）/ TRoot 新字段 n_adds/add_pnl_by_level / 5 个 orbit9 单测
- **八轨道原语**：行为 bit-exact 不变（仅轨道标注注释）
- **未改动**：其他模块（t_engine.rs / prove_guards.rs / 其他 rec_* 文件）
- **待 D-task**：MEDIUM-1 注释补充（T_ORBIT9_DISPATCH 与 T_OFF_BASELINE 互斥说明）
- **待 genealogist 评估**：add = 543 word 不动 h 特例的谱系记录

---

## session_append 标注

```
2026-06-24 B-cryst 结晶完成：orbit9-B-dispatch commit faaa0d6d93，9轨道分派+O3 add，
PASS（同质+异质），OFF L1 bit-exact，τ对称 L0，net return L3 未决（#42）。
MEDIUM-obs：add_short PnL→core_cost_basis（非short_leg_pnl），三阶段会计既有设计，D跑bear须注意归因。
MEDIUM-1：T_ORBIT9_DISPATCH/T_OFF_BASELINE 互斥未注释，不阻塞。
D-task #42 blockedBy 解除，可解锁。
谱系建议：add/recover 同构差配额 → 评估是否引 543 特例记录。
```
