# #1394 固定 cut 与正式 consumer 接线合同草稿 v3

本文件绑定 #1394。它是已授权的准备实现说明，`execution_authorized=false`；所有产品状态 `NOT_RUN`。下列形状中的尖括号是必须由根冻结的输入名，不能作为可执行命令，也不能替换成 null、旧提案值或 unit 夹具。没有创建产品运行配置。

本轮工具固定审查起点是 dfdeda34886d4a8f0a3bfa7bc8e3f3930aab28f0；3315 是工具宿主原基线。协议取证来自 b46 只读快照，既不钉死未来 producer，也不证明未来统一源兼容。根路线：1488 Native 修复（5cb 前缀只是路线信息）→1419 审后 C…F→根组合 C1 99e819192bd6c91b1dd90e058a93c2a98d5aaaf3→根固定统一新 SHA。Native 与 C1 在 b46 分叉并共改 financial_reader.rs，必须重新语义审查组合，不能以 main3315 或两个单件独审替代。

## 已实现的离线 API 与边界

`captured_cut.export_captured(contract_raw, expected_sha, captures, source, budget, closure_budget, input_raw=...)` 消费根另行固定的合同摘要、已捕获的请求/回复原字节（Capture）、独立节点原件读取器、完整输入原字节和两个累计预算。资源合同逐项校验session/reader/Job、S七门、Q18+3、每E/B输送、旧16MiB/10秒、stage/whole/cleanup/seal；缺值或超旧256MiB retained帽即拒绝。捕获帧逐route核session与reader或Q的适用帽；wire/请求按具名所属Job累计，Q页长按原Q帽，闭包节点与解码记录归reader，Q投影记录归Q。这不等于测到Native staged/store/RSS运行消耗。无网络、Popen、数据库写入或产品执行入口。`execute` 永远拒绝。

合同版本 `issue1394-captured-cut/2` 的字段为 `schema_revision/execution_authorized/source/fixed_cut/anchor/full_scope/page_size/cut_projection_digest/captures/resources/stage`。source 必有 commit/tree/binary_sha256/input_sha256/artifact_manifest_sha256/profile_hash/rule_revision/catalog_revision；commit/tree 40位，摘要64位。这里的 `expected_sha` 是外部信任边界，须由根冻结，不能由待检包作者在正式运行时自算作为批准。与 scheduler 准备合同的硬 pin 是两个不同对象。

captures 每项固定 `route/request_sha256/reply_sha256/binding`，数量和顺序必须等于实收全集；原字节调用方须随包保存，输出只引用其摘要。当前只接受 snapshot、financial-basis、financial-closure 三条已核 route；新 route 没有宽松解释分支。source(root) 必须读取独立已冻结的持久内容原件，不允许用收到的 wire 在调用现场伪装原库；toy 测试分别保存源节点和可变捕获流。

已核范围：外层 Q 会话、payload hash、因果身份、reply ID；固定16字段 cut、17字段 snapshot-token/2、原page_size与offset、issued_capture_digest、不重不漏末页；七字段 financial anchor；同根全basis闭包（只允许 path=[]），升序节点、after/next/total/done、原字节与独立原件一致、类型/长度/可达全集；完整20字段 snapshot；六族 objects/withdrawn_objects/witnesses/relations/observations/raw_history 的公开键、排序、ContentRef、数量、全集；catalog_evidence 来源根、Q投影根，逐对象 financial selected_objects 的ref/hash/index。原始输入 SHA 与冻结来源核对。

返回 `UNAUTHENTICATED_FACTS`。它保留实际对象、关系、见证和完整修订记录，不会补出 G。Q catalog 的内容目前核其冻结引用及投影根，未单独解码 Q 派生 catalog 内容；Q content/watch、Delta与七变化集合的业务验真尚未接入此模块。闭包模板0x08仍拒绝；json_text语义解码、非整数数字内容重编码未接线时拒绝。上述边界意味着该工具**不是正式全协议adapter或结构语义认证器**，不能因toy成功就宣称未来真实捕获已可全部导出。

`voice_bridge.assemble(facts, contract_raw, expected_sha, raw_by_name, budget)` 接收固定 facts 的canonical SHA、完整原件和根冻结的显式字段映射。版本为 `issue1394-voice-bridge/1`。必须有 facts_sha256、mapping、artifacts、instrument_declaration、clock_mapping、independent_gamma_proof、bundle_author_id、opening_source；execution_authorized必须false。mapping 精确包含 terminal_kinds/carrier_kinds/terminal/carrier/source；JSON pointer字段集合由代码明列，正式值尚未取得，不用toy字段冒认未来G格式。

