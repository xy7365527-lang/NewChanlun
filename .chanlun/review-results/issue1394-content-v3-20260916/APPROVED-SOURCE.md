# #1394：完整证据的公共身份与引用方案（待裁工作草稿）

起稿日期：2026-09-15；接口增补：2026-09-16。归属：#1394 的接口决策准备，承接 #1340/R2 的 C08/C09。最初盘点基线：`861ee7198f34e03ed7302f9be120a3d28b143339`；第 5 节查证基线为 `bea37f50b23efc0c0d4a58ecc4da7103afcadbd7`。**本稿尚未批准，不授权更换公开 ID、修改既有数据库或上线新协议。** 已获批的私有无损存储、同核增量证据与精确原件校验继续实施。

建议一并引入“版本化批次清单”和“增长证据的有序内容引用”，并同步调整 Q 的全切面摘要、E/B 的对象摘要及前端大字段读取。仅改变批次 Hash 仍会反复展开 CC-007 和 CC-054，消费者目前也会重算完整投影。新方案完整保留每条事实、来源、比较与历史，改变内容身份的计算对象及传输方式。独立消费者盘点见第 4 节；具体编码、字段、读取与资源候选已集中在第 5 节，等待同一次采纳决定。

## 1. 决定所依据的实际证据

冻结来源含 9011 条原始 bar，SHA256 为 `76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087`。同一生产 parser 的完整结构计数得到 9011 个包含组、9011 个步骤、581 个分型；每一步的来源恰为截至该步的全部前缀。

| 当前公开表示中的项目 | 最后一代 | 全部 9011 代或对象内容版本累计 |
|---|---:|---:|
| CC-004 的 `input_refs_json` 内部字节保守下界 | 8,689,172,135 字节 | 26,105,183,015,770 字节 |
| CC-007 的后续 bar 来源和 24 个比较数组：对象 Hash 输入的部分下界 | 未在本稿汇总 | 14,966,557,887,595 字节 |
| CC-054 中仅 CC-004 请求的 `source_coords` 数组精确字节数 | 274,736,258 字节 | 813,256,848,433 字节 |

这些数值是**现行逻辑展开表示和 Hash 输入的体积**，不能相加当作磁盘占用，也不是已完成的 9011 条 S 运行、实际 I/O 或耗时预测。下界省略实际非空身份值、嵌套字符串转义及其他字段。CC-007 仍有 2,615,078 个不同“分型×后续 bar”单元、62,761,872 个逻辑比较值需要完整产生；共享引用不能删除这些事实。

独立控制：已经实际运行的原源前 16 代 S 批次中，分型出生、后续 bar 数、每 bar 的 24 个比较、原始引用字段和 CC-054 来源数组均与计数一致。证据与限制集中在 [1394-OUTPUT-FOOTPRINT-COMBINED-VERIFIED.json](./1394-OUTPUT-FOOTPRINT-COMBINED-VERIFIED.json)，不以小例替代完整运行。

**补充实跑（2026-09-16，私有 S 基线 `71248663f218fc18812a54b832658c18d860fbd8`）：** 最终二进制实际提交同一冻结来源的前 64 条，请求与原控制的时钟逐字节相同。64 个批次共 126,942,162 字节的完整原件、65 个 AsKnown 切面（0..64）、当前重算切面和财务结构依据均与对应原件一致；SQLite 主库页空间由 173,314,048 降至 18,145,280 字节，约减少 89.5%。这证明私有共享的有界效果；没有把页空间当作含 WAL 的总占用。完整证据见 [最终64条原件与切面对拍](./1394-FINAL-PREFIX64-VERIFIED.json)。

同一 64 代库的正式 Q 启动仍超时，实际 `/api/ready` 返回 503。保持原捕获 2 秒、验证 5 秒配置的独立冷读者，在约 0.073 秒完成捕获后于验证阶段超限；16 代对照通过。随后加入单次捕获内有界 SEQ 帧复用，全部 677 根、168,990,867 字节仍与旧实现逐字节一致，111 项 Python 检查通过；64 代冷读者仍在约 5.089 秒超限。该微优化没有证明 Q64 已可用。证据见 [HTTP及进程清理](./1394-FINAL-PREFIX64-HTTP-R2.json)、[根原件对拍](./1394-Q-FRAMES-ROOT-PARITY.json)、[保持资源配置的冷读者复现](./1394-q64-after-frame-reuse.json)。这些实测进一步区分了“完整原件保持一致”和“端到端在资源内可用”；仍不能推断候选 C 已达标或给出 9011 条完成时间。

**最新实跑（2026-09-16，私有实现 `b7734b70995533c88f4df5f88e3b514a8d46325d`）：** 128 条 FinancialBasis 在 66.08 秒内完成导出，116,747,345 字节与旧控制全文一致，但仍超过现行 E/B 整份读取的 16 MiB 上限。256 条运行收到 131 个 Committed 后，下一请求在原 60 秒上限处返回 DeliveryUnknown；事后只读核实该请求已持久发布为 generation132，不能把它计作客户端已收回执。前 128 条完整回执字段经保留 JSON 类型差别的对拍一致。冻结 132 代库的独立冷／暖审计分别为 45.85／8.28 秒，两次完整审计均返回 132 条原始事实，未发生节点读集拒绝或证书失效；暖审计仍因句柄缺失发生 566 次根展开。这只定位该冻结状态的缓存成本，不代替连续写入性能、256 条成功或全 9011 条验收。原运行、对拍和诊断绑定见 [当前检查点](./FRONTIER-CHECKPOINT.json) 的 `prefix256_span_terminal`、`audit132_diagnostic`；128 全文证据见 [导出对拍](./1394-FINANCIAL128-SUBTREE-SPAN.json)。

已签 SPEC 的 C09.4 要求内容寻址的不可变批次对象、完整持久化后原子发布和版本索引，文字没有指定 Hash 必须吃每代完整快照。C08 的固定 cut、全部字段、当时可知历史和无跨页长事务仍约束新方案。改变现行实现的公开 Hash 配方、来源引用和分页结构仍是新接口决定，不能归入“私有压缩”。

## 2. 选择范围与身份含义

| 候选 | 改变范围 | 能消除的重复 | 留下的成本和代价 |
|---|---|---|---|
| A：继续现有私有优化 | ID 与公开原件完全不变 | 重复存储、部分解码和同核构造 | 每代完整批次与增长对象的原 Hash 输入仍需展开；不能承诺 9011 条可在可用资源内完成 |
| B：仅版本化批次清单 | 新版本 `batch_id` | 未变完整对象在批次 Hash 输入中的重复 | CC-007 的约 14.97 TB 部分对象输入、CC-054 的约 0.813 TB 所选字段仍在；仍需后续接口改动 |
| C：批次清单加有序内容引用（建议） | 批次、增长事实、Q 切面和 E/B 对象投影的内容身份，以及片段读取 | 同一完整证据前缀在生产者和消费者之间均可由不可变引用共享 | 需要一次协调 S/E/B/Q/前端版本及 crosswalk；所有真实新增比较、当前完整依赖校验和完整导出成本仍存在 |

本稿使用以下词义，避免把几个不同身份合为一个 ID：

