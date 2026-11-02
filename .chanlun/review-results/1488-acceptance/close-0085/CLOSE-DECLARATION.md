# #1488 关票声明草稿（CLOSE-DECLARATION，0085）

> **当前状态（2026-10-25）**：代码候选为 `c5f49adc97d8e40ab1753a7d295a5d157c15af94`，P2-1/P2-2 已修复、终面验证全过且非作者独审PASS；最新报告与完整探针见 [resume-20261025/REPORT.md](../resume-20261025/REPORT.md)。本文件下文的 0085/0091 读数是历史快照，其中“未改、待负责人裁定”不再代表当前 P2 状态。当前无合入、无关票，main 闸与独审/CI 门仍保留。

> **状态：草稿，不是关票评论。**
> - 标【合入后填】的项，只有在用户合入闸之后才能为真（G3）。现在不写结论。
> - squash 面验证读数见 §8；合并面尾部（pre-squash `8a1924993b`、再合 main `dbfb13d90b`、`5417eea488`（含 #1608 告警门控回退与 114 行 cfg_attr/allow）、`0fb96c00ae`）**需补合并面独审**；squash 面独审 0091 对 `336b53db8d` 预审判 FAIL（P1-1 bsp 白名单锚，终面已修；P2-1/P2-2/P3 交负责人裁定），终面待复审（REVIEW-CHAIN §3.4）。
>
> 本文引用的仓外原件都有收据，见同目录 `receipts/`（报告、远端日志、CI 原件目录、工位输出）与 `../receipts/`（0077/0079）。

## 0. 一句话声明（模板，未成真）

```
关票声明：resolution=【合入后填：squash 提交在 main 上的 hash】，已并入 main；本评论各项在此刻均已为真；
逐 commit 可编译性=单 commit（squash）交付；工位=<§6 已删清单>，白名单保留 <§5 四支>（D9）；
报告=.chanlun/review-results/1488-acceptance/（close-0085/、receipts/、degrade-0065/、s5/、s86/、a5s6/、min12/）；
评审面==关票面（REVIEW-CHAIN.md；squash 面独审【合入后填：独审结论】）。
主线：origin/main @ 【合入后填】（== 关票面）；CI run：【合入后填】 → 【合入后填】。
```

**范围口径**：ADR 0042 写的是 #1612 切片 5 完成后的目标态。本票终面的**生产唯一生成路径仍是完整重算（delta=None）**，增量构造只在 testonly 路径上；本票没有把生产切到增量。

## 1. 交付纪律七项 + G+（docs/agents/delivery-discipline.md「关票前」节）

| 项 | 要求 | 现状 |
|---|---|---|
| 1 resolution 引用落地 hash | 写实际落地的 commit | squash 提交在分支 `codex/1488-squash-20261021` 上（hash 见 PR 头）；main 上的 hash【合入后填】。原提交 hash 保留在 §5 白名单分支上，可达 |
| 2 已并入 main | 合不进不关票 | **未并入**。PR 只开不合，合入走用户闸（D8/S7）。0084 P2-1（再合 main 补审）、P2-2（宽外红）已由 0086/0088 闭合，见 §4；本次三次再合 main（`aa1c3885f8` → `dbfb13d90b`；`21b145250e` → `5417eea488`；`66c409e7e3` → `0fb96c00ae`）的合并面需补审 |
| 3 声明此刻为真 | 不预写 | 本草稿不作关票评论。【】项一律不进评论，直到成真 |
| 4 逐 commit 可编译 | 多 commit 时逐个声明 | 单 commit（squash）交付。squash 面两组 features 非测试 check rc=0（§8） |
| 5 清理工位 | 票关即删 | §6 清单。冻结分支按白名单豁免（§5） |
| 6 报告入 git | 仓内相对路径 | 全部收据随 squash 提交入仓：`close-0085/receipts/{reports,logs,ci,outputs}`，另有 s5/、s86/、a5s6/、min12/（从 accept-s1/s2 分支带入）。仓外大件只登记清单与哈希（`close-0085/INVENTORY.md`） |
| 7 关票面 == 评审面 | 尾部 diff 先补审 | `6badaa6049..327aa94c39` 首尾全覆盖（0084→0087→0088→0090，见 `REVIEW-CHAIN.md`）。**尾部未审**：`8a1924993b`、`dbfb13d90b`、`5417eea488`、`0fb96c00ae`（三次再合 main，需补合并面独审）、squash（fmt + 文档）：0091 预审 `336b53db8d` 判 FAIL，P1-1 终面已修，待复审（REVIEW-CHAIN §3.4） |
| G+ CI 把关面 | 附 origin/main SHA 与 CI run，红照实写 | 【合入后填】。合入前读数见 §8，已知红见 §4 |

