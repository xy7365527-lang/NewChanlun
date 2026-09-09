# 后端观察契约草案

> 工作草稿，绑定需求票 #1339；待需求范围确认。不是实施 spec，不是已存在 API，也不裁软件 owner。

本契约说明调用者必须能拿到什么以及如何判验收，不指定微服务、数据库、全局事务、事件溯源或某一候选方案。需求来源是当前票面与用户指令；已有三代表链报告只提供问题和需求线索，其静态结论不扩大为全仓现状。

## 观察边界

- 结构层交付判据与证据的结果。操作/经营层对每个重保有独立目标、经营史和经济责任，观察者只能读取相应权威的结果。
- 全互斥限定在具体对象、分类轴与良构域。不同轴、级别及允许重合的事实保留；知识不足、计算等待和撤回是不同的说明维度。
- 『完整』是对明确请求范围及一致切面的承诺。可以在等待完整结果时提供标明缺口的局部观察；是否默认展示局部结果留作呈现取舍。这里没有要求不同权威域先合成一个全局资金事务。
- 后端必须保留完整语义；界面可选择图形、表格、详情、时间线、过滤或折叠。具体视觉布局留到方向和契约确认之后。

## 逐项调用者合同

### OB-001 观察范围与来源

要求：每次查询、快照和变化均绑定标的、数据来源与输入版本、规则版本、运行模式、会话及代际、观察视角和请求覆盖范围。

拟议字段或入口：scope、instrument/source、input_revision、rules_revision、run/session/generation、view、mode；名称为草案。

验收情景：同一标的的历史回放与实时观察同时打开，任何旧会话消息不能改写另一会话；切换视角不会串用旧对象。

失败反例：跨会话/跨数据源同名标的仅靠symbol匹配。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:14–14；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:46–48。以上字段为草案推导；执行状态 NOT-RUN。

### OB-002 三种层次分别表达

要求：结构级别、操作级别、Voice相对代际分别可查；级别归属可由观察视角及引用确定，不要求冗余复制。

拟议字段或入口：structural_level/context、operation_level、voice_depth/generation。

验收情景：同一结构供两个不同操作级别的重消费，UI能分清使用关系；Voice深度变化不改写结构级别。

失败反例：把显示K线周期或裸depth当作全部层次。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:14–46。以上字段为草案推导；执行状态 NOT-RUN。

### OB-003 分类轴与适用域

要求：每个互斥分类附对象、相应级别、分解视角及良构域；允许不同轴/级别事实并存。知识不足和失效不混进教义分类。

拟议字段或入口：object_ref、axis、domain/assumptions、value、evidence_refs；适用性与知识/有效性分别说明。

验收情景：同一个点同时具有正本允许的2B和3B，两个事实都保留；不满足完成域的走势不硬归入完成三分类。

失败反例：全局first-match；为消冲突换宽判据；中枢教义候选态。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:9–14。以上字段为草案推导；执行状态 NOT-RUN。

### OB-004 对象身份与修订

要求：可追溯同一观察代际内的对象及其修订；重切、替代、合并或拆分须给新旧关联，身份规则在后续spec明确。

拟议字段或入口：object_id、revision、supersedes/replaces、input_span、structural_key/context。

验收情景：相同对象修改几何或依据时按revision更新；重切产生新对象时可追溯被替代对象，跨代际不意外复活。

失败反例：只按数组序号/marker位置追加；更改坐标后保留两个假对象。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:38–47。以上字段为草案推导；执行状态 NOT-RUN。

### OB-005 完整结构载荷

要求：后端提供覆盖表中对象、轴、边界、组成、血缘和完成依据；前端可折叠分页，但被折叠的语义仍可查询。

拟议字段或入口：geometry、range/core/envelope按对象适用；children、relations、completion_basis、tail_scope；均引用ST表。

验收情景：用户打开任一已列结构类型及变化，可看到其适用字段和依据；中心扩展同时可追踪关系与高级构造是否成立。

失败反例：只返回marker、布尔level_lift或矩形包围盒就宣称完整。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:34–36；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:46–49。以上字段为草案推导；执行状态 NOT-RUN。

### OB-006 完整快照的切面

要求：完整查询返回请求范围在一个可说明的一致切面上的状态；分页仍绑定同一切面。多权威域可用可验证的版本组合，不要求先选全局事务。

拟议字段或入口：snapshot_id、cut/版本组合、scope_manifest、cursor、complete_for_scope、missing/pending原因。

验收情景：并发结构修订与经济成交发生时，每份宣称完整的快照能说明所用各域版本及依赖；同快照翻页不偷偷读取新头。

失败反例：把新结构、旧资格、新资源无标注拼成一个完整现在。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:42–48。以上字段为草案推导；执行状态 NOT-RUN。

### OB-007 局部结果与等待

要求：结果不齐时明确缺哪一域、哪个前置及其最近可信切面。可以提供局部观察，但不能冒称完整，也不能因此新增交易资格。是否默认展示局部结果属于呈现取舍。

