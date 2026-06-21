# QQQ bar 23688 C 翻空触发条件诊断 — located 是真定位还是 degenerate 弱确认？

> 任务（接续 `qqq_root_short_no_costreduction_diagnosis.md`）：根空头在 bar 23688 翻空建仓，
> 后被强平。查 C 翻空的精确触发条件，以及 bar 23688 那个 type1 卖点是真正的 located 走势
> 完美点还是 raw 候选。**只诊断，不修复。**
>
> 认识论等级：**L2**（QQQ 单标的真实数据 1min，bar 23688 单点状态实测）。
> 复现：`UNN_DBG_QQQ=1 PYTHONPATH=src .venv/bin/python analysis/investigate_qqq_liq.py 2>&1 | grep C-FLIP-SHORT`
> 探针：`unified_necessity.rs` C-Long 翻空分支 `prove_chain` 之后（env `UNN_DBG_QQQ` 门控，bit-exact when off，11 单测全过）。

> **诚实声明**：本报告第一版（上一轮对话）的 bar 23688 翻空数据是错误地在未真正执行探针/重建的情况下产出的（工具调用被误写为文本）。下列数据是**真实重建 + 真实运行**后捕获的（`/tmp/qqq_flip_real.txt`，单行翻空），与第一版的捏造值有差异（如 located 链含 compress/confirm bar、top_extreme=305.09、证据层=Some(3)）。以本版为准。

---

## 1. 结论

**C 翻空在 bar 23688 触发，五个条件全满足；它消费的是一条形式合法的 located 链（不是裸 raw 候选），但这条 located 链是 degenerate 的——candidate 首现与确认发生在同一根 bar（compress_bar == confirm_bar == 23688），次级别证据仅是 move(L1) 层任一同侧 sell 信号（sub_evidence 弱判据）。用户问题4的判断成立：这不是"走势真正完美"，是"type1 背驰 + 次级别同侧信号"的弱确认。**

| 问题 | 答案 |
|------|------|
| **1. C 翻空触发需要什么条件？** | `sell_source = chain_source(located_sell)` 是 Some(s) ∧ `s ≥ root_ladder` ∧ `sig.sell1[s]`（type1 卖点）∧ `prove_chain` 通过（N5/N6 硬断言链完整）∧ `single_root` ∧ `root_ladder > FIRST_BSP_LADDER`（`unified_necessity.rs:831-834`）。**不是单看 sell_source≥root_ladder——还要 sell1[s] 的 type1 mask + 完整级联链 + 单根。** |
| **2. bar 23688 的 sell_source 是多少？从哪层级联？** | `sell_source=4`（recL2）。located_sell 链 = `[(层2,源4),(层3,源4),(层4,源4)]`——confirm_sell[4] 在 bar 23688 fire，`cascade_arm` 向下统一武装 located_sell[segment..=recL2]，三层 source_ladder 统一=4。**从 recL2(4) 自上而下级联到 segment(2)。** |
| **3. 是真 located 还是 raw 候选？** | **形式上是 located**（prove_chain 通过，located_sell[4] 在场，source/direction 一致），**不是裸 raw**。**但语义上是 degenerate located**：compress_bar=confirm_bar=23688（candidate 首现即确认，零展开窗口），次级别证据层=Some(3)（rec_sub_evidence 在 move(L1) 立即命中同侧 sell 证据）。 |
| **4. 弱确认不应翻空的判断？** | **成立。** located 门控形式存在但语义强度不足：`sig.sell1[s]`（type1 背驰）+ `sub_evidence`（次级别任一同侧 BSP/背驰/翻转沿）≠ 趋势终结。recL2 的一个 type1 sell 背驰可以只是上升中继的回调顶——QQQ 后续 304→608 翻倍证明这个"顶"是假顶。 |

---

## 2. 定义依据

### C 翻空触发条件（`unified_necessity.rs:830-856`，逐条）

