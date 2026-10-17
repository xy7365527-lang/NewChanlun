# #1323 / SPEC #1340：2026-10-17 恢复检查点

状态：推进中，六项 Destination 均未取得完整验收；#1279 保持开放。前端归 #1460，K4 排除。

## 实测环境

- 已读根 CLAUDE.md、Wayfinder 流程、交付纪律及现役线约束。
- 沙盒内 gh 报凭据无效；相同现有凭据通过沙盒外 gh api user 成功认证为仓库账号。未重新登录或更改凭据。
- 远端 main：fdc5c6a7fd2fcb6b8d436d419984c1c826b669f6。
- 旧 /private/tmp/goal1323-native-20261012 整个目录在沙盒内外均不存在；其已注册 worktree 元数据仍在。未删除/修剪旧元数据。
- 018c6542e5、c2c9c5640a 提交对象仍存在，已分别恢复至本仓 .sandcastle/worktrees/resume-1419 与 resume-1488，分支 codex/1419-resume-20261017、codex/1488-resume-20261017。使用 GIT_LFS_SKIP_SMUDGE=1，不认为LFS大件已恢复。
- 主仓 AGENTS.md、CLAUDE.md 既有修改保留；旧未提交修复/设计原件尚未证明恢复。
- 外盘元数据可见约120GiB可用；不等于数据可读/性能验收。148GiB完整双跑门未满足。本机cargo、正式性能/浏览器禁令继续保留。

## 远端证据

- run 38854239929：head 121949ee99959cf728408faa6b8478d9abc27935，completed/failure；含s_structure_session 16项编译错误及缺真实fixture，不是仅既有lib失败。fixture-drift和s-observer job success，不能抵扣其他失败。
- run 38785485944：head c2c9c5640a99b98299456fd8b837c58035a2f4a6，completed/failure（driver-six）。
- 对应JSON和失败原日志保存于本检查点目录；摘要不替代源日志。

## 当前派工

以下早期派工是恢复过程记录；以文末「当前续接状态」为准。

- #1419 恢复审计：定位丢失未提交面、fixture及两source_audit失败的实际依据；只读。
- #1419 codec桥：独占resume-1419的content_structure.rs，修复不存在的API调用，保持JSON与错误语义，补靶向测试；不能用__bool对象替换原布尔值。
- #1488 恢复审计：找冻结设计、准确六测和租约合同；禁止放宽guard或修改construction_state.rs/native.rs/scratch.rs。
- 全图审计：完整 #1323/#1340、原生子票/阻塞/认领及六目标证据核查。

此报告为工作草稿，不是通过报告；未push、合入或关票。

## 后续实证与审查

- 真实BTC fixture已从 ecc9b196d3 原字节恢复，SHA256 0df2091a8723b040988527bcd05612c9643530f9724be68edb52acbcf77c108a。tb02e前缀测试接回同次parser返回的SecondFoldStore；单文件fmt通过，未cargo。
- codec候选首次独审FAIL：局部store未经过parser、obj3/旧关系身份不闭合、旧完整JSON转图不符合共享值边合同。原审报告保留；升级gpt-6-astra重做，未提交为通过。
- 全图原生核对50直挂/27关/23开；11TB下77叶/8关/69开；已关直挂27/27索引齐全。#1463按URL索引可定位，首次字面匹配误报已撤回。
- C6a b6e690ecae无Actions/checks记录、无专用workflow；不能dispatch通用CI冒充验收。恢复工作树 .sandcastle/worktrees/resume-1406-c6a，branch codex/1406-c6a-resume-20261017；专用监督器/测试/workflow重建中，CONTRACT_READY保持false待独审。
- #1419恢复登记：https://github.com/xy7365527-lang/NewChanlun/issues/1419#issuecomment-6224860070
- #1323本轮状态已按即时读取+updatedAt重比协议写回，保留旧记录与所有门。

## 续接并行批次（2026-10-17）

