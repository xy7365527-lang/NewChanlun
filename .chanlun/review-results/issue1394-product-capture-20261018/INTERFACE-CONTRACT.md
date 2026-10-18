# #1394 读侧接口与准入边界

本文件为绑定1394的工作草稿，描述已实现接口和后继窄接线需求，不是产品执行合同。没有正式运行值、根 grant 或批准 receipt。

## 调用顺序

1. 根从独立审过的统一 source 产生两角色 ARTIFACTS，冻结原逐对象资源、原 deadline、原 owner 与执行环境。根提供 `driver.ExecutionAdmission` 的外部合同/准入指针并消费其单次 receipt。
2. 既有正式启动链提供同一 admission 注册的 session、Q `OwnedProcess`。读侧不自建 Q/session、不猜 Q 启动命令、不把 READY 当业务通过。各启动 command 增加根冻结 `read_endpoint`，Q 为完整 `{host,port,path}`，session 为 `{db,socket}`。
3. `ReadAdmission(admission, supervisor)` 核验下面的只读扩展。构造失败时不发任何产品动作。
4. `capture_product(gate, output)` 使用计划全部有序请求槽。结果为 `MISSING_DEPENDENCY`，区分 `UNAUTHENTICATED_RAW_CAPTURE` 与 `INCOMPLETE`。`Capture` 含 route、完整 request/reply bytes、显式对象/Job binding；`Exchange` 另含 transport reply、stderr、exit、时间与进程/命令身份。
5. `persist_originals` 写原件、逐次身份及 `ORIGINALS.json`（精确长度/SHA，不自引）。捕获错误后仍封存已收到错误原件。HTTP 在超声明帽/未知长度/断线时保留已观测前缀并标不完整，不无限读取超帽数据或宣称取得未收到的尾部；CLI 原 stdout/stderr 文件另保留并纳入清单。

## 根合同扩展

`driver` 合同的 `product_capture` exact 字段为：`schema_revision=issue1394-read-admission/1`、`adapter_sha256`、`plan`、`compatibility`、`supervision`、`supervisor_limits`、`output`。后三种原件指针沿用 `{path,sha256}`，须列入同一根 artifacts；`supervisor_limits` exact 为原 `max_frame_bytes/deadline`，不提供默认帽。`output` 是新独占绝对目录。

`plan` 版本 `issue1394-product-capture/1`，exact 字段由 `validate_plan` 列全：生命周期/capture身份、八项来源身份、固定cut与anchor、Q page_size/投影根、原完整resources/current stage、reader binary、session/Q owner与端点、逐次slots、两具名Job八维initial_usage、initial_session_requests、显式verification工具预算。

slot exact 为 `id/route/stage/deadline/binding/argv/closure_max_reply_bytes`。route 只允许 snapshot、financial-basis、financial-closure；id/stage唯一，Q在先、单basis居中、closure在后。日程必须包含实际末页所需全部槽；多槽、少槽或无法匹配实收 next 均拒绝，不自行追加命令。每个CLI槽有原 `driver.commands` 唯一stage、精确argv、binary/domain/epoch和完整资源。Q槽无进程命令，绑定已注册Q owner及唯一POST端点；没有DNS、代理、重定向、其他路由。

`compatibility` exact 为 `schema_revision=issue1394-read-compatibility/1/source/historical_freeze_sha256/adapter_sha256/verdict=COMPATIBLE`。其中 source 必须等于计划的新统一来源。该原件只有在根外部 pin 后才被消费；本轮没有输出这种认可文件。历史 b46/C1 摘录不会自动满足新统一源兼容要求。

`supervision` exact 为 `schema_revision=issue1394-read-supervision/1/implementation/capabilities/verdict`。implementation 是根 artifacts 中的精确实现原件；capabilities 按序为 `process_tree/hard_deadlines/per_object_resources/endpoint_owner/read_guard/1`；verdict 为 `APPROVE_READ_GUARD`。这是对未来独立 OS 实现的外部信任边界，不是只改字符串即可取得运行资格，也不是本模块实现了这些 OS 能力。测试中自制的临时 pin 只测试机械绑定，全部使用替身，从未作为产品准入使用。

