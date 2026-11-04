# #1419 raw/driver v6 修复报告

本轮已修复 F8-1/F8-2 的离线公共入口与会话合同，定向验证通过，交管理者冻结后独审。**没有授予产品执行资格**：authorization/execution_authorized 仍为 false；正式 source/tree/binary/frozen rule 仍为 PENDING_SOURCE。票仍开放，工作草稿不构成整票或父图验收。

固定接手基线为 `1cb46982baa0f6ca06cd0884d602bcaaa8395f2e` / tree `71c474dcbd39e2cddd649d3f7b39caf5f35e67e3`，当前独立分支 `codex/1419-raw-fix-v6-20261018`。工蜂为 Sandcastle 外部 Codex，session_id 以运行时事件为准，不声明原生子代理注册。没有开启其他工蜂。

## 修复结果

`Collector` 现在拥有一个共享会话状态，完整快照闭包和公共原始类型检查通过后才安装固定身份、按代切面及按代/模式 projection，并发行进程内快照收据。`bind_watch` 必须消费本实例实际发行的最新收据；普通 dict、外来 Collector 收据、旧失配收据及伪造实例均无效。V2 直接继承同一闭包入口。snapshot/content/call 直调查同一上下文，不能产生可信收据；原始 watch call 必须经过 watch 事务入口。A 验证后不能绑定 B，也不能接受同代 frontier999、alien cut 或 alien profile 的第二份合法闭包。

watch 实际读取并核对 observed_head、delivery_frontier 和 has_more：观测头是 RecomputedWithRevision/null，同代完整 publication 身份相同；更高头允许，且记住其身份以约束后续相同代。投递 head 必须等于观测头，retain/first 为正、first≤head+1、无 Gap 游标不能越过保留前沿；末批不越头，has_more 严格等于 head>末批。所有 cut、信封、游标、具名提交及去重检查完成后，才一次提交候选状态；去重账排序先预留比较费用，返回的 work 包含闭包后原始类型/上下文/收据费用。关闭快照或 watch 中的闭包失败、元数据拒绝、传输异常、截止/工作耗尽不提交可信状态；费用及原件不回滚。

允许同身份的新代和历史读取；AsKnown(g) 与 RecomputedWithRevision(null) 按固定正式来源解释为读取视图，分别锁定 projection。未将每个 header 强行等同 delta，也未一律拒绝 has_more=true。具名提交的输入事件数仍决定跨 writer epoch 的累计前沿，1/65/65 不退回 seq_range.to 推算。

本轮真实 raw 类型链补测同时发现：旧闭包队列遇到 JSONText 只读当前页节点，节点早页已送达时会误报缺失。现从已实际收集的页读取节点，所有重复访问仍按原预算计费。合法非 payload JSONText 正控通过，公共 payload 任意深度 JSONText 仍拒绝。`content_closure.py` 和原 records/compare_domain/validate_public_payload/plain 源码保持原字节，原引用元数据和来源闭包证明未替换。

## 验证与原件

执行：`python3 -B .chanlun/review-results/issue1419-raw-oracle-v2-20261018/fixes-v6/run_checks.py delivered`。

实际输出 `Ran 22 tests in 20.884s / OK`，0 failures、0 errors，173 条观察，见 `tests.delivered.stdout`、`delivered-observations.json`。`delivered-raw-exchange.jsonl` 保存具名核心轨迹的真实内存 exchange 请求/回复原字符串（回复含实际重新计算的摘要），不是事后重造的摘要表。

