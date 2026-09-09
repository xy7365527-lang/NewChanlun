> 仅转换链接的阅读版；[原字节](../../payload/inputs/RD-17-MODEL-PLAN.md) SHA256 `c294187293a3afff69b908e998d4066af00704eb4f4577de17e5f83285988063`。原稿状态是发生时记录，当前发布范围与限制见归档根 README。

# RD-17 完整历史模拟执行模型拟票对象

名分：#1323 发布准备工作草稿。仅本地计划；未发布 GitHub、未修改源 SPEC 或原 RD-01–16 计划。

结论：可作为第 17 项残留承接纳入发布 overlay。它填 C10.3 的真实缺口，直接继承已裁 bar 限价规则，保留旧裁定边界。

## 双路查重证据

- GitHub `撮合`，state=all，exit=0，命中 14 票；号码：#1329 OPEN, #1283 CLOSED, #1340 OPEN, #1311 CLOSED, #926 CLOSED, #1321 CLOSED, #918 CLOSED, #154 CLOSED, #927 CLOSED, #423 CLOSED, #849 CLOSED, #907 CLOSED, #463 CLOSED, #787 CLOSED。

- GitHub `模拟 限价`，state=all，exit=0，命中 13 票；号码：#1340 OPEN, #1335 CLOSED, #1336 CLOSED, #938 CLOSED, #1013 CLOSED, #933 CLOSED, #799 CLOSED, #787 CLOSED, #907 CLOSED, #815 CLOSED, #915 CLOSED, #813 CLOSED, #174 CLOSED。

- GitHub `开盘 取优`，state=all，exit=0，命中 56 票；号码：#1340 OPEN, #1250 CLOSED, #1039 CLOSED, #1226 CLOSED, #1040 CLOSED, #918 CLOSED, #385 CLOSED, #1119 CLOSED, #931 CLOSED, #908 CLOSED, #856 CLOSED, #421 CLOSED, #853 CLOSED, #872 CLOSED, #776 CLOSED, #932 CLOSED, #135 CLOSED, #854 CLOSED, #787 CLOSED, #136 CLOSED, #734 CLOSED, #864 CLOSED, #384 CLOSED, #423 CLOSED, #843 CLOSED, #258 CLOSED, #1335 CLOSED, #927 CLOSED, #154 CLOSED, #862 CLOSED, #847 CLOSED, #842 CLOSED, #859 CLOSED, #390 CLOSED, #812 CLOSED, #914 CLOSED, #485 CLOSED, #1120 CLOSED, #138 CLOSED, #925 CLOSED, #771 CLOSED, #838 CLOSED, #157 CLOSED, #839 CLOSED, #299 CLOSED, #871 CLOSED, #674 CLOSED, #59 CLOSED, #873 CLOSED, #769 CLOSED, #1121 CLOSED, #529 CLOSED, #249 CLOSED, #219 CLOSED, #248 CLOSED, #865 CLOSED。

- GitHub `混合 订单`，state=all，exit=0，命中 69 票；号码：#1323 OPEN, #1340 OPEN, #988 CLOSED, #982 CLOSED, #309 CLOSED, #1186 CLOSED, #1335 CLOSED, #1075 CLOSED, #912 CLOSED, #743 CLOSED, #1055 CLOSED, #853 CLOSED, #1077 CLOSED, #667 CLOSED, #987 CLOSED, #550 CLOSED, #938 CLOSED, #381 CLOSED, #1321 CLOSED, #847 CLOSED, #366 CLOSED, #1064 CLOSED, #1336 CLOSED, #771 CLOSED, #193 CLOSED, #775 CLOSED, #547 CLOSED, #184 CLOSED, #59 CLOSED, #231 CLOSED, #219 CLOSED, #597 CLOSED, #817 CLOSED, #467 CLOSED, #796 CLOSED, #787 CLOSED, #415 CLOSED, #187 CLOSED, #395 CLOSED, #917 CLOSED, #767 CLOSED, #776 CLOSED, #911 CLOSED, #419 CLOSED, #770 CLOSED, #840 CLOSED, #529 CLOSED, #137 CLOSED, #915 CLOSED, #870 CLOSED, #932 CLOSED, #1126 CLOSED, #813 CLOSED, #232 CLOSED, #835 CLOSED, #333 CLOSED, #815 CLOSED, #812 CLOSED, #854 CLOSED, #480 CLOSED, #250 CLOSED, #403 CLOSED, #627 CLOSED, #769 CLOSED, #872 CLOSED, #782 CLOSED, #574 CLOSED, #443 CLOSED, #768 CLOSED。

