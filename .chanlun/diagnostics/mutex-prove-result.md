# Lean 形式化互斥分类 MW1-MW8 + gap-B 不变量（mutex-prove 工位 #24）

> topo_address: swarm/mutex-prove (L0.24) | parent_callback: main
> 唯一真相源：`.chanlun/specs/2026-06-28-complete-mutex-classification-pdf-extract.md`（M01–M30）+ `推导完全互斥分类.pdf`
> 前置：#23 推导链复核（`.chanlun/diagnostics/mutex-derive-result.md`，2 个真 gap）
> 认识论等级：**L0 结构证明**（PDF §F + L3 8/8 已否证 v1 盈利，覆盖≠盈利，不膨胀）

---

## §1. ★gap-B 判定（最关键前置）：祖先生命期包含不变量 = **成立（TRUE）**，不 escalate

### 判定问题（缠论语义，非纯编码）

`∀a∈Anc(e), [λ_e,ρ_e) ⊆ [λ_a,ρ_a)`（祖先生命期包含后代生命期）在缠论结构下是否成立？
核心：**高级别走势父结束时，内部低级别子是否可能延续到父之后（ρ_e > ρ_a）？**

### 判定依据（缠论知识库 §8，定义推导）

- **走势分解定理二**：任何级别走势类型 **至少由 3 段以上次级别走势构成** ⟹ 父 = 构成它的子的
  首尾相接拼接（子 tile 父，无缝无重叠）。
- **走势必完美 / 技术分析基本原理一**：任何级别走势类型 **终要完成**；父在其 **最后一个构成子
  完成时** 完成。
- ⟹ 直接父子满足 **闭端点包含** `λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a`（第一子共享父左端 λ_e=λ_a，最后一子
  共享父结束 ρ_e=ρ_a；端点可重合）。
- ⟹ **子不能延续超过父**：父在最后一子完成时完成，更早的子已完成，ρ_e 之后无构成子存活。
- ⟹ 同时死亡（t=ρ_e=ρ_a）合法：e 因 **自己的 ρ_e** 离开，非被祖先提前剔除。

### 判定结论

**gap-B 成立**（TRUE，可证为引理），**不 escalate**（非反例，M23「吃到每个元素」无需修正定义）。
**但 M08 的开区间真包含 `I_e ⊂ I_{α_e}` 不忠实** —— 正确形式是 **闭端点包含**
`λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a`（端点可重合）。

### codex 异质独立确认（约束4 硬节点，VERDICT 逐点一致）

调用：`codex exec --skip-git-repo-check --sandbox read-only -c service_tier=fast`（stdin `- < /tmp/gapb_codex_prompt.txt`）。
独立性：codex 仅收到形式语义（无 spec、无本工位判断、无 PDF），从零判定。

| 问题 | codex 结论 | 本工位判定 | 一致 |
|------|-----------|-----------|------|
| Q1 直接父子闭包含 | YES | YES | ✓ |
| Q2 闭 vs 开 | **REVISE：闭非开** | 闭端点包含（非 M08 开区间⊂） | ✓ |
| Q3 传递到全祖先 | YES（inequalities compose） | YES（沿 ancestors 链 trans） | ✓ |
| Q4 子能否延续超父 | **NO**（父在最后子完成时完成） | NO（无反例） | ✓ |
| Q5 同时死合法性 | OWN ρ_e（合法，非提前剔除） | 同 | ✓ |
| **VERDICT** | **TRUE，闭端点形式** | TRUE，闭端点形式 | ✓ |

codex 给出 Q2 REVISE-DEFINITION（M08 的 `⊂` 应改闭包含）是真异质增量——本工位独立得同结论。

---

## §2. Lean 形式化（gap-B 处理后 MW1-MW8）

### gap-B 不变量真证：`Origin/AncestorLifespan.lean`（本工位新建）