## 2. CLOSE-GATE-DRAFT §7.1 最小证据集七项

| # | 项 | 现状 |
|---|---|---|
| 1 | 转出落地 | **已成真**。#1612 已关（ADR 0042，PR #1638）。切片票 #1639–#1645 已开（#1639 已关）。#1488 票面已加转出注（第 43 行注，原文保留，原文备份在 `decisions/1612/1488-body-orig.md`）。R3 去向：S6 trace 结案（§3） |
| 2 | A① | **成立（CONDITIONAL 已确认）**：S5（0075）终面 `3ee05a2713` 生产臂 p1/p2 对 r8a/r8b，INVARIANT 全等；same-binary p1↔p2 全等。CONDITIONAL 三项经主控 6309084398 第 1 条确认，作为改进效果（§3）。对拍脚本独审：`s86/README.md`「独审与已知限制」节 + 0068（原工具独立重跑并审 `diff_list` 修补）。squash 面复跑见 §8 |
| 3 | R1 | **成立**：append 作废根 = 0；不可达 56.11% → 38.50%；残留族与 r8a 逐项同数，已登记到 #1491（6309121665） |
| 4 | A④ 前半 | **成立（testonly）**。S3 四锁 + N1–N5 负控在锁表内。S4 锁 `s4_generation_incremental_matches_cold_full_six_stages` 按 (i) **已并入**（0086 合并面），改为 `#[ignore]` release-only（0089 `ffb89f7fa8`）；跨臂负控 0089 `eb668db7f0` 红 → `a7481aacb5` revert；uo-ci 39588126034 两组 success，S4 PASS_RELEASE 1962 s。S4 口径为「可达闭包逐位」（负责人认可，0087 ⑤-P2-2）。N3a/N3b 变异仍绿（P42），照实写：「prepare_source_audit 阶段的变异体验证在本票不成立，由 #1639/#1641 承接」 |
| 5 | A⑤ | **成立（6309084398 第 4 条口径）**。功能档逐代正确性六项核对：终面与 main 的 g1/g8/g16 全过。冻结档前缀库 head≤2 通过。冻结档全库 `/api/ready` 503 QueryBudgetExceeded，终面与 main 同形，不是回归，归 #1497（6309122652）。产品浏览器归 #1460 |
| 6 | P3 处置 | 见 `P3-STATUS.md`：60 条逐条三选一，加 0084/0087/0088/0090 新增 findings 处置表。P44 已补登 #1491（6309605312）；P27 按转 #1644 结案、不补注释（负责人裁定）；P26、P49 经 0086 修复重判为已结 |
| 7 | 关票门 | squash 已提交、验证见 §8；PR 已开（不合）；合入与 CI【合入后填】；squash 面独审 0091 预审 FAIL（P1-1 已修）待复审；G6 收据随 squash 入仓；G5 见 §5/§6 |

## 3. 验收框与转出注口径（D1–D11 + 2026-10-21 四件）