拟议字段或入口：completeness、missing_scopes、pending_reason、last_trustworthy_cut、staleness。

验收情景：某一重或专题暂不可用，调用者能区分真实空集、暂未算出、源缺失和最近可信记录；恢复后再给完整结果。

失败反例：将超时/无响应填false或空数组；由UI补算。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:14–48。以上字段为草案推导；执行状态 NOT-RUN。

### OB-008 游标后的完整变化

要求：变化查询相对指定快照切面，覆盖新增、字段更新、依据更新、修订/替代和撤回；说明顺序域与重复处理方式。

拟议字段或入口：change_id、base_cut、next_cursor、object_ref、before_revision、after_revision、operation、causal_refs。

验收情景：重复投递、乱序和批间断档后，按合同处理结果等于同切面重取快照；漏任何已列语义字段变化都能被识别。

失败反例：仅追加marker，忽略move撤回或几何变化。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:38–48。以上字段为草案推导；执行状态 NOT-RUN。

### OB-009 发生区间与何时获知

要求：保留结构所涉市场区间、系统首次获知与确认的输入前沿，必要时另有收件/应用时间；未知时间明确未知。

拟议字段或入口：market_span、first_known_at/input_cursor、confirmed_at/evidence_frontier、received_at、applied_at。

验收情景：后续输入确认了较早区间的事实，回看较早时点不能提前显示为已确认；后续更正保留原获知记录。

失败反例：用turn_source bar、最终计算时间或绘图横坐标冒充首次确认。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:28–30；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:47–48。以上字段为草案推导；执行状态 NOT-RUN。

### OB-010 判定依据追溯

要求：每个展示事实可查询对应版本的结构锚、有效输入域、判据/规则版本、证书及依赖；必要条件、完成事实与经济准入有明确不同用途。

拟议字段或入口：evidence_ref、certificate_ref、rule_ref、input_refs、dependency_cut、claim_kind/use。

验收情景：小转大必要条件出现时只能显示其真实判定内容；点击可追到来源，不能把gate_pass改名为现象完成。

失败反例：看到经济门通过就宣布结构现象发生。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:28–30；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:46–49。以上字段为草案推导；执行状态 NOT-RUN。

### OB-011 结构与重的使用关系

要求：展示每个重的目标、结构视角及所用版本，哪项证据触发了哪项经营判断或状态变化；共享结构不授予重间仲裁权。

拟议字段或入口：chong_id、target_ref、operation_view、consumed_structure_cut、decision_ref、evidence_refs。

验收情景：同一结构修订影响多个重，逐重说明已吸收/尚待处理及其影响，不自动统一各重方向。

失败反例：只给市场状态和总仓位，看不到各重依据及独立权利。

范围依据：需求票 https://github.com/xy7365527-lang/NewChanlun/issues/1339 的核心目标与第一阶段；准确采集件 /tmp/newchanlun-1339-requirements-20260908/evidence/issue-1339.json /body。三代表链报告仅作观察问题导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:49–59。以上字段为草案推导；执行状态 NOT-RUN。

### OB-012 经营观察与权威分离

要求：逐重提供FG表要求的状态与经济事实，分别标明经营判断、授权/义务、真实执行事实及观察投影；只读观察不会创建经济权利。

拟议字段或入口：chong/voice/stage、cash/lot/cost/principal/claim按批准口径、gross_obligations、physical_projection、source_refs。

验收情景：同一净头寸对应不同逐重毛账时，UI保持逐重差异；未定资金规则显示未定，不填默认钱。

失败反例：净额回推出私有账；观察者重算并写入本金或资格。

范围依据：需求票 https://github.com/xy7365527-lang/NewChanlun/issues/1339 的核心目标与第一阶段；准确采集件 /tmp/newchanlun-1339-requirements-20260908/evidence/issue-1339.json /body。三代表链报告仅作观察问题导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:49–59。以上字段为草案推导；执行状态 NOT-RUN。

### OB-013 诊断、结构事实与交易准入

要求：诊断推测/必要条件、教义已成立事实、生产准入结果分别说明语义和用途；诊断不得兜底准入失败。

拟议字段或入口：purpose、claim_kind、evidence_sufficiency、admission_ref/拒绝原因（适用时）。

验收情景：同样输入下诊断项可显示更多线索，但生产准入结论不被更改；未知不当拒绝或通过。

失败反例：弱版本补判后放行；中枢未存在却包装为候选中枢事实。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:11–14；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:49–49。以上字段为草案推导；执行状态 NOT-RUN。

### OB-014 撤回不抹账

要求：结构/资格修订和撤回保留历史与被影响引用；实际成交和既成经济责任依据其真实来源保留，后续补救另有授权。

拟议字段或入口：retraction、affected_refs、economic_application_refs、correction/compensation_refs（如有）。

验收情景：结构撤回时同时存在未发计划、可能已发订单与已成部分，展示各自去向；已成交不随图形消失。

