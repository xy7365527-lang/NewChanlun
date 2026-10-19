# #1394 C：S16 重复审计的限定只读诊断

工作草稿，2026-10-01。依据产品基线 `ba6e0472f3d2c6531ec7355f643e5299e2c230ad`。本报告只读指定源码与现有 S16 证据，未实施、构建、运行探针、启动服务或运行 9011。目标文件未被知识图索引，已按指定文件定点 fallback。所引 11 份产品/合同文件逐份 SHA 与该基线一致，详见同名 JSON；shadow 的 cfg(test) 源 SHA 与产品原源 SHA 分开登记。

**结论：合同内有一个小差异可供实施评审，即消除同一次 SQLite 成功读取到 codec 的相邻重复 F。最窄范围只改无 Budget 的审计读取，沿既有 `CheckedFrame` 调用栈传递当次证明。它不减少 SQL 次数、不删三次审计、不复用跨读取帧、不免除闭包/来源/变化集合判定。没有证据可据此承诺 9011 时长。整次 audit 帧缓存已被当前 API 明文禁止，不应顺手实施。**

## 重复发生在哪

| 调用位置 | 实际重复及仍须保留的含义 |
|---|---|
| `state.rs:99-111,125-160` | 一次完整 audit 先控制、真实原始账、完整 history，再新建第二个 ReadStore/HashReader，逐 cut 做来源审计与目录证据。两个 reader 的 overlay 独立，后者重新展开同一持久图。 |
| `history.rs:91-136,143-223,420-424` | 先载辅助/对象索引，Public 全历史根批核、Template 原件批核；每代 decode/assemble manifest、按独立出生索引 resolve、核生成成员和变化集合，末尾再 Public 核解析根。overlay 在索引加载后及每代末清空，派生写不是 SQLite 写。 |
| `aux_index.rs:104-108,808-882` | 每条 witness/relation/observation 的多个根分别 `sc::verify`，其后身份、槽位、生成原件与 wire 绑定仍逐行读。它没有像 object_index 那样批核闭包。 |
| `object_index.rs:738-759,879-905` | load 已用 `verify_many` 去重共享闭包，但 bind_originals/wire_at 每行仍读；每 cut 又枚举 generated objects、读 sealed fields、重建 immutable。数据结构不同和当代元数据不能只用出生根相同代替。 |
| `source_audit.rs:93-109,148-183,218-249,253-318,511-571` | 每 cut 新建 Public verifier，再从该 cut 的有效原始账构造 wire/numeric/raw hashes；每个数组成员单独 lookup，每个对象/见证读取字段。共享数组祖先与标量因此重访。 |
| `catalog_evidence.rs:403-404,414-518` | 同一 cut 对 generated_state、delta 再分别完整 verify，再核运行结果、冻结 metadata、counts、axes、CC-056 修订与七变化依据。它与来源审计、history 的业务判据各有职责。 |
| `history.rs:752-819` + `content_store.rs:80-94` | overlay 每次读 clone body；checked_read 每次重新形成 CheckedFrame。持久分支调用 `ReadStore.read → read_node`，SQL 读到实际 body 后先 F，然后 HashReader 再 `CheckedFrame::checked` 做 F。计费的普通 read 还调用 Budget.node 再 F。 |

S16 `PHASE-SUMMARY.json` 中末代每次 audit 的持久读为 576,246、overlay 读为 780,434，合 1,356,680 次；179,322 次 overlay put，线程 F 约 650 万次。根已核联合 digest 唯一 31,366、重复 1,325,314，不能把这个联合重复数改称 SQLite 重读数。两次 fresh 的三个完整 audit 合占 handle 的约 88.4%，SQL final commit 为 37.36/40.67ms。这些数不测磁盘 I/O，不把 F 次数等同唯一节点，不外推 9011。现有 source/hash/manifest/派生写也会计算 F，不能把全部 650 万次归给持久读双核。

三个 audit 分别在 `driver.rs:191` Accept 前、`:268` Begin 前、`prepared.rs:73` 生成暂写前；Accept 和 Begin 会推进真实控制账并提交，`driver.rs:290-300` 又创建下一事务。因此重复形状不构成删除理由。恢复在 `driver.rs:339` 的审计也须保留。

## 已有 memo 为什么消不掉

`Verifier` 的 seen/pushed、elem/field memo 按 digest 与 Flavor 区分，字节 memo 还带 UTF8/NumState，见 `s_content.rs:1472-1508`。`verify_many` 只把同一一致源、同一 Public 或 Template 角色的根并入一次 verifier，逐根 ContentRef 四字段声明仍核；退出调用即丢弃，见 `:2252-2282`。它不能覆盖后续 path/render、其他 verifier 或来源绑定。Public/Template/JsonInner 不可共用一个按裸摘要命中的成功位。

`source_audit::checked_refs` 只记同一次 `audit_generated` 的 `(digest,numeric)→coord`。当次 raw 有效映射已固定、state 已全核，复用的是完整八字段原件匹配；数组成员头与严格升序仍核。它不包含对象 ID、配方、组/见证/目录校验，也不能跨 cut 的源修订带出，见 `source_audit.rs:148-183,248-249`。