| 定理 | 内容 | 对应 codex Q |
|------|------|-------------|
| `LifespanContains` | 闭端点包含谓词 `λ_a≤λ_e ∧ ρ_e≤ρ_a` | Q2 闭形式 |
| `lifespanContains_trans` | 闭包含传递（≤ 组合） | Q3 代数核心 |
| `DirectParentContains` | 直接父子闭包含公理（走势分解二+必完美的结构编码，显式 hypothesis 非 admit） | Q1 |
| `ancestor_lifespan_contains` | **gap-B 主引理**：全祖先闭包含 e（沿 ancestors 树深归纳） | Q3 |
| `ancestor_alive_in_child_life` | **gap-B 顶点**：祖先在子生命期内必活（覆盖证明「保持步」根据） | Q4/Q5 |
| `ancestor_not_ended_before_child` | 祖先在 e 生命期内未结束（D_t 不含 e 祖先） | Q5 |
| `child_not_outlive_ancestor` | 子不延续超父（ρ_e≤ρ_a，**反例不存在**坐实 VERDICT TRUE） | Q4 NO |

公理依赖：`#print axioms` 确认三核心定理仅依赖 `propext`（Lean 核心，**非 sorry/admit/自定义 axiom**）。

### MW1-MW8 复用运行中蜂群已 GREEN 成果（无 sorry）

扫描确认运行中蜂群已实装且无 sorry/admit/axiom：
- **MW1**（M04/M05 元素五元组+ρ_e）：`Origin/MutexElement.lean`
- **MW2/MW3/MW4**（级别关系+角色四分+M12 互斥穷尽 Σ指示=1）：`MutexElement` + `OperationRole18`/`MutexExhaustive`
- **MW5/MW6**（M25 同级别反向≠短差 / M27 短差覆盖）：`MutexExhaustive`/`SeparateCoverTheorem` 相关
- **MW7**（M16 AncOK 活动集）：`Origin/AncestorClosure.lean`
- **MW8**（M26 角色感知自相似递归）：`Origin/MutexRecursive.lean`/`SelfSimilarity.lean`

★**gap-B 与 MW7/MW8 的关系（本工位补的真缺口）**：现有 `AncestorClosure` 的 AncOK 只保证
「某 t 时刻祖先在集合中」（`Anc(e)⊆B`），**不保证「祖先生命期时间上覆盖子生命期」**；
`SeparateCoverTheorem` 的「保持开启」步把「祖先不提前死」**藏为字段假设** `activeAt`。本工位
`AncestorLifespan` 把这个被藏起来的不变量 **真证为结构推论**（祖先生命期闭包含 ⟹ 祖先在子
生命期内必活），填补 #23 §3-B 识别的核心 gap。

---

## §3. 硬约束验收（acc-prove）

| 约束 | 状态 | 证据 |
|------|------|------|
| 1. lake build Origin 通过 | ✅ | `Build completed successfully (114 jobs)`，`✔ [113/114] Built Origin.AncestorLifespan` |
| 1. 核心定理无 sorry/admit | ✅ | 扫描 MutexExhaustive/MutexFinalTheorem/SeparateCoverTheorem/MutexRecursive/SelfSimilarity/AncestorLifespan 零 sorry/admit/axiom |
| 1. gap-B 不变量真证（非 admit 跳过） | ✅ | `#print axioms` 三定理仅依赖 propext（核心公理） |
| 2. codex 从 /tmp read-only | ✅ | `/tmp/gapb_codex_prompt.txt`，VERDICT TRUE |
| 3. no-workaround（反例则 escalate） | ✅ | gap-B 成立非反例，不 escalate；M08 开区间→闭包含是忠实修正非补丁 |
| 4. 认识论 L0 | ✅ | gap-B 全 L0（端点 Int 代数+树深归纳），不膨胀盈利 |

build 中的 `info`（propext/Quot.sound）+ `SelfSimilarity` `hθ` 未用变量 warning 均为**既有**，非本工位引入。

---

## §4. 结果包六要素

1. **结论**：gap-B 祖先生命期包含不变量在缠论结构下 **成立（TRUE）**，正确形式是 **闭端点包含**
   `λ_a≤λ_e ∧ ρ_e≤ρ_a`（非 M08 开区间真包含 `⊂`）。新建 `Origin/AncestorLifespan.lean` 真证之
   （7 定理，零 sorry/admit/axiom），填补 #23 §3-B 核心 gap。`lake build Origin` 全绿（114 jobs）。
   MW1-MW8 其余单元复用运行中蜂群已 GREEN 成果。**不 escalate**（gap-B 非反例）。

