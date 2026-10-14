# #1346 已裁同步清单：Comparable（#1245）与 Sel_Θ（#778）到 C(J,t) 的适用域桥

> 名分：task 查证（代理 claude-fable-5-1 依用户 2026-10-13 授权，根复核）。未写 GitHub、未改仓库、未跑 cargo/服务/浏览器。基线 main `1f5cdf76`（本机 `origin/main` 已核同值）。gh 只做 GET，缓存在 `gh-cache/`。原文四步核过（行号 / 逐字 / 正文界 / 行首与行内注；`049:38` 行尾有「(注：…)」，摘引止于其前；`031:881` 在界后，为作者答疑）。
> 本票不重开 Comparable、不重开同对象 Sel；只核对象域是否同一，并把已裁结论映射到 PDF 两层式与当前真实对象。

## 0 目标对象：`区间套.pdf` p4 两层式（qujiantao.md §6.0 照抄）

```
C^δ_k(t)            = {c : DivCand^δ_k(c,t) = 1}                    元素层（Dir ∧ Comparable ∧ Extreme）
C^δ_k(J_{k-1},t)    = {c ∈ C^δ_k(t) : J_{k-1} ⊆ I(c)}                条件化（子区间包含门）
Cand^δ_k(J_{k-1},t) = [C^δ_k(J_{k-1},t) ≠ ∅]                         集合非空门
c_k                 = Sel_Θ(C^δ_k(J_{k-1},t))      J_k = I(c_k)       选择
N^δ_{k↓e}           = Cand^δ_k ∧ N^δ_{k-1↓e}
```

对象域四件：元素 `c`（级别 k 的背驰段候选，一个次级别走势单元）、区间 `I(c)`（c 在共同坐标上的闭区间）、条件 `J_{k-1}`（下一级已选区间）、时刻 `t`（前缀）。本清单逐条把两张已裁票的对象对到这四件上。

## 1 #1245 Comparable（2026-08-25 裁定 5417835436）→ 元素层谓词的第二合取项