它枚举全部实际terminal kind和全部K kind，核 source/terminal/carrier 原件全集，profile、级别、方向、锚、确认前缀、来源对象hash、整个离散来源区间无缺口、K孩子唯一/无环/区间包含/层级、同级host唯一、γ到terminal到host绑定、全部同端同侧标签、instrument、opening_raw以及显式TestOnly时钟。多γ候选全部保留；无terminal/K/γ产缺项报告，多host拒绝。没有raw_history/OperationSequence/Active/旧50070端点兜底。字段映射中 start/end 必须由根确认确为同一输入的闭区间ordinal；否则不能用该映射。尚不证明数学γ、terminal标签全集的语义或K生成器的生产完整性。

桥返回 `UNAUTHENTICATED_CANDIDATES` 或 `MISSING_DEPENDENCY`，不输出APPROVE、不生成正式authority、不自动BindVoice。`independent_gamma_proof`的根固定原字节/作者不同仅是输入绑定检查，字符串作者名不是独立审查的认证机制；真正语义独评与根准入仍是外部必需件。


## 本轮勘误：协议、对象绑定、当前阶段

v2 的 SessionClosure 表格把首游标写为 null 是错误的。本轮只读核对 b46 原快照和固定 C1 `99e819192bd6c91b1dd90e058a93c2a98d5aaaf3` 的 `financial_reader.rs:745`：`Some("") => None` 是内部解释，外部 JSON 必须传文本；null 会被 `after须为文本` 拒绝。摘录在 `references/C1-closure-cursor.txt`，原 v2 报告与独审证据保持原字节。

旧 `/1` 捕获合同没有资源归属和当前阶段，现明确拒绝，不能在导出器内补默认帽。`/2` 仍是准备工具的候选合同，尚无产品调用者，也未获根冻结正式数值。

`resources` 在原有 session/reader/q/s_writer/economic_domains/deadlines/legacy_reader 之外增加两个必填对象：

- `object_ids` exact `session/reader/q`，各为不空且互不相同的根冻结对象身份。
- `jobs` 是两个具名 Job 的映射；每项 exact `owner/limits/started_at/deadline`。owner 分别指向本次 reader 与 Q 的对象身份，limits 保留八维原字段，reader 项必须逐值等于 `reader.job_limits`。两个 Job 的实际值都要根冻结；工具不产生数值。当前候选合同不支持共同 Job：若未来确有共同对象，须根显式冻结其语义并另审合同，不可把Q偷偷记到reader。

每个 Capture 及外部pin的清单项均包含 binding。snapshot 的 exact binding 是 `q/job`；financial-basis、financial-closure 是 `session/reader/job`，分别指向上述对象/Job。同一原请求和回复计一次所属Job请求及原件wire长度；financial另逐session累计请求，逐帧同时受session和reader限制。Q独立核max_frame_bytes、max_reply_bytes、max_page_size与所属Job；不抵扣reader帽。闭包唯一节点与其编码字节受reader Job和session节点帽，完整basis解码记录受reader Job；Q投影记录受Q Job。这里的计数是已捕获原件的可观察部分，不声称等于产品运行的全部Job消耗。

`stage` 必须点名 `resources.deadlines.stages` 的当前阶段。共享读取预算及闭包预算不得晚于当前stage、work（whole）及本次两个所属Job截止中的最早值；Job的绝对截止不得超出其原started_at+max_elapsed_ms。已经完成的历史阶段仅保留记录，不参加当前最早期限。cleanup/seal固定预留及原顺序保持，不给任何阶段或owner续期。合约须在同一monotonic时钟域冻结；离线跨机搬运的绝对时间不构成有效运行授权。

删除捕获路径借用Voice的单件64MiB/集合128MiB限制；捕获按所属对象帧帽、所属Job及显式ReadBudget受限，不新增替代大帽。Voice模块自己的64/128MiB保持不变。ReadBudget仅是工具整体原件读取预算，不能冒充经济Job或根资源授权。

max_capture_bytes 是生产端数据库 capture 的原对象帽，不能改解释为这些wire文件的累计尺寸。retained/staged/store/RSS/磁盘、数据库 capture、连接/队列/缓存及生产逐请求耗时、未捕获历史请求，不由这组离线原件测得；返回的 `unmeasured_resources` 明列这些限制。S七门、E/B输送、旧冷读取路径及Q非snapshot功能仍保留原资源声明，但本模块不执行它们，不把形状检查声称为运行验证。

