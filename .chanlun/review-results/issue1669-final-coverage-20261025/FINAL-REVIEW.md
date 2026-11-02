# PR #1676 / issue #1669 独立非作者最终复审

- 复审时间：2026-10-24T23:59:21Z（UTC）
- 候选：`c9e8a74a6e2d5a09d6e3dba59edd3626e652315f`
- base：`origin/main = 13ed4d45c8ff85175819ca987690328ab2a247a4`
- 旧 head：`438f1d440766daa4ee5195e0859cfbca6154c76f`
- 旧 rebase 基点：`3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 候选有效性：本地 detached HEAD、远端分支 `codex/1669-oq2a-short-lock`、PR `headRefOid` 三者均等于候选；PR `baseRefOid` 等于上述 base，PR 仍 OPEN。
- 方法：读取根 `AGENTS.md`、#1669、PR #1676、旧尾部复核与补正记录、ADR 0043、Lean 合同、生产实现、完整候选 diff 和 rebase `range-diff`。未改代码、票据或分支，未派孙代理，未运行本机 cargo/rustc/cargo test。

## 裁决

**代码／合同：PASS。**

| 严重度 | 数量 |
|---|---:|
| CRITICAL | 0 |
| HIGH | 0 |
| MEDIUM | 0 |
| LOW | 0 |

旧尾部复核对 `3af30f1157` 提出的 HIGH（正本覆盖面声明未进入候选）已经由实际写入 `.chanlun/definitions/sanjieduan.md` 的声明解决，不是靠 PR 措辞或追加评审替代交付。未发现 rebase 引入的新缺陷，也未发现证据退缩。

**当前整票合入门：BLOCK，但阻断项不是代码／合同发现。** 截至上述复审时刻，候选 CI 尚有 3 个 job 在运行，且 main 合入仍需授权的人执行；详见下方独立 CI 节与剩余人闸。这里的 PASS 不预写 CI 成功，也不授权合入或关票。

## 完整 diff 与 range-diff

候选相对 base 共 8 个文件、319 行新增、6 行删除：

- `rust/src/economic_session/tb03c_book_tests.rs`
- `.chanlun/definitions/sanjieduan.md`
- `.chanlun/review-results/issue1669-20261022/{MUTATION.md,review-r1.md,review-r2.md,review-r3.md}`
- `.chanlun/review-results/issue1669-final-coverage-20261025/{REVIEW-BEFORE.md,REPAIR.md}`

生产 Rust、`formal/` 和 `s_session/tests/fixtures/tb03c` 冻结夹具在候选 diff 中均为零改动；`git diff --check` 通过。

执行的对照为：

```text
git range-diff 3ff38accdd..438f1d4407 13ed4d45c8..c9e8a74a6e
```

6 个提交全部标为 `=`（相同补丁），没有 `!`、`<`、`>` 或冲突适配：

1. `86ade11acf = ca59e8b8de`：空头性质锁与覆盖面交付；
2. `056f8d0d06 = 36ec14c3bb`：rustfmt；
3. `3adb35bb08 = 21b3f090ab`：收窄 Lean 措辞、补注释与命中种子数；
4. `9cadbb5949 = 8bae5a5c97`：措辞订正；
5. `3af30f1157 = 9857803f05`：三轮评审与变异记录；
6. `438f1d4407 = c9e8a74a6e`：正本覆盖面补交。

另对旧、新 rebase 基点直接比较 `tb03c_book.rs`、`book.rs`、`tb03c_book_tests.rs`、`TotalWealth.lean`、`sanjieduan.md`，结果为空；因此这些承重文件没有需要解释的隐性行为适配。旧 head 到新 head 的净差异来自新 base 的其他集成文件，不在本 PR 写集内。

## 决定性源码与测试证据

### 1. 空头蕴含按 ADR 0043 订正式锁住，而非旧等式

ADR 0043 裁定三（`docs/adr/0043-chong-reversal-account-continuity.md:27-35`）要求：

```text
空头 growth_eligible ⇒ latch.rank ≥ CapitalRecovered；
空头事件不把 latch 推进到 EarningShares。
```

候选旧夹具断言已从空头 `latch == CapitalRecovered` 改为 `rank >= 1`，并单独排除 `EarningShares`（`rust/src/economic_session/tb03c_book_tests.rs:472-483`）。新增性质锁在 `:492-543` 对 12 个种子逐步断言：

- `r >= prev_rank`：latch 不回退；
- `r <= 1`：本票同向空头书账不会由空头事件推进到 `EarningShares`；
- 阶段事件数不减少；
- 存储资格等于唯一谓词对同一图像的重算；
- `Eligible ⇒ r >= 1`，并检查 gate 与未签政策的 `AwaitingPolicy`；
- 已 latch 而资格随后失效时，资格必须是 `NotEligible`，latch 保留。

这与订正后的单向蕴含一致，没有把蕴含误写成双向等价。

### 2. 资格重算走同一个生产谓词，不是测试另造一套判据

生产唯一谓词为 `growth_eligible`（`rust/src/economic_session/tb03c_book.rs:283-304`）：空头分支只比较 `realized_after_fees >= principal`。`Numbers::status`（`:366-374`）调用该函数；测试入口 `growth_eligible_of`（`:385-389`）也从同一 `Numbers::of(book)?.status()?` 重算。新锁在 `tb03c_book_tests.rs:516-517` 对同一存储图像逐步核对，符合 #804 的同输入同输出义务。

生产阶段导出在 `tb03c_book.rs:599-623`：只有 `status == "Eligible"` 时，空头才把本轮目标 rank 设为 1，最终 latch 取 `prev_rank.max(rank)`。因此测试同时覆盖资格蕴含和 latch 单调性，且直接挡住所记录的两类变异。

### 3. 非真空与变异负控仍在候选中

- `tb03c_book_tests.rs:501-542` 明确跑 `1..=12` 个种子，每个 `random_short_books` 返回 61 个图像（开局 + 60 步），并断言总数 `eligible > 0`、`lost_after_latch > 0`；失败信息带实际命中种子数。
- `random_short_books`（`:545-621`）生成加空、带正／负具名已实现盈亏的回补、借券期间费用和追加保证金批；`realized_step`（`:786-808`）构造对应原件与见证。
- `MUTATION.md` 保留两类负控结果：空头 Eligible 错推 rank 2，以及空头资格错读 latch；两者都由新增测试捕获，原有多头锁保持绿色。未变异基线记录为 `tb03c_book_tests` 8/0。

本审没有重跑这些变异，故把它们作为候选内的历史执行证据，而不是本机新执行结果；当前候选的独立静态核查确认相应断言仍在，未发生证据删除或断言弱化。

### 4. 跨方向边界没有被盗称覆盖

#1669 明定换向单账与开多求值点补发 `enterEarning` 的负控归 #1378。本票生成器不换向，正本也在 `sanjieduan.md:36` 明写这一边界。因此本审只认可本票同向空头序列及既有夹具上的锁，不把它扩张解释为「多头到 `EarningShares` 后换空」已经由本票测试覆盖。这是票面既定分工，不是新发现。

### 5. Lean 覆盖面声明与合同原文一致，旧 HIGH 已实销

`.chanlun/definitions/sanjieduan.md:34` 的原「证明与对拍义务」句和 `:32` 的 #1590 订正均保留；`:36` 新增带 #1669 和日期的实际覆盖面登记。逐项对照 Lean：

- `TWState`（`formal/Origin/TotalWealth.lean:97-109`）没有 `side` 字段；`units : Int` 的 #891 注记确写「多正空负」。
- `stage_rank_monotone`（`:194-205`）与 `earning_no_regress`（`:212-225`）都是单步定理，均以 `e ≠ clearCampaign` 为前件，覆盖 campaign 内阶段单调性，不是资格蕴含。
- `LegalEnterEarningStrict`（`:665-667`）的第一个合取项确为 `s.stage = capitalRecovered`；它是严格 EnterReady 的入口合法性谓词，而正本 `:28-30` 禁止 latch 作为扩量资格输入，不能冒充 `growth_eligible`。
- 对 `formal/*.lean` 检索 `growth_eligible|growthEligible` 无命中；`TotalWealth.lean` 中也没有按 `units` 正负分方向的资格定理。

所以「多头、空头蕴含及空头事件不推进第三相均由 Rust 性质锁承担，不是 Lean 已证结论」是准确的收窄声明，没有把 Lean 的其他定理误删或夸大成资格证明。

## CI（独立门，仅记录已发生状态）

候选 CI run [`40142835153`](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40142835153) 的 `headSha` 已核对为候选。截至 2026-10-24T23:59:21Z，run 仍为 `in_progress`，以下 3 项尚未完成：

1. `rust-test-s-session (shard 0)`；
2. `rust-test-s-session (shard 1)`；
3. `rust-test-s-session-extras`。

已完成并 SUCCESS：`rust-check`、`rust-test-default`、`rust-test-backtest-bin`、`rust-test-s-session` shard 2/3、`s-observer`、`fixture-drift`、`sandcastle-ts`、`test`；`macos-session` 为 SKIPPED。DevSkim run `40142835243` 已 SUCCESS。

run 未结束时 GitHub 不提供完整日志，因此本审没有声称新性质锁已在当前候选 CI 日志中单独出现并通过，也没有预写上述 3 项结果。票面「CI 全绿」目前仍未满足；若后续任一 job 失败，须按实际日志重新裁决，不能由本报告的代码 PASS 抵销。

## 剩余 main 人闸与关票门

1. 等当前候选 CI 实际完结并满足票面全绿要求；本审不代跑、不补写结果。
2. 由有授权的人决定是否合入 `origin/main`；本审不合 main、不关票。
3. 合入后按 `docs/agents/delivery-discipline.md:60-80` 核对关票面 `origin/main` SHA 与对应主线 CI run，再填写真实 resolution。
4. 多提交交付按纪律采用 squash 或逐提交可编译性声明；关票前核对「评审面 == 关票面」、清理工位，并确保拟引用的最终评审报告已经入 git。本文件按任务要求写在仓外，若关票要引用它，仍须由后续授权流程入仓，不能把当前绝对路径当作已入仓证据。