```rust
Polarity::Long => {
    if let Some(s) = sell_source {                          // ① located 卖链非空
        if s >= root_ladder && sig.sell1.get(s) {           // ② s≥根级别 ③ s层有type1卖
            prove_chain(&self.located_sell, Side::Sell, s, bar, "C-flip/clear");  // ④ 链完整(panic守卫)
            if single_root && root_ladder > FIRST_BSP_LADDER {  // ⑤ 单根 ⑥ 根非基底
                // T14 长→空 in-place 翻转
```

- **① sell_source** = `chain_source(located_sell)` = 最高有 located 的层（155-184行）。located_sell[k] 仅由 `cascade_arm` 武装，`cascade_arm` 仅在 `confirm_sell[k]` fire 时调用 ⇒ **不是 raw candidate，是经 rec_sub_evidence 确认的 located 点**。
- **③ sig.sell1[s]** = type1 卖点 mask（背驰）。located≠type1：located 是"区间套定位"，type1 是"背驰走势完美"。C 要求**两者同时**在 s 层成立。
- **④ prove_chain**（317-351行）硬断言：s≥PENDING_LO、located[s] 在场且 source/dir 一致、compress≤confirm≤bar、[FIRST_BSP..=s] 全 located 且 source 统一=s。**违反即 panic** ⇒ bar 23688 翻空必然满足了完整级联链（否则崩溃）。

### located 确认的语义判据（`nested_fugue.rs:115-148`）

```rust
fn sub_evidence(sub, side, evs, devs, flip_edge) -> bool {
    if sub >= FIRST_BSP_LADDER {
        evs.iter().any(|e| e.class.side()==side) || devs.iter().any(|d| d.side()==side)  // 任一同侧BSP∨背驰
    } else {
        flip_edge == Some(want)  // a0/bi: 同向翻转沿
    }
}
fn rec_sub_evidence(start, side, ...) -> Option<usize> {
    (lo..=start).rev().find(|&j| sub_evidence(j, ...))  // a0..=start 逆序找第一个有证据的层
}
```

**这是 located 确认的全部语义**：从 s-1 层逆序下探到 a0，**任一层**出现**任一个**同侧 BSP 事件 ∨ 同侧背驰事件 ∨ 同向翻转沿，即算"区间套次级别确认"。**没有数量要求、没有走势结构完整性要求、没有中枢背驰要求。**

### bar 23688 实测（探针真实输出）

```
[C-FLIP-SHORT bar=23688] sell_source=4 root_ladder=4 sell1@4=true sell_any@4=true
  single_root=true top_extreme=305.09
  | located_sell链(层,源,压缩bar,确认bar)=[(2,4,23688,23688),(3,4,23688,23688),(4,4,23688,23688)]
  | 次级别证据层(s-1=3)=Some(3)
```

逐字段：
- `sell_source=4, root_ladder=4`：多头根经 root_emergent_ladder 涌现爬升到 recL2（4），同层出现卖链顶 ⇒ `s≥root_ladder` 边界相等命中。
- `sell1@4=true`：recL2 有 type1 卖点背驰。
- `top_extreme=305.09`：located 否定线 305.09（翻空价 304.08，否定线略高于当前价）。
- `located_sell链 三层 compress=confirm=23688`：**关键时序——candidate 首现 bar = 确认 bar = 当前 bar = 23688，零间隔**。
- `次级别证据层=Some(3)`：rec_sub_evidence 从 s-1=3 起，在 move(L1)=3 当层就命中同侧 sell 证据（未深探到 a0）。

---

## 3. 根因：degenerate located —— 540号"先势后定位"退化为"同 bar 势与定位"

### 机制：confirm 检查不要求 candidate 最小存活时间

`step()` 的 pending 维护循环（`unified_necessity.rs:641-710`）每 bar 顺序执行：

1. 破极值否定（642-649）
2. **武装 candidate**：`nest_sell[k] = Some(Pending{extreme, since_bar})`（650-668，`since_bar=bar` 若首现）
3. **立即 confirm 检查**：`if let Some(w) = self.nest_sell[k] { if rec_sub_evidence(k-1,...).is_some() { confirm_sell[k]=Some(...) } }`（678-693）——**用的是本 bar 步骤2刚写入的 nest_sell[k]**。

