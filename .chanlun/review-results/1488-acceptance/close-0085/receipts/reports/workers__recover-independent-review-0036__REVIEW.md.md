# 收据 workers/recover-independent-review-0036/REVIEW.md

- 原件：`workers/recover-independent-review-0036/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`d1161e8acef8f25d8d80d585d517daca458c6737f85937041d5a5698154c9dd0`
- 字节数：21383；行数：148

## 关键行（行号: 原文）

```
1: # #1488 Recover/Carry 段独立复审（0036）
5: - 被审差异：`git diff 62bad42a1b 69e2568ea4 -- rust/ .github/`，共 6 个文件，+1042/-11，涉及 68 个提交。其中 0031 段 33 个（终点 ea363d5070），0034 段 35 个（实施 7 个，负控/revert 14 对）。
9: **总判定：PASS_WITH_FINDINGS**（P1 0 条，P2 1 条，P3 5 条）
15: ## 1. 合同各条款在生产代码中是否真实成立
19: | 1.1 | 同 owner 成功 Recover 保留资格 | PASS | 迁移分支在 `audit_owner.rs:2925-2953`，进入条件有三：`FixedOperation::Recover(h)`、本事务尚无构造、owner 持有已提交资格。同快照完整原件来自 `:2932-2935`。迁移后 strict=true（`:2983`）。COMMIT 成功后，`:1144-1149` 执行 `lease.close()` 并 `committed()`，资格戳改为 `to`（`construction_state.rs:636`）。原 01 锁与 incremental 锁
20: | 1.2 | 显式携带冻结预算 | PARTIAL（见 P2-1） | `RecoverHandoff` 已带 `BudgetStamp`（`construction_state.rs:71-98`，`native.rs:717-761`）。迁移时逐项比对 Ledger 指针、域 serial、request、allowance、total_limit（`construction_state.rs:515-520`、`native.rs:741-747`）。但摘要在 `audit_owner.rs:968-974` 取自 `active.complete`，比对对象是 `:2949` 的 `
21: | 1.3 | 原料连续证明 | PASS | `PendingRawState::recover` 在比对之后调用 `carry`（`construction_state.rs:521`）。`carry` 核验租约未撤销、`same_snapshot`、raw 前沿不退，游标/set/source/fee 原样 move，binding 不推进。L1 和 fee 锁都断言 binding 与 retained 不变。 |
22: | 1.4 | 只允许原有效 owner | PASS | 资格只取自 `self.owner.construction`；缺失时报 “Recover迁移缺原有效资格”（`construction_state.rs:509`）。资格戳若存在，必须等于本事务读到的持久 from（`:511-513`），由 stale_qualification_stamp 锁和负控 N-P36a' 咬住。公开路径 `driver.rs:446-459` 每次都新开 `AuditConnection`，不会持有资格，所以不会进入迁移。 |
23: | 1.5 | 持久 epoch 严格上升 | PASS | `RecoverHandoff::new`（`construction_state.rs:78-82`）现在是 `owner.recover` 中唯一的严格上升门（`audit_owner.rs:964-965`）：位置仍在读取 writer_epoch 之后、`S3Control::attach` 之前，文案逐字不变，错误顺序不变。 |
24: | 1.6 | effects 精确闭合 | PASS | strict 段先 `close_effects`，再执行 `confirm_recover(candidate.q.last_epoch_to())`（`audit_owner.rs:3168-3179`）。`confirm_recover` 要求三点：尚未确认、已闭合、persisted==to（`construction_state.rs:529-543`）。`check_commit` 拒绝未确认的迁移（`:616-618`）。L3 逐字断言 “未声明实际效果”。 |
25: | 1.7 | 真实 COMMIT 后才晋升 | PASS | `commit_as` 函数体未改。COMMIT 失败时 `clear_candidates`（`audit_owner.rs:1127`），成功后才 take 并晋升（`:1144-1149`）。L4 使用真实 commit_hook 中止 COMMIT。 |
26: | 1.8 | 旧 epoch/token/租约失效 | PASS | 旧租约：`lease.close` 推进 serial，旧 serial 的 raw token 实际 `bind` 被拒（L(c)，N-c1 红）。旧 Begin token：`recover()` 把 begin_token 清空，旧 token 走生产 `publish` 时逐字报 “当前 begin_token 不符”（N-c2 红）。旧 epoch：L1 末段旧 epoch 推进被拒。 |
27: | 1.9 | 外写、读域变化、rollback、提交失败、交接失败继续撤销 | PASS | 外写/读域/schema/材料域：`recover()` 先调用 `bind_existing(None)`（`audit_owner.rs:957`），`same_snapshot` 不符时走 reject_warm，invalidate 后报错。迁移失败：`before_fixed_write` 外层 invalidate（`:2900-2909`）。rollback：`AuditTxn::drop` 执行 `clear_candidates`（`:1172`）。COMMIT 失败：`:1127
29: ## 2. 禁止项与迁移触发条件
36: - **PASS。**
38: ## 3. 原锁与断言
40: - `driver.rs` 是纯插入：基线第 1..1016 行与 HEAD 第 1..1016 行逐字相同，基线 `1017..` 与 HEAD `1611..` 逐字相同。插入段 1017..1610 位于 `mod tests`（基线 `:465`）之后、01 测试结束之后。01 锁（`:852`）、公开新 Driver 冷审锁 `audit_owner_complete_normal_second_connection_and_new_life_are_cold`、incremental 锁均未动，expected 未改。
43: - **PASS。**
45: ## 4. 预算段冻结不变量
47: - CI feature job 的 locks.tsv 中，基线 62bad42a1b 已有的 54 条锁结果为 52 PASS / 2 FAIL(101)。2 条 FAIL 是 `normal_append…`（“stage缺材料cut、重入或顺序倒退”）和 `write_budget…`（“publication暂存资源已失效”），去掉数字后与 run 39332712343 逐字相同，属于 journal limit/8 帽段的已知项。P1-α、P1-1（两条）、P1-2、P3-d、P3g 等锁全部 PASS。
49: - **PASS。**
51: ## 5. 新锁是否真测到对应守卫
57: | N1 31af776d21 | 去掉 lease.close | FAILED driver.rs:1139（serial 不推进） |
58: | N2 1006c1c136 / N-P33 ac0fb78481 | 去掉 RecoverHandoff 门 | FAILED :1174（单元调用 2→2 被接受） |
59: | N3 fbc9186118 | 去掉“未声明实际效果” | FAILED :1206（Ok 被晋升） |
61: | N4 e962888642 / N4b a0f708a70a | 去掉 COMMIT 失败分支和析构的 clear | FAILED :1115 |
62: | N5 37fa770ec8 / N5b 02dbff4396 | same_snapshot 不比 version 与读域 / 只不比读域 | FAILED :1254 / :1271 |
63: | N6 0723b30443 | 去掉写前撤销和析构 clear | FAILED :1115 |
64: | N7 39bccc3252 / N7b 0159aa9c9c | 换空账 / 抹 retained | FAILED :1313 / :1322 |
65: | N-P31 fdbd11f91e / N-P31b 1926e8e88d | 去掉整段摘要比对 / 只去掉 serial 比对 | FAILED :1575（他账段）/ :1590（同账他域段） |
66: | N-P36a 954e005a0f（无效）/ N-P36a' ed7f60ed7f | 去掉资格戳门 | 后者 FAILED :1541（Ok） |
67: | N-P36b 81bcfcaef0 | confirm 不比 persisted | FAILED :1560 |
68: | N-a1 7031e5b8fc / N-a2 7d49a1a37a | 迁移丢许可 / 比对后把许可换到他账 | FAILED :1372 / :1376 |
69: | N-b1 da946a3dc4 / N-b2 82bc27fea6 | 去掉 reject_warm / 缺摘要时放行 | FAILED :1402 / :1415（原因逐字比对） |
70: | N-c1 3cf667f28b / N-c2 beb079165b | 不 close 租约 / 不清 begin_token | FAILED :1443 / :1463 |
71: | N-d1 cc1a2ec3ff / N-d2 4b7e0a4f12 | same_snapshot 不比 schema / 不比 data_version | FAILED :1488 / :1515 |
76:   1. `matches` 的生产比对是自指的，只能靠测试注入改写摘要来咬住（见 P2-1）。
77:   2. `holds(fee)` 是生产中唯一非自指的父账比对，但没有独立负控。N-a2 是在比对**之后**换账，测的是 fee 锁的事后断言，不是 `holds` 门（见 P2-1）。
79: - **PASS**（受限部分并入 P2-1、P3-3）。
81: ## 6. 业务语义、SQL/错误顺序、资源帽、Cargo/features、函数体与构建
86: - **PASS。**
88: ## 7. 测试证据
90: - CI run 39344926664：API 返回 head_sha=`69e2568ea4756f115541d624a9df457c8509d8c3`，workflow 为 `issue-1488-uo-iter.yml`，分支 `codex/1488-uo-ci-20261019`，conclusion=failure。两个 job 的 head_sha 相同：`uo (s_session)` success，`uo (s_session,s_capture_native_testonly)` failure。
91: - artifact 重新下载：12231140456（-0）的 sha256 为 69ec018d…，12230478341（-1）为 b0152fbf…，均等于 API digest。解压后与 `workers/uo-ci-runs/39344926664/` 逐文件 `diff -rq` 一致。
92: - feature job 的 locks.tsv 为 **66 PASS / 2 FAIL(101)**，68 条锁名与 `.github/1488-uo-locks.txt` 去注释后完全一致，14 条 recover_lock 全部 PASS。非 feature job 为 1 PASS / 67 NOT_LISTED。两个 build.log 都没有 error。
94: - **PASS。**
96: ## 8. 对 0032 各 finding 的处置核验
100: | P2-1(a) fee=Some 载荷 | RESOLVED | `recover_lock_fee_permit_moves_on_same_parent_ledger`（`driver.rs:1349`）断言 has_fee、同 Ledger 字节、retained/binding、live/peak；N-a1、N-a2 红。限制：fee=Some 资格由测试选择器 `issue_raw_qualification_for_test` 签发，不是生产生命周期（生产中 fee=Some 由原 incremental 锁间接覆盖）。 |
101: | P2-1(b) 无候选/无父账 | RESOLVED | `:1393` 两段逐字断言并撤销；N-b1、N-b2 红。`:2934` 分支仍无锁，但不可达（P3-3）。 |
102: | P2-1(c) 旧 raw/Begin token 实际调用 | RESOLVED | `:1422` 先做正控（restamp 后可以 bind），晋升后旧 token 实际 bind 被拒；旧 Begin token 走生产 `publish` 被逐字拒绝；N-c1、N-c2 红。旧 raw token 同样来自测试选择器加 restamp。 |
103: | P2-1(d) schema/材料域 | RESOLVED | `:1471`：同连接 CREATE TABLE 使 schema_version 真实变化；第二连接对 raw_events 两次真实提交。两处都逐字断言 reject_warm 原因；N-d1、N-d2 红。 |
104: | P3-1 预算隐式携带（主控裁定改为必修） | PARTIAL | 形式上满足裁定：有父账 id 和冻结域摘要的绑定、逐项比对、他账负控。实质上生产比对是自指的，见 P2-1。 |
112: ### 主控裁定 issuecomment-6278995984 第一条，逐条核验
114: | 裁定要求 | 状态 | 证据 |
117: | 迁移时核对绑定与候选父账一致，不一致拒绝 | 已实现，但生产中比对自指 | `construction_state.rs:515-520`；见 P2-1 |
118: | 仍过原总容量门、不重绑、不换账 | PASS | `audit_owner.rs:2996-3011` 未改；迁移不调用 handoff/bind |
123: ## Findings
125: ### P2-1 主控要求的父账绑定在生产中是自指的，唯一非自指的 `holds` 门没有负控
128: - 事实：摘要在 `recover()` 中取自 `active.complete.borrow().resources`，比对对象是 `before_fixed_write_inner` 中 `self.complete.borrow()` 的 `candidate.resources`。两者是同一事务里的同一个 `Budget` 值，中间没有任何代码修改候选或其父账。所以在生产中，`stamp.matches(parent)` 对 Ledger、serial、request、allowance、total_limit 的比较恒为真。主控裁定说“预算隐式携带不接受”，而这里的摘要并非来自独立
131: - 为何不判 P1：目前生产中没有发现能让资格与候选分属不同 Ledger 的路径（跨请求 `handoff` 保持同一个 Arc，`native.rs:823-848`；`bind_complete_budget` 校验 same_parent，`request_audit.rs:2124-2153`），所以没有现实的错账后果。裁定的字面要求（绑定、比对、他账负控）也都已做到，缺的是实质独立性。
132: - 建议：二选一，由主控裁定。
134:   - (b) 由主控明确接受“同调用内摘要、实质只核当前域可读和 fee 同账”作为裁定的满足口径，并在票上登记。无论选哪一项，都应补 `holds` 的独立负控。
136: ### P3
142: - **P3-5 迁移只在 Native feature 下存在**：非 feature 构建没有构造资格，`RecoverHandoff` 只起严格上升门的作用。这与 0032 的结论一致，属于预期，列出以便完整。
144: ## 未覆盖与限制
146: - 本机没有运行 cargo test，没有重跑 CI，也没有重跑负控。负控结论来自既有提交对象和 `/tmp/nc-remote` 日志（已核日志头 sha 与 neg 一致）。
147: - 0031 的负控（N1–N7b）是在 ea363d5070 之前的代码上做的。0034 改动了 `owner.recover` 门和迁移入口，此后没有对 N1、N4–N7 重做负控。本轮核对了这些守卫的代码在 0034 中未被改动（`commit_as`、`Drop`、`same_snapshot`、注入点），因此推断原负控结论仍适用。
148: - 没有覆盖 journal limit/8 帽段、receipt key 合并和 originals 增量许可（裁定第二、三条属于另一小段）。#1488 仍 blocked_by #1394，本评审不解除任何门。
```
