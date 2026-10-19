# C6a 完整证据传输修订提案（待 SPEC 决定）

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。范围：供主控审查的协议修订方案；**不是已批准 SPEC，不实施、不启用、不改源码、不授予运行资格**。本次唯一写入为本文件。

## 推荐及证据边界

推荐先做**贯通 owner→Q→验收客户端的固定 cut 字节分页**：传输的是原完整 canonical JSON 的连续字节片，不改变事实、证明、账目或完整图像定义。保留 41 个业务步骤和每步全部观察断言，只增加有界、只读的证据传输轮次。内容引用去重可以另作优化，不作为本次通过的隐含前件。

该推荐解决已证明的“完整逻辑结果必须装进单个 1 MiB 响应”矛盾，**尚未证明当前真实全 41 步能在原缓存/时间/内存帽内成功**。尤其真实完整逻辑结果是否全部适配下文建议的 4 MiB 聚合帽，尚待来源冻结后的具名计量/验收。若不适配，不能临时涨帽、裁事实或改成功定义，必须回 SPEC 决定。

复用独立证明 `c6a-wire-bound-review.md`：冻结 RAW 10,069 B，作为 canonical JSON 字符串为 11,389 B；最终必需 114 份，仅这些值为 1,298,346 B。甚至 B 证明内部的 96 份已为 1,093,344 B。此证明不使用替身总大小代替产品读数。

## 已读目标和合同

1. 证据根 `1323.json` 的六 Destination 与红线：唯一结构/经营 owner、完整历史及同核/唯一真账、资金隔离恢复、#1304 接入、完整验收前不关 #1279。前端 #1460、K4 不纳入此修订。
2. `1340.json` 的 C08（正文行 312–320）、C09（322–336）、测试决定 AT-04/07/11/13/14（412–422）与独立性要求（430–432）：固定切面、历史当时可知、有限事务、慢读不拖写、原件与恢复完整、外部可见核验。**C08/C09 已明确的 S 分页原则不等于经济协议分页已获批准**；扩展至 E/B/Q 是本提案的新技术选择。
3. `c6a-contract-recovery-v2.md` 的原始派工与资源数：WE5 20 + WE7 21；真实 E/B 和正式 Q HTTP；精确六键构建证据/11 源；外层同 t0 的 1200/1830/1920/2040/2070 秒；内层 work600/case300/final630/startup10/exchange30；Q reply/frame1 MiB、cache4 MiB、snapshots4、domains4、changes32、operation30秒；运行组 RSS2 GiB、文件系统增长6 GiB及原余量门。
4. 当前 `s_session/tests/tb03c_query_runtime.py`、`economic_query.py`、`economic_http.py`、`s_socket_client.py`，`rust/src/economic_session/service.rs` 与 `store.rs`。离线验证器当前 `c6a-semantics/semantics.py:291-355` 精确检查全部 wire/请求/返回与业务顺序。

图工具对经济 read/serve/select_cut 返回零后精读源码。本提案不依赖远端新状态；本轮没有网络、产品服务、cargo/rustc 或外盘操作。

## 当前 API 的真实边界

| 层 | 当前行为 | 不能只改哪里 |
|---|---|---|
| owner | `service.rs:655-698` 选择一条持久 commits 图像；`:711-748` ReadView/Watch 返回完整图像/完整变化集合；`store.rs:667-690` 已持久存 canonical image，且检查统一可读安全域 | 当前没有经济图像分页操作；不能声称直接调用现成经济 page API |
| owner→Q | `economic_query.py:458-477` 用 `exchange(...max_frame_bytes=1MiB)` 读完整响应；`s_socket_client.py:108-157` 单 JSON 行、超过帽拒绝 | 只分页 HTTP 仍可能在 owner 响应处先失败；必须覆盖这段以及 driver 直接 observe |
| Q snapshot | `economic_query.py:515-528` 用 canonical(body) 同时核 1MiB reply 与 4MiB cache，缓存整 body；token 是 body 摘要 | 仅增加新 HTTP route 不会移除 `_store` 的逻辑单帧限制 |
| Q watch | `economic_query.py:611-643` 按域核完整 changes、连续 seq、同 cut image，最后核整个 result≤1MiB | 仅改 snapshot 仍留下 watch 的相同失败；连续变化核不能删 |
| HTTP | `economic_http.py:104-111` 对实际 reply envelope 核 min(Q帽,server帽)，再发完整 canonical bytes | 每页都必须核完整 envelope；不能只核 chunk 字节或借 server 的 8MiB 配置放大 Q 的1MiB |
| decoder | `decode_json_value/decode_frame` 拒重复键、非精确小数/常量；`validate_reply` 核版本、会话、生产者/epoch、请求因果与 payload hash | 新页应作为合法版本化 payload；不能修改为容忍尾包/多帧/模糊类型 |
| driver/oracle | `check_public` 每步核 current/fixed/watch/as_known/old_fixed/token；`check_snapshot` 比完整 canonical image；`We7Expected.records` 比原 RAW 与完整 proof | 页必须先重建同一个逻辑结果，再原样执行这些断言；不能让“某页 Available”替代完整成功 |