| 名称 | 含义 |
|---|---|
| S 发布序号 `generation` | 本 S 会话的实际发布顺序；不是 Voice 代际、结构级别或 raw 坐标 |
| 业务槽位 `fact_key` | 当前事实家族用于识别对象位置的业务键；不证明内容相同 |
| 内容版本 `object_id` | 绑定某一明确版本规范下的完整不可变事实内容；修订可产生新 ID，旧内容仍可读 |
| 批次身份 `batch_id` | 绑定一次完整可达状态及其实际发布上下文的清单；不能仅绑定变化数量 |
| 私有存储节点 | 内容落盘与共享的实现句柄；物理切块、平衡树形状或压缩算法不能决定公开身份 |

## 3. 候选 C 的合同边界

**内容规范。** 新版本定义一种唯一、确定的有类型内容表示：标量、对象字段、有序序列及内容引用各有明确编码和 Hash 域。字段是否有序遵原合同，数组次序、数值类型、缺字段与 null 的差别全部保留。序列身份由逻辑值、顺序与规范版本决定；若使用树状承诺，其逻辑分解规则必须在公共规范中固定。相同逻辑内容不能因物理切块、缓存淘汰、AVL 旋转或 codec 改变 ID。具体版本号和编码字节属于随后 SPEC 增补，禁止实现者自行补默认。

**增长字段。** CC-004 的完整输入来源、CC-007 的完整比较单元与来源、CC-054 的完整请求与坐标序列按该规范共享。每个比较单元仍由当前同核真实计算，保留具名参考价、轴、结果、实际原始 revision 及其顺序；从引用读出已存事实不能替换成重新执行业务判定。未增长的字段也纳入完整内容绑定。不得以采样、近 K、少比较、删除重复业务事实或省略来源降低体积。

**嵌套 JSON 原件。** 当前对象的 `payload_json`、`input_refs_json`、`source_coords_json` 本身是 JSON 文本字符串；只给外层对象加引用、仍把这三列各当一个增长标量，重复展开的问题会保留。候选规范须由字段 schema 明确标出这类“规范 JSON 文本”视图，让内部数组/对象也能共享，并在完整导出时还原原字符串及其转义。不能按字符串“看起来像 JSON”猜测类型。原始来源的 `raw_text` 则保持不透明原文，空白、数字拼写与转义均不得借机重写。

构造入口也须直接从同核共享证据产生这些内容引用。先构造完整旧 Value/JSON 字符串、再转换成内容树，仍支付原有的大量构造和序列化成本，不能算该项完成。整段规范字节按固定块切分也不足以证明增长字段共享：前部字段长度变化会移动后部块边界。公共规范须在结构字段和有序序列层定义身份与片段；私有存储仍可自由选择实际切块。此处是待采纳方案的验收约束，不授权实现。

**批次清单。** 清单绑定完整事实/见证/关系/观察/原始历史/目录轴的可达内容根、真实 Delta 及其 base cut，以及实际会话、发布序号、cut、输入前沿、profile/rule/catalog/schema、scope、clock 和 attempt 上下文。最终字段元组由当前消费者清单逐项核定。所有节点完整落盘后，仍在 S 的一个 Commit 中发布清单、独立索引、Delta、clock 与前沿。独立索引继续独立核出生、撤回和成员归属，不能只信清单自述。

**避免自引用。** 当前批次 ID 不能进入计算自身 ID 的原件。规范须明列“封存前内容”和“发布后由本批次 ID 派生的元数据”边界；旧 `first_known`、batch 与 published 投影按现有实际历史保留。禁止通用字符串替换或忽略所有 `*_id` 来避开循环。

**固定 cut 与大字段读取。** 每个页面和内容片段均绑定同一完整 cut、规范版本与根；单个增长数组也应能按稳定位置分片。客户端验证片段范围、总长度和内容身份，完整遍历可无损还原全部旧逻辑字段及顺序。未下载部分必须如实显示尚未加载，不能当作空集合。Snapshot、Watch、Gap 和来源追溯应使用同一套固定 cut；重复 Delta 幂等。分页不持有跨请求读事务。盘点已确认现有分页只按整对象切页；本次须同时增加内部片段协议与前端组装，不能默称已由现有分页覆盖。

**端到端摘要。** Q 当前对完整 `rows` 投影求 Hash，前端收齐页后再次重算；E/B 当前对 S 返回的完整对象投影求 `object_hash`。新版本必须让这些摘要同样绑定规范的完整有序内容引用及各自元数据，不能只在 S 内共享、到消费边界又强制全部展开。E/B 的 Hash 要覆盖它当前所核的投影字段，包括实际 batch/first_known/withdrawn 等元数据，不能直接用内容字段较少的 `object_id` 代替。对现有 `exact()` 字段集的任何扩展都由显式接口版本承担，旧版原判据继续有效，不悄悄放宽旧解析器。

**当前字节校验。** 公开 Hash 变更不放宽已获批 2D：每次写前仍取得当前一致性读集中的全部可达根、独立索引和唯一依赖。缓存只能在本次原始行与此前成功核验原件逐字节相同后复用；仅摘要相同不是这道缓存门。根/叶损坏、丢失、同 ID 换内容及历史关联变化仍须在任何写入前拒绝。共享表示减少重复展开，不保证每次校验只随新增数据增长。

**旧版本和跨服务。** 旧 ID 与旧库不重写；旧 reader 配方继续受版本明确约束，新 writer 不得在旧 ID 下换含义。未知规范、混合引用及版本不配套明确拒绝。E/B 所消费的结构与资格事实不变；所有 S 引用携带或由不可歧义上下文绑定规范版本。跨版本 crosswalk 必须列出具体字段及一一对应的依据，分别验证两种原生 Hash 和完整逻辑内容；不能声称新旧公开 ID 字节相等。

**部署边界。** 首次实施只进入隔离的新测试/纸面会话及离线迁移副本。上线、主线合入和真实交易各按已有权限门处理。Released、MayHaveCalled、去重与尾责身份不能因结构 ID 升级而重铸；新旧不双写，旧 writer_epoch 被排除，不能理解新责任的旧程序不能恢复发送权。

## 4. 实施前补齐与验收

独立盘点在固定 HEAD `861ee7198f34e03ed7302f9be120a3d28b143339` 完成，真实 Sandcastle run 为 `6d9beede-9981-4064-aa35-e057946f6a9c`，模型 DeepSeek V4.1 Flash/max，exit=0。原始结果见 [消费者盘点 result.json](./1394-public-identity-consumer-inventory-sandcastle-run-r2/result.json)。它只读源码，没有运行服务、测试或重核规模计数。以下路径均相对该固定提交：

