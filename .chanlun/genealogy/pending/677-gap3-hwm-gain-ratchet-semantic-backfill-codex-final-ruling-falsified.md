---
id: 677
title: "GAP3 hwm_gain 高水位棘轮被 codex 终局裁定为 PDF p8③ 语义回补——不能作 acceptance bridge，GAP3 判据 (∃t,TStage=EarningShares) 恢复 FALSIFIED"
type: bias-correction
status: 已结算
date: 2026-07-02
negation_source: heterogeneous
negation_model: "codex-cli 0.142.5（本机 codex exec --skip-git-repo-check --sandbox read-only）"
negation_form: expansion
# expansion：GAP3 桥（规定者）在执行中违反自身声明的最低条件——声明了它不具备的能力（PDF 合规可达性），
# 用历史高水位浮盈峰值把已被 L0 否定的「sound 资金源存在」条件重新解释为成立（声明膨胀，231/090）
negates: "gap3-bridge acc-GAP3 『字面 PASS』承重节点（task #10 产出 / gap3-bridge-20260702.md）"
topo_effect: "split:gap3-bridge:downstream"
# split（expansion→分裂）：GAP3 桥节点分裂为两部分——
#   保留原规定：Revalue(g) 构造子本体 + 价格幅度打通管线（codex 明确「不是说任何 Revalue 构造子都非法」「不是全盘推倒」）
#   携带违反记录：hwm_gain 对 free/cum_net_cash/stage_progression 的承重链路（语义回补，作废）
# scope=downstream：影响 pdf-conformance-audit R3 段 + 依赖 Revalue 驱动阶段推进的 5 个 closed_loop 测试
authorization: "编排者 2026-07-02 主对话明令『有要裁定的直接问 codex，他全权裁定』→ 650 授权要件满足 → 裁定即终局，不回蜂群内部二次讨论合法性"
related:
  - '576'  # R vs TW 账本边界——codex 已确认本裁决未破坏其正交性，576 结论不受影响（无张力）
  - '231'  # 形式化有效域——hwm_gain 声明膨胀实例：L0/L1 结构桥被误用为 L2 可达证据（有效域≠定义域）
  - '090'  # 严格性语法——候选A『仅降级措辞不改代码』被裁定不足，措辞诚实不能豁免 ledger 语义违规
provenance: "[新缠论:形式化] codex-cli 0.142.5 终局裁定（L1 文本/代码对照，非数据验证）+ PDF《New-Chan 不变量映射：断点检测与最小验证框架》p8①②③ 一级权威 + hwm_gain 实现全文（ledger.rs:314-430 / transition.rs:359-399）"
epistemological_level: "L1（文本/代码对照裁决，产出=合规性判定，不产生 alpha 证据）；被纠正的『acc-GAP3 PASS』此前是 L2 字面 PASS 但为 hwm 棘轮口径诊断性触发，非 PDF 合规可达性"
ruling_source: ".chanlun/review-results/codex-r3-ruling-20260702.md（裁决全文）"
---

# 677号：GAP3 hwm_gain 高水位棘轮 = PDF p8③ 语义回补（codex 终局裁定 C'）

## 裁决摘要

GAP3 三阶段资金机（CostReduction → CapitalRecovered → EarningShares）为补桥新增非对称账本事件
`Revalue(g)`：把「价格重估浮盈超过历史高水位的增量」计入可分配权益（`free`），使原本在纯 L0
价格不变模型下被代码证明为 FALSIFIED 的判据 `∃t, TStage(t)=EarningShares`，在接入真实 BTC 历史
价格（461万根 K 线）后翻转为字面 PASS（`EarningShares_count=1`）。

争议焦点：`hwm_gain` 高水位棘轮（只增不还——`credit=max(0, unrealized−hwm_gain)`，价格回撤后
已入账的历史增量不撤销）是否构成 PDF p8③「Ledger 的强制介入点」所禁止的**语义回补**。