结构域 `s_session/s_query.py:304-362` 有固定切面 token、offset、完整投影摘要的先例；可复用原则和反例，不能照抄其按 records 分页。这里**一个含嵌套证明的 record 自身也可能超帧**，故推荐按原 canonical 字节分页。

## 两种方案比较

| 项目 | 内容引用去重 | 固定 cut 字节分页（推荐） |
|---|---|---|
| 表示 | 原件/证明对象按 hash 一次传，树节点变引用，另带完整闭包 | 原 JSON 不变，切成定长上限的字节片，逐片传 |
| 能否保证每帧有界 | 去重后单一独特对象或闭包仍可能超帧；最终仍需要分片 | 包括任意大单记录在内均可分片；逻辑总量另受聚合帽 |
| 真实性工作量 | 新 typed-ref 语法、转义冲突、完整闭包、引用身份/类型、循环与展开放大验证 | 页顺序/长度/摘要、固定向量及完整重组；原 JSON 验证规则继续使用 |
| 对生产存储 | 可只做 wire codec，也可改持久 DAG；后者跨资金原件来源边界更大 | 可保留现有 commits.image 原字节及所有资金写路径 |
| `_store` | 仍须明确 cache 计压缩表示还是展开表示，单改wire不够 | 去掉逻辑结果≤reply帽的绑定，保留每帧帽与逻辑cache帽，各自核账 |
| 原 oracle | 先独立解引用再核原完整图像；禁止生产解引用自证 | 先独立拼字节、核hash/strict JSON，再核原完整图像 |
| 传输/CPU | 原 RAW 重复量可显著减少，但收益尚无真实产品计量 | 重复原字节全部保留，传输量不省；增加轮次，须在原30/300/600秒内验证 |
| 最小可交付边界 | 若不配分片只能承诺“这次压得下”的受限域 | 可先不改持久schema/财务reducer，集中完成有界读取协议 |

因此不推荐“先去重直到现有 fixture 能塞入 1MiB”作为闭合标准。它不能保证已声明的完整原件有效域，也容易将单样本压缩率当协议容量证明。若未来选择去重，应由独立版本合同声明，并保留分页作为单对象/总闭包越界时的合法传输方式，而不是判据失败后的宽松兜底。

## 推荐协议草案：完整逻辑文档、有界页

下列 `EconomicReadOpen`、`EconomicReadPage`、`economic-read-pages/1` **是建议名字，不是现有 API**。最终 op 名、版本归属与 ACL 需 SPEC 明确后实现。建议页协议嵌在现有已认证身份 envelope 的 payload 中；金融写消息及持久账目 schema 不改变。

### 1. 逻辑查询先固定，再读页

逻辑查询仍是当前 snapshot 的 `(cuts, as_known_ns, snapshot_token)` 或 watch 的 base cuts，以及 owner ReadView/Watch 的选择参数。开读时只解析一次“当前”前沿；返回的描述符至少绑定：

- 页协议版本、原逻辑操作及完整 selector 摘要；session/generation、manifest_hash、预期响应 owner/Q epoch；
- snapshot 的完整四域 cut/commit_ns/image_hash，或 watch 的完整 base/next 及按 seq 排序的变化身份；
- 原逻辑文档的 canonical 编码标识、完整字节长度、完整 SHA256；稳定分片步长/总页数；
- 服务端只读 transfer 身份，关联实际缓存项和总截止。该身份不能只是客户端可任意重算的 checksum；服务端须与原签发描述符逐项重核。

