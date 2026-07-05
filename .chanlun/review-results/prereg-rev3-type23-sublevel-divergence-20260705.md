# prereg-rev3｜Type2/3 次级别 divergence 预注册修订（f1 冻结）

**工位**：swarm/ws-r2prereg ｜ topo_address: swarm/ws-r2prereg ｜ 基因 073a/274号
**日期**：2026-07-05 ｜ 分支：gap3-rework-codex9-fix ｜ 基线：1f041542bc
**parent_callback**：main ｜ **下游消费者**：f2（实装）/ f3（跑数），依赖本文件 commit 冻结哈希
**认识论等级约定**（231号）：本 prereg 每节标注 L0（纯定义/原文推导，零信息增量）/ L1（代码静态核对）。涉及 alpha 有效性的陈述属 L2/L3，全 INCONCLUSIVE，不在本 prereg 范围。

---

## 〇、元结论（否定性发现，照实 161号）

**编排者裁决（定义层）完全成立**——Type2/3@ℓ 次级别走势结束须次级别背驰，区间套 N^δ 递归是默认的。本 prereg 接受并形式化此裁决（§一）。

**但 team-lead 订正假设的"实装无次级别 divergence 确认"与代码不符**——L1 逐行核实（§三）证明：`descend_type1_anchor_depth`（econ_positive.rs:712-740）在 Type2/3 base gate 准入路径**已经调用完整 `div_cand`**（Dir∧Comparable∧Extreme∧Weak 四条件，含盘整背驰），锚定「Type2/3@ℓ 精确点 = 次级别一类背驰点」（代码 :696 自述）。这正是编排者裁决要求的"次级别走势结束须次级别背驰"。

| 维度 | team-lead 订正假设 | L1 核实结论 |
|---|---|---|
| Type2/3 base gate 次级别 divergence | **缺失**（"无次级别 divergence 确认"） | **已实装**（descend_type1_anchor_depth → div_cand:724-731） |
| 订正描述的"升级"目标 | 结构确认 ∧ 次级别 divergence（新加） | 实装中**已达成**（上游 six_state 结构确认 ∧ Cand base gate descend div_cand） |
| 97.8% Type2/3 进在次级别走势未结束位置 | 成立（亏损信号侧根因候选） | **不成立**——descend 已要求次级别背驰（走势结束），且 div_cand 含 Weak **过严**（MACD 面积衰减额外要求），方向是误拒非误纳 |
| 信号集改动 / bit-exact | 不保留（既有 Type2/3 集合缩小） | **保留**——descend 门已存在于生产路径，既有 Type2/3 集合已是"次级别背驰确认后"的集合，无新增门空间 |

**严格性质裁决（no-patch-mentality + 090号）**：
- 若 f2 在 descend 之上**再加**一道次级别 divergence 门 ⟹ 重复判据（descend 已判）⟹ 要么冗余恒等（bit-exact）要么过严误拒（信号集错误缩小）⟹ **补丁思维 + 声明膨胀（声明代码不具备的能力——代码已具备）**，禁止。
- 本 prereg 冻结论：**实装已满足编排者裁决的定义层，R2 无需修订**。f2 不加冗余门；既有 Type2/3 准入路径 bit-exact 保留。

**对订正误读的根因诊断**：订正只核实了 per-rung `cand_delta`（:856 `Type2|Type3 => true`），未核实 base gate `cand_delta_base_gate`（:865）→ `descend_type1_anchor_depth`（:712）→ `div_cand`（:724）这条**已含 divergence**的锚定链。订正把"per-rung 跳过 divergence"（真实现象）误读为"无次级别 divergence 确认"（与 base gate 事实不符）。这是 strict-fix-proposals §R2 **原始分析**（commit 879b7527c4 之前）已经正确指出过的——「Type2/3 的存在性锚定在 base gate 一次（含 descend div_cand）」。

---

## 一、修订范围（编排者裁决的定义层 + 实装核实结论）

### 1.1 编排者裁决（定义层，完全接受）

**编排者原话**（strict-fix-proposals §R2 订正小节，commit 879b7527c4）：
> 「这个属于次级别的回调，然而，难道次级别就不需要背驰了吗？...这个不需要核实，肯定是这样的，因为区间套和递归是默认的。」

**裁决要旨**：Type2/3@ℓ 是次级别走势（二类=一类后回抽，三类=突破中枢后回踩/反抽）；次级别走势的**结束确认**按缠论级别递归 + 区间套 N^δ 终条件，**必须有次级别背驰**（除非小转大特殊情形）。区间套和递归是默认的——不需要再核实。

