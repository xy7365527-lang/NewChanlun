# #1394 fullclock 成本静态代码追踪

日期：2026-10-01。仅只读源码；未构建、未启动服务、未执行 DataPath、未访问 A/B 数据库。本文为工作草稿，不是审计豁免或实施批准。

固定依据已经逐字核对：Root HEAD `ed1f184bf7513d960d87b6687c279a657c8fa481`，`HEAD:rust` tree `5a2acd1e72c3846be38cc29689a82aeb57c68dc6`。下文路径以该 worktree 根为基准。优先调用 `search_graph(project=/Users/silencehan/Projects/NewChanlun-1394-content-v3, …)`，返回 `project not found or not indexed`，随后只读指定模块及其直接依赖。没有借用另一 indexed worktree 的旧图推断本版。

## 已核实的结论

`ClockPlan` 全量读、解析和 hash 的重复存在，但不是每个历史 batch 都重新解析一次 clock。正常成功的新消息 `handle` 有 3 次完整 `state::audit`；每次 audit 的全量 clock 解码只有 1 次。历史 batch 每代消费的是 hash 字符串与真实 commit 时刻。

同一成功的新消息，从接纳、Begin 到 Commit，总计 14 次持久全量 clock 解码、28 次全 clock 规范 JSON 构造、14 次全 clock SHA256、8 次整 `ClockPlan` 深 clone。该计数不含服务启动、额外 Ready/receipt、重投、恢复或失败中途退出。fullclock9011 不因只推进 prefix16 而缩小，14 次解码均扫描整个 9011-event 计划。

逐代来源审计不解析 clock。它重核生成对象、源配方和内容帧。请求开始前已发布 `g` 代时，成功 fresh handle 的历史 source audit 次数是 `3g`，加本次新生成态一次，合计 `3g+1`。这些遍历、内容帧与对象/JSON hash 工作随历史增长。上述静态证据无法判定 30 秒失败的主因，也不能预测 9011 全量运行。

## 全量 clock 的唯一规则及实际读口

| 代码位置 | 行为 | 一次成功调用的全量工作 |
|---|---|---|
| `rust/src/bin/s_session_v2/mod.rs:68-72` | `ClockPlan` 为 `#[derive(Clone)]`，持有 `serde_json::Value` 和 hash String | clone 为整 JSON 深 clone，不是引用计数句柄 |
| 同文件 `:74-134`，`ClockPlan::parse` | 核完整五字段、协议/unit、全部 events/attempts、非负/不倒退；`:133` 规范 JSON + SHA256 | 扫全部计划；1 次规范化、1 次 SHA256 |
| 同文件 `:136-137`，`ClockPlan::load` | `fs::read → strict_json → parse` | 每次外部文件加载一次全量解码；服务不是每请求重读文件 |
| 同文件 `:192-204`，`stored_clock` | SQL 读取 `clock_plan_json,clock_plan_hash`；`:200` 严格 JSON + parse；`:201` 再规范化核原字节和持久 hash | 1 次全量 JSON decode/parse，2 次规范化，1 次 SHA256 |
| 同文件 `:207-211`，`check_clock` | 无条件 `stored_clock`，再比当前 ClockPlan hash | 不是只读 hash 列；包含上行全量工作 |
| 同文件 `:139-156`，`phase` | 从已解码 events 找 id、从 attempts 找指定相位，核时间为非负规范整数 | 不重新 decode、canonical 或 hash 整 clock |
| `rust/src/session_protocol.rs:49-74` | `canonical_json` 递归排序对象键、分配各子项 String/Vec，再 join | 不复用 parse 时第一次构造出的规范文本 |

S3 `control::S3Control::attach` 在 `control.rs:90-117` 核两协议标记、五表、`:109` 的 `check_clock`、当前域身份，然后 `:113` 深 clone clock。`bind_ingest` 在 `:198-236` 核 message key/hash/clock_event_id 后，`:232` 为 `InputContext::new` 再深 clone。`InputContext` 自身在 v2 `mod.rs:160-170` 持有 clock，非 borrow。

当前 Begin 的另一条全量入口在 `rust/src/bin/s_session_v2/content_publication.rs:262-271`。`verify_current_begin` 自己读 clock 原件、strict decode、`ClockPlan::parse`、canonical 重构，并把 manifest hash、持久 hash 与计算 hash 三方对拍。它不是 `stored_clock` 调用，却有相同的两次规范化和一次 SHA256。

## Driver open、每个 handle 与三次 audit

服务加载链为 `writer_cli::serve :462 → ClockPlan::load :472 → Driver::open_for_serve :475 → Driver::open :141 → open_with_request :151`。

`open_with_request` 在 `driver.rs:165` fresh audit，`:167` attach 当前文件 ClockPlan，核 epoch/profile/idle；随后 `:199` 恢复已有 pending。无 pending 的 open 对持久 clock 解码 2 次，attach clone 1 次，另有 CLI 文件解码 1 次。服务再调用 `Driver::identity`，`:107` audit、`:109` attach，再持久解码 2 次、clone 1 次。open 的成功审计没有存入 Driver，下个请求继续 fresh audit。