**codex 终局裁定 = C'（移除棘轮承重，非仅改措辞）**：`hwm_gain` 高水位棘轮作为 GAP3 可达性补桥
**违反** PDF p8③「禁止语义回补」。必须移除它对 `free` / `cum_net_cash` / `stage_progression` /
`acc-GAP3 PASS` 的承重作用。非法的不是任何 `Revalue` 构造子，而是当前这个具体组合：`credit =
max(0, unrealized − hwm_gain)`，只记正向峰值、不做负向回撤、并把未实现峰值永久写进可分配 `free`，
再用它推进 `RecoverCapital → EnterEarning`。

## 推导链（codex 原文，未删减）

1. PDF p8① 定义"断裂"：断裂不是新状态，而是某个已声明不变量的旧条件不再可维持。
2. 对 GAP3 而言，被声明的最低条件是：三阶段机进入 `EarningShares` 前，本金退回必须有 ledger 上
   可承载的资金源。旧 L0 证明给出否定：`TW` 守恒且 `holding ≥ notional_in` 时，`free ≤ 0`
   （互斥恒假），所以 `RecoverCapital` 的 sound 资金源不存在。
3. 这就是 p8① 意义上的否定句：不是"三阶段完成得晚一点"，而是"在当前账本语义下，进入终态的
   最低条件不存在"。
4. PDF p8② 进一步说，不变量不是结构外壳，而是生成过程能否持续的最低条件。最低条件消失后，
   不需要再用"形态变化"解释它为什么还在继续。
5. PDF p8③ 因此要求 Ledger 在条件被否定时冻结解释权：不能用"级别、调整、延续"把已被否定的
   条件重新解释成成立。
6. hwm 机制核心结构：当前浮盈超过历史高水位才正向入账；价格回撤时不撤销；`ClearCampaign` 只在
   campaign 边界清零（非回撤触发）。因此 campaign 内部的 `free` 记住的是"曾经见过的峰值"，
   不是当前可退出现金，也不是已实现利润。
7. 这个峰值随后直接进入 `stage_progression`：历史高水位浮盈被当作 `free`，触发足额退本金，再
   进入 `EarningShares`。这正是在最低条件被否定后，用"曾经达到过的价格峰值"延续三阶段解释权。
8. 与 p8③ 禁止项的对应关系直接：`延续`=回撤后仍保留已入账解释权；`调整`=把"可分配现金/已实现
   利润"调整成"高水位未实现浮盈"；`级别`=用 L0/L1 结构桥承重 L2 判据 PASS。诚实标注"非盈利
   声明"只能降低叙述风险，不能豁免 ledger 语义违规。
9. 结论：`EarningShares_count=1` 只能称为"hwm 棘轮口径下的诊断性触发"，不能称为 PDF 合规的
   `acc-GAP3 PASS`。合法账本语义下，当前 GAP3 盈利/赚份额可达性仍应保持 **FALSIFIED / 未证成**，
   直到有新的、非回补的资金源语义。

## 三候选的裁定结果

| 候选 | 内容 | 裁定 |
|------|------|------|
| A | 降级叙述——代码不变，仅把「PASS」措辞降级为「机制可触发/盈利仍 FALSIFIED」 | **否定**（措辞诚实不能豁免代码违规，090号实例） |
| B | 维持现状——已如实标注 L0/L1 口径即合规 | **否定**（同上，ledger 语义违规仍在） |
| C→C' | 移除棘轮对 free/cum_net_cash/stage_progression 的承重（保留为诊断字段） | **裁定采纳** |

## 涉及的定义与代码

- PDF p8①②③（`docs/pdfs/写作 - New‑Chan 不变量映射：断点检测与最小验证框架.pdf` 第8页）：
  断裂句法条件 / 不变量存在依赖 / Ledger 强制介入点（禁止用「级别、调整、延续」语义回补）。
