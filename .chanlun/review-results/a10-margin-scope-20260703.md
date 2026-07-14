# A10-margin 实装面三分界定（范围阶段，task #169）

- **工位**：ws-a10scope（topo_address: swarm/ws-a10scope，parent_callback: main）
- **日期**：2026-07-03
- **任务来源**：gap-master2-final-20260703.md 行34「A10 | margin M2/M3 接线+ADL/资金费强平未建模（「margin 分段快照」在办项不覆盖此部分）| 中 | 接 IBKR/多品种前收口；BTC 当前不 binding | 外部数据项后置」
- **范围**：只读三分界定，不改代码，不改 margin-model-design 文档本身。

---

## 0. 核心发现（颠覆 team-lead 原判断，须先澄清）

**gap-master2-final 对 A10 的原始表述已过时。** 该行把「margin M2/M3 接线」和「ADL/资金费强平未建模」并列为同一颗粒度的待办项，暗示整个 A10 都在等外部数据。但核实 `#113`（margin-impl，**已 completed**）与 `#135`（q4-full-pi-backtest，**已 completed**）的实际产出后确认：

**「M2/M3 接线」已经完整实装并已跑批验证，不是待办项：**

| margin-model-design v2 条目 | 实装位置 | 验证状态 |
|---|---|---|
| `MarginSchedule`（Binance 分级 + CME 简化，§2.2） | `risk.rs:597-660` | golden-vector 测试通过（`margin_binance_tier_golden`/`margin_cme_simple_golden`，对照交易所公布数字，L1） |
| `MarginScheduleBook` 分段快照 as_of（§2.7，zero-lookahead） | `risk.rs:662-694` | `margin_book_as_of_no_lookahead` 测试通过 |
| `margin_inputs()` 统一算 MM+liq_flag（§2.2/§2.4 修正） | `risk.rs:731-745` | `margin_inputs_liq_and_reachability` 测试通过 |
| fail-loud 构造期校验（§2.9） | `RiskCushions::new`/`MarginSchedule::binance_tiered`/`cme_simple` | `margin_fail_loud_rejects` 测试通过 |
| buffer1/2 敏感性网格（§3.5，codex 强制项） | `risk.rs:1270` `margin_buffer_sensitivity_grid` | 已实装为单测，暴露 buffer 网格触发态差异 |
| `KThetaRiskGate.no_increase_cap` + M2/M3 真接线（§2.8） | `coverage.rs:2105-2134` + `runner.rs:483-572`（`k_theta_risk_gate`） | `k_theta_risk_gate` 按 mode 算 `no_increase_cap`，M2/M3 真改可行集 |
| 生产 π 回测接入 | `runner.rs:767` 消费 `config.margin` | **`#135` 已用 `margin=Some(CME-simple)` 跑通 12 窗生产回测**（`q4-fullpi-results-20260703.md` §3.2）|

`#135` 的 L2 结论（Arm1 vs Arm3 逐位相同，ΔΣpnl=0/Δorders=0）说明「G7 毛头寸约束 + margin 保证金约束」在**当前 sizing/nav 规模下从未触发**——机制正确接通，只是当前账户规模远未触及保证金边界。这是**已完成的经验结论**，不是「未实装」。

**结论：team-lead 描述中「margin M2/M3 接线」这一半已经 completed，需要从 A10 的待办范围中划掉。** 真正剩余的、team-lead 括号里点名的「ADL/资金费强平未建模」才是本次三分的实质对象。

---

## 1. 三分结果

### (a) 现在可实装（纯代码接线，不依赖任何外部数据）—— **为空**

逐项核对 margin-model-design v2 全部 9 个小节（§2.1-§2.9）+ codex-margin-ruling 的 3 项致命缺口 + 2 项重要缺口 + 3 项选择裁定，全部已在 `#113` 落地并通过对应单测：致命缺口①②③（历史快照对齐/单位错误/liq_flag 签名）已修复，重要缺口④⑤（M2/M3 无消费者/输入校验缺失）已修复，选择裁定 (a)(b)(c)（buffer 归 Θ_risk+敏感性网格/one-way 默认/CME-simple 标签）已落地。

没有遗留「设计已给出、代码未接线」的缺口。**(a) 类当前无可派工项。**