| 框 / 句 | 归属 | 依据与证据 |
|---|---|---|
| 外部行为句（第 15 行：每代成本 ∝ Δ_R） | 转 **#1645** | #1612 转出注；生产化后才能体现 |
| R1 | 本票，**成立** | §2-3 |
| R2 前半（六阶段增量维护，testonly） | 本票 | S3/S4 锁与测量证据（§2-4） |
| R2 后半（第 20 行后半：读量对 Δ_R 拟合） | 转 **#1642** | 转出注 |
| R3 第 46 条 | 本票结案 | S6（0076）：**台阶不存在**；回复耗时 ∝ 累计 R，属 R2，按 D11 转 **#1642**（6309084398） |
| A① | 本票，**成立** | §2-2 |
| A②（第 26 行，整条） | 转 **#1642** | 转出注。S3/S4/S5 读数作为测量证据交接 |
| A③（第 27 行，整条） | 转 **#1645** | 10s 帽失守（终面与 main 两臂都失守，见 0076）归 **#1645 G1**。FORMED seed 归 #1419 |
| A④ 前半 | 本票，成立（testonly） | §2-4 |
| A④ 后半（第 28 行后半） | 转 **#1645** | 转出注；完整重算路径随切片 5 删除 |
| A⑤ | 本票，**成立** | §2-5。冻结档全库转 **#1497** |
| 8MiB（D7） | 转 **#1641** | 31 代中 29 代 ≥90%，中位 6.07×（6309122187） |
| 残留族 | 转 **#1491** | 6309121665；g28 冷臂多 162 不可达节点与 P44 同登记（6309605312） |
| audit_store.rs:63 逐插入×depth 保守计费（0087 ①-F-2） | 转 **#1612 R2 切片** | 已登记 |

**A① CONDITIONAL 三项 = 本票改进效果**（6309084398 第 1 条确认）：

1. `s_content_nodes` 全集比 r8a **少 224,705**（784,774 → 560,069）。这些节点全部落在 70,051 个裸序列顶点之下，就是本票修掉的 per-bar append 作废中间根。终面多出 0 个；可达集同为 344,419。
2. R1 不可达比例 **56.11% → 38.50%**（215,650/560,069）。
3. append 作废根 = **0**。逐代 stored_* 读数差同源：g86 少 224,705 个节点，节点体字节少 16,178,760。

### 3.1 生产可达的一处行为修正（0090 P2-1 更正口径）

`remove_cert_admitted` 在生产二进制**可达**：legacy v2 serve 带 `--audit-cache-source-bytes N` 时 `cache::configure(N)` 使 `Memo.limit` 非零（`s_session_v2/mod.rs:2465/2574`、`cache.rs:781`、`s_structure_session.rs:5486`，squash 面行号；评审期 `327aa94c39` 上为 2489/2593、779、5478）。0089 `d2b6e25a5d`（红测试 `b4eac1860f`）的修复：**成功路径与修复前语义等价**（0090 `f1_success_path_equivalent` PASS）；**失败路径由「漏账」改为「不改任何账、返回 Err」**。0089 生产 nobid `19be61d4…`（10594112 B）相对 0086 的差异由此而来。本票对生产行为的改动只有这一处错误路径修正。

## 4. 已知红与已修红（照实写）

| 红 | 根因 | 处置 |
|---|---|---|
| 宽对拍 `v3::financial_session` 22 条（MissingDependency） | 依赖只在 macOS 上实现，Linux 远端必败；main 同名同 panic（§8、PR 正文） | 环境性，接受（P59） |
| `v3::state::transaction_material_qualification::actual_entry_seven_exclusions_and_fresh_drop` | 不是顺序依赖：资格门 `state.rs:683` 的 `!cfg!(feature="s_capture_native_testonly")`（main #1394）使 native 组第 0 例必不发报告；测试来自 main，main CI 只在 s_session 组跑 | **修**（0086 `56ddd8d86f`，测试缺陷，归属 **main #1394**，本票随 squash 修复） |
| lib `bsp::no_stray_bsp_binding_rewrites_outside_registry` | 白名单锚随代码漂移 3090 → 3109 | **修**（0086 `66100ef764`，测试缺陷） |
| `content_bi::…raw_coords_recover_after_refused_write_with_same_roots` | S3 rebuild_suffix 改为先收齐尾叶再构造，夹具未登记新源，拒写注入未被触发 | **修**（0086 `56ddd8d86f`，测试缺陷：夹具） |
| `content_publication::…native_publication_begin_and_consumers_share_parent_scope` | P1c 裁定改为 RAII 许可，首条断言陈旧 | **改断言第十八处**（§7）；pre-squash `8a1924993b` 补 drop 后全额还账断言 |
| `staged_nodes` ×3（:675/:721/:765） | 测试读 scratch 自身账，许可实际记在父 Budget，自写入起即红 | **修**（0086 `56ddd8d86f`，测试缺陷：读错账） |
| extras alloc_json 编译失败（0084 未列） | `reserve_owned` 改名 `reserve_admitted` 后测试 5 处未跟；`allocation_budget` 未入作用域 | **修**（0086 `f0aeaf2817`，测试/编译缺陷） |
| lib native B2 codec 五条 | P1c RAII 语义下断言陈旧 | **改断言第十九～二十三处**（§7） |
| N3a/N3b 变异仍绿（P42） | prepare_source_audit 局部增量代码没被执行到 | 转 #1639/#1641 |
| fmt（P58） | 本票引入的未 fmt 差异（squash 时 29 个文件，含再合 #1608 时新加的长属性行），main 有 fmt 门 | **修**：squash 面全量 fmt，nobid 逐字节分类见 §8 |