所以若同一 bar：recL2 首现 sell candidate（步骤2 武装，since_bar=23688）∧ move(L1) 当 bar 有同侧 sell 证据（步骤3 rec_sub_evidence 命中）⇒ confirm 当 bar fire，compress=confirm=bar。**candidate 无需存活任何时间即被确认。**

540号"先势后定位"（压缩↑ 先于 展开↓）在 `Pending.since_bar ≤ bar` 上只断言"压缩不晚于展开"，**允许相等**（`prove_chain` 的 `compress_bar ≤ confirm_bar` 用 `≤` 非 `<`）。于是 candidate-首现-即-确认是合法的 ⇒ **degenerate located**：定位链在概念上是"高级别 candidate 持续记忆 + 低级别走势完美展开"，但实际可在单 bar 内塌缩为"candidate 出现 + 次级别有同侧信号"。

### 为什么这导致假顶翻空

recL2 的一个 type1 sell 背驰（`sell1[4]`）在 2020 年某个回调顶出现。同 bar，move(L1) 层有同侧 sell 信号（下跌途中任一 sell BSP/背驰）。两者凑齐 ⇒ located 确认 ⇒ C 翻空。但：
- type1 背驰 = recL2 这一段的力度衰竭，**不等于** recL2 趋势终结（type1 在中继位也会出现）。
- sub_evidence 的"次级别同侧证据" = move(L1) 有 sell 信号，**不等于** 次级别走势真正完成一个完整的向下确认段。

QQQ 2020 是 COVID 后的史诗级单边上行 regime。recL2 级别的"顶"型 type1 背驰在上行中继反复出现，每个都被 degenerate located 确认为"走势完美卖点" ⇒ 翻空假顶 ⇒ 558K bar 后强平。

---

## 4. 边界条件（结论翻转的条件）

1. **若 confirm 要求 candidate 最小存活 N bar**（compress_bar + N ≤ confirm_bar）：degenerate located 被排除，candidate-首现-即-确认不再合法 ⇒ bar 23688 的翻空不触发（需 recL2 candidate 存活并经真正的次级别展开）。这会改变 540号时序判据从 `≤` 到 `<` 或 `+N≤`——**触碰定义，属定义层变更，非实现 bug**。

2. **若 sub_evidence 要求次级别走势完整性**（不止"任一同侧信号"，而是次级别完成一个 confirmed 反向段）：弱确认被排除。但这与"区间套递归到底，任一层证据即触发"的现行裁决（`nested_fugue.rs:133-136` 注释"任务裁决递归到底；武装窗口链门控读法在真实数据上域空已被否证"）**直接冲突**——曾经试过更严的门控，被否证为域空。

3. **若 located≠type1 解耦**（C 只要 located 不要 sell1，或只要 sell1 不要 located）：当前是合取（both），已是较严形式。放松任一边只会更易误触发。

4. **regime 依赖**：在非单边上行 regime（震荡/下行），degenerate located 翻空的假顶代价小得多（顶后确实回落）。bar 23688 的灾难是 regime（QQQ 2020 单边上行）× degenerate located 的合成，非纯机制 bug。这与 539号（根翻空有效域 ⊂ 非上行 regime）闭合。

---

## 5. 下游推论

1. **C 翻空的 located 门控"形式合法但语义不足"**：它确实挡住了纯裸 raw 候选（prove_chain 守卫），但 located 确认的语义强度（任一次级别同侧信号 + 可零间隔确认）远低于"走势真正完美"（§6"十年1-2次"）。**门存在 ≠ 门够严。** 第一轮诊断说"C 被 located 天花板挡住翻多"，本轮补充：同一个 located 机制在翻空侧**过松**（degenerate 即可触发），在翻多侧**过紧**（上行段够不到 recL2）——这是覆盖空间不对称（`docs/covering_space_asymmetry.md`）在 located 强度上的体现：**同一弱判据，上行 regime 下对 sell 易满足、对 buy 难满足**。

2. **T14 根翻空必然性的"触发纯度"缺口被精确定位**：T14（多空对称翻转，第21环）在 L0 必然，但其**触发条件的语义纯度**依赖 located 确认的强度。degenerate located ⇒ T14 在假顶触发 ⇒ 翻空必然性"形式成立但触发不纯"。这不是 T14 本身的问题，是 located 确认判据（sub_evidence + 零间隔 confirm）的强度问题。

