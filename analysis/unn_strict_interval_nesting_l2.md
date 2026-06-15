# unn located 严格区间套确认 —— 三处偏离修复的 L2 实证

> 任务（编排者 2026-06-15）：按 `analysis/buysellpoint_necessity_vs_canon.md`（L0 审计）严格修复
> unn located 确认偏离缠师原文的三处，然后在 NautilusTrader 下回测。
>
> 认识论等级：实装为 **L0**（按原文收紧定义）；prove 零 panic 为 **L1**（管线/必然性自洽）；
> bit-exact 为 **L1**；8 标的回测 域空 为 **L2/L3**（真实数据否定性结果，缩小 strict located 有效域）。

---

## 1. 结论

三处偏离已严格修复并落码（`rust/src/trading/unified_necessity.rs`），cargo build + 14 unn 测试 +
325 trading 测试全绿，prove N1-N8 在 8 标的真实数据上**零 panic**（unn 验收标准达成），
NT 流式 vs 批量 **bit-exact**。**bar 23688 假翻空消失**——但经验有效域读数为 **8/8 标的总域空**
（0 入场、0 笔、+0.0%）。**strict 区间套"低三级以上"在 organic_signals 信号层上有效域 ≈ 空集**——
这是一个否定性结果（formalization-validity-domain.md：缩小有效域边界 > 确认），**经验证实了
`nested_fugue.rs:135-136` 的已结算裁决**（"武装窗口链门控读法在真实数据上域空已被否证"）。

## 2. 定义依据（三处修复 → 原文）

| 修复 | 原文 | 实装（unn 专属，不动共享 `rec_sub_evidence`/nrf） |
|------|------|------|
| ① 低三级以上 | 第64课:65 区间套定理"低三个级别以上…精确定位" | `rec_sub_evidence_strict`：命中层 `j ≤ k−3`（递归深度 ≥3）；过浅的 k−1/k−2 命中排除 |
| ② 后续走势验证 | 第29课:52/54"看反弹中次级别走势是否重新回抽中枢" | confirm 门 `since_bar < bar`（严格小于）+ `prove_chain`/`cascade_arm` `compress < confirm`（release-active assert）|
| ③ 逐级第一类 | 第29课:396"所有买点…都要下次级别以下找第一类" | strict 只认同侧 **type1**——type2/type3/裸背驰事件不算；a0(bi)层用方向翻转沿（定位到点）|

实装范畴：三处均在 unn 的 located 路径（confirm→cascade→C 翻转/清仓 + F 入场）。
**E 降成本（N7 自层 nf）保持 loose 不动**（§9"绝大多数卖点只是降成本"）——`Pending.fired_nf`
标志使 nf 每 candidate 仅 fire 一次且不消费窗口，窗口存活到 strict located 确认或破极值。
BSP 引擎（`buysellpoint.rs`）零改动。

## 3. 有效域读数（L2/L3，8 标的批量普查 300K bar/标的）

| 标的 | arms@move(L1) | arms@recL2+ | 入场 | 笔 | strat% | BH% |
|------|--------------:|------------:|-----:|---:|-------:|----:|
| OKLO | 504 | 23 | 0 | 0 | +0.0 | +948.3 |
| QQQ  | 627 | 28 | 0 | 0 | +0.0 | +77.3 |
| GC   | 434 | 23 | 0 | 0 | +0.0 | +22.5 |
| CL   | 442 | 20 | 0 | 0 | +0.0 | +55.1 |
| BTC  | 422 | 21 | 0 | 0 | +0.0 | +85.3 |
| BRN  | 438 | 19 | 0 | 0 | +0.0 | +23.5 |
| DX   | 441 | 16 | 0 | 0 | +0.0 | +1.8 |
| ES   | 249 | 11 | 0 | 0 | +0.0 | +23.7 |

NT 流式确认（QQQ 728K 全量 + OKLO 200K）：零 panic、bit-exact PASS、0 笔——与批量一致。

