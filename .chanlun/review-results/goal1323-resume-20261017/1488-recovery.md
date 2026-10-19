# #1488 只读恢复记录（2026-10-17）

已读取根 CLAUDE.md 与 Serena initial_instructions。代码图 NewChanlun 无索引，现有索引未列 resume-1488，故按实际恢复树检索。未改源码、未运行 cargo、未访问外盘、未委派。

## 当前精确基线

工作树 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1488`，HEAD `c2c9c5640a99b98299456fd8b837c58035a2f4a6`，本次 `git status --short` 为空。

主控提供远端 run 38785485944 此 HEAD 为 completed/failure；本代理未重查远端。下面是恢复清单和源码事实，不是测试通过证据。

## 准确六项与命令

正本：`.github/scripts/run-1488-driver-ci.py:25-32`。

1. `v3::driver::tests::audit_owner_complete_accept_tail_begin_recover_retry_and_revision_qualification`
2. `v3::driver::tests::audit_owner_complete_normal_second_connection_and_new_life_are_cold`
3. `v3::driver::tests::audit_owner_incremental_normal_writes_full_oracle_and_suffix`
4. `v3::driver::tests::audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate`
5. `v3::driver::tests::audit_owner_write_budget_releases_candidate_after_successful_normal_append`
6. `v3::driver::tests::audit_owner_staged_normal_append_consumes_actual_nodes_and_suffix`

runner 第 548 行构建命令（cwd=rust）：

```sh
cargo fetch --locked
cargo test --offline --locked -j 6 --features s_session,s_capture_native_testonly --no-run --message-format=json --bin s_structure_session
```

通过 Cargo JSON 选择并绑定唯一测试 executable，先执行 `<binary> --list --format pretty`，验证六名各出现一次，再逐名执行：

```sh
<binary> <上列完整测试名> --exact --format pretty --color never --show-output
```

正式入口是 `.github/workflows/issue-1488-driver-six.yml:32` 的 `python3 -B .github/scripts/run-1488-driver-ci.py`。workflow 为 ubuntu-24.04，push 分支 `validate/1488-driver-six`，整 job 45 分钟、runner 步骤 40 分钟。不能用包含 normal 的模糊过滤替代六项；旧两项 normal/idle 测试并非此清单。

runner SOURCE_FILES/REVIEWED_SOURCE 绑定 `9160fc9deec9bb8870c6e757583424b272cf3d69`；新源码改变后，必须重新审阅并更新绑定，不能原样跑旧 runner 再称新候选验收。runner 的磁盘前置为 2 GiB，属于此专用远端六项；不能混同完整历史双跑的外盘至少 148 GiB 前置，后者不因本清单改变。

## 七件冻结设计恢复结果

精确名：LEASE-OWNERSHIP.md、RESOURCE-ISSUANCE.md、HANDOFF-FORMULA.md、LIFETIME-GUARANTEES.md、REVISED-INTERFACES.md、MIGRATION-PLAN.md、AVOIDANCE-PROOF.md。

已查本地所有 git refs 的路径历史（`git log --all -- <七文件通配>`），无命中；主仓 tracked 路径无命中；已存在 `.chanlun/review-results`、`.sandcastle`、docs、analysis 下 Markdown 文件名无命中。仅能说明这些搜索范围未恢复，不能推断历史设计从未存在。未扫描外盘。原 `/private/tmp/goal1323-native-20261012/recovery-20261015/1488-lease-redesign-v1/` 已丢失为主控交接事实。

## 最新票评论能恢复的合同

来源 `/private/tmp/goal1323-resume-20261017/1488.json` 最末评论：
https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6199095051

评论时间 2026-10-15T18:15:06Z。它记载 StageOwner B 方案限定独审通过，但明确源码尚未实施；不能视作完整精确接口：

- C1：source / row / final 选择性真实 SQL 落库。
- C2：资源能力与材料能力分别一次签发。
- C3：贯通同父 scope，不另建替代 scope。
- CodecWork.run 尾检成功后才 COMMIT。
- 许可随载荷连续持有，不能提前释放、重建账目掩盖真实峰值。
- source 使用注入之后的实际根。
- 保留 warm-staged 原成功语义、后缀/节点 oracle、16/8 MiB 原预算、所有 guard；不得将成功改拒绝或旧核兜底。

完整 Owned、Native 父账/Runtime、主库/WAL身份夹持、#1491/#1481规模、S86/MIN12/正式入口及完整历史/双跑仍欠。旧234专项不替代本六项，局部完成不关闭 #1488/#1323/#1279。

## 本树实际接口与缺口

- `audit_owner/effects.rs:1169 with_material_access` 同时创建 resources 和材料访问。入口在 :1186 检 `j.resources.is_some()`、set/stages/事务条件，:1199 拒绝重复材料签发；当前资源与材料尚未拆开。:1268 按 original_work 借用或由 budget.codec_work().run 创建，:1284 为共享 codec 工作域失效拒绝。
- `audit_owner.rs:2342 with_construction_material` 仅转接 effects 接口。owner commit 路径在 :714/:820；运行期尾检与 COMMIT 顺序须按实际调用链设计和审阅，不能单删 guard。
- `prepared.rs:329` 明确拒绝 staged + work，:336 StageOwner::new(&tx) 未接受 scope；:564 stage/native_read_lease 拒绝为现存保护。
- StageOwner 实际定义在 **`rust/src/bin/s_session_v2/staged_nodes.rs`**，被 v3 使用。:21 new(tx)、:36 check_transaction、:54 put_new、:84 flush(tx,runtime)。当前只有最终 flush，报告明确 strict heap、whole Job、max store 未验证；不是完整选择性 source/row/final 实现。

仍缺需重新冻结的精确接口：资源签发 token 与材料签发 token 的拥有者/借用关系；resource lifetime 跨 source/generate/row/final 的移动和 drop 顺序；StageOwner 的阶段 root 集合、flush 幂等性及实际 SQL/journal epoch 交互；父 work 尾检返回后交给 COMMIT 的不可伪造完成资格；注入后 root 与 source_binding 的绑定时点。票评论无法逐字重建七份原设计，不能冒称恢复原接口。

## 下一步最小职责分割（提议，尚未修改）

R 负责人：`rust/src/bin/s_session_v3/audit_owner/effects.rs`，必要的 `audit_owner.rs` 转接；拆分一次资源发行与一次材料发行，保持同父账、许可连续持有、失败污染不可洗白。先冻结接口再给 S，避免两方猜类型。

S 负责人：`rust/src/bin/s_session_v2/staged_nodes.rs` 与 `rust/src/bin/s_session_v3/prepared.rs`，必要时 `commit.rs`；接同父 scope、source/row/final 可达闭包落库及实际来源根，完成尾检/提交时序接线。上述共用 v2 文件须明确影响面，不能将改动当 v3 私有。

验证负责人：`driver.rs` 已有六成功合同保持；增加真实失败注入和资源生命周期断言只能补强，不能重写成拒绝即通过。候选封冻/独审后再更新 runner 的源码绑定，按授权远端执行六项，逐名实跑与退出码/哈希齐全才计数。

禁止改 `construction_state.rs`、`native.rs`、`scratch.rs`；若设计在此禁区下无法成立，先把具体接口矛盾交主控，不偷改 guard 或预算。本次无实现、无测试通过声明。
