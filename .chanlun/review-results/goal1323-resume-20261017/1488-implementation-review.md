# #1488 R/S 实装候选静态独审

日期：2026-10-17。范围：恢复树 `resume-1488`，基线 `c2c9c5640a99b98299456fd8b837c58035a2f4a6`；S 五文件及 R 跨接口接线。R 在审查期间继续修改三个授权文件，本报告已复读并标明期间修正。源码只读；未运行 cargo、服务、外盘、六测试或编译探针。

**结论：整体 FAIL / 不可验收。S 的五文件在本次静态核查中没有发现置信度超过 80% 的新编译错误或业务判据冲突；持续父账与 Plain 载荷的释放仍不闭合，不能据此判整个 R/S PASS。** 已知 retain 累积由主控先行登记，本报告不冒充新发现。

## 概念层质询

核对 #1488 评论 `6199095051` 的三 cut、资源/材料分别一次签发、同父 scope、尾检先于 COMMIT、实际根及连续许可要求。未改缠论概念。沿用本轮接口审查已做的谱系关键词核查，没有新的概念裁定可以豁免费用释放。原三项设计矛盾已在代码结构上修正，但完整费用同域契约尚未通过。

| 原设计缺陷 | 本次实际代码 | 判断 |
|---|---|---|
| runner 强持 journal Arc，阻止 try_unwrap | `audit_owner.rs` 的 PublicationResources 不持 journal；`effects.rs::PublicationStageIdentity` 只持 Weak；MaterialAccess 闭包后显式 drop；stage 随 Runtime 调用返回析构；之后 finish | 原必失败机制已消除 |
| journal 局部 cap 被当作全局父帽 | candidate 保留既有 Budget；PublicationResources clone 同账；StageGuard 用 `min(j.cap-j.bytes, parent_limit-budget.live())`；Recording 使用共享父账 | 已拆开两个额度维度；费用项目归属仍需后续闭合 |
| raw advance 提前开第二个父域 | `AuditConnection::publish` 唯一 work.run 包住 publish_body；后者调用 `with_bound_raw_advance_in_publication`，经相同 work 到 PendingRawState::advance；validate_effects 同 scope | publication 正常路径的第二个 raw scope 已消除；旧 helper 留给非该路径 |

before/after seal 已分开：`check_publication_transition` 检查唯一 write_sequence 推进；只有 work.run 成功返回后才创建 ConstructionRun，再核验、调用真实 commit。未看到把 receipt 在 SQL commit 前交给调用者的新增路径。

## 审查期间发现并已修正的问题

曾读到 `audit_native_source`、`note_source_cut`、StageGuard 把**全父账** `budget.peak()` 填进 `j.peak`，而 `validate_effects` 又计算 `candidate.charge.total + effects.peak`。反例是旧 candidate 大于总帽一半、实际全工作集仍小于总帽：会被重复加费而拒绝。已立即通知主控和 R。

随后复读 `effects.rs` 三处确认：shared_parent 分支只记录局部 `j.bytes`，旧独立账分支才使用局部 budget.peak；Recording 再计算局部读行峰值。因此这个具体“候选 + 全父峰值”误算机制已消除，**不再列为当前开放缺陷**。仍需在最终源码指纹上验证原成功锁，静态修正不代表其成功。

## S 五文件核查结果

### 借用、复制与析构

- `StageOwner<'w>` 不借 Transaction，仅共享借 work 和持 Weak 身份。TxStore/NativeReadLease 的 StageOwner 生命周期使用共享引用，可缩短借用；没有仅凭同名 lifetime 就成立的静态 borrow 错误。
- prepared 中生成用 NativeReadLease 位于独立块，块返回 generated/objects/axes 后 lease 已析构；随后才运行根注入、source flush 和 MaterialAccess。未持独占 tx 借用进入 SQL flush。
- `ChargedNode` 正文先于正文许可；StageOwner 的 nodes 先于 tree_permit；`CutSeen` 集合先于许可。复制 ScopedFrame 另取正文许可，不转借暂存原件的许可。
- 选择性 flush 成功后才 remove 暂存条目；drop ChargedNode 才释放正文。BTree 使用高水位，不把 len 减少冒充物理容量释放。
- 现有非 scoped 返回值走 `ScopedFrame::into_frame` 的 retain 语义仍在，这属于下节持续父账问题，不在此处冒称已 RAII 闭合。

### SQL 选择性与材料证据

- `staged_nodes.rs::flush_cut` 从显式根遍历闭包；SQL 老父指向新暂存子时不会提前截断。tag/长度/子边检查与旧 flush 同族。
- 第一阶段校验 SQL + overlay；第二阶段通过**无 stage** 的 TxStore 写 SQL，保留 note_persisted_node 收据。没有把 overlay 当真实 SQL 来源交给 source/row 审计。
- SQL/暂存同摘要异原件拒绝；新 INSERT 数由 total_changes 差与 changes 核验，未按 put 成功次数计新增。duplicate cut 对已经一致落库的节点不重记 INSERT。
- `StageGuard::finish` 预读唯一实际非 Node 行时短持 journal 锁，随后释放锁、取得固定根数组并 flush，再重新进入原完整审计。因此新 cut 没有在 journal mutex 内回调 note_node。
- source cut 使用 `before_native_source` 注入后的 generated_state；root 同时传给原 source audit。
- final 根含 manifest/resolved snapshot/delta/evidence/generated_state/templates/scope/runtime；workspace 分支先落本代模板闭包再调用原 publication workspace，workspace 未被丢弃。
- strict scoped 路径使用选择性 cut；原无 scope 诊断 stage 保留旧全表 flush。未把后者接成 strict 失败后的 fallback。