合入后 main ci.yml 的 `rust-test-default`、`rust-test-s-session` 分片、`rust-test-s-session-extras` 三个 job 预期为绿（0086 已清零 0084 P2-2 各条）；`rust-check` 的 `fmt --check`、三档 `RUSTFLAGS=-D warnings cargo check --all-targets`（默认 / `backtest_bin` / `s_session`）与 #1394 编译负例脚本已在 squash 树上远端实跑全过（§8）；financial 22 条在 main ci.yml 中按 #1394 根裁定 A 以 `--skip v3::financial_session` 跳过（`ci.yml:261–297`，main `66c409e7e3`），由 `ci-macos.yml` 在 macOS 上真跑；Linux 宽对拍才暴露，照实写。合入后实况【合入后填】。

### 4.1 已知限制

- **S4 锁合入 main 后无工作流执行**（0090 P3-2）：S4 在 main `ci.yml` 中只报 ignored；其 release 执行在 `issue-1488-uo-iter` 工作流，该工作流只在 push 到 `codex/1488-uo-ci-20261019` 时触发，PR 到 main 不触发。**转出说明**：#1641/#1645（增量生产化切片）承接时，须把 S4 锁纳入 main CI。
- squash 面 S4 release 读数来自推 CI 分支手动触发的 run（§8）。

### 4.2 转出 / 回报（main 侧）

- 0088 P3-7：main #1394 测试 `actual_entry_seven…` 在 native feature 下期望缺陷，本票随 squash 修复，**回报 main**。
- 0088 P3-5（**观察项**）：`inclusion.rs` 增量测试的参照经 `IncrFactsState`，不独立。

## 5. 冻结分支白名单（D9；豁免交付纪律第 5 项）

| 分支 | 头 | 用途 |
|---|---|---|
| `codex/1488-recover-journal-frozen-20261020` | `fff166540e`（origin 有） | 冻结源，0045 终审；CI 39364508567 80/80 |
| `codex/1488-unified-ownership-v4-frozen-20261020` | `62bad42a1b`（origin 有） | v4 冻结，0030 |
| 合并面分支 `codex/1488-budget-full-chain-fix-v2-20261018` | `0fb96c00ae`（squash 源 = 其树；评审终点 `327aa94c39`） | 第 1–23 处裁改断言的原提交都在其祖先链上。经负责人批准，阶段二已推到 origin 作冻结分支（先 `dbfb13d90b`，再合 main 后依次快进到 `5417eea488`、`0fb96c00ae`） |
| runner 分支 `codex/1488-driver-six-v4-20261020` | `7f0b61c46f`（origin 有） | 六 driver 证据 run 39368457887。P32：只作「六 driver 证据」 |

## 6. 工位与分支清理清单（关票后执行；现在不删）

**关票后删除**（容器两态：残骸）：

- checkout（worktree）：
  - accept-s1/s2/s4、five-0067、runner、budget-handoff-fix-v2（白名单分支本身保留，工位可删）；
  - baseline-0080（b80 完成后）；
  - squash-0085（PR 合入后）；
  - detached 的 mut-0057、receipt-assess、rerun-0071、review2、runner-review、uo-review。