| 定向轨迹 | 结果 |
|---|---|
| 基类/V2 × 7 类同代身份冲突 × closed_snapshot/snapshot/content/call_snapshot/call_content | 70 组拒绝；每种修改独立重建合法 hash/qcut 并实际 close_projection 通过；原状态不变，随后原 watch 可继续 |
| 本 Collector A收据、B普通dict、外来收据、未发行实例、旧失配收据 | 仅正确实际收据可绑定；所有游标字段、不重绑、跨次去重另测 |
| 同代 observed_head frontier/profile/cut/evidence、错误模式/as_of、delivery head/first/retain/policy、has_more矛盾/类型、capture | 15 类拒绝且可修正后继续；费用增长 |
| 合并/分次/交错 snapshot+content 的 1/65/65 | 完整最终语义状态及 deltas 相等，last_seq=269；work=239524/246432/262462（含最终观测副本费用），calls=51/53/59；同一实例原1000000工作帽，未重置 |
| 较高head + 正常分页has_more=true；Recomputed/null；历史/未来快照交错 | 正控通过；较高head跨次变脸拒绝 |
| 后页非法、第二次传输异常、metadata耗尽、中途工作耗尽 | 整次 watch 原子拒绝；前两类修正后继续，后两类保留耗尽费用及旧状态 |
| 闭包缺节点/错误items/非法公共类型/metadata/传输/预算；已绑定后的新代闭包失败 | 不发行收据，不改变已有上下文 |
| 八受测kind × 普通/data JSONText/嵌套/数组 | 32 组真实闭包链；普通成功，24组非法原始类型在收据发行前拒绝；合法非payload JSONText另有正控 |
| 受影响 test_v2 分块内容/同预算 | 190次请求、work795047通过并使用真实收据绑定；每页重复全图在41次请求、work999999拒绝，不加帽 |

F1 原 JSONText/data 反例、F2 构造/元数据/碎片/累计费用以及 Graph hash 前准入锁定向复跑。F5 完整源文件、F1 四个函数与固定基线逐字节相同，复用 v5 正式独审的既有头/引用元数据证据，不重跑 139 前缀几何或无关 toy。

初跑与中间失败保留：`tests.initial.stdout` 是10项1失败，原因是测试的大头未超原帽；`tests.repaired.stdout` 是17项全过；历史文件名 `tests.final.stdout` 是扩展至22项后的1失败，实际为早页 JSONText 节点未被读取；`tests.accepted.stdout` 是修复后的22项通过；随后 `tests.verified.stdout` 覆盖去重账排序预收费，最终 `tests.delivered.stdout` 覆盖含提交/观测复制的完整费用回报，后两轮也均22项通过。对应源码快照和哈希由 `TEST-SOURCE-REVISIONS.json` 绑定，失败没有改名冒称通过或删除。

入口→校验→预算→提交的完整责任说明见 `ENTRYPOINT-MATRIX.md`；符号/行号见 `ENTRYPOINTS.json`。固定 JS/Rust 源摘要、逐字摘录、1cb/46a 相等核验在 `STATIC-WIRE-SOURCES.json`。当前没有图工具，未安装/索引，复用的 XERJ 记录与管理合同/issue/独审引用在 `READ-SOURCES.json`；未声称本轮在线检索成功。

## 前件与尚未验证的边界

已接通的责任是本 Collector 的真实完整快照验证、公共原始类型检查、收据发行、共享切面/观测头约束、累计费用及失败原子性。调用者不再能自报“快照已验证”。普通协议前件仍包括声明的会话、离线 exchange，以及输入与实际接纳回执的具名映射；bind_named_submissions 检查映射形状/摘要/事件数，不能证明真实 S 执行。本轮测试以实际 helper 和同一预算构建该账簿，所有传输仍为明确的内存试验。

watch 尚不证明 delta 内容闭包/应用到 Snapshot 的完整领域语义，返回 closure_verified=false；known_at、AsKnown 生命周期、entity_revision、raw_history、withdrawn_objects、first_visibility、低级来源封口、完整22轴等原 UNVERIFIED 全保。逻辑费用不是 OS 硬帽，同步 I/O 截止和真实信号未验。Python 收据是正常 API 对象身份约束，不是密码学认证或任意反射隔离。

市场源认证、原量、完整历史、正式 raw→S3/Q 非空 full-group/至少两中心一中心对、正式双跑、持久恢复、浏览器及整票验收仍欠。未运行 cargo/rustc/Lean/lake、产品/服务/DB、Long/历史或真实传输/信号；未读取凭据，未 commit/push/merge/切分支/改 Git 配置/操作 tracker，未覆盖其他工位。只有授权 driver 三文件、必要根清单及 fixes-v6 新实体有差异。

根 MANIFEST/SHA256SUMS 覆盖本轮实物。`changes.patch` 相对固定1cb包含本轮全部源码/测试/报告实体，排除补丁自身和两份可再生根清单，避免循环自引；`SCOPE.json` 给出精确集合。管理者应等待本进程实际退出后冻结该差异，再安排独立审查；本报告不预签独审结论。