若后续要补的增量（非缺口，纯锦上添花，不建议现在派工）：
- `liquidation_flag` 作为独立 helper 暴露（当前内联在 `margin_inputs` 内，设计 §2.4 本就说明这是「采纳的更干净方案」，独立暴露无必要，ponytail：不为不存在的调用者加接口）。

### (b) 依赖「margin 分段快照」外部数据（编排者唯一在办供源项）

**数据需求规格**（供编排者供源时直接对照）：

- **对象**：真实交易所历史分级维持保证金表的**时间序列**（非当前单点快照）。
  - Binance：BTCUSDT 逐时间段的 `leverageBracket` 历史值（notional_floor/notional_cap/mmr/maint_amount 每档，随时间变化的版本）。Binance 官方 API（`GET /fapi/v1/leverageBracket`）只返回**当前**值，无历史端点——历史需交易所公告考古或第三方存档。
  - CME：BTC/MBT 历史维持保证金率（`cmegroup.com/.../bitcoin.margins.html` 历史修订记录）+ 对应生效区间。
- **接入格式**：直接对应已实装的 `MarginScheduleBook::new(snapshots: Vec<(Timestamp, Timestamp, MarginSchedule)>)`（`risk.rs:667`）——**代码侧零改动，供源后是纯数据替换**：把当前 `q4-fullpi-results` 用的单段 `MarginSchedule::cme_simple(0.37, 1.10)` 替换成多段真实历史表即可复跑。
- **当前替代方案（已诚实处理，非阻塞）**：`prereg-q4-fullpi-20260703.md` §④ 已声明「CME-simple 单段近似全历史」，并显式标注有效域限制（非 SPAN、非交易所逐段历史快照、非实盘保证金，一切含 margin 结论限于此口径）——这是 231 号（formalization-validity-domain）要求的诚实降级，**当前状态本身不违规**。
- **供源后验收判据**：重跑 `#135` 式的 Arm1 vs Arm3 对照（`wverify_run.rs` q4_fullpi_policy 四臂 harness），观察真实历史分段 MM 是否仍然零 binding。**不预判结果**——当前 sizing/nav 规模下 MM 从未被触及是否会因为真实历史某些时段 MMR 更高而改变，需要真实数据验证。

### (c) 依赖 IBKR/多品种接入 或 需要全新设计工作（M4 里程碑域，明确后置）

这一类内部需要再细分——并非全部都是「等 IBKR 接入」：

1. **ADL（自动减仓）机制建模**：
   - 性质：**可能结构性不可得**，不是「等编排者供源」能解决的数据缺口。Binance ADL 队列基于「盈利程度×杠杆」排队，交易所不公开每笔历史事件的队列位置数据（这与「MM 分级表」不同——MM 分级表是公开规则，ADL 队列位置是私有实时状态，无法历史复现）。
   - 诚实标注：若要建模，只能做「ADL 触发条件的规则近似」（如「持仓杠杆/收益率超过 ADL 阈值 → 标记为潜在 ADL 候选」），而非真实历史回放——这是**新设计工作**，需要走 margin-model-design 同款的 PDF 溯源 + codex 裁定流程，不是本次三分范围内的「可直接派工实装」项。
   - 当前 binding 判定：`#135` 已证「MM 从未 binding」→ ADL（比 MM 更极端的强平后手段）在当前 sizing/nav 规模下**更不可能 binding**，后置合理。

2. **资金费（funding rate）驱动强平建模**：
   - 性质：**数据本身公开可得**（Binance historical funding rate 有公开 API），但 margin-model-design v2 §1.3 目前**只列为诚实缺口，未给出任何具体建模公式**（funding 如何计入 E_t、多频次结算如何影响 liq_flag 均未设计）。
   - 因此这不是「(b) 类：数据到位即可对照插入代码」的状态，而是「需要新一轮设计文档 + codex 裁定，才能确定实装面」——按颗粒度应归 (c)，不归 (b)。
   - 当前 binding 判定：同 ADL，当前 sizing 下 MM 都不 binding，funding 对结果的影响预期更小。

3. **Portfolio margin / SPAN 全场景**：margin-model-design §1.3 已明确「L2 延后」，需要完整波动率组合场景引擎，超出 v0 范围，无争议。

