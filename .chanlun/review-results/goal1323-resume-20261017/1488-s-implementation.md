# #1488 S 实装报告（2026-10-17）

状态：五个授权文件已落候选改动，R/S 接口已接线；仅 rustfmt 与 diff 检查通过。没有运行 cargo、六成功锁或性能测试，不能标记工程验收 PASS。R 的费用迁移、owner 唯一 runner 及独审仍是整体前置。

工作树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1488`。基线 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`。遵循接口候选独审修正，未修改三个禁区。未 commit/push/外盘/服务/真实资金动作。其他代理并行修改 R 文件，未覆盖。

## 实际实现

- `rust/src/bin/s_session_v2/staged_nodes.rs:11`：ChargedNode 以正文在前、NativeReservation 在后保存；StageOwner 的 nodes 在 tree_permit 前，树节点全部 drop 后才释放树保留费。树上界采用节点高水位，不因选择性 flush 后 len 减少而缩账。
- `staged_nodes.rs:97`：`new_admitted(tx, &PublicationScope)` 从 R 的封闭发行器取得 `PublicationStageIdentity`。token 仅含 Weak，不长持 journal 强 Arc；每个实际 TxStore 读写及 flush 都核当前 tx/work/journal，失败不可在同 owner 重试。
- `staged_nodes.rs:145`：`read_scoped` 为返回的独立副本单独预留正文许可；owner 原件许可不转借给复制帧。旧无 RAII checked_read 由 ScopedFrame.into_frame 保守 retain，主链 scoped API 可随载荷释放。
- `staged_nodes.rs:247`：`FlushCut::{Source,Row,Final}` 只遍历显式根闭包，校验所有父子节点（包括 SQL 旧父指向新暂存子），再用无 stage 的 TxStore 真实写 SQL。队列用同 work 的 ScratchVec，seen BTreeSet 在分配前取整树许可；不扫描历史控制表。SQL 与 overlay 同摘要异原件拒绝。成功写入/确认已存在后才移除暂存节点并释放正文；tree 许可保留。实际 INSERT 数由 total_changes 差及 changes==1 核认，不按 put Ok 次数冒充。
- `rust/src/bin/s_session_v2/content_store.rs:629`：增加 `NativeReadLease::new_admitted_staged`；沿用原 read_uncommitted/format/schema/poison 门。SameMaterial 只缓存真实 SQL，overlay 读使用 stage 的独立 ScopedFrame；stage put 不伪造 SQL epoch。TxStore 的 stage scoped 读取不再制造 permit=None 副本。
- `rust/src/bin/s_session_v3/prepared.rs:303`：bound publish 显式接 `Option<&PublicationScope>` 并下传同 work。只有有真实 parent 时才允许 scoped staged 路径；standalone work+stage 仍拒绝。严格路径由 new_admitted 创建 stage。生成 lease 完全析构后才允许注入、source flush 和材料签发。
- `prepared.rs:720`：source flush 根取 `before_native_source` 注入之后的 `generated.generated_state`；随后同实际根交原 source audit。finish products 和 row writers 继续用 stage-aware TxStore，R 的 StageGuard 调用 Row cut。所有材料入口由 helper 分派到同 publication 能力，无 parent 保留旧入口。
- `prepared.rs:430`：严格 staged 运行时根向量用同 work 的 ScratchVec，不新造无账 Vec。
- `rust/src/bin/s_session_v2/content_publication.rs:67`：workspace 没有被丢弃。生成 lease 结束后，先将本代 manifest/generated/template/scope 闭包真实落 SQL，再由原独占 publication workspace 租约消费；无 workspace 时以同 work 的 stage store 解析。
- `rust/src/bin/s_session_v3/commit.rs:295`：final 根显式覆盖 manifest、resolved snapshot/delta、catalog evidence、generated_state（含 catalog_axes/evidence）、两模板、scope 与全部运行时根。此处仍无 SQL COMMIT；外层 owner 负责 runner 尾检后提交。无 scope 的旧诊断 stage 仍走旧全表 flush 以保留原独立测试口径，strict publication 不走该分支。

## 费用保留与释放

| 载荷 | 收费点 | 释放点 |
|---|---|---|
| 新 stage 正文 | put_new 中 native_body_reserved 在分配前 | 成功真实 SQL flush 后移除 ChargedNode，先 body 后 permit；未选中正文继续持有 |
| stage BTree | 插入前 native_tree_storage，旧许可同时保留 | 新许可承接后旧许可释放；高水位不下降；最终整树销毁后最后许可释放 |
| 复制 ScopedFrame | 每次 read_scoped 单独 reserve | frame 正文 drop 后其许可 drop；不依赖 stage 是否还在 |
| cut queue/seen | ScratchVec / 整树许可在分配前 | cut 完成或失败时容器先于许可析构 |
| runtime roots | strict path ScratchVec | final 使用完成后离开 publication 调用栈释放 |
| journal identity | Weak 身份，无新强 Arc 长借 | stage drop；短 check 的 upgrade 在返回前 drop |

本表只覆盖 S 引入的分配；旧 candidate/raw/journal/PendingSet 的连续费用迁移由 R 报告举证，不能拿 S 表替代全图资金/资源资格。

## 新增测试源码（未执行）

1. `staged_scoped_copies_hold_distinct_permits_until_payload_drop`：两个复制帧分别计费，drop 一个只退一个；drop StageOwner 后另一复制帧仍存活且仍计费；最终回零。
2. `staged_selective_cut_follows_sql_parent_and_keeps_unselected_payload_charged`：旧 SQL parent 的新 child 能选择性落库，孤儿等待后续 cut 且仍收费；重复 cut 不多计 INSERT；len=0 时树保留费仍存，owner drop 后归零。
3. `staged_native_lease_uses_overlay_without_sql_material_cache_and_poison_cannot_retry`：staged lease 写后 SQL 仍空、复制帧不永久占缓存；SQL 原件损坏不能被 overlay 遮蔽，失败后 read/flush 不可重试。

以上测试在模块内部装配 work，只证明存储层会计/读写合同，不伪装 owner PublicationScope 签发测试。原六成功锁与 16/8 MiB 上限未修改，仍须在允许的远端环境执行。

## 已完成轻量验证和待验

- 五文件 `rustfmt --edition 2021 --check`：exit 0。
- 当前恢复树 `git diff --check`：exit 0。
- 未 cargo：不宣称编译、borrow-check、测试成功。R/S 合并独审须重点核 MaterialAccess 生命周期、row flush 的 mutex 释放、source 注入根和最终根集合、计数及原六成功合同。
- 现态为可独审候选，不是 #1488、#1323 或 SPEC #1340 完成，不改变 #1279 保持 OPEN。
