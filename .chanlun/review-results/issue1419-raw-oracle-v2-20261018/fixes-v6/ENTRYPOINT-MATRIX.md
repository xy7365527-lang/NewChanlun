# #1419 v6 公共入口、费用和状态提交矩阵

工作草稿。固定基线 `1cb46982baa0f6ca06cd0884d602bcaaa8395f2e`。完整函数/方法枚举见 `ENTRYPOINTS.json`，对照管理者固定入口图 `RAW-LIFECYCLE-ENTRYPOINTS-46A.json.reference`。枚举方法为对三个 driver 模块做完整 AST 顶层函数及类方法扫描；下表覆盖本轮合同里的公共入口及新辅助函数，不把 AST 名单当语义证明。

| 入口 | 实际校验责任和可组合路径 | 同一 WorkBudget 准入 | 可信状态提交 |
|---|---|---|---|
| `Collector.__init__` | 固定 envelope 会话身份；显式持有原 Budget/WorkBudget | 身份/query epoch 在 deepcopy/mkdir 前；上下文槽位在创建前收费 | 只建空上下文，无可信快照 |
| `Collector.call` | 原始回复/status/headers；严格 envelope/hash/发布身份/因果。识别的 op 必须匹配 route；snapshot 请求/续页身份和回复 fixed/projection 查共享上下文；content 请求/回复 anchor 查共享上下文；watch 原始直调拒绝，必须经过 `watch` | 原外来树扫描后才序列化、hash、写盘；费用及请求序号跨次保留 | 无可信状态写入。返回单页不含收据，也不声称闭包已验 |
| `Collector.snapshot` | `fixed` 先统一 `_check_cut`，再实际 `call`，分页六族/计数/顺序/token/完整切面不变；固定代际，不以 latest 补缺 | 参数、页树、rows、token 在复制/入队/散列前沿原费链 | 无；单独分页成功不能 bind_watch |
| `Collector.content` | `_check_anchor` → `_check_projection` → `_check_cut`；实际 `call` 核原件；连续路径/单位/长度/内容页 token | anchor/path/unit/limit、回复页、token 均先准入 | 无；明确 `closure_verified=false`。本准备器只实现 projection anchor，不接受其他锚伪装成已验证 |
| `Collector.closed_snapshot` | 实际 snapshot→content→call；完整 `projection/close_projection` 验原引用元数据、所有节点头/hash、路径及原始公共 payload。`records` 在 `plain` 之前拒绝 0x07；0x07 路径解析用已实际收到的累计节点 | 参数、队列、路径副本、每页节点、闭包、raw 类型、上下文扫描/复制、收据元数据均在同一账；不补默认 WorkBudget | 全部成功后一次替换 `__state`：anchor、按代完整cut、按代/模式projection、最新收据元数据。失败不发行/撤销既有有效收据 |
| `CollectorV2.closed_snapshot` 直调及组合 | 直接继承上行唯一实现；基类/子类、已绑定/未绑定路径均受同一门约束 | 同上，无 V2 新账或替代校验入口 | 同上 |
| `Collector.bind_watch` | 参数是本实例最后完整验证发行的 `SnapshotReceipt` 对象；普通 dict、未发行对象、另一 Collector 或旧失配收据拒绝。游标逐字段对应该收据内部完整 fixed | cursor、实际存储的 fixed/上下文在复制前收费；收据无调用者可填可信字段 | 一次绑定，禁止重绑；不读取调用者 `assume_verified`/closure_verified 自报旗标 |
| `Collector.watch` | 必须续接上次完整游标。实际 `call`；具名 accepted_cut 与 delta 的全部对应字段、固定身份、累计 input_frontier/事件数、提交去重；完整 observed_head、delivery_frontier、has_more；页末游标。观察到的更高 head 与后续同代数据也须一致 | 所有输入/页/元数据/上下文/去重账及副本使用同一 WorkBudget；成功前最后查截止 | 整次调用成功才提交 staged cut 表及 watch fixed/cursor/seen_commits；任何后页失败不保留前页候选。费用/原件/请求序号不回滚 |
| `Collector.session_state` | 提供完整语义状态的独立观察副本，包含收据的 fixed/root，排除仅供对象身份比较的 receipt 标记 | 扫描真实状态后复制 | 无；修改观察副本不能修改 Collector |
| `_check_cut` | 查全部字段集合、会话、历史视图合法值、固定身份、已知同代除视图二字段外逐字段一致、跨代前沿关系 | 入参扫描、历史表遍历收费 | 无；可传 staged 字典，不把该字典登记为可信 |
| `_check_projection` / `_check_anchor` | 统一走 `_check_cut`；同代同模式的 qcut/root 元数据冲突拒绝 | digest/ref/anchor 先扫描 | 无 |
| `_stage_cut` | 只有 closed_snapshot/watch 把局部候选交来，重核切面；同代 publication 比较时只规范化已经验证合法的 mode/as_of 配对 | fixed 扫描、记录槽位收费后复制 | 只修改传入的局部字典；不提交 Collector。没有公共 install/register 方法 |
| `bind_named_submissions` | 真实请求摘要、会话、具名事件数、重复输入/代际；AsKnown(g)/Recomputed(null) 两种 cut 配对 | 调用者必须传同一 WorkBudget，两棵树先准入再散列/复制 | 无 Collector 状态写入，更不会发行收据 |
| `records` / `compare_domain` 直调 | 仍从实际 Graph 解码，对原始 payload 任意深度查 JSONText，八类全集及字段门不变；compare_domain 再做定义值比较 | 原 Graph.budget 累计；不得仅相信 closure_verified 布尔值 | 无 Collector 状态写入；调用它们的返回字典不是快照收据 |
| `Graph/header_from_raw/projection/close_projection` | 原有完整闭包与 ContentRef 原元数据门逐字节未变。单独闭包成功也没有 Collector 收据 | 原 WorkBudget/迭代器收费 | 只返回验证图，无权安装 Collector |

普通调用者仍须提供：声明会话身份、离线 exchange、固定目标代际，以及具名输入与真实接纳回执的映射。这个映射在纯离线试验中是协议材料，不是已证明 S 真实运行的事实。新实现自己完成上述快照闭包、raw 类型、共享上下文、watch 信封及失败原子性校验；这些责任不再推给“外面会核”。watch 的 delta 内容闭包/应用到 Snapshot 的完整领域生命周期没有新增实现，仍返回 `closure_verified=false`；旧 UNVERIFIED 不因此转成 MATCH。

模式处理依据固定 JS `fixed()`、watch head 比较和 Rust `projection.rs`：AsKnown 的 `as_of_generation=g`，RecomputedWithRevision 为 null；两种视图合法且各自保存 projection 根。`seq_range` 按其实际值同代比较，不把跨 writer epoch 的 `seq_range.to` 当成累计 input_frontier。冻结的 1/65/65 义务仍按具名接纳事件数计；不因照抄独立 JS 子断言而退回旧前沿判据。未来正式 source 接线及完整消费者等价仍待独审/实际运行。

Python 私有属性和收据对象只提供正常 API 的不可混用约束，不是密码学认证或对任意反射/猴子补丁的隔离承诺。测试仅替换 exchange、`Path.mkdir/write_bytes`，没有安装私有可信态，没有替换 `call/snapshot/content/closed_snapshot` 或安全校验函数；预算耗尽后的状态断言只读私有状态，避免观察操作自身再次超帽。继承的 F2 专项用 spy 观察分配顺序，不替换安全判断。