- #1419：同次 store 局部修复已落源码，完整 JSON 桥仍未合格。正在并行实施直接 C/D 值边构造和 tb02e typed 单元共享校验；已冻结构造器初始接口，不改 parser 判据。
- #1488：新八文件候选交独审；已发现 journal Arc 与 try_unwrap 冲突、旧 candidate 账未纳入、raw advance 提前自建账，尚未准入实装。原保护门不删除。
- #1406 C6a：三新准备文件与12离线测试已交，原48合同报告通过；不是完整监督器。CONTRACT_READY=False；Python独审和原资源/guardian合同恢复并行。
- 没有本机 cargo、重型重放、远端 dispatch、push、合入或关闭票。

## 当前续接状态（2026-10-17，取代上面的阶段性状态）

仍为 #1323 / SPEC #1340 全图实施中，未取得任一六目标的完整验收；本地修改未提交。下列通过均仅覆盖所述范围。

- **#1419 直接结构生产**：旧完整 JSON 桥已替换为 C/D/E/F 直接值边构造；typed unit 核、同次 SecondFoldStore、实际 prior 与 CC054 单次聚合已接线。新结构规则域与旧域分开，目录增至22轴，旧域只读不自动迁移。独审明确缺口是具名分类请求的四轴知识状态，目前由原作者补齐；规则域、根接线和 prior 正在独立工程审查。相关报告 `1419-direct-review.md`、`1419-generation-wiring.md`、`1419-units-extraction.md`。所有新增 Rust 测试仅源码，未编译、未执行。
- **#1488 同账 publication**：R/S八文件候选已落。单父scope前移raw、Weak身份避免阻断unwrap、journal局部帽与父帽分离；原三设计矛盾已静态修正。独审仍 FAIL：Plain 临时及持久容器的 retain 费用缺真实释放生命周期，必须继续迁移许可，不能靠统计差额或新账修饰。新审出的全父峰误比缩小validation_limit已由主控改为 `peak > self.limit`；末尾candidate保留费用仍按原缩小限额核对。单文件 rustfmt（edition 2021）通过，未执行Rust测试。详见 `1488-implementation-review.md`、`1488-r-implementation.md`，迁移方案待复核。
- **#1406 C6a 离线绑定**：前轮三项独审缺陷已返修复审通过；最新Q-STARTUP绑定另获独审通过，作者25项、独审25项及11项独立正负控通过，原48合同前轮通过未重复算新证据。报告 `c6a-startup-review.md`；这不证明真实runner、41步、进程回收或远端运行。
- **C6a原合同已恢复**：原派工明确沿C7a资源数，五阶段同t0端点、资源帽、Linux WNOWAIT/subreaper、精确11源六键均有出处。原发行两文件是 `run-1406-c6a.py` / `issue-1406-c6a.yml`，指定分支push触发而非dispatch。详见 `c6a-contract-recovery-v2.md`。原CI v3字节仍未恢复，现按原合同并行重建两文件监督器与41步独立语义验证器；`CONTRACT_READY=False`保持。
- **运行与收尾门**：不启动本机cargo/rustc、正式服务、HTTP、浏览器或重型回放，不访问外盘；没有commit/push/远端运行/main合入/关票。外盘≥148GiB完整双跑前置仍未满足。前端#1460、K4边界不变。

旧 `1419-working.patch` 等补丁仅表示各自生成时点，不是当前全量恢复包。下一次封包须包含各工作树当前完整diff、未跟踪源文件和哈希，禁止把旧patch称为最新完整实现。

## 本轮后续推进与可恢复快照