正常 fresh ingest 的完整调用链：

1. 接纳事务 `driver::handle :203 → state::audit :209 → S3Control::attach :211 → inspect_ingest :231 → bind_ingest :232 → control.accept :255 → v2::record_accept_context :258 → record_phase :279 → check_clock :223`。
2. Begin 新事务 `driver::advance :281 → state::audit :286 → attach :291 → load_pending :292 → bind_ingest → control.begin :308 → record_begin_context :308 → record_phase :329 → check_clock :223`。
3. Commit 新事务 `driver::advance :314-318 → prepared::Runtime::commit → prepare :66 → state::audit :73 → stored_clock :74 → attach :75 → load_pending :76 → commit_time :87 → verify_current_begin :138 → generate → source_audit::audit_generated :220 → commit::publish_prepared :48 → resolve_current_births :67 → verify_current_begin :41 → stored_clock :68 → attach :69 → load_pending :71 → commit_time :79 → control.commit :118 → record_commit_context :420 → record_phase :427 → check_clock :223 → tx.commit :138`。

| 事务 | audit 内 decode | attach decode | 直接 stored_clock | record_phase decode | verify_current_begin decode | 总 decode | 整 clock clone |
|---|---:|---:|---:|---:|---:|---:|---:|
| 接纳 | 1 | 1 | 0 | 1 | 0 | 3 | 2 |
| Begin | 1 | 1 | 0 | 1 | 0 | 3 | 2 |
| Commit，prepare + publish 同一事务 | 1 | 2 | 2 | 1 | 2 | 8 | 4 |
| 成功 fresh handle 总计 | 3 | 4 | 2 | 3 | 2 | 14 | 8 |

每个成功 fullclock9011 handle 的全部-plan event 检查次数因此为 `14×9011=126154`，另有全部 attempts 扫描、28 次规范文本分配、8 次 JSON 深 clone。790213 B 若为规范 clock JSON 的实际字节数，规范文本的构造总字节为 `28×790213=22125964 B`；这不是峰值 heap、RSS 或资源第七门的计量结论。

其它分支不能套用 fresh 总数。Committed 重投在 `handle:233` 提前返回，1 audit + 1 attach，2 次持久 decode、2 clone。receipt 同样 2 次持久 decode，但只 attach clone 1 次。Ready 额外走 `v2::ready_for_protocol:1318` 的 stored_clock，共3次 decode、1 clone。失败路径只计抵达过的点，不得按14当作 A/B timeout 的实际执行计数。

## 每次 state audit 中随历史增长的工作

`state::audit :67 → audit_with_budget :102 → control_audit::prepare_s3_control_history :115 → v2::validate_control_history :810`，在 `v2::mod.rs:849` 调用 stored_clock **一次**。其后复用这个局部 ClockPlan：

- 已持久 messages 循环 `:935-1030`，每条在 `:1019` 查 accept 相位；不深 clone clock。
- 持久 s_clock_events 循环 `:1042-1120`，每行在 `:1068` 对拍计划相位与真实 ns；不解码 clock。
- 发布期望循环 `:1166-1181`，每代在 `:1179` 找该实际 attempt 的 commit 相位，把期望留给 ControlHistory。
- ControlHistory 只留 session_generation、clock_plan_hash String 与 remaining 期望，不保留 clock JSON；`mod.rs:1184` 移动 hash，无深 clone。

一般一个 audit 的 `phase` 次数为 `M + P + g`，M为全部持久输入消息数，P为全部持久 clock phase 行数，g为已发布代数，包含 recover/多attempt 时使用真实M/P。

一msg一代、无recover、每msg一个attempt，fresh成功推进前已发布g代时：接纳前为 `5g`；接纳后 Begin 前为 `5g+2`；Begin 后 Commit 前为 `5g+3`。三次 audit 共 `15g+5` 次历史 phase lookup。它们使用同次 audit 已解码的计划，不是整计划扫描。额外当前消息相位 lookup共13次，分别为handle预检查3、accept1、Begin2、prepare commit_time1、两次verify_current_begin各2、publish commit_time1、commit1。因此完整 handle 为 `15g+18` 次 phase；这是正常成功路径静态计数，不是运行测量。

历史内容链 `state.rs:127 → history::read_indexed_history :71`，每个 batch 在 `history.rs:143` 循环中 `:246` 调 `ControlHistory::consume_header`，共g次。该方法 `v2::mod.rs:713-739` 只逐项核 session_generation/hash/commit_ns 与唯一期望，消费后要求 finish闭合。**该循环不调用 S3Control::attach，也不 decode clock**。每次 fresh成功handle三次重放同g代，所以 header核验为3g次，必须保留。

`state.rs:145-178` 随后对每cut运行 `source_audit::audit_generated_metered :151` 与目录证据。source audit `source_audit.rs:206` 接收 generated_state/raw_prefix/profile/rule/deadline/budget，未接收 ClockPlan，也不查询clock表。历史原件 envelope在 `history::verify_source :441-473` 严格解码及payload hash核验，但此处读的是该message envelope，不是 clock计划。