## authority 与独评原件不得放宽

正式bundle沿用 `voice-binding-authority/1`，exact顶层 `schema/authority_id/source/artifacts/bindings`。每件artifact exact `name/path/sha256/bytes`：绝对路径、原字节摘要和精确长度；单件≤64MiB、总集合≤128MiB，桥在读内容前检查所有声明尺寸，不按分页放宽。最低原件 input/controlled_gammas/carrier_objects/instrument_declaration/opening_raw；本桥额外要求 terminal_objects/source_objects/gamma_proof/clock_mapping，均收齐原字节。

独评receipt必需字段为 `schema/verdict/authority_id/bundle_sha256/artifacts/verified_evidence_ids`，schema=`voice-binding-independent-verification/1`，verdict=`APPROVE_CONTROLLED_VOICE_BINDING`。authority_id、完整bundle摘要及artifacts必须一致；verified_evidence_ids逐一按bindings原序覆盖且唯一。本轮不制作这种receipt。旧 consumer evidence.rs 对bundle/binding/artifact使用exact，对review是逐必需字段校验而非拒绝所有额外字段；这里不把它误称为已有更严格的运行时exact检查，后续根冻结receipt须用上述既有字段。

binding exact：evidence_id/instrument/chong_id/operation_level/carrier_ref/gamma_ref/structure_view/sigma/n/parent_voice_id/opening_source。structure_view exact profile_id/execution_level/confirmation_level/nest_depth，确认级=执行级+nest_depth。opening_source exact fact_key/fact_hash/allocation_id/raw_sha256/json_pointer/quantity/units。source绑定完整input_sha256/profile_id/instrument/first_confirming_observation_source和clock_mapping；后者 exact kind/source_origin/nanosecond_origin/nanoseconds_per_ordinal，kind=TestOnlyAffineOrdinalToNs。ordinal不是墙钟。父Voice与n不得由编组器默认选择。

## producer → consumer 的实际接线顺序

