# #1346 四类谓词：输入 / 输出 / 反例，与「集合非空 ≠ chosen gate」证明

> 全部是对已裁定义（qujiantao.md §6、#1245/#1246/#778/#1224、Lean `formal/Origin/NestingCertificate.lean`）的整理，不新裁。记号：δ ∈ {buy, sell}；c 为级别 k 的候选元素；I(c) 其区间；J = J_{k-1} 下一级已选区间；t 前缀。

## 1 元素谓词 `DivCand^δ_k(c,t) = Dir ∧ Comparable ∧ Extreme`

| 项 | 输入 | 输出 | 真例 | 反例（单项为假） |
|---|---|---|---|---|
| Dir | c 的方向 dir(c)、δ | `dir(c) = −δ` | δ=buy，c 为下跌段 | δ=buy，c 为上涨段（Rust `div_cand_fail` → 1） |
| Comparable | c、c 的父走势类型与其最近中枢 B、父内的同向跨界段集 | 存在合法参照 s'：趋势 ⟹ s' = b（进入 B 的段）；盘整 ⟹ s' = A_i（c = A_{i+2} 的上一同向离开段）；二三类/小转大递归到两支 | 盘整块：A_1 下离开、A_2 回抽、A_3 再下离开 ⟹ Comparable(A_1, A_3) | 父无中枢（Segment）⟹ 无「界」⟹ false（→ 2，合法定位失败）；c 未破核心（中枢内震荡）⟹ false；首次离开且无进入段（序列起点）⟹ false |
| Extreme | c、s'、δ | buy：lo(c) < lo(s')；sell：hi(c) > hi(s')；分域：趋势/盘整必须，二三类点身份不强制（#1246） | 盘整 A_3 低点低于 A_1 | A_3 低点不低于 A_1 ⟹ false（→ 3） |

**力度（Weak）不在元素谓词内**（ADR 0006、D1、#817 N-5 裁定一）；它是候选之后的独立门，Rust `div_cand_fail` 的 4 号条件须在 TB-02-Q 拆出（CODE-MAP I-1）。

## 2 条件化集合与非空门

`C^δ_k(J,t) = {c ∈ C^δ_k(t) : J ⊆ I(c)}`；`Cand^δ_k(J,t) = [C ≠ ∅]`。

| 输入 | 输出 |
|---|---|
| 级别 k 全部 Closed 候选 `C^δ_k(t)`（元素谓词已求值）、J | 布尔 + 集合本身（CC-049：基数分区 EMPTY / SINGLE / MULTIPLE 与完整成员同时保留） |

反例：元素谓词全真但无一个 I(c) 包含 J ⟹ `Cand = false`（PDF p5 的 false-negative 警告正是「Sel 未条件化于 J」会把这类情形误判为有候选）。

## 3 选中 Sel_Θ 与 selected 身份

`c_k = Sel_Θ(C^δ_k(J,t))`，序 = end 最新 → start 最新 → idx 最小（#778 Q2，Lean `selOrder`）。

| 输入 | 输出 |
|---|---|
| 条件化集合 C（非空） | 唯一 `selected_id = (k, sel_key(c_k))`；AsKnown 归因 `selected_from` 关系指回 C 的全部成员 |

性质：`C = ∅ ⟹ Sel = none`（Lean `cand_false_implies_no_selection`）；`|C| = 1 ⟹ Sel = 那一个`；`|C| ≥ 2 ⟹` 按键序确定，MULTIPLE 不是失败也不是删除其余成员的许可（CC-049）。反例（序敏感）：两个候选 end 相同、start 不同 ⟹ 取 start 更新者；再相同 ⟹ idx 小者；三键全同 ⟹ 同一身份键（`sel_order_trichotomy_no_tie` 已锁）。

## 4 选中后门（selected gate）

G 是选中之后对 c_k 求值的门（力度门、下级完成义务、严档 Closed ∧ 零跳 等，按点型分派——#1347）。

| 输入 | 输出 |
|---|---|
| 固定 (parent, level, side, prefix, selector_version, G) 的生产请求，c_k 已定 | `SELECTED_GATE_TRUE / SELECTED_GATE_FALSE`，带 G 的 predicate_id |

**selected 失败不重选**：G(c_k) = false 时结果就是 false；不得回到 C 取下一个候选再试（RESIDUAL G-026 边界、SPEC C04.2、CC-052）。

## 5 证明：「集合非空」与「chosen gate」不是同一判断

设 `Cand = [C ≠ ∅]`，`SG = G(Sel_Θ(C))`（C 空时 SG 未定义，记 ⊥）。

1. **定义域不同**：`Cand` 的输入是 (C, J)，不含 G；`SG` 的输入含 G 与选择器版本。换一个 G（例如从 ForceL 换 MacdArea 对照档）`Cand` 逐位不变而 `SG` 可变——按 #804 判别式（同输入逐位相同才是同一判断）二者不同。
2. **真值表可分离**：存在 C 使 `Cand = true ∧ SG = false`（C = {c₁, c₂}，G(c₁) = false，G(c₂) = true，Sel 取 c₁）——这正是 CC-052 要求合法同时成立的 `SELECTED_GATE_FALSE ∧ ANY_GATE_TRUE`。若把二者当一个判断，这一行就要被改写成 true（重选）或 false（抹掉 c₂ 的观察），两种改写分别违反 C04.2 与 CC-052。
3. **蕴含只有一个方向**：`SG = true ⟹ Cand = true`（Lean `cand_false_implies_no_selection` 的逆否）；反向不成立（第 2 条）。单向蕴含的两个谓词不可能是同一判断。
4. **#799 核**：它们不是同一判断的宽严两档——`Cand` 判「有没有可承接的候选」，`SG` 判「被选中的那个过不过门」；`SG` 失败时不换到 `Cand`/`ANY_GATE_TRUE` 放行，就不存在「严的判不出换宽的」；而 `ANY_GATE_TRUE` 作为只读观察轴（CC-052 `readonly_all_candidate_result`）不进准入，满足 #799 限定词②（诊断模式不参与准入）。

## 6 观察结构（给 TB-02-Q 的对象形状；无新选择，全部是 SPEC CC-048/049/052/054 的既有要求）

```
CandidateObservation {
  request: {parent_id, level, side, prefix_cut, selector_version: "#778/sel-theta/end-start-idx/v1", gate_id: G},
  elements: [ {c_id, interval: I(c), dir, comparable: {case: trend|consolidation|type23|xzd_bridge, reference: s'_id | none}, extreme, divcand: bool, sub_of_J: bool} ],   // CC-048 全候选向量
  conditioned_set: {members: [c_id], cardinality: EMPTY|SINGLE|MULTIPLE},                                                                   // CC-049
  selected: {c_id | none, sel_key, selected_from: [c_id]},                                                                                  // ∃ + 身份归因
  production_selected_result: SELECTED_GATE_TRUE|SELECTED_GATE_FALSE|NO_CANDIDATE,                                                           // CC-052 轴 1
  readonly_all_candidate_result: ANY_GATE_TRUE|ALL_GATE_FALSE|NOT_EVALUATED,                                                                 // CC-052 轴 2（只读，不进准入）
  knowledge: {input_quality, applicability, computation, validity}                                                                          // CC-054 四轴
}
```
