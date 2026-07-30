# #802 区间套纵向下钻是不是真递归——rung 恒真的事实核查

- 日期：2026-07-30
- 票据：[#802](https://github.com/xy7365527-lang/NewChanlun/issues/802)（parent [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)，blocking [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799)）
- 性质：**只查不改**。本报告未改动任何生产代码，不给实现建议（那是 #799 的事）。
- 分支：`probe/i802-nest-descend-recursion`（自 `docs/grill-with-docs-entry` @ `5984ab0d8e` 切；`main` 不含 `rust/`）
- 标注约定：**【事实】**＝代码/文档/裁决可直接核对；**【推断】**＝由事实推出但无直接留痕；**【未能判定】**＝证据不足。

---

## 0. 一页速查

| 问 | 结论 |
|---|---|
| 1 rung 循环在构造什么 | **【事实】** 区间是逐级真取的（每级各自的含点段，不是抄上级）；但**判定**分叉：Type1 每级真跑 `div_cand`（拿该级实际子结构窗口判），Type2/3 每级恒 `true`。对 Type2/3，`n_delta` 的全部判别力 = base gate 一次 + 终端 bit——rung 链零判别力 |
| 2 Type1/Type2-3 分叉合理否 | **【事实】** 第17课 L60/L66 只保证「本级 Type2 存在且由次级别 Type1 构成」，是**基例级**存在命题；把它当上级 rung 逐级免判的授权是**引申超出原文**。且本仓自己后来把 `Cand^δ_ℓ` 定义成「塔上 per-level 背驰段谓词」（nest.rs P2 裁决①）——恒真与本仓已结算定义不一致；但 rung 级放宽另有 #97 D1 裁决背书（见 §2.3），不是无名分的偷懒 |
| 3 descend anchor 在锚什么 | **【事实】** 锚次级别 Type1（定律一，第29课 L396），且是**真结构递归**：逐级钻 `sub_moves`、每级跑完整 `div_cand`、递归到底、返回深度，有测试锁。但生产**只消费 `is_some()`**，深度全部丢弃；`lvl==0` 整个免门；Type1 信号从不下钻 |
| 4 用没用 RMove 嵌套 | **分方向**。**下钻用了**：`descend_type1_anchor_depth` 走 `sub_moves`（与 `RMove::Compose.subs` 同构的携坐标侧车）——真嵌套结构。**上行 rung 链没用**：π 门用塔索引 `partition_point` 点包含查 `tower[k]`——「用横向的表示做纵向的事」属实；但已有 bottom-up 区间包含对拍探针实测 BTC 三窗 bit-exact 差异 0，且严格装配臂（`extend_typed_upward`）本来就是真 ⊆ DFS |
| 5 横向形态可否照搬 | 可行性**高**：`div_cand` 已是「判定作参数」形（层无关，上钻/下钻同一判据复用）；`descend_type1_anchor_depth` 靠结构见底（`sub_moves` 空），无级别分支。残留违例恰好三处：`cand_delta` 的 `=> true` 臂、两处 `lvl == 0 \|\|` 免门。最小改造面即这三个锚点 + 一个待 #799 裁的 rung 谓词语义 |

**一句总判**：**纵向下钻在机制层是真递归（有实现、有测试锁），但在 π 门的消费层退化**——Type2/3 上级 rung 恒真有裁决背书、且可论证为省略；真正的空洞在消费侧：`is_some()` 弃深度、`lvl==0` 免门、Type1 从不下钻。编排者假说的强形式（「纵向没实现成递归」）**被推翻**；弱形式（「入口查一次、下面全部放行」）对 **π 门 Type2/3 的 rung 链**逐字成立。

---

## 1. 第 1 问：`build_nest_certificate` 的 rung 循环到底在构造什么

生产 rung 循环：`rust/src/theta_v0/backtest/econ_positive.rs:1030-1064`。

### 1.1 区间：每级真取，不是抄上级 【事实】

对每个上级 `k ∈ (lvl, tower.len())`，循环在 `tower[k]` 里用 `partition_point` 找**包含 `source_index` 的该级段**（`econ_positive.rs:1038-1039`，`start_index ≤ source_index ≤ end_index`），取其 `[start,end]` 作 `interval_k`。每级的含点段是不同对象（不同级别的 `LeveledMove`），区间逐级加宽——**不是**把上级区间抄下来。找不到含段即 `break`（partial chain 合法，codex #39 Q1 裁定，`econ_positive.rs:1040-1046`）。

### 1.2 判定：Type1 真判，Type2/3 常真 【事实】

per-rung `Cand^δ_k` 分派在 `cand_delta`（`econ_positive.rs:952-964`）：

```rust
match cand_type {
    BspCandType::Type1 => cand_delta_type1_extreme(rung_subs, source_index, delta, hist),
    BspCandType::Type2 | BspCandType::Type3 => true,
    BspCandType::StructBreak => false,
}
```

- **Type1**（`econ_positive.rs:892-909`）：在 `knode.sub_moves`（该级含点段自己的次级别窗口序列——每级 k 各不相同）中找 `end_index==source_index` 的段，跑**完整** `div_cand`（Extreme+Weak 四条件）。找不到 ⟹ `false` ⟹ `n_delta` 拒。这是**真的逐级重建下钻语境**。
- **Type2/3**：字面常量 `true`。该级不做任何判定。

### 1.3 对 Type2/3，rung 链的净判别力为零 【事实＋推断】

`n_delta` 递归核（`rust/src/theta_v0/classifier/nest.rs:340-359`）逐级合取 `cand ∧ is_sub(child, parent) ∧ 递归`。对 Type2/3：

- `cand` 恒真（上）；
- `is_sub` 由塔 Compose 不变量结构性保证——`econ_positive.rs:1070-1079` 的 #100 看守把「嵌套链破裂」当 **bug** 断言（debug_assert，release 编译掉）。**【推断】** release 下 `is_sub` 理论上可 false，但按不变量声明它失败即塔实现有 bug，正常运行恒真。

⟹ 对 Type2/3，`n_delta ≡ cand_delta_base_gate（构造时已过）∧ terminal.confirm_side`。rung 循环对 Type2/3 只产出诊断读数（`effective_nest_depth`、`nest_depth=rungs.len()` 进 z 第 10 维），**不产出任何判别力**。「入口查一次（base gate）、下面全部放行」对该路径逐字成立。

### 1.4 附带事实：`lvl==0` + rungs 空时，连 Type1 也不判 【事实】

Type1 无 base gate（`cand_delta_base_gate` Type1 臂恒 `true`，`econ_positive.rs:979`），其判据全在 per-rung。若 rungs 为空（无上级含段），`n_delta` = 纯基例 `confirm_side`（单 bit）。实测记录（`econ_positive.rs:361-364` 注释，BTC 300K）：**95.36% 通过门信号 rungs 空，仅 4.64% 真跨级且 max 深度=1**。#796 双臂实测（`.chanlun/review-results/issue796-nest-gate-open-20260730.md`）：52 个 admit 里真链 Pass 仅 1。即生产中该门绝大多数时候两头都不判：rung 链空 + （Type2/3 在 lvl==0）base 免门。

---

## 2. 第 2 问：Type1 与 Type2/3 的分叉、第17课 L60 援引站不站得住

### 2.1 原文核对 【事实】

- `docs/chanlun/text/blog/017-第17课.md:60`：第一类买点后「第一次次级别回调制造的低点，是市场中第二有利的位置……必须包含三个以上的次级别运动，因此后面必须还至少有一个向上的次级别运动，这样的买点是绝对安全的」——**Type2 存在性**命题（由走势必完美保证）。
- 同文 `:66` 定律一：「任何级别的第二类买卖点都由次级别相应走势的第一类买点构成」。
- `docs/chanlun/text/blog/029-第29课.md:396`：「所有买点，归根结底都是第一类买点，要找第二、三类，其精确的，都要下次级别以下找第一类」——base gate（`descend_type1_anchor_depth`）的援引，**这条用得对**：代码确实实现成「下次级别找 Type1」。

### 2.2 援引评估 【推断，与事实分开】

L60/L66 是**关于信号本级**的存在命题：本级 Type2 的精确点由**次级别** Type1 构成。它推得出：base gate 用次级别 Type1 锚定 Type2/3 存在性是有原文依据的。它推**不**出：上级 rung k（k>lvl）的 `Cand^δ_k` 可以逐级免判——原文根本没有「上级候选谓词」这个对象。两个额外弱点：

1. 第17课不含第三类买卖点（Type3 后出，中枢定理系）；代码注释把 Type3 一并挂在「第17课L60 完备性」下，**援引对象错位**（Type3 的正确原文锚是 29 课 L396 的「二、三类」，那是问答引申，非定理正文）。
2. 「存在」≠「每一级都不必再判」：存在性保证的是**至少有**，rung 判定要回答的是**这一级的这个语境里成不成立**。存在命题填不满逐级谓词。

### 2.3 但恒真不是无名分的偷懒——本仓有两道明示裁决 【事实】

- codex 673-fix 裁决①（`econ_positive.rs:948-951` docstring）：「Type2/3 存在性已由 base gate 门控（base 一次），上级 rung 载上级语境 ⟹ cand=true」。
- 严格装配臂同款：`nest.rs:989-990`「#97 初筛只看结构（D1 裁定：Cand=纯结构宽候选，力度留在②基例生产门）：rung 级不再要求 divergence_confirmed」——`extend_typed_upward` 里 rung 的 cand 同样硬编码 `true`（`nest.rs:998`），力度真值只进 sidecar 不做门。

即「rung 级不判力度、判定收敛到基例」是**两臂一致、两次裁决背书**的设计，不是单点注释的即兴省略。

### 2.4 与本仓已结算的 `Cand^δ` 定义的张力 【事实】

spec 疑点2（`nest.rs:155-157`）：`Cand^δ_ℓ(x)` 在 PDF 中仅作符号出现、无定义式。P2 落定补缺（`nest.rs:1053-1057`）：「`Cand^δ_ℓ(x) ≔ 塔上 per-level 背驰段谓词` = `CandDeltaEvent::cand_delta`」。按这个已结算定义，per-rung 恒真**不是**该谓词的实现——它是被 #97 D1 又一次改判后的「纯结构宽候选」语义。三道裁决（P2 定义式 → codex 673-fix 恒真 → #97 D1 结构宽候选）方向不一，**谁覆盖谁没有一张总裁定书**。**【未能判定】** rung 谓词的最终语义归属——这正是 #799 要裁的对象；本票只陈列三道裁决共存的事实。

---

## 3. 第 3 问：descend anchor 在锚什么、承担了下钻判定没有

### 3.1 它是真递归 【事实】

`descend_type1_anchor_depth`（`econ_positive.rs:815-843`）：

- 输入候选段 `s`，取 `s.sub_moves`（次级别序列），找 `end_index==source_index` 的段，跑**完整** `div_cand`（四条件，含盘整背驰，与上钻路径同一判据复用，no-patch）；
- 锚成立后**自递归**钻入该次级别段（`econ_positive.rs:839`），逐级收缩到最低可用级别，`sub_moves` 空 = 递归底；
- 返回 `Some(depth)`，depth = 实际穿越层数；
- 测试锁：`descend_anchor_recurses_below_one_level`（`econ_positive.rs:3042-3068`）断言两层下沉返回 `Some(2)`。

这就是「给定点往下一级钻找更精确位置」的实现，且形态正确：判定（`div_cand`）作参数化复用、无级别分支、结构见底。

### 3.2 但生产消费退化成一个存在位 【事实】

生产调用点全部只取 `is_some()`：

- `cand_delta_type2_completion` / `cand_delta_type3_retest`（`econ_positive.rs:926,945`）：`lvl == 0 || descend_…(…).is_some()`；
- `pan_div_gate_pass` 通道1（`econ_positive.rs:1392`）：同款 `is_some()`。

深度值唯一的消费者是诊断累积器（`econ_positive.rs:7318` 起，`depth_t23` 直方图）。**精确定位的产物（下沉到哪一层、锚在哪个段）在生产路径上全部丢弃**——门只知道「能下去」，不知道「下到了哪」。

### 3.3 「合理省略」评估 【推断】

- **base gate 已承担的部分**：对 lvl≥1 的 Type2/3，下钻判定确实真发生了、且递归到底——这部分 rung 恒真可辩护为省略：per-rung 若再跑一次 descend，锚仍是同一个 `source_index` 对齐段，重复判定。
- **base gate 没承担的部分**（三个空洞，皆【事实】）：
  1. `lvl==0` 免门（`econ_positive.rs:926,945`）——level0 信号（生产主体）**零下钻**。注释理由「塔不从笔递归是构造选择，非客观无次级别」（订正 #520）——即免门是表示能力缺口的妥协，不是领域结论；
  2. **Type1 从不下钻**（`econ_positive.rs:7299-7301` 诊断代码注释逐字：「Type1 走本级 div_cand，不下沉——非本群」）——第一类买卖点的区间套精确定位（第29课 L18「通过1分钟以及1分钟以下级别的精确定位」）在 π 门无对应实现；
  3. 深度弃用（§3.2）——「精确位置」这个区间套的本义产物没有被任何生产决策消费。

---

## 4. 第 4 问：纵向用没用 `RMove` 的嵌套结构

### 4.1 下钻方向：用了（经侧车） 【事实】

`descend_type1_anchor_depth` 走 `s.sub_moves`。`sub_moves` 是 `RMove::Compose.subs` 的**携坐标侧车**：`recursive_tower.rs:95-101` 明文不变量「`sub_moves[i].rmove == rmove.subs[i]`，同序同长；结构层用 rmove，坐标层用 sub_moves」。所以纵向下钻消费的就是嵌套代数结构（μF）的坐标投影——**不是**绕开嵌套改查塔。

### 4.2 上行 rung 链：π 门确实用塔索引 【事实】

`build_nest_certificate` 的 rung 定位是 `tower[k]` 上的 `partition_point` **点包含**查找（`econ_positive.rs:1038-1039`）——塔（`Vec<Vec<LeveledMove>>`）是横向表示，这里被拿来做纵向链的逐级定位。「用横向的表示做纵向的事」对这段属实。

### 4.3 但已有实测：两种口径 bit-exact 等价 【事实】

- 仓内存有 cfg(test) 对照实现 `build_nest_certificate_bottomup`（`econ_positive.rs:1142-1220`）：逐级**子区间包含** `J_{k-1}⊆I(c)` + `Sel_Θ` 选最优 + child 逐级加宽——PDF §二「最严格 bottom-up」的真嵌套口径。
- 对拍探针 `acc_bottomup_nest_parity_probe`（`econ_positive.rs:6198`）在 BTC 三窗实测**差异 0**（MEMORY：bottom-up ≡ point-contain descend，NO-SHIP 判定）。
- 含义：塔在实测窗内是严格 refinement，点包含与嵌套包含选出同一条链。**用塔索引是表示上的绕行，但在已测数据上无观测差**。**【未能判定】** 非 BTC/更深塔下二者是否恒等价——对拍只覆盖了三窗。

### 4.4 严格装配臂本来就是真 ⊆ DFS 【事实】

`nest.rs:959-1027` `extend_typed_upward`：逐级取事件、`is_sub(child, parent)` 过滤、递归上钻、失败回溯（push/pop）——教科书式嵌套关系上的 DFS。admission 链门（`admission.rs:950-1041` `chain_lookup`）再把 [L0, 链顶] 每级独立证书的 `n_delta` 逐级闭合。即**纵向的真递归承载物在仓里存在且有两处**（descend + typed DFS）；π 门的 rung 循环是三者中唯一用塔索引的。

### 4.5 对 #737 后果声明的事实陈列（不改判） 【事实】

#737 定案 78% = missing_cert。按 `chain_lookup`（`admission.rs:1005-1026`），missing_cert = 该级 `(event_level, price, anchor)` 键下**索引里没有证书**；而证书入索引的前提是该级基例 `divergence_confirmed`（`nest.rs:905`）。即链臂的逐级判定力全部压在「每级各自的基例背驰确认」上——与「下钻从未发生」是两个不同机制。本票查得的 rung 恒真/免门空洞属于 **π 门臂**；链臂的 78% 归因是否需要改判，超出本票，留 #799/另行裁定。

---

## 5. 第 5 问：横向已验证形态能否照搬到纵向

横向形态（#800 确证）：判定作参数、层是参数不是分支、测试锁。

### 5.1 纵向现状逐项对表 【事实】

| 形态要素 | 纵向已有 | 纵向违例 |
|---|---|---|
| 判定作参数 | `div_cand(DivCandInput{context, target_idx, hist, delta})` 层无关，上钻（`cand_delta_type1_extreme`）与下钻（`descend_type1_anchor_depth`）复用同一判据；`assemble_typed_certificate` 的 `terminal_of` 已是闭包参数 | `cand_delta` 按类型分支且 Type2/3 臂是常量 `true`（`econ_positive.rs:961`）——判定既不作参数也不存在 |
| 层是参数不是分支 | `descend_type1_anchor_depth` 无级别分支，结构见底（`sub_moves.is_empty()`）；`build_nest_certificate` 的 `lvl` 是参数 | 两处 `lvl == 0 \|\|` 免门（`econ_positive.rs:926,945`）——级别数值分支 |
| 测试锁 | `descend_anchor_recurses_below_one_level`（:3042）、`cand_delta_rung_dispatch`（:2666）、`cand_delta_type23_base_gate_level0_and_small_to_big`（:2616）、`acc_bottomup_nest_parity_probe`（:6198）、`nest_containment_locates_rung_where_endpoint_equality_fails`（:1886） | — |

### 5.2 可行性判断 【推断】

**形态可搬，且大半已在**。纵向的核心递归件（descend、typed DFS、`n_delta_rec`）本来就是层参数化的结构递归；违例收敛在**三个代码锚点**：

1. `econ_positive.rs:961`（`Type2 | Type3 => true`）；
2. `econ_positive.rs:926`（Type2 `lvl==0` 免门）；
3. `econ_positive.rs:945`（Type3 `lvl==0` 免门）。

最小改造面即此三处＋一个前置裁定：rung 级 `Cand^δ_k` 的语义（P2 定义式 / 673-fix 恒真 / #97 D1 结构宽候选，三道裁决现共存，§2.4）必须先由 #799 收敛成一道，否则任何改动都是在未裁定的语义上施工。`lvl==0` 免门的消解还依赖「塔不从笔递归」这个构造选择要不要改（#520 订正在案）——那是表示层决策，同属 #799 范围。本票到此为止，不给实现方案。

---

## 6. 090 声明

- 未能判定项已逐处标注：§2.4（rung 谓词语义归属）、§4.3（非 BTC 数据下两口径等价性）。
- 本票未重跑任何回测；引用的 95.36%/4.64%、Pass 1/52、bit-exact 差异 0 均为仓内留痕记录（代码注释、#796 报告、MEMORY），出处已锚。
- 编排者双向递归假说的裁定结果：**强形式推翻，弱形式对 π 门 Type2/3 rung 链成立**（§0 总判）——既非全盘证实也非全盘推翻，按证据切分。
