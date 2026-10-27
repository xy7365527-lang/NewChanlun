# 收据 workers/recover-independent-review-0032/REVIEW.md

- 原件：`workers/recover-independent-review-0032/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`ce5db33b3f3f1555341694045a8ba0099d19d57e19351b8b13bcae5544dc775c`
- 字节数：15694；行数：104

## 关键行（行号: 原文）

```
1: # #1488 Recover/Carry 段独立评审（0032）
5: - 被审差异：`git diff 62bad42a1b ea363d5070 -- rust/ .github/`，共 5 个文件，+523/-9，涉及 33 个提交（7 个实施/测试提交，13 对负控提交与 revert）。
9: **总判定：PASS_WITH_FINDINGS**（P1 0 条，P2 1 条，P3 7 条）
15: ## 1. 合同逐条在生产代码中是否成立
19: | 1.1 | 同 owner 成功 Recover 保留资格 | PASS | `audit_owner.rs:2808-2835` 是迁移分支：本事务没有构造、owner 持有已提交资格、存在同快照完整原件，三者同时满足才调用 `PendingRawState::recover`，然后 strict=true。COMMIT 成功后，`audit_owner.rs:1119-1125` 执行 `lease.close` 与 `committed()`，资格戳改为 `to`（`construction_state.rs:584-587`）。原 01 锁和 incremental 锁在 CI 中
20: | 1.2 | 显式携带冻结预算 | PASS（含 P3-1） | 迁移不重绑、不换账：候选仍持有原 `candidate.resources`，`limit = candidate.limit.min(remaining)`（`:2873-2876`）。保留费合计由 `total_limit` 门核验（`:2879-2894`）。L7 用 `complete_on_ledger_for_test(mark)` 断言前后是同一父账。N7（换成空账）会被下游的“原件未由本父账许可出生”拒绝。Recover 的生产路径本身没有 RequestAuditContext，因此“本请求冻结预算”的实际所
21: | 1.3 | 原料连续证明 | PASS | `PendingRawState::recover` 复用 `carry`（`construction_state.rs:458-475`）。`carry` 会核验租约未撤销、`same_snapshot`、raw 前沿不退，然后把游标、set、source、fee 原样 move，binding 不推进。L1 断言 `binding` 不变。 |
22: | 1.4 | 只允许原有效 owner | PASS | 只从 `self.owner.construction` 取资格。缺资格时不进入迁移，走原非 strict 路径。资格戳不为 None 时必须等于 from（`construction_state.rs:468-470`）。外写或换连接时，`bind_existing` 的 `reject_warm` 会先撤销（`audit_owner.rs:2719-2742`）。公开路径 `driver.rs:446-459` 每次新开 `AuditConnection`，不会持有资格，所以不会进入迁移。 |
23: | 1.5 | 持久 epoch 严格上升 | PASS | 原门 `audit_owner.rs:957-960` 未改，文案和错误顺序也未变。新增的 `RecoverHandoff::new` 门（`construction_state.rs:74-79`）在生产中排在原门之后，属于冗余纵深，见 P3-3。 |
24: | 1.6 | effects 精确闭合 | PASS | 迁移后 strict=true：缺 recording 时撤销并报错（`:2939-2944`）；`has_exact_coverage` 生效；`close_effects` 之后执行 `confirm_recover(Q.last_epoch_to())`（`:3050-3054`）。Q 的 `epoch_to` 只在本事务实际 Epoch 行通过 `validate_epoch_step` 后才更新，并且与 meta writer_epoch 一致（`audit_owner/q.rs:380-391`）。`check_commi
25: | 1.7 | 真实 COMMIT 后才晋升 | PASS | `commit_as` 函数体未改。COMMIT 失败时执行 `clear_candidates`（`:1102-1103`）；COMMIT 成功后才 `take` 并晋升（`:1119-1125`）。 |
26: | 1.8 | 旧 epoch/token/租约失效 | PASS | 旧租约：`lease.close` 推进 serial，`BoundRawAdvance::ensure_current` 依赖 serial 相等。旧 Begin token：`recover()` 在 prior≠idle 时清空 begin_token（`:967-975`）。旧 epoch 请求：原有 `verify_writer_epoch`（`audit_owner.rs:405/445/463/495/603`）。L1 覆盖 serial 推进、begin_token 清空和旧 epoch 推进被拒。 |
27: | 1.9 | 外写、读域变化、rollback、提交失败、交接失败继续撤销 | PASS | 外写/读域：`bind_existing` 调用 `same_snapshot`，失败则走 `reject_warm` 并 invalidate。迁移失败：外层 `before_fixed_write` 执行 invalidate（`:2783-2793`）。rollback：`AuditTxn::drop` 执行 `clear_candidates`（`:1148`）。提交失败：`:1103`。交接失败：注入点在原资格已离开 owner、尚未进入事务之后（`:2818-2823`）。没有发现可绕过
29: ## 2. 禁止项与迁移触发条件
31: - `rg` 检查了全仓 `stale.set(false)`，只命中 `Lease::default` 的初值（`construction_state.rs:45`，既有代码）。diff 中没有新建 Budget 或 Ledger，没有 `state::audit` / `audit_with_budget` 兜底，也没有在 Accept/Begin 分支加入 Recover。**PASS。**
33: - 外部写入之后也无法误触发：`recover()` 先调用 `bind_existing(None)`，data_version、schema、读域变化都会命中 `reject_warm`，资格被撤销，迁移分支的前提（owner 有资格）随之消失。L5 的三段测试与 N5/N5b 负控印证了这一点。**PASS。**
36: ## 3. 原锁与断言
38: - `driver.rs` 是纯插入：前 1013 行与基线 sha256 相同（7075d00b…），尾段 `1326..` 与基线 `1014..` 的 sha256 也相同（e3904e10…）。01 锁（`:852`）、公开新 Driver 冷审锁 `audit_owner_complete_normal_second_connection_and_new_life_are_cold`（`:1662`）、incremental 锁（`:4213`）均逐字未动。
39: - 全 diff 只删了 9 行，都在 `audit_owner.rs` 生产代码，分别是：比较改写为先取 from、`FixedOperation::Recover` 带参、close_effects 调用的改写。没有删除或修改任何 assert。`.github/1488-uo-locks.txt` 只在末尾追加。**PASS。**
41: ## 4. 预算段冻结不变量
43: - CI feature job 的锁表中，基线已有的 54 条锁结果为 52 PASS / 2 FAIL(101)。这 2 条是 `normal_append…`（“stage缺材料cut、重入或顺序倒退”）和 `write_budget…`（“publication暂存资源已失效”），消息与基线 run `20261020-043658-62bad42a1b` 逐字相同，属于 journal limit/8 帽段的已知项。P1-1/P1-2/P1-α/P3b/P3-d/P3g 等锁全部 PASS。
44: - 迁移没有改动 `bind_existing` 的预算交接、`issue_publication_resources`、request_audit.rs、native.rs、effects.rs。**PASS。**
46: ## 5. 新锁是否真测到对应守卫
48: 13 对负控提交都满足：`revert^ == neg`，且 `git diff neg^ revert` 为空。各条负控在 `/tmp/nc-remote/` 中的日志都只跑了目标锁，并且结果为 FAILED：
52: | L1 晋升与旧 epoch/token/lease | N1 去掉 `lease.close`（053559-31af776d21）FAILED | PASS |
53: | L2 epoch 不升 | N2 去掉 `RecoverHandoff` 门（053728-1006c1c136）FAILED | PASS，但只由单元调用 `recover_handoff_accepts_for_test(2,2)` 咬住；driver 段由原门拦截（P3-3） |
54: | L3 effects 不闭合 | N3 去掉“未声明实际效果”（054646-fbc9186118）FAILED，`Ok` 被晋升 | PASS，含 P3-4 |
55: | L4 rollback/COMMIT 失败 | N4（054829-e962888642）、N4b（055006-a0f708a70a）均 FAILED | PASS |
56: | L5 外写/读域 | N5（055152-37fa770ec8）、N5b（055335-02dbff4396）均 FAILED | PASS |
57: | L6 交接失败 | N6 去掉写前撤销和析构 clear（055525-0723b30443）FAILED | PASS，但两处是一起去掉的，单独去掉任一处是否会红未单测 |
58: | L7 预算 live/peak | N7 换空账（055656-39bccc3252）、N7b 抹 retained（055839-0159aa9c9c）均 FAILED | PASS |
63: ## 6. 业务语义、资源帽、构建
66: - 远端 `cargo check --locked --bin s_structure_session` 的结果：`--features s_session` RC=0；`--features s_session,s_capture_native_testonly` RC=0。新增 2 条非 feature dead_code 警告，见 P3-7。**PASS。**
69: ## 7. 测试证据
71: - CI run 39332712343：API 返回的 head_sha 为 `ea363d50705b0cc8a99a170a0d6cc48e26e4c6e7`，workflow 为 `issue-1488-uo-iter.yml`，conclusion 为 failure。两个 job 的 head_sha 相同：`uo (s_session)` success，`uo (s_session,s_capture_native_testonly)` failure。
72: - 两个 artifact 重新下载后，sha256 分别为 a16a8686…（id 12225287045，-1）和 c8d9283e…（id 12225167343，-0），与 API digest 一致；解压结果与 `workers/uo-ci-runs/39332712343/` 逐文件 `diff -rq` 一致。
73: - feature job 的 locks.tsv 为 **59 PASS / 2 FAIL**；非 feature job 为 1 PASS / 60 NOT_LISTED，两个 job 的 build.log 都没有 error。7 条 recover_lock、accept_tail、incremental 全部 PASS。**PASS。**
74: - 宽对拍：`20261020-043807-62bad42a1b`（209/20）对比 `20261020-060024-ea363d5070`（218/18），两次的命令和过滤器相同。逐测差异只有 2 条目标锁由 FAILED 变为 ok，以及新增 7 锁为 ok；没有新增失败。panic 消息去掉数字后，差异仅为少了 2 条“Carry租约已撤销”。**PASS。**
78: ## Findings
80: ### P2-1 合同点名的新增回归没有全部单独覆盖
89: 不判 P1 的原因：本评审核对了这些行为的生产守卫，都存在且路径成立（第 1、2 节）。缺口在锁，不在实现。建议在整段验收前补齐，或由根明确接受现有覆盖。
91: ### P3
101: ## 未覆盖或限制
103: - 本机没有运行 cargo test，也没有重跑 CI，测试结论来自已有日志和 artifact。没有运行 schema 变化、fee=Some 场景的探针（这些是 P2-1 的内容，属于作者应补的锁，评审不代写）。
```