2. **定义依据**：gap-B 成立依据缠论知识库 §8 **走势分解定理二**（父≥3 段次级别构成⟹子 tile 父）
   + **走势必完美**（父在最后一子完成时完成⟹最后子 ρ_e=ρ_a、首子 λ_e=λ_a）。闭端点（非开区间）
   依据「端点可重合」（首/末子共享父端点）。codex Q2 独立确认 M08 `⊂` 应 REVISE 为闭包含。

3. **边界条件**：判定翻转条件——(a) 若缠论中存在「子延续超过父」的配置（ρ_e>ρ_a，违反走势必完美），
   则 gap-B 为 FALSE（反例），须 escalate 修正 M23 的 Eat 定义（放宽到「祖先存活期内覆盖」）——
   codex Q4 + 本工位判定均确认此配置 **不存在**（父在最后子完成时完成）；(b) 若 `DirectParentContains`
   公理（直接父子闭包含）在实装 `par` 函数下不成立（如解析器产出非 tile 的父子关系），则传递链断裂——
   这是对实装 par 的结构约束，对应 rust MR2 的运行时形态；(c) 若把 M08 的开区间 `⊂` 当真（排除端点
   重合），则 t=ρ_e=ρ_a 边界处 gap-B 假命题——这正是 M08 表述不忠实之处，本工位用闭包含修正。

4. **下游推论**：(a) M23（定理3 全元素覆盖）/M26（自相似递归）的 `∀e Eat(e)` 不再依赖被藏起来的
   字段假设 `activeAt`——`ancestor_alive_in_child_life` 提供「保持开启」步的结构根据；(b) **spec M08
   的开区间真包含 `I_e⊂I_{α_e}` 应修正为闭端点包含** `λ_a≤λ_e∧ρ_e≤ρ_a`（reconcile 性质须回填——
   这是对 spec 的忠实性修正，建议 genealogist 记录「开区间⊂ vs 闭包含」的概念分离）；(c) rust MR2
   （AncOK 实装）须保证活动集更新不在 e 自己 ρ_e 之前剔除 e——对应 gap-B 的运行时形态（祖先生命期
   闭包含的实装维护）。

5. **谱系引用**：gap-B 是 #23 §3-B 首次显式化的概念缺口（祖先生命期包含 = 覆盖证明的未论证前提）。
   **建议 genealogist 核 `.chanlun/genealogy/` 确认是否为首次记录**——「M08 开区间真包含 vs 缠论闭端点
   包含」的概念分离（codex Q2 REVISE）可能值得专属谱系条目。关联 222/223/230号（有效域≠定义域）：
   gap-B 在 L0 定义域内成立（闭包含可证），不膨胀为实盘盈利（L2/L3）。

6. **影响声明**：本产出 (a) 新建 `formal/Origin/AncestorLifespan.lean`（gap-B 不变量真证，7 定理）；
   (b) 在 `formal/lakefile.toml` Origin lib roots 加 `Origin.AncestorLifespan`（紧跟 AncestorClosure）；
   (c) 新建本报告 `.chanlun/diagnostics/mutex-prove-result.md`。**不改任何现有 Lean/spec/rust/定义**
   （MW1-MW8 其余单元只读复用，未触碰）。影响下游：向 spec 发出 M08 开区间→闭包含的忠实性修正要求；
   向 rust MR2 发出 AncOK 不提前剔除的运行时约束。

---

## §5. 认识论等级标注（231号强制）

- **全程 L0**：gap-B 不变量在「构造的缠论元素树」（定义域）内由走势分解定理二+走势必完美的结构
  推导得到，是端点不等式组合的同义反复（信息增量为零）。`lake build` 绿 = 「闭端点包含+传递+
  不提前死」在 L0 定义域内逻辑成立，**不**等于真实市场每笔盈利。
- **严禁膨胀**：gap-B 成立 + M23/M26 覆盖定理成立，只是 **L0 全元素覆盖的结构性质**，不得膨胀为
  实盘盈利（PDF §F + L3 8/8 已否证 v1 盈利；覆盖≠盈利，认识论等级不同，不可互相否证）。
- **否定性边界的价值**：gap-B 的判定缩小了有效域边界——M08 的开区间真包含（排除端点重合）在
  t=ρ_e=ρ_a 边界处是 L0 内的假命题，闭端点包含才是忠实形式（这是 L0 内的忠实性修正，非 L2 否证）。