| 阶段 | 当前可构造的命令或payload形状 | 必需输入、原件和阻断 |
|---|---|---|
| 来源/构建准入 | 不提供cargo命令；由根冻结两角色ARTIFACTS | producer和consumer各自commit/tree/toolchain/features/binary SHA/构建原输出；新组合独审；工具源pin。不能把工具HEAD当producer。 |
| writer完整输入 | `<S_BINARY> advance --protocol-revision s-session/3 --db <S_DB> --profile <PROFILE> --writer-epoch <EPOCH> --clock-plan <CLOCK> --input <FULL_INPUT> --max-input-bytes <FROZEN_MAX>` | S域初始化及源具体输入格式须用新统一源审后合同；全部输入顺序、原字节hash、profile/rule/catalog、时钟逐事件、owner、资源、截止、监督ACK均冻结。不能截短9011或用null默认。服务形状见下段。 |
| writer回收→custody | P0 OwnedProcess.finish/cleanup_all → custody(S_DB, budget=共享预算) | pid/start/group、binary前后SHA、exit/wait/reap、writer已不存在；main/WAL/hash+dev/ino/size/mtime/ctime，shm身份；失败不得交给reader。 |
| financial READY | `<S_BINARY> financial-serve --protocol-revision s-session/3 --db <S_DB> --socket <SOCKET> --resources <SESSION_RESOURCES> --custody <CUSTODY> --receipts <EMPTY_RECEIPTS_DIR>` | session resources/3、独立监督、原STARTUP/READY、binary/db身份、不可变副本、anchor、资源摘要、请求模式SessionClosure、同截止；READY不是业务PASS。 |
| 固定Q snapshot | POST `/api/v3/snapshot`；信封payload首请求见下段，后续只含 op/snapshot_token | Q21资源、profile/rule/catalog和固定cut；全部页+原请求/回复bytes，后页复用原token，不混capture身份/页面。 |
| 全对象basis | `<S_BINARY> financial-basis-ref --protocol-revision s-session/3 --db <S_DB> --session <SOCKET>` | stdin exact schema_revision/session_id/session_generation/writer_epoch/object_ids；object_ids是实得全对象原序，不用空数组冒完整对象证明；返回s-financial-basis/3。 |
| SessionClosure | 同上client形状改为financial-closure | stdin exact schema_revision/anchor/path/after/max_reply_bytes，schema=s-financial-closure/1；本导出器要求path=[]、首after为文本空串`""`、之后严格等于真实上一页next；null拒绝。捕获全basis可达节点与独立源原件。 |
| 新E/B域 | `<E_BINARY> init --manifest <MANIFEST> --db <E_DB>`；`<B_BINARY> init --manifest <MANIFEST> --db <B_DB> --chong <ID>`；serve加`--epoch <EPOCH>`，B再加`--e-epoch <E_EPOCH>` | economic-manifest/1 + protocol_revision economic-session/2，新空目录、两个独立fresh A/B（不是两经营owner），每个B有独立db/socket/owner。冻结初始账/现金/固定Q/完整操作日程，不从actual倒推。 |
| CaptureExternal→ApplyAllocation | E：`{op:CaptureExternal,raw_utf8:<原件>,json_pointer:<指针>,token:<真实来源token>,source_event_time:<冻结值>}`；B：`{op:ApplyAllocation,fact_key:<E原事实>,allocation_id:<原ID>,expected_fact_hash:<原hash>}` | 原开局必须经真实E Recorded、B Applied。保存原请求/回复、facts/applications/receipts/messages、提交根/seq/ns；未知结果只查原身份。 |
| 取本B开局原事实 | `QueryFact {op,fact_key}`，`QueryAllocation {op,allocation_id}` | 从本B原fact.cut取得完整e_cut，与真实Applied前件核对。不能预填来自另一个B、另一会话或旧运行的cut。consumer内部将这个cut写入opening_basis。 |
| 真实BindVoice | payload exact `op/binding_id/evidence_id/carrier_ref/gamma_ref/parent_voice_id/sigma/n/opening_basis`；opening_basis exact `fact_key/expected_fact_hash/allocation_id` | 真独评固定authority bundle；同owner instrument/chong/操作级别/来源/数量；首次知晓时间不早于真实γ映射和Applied。请求并没有任意可填e_cut字段。parent_voice_id的协议null仅限实际根Voice且n=0，编组器不代选。 |
| 财务业务 | ImportResponsibility `{op,responsibility_id,writer_epoch}`；PrepareDecision `{op,decision_id,plan_hash,writer_epoch}`；Reserve `{op,slot_id,request_set_hash,writer_epoch}`；CommitDecision再需decision_id/plan_hash/grant_id/writer_epoch；AbortDecision需decision_id/plan_hash/writer_epoch/reason；CloseApplicants需slot_id/writer_epoch/closed/reason | 原计划、现金与原始开局、全部申请集合、责任、整slot等待；政策原样#1349/atomic-slot-wait/v1，实际cap/排序不改。原24故障组、5负域、10compare全部保留。 |
| query/history/watch/originals | ReadView `{op,cut,as_known_ns}`；Watch `{op,after_cut}`；QueryDecision/QueryGrant按原协议；QueryVoice `{op,binding_id}`；只读域库含meta/commits/receipts/messages | 指定实际cut，不选latest冒同cut。原协议的可选时间null须由正式查询合同选择，不当缺参默认。全提交/history有序全集、原key映射及幂等出现序号、原错误回复须完整。当前evidence规范化接口尚非所有wire的正式解码器。 |
| S离线恢复 | writer监督recover形状为`recover --protocol-revision s-session/3 --db <S_DB> --new-epoch <NEW> --clock-plan <CLOCK> --clock-event-id <EVENT>`；E/B `recover --manifest ... --db ... [--chong ...]` | 真实marker→同pid SIGKILL→wait/reap→custody→新epoch；仅该owner新epoch，其他owner保持原epoch，原请求bytes/key/ordinal/有序全集不变。S下线后核本地持久读取，不调用在线S补件。 |
| 统一回收封口 | stop financial session；逐已登记owner TERM/KILL/wait/reap；固定cleanup/seal窗口 | 已退出owner也wait；迟到保留reap事实但整体INCOMPLETE；陌生PID不收养、不乱杀。最终原件/错误/资源/时间/摘要链及独评另交根。 |

S writer服务形状是 `serve --protocol-revision s-session/3 --db <S_DB> --profile <PROFILE> --writer-epoch <EPOCH> --clock-plan <CLOCK> --socket <SOCKET> --resource-config <S_RESOURCES>`。Q部署CLI以及后续统一源新增G/terminal/K内容生产入口尚未由新源冻结，本轮不给猜填命令。

