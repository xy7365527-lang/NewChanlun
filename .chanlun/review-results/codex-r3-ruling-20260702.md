# codex 全权裁定：GAP3 桥 hwm_gain 高水位棘轮 vs PDF「禁止语义回补」（R3 终局）

- 工位: ws-r3ruling（task #31）| 分支: gap3-rework-codex9-fix | 日期: 2026-07-02
- 裁决路径: 编排者明令「有要裁定的直接问codex，他全权裁定」——本轮裁决即终局，不回蜂群内部二次讨论合法性
- 调用方式: 本机 `codex exec --skip-git-repo-check --sandbox read-only`（codex-cli 0.142.5），非 API
- 认识论等级: L1（文本/代码对照裁决，非数据验证；产出=合规性判定，不产生 alpha 证据）

## 1. 争议陈述

GAP3（三阶段资金机 CostReduction → CapitalRecovered → EarningShares）补桥新增非对称账本事件
`Revalue(g)`：把「价格重估浮盈超过历史高水位的增量」计入可分配权益（`free`），使原本在纯 L0
价格不变模型下被代码证明为 FALSIFIED（`TW`守恒 ∧ `holding≥notional_in` ⟹ `free≤0`，互斥恒假）的
判据 `∃t, TStage(t)=EarningShares`，在接入真实 BTC 历史价格（461万根K线）后翻转为字面 PASS
（`EarningShares_count=1`）。

争议焦点：`hwm_gain` 高水位棘轮（只增不还——`credit=max(0, unrealized−hwm_gain)`，价格回撤后
已入账的历史增量不撤销）是否构成 PDF《New-Chan 不变量映射：断点检测与最小验证框架》第8页③条
「Ledger 的强制介入点」所禁止的**语义回补**。

## 2. 证据包摘要（提交 codex 前打包）

### 2.1 PDF p8①②③ 原文（OCR 摘录，`docs/pdfs/写作 - New‑Chan 不变量映射：断点检测与最小验证框架.pdf` 第8页）

> ① 断裂的句法条件
> - 断裂必须以否定句出现
> - 不能以"完成/形成/结束"类谓词出现
> - 必须指向某个已声明的不变量
> 断裂 ≠ 新状态；断裂 = 旧条件不再可维持
>
> ② 不变量的存在依赖
> - 不变量不是"结构"
> - 而是**生成过程能否持续的最低条件**
> - 一旦条件消失，不需要再发生任何"形态变化"
>
> ③ Ledger 的强制介入点
> - Ledger 不是记录结果
> - 而是**在条件被否定时，强制冻结所有解释权**
> - **不允许用"级别、调整、延续"进行语义回补**

### 2.2 hwm_gain / Revalue 实现全文摘录

- `rust/src/theta_v0/strategy/ledger.rs:314-342`：`TwEvent::Revalue(g)` 枚举定义 + 文档注释（第7个
  非对称构造子，非守恒，`free/cum_net_cash/hwm_gain` 各增 g）。
- `rust/src/theta_v0/strategy/ledger.rs:374-430`：`tw_step` 状态转移——`ClearCampaign` 把 `hwm_gain`
  清零（仅 campaign 边界，非价格回撤触发）；`Revalue(g)` arm 三量各增 g。
- `rust/src/theta_v0/closed_loop/transition.rs:359-399`：`transition_adapter` 重估步——
  `unrealized = positions·price − holding`；`credit = max(0, unrealized − hwm_gain)`；
  `credit>0` 才派 `Revalue(credit)`；随后 `stage_progression` 在重估后的态上评估足额退本金门。

### 2.3 GAP3 桥翻转上下文（`.chanlun/review-results/gap3-bridge-20260702.md`）

判决摘要原文：「acc-GAP3 判据「∃t TStage=III」在 L2 真实 BTC 变价数据上**字面 PASS**——
`l2_btc_earning_shares_reachability` 全历史 461万 bar 实测 `EarningShares_count=1`」。
边界条件自述：「hwm_gain 是外生市价浮盈高水位口径（L0/L1 结构桥），非已卖出实现现金、非盈利/
实盘有效声明……高水位棘轮（只增不还）在单次尖峰序列上会给出可达（乐观），这是 L0 结构口径的
已知边界」。

### 2.4 蜂群此前判定（供 codex 参考，非约束）

`.chanlun/review-results/pdf-conformance-audit-20260702.md` R3 段判定为「偏离（潜在）」，四分法=
选择类（不可蜂群自决），建议 A 批叙述降级为「机制可触发（L0），盈利可达性仍 FALSIFIED」。

### 2.5 三个候选裁定（提交 codex 时给出）

