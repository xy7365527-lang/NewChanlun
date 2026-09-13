R4 成对轨迹、HTTP 与追加 GUI 独立审阅：**NOT_VERIFIED**。正式双跑和已完整 HTTP 数据支持其声明范围；完整 GUI 候选导出存在24处字符损坏，不能据此宣告全字段通过。未发现需要在本轮改产品的新缺陷；两个缺项均已具名区分证据问题与真实故障截断。

身份：`/root/r7_codex_product_review`，`codex_native_subagent`。本工位不是 Q/UI/driver 作者；不自评自己此前编写的 Rust。R4/GUI固定 `1a57fb684e5a6aa18e4f962b68ba3d9f9a908c66`；运行Rust SHA256 `5bb7f1dfd57e1d8e6cfc1097360246ae27333776504767df4aba1b9fba9ce71b`。另审 launcher 修复 `2317369c3de2e3d9e8d90a545dd32d0e759d2b8b`，不把旧轨迹改称新提交实跑。

| 核查项 | 结果 | 独立依据与边界 |
|---|---|---|
| SOURCE | PASS | 21份运行内Git文件均匹配1a57固定对象；两轮23份源码/配置冻结文件全部核SHA，仅launcher的db/port/socket不同。Rust仅核运行产物身份，不自评其实现。 |
| EXACT495 | PASS | 495完整JSON记录按类型/字段/数组顺序比较，拒重复键，无身份改名/字段排除；全15表+schema、160原BLOB与Delta/batchID闭合。 |
| INPUT_CLOCK | PASS | 原160消息和raw输入逐值绑定，144源事件/16revision，全部受理/结果/482时钟phase与固定clock-plan对齐，原op95身份不变。 |
| FAULTS | PASS | 两轮真实S afterBegin SIGKILL→recover1到2→同消息重试；Q观察G109后独立SIGKILL至epoch2，S身份不变且线程期间继续发送110/111/112并采样。 |
| FINITE_LOAD | PASS | 95seed独立500ms计划+op95故障点+64live独立500ms计划；三窗口各>=2个实测点且接纳/提交前进，最长单回执和末发到phase end均在原10s/30s界内。保留后段积压，不称稳定2Hz服务能力。 |
| HTTP_PAGE_STATE | PASS | 171请求中169完整，153分页4529行（含重复采样）及8份已安装完整state/projection/cursor独立对拍；两份不完整另外记缺证。 |
| HTTP_WATCH_GAP_TOKEN | PASS | 两轮各G33-35正常续接+原G36响应；各两类Gap的身份/缺段/保留frontier/重建token/不推进cursor完整核对；G16旧token在Q epoch2继续同cut页。 |
| HTTP_REVISION_LANE | NOT_VERIFIED | 两个G96初始分页中断，不能用底层160提交替代消费者安装证明。 |
| GUI_Q_WIRE | PASS | 648真实分页19869行（含重复）和真实G157-160四批完整对拍，G159三withdrawals/三replaces；实际G39→135 retention Gap精确核对。 |
| GUI_NEGATIVE_SEMANTICS | PASS | 六种原Q正确4批经真实浏览器传输注入及重封摘要，独立核精确变异、正确原批、明确拒收及页内完整candidate/rendered不变判断；旧原响应因果拒收。 |
| GUI_FULL_CANDIDATES | NOT_VERIFIED | 五份完整候选导出24处U+FFFD差异，不能当全字段PASS；晚到旧页内嵌原件同样损坏。 |
| GUI_VISIBLE_DETAILS | PASS | 封存截图/DOM具名展示目录116、修订源坐标2/seq98/revision2/关系与见证、filter116→1→116；有限页面观察成立，不取代完整候选。 |
| LEGACY_LAUNCHER | PASS | 2317369仅legacy参数接线修复，显式验证在写库前；服务早期exec保持。根三负控与三端完整body经本工位独立核，不将旧R4冒称新提交。 |

两份 semantic-core.jsonl 各161,080,375字节，SHA256均为 `02fdd97ed3f2efbc2493a44506d1bc33cc3619fe57f0d25a781c39a50edd0442`。逐行拒重复键、保留类型和数组顺序比较495项；完整字段没有身份重命名和排除。记录涵盖160实际输入结果、160原身份最终回执、160唯一Delta及其真实index_frontier对应BLOB、13其余全表、全部schema和生命周期记录。BLOB核完整base64、长度、SHA及batch身份；不是仅比汇总。