失败反例：删图形等于撤单完成；撤结构就删除真实成交或恢复现金。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:47–49。以上字段为草案推导；执行状态 NOT-RUN。

### OB-015 HTTP/流式会话同一性

要求：任何快照和增量传输共享声明的session/generation/cut语义；传输技术可后选。

拟议字段或入口：session_handle、snapshot_cursor、stream_binding、generation。

验收情景：REST建立回放会话后，流式订阅绑定同一逻辑回放会话及一致输入切面；重建会话后旧连接消息被隔离。

失败反例：HTTP一个回放，WS另起默认实时或独立回放。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:38–48。以上字段为草案推导；执行状态 NOT-RUN。

### OB-016 断线续接与断档

要求：可从有效游标继续；游标过期或不可恢复须明确要求重取快照，并保留可说明的缺口范围。

拟议字段或入口：resume_cursor、retention_boundary、gap/resnapshot_reason。

验收情景：断线跨越更新/撤回后重连，最终状态与同切面快照一致；过期游标不会悄悄丢变化。

失败反例：丢消息后继续追加而不告知断档；宣称传输网络恰一次。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:38–48。以上字段为草案推导；执行状态 NOT-RUN。

### OB-017 按当时可知回放

要求：历史回放明确『当时可知』与『使用后来修订重算』两模式；前者不使用未来证据，后者另标来源版本，经济回放还绑定真实经营/执行输入。

拟议字段或入口：replay_semantics、input_frontier、knowledge_cut、revision_policy、operating_input_refs。

验收情景：逐步推进到确认前后，事实只在相应前沿出现；以同源输入双跑比较状态/关系/事件；经营恢复不能只靠bars猜钱。

失败反例：拿全历史最终结构往过去贴；回放和实时用两套判据。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:30–48；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:59–59。以上字段为草案推导；执行状态 NOT-RUN。

### OB-018 范围过滤与完整性声明

要求：查询按标的、级别、重、时间和对象类型过滤时声明筛选范围与被省略项；分页/降采样不改变后台判据与原始事实。

拟议字段或入口：filters、scope_manifest、page_cut、omitted/aggregated、detail_ref。

验收情景：用户过滤掉部分重或对象后仍知道这是子集；打开详情可恢复原始证据，不能因显示上限删后台对象。

失败反例：前端只画三例就把支持列表写成全定义；用采样图替代完整状态。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:49–59。以上字段为草案推导；执行状态 NOT-RUN。

### OB-019 完整后端与可用呈现

要求：所有覆盖项均有明确可达的观察入口；常用层显示概览，其余可通过筛选/详情/时间线访问，无需全部同时铺满图表。布局尚未定案。

拟议字段或入口：capability/catalog、对象详情、关系/变化查询、逐重经营视图、evidence link。

验收情景：对每个ST/FG项从后端结果到可见状态或详情形成可验证路径；鼠标点marker不是唯一可用入口。

失败反例：库里有enum、接口字段或图上有节点就算可呈现。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:42–49；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:59–59。以上字段为草案推导；执行状态 NOT-RUN。

### OB-020 可核验的能力与版本声明

要求：明确哪些入口/模式/对象当前支持、受限或未支持；区分定义覆盖、字段存在、实际构造、接线、靶向执行和端到端验收。

拟议字段或入口：capability_manifest、mode_matrix、evidence_status、implementation_revision、known_limits。

验收情景：对每个验收项能指出哪个版本/场景/输入/输出证明；未运行项保持not-run，未读源码保持未复验。

失败反例：把171合同、94锁、91文件字节核验当作实现与运行通过。

依据导航：/tmp/issue1323-frontend-state-requirements/FINDINGS.md:5–5；/tmp/issue1323-frontend-state-requirements/FINDINGS.md:59–59。以上字段为草案推导；执行状态 NOT-RUN。

## 需要在架构设计阶段回答的协议问题

1. 哪些权威提交会推进哪些观察版本，怎样证明结构、经营资格和经济资源的依赖切面一致？由方案给出可验证协议；本草案不先选 B/A2/C3。
2. 对象身份在重切和回补时如何延续或替代，游标的顺序域/保留期是什么，外部更正如何重建当时可知视图？由方案列状态与失败恢复路径。
3. 完整查询等待多长、实时延迟/吞吐/留存数量承诺是多少？这些数值尚未得到用户要求或运行证据，不在草案中虚构。
4. 可展示局部结果不等于允许其它重继续经济动作。经济故障隔离、经营答案独立发表权等是后续需要明确的软件要求，不能从只读展示需求自动推出。

## 验证计划的层次

逐项建立正本边界情景 → 生产构造/输出 → 传输保真与会话一致 → UI消费/撤回 → 包含实际持仓、成交和未结责任的回放及恢复情景的证据。静态复用来自已验收报告，源码今日未复验处明列历史静态。本文未执行项目、服务、浏览器、Lean、交易、回放或性能测试；文档产出与记录完整性检查不替代这些运行验证。