snapshot 的原 `economic-snapshot-<digest(body)>` 语义建议保留：逻辑 body 与原模式/as_known 等字段不变。新增 transfer_id 是一次传输身份，不能混充原历史 token。第一次 open 必须取得完整逻辑 body 摘要后才能声称固定成功；不允许以“稍后最新值”填缺。

watch 在 open 时同时固定每域 next cut；后续页不能继续追新前沿。原 max_changes=32 对**整个逻辑 watch**计数，不能每页重新获得32条额度。逐 cut 连续性、同cut图像及最终前沿一致性仍按现有 `watch` 全验。

### 2. 每页只读原字节区间

续页请求仅带原 transfer 身份、固定描述符摘要及确定 offset/index。成功页携固定身份、offset、raw_byte_count、片摘要、片载荷和 done/next。载荷建议 base64 表示 canonical 字节片，避免 UTF-8/JSON escape 在边界切开后被错误解码；这是需要批准的 wire 编码选择，不是内容压缩。

页内容为原完整逻辑文档对应区间，无删改、重排或事实投影。每个页 envelope 的身份/因果/payload_hash 继续由现有公共头验证。接收端核 base64 合法性、长度、片摘要、严格递增无洞 offset、终点恰等于总长、最终全文件摘要，再执行严格 JSON 解码及原逻辑断言。中间页不得标为 CompleteVector 或提交半个变化批次给业务消费。

分片容量由完整规范 envelope 实际长度推导：`4*ceil(raw_chunk/3)+envelope_overhead <= effective_frame_cap`；owner 单行还要计 LF，HTTP 正文计完整 reply envelope。固定 stride 应根据该 transfer 最大 offset/index 数字宽度和固定 metadata 算出，随后逐帧再核实际大小。不能拍一个“1MiB chunk”后忽略 base64 的4/3膨胀。

重复读取同页允许返回同一载荷，不新增经济 commit。非末页必须正长度、offset 必须严格推进；页数由已核总长与 stride 推导，不能无限空页。对错页/跨域/跨代/错hash/缺页/多尾字节直接失败，不降格后计成功。

### 3. owner 端须同时提供有界读取

现有 commits 表已经有 `seq/root/commit_ns/image`，图像由 `readable_image` 存 canonical。建议为只读读取增加短事务路径：先固定 cut 与不可变图像身份，再按字节区间读取。SQLite TEXT 按字符截取不能当 UTF-8 字节分页；实现需显式按 BLOB 字节或其他经验证的等价方式取片。所有页重新核 session/domain/cut/root；不能把客户自带任意长度/hash 当权威。

ReadView 的逻辑 envelope 元数据及 Watch 的有限序列由同一固定描述符定义。持久图像可分片送往 Q/driver；现有完整 audit/根验证不因分片删除。**此提案不承诺现有审计和 JSON 验证已经能流式运行或满足时间帽**，需要实现审计与有界测试证明。不得每取一页重新全表审计后把重复费用排除在总截止外。

不把游标租约变成金融责任，不为读取跨请求持有 SQLite 事务，不让慢读阻塞写者；保留事实的生命周期仍由原持久合同决定。分页只读不产生新的资金 Capture/Resolve/Apply，也不改变 X 一次调用权或恢复封闸。

## `_store`、缓存和费用责任

本提案的关键新取舍是把**每帧上限**与**一个逻辑查询总量**区分，不能只删除 `len(encoded)>max_reply_bytes` 检查。

建议第一版沿用既有 max_cached_bytes=4MiB 作为**单逻辑文档聚合字节上限及现有共享缓存总帽**，max_snapshots=4 继续是最多保留数量，不承诺四份任意大小文档同时驻留。此复用属于新增保守准入规则，需 SPEC 明写；不是声称旧合同已有“逻辑文档4MiB”字段。

Q 缓存实际完整 canonical 文档或等价的完整分片集合，以同一逻辑字节总数计账；不能只记描述符几十字节却把正文放到不计费的文件/owner引用后宣称满足缓存。元数据、传输缓冲、base64临时副本、解析对象、进程运行内存另计真实资源开销，受原运行组2GiB及既有局部约束，不能把4MiB payload帽当真实RSS证明。