source audit主要路径包括 `source_audit.rs:218` 的生成态闭包；`:227-243` 的effective源重建和wire/numeric/raw三种节点hash；`:253`逐对象字段及JsonText视图；`:318-333`的input refs、source coords、fact_key解析及typed对象ID；`:479-489`的比较文本/输入引用等。这些均仍逐代执行。若prefix每代只加一个事件，raw hash工作量仅这一段就是每个audit `3×sum(j,1..g)` 次hash调用，三次audit合为 `9g(g+1)/2`，再加新生成态的 `3(g+1)`；raw有修订时必须改用各cut的effective长度，不能按此式硬算。

本轮读取的上述链里没有名为 `RootValue` 的类型。实际结构是`ContentRef`及JsonText内部摘要；`s_content/path.rs:493-514`进入JsonText是读内部摘要，不是把clock_plan_json decode成 RootValue。不能把两者的解码次数混写。

## 可审阅的最小候选边界，尚未实施

只选择 **同一SQLite事务内，已逐字确认未变的clock_plan_json/hash两列**。三个事务各从当前快照解码、校验一次，形成事务局部不可变 decoded clock；后续依赖读取仍核持久hash、当前外部ClockPlan hash、每个headerhash，以及实际phase/attempt/ns和frontier。不可用上一请求的successful audit替代当前准入。

具体需守住两点。

首先，`s_protocol_meta` 的next_attempt_ordinal/logical_phase_frontier在同事务内会推进，不能缓存整行或以“一次attach成功”豁免后来的动态判据。每个依赖点如仍读到原始clock两列，须与局部decoded素材的**原字节及hash都相等**才复用，变化则用原严格逻辑重解码/拒绝；仍保留相位前沿等当前列读取。这样不用宣称“hash相等已经证明JSON合法”。局部素材要绑定Transaction借用，不能放进Driver/Runtime或Root跨请求keeper。

其次，`attach` 和InputContext可以在该局部生命周期借同一immutable ClockPlan，消除重复全JSON clone，但不能删attach的域身份/持久clock绑定判据，也不能从ControlHistory的成功摘要反推其它audit通过。提交事务内prepare至publish虽有节点/索引暂写，clock计划两列仍需按上述相等条件确认；不能把state::audit后所有数据库状态当作不变。

候选的数量上限是新消息三事务各一次全decode，由14降到3，clone从8降到局部共享/借用；这是未来设计候选的静态上限，未承诺性能收益，当前无动态证据支持立刻实施。三次state audit、3g次header核验、全部g代source/目录/Scope判据均保留。若未来实施，最小实现可先只消除同事务重复decode，clone是否借用另行按编译借用关系判断；本工位未改生产代码。

frame/hash等其它重复也有同一素材边界，但不在此次候选内。`content_store.rs:90-106`每次checked读重新读当前原件并核帧，缓存的只是SQL语句。`history.rs:745-827`在overlay命中时复制body，未计费分支透传本次checked frame，不保存跨read核验结果。若后续需要重复读取复用，只能同tx、同已核digest/tag/body、同Template/Public/JsonText上下文并保留计费与期限门；不能顺带改成Root跨请求缓存policy。

## 最小测量建议，全部物理phase仍未测

只使用root冻结完成的A/B owned失败原件及只读复制，保留fullclock和原资源。既有正式binary的wire总时长不能分解私有state::audit；专属shadow工位需自行取得其构建权限，本工位不build、不代跑，也不读活DB。

Root本轮提供的构建证据另指出，正式binary5437是dev，debug=0、opt-level默认0；旧prefixclock16内部binary8bef为release、当前前版8f9。编译profile、代码版本、clock计划、接口和sampler都是已知混杂。14parse/8clone是每请求固定项，不能解释为它本身随g增长，也不能凭这两组总时长宣布fullclock为根因。Root当前先核同一5a2源码的release非test正式binary、同fullclock、同资源30秒探针；这是root的构建/探针工作，本工位仅登记该混杂，未执行。

建议两个靶向位置，不另拆报告：

1. `v2::ClockPlan::parse :74-134`加一个shadow专用累计计数/耗时marker，标明事件数与入参规范字节数，严格限制为只读观测。它直接覆盖全事件校验、第一次canonical和SHA256，但不覆盖调用方strict_json、第二次canonical或clone，输出必须注明这个范围；若该段占audit很小，不能宣称整个clock占比已小。
2. `state::audit_with_budget :102-195`使用已有`cfg(test)`阶段trace，尤其`control/history/generated_sources/catalog_cut/complete`，并保留既有node读取/checked-frame bytes计数。只读shadow调用已有私有state::audit，不调用Driver::open/advance，因为后者会自动resume_pending和写入。输出归入本轮同一测量summary。

若ClockPlan marker仅显示小部分耗时，而既有history/generated_sources与帧hash阶段随g扩大，下一结论应写“成本主要在内容帧/来源重建与hash”，并据真实marker给占比；本报告现在只能写“这些是随历史增长的明确路径，静态推断未动态测定”。clone的实际时间、clock strict decode/第二次canonical时间、峰值heap与RSS均仍UNMEASURED。不能用旧prefixclock16内部Driver的10.39s与本fullclock外部exchange做减法求因果。
