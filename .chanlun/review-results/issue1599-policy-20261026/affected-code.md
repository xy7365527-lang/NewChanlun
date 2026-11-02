# #1599 受影响代码与承接

基线：GitHub main 与本地 origin/main 均为 `13ed4d45c8ff85175819ca987690328ab2a247a4`（2026-10-25）。下表是本票字段的当前接缝，不把新增字段虚构成已有实现。原文摘录见 `evidence/code-excerpts.txt`；查询见 `evidence/field-search.txt`。

| 条目 | Rust 接缝 | Python 接缝 | Lean 接缝/缺口 | 承接状态 |
|---|---|---|---|---|
| A-04 扩量门 | 新字段 `expansion_gate/SR-5` 在 rust/src 零命中；当前责任基座 `economic_session/finance.rs:1081-1097`，不是现成扩量门 | 新字段在 trading_system/s_session/src/tests 零命中；旧 RiskGate 不自动继承为 SR-5 | formal 零同名字段；本票不改教义定义 | 待开具名 SR 新版本消费实施票；与 #1380（OPEN，退出/冻结）协调。#1525旧RiskGate四条不能直接充当 SR-5 实现 |
| A-05 fill_rule | `theta_v0/backtest/fill.rs:119` 是按给定 px 入账，不是触及判据；新字段在 rust/src 零命中 | `trading_system/backtest/runner.py:60-61`、`backtest_unn.py:234-235` 为现行穿越装配 | formal 零同名字段；诊断撮合选择不改形式化定义 | 待开诊断 fill_rule 实施票，生产/验收始终 cross；不得用 fill.rs:4604 诊断输出冒充成交判据 |
| A-06 profile | `economic_session/tb03c_funding.rs:21-50` 为具名profile绑定；非新场所adapter | `trading_system/config/broker_config.py:33-45` IBKR paper配置、`:48-82` HL环境装配 | formal 零本票字段；账户/场所授权不改Lean | #1382/#1523 OPEN，仅已有共核adapter/HL范围；D IBKR/E Binance新适配需待开或明确扩入真实接单票 |
| A-07 observation_period | 字段在rust/src零命中；`economic_session/book.rs:334-346` 是利润费用见证校验，不能作退休接缝 | 同名字段在Python上述四目录零命中 | formal零同名字段 | #1380 OPEN可补观察期承接条目；#1397 OPEN负责迟到尾账。当前票尚未显式接本新增字段，须主控回填 |
| A-08争用/取出 | `economic_session/finance.rs:44-45,206-238,972-987,1081-1097`：政策id、校验、防插队与cap判定；两新字段零命中 | `s_session/tests/tb03b_oracle.py:48-49,64-68`固定现行政策和等待oracle | formal零同名字段 | #1408 OPEN，其正式政策范围适配；需显式接两字段。#1445仅消费换档，不替政策实现 |
| A-11完成锚 | `theta_v0/classifier/nest_lifecycle/book.rs:219-222,636-656`；`feed.rs:218-228`；`observation.rs:45-76`。配置字段零命中 | completion_anchor在Python上述四目录零命中 | formal零同名字段；中枢定义不改，接桥如新增形式化义务由实施列明 | 待开中枢生命周期到活假设账本桥接票；不拿 #1429 替代 |
| B-09分红 | `economic_session/tb03c_book.rs:23-57,178-196,318-327,522-523,787-791`：净额、税、K、资本化、资格门 | dividend_treatment在Python上述四目录零命中，无该备选分派 | `formal/Origin/CompleteStateEvent.lean:320-326`有dividend原件，不实现两种K政策；新字段零命中 | #1378 OPEN覆盖公司行动/阶段，可补两policy；#1397/#1421只承担尾账/Voice影响，不替新增分派 |

全部 Rust 路径以上以 `rust/src/` 为前缀。逐语言零命中方法：`git grep -n -E 'expansion_gate|SR-5|fill_rule|observation_period|on_contention|withdraw_rule|completion_anchor|dividend_treatment' origin/main -- <域>`，域为 rust/src、formal、trading_system、s_session、src、tests；完整结果为空。本断言只针对精确字段，不推断不存在同义机制。

附带消费者：`s_session/browser/tb03a-client.js:181,406` 固定政策与等待显示（复用既有补件核查，实施前需对应新版本再核）；前端产品交付仍归 #1460。

`evidence/receivers.jsonl`保存9张候选承接票完整正文和OPEN状态。OPEN和主题相近不等于已接新增义务：本轮未改下游票，主控须补承接条目或建票后逐项回填；未补齐不得关 #1599。#1560/#1561/#1394已关闭，不承接新增实现；#1699/#1700不作预留编号。