**根因（非 bug，计数确证）**：pending 势源下界 `PENDING_LO = move(L1) = k=3`，而 8 标的候选
**~95% 落在 move(L1)=k=3**。第64课"低三级以上"从 move(L1) 向下只有 segment(2)、bi(1) **两级**
（k−3=0=bar 层无 BSP/翻转沿）⇒ move(L1) 候选**结构上永远无法 confirm located**（能 arm pending、
能 fire nf 走 E，但 located 需 k≥4 = recL2+）。recL2+ 候选仅占 ~5% 且极难同时满足 strict
type1 + 后续走势时序 ⇒ located 域空 ⇒ F 永不入场 ⇒ 无根 ⇒ E 无处 spawn ⇒ 全链 0 笔。

## 4. 边界条件（结论翻转条件）

1. **若 "3级" 重映射为"递归到 bi 笔级点"（k−2，含 segment+bi-flip）**：move(L1) 候选可
   confirmed（segment type1 + bi 翻转沿）⇒ 引擎重新有交易。但这偏离编排者字面"递归至少深入
   3级"。**ladder↔级别映射是翻转点**（`buysellpoint_necessity_vs_canon.md` 边界条件1 预标注待核）。
2. **若 PENDING_LO 下移 / 信号层产出更高级别（recL2+）BSP**：高级别候选增多 ⇒ strict located
   有效域非空。当前 organic_signals 的 BSP 级别分布（move L1 主导）是域空的直接原因。
3. **若 strict 区间套改为时序累积（多 bar 逐级 confirm 状态机）而非单 bar 同时 type1**：
   引擎读单 bar evrows，真实区间套是跨 bar 的——单 bar 要求 3 级同时 type1 本就近乎不可满足。

## 5. 下游推论

1. **bar 23688 假翻空根除**：决定性死因是 ②（`compress=confirm=23688` 同 bar ⇒ `since<bar` 假）。
   该机制泛化到全部 confirm ⇒ 总域空。假翻空消失是真，但代价是引擎不参与。
2. **矛盾定位到信号层**：strict located 域空把"走势级别 1买/1卖 区间套确认"的缺口指向
   **organic_signals 信号层产出 BSP 的级别太低（move L1 主导）**，而非交易层。下一轴 = 信号层
   是否应产出 recL2+ 级别的 BSP / 走势完成事件。
3. **确认已结算裁决**：本结果是 `nested_fugue.rs:135-136`"深度门控域空已被否证"的独立 L2 复现。
   strict 正典忠实实装 → 域空，正是当初裁定"任一层证据即触发"的经验根据。

## 6. 谱系引用

- **L0 审计**：`analysis/buysellpoint_necessity_vs_canon.md`（本实装的定义依据；其第6.4/7 节预标注
  "需 escalate / 共识仪式" + 边界条件1 "ladder↔级别映射待核"——本 L2 实证把理论张力变成经验否证）。
- **已结算裁决**：`nested_fugue.rs:135-136`"武装窗口链门控读法在真实数据上域空已被否证"——本结果复现之。
- **539号 / `project_unn_root_flip_mtm`**："根翻空有效域⊂非上行 regime"——本结果给出更深层根源
  （located 确认级别太低）。
- **不确定**：是否有谱系条目专门处理"区间套定理实装深度 vs 信号层 BSP 级别分布"的矛盾。
  若无，本报告 + L0 审计共同指向一条待结晶的语法记录：**strict 区间套有效域 = 信号层级别分布的函数**。

## 7. 影响声明

- **改动**：`rust/src/trading/unified_necessity.rs`——新增 `rec_sub_evidence_strict`；`Pending` 加
  `fired_nf`；confirm 块 nf/located 解耦（nf loose 不消费 + located strict 消费）；`cascade_arm`/
  `prove_chain` 时序判据 `≤→<`；新增 3 个修复验证测试（fix1/fix2/fix3）。
- **不改**：`buysellpoint.rs`（BSP 引擎）、`nested_fugue.rs`（共享 `rec_sub_evidence`/nrf v4 基座，
  325 测试守卫不变）、其余 PolarityMode。
- **守恒律**：prove N1-N8 零 panic（含 N8 守恒、T1/T14/A5），8 标的 L2 达成；bit-exact 流式==批量。
- **有效域**：strict located 有效域 ≈ 空集（8/8 域空）——否定性 L2 结果，已按等级标注。