Q首payload exact：`op=snapshot, session_id, session_generation, scope, history_mode=AsKnown, as_of_generation=<固定代>, page_size, order_version=s-record-order/2`；后续exact `op=snapshot,snapshot_token=<原17字段token>`。外层信封 exact来源需沿session_protocol实际版本生成，至少保留 schema_revision/session_id/session_generation/source_namespace/source_epoch/message_id/producer_id/producer_epoch/payload_hash/causal_refs/payload；Q用s-session/3，经济用economic-session/2。回复不能只拆payload丢原信封。

财务content的已知payload exact：`schema_revision=s-financial-content/1,anchor,path,unit,offset,limit`。Q content为 `/api/v3/content`，首payload `op/anchor/path/unit/offset/limit`，续页 `op/content_token`（token9字段）。这些原协议形状已取证，但当前导出器仅走全basis SessionClosure，未冒称实现Q content或watch的全接线。

## 资源逐对象冻结，未定值一律阻断

| 对象 | 必须原样保留的字段/帽 |
|---|---|
| S writer服务 | s-structure-resources/1 exact外层schema_revision/s_resources；内部max_frame_bytes/read_timeout_ms/write_timeout_ms/response_timeout_ms/audit_cache_source_bytes/queue_capacity/max_connections。 |
| financial session | s-financial-session-resources/3：startup_timeout_ms/retained_bytes/max_requests/max_request_bytes/max_reply_bytes/request_timeout_ms/max_capture_bytes/max_closure_nodes，加schema_revision。保留旧256MiB retained边界，不能把本票toy值当新部署值。 |
| financial reader | s-financial-reader-resources/1：max_request_bytes/max_reply_bytes/max_stderr_bytes/request_timeout_ms/job_limits，加schema_revision。独立于session，不能相互代替。 |
| 每个Job | max_total_wire_bytes/max_total_node_bytes/max_total_nodes/max_total_records/max_total_requests/max_elapsed_ms/max_staged_bytes/max_store_bytes 八维，跨页累计；初始已保留store/staged也计费。 |
| Q原18项 | max_frame_bytes/max_pending_connections/socket_read_timeout_ms/socket_write_timeout_ms/max_capture_bytes/capture_deadline_ms/verify_deadline_ms/max_query_workers/max_query_queue/max_reply_bytes/max_cached_images/max_cache_bytes/poll_interval_ms/max_client_leases/client_lease_ms/max_client_pending_batches/max_page_size/max_watch_batches。原值分别保留。 |
| Q三Delta项 | max_atomic_delta_node_bytes/max_atomic_delta_nodes/max_atomic_delta_records；不是用一个聚合字节帽替代；s-query-resources/3外层schema_revision/limits。 |
| 每个E/B输送 | max_frame_bytes/read_timeout_ms/write_timeout_ms/response_timeout_ms/queue_capacity/max_connections；各自db/socket/owner/epoch、往返原件与最早response截止。 |
| 旧冷读/读取帽 | 旧16MiB/10秒不消失；必须按其原对象登记，不能套到别的Job或把新会话帽用于放宽旧路径。冷6参独立对照臂不参与生产准入。 |
| 工具/进程树 | 每stage、whole最早绝对monotonic截止；ReadBudget与closure预算不延长；pid/start/group/parent、进程数/RSS/磁盘/输出、心跳、采样间隔；cleanup/seal独立固定预留。日志与原件尺寸也算。 |
| Voice原件 | 单件64MiB/集合128MiB；精确bytes/SHA/绝对路径；不能把分页或未认证候选当豁免。 |

本轮未收到上述对象的新运行值，不生成任何可启动运行包。旧PREPARATION里的null/提案只保留历史。工具的Python读预算、闭包账与toy Probe不证明OS硬隔离，也不替代Native资源账。正式执行必须在根固定新统一源和逐对象完整资源后，由根发精确单次准入。

## 当前最小缺件

新统一producer SHA与组合语义独审；正式完整输入所得同域terminal全标签/完整K森林/实际source对象/独立γ证明及其确切字段映射；两个经营owner各自的真实authority/独评；两角色ARTIFACTS/toolchain；全资源与实际请求日程/精确运行准入；实际OS Probe/服务传输；最终经济原库与完整history/原回执。另有本导出器明确拒绝的template/json_text/非整数数字及Q content/watch解码，遇到时须先接审后准确协议，不能换宽松适配器。

这些缺件未满足前，P11、完整历史、资金链和AC1..5均NOT_RUN。#1394/#1361/#1340/#1323/#1279保持开放。新模块的轻量成功与正确拒绝均不销这些成功义务。