| 影响面 | 现行行为与代码依据 | 增补须明确 |
|---|---|---|
| S 批次 | `rust/src/bin/s_structure_session.rs:2739` 组装完整 batch，`:2771` 计算 `batch-` Hash，`:2775` 才给对象附加 batch_id；`:379` 的读取入口核前缀、长度和完整 Hash。`s_session_v2/mod.rs:347` 注入 protocol/session_generation/clock/semantic_commit 绑定 | 封存前原件、发布后派生字段、完整清单元组、版本与旧 reader 分派 |
| S 对象与前端 | CC-006 的身份式在 `s_structure_session.rs:1867`；typed 身份式在 `s_session_v2/tb02a_facts.rs:231`。`s_session/s_tb02_contract.py:297` 和 `s_session/browser/tb01c-client.js:356` 重算，不是只存字符串 | 每个对象家族的新内容身份和旧配方；不得按统一删字段的方式做 crosswalk |
| E/B 的结构来源 | `rust/src/economic_session/finance.rs:278` 严核 s_ref 的 object_id/object_hash/profile_id 与 o_ref；`:557` 核固定 S 二进制，`:573` 核 FinancialBasis 会话/epoch，`:580` 核 pending，`:585` 查对象并重算其完整投影 Hash。E/B 共用该实现 | 新版本 FinancialBasis 与摘要规范；固定二进制变更登记；结构内容和投影 Hash 区分。任何协议新增字段须同步 `exact()` |
| Q 固定切面 | `s_session/s_query.py:21` 固定 token 字段集；`:258` 对含全部 rows 的投影求 Hash，`:268` 装 token，`:283` 核续页。`s_query_integrity.py:1115` 按完整 batch 原件核 index_frontier | 切面根应绑定全部原 token 语义和完整行集合；明确新旧摘要版本与片段范围证据 |
| 分页与前端 | `s_query.py:319` 只切 rows；`:425` 拒绝过大的单条 Delta；`s_readonly_server.py:1033` 拒绝超回复预算。`tb01c-client.js:544` 要求每对象唯一整行，`:553` 核完整投影 | **现状不支持单对象/来源数组内部拆分。** 须增加固定 cut 的片段读取并同步前端，不能靠降低 page_size 解决 |
| 其余来源身份 | receipt 在 `s_structure_session.rs:1621` 由业务身份+payload_hash 派生并在读时重核，Q 在 `s_query_integrity.py:595` 独立重核；witness 在 `s_structure_session.rs:1894` 的输入含 object_id | 原始 receipt 配方保持；对象 ID 改动会传递到 witness，不承诺其字节不变；旧 witness 仍按原件可读 |

固定 cut 至少保留已签 C08.2 的 `session/generation/structure_cut/catalog_revision/scope/index_frontier`，并保持当前 token 的 session_generation、history_mode、profile_hash、rule_revision、页序和捕获绑定。C09.4/C09.11 的完整落盘、原子发布、旧写者排除与离线 crosswalk 继续适用。消费者盘点没有授权新协议；下一步是在本稿内定稿唯一编码、具体版本、完整字段及迁移拒绝行为，再一次提交可审阅的增补。

实施需按一次连贯交付验收，不把同一方案拆成反复补件的批准链：

1. 冻结唯一规范、完整字段元组、ID 派生顺序、片段协议和旧版本拒绝行为，逐接口给出旧新 crosswalk；不改结构判据和资金政策。
2. 先用可完成的同源前缀逐 cut 比较完整逻辑字段、来源 revision、关系、首获知、发布/撤回和 Delta 应用结果。分别验证原生 Hash，保留明列身份映射；真实同价修订、消失再返回、旧 cut 不变、Gap/断线/重复包及会话隔离一起覆盖。
3. 实际运行完整冻结 9011 输入，在同输入、版本、时钟与调度下双跑；记录全部内容节点、引用闭包、逻辑事实数量、实际比较数、构造/Hash/读取/审计/持久化成本、RSS 和库/WAL 增量。下界计数、私有存储小例和一次启动成功均不能替代此项。
4. 使用已有故障定义执行实际进程终止与恢复；校验部分节点落盘未发布、完整原子发布、旧 epoch、同 ID 冲突和暖/冷/超预算缓存中的损坏拒绝。已覆盖且代码路径未变的证据明确复用，不重复凑测试数量。
5. Q 与实际前端完整消费同源结果；E/B 完成引用核验、逐重可观察和财务场景验证。历史回放和 paper 共核证据分别成立后，才按总图 Destination 验收。

本稿没有完成时间承诺。消费者改动面、剩余唯一依赖的重复读取成本与完整运行的资源读数仍未测定。当前可以确定的是：增加工蜂并行度不能消除现行公共表示要求的重复展开，选择 C 会改变公开接口，应一次明确决定。

## 5. 候选 C 的接口增补正文（2026-09-16，未批准）

本节给出一份具体候选，供一次审阅；前四节的“随后定稿”在本节内落实。**状态是待审草稿，不是已签 SPEC，不授权实施、迁移旧账、合入或上线。** 本节若被采纳，须与 #1340 的有效合同一起登记；既有结构判据、原始来源、资金政策和六个 Destination 不变。新增版本号、编码、分片与资源计费方式均属于本次待裁范围，不能只采纳名字而继续由实现者补隐含规则。

当前代码证据固定于 bea37f50b23efc0c0d4a58ecc4da7103afcadbd7。两项新增事实影响实施边界：

- Q 的页面、游标和前端安装过程目前要求收齐完整正文；Q 逐页先重建完整 projection 再切 rows。片段协议必须同时改生产者、核验器和前端状态表示。
- E/B 的 FinancialBasis 当前固定为七字段，消费者没有对该正文做 exact/schema_revision 校验；finance.structure、s_ref、o_ref 则均为 exact。完整 FinancialBasis 还被复制进经济历史。当前 S→E/B reader 限整份 stdout 为 16 MiB、进程期限 10 秒；64 条实测财务结构依据已为 21,150,983 字节。新引用不能只加进旧字段集，也不能原地替换已提交历史正文。

### 5.1 版本、数字和规范字节

| 面 | 旧版继续只按原合同读取 | 本候选新增 |
|---|---|---|
| S/Q 消息 envelope | s-session/2；原 11 字段 | s-session/3；同 11 字段，payload 按本节类型分派 |
| 内容图 | 私有 evidence-graph/2、/3 不等于公开身份 | s-content/1 |
| 批次清单 | batch- + SHA256(完整旧 batch bytes) | s-batch-manifest/1，batch3- 前缀 |
| Q 页面 token | s-snapshot-token/1 | s-snapshot-token/2；新摘要配方 |
| 财务来源正文 | 无自身版本的 FinancialBasis | s-financial-basis/3，kind=FinancialBasisRef |
| 财务输入及 manifest 子合同 | tb03b-finance/1 | tb03b-finance/2 |
| E/B envelope 与持久域 | economic-session/1 | economic-session/2；只用于全新测试会话，旧域不混写 |

文本必须是有效 UTF-8，无孤立代理项，不做 Unicode 归一化。对象字段按键的 UTF-8 字节升序；重复键拒绝。公开控制数值继续用规范十进制字符串，业务整数保留现有 i64 域与各字段正负约束；frontier 的 -1、缺字段和 null 不合并。数组顺序保持。

内容图也能表示旧封存原件中的 JSON number：保存其**精确合法 JSON 数字 token**，不把 1、1.0、1e0 合并。该类型用于保持原件字节和数值表示；具体业务字段仍须通过原域校验。不能用通用 JSON 解析器把数字先变成浮点再猜回原件。

规范 JSON 渲染规则：无空白；字符串使用双引号，双引号和反斜线分别转义；U+0008/0009/000A/000C/000D 分别使用短转义；其余 U+0000..001F 使用小写四位十六进制转义；其他 Unicode 字符直接 UTF-8 编码，斜线不转义。数字使用上述 token，bool/null 使用小写 JSON 字面量。对旧数据的引入须先核其原有规范字节，不将非规范旧原件“修正”后认作同一证据。

### 5.2 唯一内容编码

以下是本候选的完整 Hash 分帧。U64 表示八字节无符号大端长度，允许值为 0..2^63-1；协议上传输该长度时仍用十进制字符串。所有子摘要均为原始 32 字节，不把十六进制文本拼进 Hash。

~~~text
D = UTF8("NewChanlun/SContent/1") || 0x00
F(tag, body) = SHA256(D || one_byte(tag) || U64(len(body)) || body)

