# #1394 SQL 转换边界与来源临时集合

工位仍在 `/Users/silencehan/.codex/worktrees/issue1394-json-business/NewChanlun`，接续 `d01175eab5883bd5027dfeaea779e2ac096c71aa`；#1394 / SPEC #1340 已批准 C，同范围隔离实施与验证，不合 main、不改 GOAL 事件、不扩产品前端/K4。

Root 接实际 content-store/目录 SELECT；shared_budget_sqlite_impl 接控制/schema/epoch/profile 的 SELECT；content_bi_borrowed_adapter 接 raw/对象/辅助索引 SELECT；owned_json_impl 接来源临时集合及历史 SELECT/基础摘要集合。sqlite_temporary_allocation_boundary 的新库 guard/primitive 和真实 UTF16 probe 已交回，owned_audit_independent_review 只读复核。

SQL 迁移在真实 prepare/query 所有者处分配并持有 statement，不枚举或猜指针、不转型 rusqlite 私有布局。严格类型先于转换，column_text 后立即检查 NOMEM，column_bytes 后再检查，借用锁定 row/statement 生命周期；成功 finalize 检查，失败清理不覆盖初始 cause。

小 UTF16 夹具转换未触及 malloc 拒绝的红日志保留。新增大真实 UTF16 TEXT 只用于验证 afterROW 转换分支，不修改 lookaside 或原 50 MiB 测量额度；实际 UTF8 数据库完整状态/F/work/990 对拍另行核验，两个证据不互相代替。

来源集合直接同账创建，数字顺序以排序借用索引保持，集合并集/差集采用线性归并；不在普通容器完成后复制成成功 token，不以低额度失败切换普通路径。

生产捕获、完整资源资格与正式 9011 仍未通过；原16 MiB失败域、新资源合同、main和真实外效批准门保留，goal保持 active。

本轮本地交付已冻结于 `243a01ee753361ea032e12d27b0fa2c28d8ace79`。18 项应用检查、2 项库检查、1 项借用拒编检查及真实数据库正例/SQL 拒绝均通过；完整状态与 990 非计时事件和原基线相同。独审为 PASS_WITH_SCOPE_LIMITS、零未修复确定缺陷，原 16 MiB 红例保留，证据见 `.scratch/sql-checked-source-collections/ROOT-ACCEPTANCE.json`。

全部实施/比较/独审子代理已接回并停止；自有外盘 target 清理约 340 MiB，保存程序、源及数据库不动。整图 goal 仍 active，剩余分配路径、真实业务链和正式 9011 尚未验收，不关票、不扩 #1460 前端。