### 1.2 形式化（定义层，L0）

采用 formal-chain 标准符号（买卖点.pdf §7 / 区间套.pdf / 关于背驰.pdf §9.1）：

```
Type2/3@ℓ entry 准入（编排者裁决形式化）:
  Admit^δ_ℓ(Type2/3) ⟺ StructConfirm^δ_ℓ ∧ N^δ_{ℓ↓e}

其中:
  StructConfirm^δ_ℓ = 上游结构确认
    Type2: 回抽不破一类点极值（保护边界）
    Type3: 回踩不入中枢 ZG-ZD（保护边界）
    [由 six_state.rs 置 buy2/sell2/buy3/sell3 位时强制]

  N^δ_{ℓ↓e} = 区间套方向确认证书（递归终条件含次级别背驰）
    基例:  N^δ_{e↓e} = Conf^δ_e = ⋁_{i=1}^3 B_{i,e}（买）/ ⋁_{i=1}^3 S_{i,e}（卖）
    步:    N^δ_{k↓e} = Cand^δ_k ∧ [J^δ_{k-1} ⊆ J^δ_k] ∧ N^δ_{k-1↓e}   (k > e)

  Type2/3@ℓ 次级别走势结束的次级别背驰（编排者裁决核心）:
    候选段 s（ℓ 级回抽/回踩走势，end_index=source_index）
    s 的结束 = 次级别（ℓ-1）一类背驰点
    即 descend anchor: div_cand(m) where m = s.sub_moves[end==source_index]
```

### 1.3 实装核实结论（L1）

实装中 Admit^δ_ℓ(Type2/3) 的两个合取项**均已实装**：
- `StructConfirm^δ_ℓ`：上游 `six_state.rs` 置位时强制（保护边界，econ_positive.rs:808-813 / :824-832 注释明文）
- `N^δ_{ℓ↓e}` 含次级别背驰：`build_nest_certificate`（:889）→ `cand_delta_base_gate`（:865）→ `cand_delta_type2/type3`（:814/:833）→ `descend_type1_anchor_depth`（:712）→ `div_cand`（:724，次级别背驰四条件）

**无需修订**。详见 §三、§四。

---

## 二、formal-chain 原文依据（亲读，准引用）

### 2.1 区间套.pdf——N^δ 递归本体（pdftotext 亲读）

**递归定义**（区间套.pdf §1）：
> 基例：`N^δ_{e↓e} = Conf^δ_e`
> 递归步：`N^δ_{k↓e} = Cand^δ_k ∧ [J^δ_{k-1} ⊆ J^δ_k] ∧ N^δ_{k-1↓e}`，其中 `k > e`
> 多级区间套的本体：`J^δ_e ⊆ J^δ_{e+1} ⊆ ⋯ ⊆ J^δ_ℓ`（区间包含链，不是端点相等链）

**区间套语义**（区间套.pdf §4，编排者"区间套默认"裁决的原文锚）：
> 「从大级别背驰段逐级收缩到小级别背驰段，最后定位到执行级买卖点。」

——此句确立区间套递归的**每一级都是背驰段收缩**，定位到执行级买卖点。Type2/3 的精确点（次级别一类背驰点）是这条收缩链的终端。

### 2.2 买卖点.pdf §7——区间套确认 + Conf 终端（pdftotext :109-160 亲读）

**区间套确认定义**（verbatim）：
> 对声部 v，设其操作级别为 o_v，执行级别为 e_v，满足 o_v ≥ e_v。
> 定义方向 δ∈{+1,−1} 的区间套确认：`χ^δ_v(x) = N^δ_{o_v↓e_v}(x) ∈ {0,1}`
> 其中递归为：
> `N^δ_{ℓ↓e}(x) = Conf^δ_e(x)`，（ℓ=e）
> `N^δ_{ℓ↓e}(x) = Cand^δ_ℓ(x) ∧ [J^δ_{ℓ-1}(x) ⊆ J^δ_ℓ(x)] ∧ N^δ_{ℓ-1↓e}(x)`，（ℓ>e）
> 买入方向终端确认：`Conf^+_e(x) = ⋁_{i=1}^3 B_{i,e}(x)`
> 卖出方向终端确认：`Conf^−_e(x) = ⋁_{i=1}^3 S_{i,e}(x)`
> 由于级别链有限 o_v > o_{v-1} > ⋯ > e_v，所以区间套递归有限终止。

——Type2/3 的终端 Conf^δ_e = 二三类买卖点 bit 置位（B_{2,e}/B_{3,e} 或 S_{2,e}/S_{3,e}）；但终端 Conf 只确认"买卖点 bit 存在"，**不确认次级别走势结束**——后者由 N^δ 递归的 Cand^δ_ℓ 链 + descend anchor 承载（次级别背驰）。