M([])       = F(0x11, empty)
M([h])      = F(0x12, h)
M(h[0:n])   = F(0x13, U64(n) || M(h[0:k]) || M(h[k:n]))  (n > 1)
k           = 严格小于 n 的最大 2 的幂

B(bytes)    = M([F(0x10, chunk_0), ..., F(0x10, chunk_m)])
~~~

B 的块从字节偏移 0 开始，每块恰好 4096 字节，只有末块可以较短；非空输入不允许空块，空输入使用 M([])。UTF-8 可以在块间分开，接收者用增量解码器检查完整字符串。这里只固定公开逻辑分解，私有压缩、文件切块、页布局和树实现可以不同。

序列的分解形状采用 [RFC 9162 §2.1.1](https://www.rfc-editor.org/rfc/rfc9162.html#section-2.1.1) 的按元素数量唯一确定的二分规则；本候选有自己的类型标签、长度与域分隔，**不是 CT 协议或其 Hash 字节配方**。

| tag | 值／内部节点 | body |
|---|---|---|
| 0x00 | null | empty |
| 0x01、0x02 | false、true | empty |
| 0x03 | UTF-8 字符串 | U64(值的 UTF-8 字节数) + B(值的 UTF-8 字节) |
| 0x04 | 精确 JSON 数字 token | U64(token 字节数) + B(token ASCII 字节) |
| 0x05 | 数组 | U64(元素数) + M(依原顺序排列的元素值摘要) |
| 0x06 | 对象 | U64(字段数) + M(依键 UTF-8 顺序排列的 0x14 节点摘要) |
| 0x07 | 规范 JSON 文本视图 | 内部 JSON 值的 32 字节摘要；渲染为该值规范 JSON 文本所对应的字符串 |
| 0x08 | 当前批次占位 | empty；仅允许在 §5.4 明列的发布模板位置 |
| 0x10 | 字节块 | 块的原字节，非空且不超过 4096 字节 |
| 0x11、0x12、0x13 | 空序列、单叶、序列分支 | 如上 M |
| 0x14 | 对象字段 | 字符串键的值摘要 + 字段值摘要 |

称以上值的摘要为 V(value)。数组元素只允许值节点；对象只允许字符串键与唯一有序字段；内部字节块和序列节点不得冒充逻辑值。每个分支的子树数量必须符合 k 的规则；未知标签、错误长度、错误子类型、缺节点及非规范形状全部拒绝。节点 Hash、类型和长度都校验，不能仅核最终根。

**ContentRef 的 exact 字段**为 format,root,kind,extent。format=s-content/1；root="sc1-"+64 位小写十六进制 V；kind 取 null/bool/string/number/array/object/json_text/template_batch。string/number 的 extent 是字节数，array/object 是元素／字段数，其余为 null。extent 必须等于对应节点的已核 header。JSON 文本视图不要求先算出巨大渲染字符串的长度。

transport 中的 ContentRef 是带 schema 的边；构造 V 时接入它所指的**值摘要**，不对“描述引用的四字段对象”再求值摘要。只有本节声明为 ContentRef 的位置允许此解释；源数据里长得相同的普通 JSON 对象仍是普通对象。

0x07 仅用于原存储的 payload_json、input_refs_json、source_coords_json 三类明确列。引入时验证内部 JSON 和规范原文逐字相等；公开 wire 的 payload/input_refs/source_coords 接入其内部值根。raw_text 一律作为不透明字符串保存。完整导出递归渲染并保留原字符串内容与转义；读取事实不重新运行业务判定。

0x07 内部是普通 JSON 值树：值标签只允许 0x00..0x06，其任意后代不允许再出现 0x07 或 0x08。内部本来含有的 JSON 文本字符串仍编码为普通 0x03 字符串，不再次解析为文本视图。对象字段／序列／字节块仍按上表使用相应内部标签。

### 5.3 身份配方与全文绑定

| 身份 | 唯一配方 |
|---|---|
| typed object_id | "obj3-"+hex(F(0x20, V({kind,fact_key,payload,input_refs,profile_id,rule_revision}))) |
| CC-006 object_id | "obj3-"+hex(F(0x20, V({branch,dir_ab,dir_bc,window_start,window_mid,window_end,merged_highs,merged_lows,comparisons,input_refs,rule_revision,profile_id}))) |
| batch_id | "batch3-"+hex(F(0x21, V(§5.4 manifest))) |
| cut_projection_digest | "qcut3-"+hex(F(0x22, V(§5.5 的完整七字段逻辑 projection))) |
| E/B object_hash | hex(F(0x23, V(当前完整 wire 对象)))；包括全部元数据、payload、来源和生命周期 |

V 的输入字段取现有相应配方的类型和值；CC-006 的三组 merged_highs/merged_lows 继续从同 cut 组事实核定。两个 object_id 家族不改成通用删字段规则。内容 ID 仍不包含其未声明的发布元数据；E/B 的 object_hash 则完整包含，二者不得互换。

witness 仍按当前具名配方由实际新 object_id 和原来源派生，只将其版本明确为新 S 域；原 witness 不重命名。receipt/input payload 的原始身份配方不借本次内容图优化更改。HTTP envelope 的 payload_hash、token_checksum 仍对其有界的实际传输 JSON 求原 SHA256，属于后置传输绑定，不反向进入 batch 身份。

生产者必须从同核共享证据直接建立上述值边。先生成完整旧 Value/JSON 字符串再拆图不满足本候选的增长构造要求。全部 24 个比较、所有来源、所有版本及目录证据仍存在于可完整遍历的图中。

### 5.4 S 发布清单与无循环派生

manifest 的 exact 字段为：

~~~text
schema_revision, content_format, protocol_revision,
session_id, session_generation, writer_epoch,
generation, base_cut, structure_cut, input_frontier,
source_commit, clock_plan_hash, semantic_commit_ns,
profile_id, profile_hash, rule_revision, catalog_revision, scope,
generated_state, publication_template, delta_template
~~~

schema_revision=s-batch-manifest/1，content_format=s-content/1，protocol_revision=s-session/3。source_commit 恰为 source_namespace,source_epoch,message_id,payload_hash,producer_id,producer_epoch,clock_event_id,attempt_index；attempt_index 是原 clock plan attempts 的零基位置。其它标量取实际 Begin/Commit 与冻结输入，不增加时钟或资金默认值。

后三项是 ContentRef：

- generated_state 绑定原 sealed batch 的全部生成态逻辑字段：generation,structure_cut,input_frontier,catalog_revision,profile_id,profile_hash,rule_revision,raw_events,effective_events,objects,witnesses,relations,observations,scope，以及 OHLC 的 catalog_axes；原 v2 的 protocol/session-generation/clock/commit 绑定由 manifest 对应字段承接。生成态元数据与独立索引的首获知元数据继续分开。
- publication_template 绑定 §5.7 二十字段逻辑 snapshot，含完整 active/withdrawn 记录、见证、关系、观察、raw_history、目录证据与实际首获知／撤回元数据。
- delta_template 绑定 §5.5 D 字段加七个完整变化集合；只发真实变化，base Snapshot 加 Delta 仍须逐字段等于 next Snapshot。

0x08 只准出现在 publication_template／delta_template 的以下位置：本批新生对象的 batch_id、本批首次观察的 batch_id、catalog_evidence.batch_id，以及当前发布的 index_frontier；其嵌套副本按同一字段 schema 处理。旧对象和旧观察保留原首版 batch_id。既有独立出生表、首次观察表、代号和当前批次上下文必须同时证明“本批”，否则占位非法。payload、raw_text、来源原文及任意名称相似字段不允许占位。

先求 manifest 的 batch_id，再仅在上述位置把 0x08 解析为这个 batch_id 的字符串值，得到唯一的实际 snapshot、Delta 和 Q 投影值根。解析只更换已定位的标量及祖先节点，不展开共享大字段。manifest 不包含后置 Q 投影根、token、HTTP 回复或自身实际 ID；因此不存在 Hash 自引用。0x08 不得从最终公开逻辑值泄漏出去。

~~~mermaid
flowchart LR
  A[原始来源与同核完整事实] --> B[对象内容 ID 与生成态根]
  B --> C[发布模板及 Delta 模板]
  C --> D[manifest / batch3 ID]
  D --> E[解析本批占位后的实际切面]
  E --> F[Q 完整投影根 / E-B 对象摘要]
  F --> G[token 与实际回复摘要]
~~~

节点完整落盘后，原 Commit 原子发布 manifest、独立索引、Delta、clock 和前沿。独立索引仍由各自的真实记录核出生、撤回、成员和来源，不能由 manifest 自述替代。所有写入前仍在当前一致性读集中取得全部可达根、独立索引与唯一依赖原件；同 ID 换内容、缺节点、历史损坏、冲突写入及旧 epoch 的拒绝义务不变。

### 5.5 Q v3：逻辑字段不遗漏，传输大值改为引用

共同 envelope 保留原 11 字段与因果回引规则。入口为 POST /api/v3/snapshot、/api/v3/watch、/api/v3/content；v2 路由不接受这些新值或新 token。

**逻辑 fixed_cut exact 16 字段**：
session_id,session_generation,structure_cut,cut_generation,catalog_revision,index_frontier,profile_id,profile_hash,input_frontier,seq_range,catalog_run_status,catalog_evidence,scope,rule_revision,history_mode,as_of_generation。
history_mode、scope、seq_range 及零代规则沿现行合同。其 wire 中仅 catalog_evidence 改为 ContentRef，其余标量与小对象保持；解析后仍是完整逻辑 fixed_cut。

**逻辑 projection exact 七字段**：fixed_cut,catalog,rows,counts,scope,omissions,order_version。rows 的顺序、六族、key、重复排除和生命周期均不变；row exact 为 family,key,record。wire 中 catalog 和 record 为 ContentRef。counts 仍为六族加 total，omissions 必须为 []。qcut3 摘要绑定解析后的完整逻辑 projection，包括全部 rows，而不是当前页的引用列表。

**Snapshot 请求和 page**保留原 exact 字段集，按 v3 的引用类型解释：

| 项 | exact 字段 |
|---|---|
| 首页请求 | op,session_id,session_generation,scope,history_mode,as_of_generation,page_size,order_version |
| 续页请求 | op,snapshot_token |
| page | fixed_cut,catalog,rows,counts,scope,omissions,order_version,offset,done,next_token,cut_projection_digest,validated_capture_digest,ok |
| token | token_schema,session_id,session_generation,cut_generation,scope,history_mode,structure_cut,catalog_revision,index_frontier,profile_hash,rule_revision,order_version,page_size,offset,cut_projection_digest,issued_capture_digest,token_checksum |
| cursor | session_id,session_generation,catalog_revision,scope,after_generation,base_cut,index_frontier,last_seq,order_version |

token_schema=s-snapshot-token/2。token_checksum 绑定除自身之外全部 token 字段；它是校验和，不是访问权限凭据。服务端仍从本次已验证资料重建指定旧 cut 的实际根，逐项比较身份、顺序、页长和摘要，不能仅凭 checksum 放行。issued_capture_digest 在续页中保留，validated_capture_digest 来自本次实际核验；它们均不进入逻辑 projection。新提交和 Q 重启本身不取消合法旧 cut token。

首次取得内容读取 anchor 的步骤固定为：先收齐有界的所有 row 引用页，核 counts、公开键无重复、页 offset、固定头及 token 连续性，再把 catalog、record、catalog_evidence 的已声明引用作为值边计算完整 projection_ref，并重算 qcut3。此时只确立待读取的完整根，尚未通过正文语义检查，也不安装新 cut。随后按 §5.6 取得正文，才能核 row.key 与实际记录的派生键相同、原完整排序元组及其余全部语义；例如 kind/fact_key/object_revision 在 record 引用内，不能提前宣称已核排序。page 不额外增加 projection_ref 字段。

Watch 请求仍为 op,cursor,max_batches,client_id；响应仍为 deltas,next_cursor,observed_head,delivery_frontier,has_more,gap,validated_capture_digest,ok。定义 D 为 generation,session_id,catalog_revision,base_cut,next_cut,seq_range,index_frontier,input_frontier,catalog_run_status,catalog_evidence。每个 Delta 外层仍为 D+delta，wire 的 catalog_evidence、delta 改为 ContentRef；其完整内层为 D+upserts,withdrawals,replaces,witnesses,relations,observations,raw_history_added。

Gap 的五字段、三种 reason、原 cursor 不推进、missing_range 和 rebuild 精确目标沿旧合同；snapshot_token 使用新版。租约 delivered 仍只在完整响应成功发送且非 Gap 后记录。声明一个 Delta 引用不等于客户端已完成该批应用；游标只在完整变化和同代结果对拍成功后推进。

**排序不改变**：六族顺序为 objects,withdrawn_objects,witnesses,relations,observations,raw_history；数字用整数比较，文本用 UTF-8 比较。沿 s-record-order/1、/2 的原元组。新的内容 ID 参与对应版本的原排序，不用旧 ID 排完后悄悄替换。旧、新版本之间的序列对拍必须用明列 crosswalk，不声称 ID 字节相等。

### 5.6 片段、证明和有界完整读取

content 首次请求 exact 为 op,anchor,path,unit,offset,limit；op=content。anchor 是按 kind 分派、不得混合字段的两种 exact 类型：

| kind | exact 字段与根 |
|---|---|
| projection | kind,fixed_cut,cut_projection_digest,projection_ref；根是完整七字段逻辑 projection 的 ContentRef |
| delta | kind,session_id,session_generation,generation,base_cut,next_cut,index_frontier,delta_ref；根是完整 D 加七集合的实际 Delta ContentRef |

projection anchor 必须与本次已验证资料导出的指定 cut 相等。delta anchor 必须在本次已核发布历史中定位唯一同会话／同代的发布记录：index_frontier 等于该实际 batch3；重算 §5.4 manifest 身份，核其 base_cut/structure_cut，并按允许的出生位置解析 delta_template 的本批占位，所得根须逐字等于 delta_ref。生成态、模板、独立索引、发布记录之间的全套核验继续执行。anchor 不携带由客户端任选的 manifest 替代已封存原件；batch3 即 manifest 的类型化身份。该关系不要求 Delta 在 projection 内可达。

path 分别从该类型指定的根出发。不能通过裸 Hash 读取其他会话、未发布内容或不属于该 anchor 的节点。Delta 超出当前交付保留窗口时，返回 Incomplete/ContentAnchorUnavailable，由客户端按原 Watch Gap 流程重建；不得悄悄换成当前 Delta。projection 的合法旧 cut token 仍按原历史读取规则处理。

path 是有序步骤数组；每步只取以下一种 exact 形状：

- {field: UTF-8字段名}：对象字段；
- {index: 规范非负整数文本}：数组元素；
- {json_text: true}：进入 §5.2 的明确 JSON 文本视图。

unit、items 与计数的唯一形状如下。所有 offset/limit/end/total 都是规范非负十进制字符串，limit 必须正；先查溢出，再令 end=min(offset+limit,total)，且 0<=offset<=total。done 当且仅当 end=total；到达末尾的空范围有唯一空 items 表示，不伪造元素。

| 目标／unit | total | items exact 形状 |
|---|---|---|
| 数组／items | 数组 extent | 按位置排序的 ContentRef 数组；长度 end-offset |
| 对象／items | 对象 extent | 按键 UTF-8 排序的数组，每项 exact 为 key,value_ref；key 是字符串，value_ref 是四字段 ContentRef |
| 字符串或数字／bytes | 字节 extent | 恰好 end-offset 字节的小写十六进制字符串；空范围为 "" |
| null、bool、json_text／value | 固定 "1"（不等于 ContentRef 的 null extent） | offset=0,limit=1 时为 [target_ref]；offset=1,limit=1 时为 [] |

template_batch 不属于公开可读值。json_text 的 value 只取得该值节点，读取内部值仍显式走 json_text 路径；它不豁免完整遍历。

响应 exact 为 anchor,path,target_ref,unit,offset,end,total,items,nodes,done,next_token,validated_capture_digest,ok；anchor 必须逐字回显请求的已核类型化 anchor。nodes 按 root 去重、升序排列，每项 exact 为 root,tag,body_hex；证明必须从 anchor 的根覆盖到目标及所给范围的路径、类型和位置。数组元素值与对象项至少包含验证其 ContentRef header 所需的节点。tag 为两位小写十六进制；body_hex 拒绝奇数长度、非小写十六进制或额外字节。

每个片段的证明独立完整：允许祖先节点在后续响应重传，全部实际传输量照实累计。接收方可按根复用已验证节点；发生相同根不同字节即拒绝。不得省略本响应必需的证明节点后假设客户端曾缓存；大内容的子树另按路径读取。末页 next_token=null，否则 token.offset=end，其他 anchor/path/target/unit/limit 保持。客户端只接受下一 offset 等于上片 end；首次直接请求非零范围只能作为独立区间记录，不能被当成已完成前缀。

片段 token exact 为 token_schema,anchor,path,target_ref,unit,offset,limit,issued_capture_digest,token_checksum；token_schema=s-content-token/1。续片请求只允许 op,content_token。服务端核实际根、路径和已到达 offset；客户端核 token、所有节点 Hash、规范分解、目标范围、内容值及无跳漏／重叠，随后才记录已完成区间。换会话、换 cut、替换目标或重新签名伪造范围都不能混入既有暂存。

**证明的边界：** 范围证明只证明该片属于完整根，不自动证明未下载部分的类型、业务值域或来源关联正确。现有前端的字段、目录、来源、对象 ID、生命周期和 Delta 对拍校验必须改为完整图遍历／可复用的已验证子图检查。复用须绑定原件、schema、判据的全部上下文和相关根；不能用一个根 Hash 代替原来应执行的检查。

前端保留 committed 与 staged 两态。staged 中的未取得范围明确显示“尚未加载”，不得当空数组；校验未完成不替换 committed cut/cursor。全部必需验证完成后原子安装的是完整内容图视图和索引，**不要求把所有嵌套 JSON 展成一份内存对象**。渲染、导出和来源追溯通过同一根遍历器访问全部字段；完整导出可以流式写出。迟到页、坏片、超限和无效的新请求沿旧合同保留原 committed 状态。

资源合同统一见 §5.6.1。完整读取跨分页、片段、证明重传、同代对拍及重试累计，不能用多次调用重置额度。首批数值由冻结内容规模与实测结果形成具名测试输入；缺少它只允许验证规范，不能运行新服务或声称 9011 验收通过。

Q 单次一致捕获、写前当前原件审计及分页不得持有跨请求读事务的约束不变。实现应直接处理唯一节点、紧凑索引和图视图；若仍不能满足已声明 capture/verify 资源门，必须报告真实 Incomplete。本节没有预先证明这一成本已经消失，也不以新的客户端 job limits 掩盖 Q64 的现有失败。

#### 5.6.1 资源字段、冻结位置和旧限制的归属

job_limits 的 exact 八字段如下；均为显式正整数，解析域为 1..2^63-1，不提供默认、无限量或按片重置。内存对象可用整数，公开 JSON 配置使用规范十进制字符串。

| 字段 | 单位与计费边界 |
|---|---|
| max_total_wire_bytes | 从 job 首次请求开始的应用层实际读写字节总数；包含请求、全部响应及错误、CLI stdout/stderr、对端 proof 与节点导入流；不计 TLS/HTTP 传输头。重试和证明祖先重传全额重计 |
| max_total_node_bytes | 本 job 接纳的不同根节点分帧字节之和：len(D)+1+8+len(body)；相同根只记一次，包括来自本地复用的节点；不按展开值重复记 |
| max_total_nodes | 上述不同根的数量，包含值、块、序列和字段节点；每个节点仍须执行必要的上下文检查 |
| max_total_records | 逻辑六族 Snapshot rows 和七个 Delta 变化集合的顶层记录访问次数；每次完整校验、嵌套 basis 及同代对拍各自累计，内容节点去重不免除记录计数；失败后重新访问同样计数 |
| max_total_requests | 本 job 发起的 HTTP、reader 子进程或对端本地证据捕获次数，包括错误与重试；无请求但读取本地缓存仍受其余预算 |
| max_elapsed_ms | 首次读取开始至安装／经济提交前全部读取验证结束的单调时钟绝对期限，排队、重试和来源变化重读均消耗；不能逐片续期 |
| max_staged_bytes | 本 job 暂存文件、索引和日志的实际分配字节峰值；写前预留最坏分配量，写后核实，不足即停止；取消或失败释放无历史引用的暂存 |
| max_store_bytes | 本地内容存储整体分配字节上限，含既存已提交内容、暂存、索引及日志；不能通过删去仍被 committed cut 或经济历史引用的节点腾挪额度 |

浏览器 v3 的配置 exact 为 schema_revision,pageSize,watchBatches,pollMs,timeoutMs,replyBytes,job_limits，schema_revision=s-query-client-resources/3。旧 totalBytes/maxRows/maxPages 由八字段累计合同整体替代，其余具名控制保留原作用。一个 job 从用户开始重建或一次 Watch 调用开始，涵盖其全部同代 Snapshot 对拍；取消后新 job 可重新发起，但旧未完整结果不得接力安装。运行证据记录完整配置及其规范 SHA，客户端在 job 中冻结；此配置不是服务端信任凭据，也不进入 S 内容身份。

Q v3 服务端资源文件 exact 为 schema_revision,limits，schema_revision=s-query-resources/3。limits 为原十九字段集合去掉 max_atomic_batch_bytes，再新增 max_atomic_delta_node_bytes,max_atomic_delta_nodes,max_atomic_delta_records；其他十八项保留原含义与原数值。三个新增正整数分别限制**单个完整实际 Delta 可达闭包**的唯一节点分帧总字节、唯一节点数及七变化集合的记录总数，证明节点不替代闭包计量；均须在返回其引用前完成计数。值计量采用上表规则。拟交付的任一 Delta 超出这三项即整个请求 Incomplete、不推进游标；只有实际回复装不下后续已通过的整批时，才沿旧回复限制返回前批。首批回复也装不下则失败。

这明确替代 v2 对完整展开 Delta canonical bytes 的 max_atomic_batch_bytes，不能声称两者是相同资源判据；v2 保留原门。v3 的 max_reply_bytes/max_frame_bytes 继续计量实际完整传输消息，队列、worker、捕获、校验、缓存和租约等限制不变。Q 运行配置在启动时冻结并记录规范 SHA，不按请求放大。上述替代及新增数值都属于候选 C 的待采纳资源合同。

E/B 新 finance.structure 增加 reader_resources，其 exact 为 schema_revision,max_request_bytes,max_reply_bytes,max_stderr_bytes,request_timeout_ms,job_limits；schema_revision=s-financial-reader-resources/1。前三项限制每次 reader 调用的真实字节，request_timeout_ms 限从启动至回收的单次绝对期限；job_limits 八项按上表累计，包括 peer proof 内全部历史引用闭包的捕获和验证。一项不足即 Incomplete/MissingDependency，不产生经济提交。该完整对象位于冻结 manifest 内，全部参加 manifest_hash；既有 E↔B resources.max_frame_bytes/response_timeout_ms 仍各自适用，取同时生效的最早期限。job 始于本次经济操作的首次依赖读取，Reserve 的全部申请共用同一 job；发生来源变化而重读仍用同一余额和绝对期限。

旧 financial-basis 的整份 16 MiB/10 秒合同保持；新 reader 不借其旧名称宣称等效。独立有界的物理导入与完整逻辑恢复各自受上述限制。任何共享缓存、既存本地原件或当前一致捕获的复用，仍不能省去原要求的完整性与时钟校验。

### 5.7 S→E/B：完整来源承诺、点查与本地证据所有权

新的 finance.schema_revision=tb03b-finance/2。finance 仍为 schema_revision,input_path,input_sha256,coordinator_source,structure；structure 的 exact 字段新增 reader_revision,content_format,reader_resources，即：
binary,binary_sha256,db,session_id,session_generation,writer_epoch,reader_revision,content_format,reader_resources。
reader_revision=s-financial-basis/3，content_format=s-content/1；reader_resources 精确采用 §5.6.1。整个 manifest 仍冻结并参加原 manifest_hash；不能把修改 binary_sha256 当作旧经济 DB 的热替换。

S 新增独立 CLI financial-basis-ref，读取一个严格 JSON 请求；请求 exact 为 schema_revision,session_id,session_generation,writer_epoch,object_ids，schema_revision=s-financial-query/1。每次 Prepare、Reserve、Commit 都由对应 owner 新发请求；不接受调用者提供的旧 basis 冒充本次当前读。原 binary SHA、session/generation/epoch、pending 和时钟门全部保留。

响应 exact 为 schema_revision,content_format,kind,session_id,session_generation,writer_epoch,pending_revision,commit_ns,basis_ref,snapshot_ref,selected_objects,proof_nodes。
schema_revision=s-financial-basis/3，kind=FinancialBasisRef。basis_ref 绑定原七字段逻辑 FinancialBasis；其中逻辑 kind 仍是 FinancialBasis，snapshot 是完整值。snapshot_ref 必须是其 snapshot 子值。

逻辑 snapshot exact 二十字段：
session_id,generation,structure_cut,catalog_revision,scope,index_frontier,profile_id,profile_hash,history_mode,as_of_generation,objects,withdrawn_objects,witnesses,relations,observations,raw_history,input_frontier,seq_range,catalog_run_status,catalog_evidence。
模式仍为 RecomputedWithRevision，as_of_generation=null；不得用 AsKnown 片段代替本次财务当前观察。

selected_objects 每项 exact 为 object_id,object_hash,object_ref,snapshot_index。proof_nodes 使用 §5.6 的节点格式，证明 header 标量与 basis_ref 绑定、snapshot_ref 是其子值、所选完整对象在 snapshot.objects 的给定位置且 object_id 相符。object_hash 按 §5.3 全 wire 对象求值，不能用 object_id 代替；profile_id 仍与 snapshot 相等。重复 object_ids 请求拒绝。

大依赖由 financial-content 独立入口分片读取。请求 exact 为 schema_revision,anchor,path,unit,offset,limit；schema_revision=s-financial-content/1。anchor exact 为 session_id,session_generation,writer_epoch,structure_cut,index_frontier,commit_ns,basis_ref；path 从 basis_ref 开始，范围和证明格式沿 §5.6。每片须在新的短读事务确认该当前控制元组仍一致且 pending=false；变化则 MissingDependency，整次尚未提交的来源读取重新取得当前 basis，不把两次观察拼接。这个多请求一致性行为属于新版本明文合同。

financial-content 响应 exact 为 anchor,path,target_ref,unit,offset,end,total,items,nodes,done,ok；分页推进由明确 end 决定，无隐藏游标。items 的四类形状、边界和独立完整证明按 §5.6；此入口的 anchor 使用本节财务类型，不接受 Q 两种 anchor。整次读取受冻结 reader_resources 及已有经济操作绝对期限共同约束，片段不能重置该期限。旧 financial-basis 的整份 16 MiB/10秒合同不改。

**s_ref/2 exact** 为 schema_revision,object_id,object_hash,profile_id，其中 schema_revision=s-object-ref/2。o_ref 仍 exact 为 operation_id,kind,source_ref,s_object_id,quantity，现有 TestOnlyCashBuy、对象身份与 plan.quantity 等式不变。新财务输入的原始 input_sha256 重新冻结；不能在旧输入 Hash 下替换字段。

持久 FinancialBasisEvidence 的唯一 exact 形状为：

~~~text
schema_revision,content_format,kind,session_id,session_generation,
writer_epoch,pending_revision,commit_ns,basis_ref
~~~

schema_revision=s-financial-evidence/1，content_format=s-content/1，kind=FinancialBasisEvidence。basis_ref 为 ContentRef；其指向的原七字段逻辑 FinancialBasis 中 kind 必须等于 FinancialBasis，其余 session_id/session_generation/writer_epoch/pending_revision/commit_ns 必须逐项等于持久头。同旧消费者域，pending_revision=false，commit_ns 为规范非负整数文本，writer_epoch 为正整数文本；null commit_ns 不得进入经济准入。knowledge_floor 使用经这个绑定校验后的持久 commit_ns。snapshot 从 basis_ref 的同名子值取得；运输用 snapshot_ref、selected_objects、proof_nodes **不存入经济 image**，不是另一种可选持久表达。

引用替换只发生在以下 schema 路径，不按字段名称全局递归猜测；所有未列字段按本版原经济合同保存：

| image 内路径 | 唯一表达 |
|---|---|
| history[].witness.structure（对应 PrepareDecision／Reserve／CommitDecision） | FinancialBasisEvidence |
| finance.decisions[].structure_evidence | Prepare 时同一 FinancialBasisEvidence |
| history[].record.structure_evidence（Prepared 决定记录及其后复制） | 同一 FinancialBasisEvidence，不重读或重写首版 |
| history[].witness.prepared_proofs[].image / current_image | 原经济 proof 所捕获的完整 economic-session/2 image；递归遵守本表，保持对端已封存原字节与原 root |
| history[].witness.grant_proof.image / current_image | 同上；不得在已哈希的对端 proof 内做新的正文→引用转换 |
| recorded[].e_commit_proof.image | CaptureExternal／ApplyAllocation 所读取的对应 E 提交的完整 economic-session/2 image；递归遵守本表，保持 E 原封存字节和 root |

**所有这些路径引用的完整证据闭包，均须先由当前经济 owner 验证并导入自己拥有的不可变内容存储**，再在原子经济提交中公布引用。覆盖本次 S basis、prepared_proofs/grant_proof 的 image/current_image、recorded[].e_commit_proof.image 及其中更早历史的全部引用；仅导入本次 S 来源不足。对端 proof 继续沿现有只读 DB 入口在一致捕获中核对其提交、当前 image、epoch 和依赖闭包；不增加网络读取权限。引用节点无外部地址，导入后的同根内容由本 owner 负责保有。ApplyAllocation 捕获也从首次依赖读取开始同一个预算 job，完整闭包导入、计费和离线恢复义务与财务三阶段相同。

导入可以物理去重，但不能只留下未取得的根。发布事务核所需节点全部持久且可达；失败、取消或半导入不产生 Prepared、grant 或责任。历史恢复、原 receipt 幂等返回和本地审计不依赖 S 或原 peer 仍在线。节点缺失／冲突在任何新经济提交前拒绝，仍有经济历史引用的节点不能清理。实际节点传输与 proof 原件读量按同一 reader_resources 累计。

economic-session/2 的经济 image 仍保留完整逻辑状态和历史；上述明确位置的实际持久表达参加原 canonical JSON Hash，再按原 image_hash→经济 root→receipt 配方生成新域身份。transport proof_nodes 不影响 image_hash；FinancialBasisEvidence 的每个头字段和 basis_ref 均影响。不能声称新字节等于 economic-session/1 的旧 image/root。新的 plan_hash 影响 grant_id，必须在新域按原派生公式计算并实际校验；不得映射成一条原 grant 来接管旧责任。

### 5.8 旧新 crosswalk 与迁移拒绝边界

| 旧项 | 新项与逐项证明 |
|---|---|
| raw_text、市场 bar、原始 revision、接纳顺序 | 原文与类型保持；每条源及其全部引用叶现场逐字段相等，不用重命名掩盖差异 |
| 原 obj-/batch- | obj3-/batch3-；分别核各自原生 Hash，按原生槽位、具体来源版本和完整内容建立映射；不主张 ID 字节相等 |
| payload_json/input_refs_json/source_coords_json | 明确 JSON 文本视图；流式重建原规范文本逐字相等；不解释 raw_text |
| 固定 cut、生命周期、目录与 Delta | 保留全部量词、顺序和 first_known/published/withdrawn 等实际历史；映射明确列出的新内容 ID 后逐字段比较，不能删除额外不同项 |
| Q 整体平面 SHA | 完整七字段 projection 的 qcut3 承诺；页面／片段只改变传输形状，完整校验与原子安装义务保留 |
| E/B 旧完整 wire object_hash | s_ref/2 的完整对象内容图承诺；包含全部 batch、首获知、撤回及正文元数据，不降成内容 object_id |
| 旧 manifest、plan、grant、receipt、经济 root | **不重写、不重铸。** 旧 reader 和原 native Hash 继续解释旧历史；新试验域产生自身版本的新身份，不能据 crosswalk 取得旧权利、退款或释放旧责任 |
| 完整 witness 的本地可恢复性 | 新版先取得并验证完整依赖闭包再提交根；原 S 不在线仍能按本地原件还原和核验 |

首次实施只开隔离的新测试会话，完整输入、版本、clock、资源和 peer 绑定作为具名输入冻结。旧库不原地改格式，旧金融 manifest 不自动换 binary；本节不授权任何带未结责任会话的热迁移。将来若需迁移此类会话，必须另具具体证明和既有权限门，不能借这份只读 crosswalk 执行。

未知版本、混合域、旧 validator 收到新字段、新根缺完整依赖以及 old/new peer 交叉连接均明确拒绝。已提交的相同请求仍优先返回原 receipt；S 不可读不推翻已提交经济事实。Abort、Close、Commit 的原责任释放／执行资格语义和 #1349 政策等待保持。

### 5.9 本增补的统一验收

本节只产生一个待审合同包，不能把编码、Q、E/B 各自绿灯当作完成：

1. 编码向量必须覆盖空值、字符串／数字差别、UTF-8 与转义、不同数组长度、非二幂尾部、字节块边界、对象键序、嵌套 JSON 文本、当前批次占位及非法类型／长度／形状。至少两个独立实现逐字计算相同节点与根；这只验证规范，不代表服务落地。
2. 旧新同源前缀通过完整内容、来源、顺序、目录、生命周期、真实修订与 Delta 结果 crosswalk；S/Q/E/B 各自核原生身份。全 9011 条必须实际运行并双跑确定；不以逻辑体积统计代替。
3. 使用原故障定义检验未发布节点、完整闭包后发布、Commit 前后、重启／旧 epoch、重投幂等和历史 cut。节点、根、槽位、独立索引、clock、manifest 与经济本地证据闭包的损坏须在相应新写入前拒绝。
4. Q/前端真实执行多页、多对象内片段、缺页／重复／乱序／错根／跨会话／迟到／Gap／重连。完整验证前不安装，不把未加载内容当空；旧 cut 和 source revision 不能漂移。
5. E/B 实际完成 Prepare→E Reserve→B Commit、三阶段及六故障，保持责任先增、公共责任与自有权利、所有知识时钟和原身份返回规则。来源变化或读取中断不能留下非法本地提交。
6. 分别记录真实新节点、唯一依赖读量、所有逻辑比较、Hash、审计、构造、落盘、RSS、库/WAL、网络／job 总量。只有原声明资源门与明列 v3 job/reader 限额均通过，才可声称该受测域可用。通过本节也不自动关闭 #1323 六 Destination 的其余义务。

### 5.10 草稿本身的有限核验（不等于采纳或实施）

Python 与独立 Node 实现实跑：23 个正例的根一致；其中 22 个可渲染值的规范 JSON 字节一致，未解析的批次占位不渲染；96 个正例可达节点逐一对拍。10 个非法根全部拒绝，覆盖错误序列分解、长度、块边界、UTF-8、数字 token、键顺序、内部节点冒充值、内部节点 Hash 不符、嵌套 JSON 文本视图及文本视图内批次占位。向量文件共 123 个节点，包含刻意损坏的负例，不能把混合向量库整体当有效内容库。

输入向量 SHA256：`78984501cbb465be8251e3d212e34c0cb6fb33a5aab9a4999a3f1db592ce71f0`。见 [有限编码向量](./1394-CONTENT-FORMAT-VECTORS.json) 和 [独立 Node 实跑结果](./1394-CONTENT-FORMAT-INDEPENDENT.json)。第二实现只根据本节规范和向量输入/预期输出编写，未读 Python 实现。

独立合同复核指出的四项缺口已在本稿集中修订：Delta 的合法读取 anchor、持久 FinancialBasisEvidence 的唯一字节形状与全部列明的历史闭包、资源字段及旧限制归属、片段响应 exact 形状。复核追加的排序检查时序与 recorded[].e_commit_proof.image 遗漏也已修正并按当前源文件核对。

这些检查只覆盖草稿的指定合同缺口和有限编码向量。SelfBatch 的真实业务出生许可、服务端片段证明、完整前端安装、E/B 原子导入、全 9011 条双跑及新资源数值仍须在获准实施后按 §5.9 实际验收；本稿不声称这些已完成。原 Q64 超限证据保持。
