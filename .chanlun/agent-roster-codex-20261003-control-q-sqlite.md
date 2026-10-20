# #1394 控制审计与 Q SQLite 所有权接线

票与授权：#1394 / SPEC #1340 已批准 C。接续本地 `905ffa559a17429d27ea3a117106c3b921585d64`；保持旧输入、失败域、主仓未提交面与 main 合入门，前端/K4 不纳入。

工位：`/Users/silencehan/.codex/worktrees/issue1394-json-business/NewChanlun`，分支 `codex/1394-json-business-20261003`。

分工：Root 负责实际 state/capture 调用者与目录临时所有者；owned_json_impl 负责共享控制/schema/epoch/profile 链；content_bi_borrowed_adapter 负责已有同账集合和消费字节接口；shared_budget_sqlite_impl 负责共享 Envelope/Clock 视图核。control_legacy_json_owner 的明确 legacy-shape 同账解析支持、sqlite_temporary_allocation_boundary 的鲜进程 Q+SQLite 生命周期探针已接回，待统一构建验证。

控制层保持原顺序：capture → markers → schema → profile → epoch → control history → publications。strict wire JSON 与既有 profile/branches JSON-shape 是不同输入判据；各自保持原规则，禁止严格解析失败后转宽松重判。Clock、Envelope 和控制消费判据只保一份共同核。

新资源合同、默认数值和资金政策未采纳。旧 16 MiB 失败与原正式 9011 终态不销；50 MiB 仅沿用具名测量。whole audit、生产捕获、正式 9011 和整图 Destination 均未据此宣称完成。

本轮验收：9 项库支持、12 项新应用探针、17 项有变更的控制合同检查，共 38 项通过。正常 `s_session` 编译通过；旧 905 的 59 项冻结矩阵未重复扩测。源码/Cargo 输入 494 项已冻结。

真实 Q 鲜进程首先安装 SQLite 同账 allocator，随后执行真 Fixture/捕获、Owned 审计、Packed 创建与两次 checkout/restore。实际 SQLite OutOfMemory code7 显示文本丢原因的边界已按 typed code 修复；拒绝时旧页/raw/typed 指针和账保留，清注入后重试成功、finish live0/broken=false。该证据只关闭具名两发布夹具的 Q Rust/SQLite 缺口。

同一数据库与上一冻结版完整状态、F=8,497,570、work、SQL节点读、capture读、overlay和990个非计时事件一致。50 MiB 扩覆盖测量峰 50,764,086 B，10,869,352 次分配全释放；16 MiB 仍真实拒绝且成功型外壳 exit101/0pass1fail，原件保留，不计绿。

独审覆盖全部 15 个源码差异，PASS_WITH_SCOPE_LIMITS，确定未解决问题 0 项。源码 final 相对二进制 r4 输入只改 content_store 的资格注释，已分别绑定，不声称编译输入字节相同。

未取得资格：旧无 definition profile 的 evidence Reader/bytes、残余审计临时集合、普通 Q 工作集、rusqlite 内部 TEXT 转换型 OOM、外层诊断、allocator 开销及物理 RSS/I/O。证据在 `.scratch/control-q-sqlite/`；子工位均已接回结束，main/新资源合同/正式域启用均未授权，goal 保持 active。