| # | #1245 原句 | 映射到 PDF 对象 | 当前真实对象（main 1f5cdf76） | 结论 |
|---|---|---|---|---|
| C-1 | 「Comparable 按走势类型分派」 | `DivCand` 的第二合取项是一个**二元关系** `Comparable(s', c)`，按 c 所在父走势的类型 case 分派 | Rust `cand_predicate.rs:430-443`（条件 2）以父 Compose 末中枢为「界」统一取段；Lean `NestingCertificate.lean:95-141` `Comparison ::= trend ∣ consolidation ∣ type23 ∣ xiaozhuandaBridge` 显式按类型分派 | **同一对象**：关系的定义域是 (s', c) 对，分派键是父走势类型。Lean 侧是裁定原形；Rust 侧是 #814 D-3 的统一操作化（见 C-5） |
| C-2 | 「趋势＝倒数第二中枢到最后的连接段 b 比最后离开段 c」（`037:16-18`【正文】、`031:881`【答疑】） | 父 = 趋势块 a+A+b+B+c；c = 候选元素；s' = b | Rust：父 `RMove::Compose{centers}` 取 `centers.last()` = B（`parent_last_center`，`:224-230`）；`d3_prev_crossing_anchor(…, bound = c.start_index, …)` 在首次离开时命中「`end_index ≤ B.start_index` 的最近同向段」= b（`:355-361`） | **同一对象**：b 是进入最后中枢 B 的段，D-3「首次离开 = 进入段」给出的正是它 |
| C-3 | 「盘整＝同中枢同向走势 A_i 比 A_{i+2}」（`027`、`049:38`【正文】「最标准的情况，当然是前面最近向下的」、`061:26`【正文】） | 父 = 盘整块（唯一中枢 B）；c = A_{i+2}（这次离开段）；s' = A_i（上一次同向离开段） | Rust：同函数，反复震荡时命中「上一次同向破核心段」（`d3_crosses_core`）；中枢内未破核心的震荡段两谓词均不中，跳过 | **同一对象**：A_i 与 A_{i+2} 之间恰隔一段反向走势，就是「上次同向离开段」。D-3 多出的「只算跨界段」是 `049:38`「向下**离开**」与 `061:26` 例子（55-60 进入、65 离开，两段都在中枢外）的操作化，#814 D-3 已裁 `[新缠论:推论]`，#1231（2026-08-26，晚于 #1245）再次确认为唯一取段规则 |
| C-4 | 「二三类点与小转大 c′ 递归落入两支、不另立口径」 | Comparable 的四类入口归约为两支 | Lean `comparableComponent`：`type23 base ⟹ baseComparable base ∧ baseExtreme base`；`xiaozhuandaBridge base ⟹ baseComparable base`（`:138-141`）；Rust 由 `parent_center` 是否存在决定能否取段（无中枢 ⟹ 条件 2 合法失败） | **同一对象**；#1347（g3）Q1 已把 Type2/3 证书收窄为「基例确认 ∧ 自身下级完成义务」，与此递归委托不冲突 |
| C-5 | 「多合法参照段保留全配对＋外层存在量化；『最近』只作 Sel_Θ，不进 Comparable；最近同向前驱私货（`cand_predicate.rs` / `xzd.rs`）废弃」 | 对每个 c，Comparable 给出其合法参照 s'（趋势唯一 b；盘整唯一 A_i）；∃ 量化在 s' 上；**候选间**的「最近」在 Sel_Θ | 被废的私货是 **#883 之前**的「同父前序最近同向段、无中枢」口径（`cand_predicate.rs:24`「已退役，不并存」）。现行 D-3 的 `rfind` 找的是**参照 s'**（同向跨界段中最近者），不是候选 c 之间的选择；且对给定 c 它是唯一的（趋势只有 b，盘整只有 A_i） | **不冲突，但要写清一句**：D-3 里「往回最近」是关系 Comparable 的**定义本身**（A_i 是 A_{i+2} 的前一同向离开段），不是候选集上的选择器；候选集上的「最近」仍只在 Sel_Θ（见 §2）。参照侧不存在「多个合法 s' 被归约」的情形——`049:38` 的「最近向下的」指的就是唯一的前一同向离开段 |
| C-6 | 「实施落 #1225」 | — | #1225 已关（2026-08-26 收口 `0199749c48`）：Lean 定义式落 `NestingCertificate.lean:151-153` `DivCand = dir ∧ comparable ∧ extreme`；Rust `div_cand` **仍为四条件**（含 Weak，`:384-386`、`:454-470`） | **实施缺口 I-1**（见 CODE-MAP.md）：元素谓词三条件在 Rust 只以 `div_cand_fail ∉ {1,2,3}` 的形式可提取，未单独成函数；qujiantao.md §9 清单 46 已登记 |

**Extreme（#1246，同批）**：趋势必须（`037:20`）、盘整必须（`044:229-234`）、二三类点身份不强制但其子判断内必须、小转大桥接不适用 ⟹ Lean `extremeComponent`（`:147-149`）逐字落；Rust `div_cand_fail` 条件 3（`:445-452`）只实现了趋势/盘整两支（它只在有 `parent_center` 时求值，Type2/3 入口由 #1347 另路分派）。对象域同一。

## 2 #778 Sel_Θ（2026-08-15 裁定 5299876634）→ 条件化集合上的选择

| # | #778 原句 | 映射到 PDF 对象 | 当前真实对象 | 结论 |
|---|---|---|---|---|
| S-1 | Q1「∃ + 身份归因，不是唯一正确链；确定性由固定 Sel_Θ 给」；「拐点唯一 ≠ 候选唯一」 | `Cand = [C(J,t) ≠ ∅]` 是 ∃ 门；`c_k = Sel_Θ(C(J,t))` 给身份；两者是同一集合上的两个算子 | Lean `Cand`（`:246-247`，`!isEmpty`）与 `SelTheta`（`:239-240`）分写；定理 `cand_false_implies_no_selection`（`:250-259`） | **同一对象**：#778 的 Q1 就是 PDF 两层式的语义名分 |
| S-2 | Q2「Sel_Θ = end 最新 → start 最新 → idx 最小（Lean `selOrder`，`Strict/Nest.lean:117`）」 | Sel_Θ 的序材料 = `I(c)` 的 (end, start) 与候选编号 idx，都是结构身份键的纯函数（#780 票面要求） | `Strict/Nest.lean:117-120` `selOrder`；Rust `strategy/nest.rs:55-59` `sel_order`（end_time/start_time/idx）；`classifier/nest.rs:63-67` `sel_order`（end_index/start_index/idx）；三处逐字同向 | **同一对象**：键取自 `I(c)` 的端点与 idx。坐标：`strategy/nest.rs` 用 `u64` 时间，`classifier/nest.rs` 用 `source_index`，PDF 用抽象下标——同一区间在不同编号域的投影（#817 N-1 编号域裁定），不改序 |
| S-3 | 「现存两条 DFS 都反了 Lean 方向（取最早可行而非 end 最新）」 | 选择器实现方向 | `classifier/nest.rs:978-985` `order.sort_by_key(sel_key)` **升序**（最早 end 先）；`:1250-1258` 按 `(parent start,end)` 升序——**仍未对齐** | **实施缺口 I-2**：#778 连带动作 1「随下游实施票落」至今未落（qujiantao §4.8 该链单元素时序无实效，但 TB-02-Q 要产出真实多候选，必须先改） |
| S-4 | Q3「选择器是消费侧 NestChainGate 的职责；输入只收 Closed」 | 候选集 `C^δ_k(t)` 的元素 = 已闭合（Closed）的证书对象；Open 尾不是元素 | `admission.rs:500` `strict_chain_verdict`（#1225 新函数，严档 Closed ∧ 零跳）；`classifier/nest.rs` 装配只读 `events_by_level`（已判事件） | **同一对象**：PDF 的 `DivCand(c,t)=1` 在 t 时刻成立 ⟺ 证书已 Closed；活假设不进 C(t) |
| S-5 | 「`selected_key_unique` 语义从『唯一』改『确定 + 可归因』」 | 选中身份 = `(level, sel_key)`，AsKnown 归因 | `classifier/nest.rs:146` `selected_keys()`；S 层关系种类已声明 `selected_from`（`s_session/s_tb02_contract.py:890`、`tb01c-client.js:446`）但**无生产者** | **实施缺口 I-3**：归 TB-02-Q #1439 |
| S-6 | 「先子区间包含过滤、后 Sel」（#1224/#1225 落 Lean `eligibleCandidates`） | `C(J,t)` 先于 Sel | Lean `:232-240` 严格先过滤；Rust `classifier/nest.rs:987-996` 在排序后循环中以 `is_sub(child,parent)` 过滤 ⟹ 等价于先过滤后取序首；`strategy/nest.rs:99-101` `chosen()` **不吃 J**（PDF p5 反例，qujiantao §9 清单 49） | **实施缺口 I-4**：`strategy/nest.rs` 侧未条件化（但该侧链恒单元素，qujiantao §4.8） |
| S-7 | 「旧模块 owner 不覆盖已批准 R2」（票面继承句） | — | R2 `economic_session` / S 会话层无候选集、无 Sel、无 selected gate 对象（grep `selected|chosen|sel_key` 只命中合同声明与测试） | 已裁语义可直接复用；owner 换到 R2 结构判定域（ADR 0027），由 TB-02-Q 实装 |

**Lean 内部一处弱化（不是新选择）**：`NestingCertificate.lean:239` `SelTheta = (eligibleCandidates …).head?`，注释「调用方可按『最近优先』排列输入」——它把 Sel_Θ 做成了对输入序敏感的 `head?`，而 #778 裁的是固定键序 `selOrder`（`Strict/Nest.lean`）。这是**形式化侧的实施缺口 I-5**（应改为 `eligibleCandidates` 上按 `selOrder` 取极值，或证明输入已按 `selOrder` 排序为前件），不是对 Sel_Θ 语义的第二种选择。

## 3 对象域同一性结论

| 维度 | PDF C(J,t) | #1245 | #778 | 判 |
|---|---|---|---|---|
| 元素 | 背驰段候选 c（次级别走势单元） | 比较对（s', c）的 c | 证书事件的 typed interval 所指的候选 | 同一 |
| 层级 | 级别 k 的候选，父为 k+1 走势 | 父走势类型决定分派 | `events_by_level[level]` | 同一 |
| 坐标 | 抽象区间 I(c) | 段的 start/end（source_index） | NestInterval(start_index,end_index,idx)/Interval(end_time,start_time,idx) | 同一区间、不同编号域投影 |
| 条件 | J_{k-1} ⊆ I(c) | — | `is_sub(child,parent)` / Lean `subB` | 同一 |
| 选择 | Sel_Θ | 「最近」只在此 | end 最新→start 最新→idx 最小 | 同一 |
| 门 | [C ≠ ∅] | — | ∃ + 身份归因 | 同一 |

**⟹ 两张已裁票与 PDF 两层式是同一对象域，可直接复用；差异全部在实施侧（I-1～I-5），无一处需要新的语义选择。** 见 `NEW-GRILLING.md`（无）与 `CODE-MAP.md`。