## 尚缺的 read_guard/1 服务端

在产品I/O前，先重核原owner/binary/endpoint与原Supervisor ACK，再通过既有有界控制输送发送 JSON+LF：`kind=READ_GUARD/schema_revision=issue1394-read-guard/1/binding`。

binding 包含 root contract SHA、plan SHA、新nonce、phase（before/after/finalize）、完整owner/endpoint、原slot、request SHA、最早原deadline、lifecycle_id、两Job八维usage_floor及session_requests_floor。回复必须 exact `kind=READ_ACK/binding/checked_at/job_usage/session_requests`，binding逐字相同、采样发生在本次调用内且未超原deadline；累计不可回退、不可超原帽。旧ACK绝不作为新接口的替代。

**当前没有服务端实现，不能启用产品。** 后继本票 OS 纵片需让独立监督实际验证：完整进程树与身份；全过程端点确属于已登记owner；原session/reader/Q各自资源与两个Job实际累计（包括Native审计、staged/store、连接/队列、heap/RSS/磁盘/输出）；原work/stage/request/Job截止和回收封口。不得只回显request中的floor充当测量，也不得由caller自报OS能力。必须提供实际实现、原件和独评，由根pin后才可接本接口。R-F5旧发送尾差仍须管理者先修复并独审。

Supervisor的常规stage ACK绑定原slot绝对期限；更早的本次request timeout另随READ_GUARD绑定，并在ACK返回后检查。所有生产请求前后及最终解码后均调用guard。旧SupervisorCore只有普通ACK/采样，不能满足上述扩展；新增适配不会静默放行。

## OwnedProcess 窄适配

现有 `OwnedProcess.__init__` 的gate使用缓冲stdin读取控制行并在放行后关闭stdin，无法给会话客户端送原stdin。新 `_StdinProcess` 只适配这段初始化：相同根command单次消费、独占输出目录、原身份校验及ACK后释放，控制行用逐字节 `os.read`，随后写原request再EOF，沿用原cleanup/reap。

原 `register_owner` 禁止同S域有活进程，而 financial客户端必须与financial session并活。因此新适配只允许计划点名的那个session作为同域活前件，其余同域活client仍拒绝；新client本身在任何身份探针/ACK前登记，错误立即按原cleanup回收。未修改driver的writer行为或旧41方法。后续若合并公共初始化，建议只增加“原stdin bytes＋已冻结session前件”接口并保留所有原检查，不能一般化为任意同域并活。

## export 的窄patch建议（未实施）

现 `captured_cut.py:350` 使用 `wanted=[r['object_id'] for r in snapshot['objects']]`，随后要求请求原序等于存储顺序；`:359` 按存储序构造selected列表。真实Q页按 `projection.rs` 的六族公开排序输出，无法在获取basis前知道snapshot存储顺序。本次有限fixture明确出现 Q顺序 `carrier/source/terminal`、存储indices `2/0/1`，按真实financial_reader的wanted原序返回完全合法。

建议只改导出器上述比较：请求object_ids须逐字等于已核Q页objects的原序；集合必须等于完整snapshot活动对象且无重复；按每个真实snapshot_index从独立已核snapshot取得记录，再按请求顺序核selected的id/ref/hash/index。不可重写Capture.request，不可丢原件，不可改成仅比较集合。本模块已按这条协议采集，旧导出器保持原字节。

后继导出还须根另行固定实际capture清单和合同原bytes的 `expected_sha`，提供独立持久库source读取器及原完整输入；本模块不能用收到的闭包伪装原库、不能自产pin后调用export。无terminal/K/γ仍允许采集完整读侧原件，后续业务成功义务保持。