- `rust/src/theta_v0/strategy/ledger.rs:314-342`：`TwEvent::Revalue(g)` 枚举定义（第7个非对称
  构造子，非守恒，free/cum_net_cash/hwm_gain 各增 g）。
- `rust/src/theta_v0/strategy/ledger.rs:374-430`：`tw_step` 状态转移——`ClearCampaign` 清零
  hwm_gain（仅 campaign 边界，非价格回撤触发）；`Revalue(g)` arm 三量各增 g。
- `rust/src/theta_v0/closed_loop/transition.rs:359-399`：`transition_adapter` 重估步——
  `credit = max(0, unrealized − hwm_gain)`；`credit>0` 才派 `Revalue(credit)`；随后
  `stage_progression` 在重估后的态上评估足额退本金门。
- `.chanlun/review-results/gap3-bridge-20260702.md`：此前声称的「字面 PASS」判决摘要（已翻转）。

## 谱系比对结果（张力检查）

- **576号**（R=Π-A-W vs TW 三阶段账本不同构）：codex 明确确认本裁决**未破坏 576 正交性**——本次
  改动只涉及 hwm_gain 承重链路，不触及两账本范畴的不同构结论。576 结论不受本裁决影响，**无张力**。
- **231号**（形式化有效域：有效域 ≠ 定义域）：本裁决是 231 的**新实例**——hwm_gain 是 L0/L1
  结构桥（外生市价浮盈高水位口径），被误用为 L2 可达性证据。声明膨胀 = 把 L0/L1 有效域冒充为
  L2 有效域。与 231 **一致（深化）**，无矛盾。
- **090号**（严格性语法：声明膨胀禁止）：候选A「仅降级措辞不改代码」被裁定不足——措辞层面的
  诚实标注不能替代代码层面的机制移除。这是 090 在「诚实标注 ≠ 豁免代码违规」语境的**新实例**。
  与 090 **一致**，无矛盾。

**结论：三条关联谱系均为一致深化或新实例，无不可分层解决的矛盾，不产生概念分离中断（#1）。**

## 影响声明与下游行动

本条目为裁决归档，零代码改动、零 git 操作。下游行动已由 Lead 拆分为独立工位：

1. **代码改动（task #34，行动类）**：移除 hwm_gain/Revalue(credit) 对 free/cum_net_cash/
   stage_progression 的承重；RecoverCapital/EnterEarning 不得由 hwm 棘轮驱动。涉及 ledger.rs、
   transition.rs。影响下游 5 个 closed_loop 测试（`l2_btc_*`、`price_magnitude_drives_tw` 等）
   需重新评估/改写。
2. **文档订正（task #35，行动类）**：gap3-bridge-20260702.md、pdf-conformance-audit-20260702.md
   R3 段、任何引用「acc-GAP3 PASS」/「L2 可达 count=1」的结果包，全部降级为「hwm 棘轮口径可
   触发（诊断性），PDF 合规判据仍 FALSIFIED」；pdf-conformance R3 从「偏离（潜在）」改为
   「偏离（确认，codex 终局裁决）」。
3. **task #10（GAP3 补桥）状态**：标注为「部分作废」——Revalue 构造子本体 + 价格幅度打通管线
   **保留**（codex 明确保留其合法性），stage_progression 承重链路**作废需重做**。非全盘推倒。

## 边界条件（结论翻转条件）

若未来改造为以下任一非回补资金源语义，可重新提交裁决：
- (a) 保留 hwm_gain 作诊断字段，但从 free/cum_net_cash/stage_progression 承重链路移除；
- (b) 若坚持价格桥路径，改为「已实现利润」入账（而非未实现浮盈峰值）；
- (c) 做可正可负的当前 MTM 账本，把"权益"和"可退现金"概念分离。

在此之前，GAP3 盈利/赚份额可达性判据 `∃t, TStage(t)=EarningShares` 保持 **FALSIFIED / 未证成**。