S故障：A events行556/560，PID78225→78289；B行555/559，78758→78820。marker完整匹配PID/stage/db/exe；恢复原reachable G95，op95保留attempt0 Begin无Commit，恢复后attempt1发布G96，同message/payload未改。Q在两轮events行662观察G109后SIGKILL，PID77877→78373、78481→78916；S保持新PID，重启期间继续送110/111/112，分别有18/19个frontier样本。停止回执中的退出期间argv警告原样保留；控制器在真实已停止后才返回stopped，不能把警告当成清除记录的理由。

有限负载：三窗均只取声明闭区间内实测首末点。A提交进展20/19/16，B为20/20/17。live末次回执A3689.554ms、B2845.149ms；末发至phase结束3690.283/2845.941ms，运输并发峰值6/5，采样已接纳未提交峰值均1。500ms是发送计划，回执速度并非稳定2Hz；积压、最大约5.2ms调度迟到和窗口未观测边缘均保留于原件，不事后延长10s/30s边界。

HTTP共A82/B89请求，完整81/88。每份完整HTTP首行、唯一头、Content-Length、EOF、请求/响应原bytes/hash及公共因果头独立核对；153页含4529行是实际页行计数（有重复），不是4529个独立结构对象。独立纯解析器直接从495中的最终SQL记录按first-known/withdrawn/published与cut输入前沿投影六族，不导入Q投影器；全部字段和顺序进入比较。旧G16 token在Q epoch2返回offset31..62同cut页；G36到G41未消费5代的backlog Gap缺段37..41，G32到G41保留Gap缺段33..33，token与重建G41对应。显式load完成G41与收到Gap第一页是两项实际操作，不冒称已证明同一个自动Gap流程。

`R4-E2`：revisions G96初始分页A停在第17请求、B第24请求，两份末响应均0字节/EOF，之前仅465/682行（完整应849）；两lane无安装。保留NOT_VERIFIED。GUI之后的真实G157–160四批带G159三撤回/三替代，可作另域补证，不能改写这两次截断。

`R4-E1`：GUI-FREEZE1405文件/139,793,336字节哈希全匹配，但5份candidate仍有24处内容差异。例 `gui/actual-gap-rebuild-state.json#/result/candidate/projection/catalog/items/5/domain` 的“均非包含”变“均��包含”；`final-full-state.json`也有3处差异。精确全部pointer/实际值/权威值见 [GUI-EXAMINATION.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/r4-pair/independent-review/GUI-EXAMINATION.json)。对应 network response 不含U+FFFD，六份注入原件亦无，支持采集/保存层故障解释；尚未定位实际解码实现。不得凭hash正确抹去语义差异，也不得按权威内容修补旧导出。应另行无损导出真实候选（ASCII转义或完整字节base64）并增量核验。

GUI网络648页/19869行（重复包含在内）与原记录匹配；真实G39→135保留Gap和完整4批正常Watch均匹配。7份大型Watch在network目录body=null，六负控及healthy脚本单独保存完整原Q body，本审阅对这些body与权威Delta全字段比较，并核匹配网络请求/HTTP200。六类负控仅按源码具名变异并重封摘要，均显式拒绝；没有让Q实际产坏批次。完整before/after串相等与rendered相等是在真实页内执行的冻结脚本证据，未另导出两份串。旧真实Watch响应在新请求因果拒收；late-held内嵌原件损坏，不能称其与网络原帧逐字相等。session_generation=2只是拒收负控，不是实际新会话迁移。

目视两张封存截图，并核具名DOM字段：修订事件evt-0002的seq98/source_coord2/revision2及见证关系、最终G160元数据/142活动48撤回/116目录可见；filter116→1→116仅改变显示，页内全state比较结果保留。完整状态仍受R4-E1限制。

另见 [LEGACY-REVIEW.md](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/r4-pair/independent-review/LEGACY-REVIEW.md)：launcher增量有界通过。没有新增服务/测试重跑，没有访问任何活DB/API。独立脚本首轮把Watch的max_batches写成limit、把公开Delta包装误当原delta_json，失败日志和更正前脚本均留存；GUI脚本发现字符损坏及network大body缺失后改为明确NOT_VERIFIED/分来源，未改原件。所有实际读件SHA见 [READING-LOG.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/r4-pair/independent-review/READING-LOG.json)，语义审查范围与仅哈希检查分列。此报告不关闭完整C或#1323，也不抹去历史R1/R3失败。
