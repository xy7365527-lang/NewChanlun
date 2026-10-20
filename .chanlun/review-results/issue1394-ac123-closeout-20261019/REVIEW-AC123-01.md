# 独立审查 REVIEW-AC123-01：#1394 AC1–AC3

独立审查 REVIEW-AC123-01，审查者 Claude Opus 5.5（独立 CLI 会话），非作者、非映射编制人。

- 被审 source：`origin/codex/1394-integration-20261013` HEAD `b46ba296993371cb56b2dff7ef878647dc0665ad`（只读 detached worktree，审毕已移除）
- 证据映射：`/Volumes/AgentStorage/issue1394-ac123-closeout-20261019/AC123-EVIDENCE-MAP.md`
- CI：GitHub Actions run 38516525815（headSha = b46ba29699，已核）
- 本机未跑 cargo；行为结论来自读代码 + CI 日志。
- 下文行号均指 b46 上的 `rust/src/economic_session/` 文件。

## 结论一览

| AC | 判定 | 一句话 |
|---|---|---|
| AC1 先 E 增责任才 B Committed；失败/重复/旧版本保原状态 | **PASS** | 经济域里 `Committed` 只在 Commit 判定器一处赋值，且必须先通过 E 的「grant 已入责任 + 时序」检查；失败、重复提交、版本陈旧的负向测试都核对了原状态不变 |
| AC2 实际跑 E 财务步骤，不伪造 grant、不内部赋值 Committed | **PASS** | 测试夹具的 grant_id 与 grant 证明都从 E 库真实 Reserve 回执回读；夹具不直接写 responsibilities 或 Committed |
| AC3 共同满足正常提交；不足给精确等待；FIFO 不代资金排序 | **PASS** | cap 80 时 alpha(30)、beta(40) 各自都塞得下余量 60，但实现一个都不发 grant，只给出 `waiting_for=#1349/atomic-slot-wait/v1`——正好否定了 FIFO 先到先得 |

资金数值未签署（#1349/#1355/#1357），按委托口径不以缺数值判 FAIL；实现只输出 AwaitingAllocationPolicy，没有用默认值代填。

## AC1 依据

**唯一赋值点。** 经济域里 `"Committed"` 的写入只有 `finance.rs:1112`。`s_session_v2/v3` 里的同名字符串是 S 会话消息回执的 kind，属于另一个域。外部入口只能经 `service.rs:699-705` → `finance::write_with_ingress`；`persist`/`reduce` 都是私有函数。

**Commit 前置门（`finance.rs:1043-1110`），按顺序：**
1. 本地仍为 `Prepared` 状态，且通过版本 CAS（:1053-1058）。
2. E 的证明 `verify_proof`（:1060）：在 E 的 owner 下重算 root/cut。
3. slot 当前为 `Reserved`，且与回执逐字相等（:1067）。
4. Reserve 之后新增、未被覆盖的 pending 会阻断提交（:1073-1075）。
5. 已关闭的申请代际被拒（:1081-1088）。
6. grant 必须与原计划、owner、金额、代际一致（:1090-1097）。
7. **E 的 responsibilities 里必须已有该 grant 的完整责任**（:1099-1106，「E 未先保留完整责任」）。
8. `reserve_ns ≤ commit_ns`（:1107-1109）。

**witness 来源不由客户端提供。** 生产 `/1` 路径由服务自己 `proof()` 只读打开 E 库生成 witness（:1399-1406）；`/2` 路径用 `proof_inner`（:1487-1527）。`proof()` 读之前先对 E 库做 audit（:550-558）。

**重复/幂等。** `write_with_ingress` 先查原消息或原回执，查到即返回原回执（:1338-1346）。测试 :323 与 :432-433 断言重发返回同一份回执，后来出现的 Pending 也不会倒写这份回执。

**失败/旧版本保原状态：**
- `:418`：拿旧 grant 首次 Commit 被 MissingDependency 拒绝；beta 仍为 Prepared，责任仍为 90。
- `:450`：epoch 陈旧、S 修订中、plan_hash 不符，`latest` 都与修改前相等。
- `:702`：own_lock CAS，以及 Reserve 的申请者集合不完整，E 状态不变。
- `:505`：Abort 并关闭代际之后，Commit 被拒；E 保留的 90 不释放（Abort 不退款，见 :1134）。
- `:726`：关闭代际时 reduce 返回 StaleWriter；获知时刻早于恢复代际同样被拒；beta 仍为 Prepared。
- `:583`：6 种操作 × 提交前/提交后 × 两个新进程，真 SIGKILL（断言 signal 9）后恢复 epoch2，回执和责任总额都符合预期。

## AC2 依据

- 夹具 `Fixture::request`（finance_tests.rs:242-253）里的 `grant_id` 取自 E 库的 `receipts` 表中真实的 Reserve 回执。`witness`（:257-278）的 grant/prepared 证明调用的是生产同一个 `proof()`，从 E/B 真实库回读。
- 前置步骤都通过实际服务或 persist 执行：Capture/Apply/BindVoice 走 `service::handle`（:199-206），ImportResponsibility 走 `persist`（:208）。全文件没有对 `responsibilities`、`slots`、`state` 的直接赋值；`"Committed"` 只在断言里出现（:315/375/423/436/493）。
- 只有 S 结构基准是单位夹具，用 `self.basis` 代替读取 S 二进制；文件头 :1 已明示这一点，属于声明过的 TestOnly 前置，没有越界。数量、价格、cap 都是 TestOnly 输入，没有冒充生产资金政策。
- 正式 CLI/Unix 端到端证据见 EVIDENCE.md:23、:35、:85：三个 serve 进程，Prepare→E Reserve→B Commit，并声明「没有伪造 grant 或手工置 Committed」。