### 原有资格门

NativeReadLease 的活动事务、read_uncommitted=0、format、schema/insert-preserve 和 poison 检查保留。SameMaterial 只缓存真实 SQL；overlay 复制走 scoped read。scoped staged 必须有 PublicationScope；非 owner 的 stage+work 缺身份仍拒绝。原六测试文件及 16/8 MiB 常量未列入改动面。

未发现“可直接指认一定编译不通过”的新增符号或借用错误；未编译意味着这一项仍待验证，不能把 rustfmt 当 borrow-check 结果。

## 已知持续账释放问题的必要修复范围

`audit_store.rs::CodecRead::native_admit` 把 reserve 后的 NativeReservation retain；native_vec_room/native_codec_room 的每次扩容也 retain 新块，旧块不随真实 reallocation 退账。native_string/native_ref 等返回普通 String/ContentRef/Vec，载荷 drop 时没有释放句柄。以前短命 Budget 的生命周期掩盖了这一差别；现在 CompleteCandidate 持同 Budget 跨多个事务，已死临时对象的费用也跨代累积。

确定的静态反例：在同一持久 parent 上重复执行“借 CodecRead 创建非空 Plain String → drop String”，实际存活量回到基线，但账 live 每轮增加；有限帽下必然拒绝。该反例证明问题存在，**不能直接推断六个有限 fixture 全部第一轮必红**；没有执行结果或完整容量读数，不能填这样的结论。

必要修复不限于 audit_store 一个文件：

1. `audit_store.rs`：资源许可应随返回的 Plain 载荷持有；扩容要先保新旧共存峰值，再在旧块真实释放后归还旧费。不能用总 live 快照差作批量释放。
2. `business_json.rs` 及实际持有 TextValue/JsonOwner/RefValue/RefFields/数组的上层 owner：把 Plain 内容与 RAII 许可一起 move，保留真实所有者，先载荷后许可析构。应按调用链点名最小类型面，不能只在入口改 retain。
3. `content_store.rs` 的 `NativeReadLease::current_checked` / `TxStore::checked_read` 会调用 `ScopedFrame::into_frame`；主链中能 scoped 的读取应改成 scoped，确需 Plain 兼容结果的调用者必须接收有许可的聚合所有者。只修 audit_store 的字符串不能清掉这条费用。
4. 原 candidate/raw/source/set/effects 的持久保留费只能由实际 owning 容器收取/释放。R 新增同 Budget 句柄证明账身份，不自动证明每个费用项目可回收。`PendingSet` 只持 Budget clone 也不等于 set drop 会 release 费用；禁区 PendingRawState 可继续按值持 PendingSet，由非禁区聚合 owner 实现释放。

三个禁区仍可不改：原 NativeReservation 已能随聚合值析构，无需改 scratch.retain 意义或加 Ledger getter；construction_state 接值的合同可承接带许可的新聚合值。若某个上层 Plain 类型目前没有空间携费，应明确扩大那个实际 owner 的文件面，不能引入全局 side table/清账数字绕过。

尚需检查 cold parent 持久化后冷审计期间临时项目是否已退，以及当前预算合同变化时的原成功行为；不能把所有 changed-limit 直接拒绝当作费用修复。当前尚未证明该条件导致原六成功锁拒绝。

## 验证与源指纹

本次运行 `git diff --check` 通过。未执行其他构建/测试；S 报告的 rustfmt 通过未重复。三个禁区不在当前 diff 中。所有结论只覆盖以下已读源码，R 后续费用修改需按新 diff 再审。

S 五文件 SHA256：

| 文件尾部 | SHA256 |
|---|---|
| v2/staged_nodes.rs | b5debf822b6fcba5806ec417224ce3465a02cac8fa098660918fc6af1ceb3d6d |
| v2/content_store.rs | bde21fae685835762c43ab064c3870325a1474433241ed476712e30c4ff26cc1 |
| v2/content_publication.rs | 40989a45f3379d8d7cbd57379ea6b9a6ed31ea693cc90dc63ac0afec292efeee |
| v3/prepared.rs | d045a9d0c4b92aac6f11a47e12738c4961340f1cc31ca484c4d912e797dacb62 |
| v3/commit.rs | 5030fc3393f6c052fa23d41d6367feb90aa1657c65f632708ead75da931264d3 |

## 工程层结论

整体概念层的“许可随载荷连续持有并在真实销毁后释放”仍未闭合，按审查纪律不给工程 PASS 表。R/S 结构修复可保留并继续 RAII 施工，不能提交为最终验收通过。下一轮应锁最终 source hash，核费用迁移后再取得原六成功测试的实际结果；#1488、#1323、SPEC #1340 和 #1279 的完整交付门均不改变。
