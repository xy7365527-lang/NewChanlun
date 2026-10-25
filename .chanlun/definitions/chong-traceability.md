> 正本：本文件是 Chong 逐 bar 追溯的教义正本。
> 最后裁定：#1533（FU-10，2026-10-26 返修，沿用既有 T 类裁定）。
> 受影响代码清单：本文件「受影响代码与承接」节，定位基线 main `99df2ed762201236b89f0e81c0276cb3aa847f0a`。

# Classification、旁路、下钻与 Chong 的追溯合同

## 每 bar 的正式路径由 production_session 装配

Classification 是结构判定域的唯一输出。S 产生结构事实，重操作程序只读消费，不产生第二份 Classification，也不修改结构主干。职责依据为 [ADR 0027 裁定一、五](../../docs/adr/0027-classification-chong-operating-program-ownership.md) 与 `CONTEXT.md:9`。

唯一正式 launcher 是 `production_session`。它拉起独立的 S、E、各 Bᵢ 与 X，只负责装配、监督和换代，不持域写者身份，不产生交易意图，凭据只送入 X。[ADR 0039 裁定二、三](../../docs/adr/0039-production-entry-inventory-and-unique-launcher.md) 已将 `theta_backtest` 定为冻结影子，它只能提供组件证据。`run_theta_v0_pi` 不是正式 launcher 的替代链。

正式逐 bar 证据须能从原始输入和所用 cut 找到 S 的 Classification，再找到对应重的旁路读法、下钻定位、Voice、阶段事件史、筹码与毛账。身份、epoch、policy/model 版本、前后状态、撤回和续接的具体事件合同由 #1534 承接。本票不提前发明第二套事件 schema。

## 旁路和下钻只读主干

一重包含挂载层、横向读法、向下定位器和专属筹码。横向读法确定操作段，向下定位器确定进出场点。二者消费主干，结果均不回流主干，重内单向、重间不仲裁。依据为 [ADR 0011](../../docs/adr/0011-operation-decomposition-layer.md):92–95 与 ADR 0027:72–73。

已有 `operation_bypass_no_backflow_lock` 比较同输入在空挂载与挂载 `[0, 1]` 时的 Classification，且要求旁路确有产物。这是 #881 的旁路组件锁。它不覆盖正式 launcher 的进程装配、完整下钻定位或逐重毛账。正式路径仍须在 TB-02、TB-03、TB-04、TB-06 的验收中运行并出证，不能由本段文字替代。

## 阶段事件史逐重保存，方向属于在世 Voice

一重拥有自己的 TStage、Voice、专属筹码与毛账。重是持久运作单位，Voice 是一次建仓生命周期，Voice 终结不等于重终结。依据为 [ADR 0010](../../docs/adr/0010-chong-persistent-operating-unit.md):52、[ADR 0013 裁定二](../../docs/adr/0013-partition-machine-bidirectional-form.md) 与 ADR 0027:26、72。

TStage latch 只投影 campaign 内已经发生的阶段事件史。`phase_events` 追加写，AsKnown 不回填。当前扩量资格由账本事实计算，不读取 latch 充当准入条件。依据为 [ADR 0031 裁定三](../../docs/adr/0031-principal-batches-phase-predicate-corporate-actions.md) 与 [三阶段正本 §2](sanjieduan.md)。

多头事件可推进到 EarningShares。空头只有降成本与退本金两相，空头事件不把 latch 推进到 EarningShares。若同一重此前已到 EarningShares，换成空头 Voice 后仍保留原阶段史，不能强制退回 CapitalRecovered。空头的资格蕴含为 `growth_eligible ⇒ latch.rank ≥ CapitalRecovered`。开多 Voice 的求值点若资格为真而 latch 尚未到 EarningShares，则按既有 LegalTransition 追加 enterEarning 事件。以上直接沿用 [ADR 0043 裁定三](../../docs/adr/0043-chong-reversal-account-continuity.md)，不新设阶段。

换向属于同一重的 Voice 换代。P、K、R、费用、phase_events、G 与 withdrawn 连续，不重置、不按方向拆账，也不因换向新增本金批次。本金只能由已授权的 principal_contribution 等既有合法来源形成。具体资金政策仍按 ADR 0031、ADR 0043 及已批准记录执行。

## Voice、筹码与毛账保留逐重归属

每重可先后拥有 Voice，当前 Voice 的标识按 `CONTEXT.md:89` 的 `(carrier, γ, σ, n)` 保存。追溯必须区分重身份与 Voice 生命周期，不能拿 Voice 的退出清空重账。

逐重毛账按腿记录，不以净额代替。毛暴露是两腿绝对值之和，子对冲额度受父级短差额度约束。零筹码期间仍保留金额账和阶段事件史。依据为 `CONTEXT.md:427、431`、ADR 0027:26 与三阶段正本 §4。

## 受影响代码与承接

以下行号固定在 main `99df2ed762201236b89f0e81c0276cb3aa847f0a`。清单标识需要对齐或验证的边界，不宣称这些组件已完成正式全链。后续实施按当时 head 重新定位同名符号。