- GitHub `同slot`，state=all，exit=0，命中 13 票；号码：#1340 OPEN, #981 CLOSED, #983 CLOSED, #474 CLOSED, #200 CLOSED, #157 CLOSED, #154 CLOSED, #180 CLOSED, #837 CLOSED, #396 CLOSED, #787 CLOSED, #398 CLOSED, #734 CLOSED。

- GitHub `ModelOrderUnspecified`，state=all，exit=0，命中 1 票；号码：#1340 OPEN。

- 仓内 `开盘.{0,12}取优|限价.{0,12}穿越|bar.{0,20}限价|GTC|ModelOrderUnspecified`，范围 analysis/、docs/、.chanlun/，exit=0，30 行；完整匹配保存在同名 JSON。

- 仓内 `撮合.{0,25}(并发|顺序|多重|混合)|模拟.{0,25}(并发|混合订单|顺序)|同.?slot|TestOnly`，范围 analysis/、docs/、.chanlun/，exit=0，3 行；完整匹配保存在同名 JSON。

GitHub 命中 #1329 仍暂停；#1340/#1323 为上位容器。closed #918/#926/#927/#938 正文与评论已核读并在 JSON 留存；裁定射程详见拟票正文。没有找到已开放、已获同范围开工且能够直接代替本具体对象的票。

## 拟发布正文

Part of #1323

已批准基准：{{APPROVED_SPEC_PERMALINK}}；S2-R1 清单 SHA256 `38e3176b6dfff456b8e3704410401c3bedddf0671751464bb9ea5b829e460260`。

本票承接现有 C10.3/C10.4 与 DST-04/05 的完整历史模型义务；不新增 G/FU 需求。#1339 为需求来源，旧 #1329 保持暂停；#918/#926/#927/#938 的关闭不代表新 R2 并发/混合域已裁。

待完成：
- 具名且有版本的 simulated execution profile/model：历史输入适用域、同时性与事件先后、订单可成交/撤改/存续、模拟成交与费用事实的生成规则及仍未支持的域，逐项区分继承规则和本次新裁定。
- 对多重并发、混合市价/限价、同 bar/同 slot 竞争给出确定且可审计的解释；同模型、输入与已固定调度轨迹产生同一组完整事实，不把独立重的方向合并为策略仲裁。
- 模型与 RD-11 资金准入、RD-12 分配/更正政策分别具名并组合绑定。RD-12 负责外效发生后到各重/订单的唯一归属；本票负责模拟外效如何发生，不替代或重裁 RD-12。
- 裁定落相应工程 ADR，附受影响 Rust/Lean/Python 代码清单及不受影响理由；点名把模型实现、独立预期凭证和适用域实例回填 TB-07，不能以本票文档关票声称实现已完成。
- TB-07 的验收接线须从正式会话入口使用正式批准模型，固定输入/修订/模型/政策/时钟/种子/数值/故障/调度，双新进程比较逐重状态、毛账、责任、命令、回报、分配与 AsKnown；本票交付其模型期望与回填契约。

直接继承：
- 承接已批准 SPEC C10.3 的完整历史模型成功义务。C10.3 是现有合同条款定位符，不新建 G/FU 需求，不扩大 R2。
- ADR 0018 已裁的 bar 限价严格穿越、开盘取优、GTC、限价费用口径及其适用范围继续有效；只在裁定射程内继承，不以同栏存在某个排序段就泛化到 R2。
- ADR 0019 明写市价单与限价单同 bar 并存的成交次序未定义；旧路径同 bar 不可达仅是已查路径的有限结论，不能证明 R2 多重并发不需要模型。
- 同 bar 先平后开、价优与稳定次序的旧裁定及其订正须一并核对；既有 NT planner 的 ConflictKey 不是历史撮合器，也不是共同资金政策。不得按 OHLC 形状虚构未知的 bar 内真实路径。
- 历史、paper、live 共享完整核心与身份/责任；模拟 fill 只能作为明确 simulated profile 下的唯一事实源。R2 的 P4/P7、每轮固定 L、逐重账及净省禁复用均不重开。
- 未裁模型域显式 ModelOrderUnspecified；TestOnly 的人工顺序可供协议实验，不能据此销完整经营/历史验收。