## AC3 依据

- **正常提交**：`:302` cap=100，责任 20+30+40=90，两个 grant 都 Committed；Q/持量/锁逐项断言，历史时点 85 仍为 Prepared（不倒写历史）。
- **精确等待**：`finance.rs:1027-1029`。总额超 cap 时，状态为 `AwaitingAllocationPolicy`，`waiting_for` 取输入里的 `allocation_policy_id`（`input()` 强制等于 `ALLOCATION_POLICY_ID`，见 :231 附近），不发任何 grant，责任不增加。`advance` 收到等待态后直接返回，不发起 Commit（:1759-1761）。
- **FIFO 不代资金排序**：`:342` cap=80、既有责任 20，余量 60。alpha(30) 或 beta(40) 单独都能放下，FIFO 会先批 alpha；测试断言 `grants == []`、`retained_total == 20`，即整个 slot 原子等待、不挑赢家。另外，只要有任一 slot 在等待，其它 slot 的 Reserve 一律被拒（`finance.rs:918-925`「不得让其他 slot 插队」）。
- 健康申请者不被不相干的故障挡住：`:358` 与 `:485`。

## 映射核对

- 14 个测试名和行号（302/342/358/384/418/450/485/505/521/583/702/726/765/836）与 b46 源码逐一吻合；`durable_child` 在 :533 标注 ignored，属实。
- CI 38516525815：headSha = b46ba29699。`rust-check` 和 `macos-session` 两个作业里，`economic_session::finance::tests` 都是 14 ok + 1 ignored。`rust-check` 只有 `theta_v0::backtest` 3 条失败（l3_delta_r_alpha m3_partition_synthetic_holds、tick calibration_sweep…、tick seam_btc_tick…），#1520 为 OPEN 状态「main CI 红：3 个 theta_v0::backtest 测试失败（非 #1394 回归）」。其余作业 success。映射的 CI 描述属实。

## 新发现

### HIGH
无。

### MEDIUM
- **M1 关票 source 口径不一致。** #1394 正文写「AC1–AC3 由本票在 **b46+C1** source 上完成」，但映射和 CI 都只覆盖 b46。C1 = `99e819192b`（父提交为 b46），只改了 `rust/src/bin/s_session_v3/financial_reader.rs` 与其测试（S 侧末块预算），不触 `economic_session/`，所以不影响 AC1–3 的判定代码。不过 issue 评论里记录 C1 的「Rust NOT_RUN」，本机能看到的远端分支里也没有包含 C1 的分支。建议关票时写明「AC1–3 证据 source = b46；C1 不触 AC1–3 路径」，或者补一次 C1 的 CI。不阻断 AC1–3。
- **M2 `/2` 新域的端到端证据早于 b46。** b46 上 CI 跑的 AC1–3 单测全部在 `/1` 域（夹具 `schema_revision: REV`）；`/2` 域没有 Reserve/Commit 测试（只有 :765 的配置校验）。EVIDENCE.md 的正式 CLI/Unix 与浏览器证据产生在 `eb1c60bf…` 或更早的提交上。从那以后：
  - `financial_content`/`content_store`/`finance_transport` 共改动约 1160 行；
  - 政策 id 由 `#1349/unresolved-common-cap-allocation` 改为 `#1349/atomic-slot-wait/v1`（finance.rs:231），旧输入在 b46 上会被拒。

  AC1–3 的判定逻辑（`reduce` 的 Reserve/Commit 分支）是 `/1`、`/2` 共用的，所以单测加 CI 足以支撑 PASS。但「`/2` 读取链上的 AC1–3」在 b46 上没有实跑证据。这件事已经归 #1552（「另须在统一 source 上复证 AC1–AC3」），这里只登记、不扣分。

### LOW
- **L1 AC2 映射引用不当。** `:836 trusted_ingress_deadline_is_not_restarted_by_economic_job` 只测期限计算，没有执行任何 E 财务步骤，不能作为 AC2 的依据。删去它不影响 AC2 结论。
- **L2 缺两条负向测试。**
  - 「E 尚未 Reserve 就发 B Commit」：生产路径里 `proof()` 会返回 MissingDependency（:558），但没有直接测试；夹具的 `witness` 在这种情况下会 unwrap panic，导致这条路径写不出测试。
  - 「slot 处于 AwaitingAllocationPolicy 时尝试 Commit」及「其它 slot 插队被拒」（:918-925）：夹具只有一个 slot，没有测到。

  代码读下来是正确的，属于覆盖缺口。
- **L3 单测绕过 `write` 的 witness 组装。** Reserve/Commit 单测直接调 `persist` 并自组 witness，所以 `write` 自己组装 witness 的那段代码（finance.rs:1371-1407）只被正式 CLI 证据（旧 source）覆盖。自组用的是同一个 `proof()`，不构成伪造，但与 M2 叠加。

## 最终裁决

AC1、AC2、AC3 均 **PASS**。M1/M2 是 source 口径与 `/2` 复证问题，后者已归 #1552；它们不推翻 AC1–3 在 b46 上的行为结论。

VERDICT: PASS
