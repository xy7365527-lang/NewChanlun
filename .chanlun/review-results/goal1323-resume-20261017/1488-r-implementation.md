# #1488 R 实装中间报告（2026-10-17）

状态：R_BASIC_INTERFACE_READY_FOR_REVIEW / NOT_COMPILED / 不可验收。不是旧七件设计恢复。

工位 `.sandcastle/worktrees/resume-1488` @c2c9c5640a。R 只写 audit_owner.rs、audit_owner/effects.rs、construction_set.rs；S 在共享树写另外五文件。三个禁区未改。本机 cargo/服务/外盘/push 未执行；rustfmt 仅语法解析成功，不算类型或测试成功。

## 已实际接线

- PublicationResources 从 CompleteCandidate 保存的既有 Budget clone，candidate 最后字段保存 budget，不从 charge.total 重建账。
- publication runner 在 bind_existing 后、raw advance 前开启；raw、Runtime、effects 全投影验证借同一个 WorkScope。非publication保留旧独立scope与业务/资格流程，validate_effects同样继承candidate同账。
- Recording父账附接与publication身份附接分开，各一次；publication身份用Weak，不妨碍Recording.finish的Arc::try_unwrap。
- journal局部cap与父limit分开，stage读余量取局部剩余与父剩余最小值；局部peak不再拿全父peak相加。
- MaterialAccess单独material_issued，源/材料原guard保留；SourceMaterial始终只从实际SQL读取。
- StageGuard row flush在释放journal mutex后执行，随后回读原件并走原判据。
- ConstructionRun含before/after seal，仅尾检成功后构造，真实提交仍走原commit_publication→commit_as/check_commit。
- PendingSet携同Budget保留句柄，不携journal强引用；construction_state无需改。

## R费用迁移表（执行依据，不可用统计差额释放）

| 载荷 | 原收费/归属 | 当前处理 | 仍需完成 |
|---|---|---|---|
| cold candidate、audited、capture、originals、q | CompleteCandidate::capture 的原parent及Permit::commit；无parent分支一次Budget::new | 在Self最后字段resources保存同一Budget；跨Ended/owner/active/pending按值移动 | 确认每个冷源Plain历史分配是否全已入该parent，禁止二次按charge.total reserve |
| slot.result/binding/source_pending | 原冷审计/slot账，charge中部分是保守统计且可能Rc共享 | detach_slot真实drop仍先执行；暂不按released统计猜budget.release | 必须分类原费用与共享费，给每个真实唯一载荷许可；统计released不能作释放金额 |
| RawPrefixIndex、winner edits、committed/pending raw | NativeAdmission通过audit_store.native_*调用reserve_native(...).retain | 同一publication scope真实准入；父账连续保留 | retain累计必须修，见下方最小类型面；不能在run返回前一把清掉 |
| journal初始、keys/nodes容器 | Recording::start已创建；attach_parent只对j.bytes承接一次；后续reserve_one增量permit | shared_parent + journal_owned_bytes只记本journal实际已commit费用；扩容新旧共存 | 失败路径审查（退休后不可复用该有污染父账） |
| stage receipts Vec、consumed索引、stage实际行副本 | budget.vec_grow/reserve_consumed/read_permit.commit | 将实际容量增量、实际行heap归journal_owned_bytes；StageOriginals先drop头部再释放头部费 | 不与source set等scope收费混为journal_owned_bytes |
| source/set/capture codec载荷 | 原scope保留容器、NativeAdmission、NativeReservation | PendingSet末字段保留原Budget；不另造父账；不提前释放 | scope内Plain临时与跨代持久费用分类仍需容器许可改造 |
| Recording.finish changes Vec/ExactRows | 原只有cap统计，后续新Budget隐式承接 | 现同parent先reserve变化Vec；逐行以min(local,parent)预留，实际row move，头部drop再release | Effects.apply真正丢旧行、Node row body转capture后的许可连续交接需独审 |
| journal完结 | keys迭代已drop，nodes/stages/consumed显式drop | 仅释放journal_owned_bytes精确记录；set独立句柄继续持账 | 不释放完整j.bytes或budget.live差额 |
| validate_effects/cut更新 | 旧实现Budget::new + 两次codec_work | publication都借同scope；非publication同Budget开旧独立scope | 所有临时row_ids/变化载荷必须实际drop再解对应费 |
| 下一candidate | 原统计charge更新 | 同Budget move回candidate，任何guard失败保持拒绝/回滚 | 六原成功锁必须实跑；16/8 MiB不改 |

## retain累计的确定缺口与最小可分派类型面

不能仅改audit_store.rs解决：当前API只收裸容器/返回裸类型，返回时已丢失permit，helper无法观察调用方何时drop。把retain改drop会给活载荷提前释放额度。下列是已核真实调用点，非猜测：