| 侧 | 文件与行 | 影响及现有承接 |
|---|---|---|
| Rust | `rust/src/theta_v0/classifier/pipeline.rs:116、552、623` | Classification 与批量、增量分类入口。S 结构事实只读边界和逐 bar 关联归 TB-02 #1360、FU-11 #1534。 |
| Rust | `rust/src/theta_v0/classifier/operation.rs:64、84、694、711` | 操作序列及增量旁路。主干不回流和完整定位的正式验收归 #1360、#1364。 |
| Rust | `rust/src/theta_v0/classifier/tests/operation.rs:102–117` | 已有旁路组件锁的准确覆盖面。#1360 扩展到正式程序证据时保留此锁，不用它代替端到端。 |
| Rust | `rust/src/theta_v0/strategy/chong.rs:37、48、102`；`rust/src/theta_v0/strategy/voice.rs:138`；`rust/src/theta_v0/strategy/ledger.rs:94` | 旧组件中的 ChongKey、ChongBook、VoiceState 和 latch 镜像。对齐持久 Bᵢ 时保留正确语义，不给影子链恢复生产写权。归 #1361、#1534。 |
| Rust | `rust/src/bin/s_structure_session.rs:27–28、5557`；`rust/src/bin/b_chong_session.rs:2–3`；`rust/src/economic_session/service.rs:1480` | S 组件入口与 Bᵢ 独立 owner。正式装配归 #1523，输入侧恢复归 #1527，逐 bar 身份与状态链归 #1534、#1362。 |
| Rust | `rust/src/economic_session/book.rs:80、87、97、387、405`；`rust/src/economic_session/tb03c_book.rs:366–373` | 目标重、事实键、解码、账本应用与当前资格计算。逐重金额及阶段史、换向连续性归 #1406、#1361、#1364；事件外壳归 #1534。 |
| Rust 新文件 | `rust/src/bin/production_session.rs`、`rust/src/bin/x_execution_session.rs` | 该基线尚无这两个文件，也无 production_session Cargo 项，不能编造行号。ADR 0039:266–288 已指定由 #1523 交付，#1536、#1538、#1550 消费正式 launcher。 |
| Lean | `formal/Origin/TotalWealth.lean:66、140、194、212` | TStage、advanceTo、阶段单调性及 earning_no_regress。现有定理不能当作全部多空资格关系证明。#1406 按三阶段正本 §2 声明模型覆盖面及 Rust 对拍义务，#1361 验收。 |
| Python | `trading_system/live/runner.py:31`；`trading_system/strategy/theta_rec_strategy.py:73、123` | 现有 Python 运行与发送路径只用于旧入口终态核查，不能成为第二个正式 launcher 或决策主干。终态实施归 TB-10 #1368，三模式同核与外壳接入归 #1538、#1550。 |

定位方法为先查询代码图中的 Chong、TStage、production_session，再在图缺项时检索 `rust/src/` 的上述符号、`formal/Origin/TotalWealth.lean` 的阶段定义及 `trading_system/` 的 `on_bar`、`submit_order`、`main`。`git ls-tree` 核新文件不存在，`git grep` 核 Cargo 尚无 launcher 项。没有新增 Lean 或 Python 判据；它们列为覆盖面与旧入口核查对象。

## 后续交付与本票关票分开验收

本票交付正确落文的 T 类技术合同、受影响代码清单和真实承接关系。文书验证不证明整图行为已完成。八条既有原生 blocking 边全部保留，合法关闭 #1533 后由依赖状态自然满足，不删除边。

| 承接票 | 必须消费的本票合同 |
|---|---|
| #1534 | 用逐 bar 事件关联 cut、身份、epoch、policy/model 版本及前后状态；覆盖重与 Voice 分离、阶段史、筹码、毛账、撤回与续接。 |
| #1536 | 完整输入与模型 profile 经 production_session 运行；保留结构、资金、订单、成交、持仓和故障，不用影子 CLI 代正式路径。 |
| #1538 | 三模式保持同一结构与经营核心，只替换已裁 adapter；同输入核对本票的逐重语义结果。 |
| #1550 | #1304 外壳调用正式 launcher，输出逐重状态与毛账、恢复和 AsKnown 证据，完整 AT-14 报告。 |
| #1360 / TB-02 | 正式逐 bar Classification、旁路与下钻关系及不回流证据。 |
| #1361 / TB-03 | 持久 Chong、Voice、阶段事件史、筹码与毛账的逐重独立、换向连续和恢复证据。 |
| #1362 / TB-04 | 正式 S/E/Bᵢ/X 调用、成交回报与账本链路证据。 |
| #1364 / TB-06 | 按多空及换向条件核阶段、当前资格、完整经营周期与尾账。 |

关本票前核对以下四项。实施与整图验收仍留在上述承接票。

1. 非作者审查本文件对 ADR 0039、ADR 0043、三阶段正本及代码定位的引用，结论为 PASS。
2. `git diff --check` 无错误，三行头齐全，清单路径在注明基线可定位，新文件明确未实现。
3. `gh api repos/xy7365527-lang/NewChanlun/issues/1533/dependencies/blocking --jq '.[].number'` 保留上述八票；各票具有本合同承接评论。
4. 用户批准后将精确评审 head 合入 main，记录 commit、CI 与评审路径，再发布修订 resolution。关闭状态是结果，不作为前置证据。

## 考古编号保持同一含义

| 点 | 问题 | 归类与已裁依据 |
|---|---|---|
| Q1 | 每 bar Classification 路径 | A，ADR 0027、ADR 0039，唯一 launcher 为 production_session。 |
| Q2 | 定位到每重的路径 | A，ADR 0011:95、ADR 0013 裁定七、ADR 0027:73。 |
| Q3 | Chong 状态追溯 | A，ADR 0010:52、ADR 0031、三阶段正本 §2。 |
| Q4 | 逐重毛账追溯 | A，ADR 0027:26、CONTEXT.md:427、431。 |
| Q5 | 旁路不回流机制 | A，ADR 0011:92、ADR 0027:73；#881 仅提供已述组件锁。 |
| Q6 | TStage 独立性 | A，ADR 0013 裁定二、三阶段正本 §2、ADR 0043。 |
| Q7 | Voice 独立性 | A，ADR 0010:52、CONTEXT.md:89、ADR 0027:26。 |