snapshot 与 watch 的活跃传输共享既有总缓存/数量预算。建议将 watch 活跃项也计入同一最多4项和4MiB，结束即释放；这是比旧仅snapshot保留更保守的新选择，可能影响token存活期，须独立验证现有 `old_fixed` 随即 token 重读成功，不能用新淘汰策略放过此正控。若实际并发模式需不同划分，回 SPEC，不另建无帽 watch 缓存。

owner 获取、Q 验证/编码、driver 重建、独立验证器、归档原件分别拥有各自真实缓冲，费用不可互相隐去。同一文件系统增长6GiB覆盖所有新增页原件、索引、临时重组与归档；原最低磁盘/内存余量继续检查。不能把源/证据迁到未监测目录规避总量，也不增加外盘权限。

一项逻辑查询从 open 前计时，所有页、重试、base64编解码、完整hash、strict decode和oracle处理共用**原30秒绝对截止**；每页传输只取剩余时间，不能重新获得30秒。client每页调用、Q处理与owner子请求都应传递/钳制同一剩余窗，服务端保留原签发截止，不依赖客户端自报延期。case300/work600/final630及外层五阶段端点不变。缓存固定历史token可另起一次合法查询，但一次失败分页不得偷偷用新查询身份续命。

## 41 步及独立验收怎样保持

**可以在原41业务步骤下仅增加证据传输轮次，但必须显式改 C6a 传输协议和证明合同。** 现在的 semantics 已区分第一次 `step-/we7-` 业务消息和其余只读 wire，41是业务写顺序，不是总HTTP次数。

建议维持两个账：

- 业务账：WE5原20、WE7原21，原payload/业务message ID/outcome/cut/重复消息回执一项不减。每步后仍执行原current、fixed、watch、as_known、old_fixed、token读取，以及 owner before/after、精确重送无新cut断言。
- 传输账：每项逻辑观察有独立query身份、固定描述符、开始/截止和连续页序号；所有实际请求/响应原件逐条hash/长度/身份/因果/页范围可追。重复页有完整记录，不算业务步骤，也不能冒充新的经济事实。

driver 的 `Case.owner` 只对只读完整观测增加分页适配，业务写路径保留原请求重送规则；`Case.http` 重建完成后向 `check_snapshot/check_watch/We7Expected` 返回与旧版同形的完整逻辑对象。不是把这些断言移到服务器或删除，而是把物理多帧和逻辑一结果显式分层。

离线 semantics 必须更新对页协议的精确字段检查，独立实现页范围/长度/hash/根/顺序核验，并复核全部原始wire。它不能直接调用生产重组函数后认为协议已证；也不能只验证driver保存的重组JSON。现有冻结RAW/HAND和金融独立oracle继续提供业务预期，未新增“接受缺页”等宽松臂。源码 hash-before-exec、外部时间锚、进程生命周期绑定继续保持。

当前11源/六键构建证据要和新源基线同步：若将辅助逻辑放入现有11文件，仍逐项冻结；若新增Python codec/assembler依赖，则必须明确扩充被核源码闭包和名单版本，不能保持11名单却执行未覆盖新helper。Rust所有新增只读路径须绑定同一 source tree 和真实E/B构建hash。发行两文件尾差规则仍作用于**新的已独审源基线**，不把产品改动夹进CI尾差。

## 断线、恢复和隔离

1. 在同transfer有效期内，从最后完整且hash正确的页边界续读；半页原wire保留但不能应用。重复/断线只涉及只读证据，不触发金融重送。
2. Q重启或缓存淘汰后原临时transfer失效，返回显式Gap/SnapshotExpired；不能换最新cut续接。若允许重新open，必须以原固定完整向量重新验证，且不能为本次受限任务重置case/work截止。原业务receipt/cut及历史token语义不删除。
3. owner重启/epoch变化须重新确认获准epoch和持久cut可读，旧transfer不得静默跨代。history可用才重新取得同根；缺项准确失败，不能借另域副本作为新权威。
4. 读错误不能回滚经济提交或自动释放责任。慢Q、坏B、缓存满仅结束本读；健康S继续正式产出，健康其他域的读取机会仍按原并行隔离合同保留。
5. page描述符/临时缓存可回收；长期事实、业务去重身份、Capture/Resolve/Applied原件、责任及尾账不随其回收。没有新金融费用政策，手续费/分摊仍来自原RAW/HAND；这里的“费用”仅指计算/IO/缓存/传输资源归属。