- #1419工程独审发现发布前source核只接A/B，以及HEAD既有rule缺参；已重派来源审计工位补新域同核，新增source_structure.rs及必要source_pending/source_contributions接线获主控授权。请求二元键、外部slot边规范渲染和目录具名轴归属已返修静态复读；请求四轴专项与oracle历史域投影正在复审。详见 `1419-wiring-engineering-review.md`、`root-integration-fixes.md`。
- #1488主控已批准 `1488-raii-migration-plan.md` 的实现方向，两工位分别负责库内实际owner与bin raw适配。冻结三个禁区、旧成功合同和资源帽；正在实施，未闭合全域。
- C6a监督器首版27项纯替身测试通过，`c6a-linux-supervisor-candidate.md` 明确verify_runtime尚PENDING；独审已发现回收失败时scratch原件不在workflow上传范围，待返修。
- C6a语义候选 `c6a-semantics/REPORT.md` 已冻结：10项纯数据测试通过，包括WE5完整20步正控；全41步合成wire按原容量拒绝（12个WE7响应超限）。编码已是正式紧凑格式，不能去空白解决；数据仍来自替身，不能外推为真实服务失败。完整41步合法wire正控尚缺，绝不据此翻运行资格。
- 已封当前阶段字节快照：`snapshot-1488-before-raii/`（8文件）、`snapshot-1419-direct-request-review/`（15文件含fixture）、`snapshot-1406-c6a-supervisor-review/`（5未跟踪准备/发行文件）。各含MANIFEST、完整changed-file副本与工作diff；均标未验收，后续改稿将产生新指纹。
- 部分显式模型工位长时间无回报，已interrupt后以继承当前模型重派；重派工位已确认实际开始。没有把路由无回报猜作额度或认证故障。

## 最新续接：独审与第二批迁移

- #1419 请求四轴专项及历史 oracle 域投影已获限定静态通过，见 `1419-request-rereview.md`。来源审计候选已落五文件，见 `1419-source-audit-fix.md`；独审正在核对，并已发现合法多代 change 的事实代次/结构代次混用，以及 relation 端点未与 witness 内容绑定，须返修，不能宣称来源核通过。
- #1488 A/B raw RAII 切片限定静态通过，见 `1488-raw-raii-review.md`；9+3 项 Rust 源码锁未编译、未执行。第二批正在改 TextValue/RefValue/RefFields/JsonOwner 内部持有者，source/history 与 index/control 消费面并行核对。既有 raw 静态结论不覆盖第二批新字节；warm restore、六成功及全域释放仍未验。
- C6a 监督器 v2 原两项缺陷与新增隐藏原件上传遗漏已关闭，见 `c6a-supervisor-rereview-v2.md`。32 作者测试与5独立正反控沿用，配置一行修改未重复全测。workflow SHA256 为 `75a06f24302d861c09630a99579192e815443a2a1c51a0ea4ea31ae919bfeace`，仅专用证据目录启用隐藏文件上传。
- C6a 语义器两 HIGH 已返修，作者14项纯数据测试通过；新 semantics SHA256 为 `731fffd517420ab4c749bae2fe6987b25e7ee3722365ae9bc27551c92533208d`。独立复审在进行。真实外层尚缺独立精确时间锚，全41步合法 wire 正控仍缺；原 REPORT/MANIFEST 属上一版历史，不能当新版本审包。
- 保持所有运行/资源/合入门；本轮未进行本机Rust编译、正式服务、外盘、push、远端运行或关票。

## 最新在飞批次（上述历史结论按各自指纹保留）

- #1419 来源两 HIGH 返修 v2 已冻结，报告 `1419-source-audit-repair-v2.md`；新增6项源码测试全部 NOT_RUN，原审已开始复审。生成器/分类器未为通过审计而改判据。
- #1488 第二片 A 后续/B2/字段缓存与主控Q兼容获限定静态通过，见 `1488-plain-slice-review.md`。15文件稳定字节已封 `snapshot-1488-plain-slice-reviewed/`，包含未跟踪 native_owned.rs；明确不是整树验收包。来源两个消费文件正在实施，主控为它接上只读 producer 初始化 owner 和 typed witness 输出，见 `1488-root-compatibility.md`；这部分新字节不在前一片审结范围。
- #1488 共享 Rc/Arc 与弱引用控制块仍在设计；历史/SQL/对象索引广泛消费面未完成。`1488-publication-route-check.md` 解释现有正式 publication 路由，未以旧辅助函数存在作误报，仍保留 warm restore/跨请求费用交接验证。
- C6a 语义两 HIGH 返修获限定离线 Approve，见 `c6a-semantics-rereview-v2.md`；独立复验包含20秒迟到重基、缓存投毒、已核字节读盘替换等。新清单 `c6a-repair-v2-manifest.json`，原历史 MANIFEST 未覆盖改写。
- 主控批准 C6a 外层时间锚方案，实施在飞。W 取 runtime Popen 前同一个 started，解释器与首次 IPC 计入原600/630，是明确的保守前移；case/q 仍原300/10，外层五端点不变。见 `c6a-external-anchor-plan.md`。旧监督器静态审结不覆盖新 broker；所有开关仍 False，source/tree PENDING。全41合法wire正控的具体缺口另有只读工位分析。