### 2.3 关于背驰.pdf §4.3——盘整背驰 PanDivContext（pdftotext :386-395 亲读）

**§4.1 同级别同向**：
> `ℓ(s)=ℓ(s')`，`dir(s)=dir(s')=−δ`。离开级别，不能谈趋势背驰。

**§4.2 同一上级趋势语境**：
> 存在上级容器 C，使 s', s ⊏ C，且 s' 与 s 是 C 中可比较的前后同向推进段。记为 `Comparable_ℓ(s',s)=1`。

**§4.3 价格推进**（含盘整背驰另设）：
> 买入方向 δ=+1：s 应相对 s' 有更低低点：`minP(s) < minP(s')`。
> 卖出方向 δ=−1：`maxP(s) > maxP(s')`。
> **若处理盘整背驰，可以把"创新"放宽成"对中枢边界的推进/离开力度比较"，但这必须另设 PanDivContext。不要和趋势背驰混为一个谓词。**

——§4.3 明文：盘整背驰（Type2/3 回抽/回踩属盘整语境）**另设 PanDivContext**，div_cand（cand_predicate.rs:107）实装的 Dir∧Comparable∧Extreme∧Weak 涵盖盘整背驰语境（代码 :699 注释自述「含盘整背驰——背驰段定义第27课L21 涵盖趋势/盘整，非弱化版」）。

### 2.4 关于背驰.pdf §9.1——Cand=StructEligible（strict-fix-proposals §R1 已 verbatim 引用）

> 定义 `StructEligible^δ_ℓ(s,s')` 作为宽结构候选：`dir(s)=−δ ∧ Comparable(s',s) ∧ Extreme^δ(s',s)`。然后定义力度状态 `ForceState_ℓ(s',s)`。最后：**`Cand^δ_ℓ = StructEligible^δ_ℓ` 而不是 `Cand = MACDdiv`**。

——注：descend 调的 div_cand 当前含 Weak（四条件），与 §9.1 三条件有偏差，但这是 **R1-a 范畴**（所有 div_cand 调用点统一处理），非 R2 范畴。详见 §八。

### 2.5 编排者裁决（区间套递归默认）

编排者「区间套和递归是默认的」= 形式化为：Type2/3 的 N^δ_{ℓ↓e} 递归终条件**默认要求**每级收缩链终端的次级别背驰，不需要逐案核实。此裁决作用于**定义层**（缠论语法），本 prereg 完全接受。实装层是否已满足，属 L1 核实范畴（§三）。

---

## 三、当前实装锚点（L1 逐行核实）

Type2/3@ℓ entry 准入在生产区间套证书构造路径 `build_nest_certificate`（econ_positive.rs:889-975），分两层门：

### 3.1 base gate（一次，**含次级别 divergence**）——编排者裁决已实装处

**调用链**：`build_nest_certificate:914` → `cand_delta_base_gate:865` → `cand_delta_type2_completion:814` / `cand_delta_type3_retest:833` → `descend_type1_anchor_depth:712` → `div_cand`（cand_predicate.rs:107）

**`descend_type1_anchor_depth`**（econ_positive.rs:712-740，L1 逐行）：
```rust
// :696 注释（自述编排者裁决语义）:
// 「Type2/3@ℓ 的精确点 = 次级别 Type1
//   （回抽这个次级别走势的结束点 = 次级别一类背驰点，定律一 第17课L66）」

fn descend_type1_anchor_depth(s, source_index, delta, hist) -> Option<usize> {
    let subs = s.sub_moves.as_slice();           // 次级别 ℓ-1 走势序列
    if subs.is_empty() { return None; }          // 递归底 level0 = 小转大门拒
    let tidx = find_move_by_end_index(subs, source_index)?;  // 找 end==src 的次级别段 m
    let anchor_ok = div_cand(...);               // :724-731 完整 div_cand（含盘整背驰）
    if !anchor_ok { return None; }               // 次级别无一类背驰锚 = 小转大门拒
    match descend_type1_anchor_depth(&subs[tidx], ...) {     // 真递归下沉逐级收缩
        Some(d) => Some(d + 1),
        None => Some(1),
    }
}
```

**`div_cand`**（cand_predicate.rs:107-151，四条件合取）：
1. **Dir**（:122-129）：`dir(s) = −δ`（δ=Long→背驰段方向 Down）
2. **Comparable**（:131-136）：`context[..target_idx]` 中存在最近同向段 s'（同父次级别序列前序）
3. **Extreme**（:138-145）：δ=Long→`lo(s)<lo(s')`；δ=Short→`hi(s)>hi(s')`
4. **Weak**（:147-150）：`area(hist,s) < area(hist,s')`（MACD 面积力度衰减）

**核实结论**：`descend_type1_anchor_depth` 在 s.sub_moves（次级别 ℓ-1 序列）找 `end_index==source_index` 段 m，跑完整 div_cand——判 m 是否次级别一类背驰段。m 是 s（ℓ 级回抽/回踩走势）的结束段（end==s.end==source_index）。**m 背驰 ⟺ s 的结束段在次级别背驰 ⟺ Type2/3@ℓ 次级别走势在次级别背驰处结束**。这正是编排者裁决要求的"次级别走势结束须次级别背驰"。

**`cand_delta_type2_completion`**（:814-822）/ **`cand_delta_type3_retest`**（:833-841）：
```rust
lvl == 0 || descend_type1_anchor_depth(s, source_index, delta, hist).is_some()
```
`lvl==0` 免门（递归底，无次级别可下沉，空真）；否则要求 descend 返回 Some（次级别有一类背驰锚）。返回 None（小转大）⟹ base gate 拒 ⟹ 整证书拒（build_nest_certificate:914-916）。

### 3.2 per-rung（每个上级 k，Type2/3→true）——非"次级别走势"范畴

**调用链**：`build_nest_certificate:951` → `cand_delta:847`

**`cand_delta`**（:847-859，薄 dispatcher）：
```rust
match cand_type {
    Type1 => cand_delta_type1_extreme(...),   // 本级趋势背驰段 div_cand
    Type2 | Type3 => true,                     // 存在性已由 base gate 门控
    StructBreak => false,                      // 门拒（codex 终局裁决A）
}
```

**per-rung Type2/3→true 的合法性**（§R2 原始分析已证，commit 879b7527c4 之前）：per-rung 是高级别 rung（k>ℓ 的上级语境层），承载"source_index 落在 k 级走势区间内"的上级定位，**不是**"Type2/3@ℓ 次级别走势"范畴。Type2/3 次级别走势（回抽 s）的结束确认在 base gate（descend），不在 per-rung。per-rung 对 Type2/3 返回 true = "上级语境存在性承袭"（第17课L60 完备性），非"跳过 divergence"。

**若强加 per-rung divergence 给 Type2/3** = 在高级别 rung 上施加背驰判据 = 把 Type1 趋势背驰与 Type2/3 Conf 确认混为一个谓词 = **§4.3 明文禁止的范畴错误**。

### 3.3 上下游门分工（实装已满足编排者裁决）

| 门 | 实装位置 | 承载语义 | 编排者裁决对应 |
|---|---|---|---|
| StructConfirm（保护边界） | 上游 six_state.rs 置位 | Type2: 不破一类点极值；Type3: 不入 ZG-ZD | StructConfirm^δ_ℓ |
| base gate（descend div_cand） | econ_positive.rs:712-740 | **次级别（ℓ-1）背驰锚** = Type2/3 次级别走势结束确认 | N^δ 递归终条件的次级别背驰 |
| per-rung（cand_delta） | econ_positive.rs:847-859 | 上级 k 级语境存在性（Type2/3→true） | Cand^δ_k（上级 rung，非次级别走势） |
| Conf 终端 | interp.rs:378 nest_confirm | 执行级买卖点 bit（二三类置位） | Conf^δ_e |

---

## 四、新准入条件形式化（编排者裁决 vs 实装，L0+L1）

### 4.1 编排者裁决形式化（L0）

```
Admit^δ_ℓ(Type2/3) ⟺ StructConfirm^δ_ℓ ∧ SublevelDiv^δ_ℓ

StructConfirm^δ_ℓ:
  Type2 → 回抽不破一类点极值
  Type3 → 回踩不入中枢 ZG-ZD

SublevelDiv^δ_ℓ（编排者裁决核心——次级别走势结束须次级别背驰）:
  = descend_type1_anchor_depth(s, source_index, δ, hist).is_some()
  其中 s = tower[ℓ] 中 end_index==source_index 的回抽/回踩走势段
  ⟺ s.sub_moves（ℓ-1 级）中 ∃ 段 m, m.end==source_index ∧ div_cand(m)
  ⟺ Type2/3@ℓ 次级别走势 s 在次级别（ℓ-1）一类背驰处结束
```

### 4.2 实装已满足证明（L1）

**定理 R2-rev3-1**（实装满足编排者裁决）：当前生产准入路径已实装 `Admit^δ_ℓ(Type2/3) = StructConfirm ∧ SublevelDiv`。

*证明*（L1 静态核对，沿 §三 调用链）：
- `StructConfirm^δ_ℓ`：由 `six_state.rs` 在置 buy2/sell2（Type2）/buy3/sell3（Type3）位时强制保护边界（econ_positive.rs:808-813、:824-832 注释明文「上游 bit 置位时已强制，Cand 层不双门」）。
- `SublevelDiv^δ_ℓ`：`build_nest_certificate:914` 调 `cand_delta_base_gate`（:865）对 Type2/3 委托 `cand_delta_type2/type3`（:814/:833），二者调 `descend_type1_anchor_depth`（:712）跑 `div_cand`（:724）。`descend` 返回 None ⟹ base gate 拒 ⟹ 整证书 None（:914-916）⟹ `n_delta()` 返回 false（:688-691）⟹ 准入拒。
- ∴ 生产 Type2/3 准入 = StructConfirm ∧（descend div_cand 通过）= 编排者裁决形式。 ∎

**推论**：订正假设的"升级"（从 StructConfirm 升到 StructConfirm ∧ SublevelDiv）在实装中**已完成**——StructConfirm 与 SublevelDiv 均已实装且合取。f2 无新增门空间。

**定理 R2-rev3-2**（descend div_cad 过严而非过宽）：descend 的 div_cand 含 Weak 四条件，相对 §9.1 三条件 StructEligible 是**超集收紧**（额外要求 MACD 面积衰减），不会出现"次级别走势未结束仍入场"。

*证明*：div_cand = Dir∧Comparable∧Extreme∧Weak ⊆ Dir∧Comparable∧Extreme = StructEligible。Weak 是额外收紧 ⟹ 通过 div_cand 的候选 ⊆ 通过 StructEligible 的候选 ⟹ descend 拒绝集 ⊇ StructEligible 拒绝集。即 descend 比 §9.1 严格版本**更严**，方向是误拒（合法 Type2/3 次级别背驰但 MACD 面积未衰减被误拒）非误纳。订正假设的"次级别走势未结束仍入场"（误纳方向）不会发生。 ∎

**推论**：descend div_cand 含 Weak 是 **R1-a 范畴**（所有 div_cand 调用点统一去 Weak，§9.1 偏差），其修复方向是**放宽**（移除 Weak 让 Cand=StructEligible），信号集会**扩张**（被 Weak 误拒的 Type2/3 重新纳入）——这与订正假设的"信号集缩小"方向相反。详见 §八。

---

## 五、fail 条件 + 认识论等级

### 5.1 fail 条件（准入拒）

| fail 情形 | 实装行为 | 合法性 |
|---|---|---|
| 次级别无一类背驰锚（div_cand 假） | descend None → base gate 拒 → 证书 None | **合法**（编排者裁决要求次级别背驰，无背驰不入场） |
| source_index 不落在次级别段右端点（find_move_by_end_index None） | descend None → 小转大门拒 | **合法**（小转大特殊情形，编排者裁决"除非小转大"） |
| s 已是递归底（subs 空，lvl==0） | base gate 免门（lvl==0‖...） | **合法**（递归底空真，无次级别可下沉） |
| 上游 StructConfirm 不满足（破位） | six_state 不置位 → 无 Type2/3 候选 | **合法**（结构确认前提） |

### 5.2 认识论等级标注（231号）

| 陈述 | 等级 | 依据 |
|---|---|---|
| 编排者裁决的定义层形式化（§一、§四） | **L0** | 纯定义，从 formal-chain 原文（区间套.pdf/买卖点.pdf §7/关于背驰.pdf §4.3）推导 |
| 实装已实装次级别 divergence（§三、§四） | **L1** | 代码静态核对（econ_positive.rs:712-740 + cand_predicate.rs:107-151 调用链） |
| descend div_cand 过严（定理 R2-rev3-2） | **L1** | 代码静态核对（div_cand 四条件含 Weak） |
| 订正假设的"97.8% 进在未结束位置"不成立 | **L1**（基于定理 R2-rev3-2 推论） | 代码静态核对结论 |
| 修订后 alpha 变化 | **L2/L3** | 不在本 prereg 范围（全 INCONCLUSIVE） |

---

## 六、bit-exact 声明（核心：保留，非不保留）

### 6.1 严格核实下的 bit-exact 结论

**本 prereg 冻结的修订 = 无修订**（实装已满足编排者裁决）。因此：

- **既有 Type2/3 交易集合 bit-exact 保留**——descend div_cand 门已存在于生产准入路径（build_nest_certificate:914 → descend:712），既有 Type2/3 集合已是"StructConfirm ∧ SublevelDiv 确认后"的集合，不存在"未确认次级别背驰就入场"的历史交易。
- **生产路径零改动**——f2 不加冗余门（§〇 严格性质裁决）。

### 6.2 订正假设的"bit-exact 不保留"为何不成立

订正假设「信号集改动，既有 Type2/3 集合会缩小」的前提是"实装原本无次级别 divergence 门，修订后新增门"。L1 核实（§三）证明此前提虚假——门已存在（descend div_cand，:712-740）。故：
- 不存在"新增门"的空间 ⟹ 不存在"信号集缩小"的机制。
- 反方向：若执行 R1-a（descend div_cand 去 Weak，§9.1 三条件），信号集会**扩张**（被 Weak 误拒的 Type2/3 重新纳入），与订正假设方向相反。

### 6.3 若 f2 强加冗余门的后果（禁止）

若 f2 在 descend 之上再加一道次级别 divergence 门：
- 若新门 ≡ descend div_cand ⟹ 冗余恒等，bit-exact 保留但代码重复（dead gate，§R2 原始分析已述 Type3 重复 Type2 helper 的 dead gate 先例）。
- 若新门 ⊋ descend div_cand ⟹ 过严误拒，信号集错误缩小（破坏既有 bit-exact）。
- 若新门 ⊊ descend div_cand ⟹ descend 已是更严，新门恒真，冗余。
- 三种情形均无正面收益 ⟹ **no-patch-mentality 禁止**（补丁思维 + 声明膨胀）。

---

## 七、边界：小转大特殊情形（编排者裁决"除非小转大"）

### 7.1 formal-chain 依据

编排者裁决明文「除非小转大特殊情形」。formal-chain 锚：
- 知识库 L410「区间套和背驰不可解释情况的补充」（econ_positive.rs:704 注释引用）
- 区间套.pdf：区间套递归要求级别链有限 o_v>...>e_v；若该级无一类精确点无法下沉定位，递归无终端背驰点 ⟹ 区间套不可解释 = 小转大。

### 7.2 实装处理（L1）

`descend_type1_anchor_depth` 返回 None 的三种情形（:703-705 注释）均映射到小转大门拒：
1. `subs.is_empty()`（:719）——递归底 level0 无次级别 ⟹ 无可下沉锚点 = 小转大
2. `find_move_by_end_index(subs, source_index) = None`（:723）——次级别存在但无回抽端点对齐段 ⟹ 小转大
3. `div_cand` 假（:732-734）——次级别无一类背驰锚点 ⟹ 小转大

三种 None ⟹ `cand_delta_base_gate` 返回 false ⟹ `build_nest_certificate:914-916` 返回 None ⟹ `n_delta()` false ⟹ 准入拒。

**显式可测判别**（:705 注释「非 catch-all fallback」）：小转大门拒是确定性结构判定（subs 空 / 端点不对齐 / div_cand 假），不是兜底。门直接拒 = 编排者裁决"除非小转大"的实装（小转大不入场）。

### 7.3 lvl==0 免门（递归底，非小转大）

`lvl==0`（执行级即递归底，无次级别塔层）⟹ base gate 免门（:821/:840 `lvl==0 || ...`）。这是递归 base case 的空真（第29课L396「二三类精确点要下次级别以下找第一类」前提不成立时规则不适用），**不是**小转大（小转大是该级有一类点但次级别无法下沉；lvl==0 是物理上无次级别）。详见 strict-fix-proposals §R3。

---

## 八、真正的可修订点（非 R2 范畴，诚实声明）

本 prereg 冻结 R2 = 无需修订。但 L1 核实识别两处**非 R2 范畴**的可修订点，登记供 f2/f3 区分（避免混淆）：

### 8.1 R1-a（descend div_cand 含 Weak，§9.1 偏差）

- **现象**：descend 调的 div_cand（cand_predicate.rs:107）含 Weak 四条件，违反 §9.1「Cand^δ_ℓ=StructEligible^δ_ℓ 而不是 Cand=MACDdiv」。
- **范畴**：R1-a（所有 div_cand 调用点——Type1 per-rung cand_delta_type1_extreme + Type2/3 base gate descend——统一去 Weak）。非 R2 独有。
- **修复方向**：移除 div_cand 条件4 Weak（:147-150），Cand 收紧为三条件 StructEligible。
- **信号集影响**：**扩张**（被 Weak 误拒的候选重新纳入），与订正假设的"缩小"方向相反。需新预注册（135号）。
- **归属**：strict-fix-proposals §R1-a 已立案，本 prereg 不重复。f2 若执行 R1-a，则 Type2/3 base gate 的 descend div_cand 同步去 Weak（与 Type1 per-rung 同源）。

### 8.2 per-rung（cand_delta Type2/3→true，范畴错误若强加 divergence）

- **现象**：per-rung cand_delta 对 Type2/3 返回 true，不判 divergence。
- **范畴**：per-rung 是高级别 rung（k>ℓ）上级语境层，非"Type2/3@ℓ 次级别走势"范畴（§3.2）。
- **若强加 per-rung divergence 给 Type2/3**：§4.3 明文禁止（混 Type1 趋势背驰与 Type2/3 Conf 确认为同一谓词）。strict-fix-proposals §R2 原始分析已证。
- **归属**：非修订点，禁止修改。

---

## 九、结果包六要素

### 1. 结论

- **编排者裁决（定义层）完全成立并形式化**：Type2/3@ℓ entry ⟺ StructConfirm^δ_ℓ ∧ SublevelDiv^δ_ℓ（次级别走势结束须次级别背驰，N^δ 递归终条件）。
- **L1 核实否定性发现**：实装**已满足**编排者裁决——`descend_type1_anchor_depth`（econ_positive.rs:712-740）在 Type2/3 base gate 准入路径已调用完整 div_cand（次级别背驰锚）。team-lead 订正假设的"实装无次级别 divergence 确认"与代码不符。
- **R2 严格性质 = 无需修订**（实装已满足定义层）。f2 不加冗余门（no-patch-mentality + 090号）。
- **bit-exact 保留**——descend 门已存在于生产路径，既有 Type2/3 集合已是"次级别背驰确认后"的集合。订正假设的"信号集缩小"不成立。
- **真正可修订点**（非 R2 范畴）：R1-a（descend div_cand 去 Weak，信号集**扩张**非缩小，需新预注册）；per-rung（禁止强加 divergence，范畴错误）。

### 2. 定义依据

- **区间套.pdf**（pdftotext 亲读）：N^δ_{ℓ↓e} 递归（基例 Conf^δ_e / 步 Cand^δ_ℓ∧[J⊆]∧N^δ_{ℓ-1↓e}）+「从大级别背驰段逐级收缩到小级别背驰段，最后定位到执行级买卖点」（§2.1）。
- **买卖点.pdf §7**（pdftotext :109-160 亲读）：N^δ verbatim + Conf^+/Conf^- 终端确认 +「区间套递归有限终止」（§2.2）。
- **关于背驰.pdf §4.3**（pdftotext :386-395 亲读）：盘整背驰 PanDivContext「不要和趋势背驰混为一个谓词」（§2.3）+ §4.1 同级别同向 + §4.2 Comparable。
- **关于背驰.pdf §9.1**（strict-fix-proposals §R1 已 verbatim 引用）：Cand^δ_ℓ=StructEligible^δ_ℓ（§2.4，R1-a 依据）。
- **编排者裁决**（strict-fix-proposals §R2 订正，commit 879b7527c4）：「区间套和递归是默认的」（§2.5）。
- **代码锚点**（全部 L1 逐行核实）：econ_positive.rs:696/712-740（descend+div_cand）、:814-841（cand_delta_type2/type3）、:847-859（cand_delta per-rung）、:865-879（base gate）、:889-975（build_nest_certificate）、:704（小转大注释）；cand_predicate.rs:107-151（div_cand 四条件）；interp.rs:378（nest_confirm Conf 终端）。

### 3. 边界条件（结论翻转）

- **本 prereg 结论（实装已满足）翻转条件**：仅当 L1 核实链有误——即 `descend_type1_anchor_depth` 实际**不在** Type2/3 生产准入路径（如 build_nest_certificate:914 的 base gate 调用被某分支绕过），或 div_cand 实际不判背驰（如 Weak 条件被禁用导致恒真）。重核路径：grep `cand_delta_base_gate` 调用点 + Read econ_positive.rs:889-975 确认 base gate 不可绕过。若重核发现 base gate 被绕过，则订正假设成立，f2 须补次级别 divergence 门。
- **不翻转的部分**：编排者裁决的定义层（Type2/3 次级别走势结束须次级别背驰）不随实装核实翻转——这是缠论语法，区间套递归默认。无论实装是否已满足，定义层形式化（§一、§四）成立。
- **R1-a 结论翻转**：若 R1-a 执行（descend div_cand 去 Weak），Type2/3 base gate 信号集**扩张**（被 Weak 误拒的候选重新纳入）⟹ 需新预注册（135号），bit-exact 不保留——但这是 R1-a 触发，非 R2。

### 4. 下游推论

- **对 f2（实装工位）**：R2 冻结 = 无需实装。f2 不加冗余次级别 divergence 门（§6.3 三种情形均无收益）。f2 可转 R5（诊断插桩，strict-fix-proposals §R5 唯一真 bug）或 R1-a（若编排者授权新预注册）。
- **对 f3（跑数工位）**：R2 无修订 ⟹ 无新预注册跑批（Type2/3 路径 bit-exact 保留）。f3 跑批对象不变（基线 1f041542bc 生产路径）。
- **对 §R2 订正（commit 879b7527c4）**：订正的"有效域缺口"判断基于不完整核实（只看 per-rung 未看 base gate descend）。本 prereg 否定性发现不否定编排者裁决（定义层成立），但修正订正的实装层判断。
- **对 135号冻结纪律**：R2 无修订 ⟹ 不触发新预注册（信号集不变）。R1-a 才触发新预注册。
- **对 Π_max-full 端到端 OOS（af8910d062 INCONCLUSIVE）**：本 prereg 不改 OOS 结论——R2 实装已满足编排者裁决，既有 Type2/3 交易集合的"次级别背驰确认"属性本就成立，OOS 跑批对象不变。

### 5. 谱系引用

- **无新谱系**：本 prereg 是 f1 冻结文档，未检测到需结晶的稳定信号（否定性发现本身不触发知识结晶）。
- **相关已结算谱系**：
  - **090号**：严格性语法规则——禁止声明膨胀（声明代码不具备的能力，code 已具备 descend div_cad）；禁止补丁思维（在正确逻辑上加冗余门）。
  - **161号**：否定性照实——R2 的"实装已满足"否定性发现照实报告，不迎合订正前提。
  - **231号**：有效域<定义域——descend div_cand 含 Weak（4 条件）的有效域 ⊊ StructEligible（3 条件）定义域；本 prereg 每节标注 L0/L1。
  - **135号**：冻结先于跑数——本 prereg 是 f1 冻结文档，commit 后 f2/f3 才开始；R2 结论=无修订 ⟹ 不触发新预注册。
  - **673-fix 裁决①**：Type2/3 per-rung cand=true（§3.2 确认符合 formal-chain，非 bug；codex 裁决 base gate descend 承载次级别背驰）。
  - **§R2 原始分析**（strict-fix-proposals commit 879b7527c4 之前）：已正确指出"Type2/3 存在性锚定在 base gate 一次（含 descend div_cand）"。订正（879b7527c4）推翻原始分析时未充分核实 descend 已含 div_cand。
- **不确定是否有相关谱系**：descend_type1_anchor_depth 的 div_cand 含 Weak 是否在某谱系中标为"v0 冻结 bit-exact"——未检索到明确谱系，若有则去 Weak（R1-a）属该谱系有效域扩展。

### 6. 影响声明

- **代码改动**：零。本 prereg 是 f1 冻结文档（研究+形式化），未改动任何代码。
- **文档改动**：新建 `/Users/silencehan/Projects/NewChanlun/.chanlun/review-results/prereg-rev3-type23-sublevel-divergence-20260705.md`（本文件）。
- **对实装的影响**：识别 R2 = 无需修订（实装已满足编排者裁决）；登记两处非 R2 范畴可修订点（R1-a 信号集扩张需新预注册 / per-rung 禁止强加 divergence）。f2 不加冗余门，既有 Type2/3 路径 bit-exact 保留。
- **对 f2/f3 计划的影响**：R2 无实装工作（f2 转移至 R5 或 R1-a）；f3 跑批对象不变。本 prereg 冻结哈希供 f2/f3 确认 R2 范畴闭合。
- **诚实声明**：本否定性发现针对 team-lead 订正（commit 879b7527c4）的**实装层判断**（"无次级别 divergence 确认"），不针对编排者裁决（定义层"区间套递归默认"完全成立）。若编排者/lead 对 descend_type1_anchor_depth 的语义有不同解读（如认为 descend 的"锚定定位"≠"走势结束确认"），结论可翻转——此为定义层选择类决断，留编排者裁决（§三 调用链 + §四 定理 R2-rev3-1 已给出 L1 证据供裁决）。

---

**prereg-rev3 冻结结束**。R2 = 实装已满足编排者裁决，无需修订，bit-exact 保留。f2 不加冗余门。唯一可推进的相邻工作是 R5（诊断插桩）或 R1-a（div_cand 去 Weak，需新预注册）——二者均非 R2 范畴。