- **候选 A**：降级叙述——代码不变，仅把「PASS」措辞降级为「机制可触发/盈利仍 FALSIFIED」。
- **候选 B**：维持现状——已如实标注 L0/L1 口径即合规，不改代码不改叙述。
- **候选 C**：移除棘轮——认定结构性违反，重新设计 Revalue 机制或放弃 L2 补桥路径。

## 3. codex 裁定原文（完整，未删减）

> 已按 ceremony 热启动校准；当前中断点快照陈旧，所以我以当前源码和 review 工件为准。
>
> **终局裁定**
>
> 裁定选 **C，但精确化为 C'**：`hwm_gain` 高水位棘轮作为 GAP3 可达性补桥 **违反** PDF p8③「禁止
> 语义回补」。必须移除它对 `free` / `cum_net_cash` / `stage_progression` / `acc-GAP3 PASS` 的承重
> 作用。
>
> 不是说任何 `Revalue` 构造子都非法；非法的是当前这个组合：`credit = max(0, unrealized -
> hwm_gain)`，只记正向峰值，不做负向回撤，并把未实现峰值永久写进可分配 `free`，再用它推进
> `RecoverCapital -> EnterEarning`。
>
> **推导链**
>
> 1. PDF p8①定义"断裂"：断裂不是新状态，而是某个已声明不变量的旧条件不再可维持。
> 2. 对 GAP3 来说，被声明的最低条件是：三阶段机进入 `EarningShares` 前，本金退回必须有 ledger
>    上可承载的资金源。旧 L0 证明给出否定：`TW` 守恒且 `holding >= notional_in` 时，`free <= 0`，
>    所以 `RecoverCapital` 的 sound 资金源不存在。
> 3. 这就是 p8①意义上的否定句：不是"三阶段完成得晚一点"，而是"在当前账本语义下，进入终态的
>    最低条件不存在"。
> 4. PDF p8②进一步说，不变量不是结构外壳，而是生成过程能否持续的最低条件。最低条件消失后，
>    不需要再用"形态变化"解释它为什么还在继续。
> 5. PDF p8③因此要求 Ledger 在条件被否定时冻结解释权：不能用"级别、调整、延续"把已被否定的
>    条件重新解释成成立。
> 6. 当前 hwm 机制的核心结构是：当前浮盈超过历史高水位才正向入账；价格回撤时不撤销；
>    `ClearCampaign` 只在 campaign 边界清零，不在回撤时修正。因此 campaign 内部的 `free` 记住的
>    是"曾经见过的峰值"，不是当前可退出现金，也不是已实现利润。
> 7. 这个峰值随后直接进入 `stage_progression`：历史高水位浮盈被当作 `free`，触发足额退本金，再
>    进入 `EarningShares`。这正是在最低条件被否定后，用"曾经达到过的价格峰值"延续三阶段解释权。
> 8. 所以它与 p8③禁止项对应关系很直接：
>    `延续` = 回撤后仍保留已入账解释权；
>    `调整` = 把"可分配现金/已实现利润"调整成"高水位未实现浮盈"；
>    `级别` = 用 L0/L1 结构桥承重 L2 判据 PASS。
>    诚实标注"非盈利声明"只能降低叙述风险，不能豁免 ledger 语义违规。
> 9. 结论：`EarningShares_count=1` 只能称为"hwm 棘轮口径下的诊断性触发"，不能称为 PDF 合规的
>    `acc-GAP3 PASS`。合法账本语义下，当前 GAP3 盈利/赚份额可达性仍应保持 **FALSIFIED / 未证成**，
>    直到有新的、非回补的资金源语义。
>
> **下游改动**
>
> 代码需要改：禁用 `hwm_gain` 对 `free`、`cum_net_cash` 和阶段推进的承重作用。可保留为诊断字段，
> 但不得驱动 `RecoverCapital` / `EnterEarning`。若继续做价格桥，优先走已实现利润路径；或者做可
> 正可负的当前 MTM 账本，并把"权益"和"可退现金"分开。
>
> 文档也需要改：所有"acc-GAP3 PASS""L2 真实 BTC 可达""count=1 证明可达"的表述都要降级为"hwm
> 棘轮实验口径可触发，非 PDF 合规可达性"。`gap3-bridge-20260702.md` 的摘要、
> `pdf-conformance-audit-20260702.md` 的 R3 项应从"潜在偏离"改成"确认偏离"。
>
> 还需要补一条 genealogy：当前沙箱只读，我没有写入；下游执行时应记录"hwm_gain 高水位棘轮被裁定
> 为语义回补，不能作为 GAP3 acceptance bridge"。

## 4. 终局结论