## 当前继续批次：2026-10-17（取代以上“在飞”状态）

- #1419 来源 v2 的两 HIGH 已限定静态消除，见 `1419-source-kernel-rereview-v2.md`。19文件最新完整恢复包为 `snapshot-1419-source-v2-reviewed/`，MANIFEST SHA `71273d6a824d11d295310c0ee327b6aaa08eb809cdec43e2614be3e98f31f0a9`。`1419-remote-validation-readiness.md` 明确旧16错误形状静态修复、真实fixture已封，但没有源码提交或编译结果；现有 ci.yml dispatch 会启动广泛 jobs，不能当仅编译本票。不得拿旧包/旧远端结果当本轮验收。
- #1488 共享 Rc/Arc/Weak owner、来源前缀迁移、scoped字段/witness缓存测试获限定静态通过：`1488-shared-owner-review.md`、`1488-prefix-memo-review.md`。source_order迁移与主控 allocation_bytes 统计接口亦已限定静态通过，见 `1488-order-stats-review.md`；首审的默认feature引用testonly模块缺陷已修。所有Rust测试 NOT_RUN。主控另改 `history.rs` 的 CodecRead 实现接PlainJson/typed Native refs，报告 `1488-history-reader-compat.md`，独审在飞；不是整个history完成。
- C6a 外层时间锚序号类型漏洞及协议类型契约已返修并获原审 Approve，见 `c6a-external-anchor-rereview-v2.md`。监督器53、driver62与独立17项均为纯替身；最终runner SHA `3a661d69a43ddaad19e353ca2d3abd35696b05050a1338d846ee5bd1769a72d7`。尚未嵌入完整独立语义器或真实Linux验收。
- C6a 冻结完整成功合同与1MiB单正文帽的静态冲突已独审坐实：仅证明内96份RAW字符串即1,093,344 B；全部114份为1,298,346 B。报告 `c6a-wire-bound-review.md`，登记 https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6226271522 。这不是实跑故障，也不定位最早失败步骤。
- **用户已明确批准具体分页提案实施**，原话“批准按提案实施”。提案 `c6a-wire-revision-proposal.md` SHA `9edcded8451f1edb20cc45c4a3752cb3741556d37fded08c49e04de52bbdc6b6`；批准登记 #1406 comment6226368339、SPEC #1340 comment6226369555。无需重复请求这份协议的spec批准。保41业务步骤和全部原件证明；每帧1MiB、逻辑文档及共享cache4MiB、整逻辑读取30秒不续窗；不批准扩大资源帽或合入main。
- `c6a-pagination-contract-v1.md` 由主控核认字段冻结：三op EconomicReadOpen/Page/Close，页版本economic-read-pages/1，Q POST /api/economic/read，整数十进制字符串、stride262144、精确descriptor绑定。共享生产客户端 `read_economic_pages(query, fetch, ...)` 位于 s_socket_client.py；独立semantics不得复用它自证。Close失败不交付，受原期限；跨进程传remaining毫秒而非monotonic绝对值，根端绝对门仍承重。合同作者还在追加逻辑kind投影细则，不得各工位私改字段。
- 分页在飞归属：`c6a_owner_pagination` 独占恢复树economic_session/service.rs/store.rs；`c6a_q_pagination` 独占economic_query.py/economic_http.py及test_c6a_economic_pagination.py；`c6a_client_pagination` 独占s_socket_client.py及test_c6a_page_client.py。driver与离线语义器尚未接新分页，需接续明确工位。这些Python改动须独审及纯替身检查，Rust仍不在本机编译。
- 未执行本机cargo/rustc、正式服务、真实HTTP/socket/浏览器、外盘或重型回放；未commit/push/远端运行/main合入/关票。原全历史≥148GiB门仍保留，容量未重查。前端#1460和K4边界不变。