`AuditJob` 的节点表与 Receipt 去重的是用量，不是验证，见 `audit_budget.rs:47-91,110-123`。同 job 相同根节点只记一次，但顶层记录每次访问仍累计。旧 source memo 的 test retention 上限明确不映射 writer/Q/job 资源，`state.rs:396-400`。Q 的 `AuditReader` 则保留已核 query_only 私有图像并按同连接版本与文件身份夹持，不开放写面，`capture.rs:934-971`；它不是 writer audit 的跨事务授权。

## 合同允许的最小差异与条件

正式源 SHA `fab912c6ebe3613969d65fc924368a1d5c9b343f0c3f7a123241645413771d30`。`APPROVED-SOURCE.md:65` 要求当前原始行逐字节相同才允许旧结果复用；`:286` 要求原件、schema、全部判据上下文及相关根绑定；`:292` 保留当前捕获、写前审计和无跨请求读事务。`:301-305` 区分节点一次计量、记录/请求实际访问及绝对期限。更具体的 API `s_content.rs:421-422` 已允许同栈传递 CheckedFrame，且禁止跨读取缓存帧。

**优先建议只评审两份产品文件与对应靶向测试的差异。** `content_store.rs:80-94,155-158` 的真实 SQL 行在本次读取内构造 CheckedFrame，普通 ReadStore.read 仍消费帧为 NodeRead、保持读后 F 拒绝；`history.rs:778-808` 的 `budget=None` 持久分支调用 source.checked_read，逐次核 frame.digest 等于本次请求，再把同一帧交 codec。overlay 分支仍对本次 clone 出的实际 body 做 CheckedFrame::checked，绝不信任 put 时的 digest；budget=Some 先保留现有实现，Budget API/Job/资源字段不改。这样是当前字节同栈搬运，既不积累节点表也不移除真实重复读取，适用于三次 S16 writer 的 `state::audit` 无 Budget 入口。若实现采用其他内部结构，也须证明同样条件。

需要保留每次外部 deadline/precheck、source 错误上抛、Missing、坏 tag/body、读后门和根请求摘要对照。不应把 `Budget::accept_prepared_frame` 改当已缓存帧入口，其注释要求当次读取及 prepare_node 先于相应 F。覆盖计费分支需要另做门序证明，不能直接先 source.checked_read、再套普通源的起钟语义：当前 SQLite 的 source.read 在 Budget.prepare_node 前已核 F，所以 SQLite 坏帧先抛错、未起钟；MemoryNodes 坏帧则 Found 后起钟再在 CheckedFrame::checked 拒绝。两者不能用同一个未分析的伪造测试替代。

另一处可独立评审的合同内缩读是 `aux_index::load` 的多个 Public 根并入现有 `verify_many`，维持同次不可变源并逐行做全部身份/生命周期/wire 绑定。它会改变故障被首先发现的顺序及临时根集合，须核既有错误门与内存边界；本报告不把它作为已获验收的修复，也不建议同批扩大到 manifest、逐cut来源或 catalog 重写。

## 必须重读与最强反例

- 每次 source 真读，当前 tag/body 必须核 F；删去的只能是同一次已核帧下传时的第二次 F。换事务、Accept/Begin/发布后的状态、recover/epoch 变化、重启冷读，以及任何写面或源身份变化，都不能继承旧成功。ReadStore 持有 Transaction 只是读取同一事务，不构成所有调用者永不在该事务中 UPDATE 的类型证明。
- 同一 Transaction 内先合法读取，随后 SQL 改 tag/body 或 DELETE，再读同 digest，必须拒绝/缺失；TxStore 新增、回滚后新事务读也必须依真实存储。第二次读 source 返回 Err 的替身源必须上抛，不能凭第一次帧跳过真实调用。
- 先 Template 后 Public 的同根占位、同 JSON_TEXT 根先 Value 后 JsonInner、第二根 ContentRef kind/extent 错误，仍必须拒绝。现有 `s_content.rs:2867-2954` 覆盖这些角色边界，优化不得将它们退化为 digest 已见即可。
- 合法 Hash 的替代 snapshot/manifest、源 revision 改变而 coord 未变、writer epoch 错配、修订/撤回/replaces 与目录 counts 错配，不能凭完整图校验成功放行；各业务链仍实际执行。
- 停止门须在下一次 SQL 前拒绝，Missing 不伪造首读，坏帧不入节点账。同根复访不再加 nodes/node_bytes，但 record 重访仍累计、绝对期限不续期。现有 `audit_budget.rs:179-274,307-338` 只证明 MemoryNodes/替身源及用量门；需补真实 ReadStore/SQLite 路径同次 F 计数与故障/期限对拍，才可声称相邻双核已经消除。

最小建议不保留新 body map，原 Vec 所有权移动到 CheckedFrame 后被消费；不要求新增内存资源值。若采用整次 audit 帧/字段/闭包成功缓存，须解决当前跨读取禁令、明确只读源捕获类型和来源角色，以及临时保留的容量计量。`q_retained.rs:1-3,116-135` 只计被保留图像拥有的 Vec/String capacity 与类型尺寸，明文不含查询临时解码；job 的 staged/store 是实际分配的文件、索引、日志合同，不能私自当 audit heap 上限。Q max_capture_bytes/max_cache_bytes 与 deadline 仍各自执法，不能用 codec 节点账去替代。writer 无 Budget 路径也没有可直接挪用的新增 audit-heap 数值。此处缺的是被批准的临时缓存边界及其计量归属，不是一个默认容量。

以上是源码诊断和可行条件，所有建议未实施、未运行其反例。9011 的端到端、完整客户端与新资源域验收仍由根任务按原门完成。
