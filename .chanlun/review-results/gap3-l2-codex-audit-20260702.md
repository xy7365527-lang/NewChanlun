# Codex 异质审计：GAP3 引擎不消费价格幅度——定义冲突 vs 实装缺口

审计节点：codex-challenger（工位 ws-codex-gap3，task #9）| 日期：2026-07-02
被审计对象：`.chanlun/review-results/gap3-l2-reachability-20260702.md`（ws-gap3-l2 / task #6 产出）
Codex 原始交互记录（prompt+response 全文）：`.chanlun/review-results/codex-diagnose-20260702-0214.md`

## 判决摘要（一句话）

**CONFIRM_GAP**——这是实装缺口，不是 576 号意义上的定义冲突。我独立阅读代码得出的诊断链与
Codex 独立诊断（未预先告知我的倾向）**完全收敛**：三处丢弃点定位准确、价格幅度不变性测试
有效、576 号"R vs TW 不同构"与本次"TW 内部是否消费价格"是两个正交问题、补桥不能改写
`ShortDiff` 现有语义（会使已证明定理变假），须新增非对称转移事件并为其单独证明新引理。

## 六要素结果包

### 1. 结论

GAP3 三阶段引擎的 `EarningShares` 在真实 BTC 461 万 bar（L2 变价数据）上 count=0，根因是
**实装缺口**：真实价格幅度在到达 TW 账本之前，经过三个连续的结构性丢弃点被完全清除，导致
461 万根真实 K 线的价格波动对 `TW=free+holding+withdrawn` 的净效应恒为零。三处丢弃点（逐行
核实，Codex 独立复核确认无误）：

- **丢弃点1**（`runner.rs:823`）：`let rising = bar.close >= prev_close;`——真实价格 `bar.close`
  （f64）被比较运算符投影为 1 个 `bool`，数值本身此后不再出现在事件流中。
- **丢弃点2**（`state.rs:54`）：`MicroEvent::NewBar(bool)`——事件类型本身只有 1 个 bit 的信息
  容量，不是"漏传参数"，是事件字母表的构造子签名本身无处承载幅度。
- **丢弃点3**（`transition.rs` `schedule_adapter`，约149-177行）：只用"单位数增量" `filled_delta`
  派生 `TwEvent::ShortDiff(-filled_delta)`，从未按真实成交价换算现金流，也从未派生 `TwEvent`
  枚举中唯一携带 `profit` 字段的 `CloseShareLeg` 构造子。该函数自身文档注释承认"具体金额=
  单位数·price 是...L2，price 由下游 fill 侧填充"——设计者知道需要价格桥，但这座桥从未建成。

`ledger.rs` 逐行确认了为什么即使丢弃点1/2 被修复，丢弃点3 仍会挡住 TW 增长：`ShortDiff(d_cash)`
的状态转移是**结构对称**的（`free += d_cash, holding -= d_cash`，同一个 `d_cash`）——这是该
构造子的定义本身，不是"同价特例"，无论 `d_cash` 代表单位数还是真实成交金额，TW 恒守恒，
永远无法表达"卖出价高于成本"的净利润。唯一携带利润字段的 `CloseShareLeg(profit)`，其状态
转移函数把 `profit` 显式路由到与 TW 三量正交的 `cum_net_cash`（`free/holding/withdrawn` 均
不变）——这也是设计者的明确意图，不是遗漏。

Codex 额外指出一处我审计时未单独点名的**维度错误**（补充发现，未改变最终判决）：
`schedule_adapter` 中 `affordable = requested_delta.min(x.tw_state.free.max(0))` 把"仓位单位数
增量"与"现金 `free`"直接取 min 比较——这隐含假设"单位价格恒为1"，一旦引入真实价格，
买入上限应改为 `free_cash / fill_price` 而非 `Δ.min(free)`，这是补桥时必须一并修正的维度
一致性问题，而非独立缺陷。

### 2. 定义依据

- **缠师第31课「资金管理的最稳固基础」**（`docs/chanlun/text/blog/031-第31课.md`，一级权威）：
  "股票开始上涨后，一定要找机会把股票的成本变成0...还要在股票达到1倍升幅附近找一个大级别的
  卖点出掉部分，把成本降为0。这样，原来投入的资金就全部收回来了。"——**"退本金"在定义上要求
  卖出价高于买入价**（"1倍升幅"）。若买卖同价，"出一半后成本就是0"在数学上不成立。这确认了
  L0 同价数据下 EarningShares 不可达是定义的**正确结构后果**（acc 报告已证，非本次待裁问题）；
  本次待裁问题是"在价格确实上涨的真实 L2 数据下，为什么代码仍然不可达"——第31课原文不涉及
  这一问题，它只确立了"退本金必须依赖真实涨幅"这一前提，本次诊断的落点是**代码是否让这个
  前提有机会被满足**（结论：没有，因为价格幅度在到达账本前已被清除）。

- **576号谱系**（`.chanlun/genealogy/pending/576-...md`）：TW 模型（模型B）自身声明
  `holding=Σunits·c` 依外生市价 c（跨 bar 变动=盈亏）——这是模型 B 定义的一部分，本次诊断确认
  **这部分声明从未被实装**（`holding` 当前是扁平标量，无逐单位成本结构，也从不随外生价格重估）。
  576 号讨论的"R 账本 vs TW 账本能否合一"是两个账本之间的关系问题；本次诊断的"TW 账本自己
  内部是否消费真实价格"是 TW 账本单方面是否完整实装了自己声明语义的问题。Codex 独立确认
  ("这不是 R 账本与 TW 账本能否同构的问题")这两个问题正交——576 的不同构裁定不阻塞、也不
  被本次补桥影响。

