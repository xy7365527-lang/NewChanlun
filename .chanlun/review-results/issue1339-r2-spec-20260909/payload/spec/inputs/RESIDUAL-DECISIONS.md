# #1323 SPEC 残留决定核查（2026-09-09）

状态：**只读核查完成；不是残留裁定、SPEC 批准或当前运行验收。** 用户已采纳 R2；本件只覆盖 G-001/002/007/025/026/028/029 与 FU-01/02/03/05/07，K4 不进入结论。未修改仓库/GitHub，未跑程序/测试/探针，未读取禁列旧 #1321/#1322/#1325/#1326 方案审计原件。

当前最重要的差异是 **G-029 有后裁**：[#907 Resolution](https://github.com/xy7365527-lang/NewChanlun/issues/907#issuecomment-5194145006) 将阶段二第一层由“延伸前算出最低级上限”收窄为“给出当前 L0 余量，延伸后立即复测”，并裁定 ROUNDTRIP 配对。qujiantao.md:507 旧文字尚未同步。G-007/G-025/G-028 的具体残留尚未找到明确后裁，不能由 closed 票状态推断闭合。

## 核查方法与界限

- 起点：[CLASSIFICATION-CATALOG.md](/tmp/newchanlun-1339-architecture-20260908/author/CLASSIFICATION-CATALOG.md:1475) 与当前 [FUGUE.md](/tmp/newchanlun-1339-requirements-20260908/author/parts/FUGUE.md:689)。
- 两路均已查：GitHub `gh issue list --state all` 精确词与别名；非代码 `docs/ analysis/ .chanlun/` 的 `rg`，按正本/ADR/推论层区分名分。具体检索式及最相关命中在配套 JSON。
- 针对 #813/#817/#847/#907/#915/#836/#1050/#659/#647 的相关正文/评论行回核，不读取整套旧设计。
- 这是有界关键词查证；“后续已裁未证”表示本次未取到明确裁定，不等于已穷尽全仓所有别名。历史文档中的生产状态、运行数字不被重报为当前事实。

## 结构残留

### G-001：SPEC工程实例待明确

**已有**：方向性包含规则与顺序已定义；dir 未建时默认 UP 是已有工程处理，不是原文唯一方向。

**仍需**：须明确合法前缀如何建立初始方向、全包含前缀如何处理及输出影响。

**依赖切片**：原始输入到包含组的构造；跨语言映射与全前缀确定性。

**SPEC 可提出**：具名并版本化初始化策略；用首个非包含方向建立方向或显式声明默认方向的完整分支；具体方案随 SPEC 提审；保留方向未建立状态和原始前缀证据。

**边界**：不能把默认 UP、Python close/open 推断或任一既有实现自动升为唯一教义。

来源：[.chanlun/definitions/baohan.md:117](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/baohan.md:117)，[.chanlun/definitions/baohan.md:141](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/baohan.md:141)。

### G-002：身份/证明实例待明确

**已有**：取最高/最低原始 K 的语义及原始序号—组锚关系已定义；组锚是组内首根，与极值根不同。

**仍需**：同价多根极值、同型同价端点、组锚/端点/对象身份必须给出一致确定选择，不能由价格唯一推导 raw 身份唯一。

**依赖切片**：原始/合并映射；笔和全部结构对象身份；按当时可知回放。

**SPEC 可提出**：以原始序号组成稳定 tie key，明确选择与输出影响；同输入同 policy 得到同身份的验证义务。

**边界**：稳定 ID 编码本身不能证明端点选取语义或确认前缀不回退。

来源：[.chanlun/definitions/bi.md:163](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/bi.md:163)，[.chanlun/definitions/bi.md:168](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/bi.md:168)，[.chanlun/definitions/bi.md:391](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/bi.md:391)，[.chanlun/definitions/baohan.md:143](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/baohan.md:143)。

### G-007：明确未裁；后续已裁未证

**已有**：P2 特征序列语义已裁；G-5b TAIL_WINDOW=7 撤票不涉及本项。

**仍需**：第二特征序列 50 vs 完整扫描仍未取得后裁；50 曾兼 checkpoint margin，改动影响不能省略。

**依赖切片**：第二特征序列终结判断；增量扫描/checkpoint/resume；完整结构生产验收。

**SPEC 可提出**：扫描预算、continuation cursor、待继续原因、checkpoint 安全边界可以提方案；完整扫描范围与调度预算分开表述；预算耗尽不得写成结构否定。

**边界**：本次精确词、别名两路查重未找到具体后裁；不声称全仓绝无别名后裁，不凭历史代码的生产标签认定当前生产状态。

来源：[.chanlun/definitions/xianduan.md:182](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/xianduan.md:182)，[.chanlun/definitions/xianduan.md:191](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/xianduan.md:191)，[.chanlun/definitions/xianduan.md:421](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/xianduan.md:421)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/813#issuecomment-5196921999)。

### G-025：明确教义选择未裁；后续已裁未证

**已有**：后代递归极值链中价格包含恒真有有效域；独立选父的 NestInterval 链不在该域内。

**仍需**：独立选父跨级价格应挂哪套价格序列/区间仍未裁；price-subset 的证明和实测都依赖它。

**依赖切片**：独立选父 NestInterval 价格投影；跨级 price-subset 证书；该路径完整准入验收。

**SPEC 可提出**：携带尚未选择的 policy 引用、所需输入价格证据和比较接口；列候选挂法及验证义务供明确裁定。

**边界**：不能把共用同一原始价格序列当作已裁；真实 descendant 的 min/max 单调性不能跨域。

来源：[.chanlun/definitions/qujiantao.md:309](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:309)，[.chanlun/definitions/qujiantao.md:1254](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:1254)，[.chanlun/definitions/qujiantao.md:1317](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:1317)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/817#issuecomment-5193950348)。

### G-026：候选/选择器工程实例与证明待明确

**已有**：元素谓词 Dir∧Comparable∧Extreme；条件候选集 C(J,t) 非空与 Sel 是同一集合上的不同算子；力度不进候选身份；Sel 属选择。

**仍需**：具体对象投影、候选全集、确定 Sel 与 selected 后门的完整实例未由本轮核查证明。

**依赖切片**：候选全集构造；区间套选父与 selected gate；候选观察/归因。

**SPEC 可提出**：全集保留与条件过滤清单；具名版本 Sel 全序及来源证据；集合非空、唯一选择、选中失败、不适用等结果分别承载。

**边界**：不得遇 selected 失败重选，不得以全候选真值观察代替 selected gate；旧 endTime/startTime/idx 可作工程选项，不是原文唯一规则。

来源：[.chanlun/definitions/qujiantao.md:733](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:733)，[.chanlun/definitions/qujiantao.md:761](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:761)，[.chanlun/definitions/qujiantao.md:813](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:813)。

### G-028：不适用域及三值承载已裁；递归证书处置未裁

**已有**：Type1 有效域与 Type2/3 上级 rung N/A 已裁；cand bool 升三值、depth 不冒充判据深度的方向已裁。

**仍需**：n_delta_rec 如何解释 N/A 仍为正本明示的单独裁定项，尚未取得后裁。

**依赖切片**：递归 NestCertificate 判定；Type2/3 上级 rung 与深度解释；Type2/3 端到端准入验收。

**SPEC 可提出**：ApplicableTrue/ApplicableFalse/NotApplicable 与原因来源的类型表达；分别记录判据深度和几何链长；将组合规则待裁精确列为依赖。

**边界**：不能将 N/A 填 true，也不能填 false 后翻掉全部 Type2/3；本报告不选择 skip/三值传播/条件证书等合成语义。

来源：[.chanlun/definitions/qujiantao.md:1084](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:1084)，[.chanlun/definitions/qujiantao.md:1106](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:1106)，[.chanlun/definitions/qujiantao.md:1108](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:1108)，[.chanlun/definitions/qujiantao.md:1228](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:1228)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/817#issuecomment-5186615398)。

### G-029：有后裁须同步；当前成本 policy 实例及进阶证据仍待

**已有**：成本门与塔底谁先到谁停止；不得强制降到底；阶段二方向已批准但有条件。#907 后裁确认 ROUNDTRIP，并把第一层从延伸前算出上限改为当前 L0 余量＋延伸后立即复测。

**仍需**：具体 fee/slippage/latency/平均波幅和阈值 policy 的现行实例、足够不确定性证据，以及第二层实际到底失败分类证据，未取得当前验证。

**依赖切片**：下钻成本停止门；操作级别与容量测量；阶段二基底延伸验收。

**SPEC 可提出**：版本化成本 policy 与输入来源；缺参/证据不足独立状态及当时可知回放；以最新 #907 条款表述第一层；保留第二层到底证据义务。

**边界**：#907 历史是 BTC/1m、2 个重叠前缀窗、置信区间未算、真实误差只有代理下界、未计 holding costs；不能作为当前执行条件已满足或任意下扩的证明。

来源：[.chanlun/definitions/qujiantao.md:408](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:408)，[.chanlun/definitions/qujiantao.md:443](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:443)，[.chanlun/definitions/qujiantao.md:477](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:477)，[.chanlun/definitions/qujiantao.md:507](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/qujiantao.md:507)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/907#issuecomment-5194145006)。

## FU 已裁约束与已有承接

### FU-01：核心会计口径已裁；细节政策未证

**原样继承**：成本基历史买入成本扣真实已实现回收，手续费与资金费率入成本；市值仓位和零成本基不同；阶段目标不削减。

**具体残留**：保证金/现金/增长权益之间本金精确定义；分红除权；费用跨零后的阶段回退；零数量与零成本分母。

**票据复用**：[#836](https://github.com/xy7365527-lang/NewChanlun/issues/836)（CLOSED）：成本与费用口径原样继承，分红除权明确遗留；[#1339](https://github.com/xy7365527-lang/NewChanlun/issues/1339)（OPEN）：当前 FU-01 残留归入已签需求，不新开票。

**依赖切片**：重的经济账本与阶段迁移；本金回收/零成本/增长；经营计量。

**SPEC 可提出**：保存数量、现金流、费用、回收、权益、公司行动原始事实；policy 引用及未知原因可观测；账务变动和阶段判定分开。

**边界**：无明确后裁证明这些细节已定；不得默认 0、默认公式或沿旧 campaign 实现代裁。

来源：[docs/adr/0024-total-ledger-convention.md:27](/Users/silencehan/Projects/NewChanlun/docs/adr/0024-total-ledger-convention.md:27)，[docs/adr/0024-total-ledger-convention.md:32](/Users/silencehan/Projects/NewChanlun/docs/adr/0024-total-ledger-convention.md:32)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/836#issuecomment-5324008174)。

### FU-02：量层形状已裁；完整参数与增长衔接未证

**原样继承**：动作量 w(d)×固定 Q、容量比公式与容量/频率分维、空仓不挪钱、增长撞容量走换档已裁。

**具体残留**：完整高档有效量、绝对容量、投入总额、经验更新、固定 Q 约束下增长换档的会计及权利分拆。

**票据复用**：[#835](https://github.com/xy7365527-lang/NewChanlun/issues/835)（CLOSED）：ADR0016 已裁形状；[#914](https://github.com/xy7365527-lang/NewChanlun/issues/914)（CLOSED）：ADR0017 已裁动作量及有效域；[#915](https://github.com/xy7365527-lang/NewChanlun/issues/915)（CLOSED）：窄域底档/容量/频率探针；不是完整高档政策；[#924](https://github.com/xy7365527-lang/NewChanlun/issues/924)（CLOSED）：仅旧 sizing 注释的具体承接，不闭增长衔接；[#1339](https://github.com/xy7365527-lang/NewChanlun/issues/1339)（OPEN）：FU-02 当前残留。

**依赖切片**：动作量与容量策略；固定 Q 权利账；经营增长、换档和新重。

**SPEC 可提出**：固定 Q 及增长权益分别记账；policy 版本、域、有效时间和分拆凭证；完整换档协议的接口与不变量。

**边界**：ADR0017 的历史约 0.32 限 1m/δ=0.5/中位数/底档；不外推到全高档、tick 或当前资金参数，不重新打开固定 Q 边界。

来源：[docs/adr/0016-chong-quantity-layer-capacity-frequency-shift.md:118](/Users/silencehan/Projects/NewChanlun/docs/adr/0016-chong-quantity-layer-capacity-frequency-shift.md:118)，[docs/adr/0016-chong-quantity-layer-capacity-frequency-shift.md:125](/Users/silencehan/Projects/NewChanlun/docs/adr/0016-chong-quantity-layer-capacity-frequency-shift.md:125)，[docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:134](/Users/silencehan/Projects/NewChanlun/docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:134)，[docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:136](/Users/silencehan/Projects/NewChanlun/docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:136)，[docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:439](/Users/silencehan/Projects/NewChanlun/docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md:439)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/915#issuecomment-5209250256)。

### FU-03：毛净约束已裁；资金排序/净额外效分配未证

**原样继承**：重间不仲裁方向；共同毛约束；净省禁复用；总账毛净帽与逐区约束各有用途；净额验收和毛账归因须随执行形态。

**具体残留**：同时超共同上限的资金排序、净额外效/费用分摊、部分成交归属、公用资金注入退出及 cap 参数。

**票据复用**：[#836](https://github.com/xy7365527-lang/NewChanlun/issues/836)（CLOSED）：共同上限与账目分工；cap 参数仍下游登记；[#1050](https://github.com/xy7365527-lang/NewChanlun/issues/1050)（CLOSED）：对账形式化承接可作既有证明导航；未查当前代码/运行，且评论存在历史重写重投记录；[#1339](https://github.com/xy7365527-lang/NewChanlun/issues/1339)（OPEN）：FU-03 当前残留。

**依赖切片**：共同资本与资源准入；净额执行/分配回执；费用、部分成交和公用资金。

**SPEC 可提出**：exact-cut bound 验证、分配凭证、唯一 allocation、保守 liability 与恢复协议；公平性/资金排序 policy 插槽及生效版本；调度 FIFO 与资金优先权明确区分。

**边界**：对账恒等式以成本已按腿分配为前提，不决定怎么分配；S 稳定点 FIFO 每点一请求不能默认为资金政策。

来源：[docs/adr/0014-intra-chong-unidirectional-inter-chong-no-arbitration.md](/Users/silencehan/Projects/NewChanlun/docs/adr/0014-intra-chong-unidirectional-inter-chong-no-arbitration.md)，[docs/adr/0024-total-ledger-convention.md:20](/Users/silencehan/Projects/NewChanlun/docs/adr/0024-total-ledger-convention.md:20)，[docs/adr/0024-total-ledger-convention.md:35](/Users/silencehan/Projects/NewChanlun/docs/adr/0024-total-ledger-convention.md:35)，[docs/adr/0024-total-ledger-convention.md:67](/Users/silencehan/Projects/NewChanlun/docs/adr/0024-total-ledger-convention.md:67)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/836#issuecomment-5324088421)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/1050#issuecomment-5325648529)。

### FU-05：闸门前统计主干已裁；计样边界与闸门后参数待

**原样继承**：费后成本相对降幅/占用时间；单边95%下界、日历月 block bootstrap；事件≥30、月块≥12、≥3非重叠窗各过线并合计≥50%全史；频率单独配套。

**具体残留**：成本下降事件与全部亏损/费用取样一致性；同时间多事件、跨闸、短仓基价、零时长、C_before≤0；闸门后判据参数。

**票据复用**：[#853](https://github.com/xy7365527-lang/NewChanlun/issues/853)（CLOSED）：全部已裁统计主干及闸门后参数触发条款直接继承；[#847](https://github.com/xy7365527-lang/NewChanlun/issues/847)（CLOSED）：历史 S8 承接导航；没有因此证明当前残留已闭；[#1339](https://github.com/xy7365527-lang/NewChanlun/issues/1339)（OPEN）：FU-05 当前残留。

**依赖切片**：完整经济事件与经营报告；闸门前样本构造/验收；闸门后评价生效。

**SPEC 可提出**：所有盈亏/费用完整事件账与排除原因；不可算状态、分母/时长边界显式承载；闸门后参数在首次触达后、第一次启用前落盘的触发协议。

**边界**：不能只收正事件，不能改动已裁统计门，不能为赶全功能验收给闸门后参数编数；参数未满足只可标待政策，不能算整体完成。

来源：[docs/adr/0022-acceptance-criterion-parameters.md:13](/Users/silencehan/Projects/NewChanlun/docs/adr/0022-acceptance-criterion-parameters.md:13)，[docs/adr/0022-acceptance-criterion-parameters.md:21](/Users/silencehan/Projects/NewChanlun/docs/adr/0022-acceptance-criterion-parameters.md:21)，[docs/adr/0022-acceptance-criterion-parameters.md:41](/Users/silencehan/Projects/NewChanlun/docs/adr/0022-acceptance-criterion-parameters.md:41)，[docs/adr/0022-acceptance-criterion-parameters.md:45](/Users/silencehan/Projects/NewChanlun/docs/adr/0022-acceptance-criterion-parameters.md:45)，[docs/adr/0022-acceptance-criterion-parameters.md:51](/Users/silencehan/Projects/NewChanlun/docs/adr/0022-acceptance-criterion-parameters.md:51)。

### FU-07：风险名分已裁；完整退出授权策略未证

**原样继承**：#659 将13课均线语境降为辅助参照，结构止损价名分为系统自设 Θ_risk，非缠论判据；#647 后裁继续保持该门名分。

**具体残留**：新完整方案的退出授权、责任退休及与 Q/阶段/计样残留的衔接，未取得具体后裁。

**票据复用**：[#659](https://github.com/xy7365527-lang/NewChanlun/issues/659)（CLOSED）：名分降级可直接继承；[#647](https://github.com/xy7365527-lang/NewChanlun/issues/647)（CLOSED）：具体旧入场门后裁，不等于新完整退出授权政策；[#1339](https://github.com/xy7365527-lang/NewChanlun/issues/1339)（OPEN）：FU-07 当前残留。

**依赖切片**：经营退出/封口/责任退休；风险与结构触发来源区分；旧持仓与未结责任恢复。

**SPEC 可提出**：结构事实、风控决策、用户权限、执行动作各带来源；已授权退出与未结外部责任分开状态；退出不能使在途订单责任凭空退休。

**边界**：生成态 fengkong 文档的 risk 命名禁令不能约束新架构；旧退出/入场门不恢复成全系统默认政策。

来源：[.chanlun/definitions/fengkong.md:31](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/fengkong.md:31)，[.chanlun/definitions/fengkong.md:34](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/fengkong.md:34)，[.chanlun/definitions/chanlun-trading-system.md:21](/Users/silencehan/Projects/NewChanlun/.chanlun/definitions/chanlun-trading-system.md:21)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/659#issuecomment-5122927120)，[裁定评论](https://github.com/xy7365527-lang/NewChanlun/issues/647#issuecomment-5123124627)。

## 交付与实施依赖

本轮没有找到可把全部 FU 残留直接闭合的独立现役裁定票；当前承接仍是 #1339 内具名 FU 需求，再由 #1323 SPEC 明确依赖。closed #835/#914/#915/#836/#853/#659/#647 可继承各自已裁约束，不能被重新解释成新资金政策或完整退出策略。#1050 是对账形式化承接导航，成本已分配的前提不能代替费用如何分配；本轮未验证其当前代码或运行状态。

入口核、日志、精确资源上界、观察、恢复及 policy 承载等不依赖具体资金/教义选择的切片，可以在 SPEC 获批后独立推进。所有依赖项仍留在完整 #1323 DAG 上；`AwaitingPolicy`、N/A 的类型表达、可记录未知事实均不是完整功能验收完成。资金排序与 S 稳定点 FIFO 调度分开；本报告不把调度顺序裁成资金优先权。

完整机器可读条目见 [RESIDUAL-DECISIONS.json](/tmp/newchanlun-1323-spec-20260909/inputs/RESIDUAL-DECISIONS.json)。