可独立备料：
- 逐项摘录 #918/#926/#927/#938 与 ADR 0018/0019 的已裁规则、订正和未定义适用域，形成可继承/须补裁/仅旧软件组织三分表。
- 按同 bar/同 slot 的多重同向或反向、平开同存、市价/限价混合、既存/新挂单、撤改与成交竞争列最小差异例；比较候选模型如何产生不同的模拟事实及责任，不以代码当前遍历顺序代裁。
- 明确在给定历史输入精度下可知的事件次序与模型假定次序；列出需要显式版本化的模拟能力和数值/时间字段，不从缺失 tick 或盘口推断真实成交路径。
- 梳理模型输出的不可变外部事实到 RD-12 分配政策的接口，以及 RD-11 共同资金准入政策的引用边界；模拟成交发生与成交后归属分别取证。

仅阻以下成功子义务，不阻无关开工：
- TB-07：R2 多重并发、混合订单和同 bar/同 slot 域的完整历史经营与全状态确定性双跑成功；TestOnly 或 ModelOrderUnspecified 仅证明相应实验/失败分支（AT-14）。
- TB-08：仅共享核心/可替换 adapter 的跨模式验收中实际消费该历史模拟模型的对照成功；真实场所能力取证自身不被本票一刀切阻塞（AT-15）。

仍可继续：
- TB-01/02 的正式结构、会话和观察；S 健康时继续产出，与经济模型缺项分开。
- TB-07 的原始输入、恢复、seek、AsKnown、调度记录与全状态比较基础设施；已有具名适用模型域的受控检查，以及明确 TestOnly 的协议实验。
- RD-11 资金政策备料、RD-12 分配规则备料、RD-13 场所/账户/权限只读研究与 TB-09 隔离设计；真实场所 profile 不作为本历史模型备料的整票前置。

后续讨论节拍：一次一问；不自动代答。本票发布不等于已向用户提问。
首个备好材料后再问的问题：在保留 ADR 0018/0019 已裁适用域的前提下，R2 多重并发和市价/限价混合的同 bar 或同 slot 执行，正式历史验收采用哪一份具名模拟执行模型？
激活条件：AFTER_PUBLICATION_AND_MODEL_ALTERNATIVES_PREPARED

来源：
- SPEC.md:334-342（C10.1-C10.7，尤其 C10.3/C10.4）
- SPEC.md:366-375（TB-07 完整历史模型与局部成功门）
- docs/adr/0018-limit-order-fill-model.md:69-178
- docs/adr/0019-limit-order-state-flow-interface.md:39-49
- docs/adr/0019-limit-order-state-flow-interface.md:249-253
- docs/adr/0019-limit-order-state-flow-interface.md:468-475
- https://github.com/xy7365527-lang/NewChanlun/issues/918#issuecomment-5210716261
- https://github.com/xy7365527-lang/NewChanlun/issues/926#issuecomment-5210868134
- https://github.com/xy7365527-lang/NewChanlun/issues/927#issuecomment-5211124225
- https://github.com/xy7365527-lang/NewChanlun/issues/938#issuecomment-5223998272

关票条件：完整模型明确且按适用范围落相应 ADR，受影响代码/独立期望凭证与 TB-07 实施验收承接有落点；本票不以模型文档关闭替代 TB-07 或整图实装完成。只写 ModelOrderUnspecified 或采用 TestOnly 次序不替代必需完整经营成功路径。

本票不授权真实下单、转账、充值、账户变更或合入 main；不填写未经确认的资金、费率或退出默认。


## 计划边界

RD-11 资金准入、RD-12 经济分配、RD-13 真实场所权限分别持有其问题；本票无整票 prerequisite，相关资料并行准备。成功组合仍须引用实际使用的正式政策。source_gap_ids 的 C10.3 仅是既批合同条款定位符，不是新增需求 ID。

## 交付索引

同名 JSON 的 `plan` 对象字段与原计划每行完全一致；`github_dedupe`、`repository_dedupe`、`issue_detail_evidence` 为真实查重/核读证据。父代理只需将 `plan` 作为第 17 行 overlay，随后验证生成正文与局部成功门映射。