4. **hedge 账户模型（双腿簿记）**：codex 裁定 (b) 默认 one-way/net，hedge 显式 opt-in——当前 `AccountState`（`strategy/mod.rs:119`）是单一 `voice_qty` 净仓骨架，hedge 需要额外双腿簿记结构，属于账户层架构扩展，非当前 v0 缺口（是主动裁定的范围收窄，非遗漏）。

5. **借券费/替代股息**：仅在标的扩展到股票短卖时 binding，当前 BTC perp/futures 下不适用，随「多品种接入」（M4）一并考虑，与 team-lead 原描述「接 IBKR/多品种前收口」精确对应。

---

## 2. 结果包（六要素）

1. **结论**：A10 三分——(a) 现在可实装类**为空**（margin-model-design v2 全部设计内容已在 `#113` 完整实装 + `#135` 完整跑批验证，M2/M3 接线不再是待办项）；(b) 依赖 margin 分段快照外部数据（编排者在办供源项）**仅剩一项**——真实交易所历史分级 MM 表时间序列，代码侧已就绪（`MarginScheduleBook`），供源后零改动可直接复跑；(c) 依赖 IBKR/多品种接入或全新设计工作，含 ADL（可能结构性不可得）、资金费建模（数据可得但需新设计）、SPAN/portfolio margin（L2 延后）、hedge 账户模型（主动范围收窄）、借券费（标的扩展后 binding）。

2. **定义依据**：`margin-model-design-20260703.md` v2 全文 §1-§7（设计规格）+ `codex-margin-ruling-20260703.md`（3 致命缺口 + 2 重要缺口 + 3 选择裁定，逐条核实其在 `risk.rs`/`coverage.rs`/`runner.rs` 中的实装落点）+ `q4-fullpi-results-20260703.md` §3.2/§4/§5（`#135` 生产回测中 margin 的实测 binding 判定与有效域声明）。

3. **边界条件（结论翻转条件）**：
   - 若真实历史分段 MM 数据到位后重跑 `#135` 式对照，发现某些历史时段 MMR 显著更高导致 margin 从「零 binding」翻转为「binding」→ 需要重新评估该结论对 alpha 冻结的影响（同 231 号：不同 π 口径 = 不同订单流，需重跑重冻结）。
   - 若编排者/团队决定 ADL/资金费建模优先级提升（不再等 IBKR/多品种）→ (c) 类中的 ADL/funding 两项可提前拆出，走独立设计工位（先出 PDF 溯源+设计文档+codex 裁定，再排实装），但**不能跳过设计阶段直接派实装**——当前无设计公式可依。
   - 若发现 ADL 历史队列数据实际可从第三方渠道获得（非官方 API，如做市商/数据商）→ ADL 从「结构性不可得」翻转为 (b) 类外部数据依赖项。

4. **下游推论**：team-lead 排工时，A10 应从任务列表中**拆分而非整体挂起**——(a) 为空意味着当前无需为 A10 单独派实装工位；(b) 唯一数据缺口应并入编排者现有的「margin 分段快照」供源渠道（若该渠道已在办，A10 无需新开供源请求）；(c) 五项应留在 M4 里程碑域，其中 ADL/funding 若要提前处理需先排一个「ADL/资金费建模设计」工位（产出物是新设计文档，非代码），而非直接假定「数据到位即可实装」。

5. **谱系引用**：formalization-validity-domain（231号）——(b) 类当前用 CME-simple 单段近似的有效域降级声明合规，供源后需重跑重冻结口径判定的依据；no-patch-mentality（090号）——(c) 类不允许在无设计公式的情况下用占位/近似「打补丁」式实装 ADL/funding，必须先走设计+裁定流程；675号/674号——本次核实未涉及三阶段账本/probe 路径，无新谱系张力。

6. **影响声明**：本工位纯只读界定，**未修改任何生产代码**，未修改 margin-model-design 文档本身。产出：本文件一份。下游消费：team-lead 据此更新 A10 在 gap-master 系列清单中的表述（M2/M3 接线项标记为已完成，剩余仅指向 (b)/(c) 两类）；若编排者的「margin 分段快照」供源到位，直接消费本文件 §1.(b) 的数据格式规格。