- 工位本地分支：
  - `codex/1488-accept-s{1,2,4}-20261020`：内容已带入合并面（S4 按 (i) 并入）或 squash；
  - `codex/1488-five-0067-20261021`；
  - `codex/1488-frozen-baselines-20261021`（b80，完成后）；
  - `codex/1488-cross-request-budget-handoff-20261018`。
- origin 上的历史分支（删除属对外动作，须用户批准）：
  - `codex/1488-{a2-c2-ci,a2-ci,abc-fix,abc-integration,b2-native,fields,sequence}-20261015`；
  - `codex/1488-stageowner-check{,-r3,-r4,-r5}`；
  - `codex/1488-{fixed-radix-borrowfix-six,r5-driver-six,successor-testfix-six,cross-request-budget-handoff}-20261018`；
  - `codex/1488-uo-ci-20261019`（CI 专用；删前须先按 §4.1 把 S4 纳入 main CI，否则 S4 无执行面）；
  - `codex/1488-squash-20261021`（合入后）。
- refs：`refs/uo1488/*`、`refs/c85/*`，在 NewChanlun 本地与工位仓。
- WSL：`target-*`（c85/m83/r84/b80/s75/a76…）可在关票后回收；`/mnt/g/AgentStorage/1488-accept/` 大件保留，`INVENTORY.md` 登记了它们的哈希。

**白名单保留**：§5 四支。

## 7. 二十三处裁改断言

- **第 1–17 处**：清单与裁定出处见 CLOSE-GATE-DRAFT §4（收据 `receipts/reports/workers__p3-0074__CLOSE-GATE-DRAFT.md.md`）。第 4 处的最终形态 `handoff_peak − entry ≤ candidate.limit` 已登记在锁表中（P18）。
- **第 18–23 处**：依据 P1c 裁定（`control/1488-budget-full-chain-counterexample-fix-v2.md:9`，RAII 持原父许可至真实析构），提交 `56ddd8d86f` / `f0aeaf2817`。**主控已追认**（#1488 评论 6313949653；0088 `RESULT.json` `ratification_18_23.ok == true`，六处均无放宽、删除或 ignore）。

| 处 | 测试 | 改动（0088 核实） |
|---|---|---|
| 18 | `content_publication.rs` `native_publication_begin_and_consumers_share_parent_scope` | 首条 `begin_held > before+M` 改为 `peak() > before+M` + `begin_held == before+M`（更严）；pre-squash `8a1924993b` 补测试尾 drop 后 `live == before` 全额还账，陈旧消息改为 RAII 语义（0088 P3-1） |
| 19 | `s_content.rs` `b2_owner_codec_cross_scope_preserves_only_success_and_charges_current_parent` | 在场 strong `==1` 改为持父账；`8a1924993b` 改为精确 `== 268`（探针：本地 1 + 证明节点许可 267，0088 P3-2）；drop 后 `live==0`、`strong==1` |
| 20 | `s_content.rs` `b2_owner_codec_avl_fixed_delta_keeps_old_addresses_and_logarithmic_paths` | 在场 `strong == 1 + tree.len()`；drop 后 `live==0`、`strong==1` |
| 21 | `s_content.rs` `b2_owner_codec_successful_verify_then_partial_promotion_failure_revokes_all` | 撤销时刻 live 改 0，新增 `peak >= one_node` |
| 22 | `s_content.rs` `codec_original_moves_pointer_capacity_and_releases_all_permits_on_drop`（原名 `…releases_physical_body_only`，`8a1924993b` 改名，0088 P3-3） | drop 后残留改 0（更严） |
| 23 | `s_content.rs` `codec_original_sparse_node_move_drops_only_discarded_body_permits` | drain 段精确断言保留；drop 后改 0（更严） |

**测试缺陷修复（未放宽，0088 判定），报主控知悉**（0088 P3-4）：`staged_nodes` ×3（改读父账）、`state.rs:2893`（native 下期望，归属 main #1394）、`content_bi` 夹具（新源登记前移）；另有 bsp 白名单锚、alloc_json 编译修复。这些不计入裁改断言。

squash 之后，第 1–23 处原提交 hash 只保留在 §5 合并面分支上。