- **溯源纠正**（本次审计独立发现，非 Codex 提出）：`acc-gap3-earningshares-reachable-20260701.md`
  第42行把"降成本=短差（买卖等量）"的依据引用为"缠师第17课"。经核实 `docs/chanlun/text/blog/
  017-第17课.md` 原文内容是"走势终完美"（中枢/趋势/盘整定义、走势分解定理），完全不涉及降成本
  或资金管理语义。正确出处是第31课（576号谱系文件本身已正确引用第31课；只有 acc 报告的引用
  有误）。这是一处定义忠实度问题，建议 acc 报告后续订正引用，但不影响 acc 报告已证明的 L0
  不可达结论本身（该结论的实质论证不依赖具体课号引用，只依赖"降成本需买卖等量"这一语义，此
  语义在第31课原文中同样成立："成本为0前，只补进相同的数量，仓位不增加"）。

### 3. 边界条件（结论翻转条件）

本判决（CONFIRM_GAP）在以下条件下会翻转为 CONFIRM_CONFLICT：

- 若"TW 守恒"被要求是一个**对所有可能新增事件都必须成立**的全局不变量（即不允许引入任何
  能合法改变 TW 的新事件类型），则本次问题无解——因为让 TW 反映真实盈利在数学上要求某个
  事件的 `free` 增量与 `holding` 减量不相等，这与"处处守恒"矛盾。但 576 号谱系与 ledger.rs
  代码注释均未做出这一全局要求——现有 `tw_step_preserves_tw` 定理明确只覆盖当前六个 `TwEvent`
  构造子，新增第七个构造子（非对称转移）在数学上与该定理共存，不需要否证或修改它。若日后
  发现某处已结算定义**明确禁止**新增非守恒事件类型（本次审计未找到这样的定义），则本判决翻转。

- 若价格幅度不变性对照测试存在我与 Codex 均未察觉的构造漏洞（例如全仓库存在某个绕过
  `MicroEvent`/`run_closed_loop` 直接读取 bar 价格的旁路通道），三处丢弃点的"完全清除"论断
  会被削弱。Codex 与我均未能排除这种旁路的可能性（本次审计范围内的静态代码阅读 + 已有测试
  证据不能覆盖全仓库搜索）——若后续 code-verifier 或全仓库 grep 发现这样的通道，需要重新评估。

### 4. 下游推论

- 补桥方案确定为：(a) 扩展 `MicroEvent::NewBar` 携带真实价格（或新增独立 `PriceMark`/`Fill`
  事件）——结构性变更 `state.rs` 的事件字母表；(b) 在 TW 状态中拆分"单位数"与"价值"，至少
  跟踪 `position_units`/`last_price`/`holding_value`，若要精确计入已实现利润还需 `avg_cost`
  或逐 lot 成本结构；(c) 修正 `schedule_adapter` 的维度错误（现金约束应为 `free_cash /
  fill_price` 而非 `Δ.min(free)`）；(d) 保留现有 `ShortDiff` 为守恒事件不变，新增一个非对称
  转移的价格重估/成交事件，并为它单独证明一条新引理（不复用 `tw_step_preserves_tw`，也不
  修改其覆盖范围）。这是一个跨 `state.rs`/`transition.rs`/`ledger.rs` 的结构性工作量，不是
  局部 patch，建议作为独立工位（非 escalate）分派。
- acc 报告第42行的溯源标签建议订正为第31课（不影响其已证明的 L0 结论）。
- 576号谱系的三选一（A/B/C）产品级决断**不受本次结论影响**——本次审计确认两个问题正交，
  576 号仍待编排者对 A/B/C 表态。

### 5. 谱系引用

- `.chanlun/genealogy/pending/576-ledger-r-vs-tw-three-stage-semantic-alignment.md`：R vs TW
  不同构（machine-checked），本次诊断确认其与"TW 内部价格消费缺口"正交，不互相阻塞。
- memory `project_gap3_l0_earning_unreachable`：三致命=Δ驱动病理；退本金需已实现利润(L2)；
  机制正确但 L0 同价不触达（本次 L2 审计确认该记录在"价格幅度未被消费"这一新维度上仍然成立
  ——即使切到 L2 真实数据，若不补桥，结论不变）。
- `.chanlun/review-results/acc-gap3-earningshares-reachable-20260701.md`：L0 结论 + 第42行
  溯源引用需订正（第17课→第31课）。
- `.chanlun/review-results/gap3-l2-reachability-20260702.md`：被审计对象，三处丢弃点诊断经
  本次独立核实+ Codex 独立诊断，结论一致（无反驳）。
- `formalization-validity-domain.md`（231号规则）：本次结论是又一例"有效域 < 定义域"——
  TW 模型声明的语义（`holding=Σunits·c` 依外生价格）定义域覆盖 L2/L3，但当前实装的有效域
  仅覆盖 L0（同价）。

### 6. 影响声明

本工位（codex-challenger）仅执行独立代码核实 + 调用 Codex 异质诊断 + 落盘审计报告，**未修改
任何生产代码/测试**。涉及模块（供后续补桥工位参考，非本次改动）：`rust/src/theta_v0/closed_loop/
state.rs`（`MicroEvent` 枚举）、`rust/src/theta_v0/closed_loop/transition.rs`（`schedule_adapter`/
`stage_progression`）、`rust/src/theta_v0/backtest/runner.rs`（`run_closed_loop` 的 rising 投影）、
`rust/src/theta_v0/strategy/ledger.rs`（`TwEvent`/`tw_step`/`tw()`）。Codex 原始交互记录已由
CLI 自动持久化到 `.chanlun/review-results/codex-diagnose-20260702-0214.md`（304行，prompt+
response 全文）。