## 必须明确批准的 SPEC 取舍

| 决定 | 推荐文字 | 不是已有事实 |
|---|---|---|
| 经济读取新增版本 | 固定cut的canonical字节分页贯通E/B/Q及公共客户端，现有经济写语义不变 | 当前没有该API或能力声明 |
| 完整性单位 | CompleteVector/完整changes是全部页经核验后的逻辑对象，单页不具备该资格 | 当前协议单响应即完整对象 |
| 容量单位 | 1MiB继续限制每物理帧；逻辑文档和所有保留文档受4MiB聚合/共享帽，活跃watch同账 | 旧 `_store` 同时要求完整body≤1MiB；更改此条件须明示 |
| 时限单位 | 一项逻辑读所有页共享原30秒，不延长case/work或外层端点 | 每页重新 `_start()` 会改变合同，必须禁掉 |
| 字节表示 | base64连续canonical字节片，hash/长度/固定身份逐层核验 | 不是已有wire格式，不得当透明内部优化 |
| 断线恢复 | 同固定向量续读；失效显式Gap，不拼新cut；不复制金融动作 | 需新增token/恢复反例证明 |
| 兼容退出 | C6a新验收选定新只读版本；旧单帧读取仅保留明确旧版本受限兼容，不作为新失败后的另一判据 | 后续需点名调用方/升级与退出条件 |
| 证据口径 | 41业务步骤不变，物理读取轮次另账且全部保留；新源码名单/基线重新独审 | 现有41步wire工厂不能原样当新协议已通过 |

上表获明确决定前不改已签 SPEC 原件；通过新的决策/实施承接记录修订，引用旧签署字节保留发生史。若现有 #1406 无权承接跨owner/公共协议裁定，主控应按Wayfinder查重并交对应决策票，不能把准备票实现者当架构批准者。

## 源码职责与后续可并行实施边界（未派发）

| 职责 | 实际责任面 | 必要验证 |
|---|---|---|
| owner有界原图像读取 | Rust `economic_session/service.rs`、`store.rs`只读路径；新op验证/ACL | 固定cut/AsKnown同根，单记录跨页，短事务，epoch/坏根/超帧/慢读负控 |
| Q逻辑查询与预算 | `economic_query.py` 的 owner/view/_store/snapshot/watch | 聚合4MiB与每帧1MiB分别锁；watch连续性不减；缓存淘汰/并发隔离 |
| HTTP与页编解码 | `economic_http.py`、`s_socket_client.py`及明确登记依赖 | 实际envelope与LF/HTTP头边界、重复键、UTF-8分片、错页/截断/巨大长度负控 |
| driver与离线独审核 | `tb03c_query_runtime.py`、contract tests、C6a semantics | 原41业务账和全部观察断言；独立重建；外锚/总期限/全部wire原件绑定 |
| 源码/资源/恢复最终资格 | 主控与独立审查 | 精确新基线、二进制/依赖闭包、原总资源帽、真实进程及断线/重启验收 |

协议字段、版本、摘要对象和预算归属冻结后前三类可并行；driver/独立核可按同一冻结合同独立实现。不能为了并行让多工位各自定义token或费用口径。

## 接受本提案后的最小验证顺序

1. 先纯数据codec边界与对抗控：跨UTF-8/escape切片、超长单record、错hash/缺页/重页/重叠/尾垃圾、跨cut/epoch、元数据超帧、逻辑总量和缓存过帽。只验证协议，不冒产品成功。
2. 保留原冻结输入与全部断言，计算真实生产完整逻辑文档大小及峰值责任，确认新4MiB聚合规则可接受全部受测步骤；任何不适配留为阻塞，不改RAW/HAND/帽。
3. 新源码精确独审后按原授权门安排具名远端真实全41步，全部新增只读wire原件保留，实际RSS/磁盘/时限同原合同。正控必须完整成功，拒绝样例只是负控。
4. 对已声明的分页续接/Gap/进程恢复/局部隔离另做实际验收；C6a正常轨通过仅证明其范围，不直接代销#1340 AT-04/07/11/13或#1323全图。

当前状态：**方案已具体化，待主控审查及明确 SPEC 决定；实现、运行、关票资格均未开启。**