| audit_store入口 | 实际调用/载荷 | 必要改造 |
|---|---|---|
| native_vec_room(:55-83) | business_json.rs:558/563 removed/added（本次临时）；object_index.rs:859/863 active/dead（计划临时） | 临时Vec改NativeOwned<Vec<T>>或ScopeVec；新块许可覆盖旧新共存，成功换块后释放旧块许可，随owner move |
| native_codec_room(:85-124) | business_json.rs:538 RawPrefixChanges.winner_edits；construction_state.rs:299 move到Issued/BoundRawAdvance，其字段:71/84是CodecVec | 包装返回类型会碰禁区。可在可改上层owner中保持winner_edits外部许可到整个Issued/BoundRawAdvance确实析构；需明确token move轨迹，不能只在RawPrefixChanges上挂字段后把Vec单独取走 |
| native_tree_room(:180+) | business_json.rs:573 RawPrefixIndex.ordered持久；source_contributions.rs:111/214 Map/Tree持久；aux_index.rs:1230/1958索引；object_index.rs:1932/1956 | 将许可放实际持久容器/树节点所有者；共享Rc必须共享许可，最后一个节点owner释放；不能按每次调用独立临时guard |
| native_std_map_room/native_map_room(:150-177) | source/object/auxiliary持久map与临时map混用 | 在具体map类型加permit；grow后释放旧真实块费用，不能仅凭len下降放树/哈希容量 |
| native_admit(:41-47) | source_contributions.rs:1055 Rc<Object>；business_json.rs:518 identity字符串；object_index.rs:1769等ID；aux_index.rs:1192/1195引用克隆 | 引用/字符串/对象需带许可的owning类型；Rc节点的permit随最后强引用释放 |
| native_string/native_format(:233-292) | 返回String，可能进入持久ID、RefValue、PathStep临时值 | 单改返回类型影响调用方字段；优先增加新NativeOwnedText并明确适配，禁止隐式into_inner丢许可 |
| native_plain_ref/native_built_ref(:199-230) | 返回ContentRef/RefValue::Plain，其中两String | RefValue的Plain变体或外层持有者携许可；Owned分支不能混账 |
| native_verify_original Current.read(:325-333) | 临时NodeRead body | scoped主链已有ScopedFrame许可；非scopedNodeRead裸返回不能提前释放，需要改调用方或保守保留（仍欠） |

最少需独立处理business_json.rs、source_contributions.rs、object_index.rs、aux_index.rs及audit_store.rs，可能触及定义Plain ContentRef/CodecVec的库接口；不是三文件或单audit_store修复。三个禁区持续有效。R已经向主控发送具体调用/归属用于拆分，不把此缺口延后到六测失败才承认。

## 验证与后续

已新增真实journal Weak/独立局部cap/父scope身份/尾检拒提交锁（下列四个）。所有Rust测试当前NOT_RUN。完整Owned/Native验收、#1491/#1481、S86/MIN12、正式入口、完整历史与双跑、64GiB/store及148GiB外盘前置不减。#1488/#1323/#1279不关闭。


## 交独审增补（本次续接）

- audit_owner.rs:2034：shared_parent 的 recording_peak 明确取继承的 resources.peak，不再叠加 candidate 统计；用 Budget 的 `peak > limit` 越界口径，因为同父账预留完整 remaining 可以恰抵 limit。非共享路径保留原 `retained + effects.peak >= limit`。journal 的局部帽和逐次预留条件原样保留。
- effects.rs:1238/:1608/:1894：三个 shared_parent 分支仅记录 journal 局部 j.bytes，非共享夹具仍记独立 budget.peak；独审已复读确认不再混峰。
- effects.rs:750：旧 SQL 行真实 drop 后才 release(old_payload)，新行按值移交 originals；余下 changes 真实 drop 后才释放各剩余实际容量。
- audit_owner.rs:2088 附近：重复 digest 的 SQL body 已由 append 实际丢弃才 release(body.capacity)；首次 body 移交 capture 不释放。
- audit_owner.rs:2232：append_cut 的 objects/auxiliary 临时行号 Vec 按各实际 capacity 合计，真实 drop 后 release；不按全 budget live 差额释放。
- construction_set.rs:272/:330：PendingSet 最后字段保存同一 Budget，不保存 journal Arc，禁区 close_set 的原按值移交可直接携带。

新增测试源码（全部 NOT_RUN）：

1. effects.rs:2178 `publication_weak_identity_does_not_block_finish_with_large_parent_retention`：实际 recording，8MiB 父帽、5MiB 旧保留（大于父帽一半）、64KiB journal 子帽；真实 source cut/finish 成功正控，Weak token 不阻止 unwrap；局部峰小于1MiB且完整父峰不越8MiB。
2. effects.rs:2236 `publication_resource_attachment_is_once_and_foreign_scope_rejects`：foreign scope 拒绝、重复附接 poison、后续真实 finish 拒绝。
3. audit_owner.rs:4163 `publication_single_parent_scope_reaches_raw_runtime_material_and_validation`：真实 fixture Publish 同 work 地址 raw/runtime/material/validation/tail 各一次，真实 COMMIT 一次、generation 2。
4. audit_owner.rs:4167 `publication_resource_tail_failure_prevents_actual_sql_commit`：仅在真实验证成功后注入 Ledger 尾检故障，强制核故障实际消费，真实 COMMIT 计数0、generation 1、lease撤销。不会把提前失败冒充负控通过。

执行证据：三文件 rustfmt `--edition 2021 --config skip_children=true` exit 0；三文件 `git diff --check` exit 0。未运行 cargo、服务、外盘访问、push。未修改三个禁区或 S 五文件。上表广域裸容器 retain 的许可迁移仍未完成，由主控另分工；本报告不能作为 #1488 全票通过或六原成功锁通过的证据。