## 8. squash 面验证（阶段二步 7）

验证提交 `336b53db8d`（= main `66c409e7e3` 上 `merge --squash 0fb96c00ae` + P58 fmt + 文档初稿）。最终 squash 提交只在其上 amend 文档，`rust/` 与 `.github/` 树与 `336b53db8d` 逐字节相同，以下读数沿用。远端 nc-remote，`NC_TARGET=target-c85` 串行；大件在 WSL `/mnt/g/AgentStorage/1488-accept/close-0085/`（INVENTORY §3.4）。

本节是第三轮读数。前两轮同套验证均全部通过：`d5ae676bf9`（基于 main `aa1c3885f8`，源 `dbfb13d90b`，uo-ci 39609217753）与 `5b5e264276`（基于 main `21b145250e`，源 `5417eea488`，uo-ci 39640543315）。每轮跑完、开 PR 前 fetch 都发现 main 已前进（#1632 的 `-D warnings` 门；#1659/#1662），按控制文档步 5 再合 main、整套重做。第三轮之后 main 若再动不再追，交主控裁定。前两轮收据仍在 `receipts/`。

| 项 | 结论 | 证据 |
|---|---|---|
| 树等同 | `merge --squash 0fb96c00ae` 的树与 `0fb96c00ae` 树相同；`0fb96c00ae` = 评审终点 `327aa94c39` + pre-squash `8a1924993b` + 再合 main `aa1c3885f8`（`dbfb13d90b`）+ 再合 main `21b145250e`（`5417eea488`）+ 再合 main `66c409e7e3` | `git diff` 空 |
| fmt | P58 `cargo fmt --all` 29 文件 +1082/−336（`fmt2.patch` sha `032ba483…`，在 `336b53db8d` 上干净应用）；fmt 后 `cargo fmt --all -- --check` rc=0。非空白 token 变化只有 rustfmt 的尾逗号与 match arm 块括号 | log `20261022-005529-c582e1e9ee`、`20261022-033549-336b53db8d` |
| 非测试 check 两组 | `--features s_session --lib --bins` rc=0（0 告警）；`--features s_session,s_capture_native_testonly --lib --bins` rc=0（36 告警，不在 CI 门内） | log `20261022-033549-336b53db8d` |
| main `ci.yml` rust-check | 三档 `RUSTFLAGS=-D warnings cargo check --all-targets`（默认 / `--features backtest_bin` / `--features s_session`）rc 全 0；`.github/scripts/check-compile-negatives-1394.sh` 两个负例 PASS（`issue1394_capture_callback_refusal`、`issue1394_closed_owner_refusal`）。native 档（`s_session,s_capture_native_testonly --all-targets`）54 条告警、0 错误，不在 CI 门内 | 同上 |
| 全锁（uo-remote） | debug 97/0（191.11 s）；S4 release `--ignored --exact` 1/0（1042.35 s） | log `20261022-034526-336b53db8d`、`20261022-045435-336b53db8d`（`workers/close-0085/uo-remote-336b53db8d.txt`） |
| uo-ci 两组（推 CI 分支 `codex/1488-uo-ci-20261019`） | run **39654630073** @`336b53db8d` 两组 success。组 0 `s_session`：4 PASS / 93 NOT_LISTED（native 锁不编入该组，同 0089 run 39588126034）；组 1 native：97 PASS + S4 **PASS_RELEASE**（1715.81 s） | `receipts/ci/` 中 39654630073 条目 |
| 宽对拍（native 整 bin，`--skip` S4） | 978 ok / 22 failed / 23 ignored（比上轮多 1 条 ok，为 #1659 新增测试）；失败 22 条全为 `v3::financial_session`（非 financial 0 条）。宽子集（`v3::`/`content_manifest`/`content_store`）753 条：713 ok / 22 failed / 18 ignored | log `20261022-034338-336b53db8d`；`all-failed-336b53db8d.txt` sha `3f7454da…`（与前两轮同件） |
| 失败集对基线 | 对 0086（`56ddd8d86f`，22 条）**逐项全同**；对 0083（`026e3e97b0`，28 条）少 6 条 = 宽外 5 + state 1，均由 0086 修（§4）；对 0079（`2c2d467e26`，23 条）少 state 1 条。无新增失败 | log `20261022-044955-66c409e7e3` |
| financial 22 对 main | `--features s_session --bin s_structure_session v3::financial_session`：squash 4/22/3ign，main `66c409e7e3` 4/22/3ign；22 条 (测试全名, panic 位置, panic 消息) 两边逐行 diff 为空（`fin-pairs-*.tsv` 两边 sha 同为 `0f167731…`）。panic 位置 `financial_session_tests.rs:40:32` ×17、`:633:5`、`:677:35`、`:721:35`、`:806:35`、`:1567:39` 各 1；消息全为 MissingDependency「会话保留量读数当前只在macOS实现」。归因：`financial_session.rs:504–506` 在 `cfg(not(target_os = "macos"))` 下 `heap_probe` 直接返回该错误；本票未改这两个文件 | squash log `20261022-034338-336b53db8d`，main log `20261022-044955-66c409e7e3` |
| 生产 nobid（P58） | fmt 前 `0fb96c00ae` `2b82e38686e81b0c…`，fmt 后 `e4f7ee3b7792c3d4…`，同长 10600736 B；581 字节差全在 `.data.rel.ro`，RELATIVE 重定位两侧 6641 条全等；560 条 panic Location 记录全部只变行号（列不变），判定 `ALL_PANIC_LOCATION`。记录按文件：s_structure_session 280、s_session_v2/mod 56、object_index 46、cache 41、audit_owner 25、business_json 25、aux_index 19、generation 13、evidence_store 9、source_audit 9、content_manifest 9、s_content 8、fixed_radix 6、content_producer 4、s_fact_cache 3、state 2、content_publication 2、catalog_evidence 1、audit_store 1、native_owned 1 | log `20261022-033217-0fb96c00ae`、`20261022-033549-336b53db8d`；分类器 `workers/close-0085/nobid_classify.py` |
| S86 cross-binary（ordinary，86 代） | squash 生产构建 `417d3bc3…`（12902024 B，`rel-full-squash3`）跑 s86 输入，driver rc=0。**对 `s1/r8a`（main 基线 `6badaa6049` 二进制）**：cmp_rc=0，VERDICT PASS；INVARIANT 全部 EQUAL（init、86 回复帧逐位、逐代公开根与 FinancialBasis 根、basis-ref、全部业务表、公开根集合、可达闭包 344419/344419）。CONDITIONAL 有三项 DIFF：binary_sha256（预期）、`s_content_nodes` 全集 784774→560069、逐代 stored_nodes/stored_body_bytes（51261418→35082658 B）。不可达节点 440355（56.1%）→215650（38.5%），这正是本票去作废中间根的效果，可达部分完全相同。**对 `s5/p1`（评审期 `3ee05a2713` 生产构建）**：cmp_rc=0，VERDICT PASS；INVARIANT 全等；CONDITIONAL 只有 binary_sha256 一项 DIFF，`s_content_nodes` 全集 560069/560069 与逐代 stored_nodes/bytes 全等。末代 snapshot 根 `7508b621…`、basis 根 `sc1-c195df59…` 三方一致。前两轮（p1@`d5ae676bf9`、p2@`5b5e264276`）结论相同 | log `20261022-045545-336b53db8d`；`close-0085/p3/cmp-s1-r8a.json` `bc2759a1…`、`cmp-s5-p1.json` `3c9db71e…` |

## 9. 2026-10-25 续接关票门（当前）

代码候选为c5f49adc97，功能分支为codex/1488-fix-p2-resume-20261023。交付计划为squash merge；本分支保留故障与修复过程，不宣称各过程commit逐个可编译。旧§8的树等同和CI仅属于其注明的旧SHA。

P2-1/P2-2已修，具体逐档RC、PR和CI以最新REPORT为准。P1/P2修复与封包独审已PASS；当前PR/CI、实际main合入hash、合入后CI与清理工位尚未齐，因此本声明仍是草稿，不能用于关票。生产增量化仍由#1645承接；不扩大#1488验收范围，也不关闭#1279。