**裁定 = C'（移除棘轮承重，非仅改措辞）**。三候选中候选 A（仅降级叙述）与候选 B（维持现状）均被
否定——codex 明确指出"诚实标注'非盈利声明'只能降低叙述风险，不能豁免 ledger 语义违规"，即
措辞层面的诚实标注不能替代代码层面的机制移除。

- **GAP3 acceptance 判据**（`∃t, TStage(t)=EarningShares`）在真实 BTC L2 数据上的状态**恢复为
  FALSIFIED**（此前 `gap3-bridge-20260702.md` 声称的「字面 PASS」不再成立——该 PASS 是 hwm 棘轮
  口径下的诊断性触发，非 PDF 合规可达性）。
- `hwm_gain`/`Revalue` 机制本身不是全盘非法（codex 明确「不是说任何 Revalue 构造子都非法」），
  非法的是"只记正向峰值+不撤销+驱动 stage_progression"这个具体组合。允许的改造方向：
  (a) 保留 `hwm_gain` 作诊断字段，但从 `free`/`cum_net_cash`/`stage_progression` 承重链路移除；
  (b) 若坚持价格桥路径，改为「已实现利润」入账（而非未实现浮盈峰值）；
  (c) 或做可正可负的当前 MTM 账本，把"权益"和"可退现金"概念分离。
- `pdf-conformance-audit-20260702.md` R3 段的判定需从「偏离（潜在）」更新为「偏离（确认，codex
  终局裁决）」。

## 5. 下游行动清单（不自行实施——按四分法拆分为后续任务）

1. **代码改动（行动类，需实施工位）**：移除 `hwm_gain`/`Revalue(credit)` 对 `free`/
   `cum_net_cash`/`stage_progression` 的承重作用；`RecoverCapital`/`EnterEarning` 不得由 hwm 棘轮
   驱动。涉及 `rust/src/theta_v0/strategy/ledger.rs`、`rust/src/theta_v0/closed_loop/transition.rs`。
   影响下游：`gap3-bridge-20260702.md` 中依赖 `Revalue` 驱动阶段推进的全部测试（`l2_btc_*`、
   `price_magnitude_drives_tw` 等 5 个 closed_loop 测试）需重新评估/改写。
2. **文档措辞订正（行动类）**：`gap3-bridge-20260702.md`、`pdf-conformance-audit-20260702.md` R3
   段、以及任何引用「acc-GAP3 PASS」/「L2 可达 count=1」的结果包，全部降级为「hwm 棘轮口径可
   触发（诊断性），PDF 合规判据仍 FALSIFIED」。
3. **谱系记录（genealogist 职责）**：写入本次终局裁决——「GAP3 hwm_gain 高水位棘轮机制被 codex
   终局裁定为 PDF p8③ 语义回补，不能作为 GAP3 acceptance bridge；GAP3 判据恢复 FALSIFIED」。
4. task #10（GAP3 补桥）的产出状态需标注为"部分作废"（Revalue 构造子本身+价格幅度打通管线
   保留，stage_progression 承重链路需重做）——不是全盘推倒，codex 明确保留了构造子本体的合法性。

## 6. 结果包六要素（精简版，工位性质=裁决记录，非代码产出）

1. **结论**：codex 终局裁定 C'——GAP3 桥 hwm_gain 高水位棘轮违反 PDF p8③ 禁止语义回补，须移除
   其对 free/stage_progression 的承重；GAP3 acceptance 判据恢复 FALSIFIED。
2. **定义依据**：PDF p8①②③ 原文（断裂句法条件/不变量存在依赖/Ledger强制介入点）+ hwm_gain
   实现全文（ledger.rs/transition.rs）+ gap3-bridge 结果包翻转声明。
3. **边界条件**：若未来改造为「已实现利润」入账或可正可负 MTM 账本（不驱动棘轮式承重），可
   重新提交裁决；若仅改措辞不改代码，裁定不认可（候选 A/B 已被明确否定）。
4. **下游推论**：task #10 GAP3 补桥部分作废；`pdf-conformance-audit-20260702.md` R3 从潜在偏离
   转为确认偏离；GAP3 acceptance 状态回退 FALSIFIED，直到重新实装。
5. **谱系引用**：本裁决触及 576号（R vs TW 账本边界，codex 已确认本次改动未破坏 576 正交性，
   该结论不受本裁决影响）、231号（形式化有效域，hwm_gain 声明膨胀问题）、090号（严格性语法，
   候选A"仅降级措辞"被裁定不足以豁免代码违规）。
6. **影响声明**：本文件为裁决归档，零代码改动、零 git 操作。产出=终局裁决记录 + 下游行动清单
   （代码改动/文档订正/谱系记录三项需后续工位分别执行）。