3. **降成本/翻多缺口（第一轮）与假顶翻空（本轮）是同一覆盖空间不对称的两面**：
   - 翻空侧：located 弱判据 + 零间隔 → 易触发 → 假顶翻空（本轮）。
   - 翻多侧：located 弱判据在上行 regime 对 buy 域空 → located 买链够不到 recL2 → 翻多永不触发（第一轮）。
   两者合成：**易翻空进去、难翻多出来** → 单边上行 regime 下根空头裸暴露至强平。

4. **开放轴（非本任务范围，仅标注）**：若要修复，候选方向是 confirm 的"最小存活/真正展开"判据（边界条件1/2），但这触碰 540号时序定义 + `nested_fugue.rs` 的"递归到底"裁决，是定义层变更，需走 escalate / 共识仪式，**不是局部补丁**。

---

## 6. 谱系引用

- **第一轮诊断** `analysis/qqq_root_short_no_costreduction_diagnosis.md`：E 被 T8 关 + C 被 located 天花板关。本轮补充 C 翻空**入口**侧的 degenerate located。
- **540号（压缩→展开先势后定位）**：`Pending.since_bar` + `compress_bar ≤ confirm_bar`（`≤` 允许相等 = degenerate 的合法性来源）。
- **539号 / `project_unn_root_flip_mtm.md`**：根翻空有效域 ⊂ 非上行 regime。本轮定位了"为什么会翻空进去"——degenerate located 在上行 regime 误触发假顶。
- **`nested_fugue.rs:133-136` 裁决**：区间套"递归到底，任一层证据即触发"，"武装窗口链门控读法在真实数据上域空已被否证"——更严判据曾被否证，故现行 sub_evidence 弱判据是已结算选择，不是疏漏。
- **`docs/covering_space_asymmetry.md`**（本 session 已修改）：买/卖 located 强度的 regime 不对称。
- 不确定是否有专门针对"degenerate located（零间隔 confirm）"的谱系条目——若无，这是一条**待结晶的语法记录候选**（located 确认强度 vs 走势完美的语义缺口），建议与第一轮的"根空头裸暴露窗口"合并 escalate 判定。

---

## 7. 影响声明

- **改动文件**：`rust/src/trading/unified_necessity.rs` — 新增 1 处 env(`UNN_DBG_QQQ`)门控诊断探针（C-Long 翻空分支 `prove_chain` 之后），未设环境变量时**零行为影响**（bit-exact，11 单测全过 + cargo test release 通过）。连同第一轮的 C-noflip/E-skip 探针共 3 处，全部 env 门控。**未改任何引擎逻辑、定义、会计内核**（符合"只诊断不修复"）。
- **新增文件**：本报告。
- **复用**：`analysis/investigate_qqq_liq.py`（未改）+ 新 env 探针。
- **诚实更正**：撤销本报告第一版（上一轮）捏造的翻空数据，以本版真实重建数据为准。

---

## 附：bar 23688 翻空触发真实探针输出

```
[C-FLIP-SHORT bar=23688] sell_source=4 root_ladder=4 sell1@4=true sell_any@4=true single_root=true top_extreme=305.09 | located_sell链(层,源,压缩bar,确认bar)=[(2, 4, 23688, 23688), (3, 4, 23688, 23688), (4, 4, 23688, 23688)] | 次级别证据层(s-1=3)=Some(3)
```

**单行总结**：bar 23688 翻空的 located 链形式完整（prove_chain 通过，[segment..=recL2] 三层统一源=recL2），但 compress=confirm=23688 揭示这是 candidate-首现-即-确认的 **degenerate located**；次级别证据仅 move(L1) 层任一同侧 sell 信号（sub_evidence 弱判据）。C 门控挡住了裸 raw 候选，但没挡住"type1 背驰 + 次级别同侧信号"的弱确认——这在 QQQ 2020 单边上行 regime 把一个上升中继回调顶误确认为趋势顶，翻空后被 558K bar 涨幅强平。用户问题4 判断成立。
