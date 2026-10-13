# #1414 / TB-02-D 独立审查（P2 再验收实际运行独审、增量整体独审与历史整体独审）

> 名分：工作草稿，活期绑定 [#1414](https://github.com/xy7365527-lang/NewChanlun/issues/1414)（`docs/agents/generation-constitution.md` §5）。本文件是关票门第 7 项“评审面 == 关票面”的仓内载体。
>
> 本文件由 C5 重写起草工位汇编。现行部分是 P2 修正后新字节的实际运行独审：审查者 `/root/tb1414_pr_p2_triage`，独立于 `/root/tb1414_p2_fix`、`root`。下面的 `REVIEW.md` 全文放在四个反引号的围栏里，围栏行本身不属于原文；起草工位没有改动、删节或评论原文。历史部分是 787f5cf 提交面的两轮整体独审，逐字取自本文件上一版，按根裁定 ROOT-RULING（`600207e3a2f2`）只作历史记录。
>
> 第九个提交（登记 CI 与 DevSkim 增量）追加了 `cea15bd7` 提交面的增量整体独审：审查者 `/root/tb1394_r6_contract`，此前未参与 #1414。它的 `REVIEW.md` 全文同样放在四个反引号的围栏里，起草工位没有改动、删节或评论原文；本文件此前已有的逐字收录节与历史节一字未动。
>
> 核对方法：取围栏内的内容（不含首尾两行围栏），其 bytes 与 SHA256 应等于下表所列原件。

## 结论

P2 修正后的新字节（写者 `58a135103968`，HEAD `179412c33d52`）上，四臂 fresh 与两对 221 行比较的实际运行独审为 `PASS_TB02D_P2_RERUN_ACTUAL_ACCEPTED`：阻断 0，非阻断 5，延后 0。根已签收 `ROOT_ACCEPTS_TB02D_P2_RERUN_ACTUAL`（2026-10-13T12:06:07Z），deferred 为空，五条非阻断项随附。本裁定只覆盖 TB-02-D 的 P2 新字节四臂与两对比较，不裁整图、不裁合入。

C5 报告提交 `cea15bd7` 的提交面增量整体独审（`787f5cf1` → `cea15bd7`）为 `PASS_INCREMENTAL_SURFACE_READY_FOR_MERGE_REQUEST`，阻断 0。它要求合入请求随附两条 CI 事实（IR-M1 rust-check 实测、IR-M2 DevSkim 增量），已由第九个提交的 `CI-DELTA.md` 与 `ACCEPTANCE-REPORT.md` §8、§12 登记。该审只绑定 `cea15bd7`；第九个提交改了本目录四个文档文件，对这些文件的轻量增量复核 PENDING。

## 现行独审字段

归档根 `CTRL-20261012` 的历史定位是 `/private/tmp/goal1323-native-20261012`（本机临时目录）；完整收据另见 `EVIDENCE-INDEX.json` 的 `acceptance.actual_rerun_review` 与 `acceptance.root_acceptance`。

| 字段 | 值 |
|---|---|
| `actual_rerun_review.location` | `CTRL-20261012:tb1414-pr-p2-triage/actual-rerun-review/` |
| `actual_rerun_review.review_md.bytes` | 12095 |
| `actual_rerun_review.review_md.sha256` | `48c3fc552f3cd4085d4118ef90ffc568f18b92390f0263f8c308a210be2aa34e` |
| `actual_rerun_review.result_json.bytes` | 4637 |
| `actual_rerun_review.result_json.sha256` | `2c53227fe2e3966d6bd7c7c1deeb49183e0bd493201479dce88dab5bdebecfc4` |
| `actual_rerun_review.binding_json.bytes` | 17597 |
| `actual_rerun_review.binding_json.sha256` | `7d76eaf76be9d648af9f5b33827c5810050e4f7b5b6e5aab0f1e6345addb8c0f` |
| `actual_rerun_review.status` | `PASS_TB02D_P2_RERUN_ACTUAL_ACCEPTED` |
| `actual_rerun_review.blocking_findings` | `[]` |
| `actual_rerun_review.non_blocking_findings` | N1, N2, N3, N4, N5（本仓记为 P2R-N1–N5，见下表） |
| `actual_rerun_review.deferred` | `[]` |
| `actual_rerun_review.checks.check_actual_p2` | 174/174 |
| `actual_rerun_review.checks.large_recheck` | PASS，16 件 + 4 份 core，读 7,007,878,532 B |
| `root_acceptance.path` | `CTRL-20261012:tb1414-p2-rerun-root-release/ROOT-P2-ACTUAL-ACCEPTANCE.json` |
| `root_acceptance.bytes` | 4627 |
| `root_acceptance.sha256` | `49dc3b93dfb81ddc61b50bb9ef6bfa90cfa1ec5695a55e5f13c6f3359b8c0ce1` |
| `root_acceptance.status` | `ROOT_ACCEPTS_TB02D_P2_RERUN_ACTUAL` |
| `root_acceptance.accepted_at_utc` | `2026-10-13T12:06:07.712319+00:00` |
| `root_acceptance.deferred` | `[]` |

非阻断发现（RESULT 原文）：

| ID | 内容 |
|---|---|
| N1 | 浏览器30s帽余量0.44–0.62s，较attempt 2的0.65–1.06s更薄；耗时去向未定位；帽值不得静默放宽 |
| N2 | RESULT.json semantic_oracle/browser not_evaluated 标签过时（同上次线索一） |
| N3 | PID退出观测警告措辞与wrapper只核PID的残余同族风险（同上次线索二） |
| N4 | requestfailed 25/16/19/15，不在门内，无pageerror；HAR只做SHA未分析 |
| N5 | Chrome主文件ctime变化，在已声明合同内；整bundle未验证 |

根签收随附的非阻断项（原文）：

- 浏览器 30 s 帽余量 0.44–0.62 s，再跑或进 CI 前须先查清耗时分布，不得悄悄放宽上限
- RESULT not_evaluated 标签过时
- PID 退出警告措辞与 wrapper 只核 PID
- requestfailed 计数 25/16/19/15 不在门内
- Chrome 主文件 ctime 变化在合同内

## 增量整体独审（`787f5cf1` → `cea15bd7`）

完整收据另见 `EVIDENCE-INDEX.json` 的 `acceptance.incremental_final_review_of_new_surface`、`acceptance.ci` 与 `acceptance.devskim_codeql_delta`。

| 字段 | 值 |
|---|---|
| `incremental_surface_review.location` | `CTRL-20261012:tb1414-incremental-surface-review/` |
| `incremental_surface_review.review_md.bytes` | 13202 |
| `incremental_surface_review.review_md.sha256` | `c6b0adf14e108d95b17916dd95e85d9ab99f4ec0f5c1bde813faa8d8d6520c5a` |
| `incremental_surface_review.result_json.bytes` | 8854 |
| `incremental_surface_review.result_json.sha256` | `8d8d2a800b60eeb4970628581ef397021b2a6e7c0fa08da8bf6981cb2501c917` |
| `incremental_surface_review.binding_json.bytes` | 9800 |
| `incremental_surface_review.binding_json.sha256` | `1b82b5dc3acd57385165289b32846bc78a7434e772497fa96f9a74c82ea0b2f8` |
| `incremental_surface_review.reviewer` | `/root/tb1394_r6_contract`（此前未参与 #1414） |
| `incremental_surface_review.subject` | HEAD `cea15bd735bc351cde21a2d734f77c020d26555f`；增量 `aa4039cc`、`179412c3`、`cea15bd7`（自 `787f5cf1`）；相对 main 47 路径 |
| `incremental_surface_review.status` | `PASS_INCREMENTAL_SURFACE_READY_FOR_MERGE_REQUEST` |
| `incremental_surface_review.blocking` | `[]` |
| `incremental_surface_review.merge_request_must_carry` | IR-M1、IR-M2 |
| `incremental_surface_review.scope_note` | 本审绑定 `cea15bd7`；任何新提交（含登记 CI-DELTA 或改报告）需对新增文件做轻量增量复核 |

发现（据 RESULT.json 摘录，文字有压缩；末栏为第九个提交的处理）：

| ID | 级别 | 发现 | 审查给出的处置 | 第九个提交的处理 |
|---|---|---|---|---|
| IR-M1 | MEDIUM，非阻断 | CI 实际结论未登记；漏登 `179412c3` 上 tower_scaling 0.723 的计时红 | 合入请求随附 `ci/CI-RUNS.json`；计时测试抖动另开票，不得悄悄放宽 0.7 门 | 登记于 `CI-DELTA.md` 与报告 §8、§12；0.7 门不放宽，建议另开票跟踪，本提交不开票 |
| IR-M2 | MEDIUM，非阻断 | DevSkim 增量未登记 | 按 `DEVSKIM-DELTA.json` 登记；CI-DELTA.md 或基线由根定，基线须在最终 head 生成 | 按 #1404 先例登记于 `CI-DELTA.md`，未改基线；需修 0 条 |
| IR-L1 | LOW | 仓内自述停在提交前（报告 HEAD、索引 `commits.head`、`ci.ci_result`、提交后重核待办） | 本审已完成提交后重核；不要求为此再提交 | 随本次登记一并订正报告 §4 与索引 `commits`、`ci` |
| IR-L2 | LOW | 跨对象 `facts.from_stroke == strict.elements[1]+1` 未锁 | TRIAGE 已列可选，留后续 | 留后续 |
| IR-N1 | note | `test_tb02c_compare` 依赖 ijson | 环境项；CI 先装 `requirements-tb02c.txt` | 无改动 |

## P2 的前置独审

两份前置独审都由同一独立工位完成，结论与发现见 `ACCEPTANCE-REPORT.md` §6、§7.1–§7.2，原件不在本文件逐字收录：

| 独审 | REVIEW.md | RESULT.json | BINDING.json | 结论 |
|---|---|---|---|---|
| P2 修正（`aa4039cc`） | `dcfc1b8d8bb2` | `a4a630aba25c` | `1e72e48bc7e4` | `PASS_P2_FIX_READY_FOR_FRESH_RERUN`；FX-L1/FX-L2 由 `179412c3` 处理 |
| 再验收包 | `eb9beae33abb` | `c0caf1e5c3ea` | `432425c2039e` | `PASS_P2_RERUN_PACKAGE_READY_FOR_ROOT_AUTHORIZATION`；PK-L1–PK-L3 为 LOW |

## P2 实际运行独审全文（逐字）

````markdown
# #1414 P2 修正后四臂 fresh 实际运行独立审查

结论为 `PASS_TB02D_P2_RERUN_ACTUAL_ACCEPTED`。P2 修正后的新字节在 UP/a、UP/b、DOWN/a、DOWN/b 四臂上按授权 PLAN 和原全部成功门真实跑完；两对 221 记录用原比较器逐字节比较，均为 PASS。已发布的 SECOND `scan.from_stroke` 在 Q 安装状态、Q 原始响应和浏览器卡片中都是 4，且等于 `facts.from_stroke`。上次延后的大件全部按运行内收据当场重算，结果一致，本次不再有延后项。没有阻断项。本裁定只覆盖 TB-02-D 的 P2 新字节四臂与两对比较，不裁整图。

审查人 `/root/tb1414_pr_p2_triage`，独立于作者与根。审查对象：OUT `/Volumes/AgentStorage/issue1414-p2-rerun-20261013`，授权 `ROOT-P2-RERUN-AUTHORIZATION.json`（`5be5cb41…`），前门重建收据 `tb1414-p2-rerun-pregate-1/REBUILD-RECEIPT.json`（`4b6265d7…`），启动记录 `tb1414-p2-rerun-launch-1/`。审查标准沿用 `tb1414-explicit-composite-review/actual-remaining/REVIEW.md`。本审核查脚本 `check_actual_p2.py` 输出 CHECK.json，174 项全过。

## 授权、前门、冻结与时序

授权文件现场重算为 `5be5cb41…`，各字段如下：

- status 为 `ROOT_APPROVED_P2_RERUN_FOUR_FRESH_ONCE`，output 等于 PLAN 的 OUT；
- plan_sha256 为 `4a097a2c…`，与现件一致；
- preparation_binding 为 `21aa94c4…`；
- 四个执行物的 SHA 与准备目录现件逐件相同；
- runs 四元组等于 PLAN.runs；
- independent_review 共 5 件，现场 SHA 都没变；
- binary_rebuild_receipt 为 `4b6265d7…`；
- calls=1，no_retry 与 first_failure_stop 都为 true。

前门收据 status 为 `PASS_REBUILT_WRITER_MATCHES_EXPECTED`，plan、script、rebuild、toolchain 都与 PLAN 相同。HEAD 构建前后都是 `179412c3…`，已跟踪文件前后都干净，exit 0，没有剥离任何环境变量，构建耗时 23.3s。二进制从 `9f2a9f52…` 变为 `58a13510…`（4254944 B），build stdout/stderr 的 SHA 与收据一致，目录里只有三件文件。worktree 二进制现在的 SHA 与 lstat 六字段都等于收据中的 `binary_after_stamp`。上次包审记下的 L2（在 worktree target 中能否重建出同一字节）由这次实测解决：逐字节相同。

OUT/freeze 中的 7 件（PLAN、授权、四个执行物、REBUILD-RECEIPT）都与原件 SHA 相同。PACKAGE-BINDING 的 12 项也逐件相等（6 件包、1 件前门收据、5 件独审）。准备目录自本人包审以来只新增了授权文件，其余文件逐件未变。

时序如下：前门收据 19:23:20，根签名 19:26:35，controller 启动 19:26:51，OUT 中最早的文件 19:26:51.5，OUT 中最晚的文件 19:51:43.281，controller 结束 19:51:43.290（都是 +08:00）。OUT 共 87354 个文件，没有晚于运行结束时刻的。CONTROLLER-EXIT 记录 exit 0、waited_and_reaped、无终止信号，耗时 1492.13s；controller、supervisor 的 stdout/stderr/log 都是 0 字节。EXECUTION.json 的 cause 为 null，四臂都在 completed_fresh 中，总耗时 1491.73s。

## 逐臂成功门

四臂同构，都满足以下各项：

- EXIT 为 `{"exit":0}`；RESULT 状态为 `CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER`，input_count 40，core_records 221，cleanup.errors 为空；stdout 与 RESULT 相同；6 个 stderr 都是 0 字节。
- 40 条 ingest 的 operation 为 0..39，全部 Received。messages 与 clock 的 SHA 等于矩阵 historical_generated_inputs；prepared 三件等于 RUN-PLAN 记录。
- 124 条 bounded_query 的命令序列逐项等于矩阵 public 120 条加 recovery 4 条，deadline 都是 30。四臂共 496 份 Q 收据全部为 `captured_validated_commit`，且 full_state_installed、sources_unchanged 都为 true。
- query-oracle 为 passed，七个 cut 15/26/27/31/38/39/40 依次为 NO_FIRST、NO_FIRST、PENDING_CASE_TWO、PENDING_CASE_TWO、PENDING_CASE_TWO、CASE_TWO、CASE_TWO。本次 oracle 由新的 `tb02d_verify.py`（`1dc3de9a…`，各臂快照已绑定）执行，它包含新增的「已发布 scan 四字段对 oracle second_scan」比对（`:71`）；passed 即表示这条比对在各 cut 上通过。
- browser/RESULT 为 passed，cuts 为 ["27","40"]，page_errors 为空。READINESS 记录 failure=false、cleanup_errors 0、pageerror 0，五张 PNG 齐全。txt 中的业务字符串全部在场：CC-012 有 FIRST_SEQUENCE、SECOND_SEQUENCE、matched；CC-013 有 CASE_TWO、geometric_end、terminated_known_at；CC-014 有 fact_vector；developing（切到 27 AsKnown）有 PENDING_CASE_TWO、stroke_broken、segment_broken。
- **SECOND 卡片 `scan.from_stroke` 为 4，不是 0。** 四臂 cut40 的 CC-012.txt 中，SECOND_SEQUENCE 卡片在 scan 下的 from_stroke 值都是 `4`（三张 FIRST 卡分别为 0、3、6）。该卡的对象 id 等于 Q 中 cut40 的 SECOND 对象（UP `obj-0e8992b5…`，DOWN `obj-ac711b69…`）。cut27 developing 卡引用的 `second_sequence_object_id` 等于 Q 中 cut27 的 SECOND 对象（UP `obj-fc50c198…`，DOWN `obj-cdf49837…`）。旧字节下 UP cut27 的 SECOND 对象是 `obj-736ad23a…`，这次不同，正说明新字节生效。
- 正常恢复：equal=true，cuts [27,40]，epoch 1→2 且 epoch_transitioned，start 回到 ready，generation 40，writer_epoch 2。6 条控制事件 exit 都是 0。
- 资源回收：WRAPPER-CLEANUP passed；S/Q 的登记 PID 在 ps 中无行，端口 errno 61，socket 不存在；wrapper-stop-s/q 都走已停路径（无 signal，process_identity 等于登记）；运行时停机记录的 PID 等于登记，信号为 SIGTERM。stop-q/stop-s 的退出观测警告措辞与上次相同，属已知过渡现象，见发现 3。

## 窗与帽

| 臂 | 整臂 | 单Q最大 | Q合计 | collect_core上界 | oracle | browser（mtime/内计） | 距30s余量 |
|---|---:|---:|---:|---:|---:|---:|---:|
| UP/a | 350.3s | 7.06s | 223.0s | 31.4s | 7.4s | 29.53/29.35s | **0.47s** |
| UP/b | 352.6s | 7.20s | 224.1s | 34.1s | 7.6s | 29.50/29.32s | **0.50s** |
| DOWN/a | 360.6s | 7.39s | 229.6s | 34.5s | 7.6s | 29.56/29.38s | **0.44s** |
| DOWN/b | 358.9s | 7.28s | 228.7s | 34.6s | 7.7s | 29.38/29.20s | **0.62s** |

所有窗与帽都在限内：arm900、collect120、oracle180（oracle 内 node 帽 120）、query30、browser30。没有出现 browser_cleanup 事件，所以 +5 回收分支没有触发。浏览器余量 0.44–0.62s，比 attempt 2 的 0.65–1.06s 更薄，见发现 1。

## 两对比较

PAIR-COMPARISON-RESULT 为 PASS，比较对为 UP/a↔UP/b 与 DOWN/a↔DOWN/b，fresh_runs 是四臂的固定顺序。比较器是原 `tests/tb02c_compare.compare_exact_traces`（`96c7bb45…`，CURRENT 与各臂快照都已绑定），布局为 `expected_layout(40)` 共 221 行，method 为 `exact_bytes_and_strict_jsonl`，`excluded_fields=[]`，`identity_renaming=[]`。

- UP 一对：两侧 sha 都是 `5e85f955…`，1456959869 B。
- DOWN 一对：两侧 sha 都是 `0696fb1f…`，1461830301 B。

compared_bytes 等于两侧文件的实际大小。结果中不含旧锚 `c0dbe0db`，OUT 里也没有 COMPOSITE-COMPARISON-RESULT。UP core 的字节数与旧锚相同而 sha 不同：修正把 `"0"` 换成了 `"4"`，长度不变，内容变了，与预期一致。

## 新字节确为本次运行来源

- 四臂的 source-before-run MANIFEST 各 105 项，逐项等于 RUN-PLAN.source_sha256（102 项）加 prepared（3 项）。RUN-PLAN 等于 CURRENT 中运行时子集加二进制。本审对 4×105 份快照字节（包括 4MB 的二进制副本 0003）全部重算，无一不符。
- 快照与 CURRENT 中的身份：client `d8ae4af5…`、合同 `7d138949…`、`tb02d_verify.py` `1dc3de9a…`、runner `a3d32f0b…`、runtime `e17fd1f6…`、比较器 `96c7bb45…`、写者 `58a13510…`。http/SESSION.json 在 helper 启动时记下的 client 也是 `d8ae4af5…`。
- `tb02c.rs` 不在运行时快照域内。它的身份 `25181131…` 由三处同时成立：`git show 179412c3:` 的 blob、worktree 现件与 CURRENT 三者相等；前门正是在这个 HEAD 上重建出 `58a13510…`；运行内 SOURCE-POSTCHECK 为 552/27 零失配。
- 本审现场重算 CURRENT 全部 552 件（包括二进制）与 frozen_evidence 全部 27 件：零失配。

## 已发布的 SECOND 扫描范围

三个口径独立核对，都成立：

1. **Q 安装状态**（`committed-candidate.json`，cut c 对应 `known-(c-1)`）：四臂在 cut 27、31、38、39、40 都恰有一个 SECOND 对象，`scan.from_stroke` 为 "4"，scan 的四个字段等于 facts 的四个字段，也等于 hand-oracle 中 `second_scan` 的 wire 值（through 为 5/6/7/8/8，input 为 6/7/8/9/9，resource_truncated 为 false），状态为 PENDING/PENDING/PENDING/CONFIRMED/CONFIRMED。
2. **Q 原始响应体**（`raw_q_second.py` → RAW-Q-SECOND-CHECK.json）：抽查 known-26、known-39、after-normal-27、after-normal-40，cut27 与 cut40 的 SECOND 对象 scan 等于 facts，from 都是 4；恢复前后是同一对象。
3. **浏览器**：见逐臂成功门，cut40 卡片显示 4，cut27 引用同一对象。

## 大件：按运行内收据直接重算，不再延后

四臂各自的 `LARGE-ARTIFACT-SHA256.json` 都存在，errors 为空，business_gate=false，identity_stable 全为 true，收据 SHA 等于 EXECUTION 中的记录。本审（`rehash_large.py` → LARGE-RECHECK.json）重算了 16 件 HAR/DB/WAL/backup：SHA、字节数和 lstat 五字段都等于收据，读取期间身份稳定。同一 case 的两臂 `session.sqlite` 与 database-backup 字节相同，WAL 与 HAR 不同。另外重算了四份 semantic-core，SHA 等于 PAIR 中的记录、大小等于 compared_bytes；写者二进制与四份快照 0003 都是 `58a13510…`。共读 7,007,878,532 B，结果 PASS。

## 依赖与现场资源

DEPENDENCY-PRE 与 POST 都是 PASS，没有 mismatch 或 error。python、node、playwright 三项前后收据全等。Chrome 只有 ctime 前后不同，这在已声明的 `named-chrome-object-content-endpoints/1` 合同之内，与上次相同。

升级只读复核（LIVE-RESOURCE-CHECK.json）：18990–18993 都是 ECONNREFUSED，四个 socket 都不存在，ps 中没有本次运行的 S/Q/runtime/driver/helper/runner 进程。69 行自动化 Chrome 都是本次运行前就已存在的常驻或其他工位进程，最早 10-11 19:25、最晚 10-13 06:26；19:22–19:52 之间启动的一个都没有。核查命令自身的 2 行已排除。

## 发现

阻断：无。

非阻断：

1. **浏览器 30s 帽余量进一步变薄**，只有 0.44–0.62s（内计 29.20–29.38s）；attempt 2 是 0.65–1.06s，codepoint observed 是 29.14s。这是系统性贴帽，再跑或进 CI 有翻转风险。前 128 个事件之后的 244 个被丢弃，耗时去向仍然不可见。建议在下一次需要该运行之前先做定位；帽值不得静默放宽。
2. RESULT.json 中 `semantic_oracle`/`browser` 的 `not_evaluated` 标签已过时，原因同上次线索一，四臂相同。交付文案不要单独引用这两个字段。
3. PID 退出观测警告措辞与 wrapper 只核 PID 的残余同族风险，同上次线索二，不阻断。
4. 浏览器 requestfailed 计数为 25/16/19/15，不在门内，也没有伴随 pageerror。HAR 本次只做了 SHA 重算，没有打开分析。
5. Chrome 主文件 ctime 变化，在已声明合同之内；整 bundle 未验证。

## 实际所做

全部只读：

- `check_actual_p2.py` → CHECK.json：174 项全过；
- `rehash_large.py` → LARGE-RECHECK.json：PASS，读 7.0GB；
- `raw_q_second.py` → RAW-Q-SECOND-CHECK.json：PASS（第二次以文件形式重跑，输出逐字节相同；比对时用过一个 `/tmp` 临时文件，已删除）；
- 内联重算 CURRENT 与 frozen_evidence → SOURCE-RECHECK.json：零失配；
- 升级只读的 ps 与端口复核 → LIVE-RESOURCE-CHECK.json。

没有启动 Q、S、浏览器、runtime 或构建，没有改 OUT、包或 worktree；OUT 的文件数与最晚 mtime 在审查前后不变，worktree 干净。没有 commit/push/PR。
````

## 增量整体独审全文（逐字）

````markdown
# #1414 / PR #1477 增量提交面整体独审（787f5cf1 → cea15bd7）

审查人：`/root/tb1394_r6_contract`（此前未参与 #1414）。对象：worktree `/Users/silencehan/.codex/worktrees/goal1323-tb1414/NewChanlun`，分支 `codex/1414-second-sequence-20261012`，HEAD `cea15bd735bc351cde21a2d734f77c020d26555f`（工作区干净；`origin/main` 与 merge-base 均为 `f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9`，main 未移动）。增量为 `aa4039cc`（修正）、`179412c3`（测试）、`cea15bd7`（报告）三个提交。只读：未改 worktree、未提交、未在 GitHub 写任何东西；GitHub 只做 GET（check-runs、作业日志、`gh run download` 取 SARIF 工件）。

## 结论

**`PASS_INCREMENTAL_SURFACE_READY_FOR_MERGE_REQUEST`，阻断 0。** 修正、测试与报告三层都对得上；概念层与原文一致，代码层两侧门同序同文案并被变异测试证实能拦住字符串取锚与 Number 精度两种错法，报告层 121 处抽样与 OUT 全量清点全部相符，三份「全文（逐字）」逐字等于原件。

合入请求必须随附两条 CI 事实，它们在报告里仍写作 PENDING 或「预计」（IR-M1、IR-M2，均非阻断、均无需改码）：

1. `179412c3` 那次 rust-check 多出一个红：`incremental_tower_scaling_dominates_full_synthetic` 比值 0.723 对门 0.7。它是计时抖动：库字节在 787f5cf1、aa4039cc、179412c3、cea15bd7 四个提交上逐字节相同（P2 只改 `rust/src/bin/` 下的 `tb02c.rs`），该测试在其中三次和 main 上都通过，cea15bd7 的运行 38397240254 又只剩 3 个既有红。
2. DevSkim：cea15bd7 相对 787f5cf1 与相对 main 的新增告警全部是误报或 loopback/固定回调提示，**无一需修**；门在 main 上本来就红（main 自身 33443 条基线外），处置走 CI-DELTA.md 还是登记基线由根定。

## 概念层

正本未改：`067`、`078` 两课博文与 `.chanlun/definitions/xianduan.md` 相对 main 无改动。修正落的是 067:38【正文】「从该分型最高点开始的向下一笔开始的序列」与 078:54「方向是和原线段一致」（xianduan.md §3 G-3，:131-150）：库内 `second_sequence`（`feature_seq.rs:298-330`）早已取 `from = b+1`、只收与原线段同向的笔，P2 修的是发布面——`tb02c.rs:163` SECOND 的 `scan.from_stroke` 由 `scan.from_stroke` 改为 `second.from_stroke`，与另外四个已取自 `second` 的字段对齐；FIRST（:146）仍是 `c.start_stroke`。身份锚取 `stroke_refs` 中数值最小的笔号：refs 由 `stroke_refs()` 从 `c.start_stroke` 到 `observed_through` 升序枚举，最小值恒为段起笔，与 Rust 侧段身份 `strokes[c.start_stroke].identity_anchor`（:140）同源。

#799/#804：同一锚规则同时服务 FIRST 与 SECOND，锁是严格等值与严格大于，没有宽严两档，也没有按级别分叉的判据。未锁的跨对象关系 `facts.from_stroke == strict.elements[1]+1` 仍只由 Rust 构造保证（IR-L2，TRIAGE 已列为可选）。

## 代码层

`wire()`（`tb02c.rs:18`）把一切 JSON 数字转为 `n.to_string()`，`stroke_index` 是 `usize`，所以 refs 的键是规范十进制文本，两侧按文本等值比较可靠。

Python `s_tb02_contract.py`：refs 键经 `integer(...,0)` 校验并拒重复；:592 `if not refs` 先拒空，:595 `min(refs, key=int)` 取锚，:596 FIRST 锁 `scan.from_stroke == 锚`，:604 SECOND 四字段等于 facts 且 `int(scan.from_stroke) > int(锚)`，:773 身份由 `refs[segment_start]` 推出。JS `tb01c-client.js` :286-294 同序：先 `refs.size > 0`，再 BigInt reduce 取最小，再 FIRST 锁、SECOND 锁；拒绝文案逐字相同，`test_empty_stroke_refs_rejected_by_both_sides` 断言了两侧同一道门同一文案。`tb02d_verify.py:71` 新增已发布 scan 四字段对 oracle `second_scan` 的比对。下游读者（rg）只有上述三处与合同测试，`tb02d_browser.cjs` 只匹配角色文本，object_id 级联已由 TRIAGE §6 说明。

在 `git archive cea15bd7` 导出副本上做了两次变异：把 JS 取锚与 SECOND 比较改成 `Number`，`test_identity_anchor_keeps_precision_above_2_53` 失败；把 Python 改成 `min(refs)`（字符串序），`test_identity_anchor_is_numeric_minimum_of_decimal_text` 出错。两条新锁都确实拦得住它们要拦的错法。

## 测试读数（导出副本，本机）

| 项 | 结果 |
|---|---|
| TB-02-D 五模块（CPython 3.14.6，CI 同命令） | Ran 40，OK |
| `test_tb02c_*.py`（runtime venv 3.14.6 + ijson 3.5.1） | Ran 14，OK（系统 Python 缺 ijson 时 `test_tb02c_compare` 导入失败，属环境；CI 先装 `requirements-tb02c.txt`） |
| `node --test test_tb02c_contract.cjs` | 6 过 0 败 |
| `cargo test --locked --offline --features s_session --bin s_structure_session`（nice 10，独立 target `/private/tmp/tb1414-incr-review-target`） | 81 passed，0 failed，1 ignored，471.74 s；含 `tb02d_all_frozen_cases_project_into_strict_read_contracts` |
| 变异：JS Number / Python 字符串取锚 | 各被对应测试拦截 |

CI 上的 rust-check 在「default targets」失败后不再执行 s_session bin 全量与 backtest_bin 两步，所以上面这次 bin 全量是对 cea15bd7 字节的唯一一次全量 bin 读数。

## CI 实际结论

| 提交 | 运行 | rust-check default targets | s-observer | DevSkim |
|---|---|---|---|---|
| cea15bd7 | 38397240254 / 38397240180 | 2955 过、3 败（3 个既有红），tower_scaling ok | 成功；TB-02-D 五模块 Ran 40 OK，3.14.6，布局 True | 红 |
| 179412c3 | 38387197713 / 38387197687 | 2954 过、**4 败**：3 个既有红 + tower_scaling（0.723 ≥ 0.7） | 成功 | 红 |
| aa4039cc | 38385269425 / 38385269384 | 2955 过、3 败，tower_scaling ok | 成功 | 红 |
| 787f5cf1 | 38344720471 / 38344720431 | 3 个既有红，tower_scaling ok | 成功 | 红 |
| main f95a7f3e | 38201220739 / 38201220747 | 3 个既有红，tower_scaling ok | 成功 | 红 |

tower_scaling 的判定依据：该测试（`incremental_parity.rs:1422-1484`）直接构造带合成段的 `ParseLayer` 调 `classify`/`classify_incremental`，不走本 PR 改过的分段与特征序列代码；`git diff f95a7f3e cea15bd7 -- rust/src/theta_v0/classifier` 为空，classifier 不引用 `FeatureSeqState`；它对 debug 构建下 n=400 的墙钟比值设 0.7 门，179412c3 那次 full 0.158 s、inc 0.115 s。报告提交于 12:24:59Z，晚于 179412c3 的 rust-check 完成（11:07:51Z），却仍写「预计与 main 一样因 3 个既有红失败」「CI 实际结论 PENDING」（ACCEPTANCE-REPORT.md:207、:306），漏登了这次多出的红。作业日志存 `ci/`，汇总在 `ci/CI-RUNS.json`。

## DevSkim 增量（三份 SARIF 实物）

三次运行的 `devskim-sarif` 工件已下载到 `devskim/`，扫描器均为 `devskim 1.0.90+fb2d676ce4`。按 `scripts/devskim_sarif_gate.sh` 原样对 `.github/devskim-baseline.json`（8201 键，本 PR 未改）跑门：cea15bd7 基线外 34616、787f5cf1 34265、main 33443，三者「Fixed since baseline」都是 14。门在 main 上本就红，PR 归因只能看相对 main 实测 SARIF 的差集。

| 比较 | 门口径（规则\|路径\|行） | #1404 先例口径（多重集，忽略行列） |
|---|---|---|
| cea15bd7 vs 787f5cf1 | +600 / −249 | +393 / −42，全部 DS173237 |
| cea15bd7 vs main | +1176 / −3 | +1173 / −0 |
| 787f5cf1 vs main | +825 / −3 | +822 / −0（与根当时记录的 822 相符） |

增量里的非 DS173237 项全是行号迁移，多重集口径下增删都是 0：

| 规则 | 路径 | 787f5cf1 → cea15bd7 | 成因 |
|---|---|---|---|
| DS162092 | run-package/remaining-driver.py | 101 → 185（`connect_ex(('127.0.0.1',…))`） | 驱动整体替换（595fa4c00937 → ab80440cdebd），同一语句下移 |
| DS162092 | run-package/remaining-driver.py | 156 → 241（`s.bind(('127.0.0.1',…))`） | 同上 |
| DS197836 | run-package/remaining-driver.py | 170 → 255（`runplan['source_sha256'] == expected_runtime`） | 同上 |
| DS172411 | s_session/browser/tb01c-client.js | 647 → 652、742 → 747（`setTimeout(() => abort.abort(), …)`） | aa4039cc 在 :244 后加 1 行、:286 后加 4 行 |

DS173237 增减 595+244 个键（相对 main 1164 个）逐条取 cea15bd7/787f5cf1 源行核对：命中值全部是 64 位 SHA-256 或 40 位十六进制，后者 10 个值逐个 `git cat-file` 为本仓 commit 或 blob；字段名只有 sha256 类、commit/head/base 类与「文件路径 → SHA」映射，没有 token/secret/password/key 类字段。相对 main 另有 DS117838（Critical）5 条：CURRENT-SOURCES.json 中路径含 `key`（`key.rs`、`key_truth.rs`、`key_reuse.py`、`two_pass_evidence.rs`）的「路径 → SHA」行；DS162092 两条在一次性驱动、一条在本机验收脚本 `tb02d_runtime.py:98`（`http://127.0.0.1:` 浏览器地址）；DS197836 是把 `runtime` 里的 time 当成时间值；DS172411 的回调是固定箭头函数。判定：**全部误报或提示，需修 0 条**。main 已有的三条（`s_readonly_server.py` 1193→1234、`tb01c-client.js` 583→747、118→146）只随 PR 下移。

若根选择登记基线：键含行号，`EVIDENCE-INDEX.json` 与 run-package 每改一次就整体迁移（170→255 即一例），只能在最终 head 上生成；#1404 先例的 CI-DELTA.md 用多重集口径，对迁移不敏感。逐键清单与判定见 `devskim/DEVSKIM-KEYS.tsv`（2028 行），汇总见 `devskim/DEVSKIM-DELTA.json`。

## 报告层

`scratch/sample_report.py` 对报告所列数字与 SHA 重算实物 121 处，全部相符：依据材料 20 件 SHA（TRIAGE、ROOT-RULING、两版 schema、修正独审、实际运行独审三件、根签收、包独审三件、P2-fix 与 C2 结果、PREPARATION-BINDING、CONTROLLER-EXIT、SECOND-SCAN-Q-CHECK、HISTORICAL-RECEIPT-VERIFY、负例）；OUT 七小件 SHA；run-package 七件的 HEAD blob SHA 与字节数，以及与 OUT `freeze/`、CTRL 原件逐件相等（28 处）；新字节 10 件身份与写者二进制（`58a135103968`，4254944 B）；两对比较的状态、字节、记录数与两侧 SHA；EXECUTION 1491.73 s 与四份大件收据 SHA；SOURCE-POSTCHECK 552/27/零失配；PACKAGE-BINDING 12 项；DEPENDENCY-POST PASS；两臂大件收据值与 stat 字节（不读内容）；四臂 CC-011/CC-012 截图 SHA；四臂浏览器窗 mtime 口径 29.53/29.50/29.56/29.38 s；前门 23.3 s 与 19:23:2x；controller 19:26:51.152–19:51:43.289、1492.13 s、exit 0；根签收 12:06:07Z、deferred 为空。OUT 全量 stat 清点为 87354 文件、28,687,713,979 B，与报告一致。

`FINAL-INDEPENDENT-REVIEW.md` 中三份「全文（逐字）」作为连续子串逐字等于原件：实际运行独审 REVIEW（`48c3fc552f3c`，12095 B）、r1（`1149a14bd331`）、r2（`3165f21d94fb`）；787f5cf1 版从 r1 起的历史节整段原样保留。报告 §10 的限制、§11 的历史降级与 §12 的 3 个既有红登记都与 ROOT-RULING 和根签收一致。

## 提交面

相对 main 恰 47 路径（19 M、28 A），与报告口径一致；本增量 17 个文件，`rust/` 下只有 `tb02c.rs` 一行。run-package 移出 `completion_io.py`、`explicit_composite.py`、`reuse_state.py`，加入 PLAN、授权、前门收据，名分仍是绑定 #1414 的工作草稿证据副本。密钥模式无命中（DevSkim 结论见上），无会话导出或工具残留，HAR/DB 只在报告文字中被引用。

## 发现

| ID | 级别 | 发现 | 处置 |
|---|---|---|---|
| IR-M1 | MEDIUM（须登记，非阻断） | 报告 §8、§12 仍写 CI PENDING 与「预计 3 个既有红」，漏登 179412c3 上 tower_scaling 0.723 的计时红；cea15bd7 的实际结论（只剩 3 个既有红、TB-02-D 五模块 CI 40 OK）也未入报告 | 合入请求随附 `ci/CI-RUNS.json` 的事实；该计时测试的抖动另开票跟踪，不得悄悄放宽 0.7 门 |
| IR-M2 | MEDIUM（须登记，非阻断） | DevSkim 增量在报告中仍为 PENDING | 用 `devskim/DEVSKIM-DELTA.json` 登记；CI-DELTA.md 或登记基线由根定，基线须在最终 head 生成 |
| IR-L1 | LOW | 仓内自述停在提交前：报告 :49「当前 HEAD 与远端分支同为 179412c33d52」，索引 `commits.head=179412c3`、`ci.ci_result=PENDING`、:99 的提交后重核待办 | 提交后重核已由本审完成；不要求为此再提交 |
| IR-L2 | LOW | 合同未锁跨对象 `facts.from_stroke == strict.elements[1]+1` | TRIAGE 已列可选，留后续 |
| IR-N1 | note | `test_tb02c_compare` 依赖 ijson，系统 Python 缺包时导入失败 | 环境项；CI 已先装依赖 |

照旧随附：P2R-N1–N5（浏览器 30 s 帽余量 0.44–0.62 s 等）与 FS-N1–FS-N8，本增量未改变其事实。若根为登记 CI-DELTA 或修正报告再提交，HEAD 会变，本审只绑定 `cea15bd7`，新提交需要一次针对该文件的轻量增量复核。

## 读取边界

未读 semantic-core（约 1.46 GB）、HAR、session.sqlite、WAL、database-backup 的内容，大件只读收据与 stat 字节；未跑全量测试、服务或浏览器；未用 `git worktree add`，导出副本经 `git archive` 落在本目录。
````

## 历史：787f5cf 提交面的两轮整体独审

下面两轮审查的对象是 787f5cf 时的五提交提交面（审查者 `/root/tb1414_final_surface_review`，Python 逐行审由其子工位 `py_line_review` 完成）。按 ROOT-RULING，它们只作历史记录；新提交面需要新的增量整体独审。首轮 `FAIL_WITH_FINDINGS`，唯一阻断 B1 已改为显式五模块；复审 r2 为 `PASS_FINAL_COMMIT_SURFACE_READY_PENDING_RUNTIME_CHECKS`。两轮的非阻断项 FS-N1–FS-N8 仍记在 `ACCEPTANCE-REPORT.md` §10。

| 轮次 | 原件（CTRL-20261012 相对路径） | bytes | SHA256 | 结论文件 RESULT.json | 绑定 BINDING.json |
|---|---|---:|---|---|---|
| r1 首轮 | `tb1414-explicit-composite-review/final-surface/REVIEW.md` | 13794 | `1149a14bd331f652c96fca2d1286c8728cf2cc900a3efaa2b3c4c8665fae9db8` | `24f9332d47cb3048910c48ceb8f5ae9c0c0ceeb3aa6222add21e28b8f919acf1`（FAIL_WITH_FINDINGS） | `47caee9123858a9f31c8d98f61db3899ad81c74c08e04433f2c63d41982fb675` |
| r2 复审 | `tb1414-explicit-composite-review/final-surface/r2/REVIEW.md` | 7416 | `3165f21d94fbcce52034498d566d71a90c0defd3c0178113071ebeca741c8dee` | `c055481d64a9fca17a3c80741c76050b4ba9f87556e85c8b9f64b96b8a0053f7`（PASS_FINAL_COMMIT_SURFACE_READY_PENDING_RUNTIME_CHECKS） | `f6327077651ccff687ba5572a4c7296d06c9b2668db531ea53c23dabee0be3a7` |

## r1 首轮全文（逐字）

````markdown
# #1414 / TB-02-D 提交面最终整体独审

> 名分：独立审查工位 `/root/tb1414_final_surface_review` 的审查报告（独立于作者与根），工作草稿。只读 worktree，未 stage/commit/push，未构建、未跑测试或产品；仅做小文件读、git 只读命令、内存编译/AST、`node --check`、`rustfmt --check` 与少量 gh 只读查询。Python 逐行审由子工位 `py_line_review` 完成（`work-A/FINDINGS.md`），本人复核其阻断项后采纳。

## 结论

**FAIL_WITH_FINDINGS。阻断 1 项（B1），非阻断 8 项。** 概念层质询通过：SECOND 同向笔、任意分型、独立方向性包含、strict 首笔封口、065 笔破坏与段破坏分立，均与 `.chanlun/definitions/xianduan.md` 及博文正本一致，未见矛盾。工程层的唯一阻断在 C5：新增的 CI 步会把 `test_tb02d_projection_cache.py` 拉进 discover，而该测试需要 Rust 写者二进制，CI 下必然 ERROR；同时 C5 注释和 C6 报告都把它说成已进 CI。

B1 的修法只需改 `ci.yml`（C5）与报告（C6），不触碰 35 件运行来源字节，已跑的三 fresh 证据仍覆盖产品与测试代码。修后需对改动字节做一次小范围复审。其余 44 件（C1–C4 与 run-package、EVIDENCE-INDEX）本审不要求改动。

## 提交面身份（已核）

- 清单 `COMMIT-SURFACE.json` SHA256 = `30e3a205ffa5f3f18f969ab8902b65db6dd6e0737703abad6449647445fccbee`，与指派一致。
- worktree HEAD `f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9`，分支 `codex/1414-second-sequence-20261012`，暂存区为空；`git status --porcelain=v1 -uall` 恰 46 路径，与清单逐一相同，无清单外路径（子工位报 34 项是未加 `-uall` 时目录折叠的计数，非漂移）。
- 46/46 件 bytes、SHA256、HEAD 原值（或 ABSENT）、git 状态全部与清单一致（`byte-check.json`）。分提交计数 C1 5 / C2 8 / C3 19 / C4 4 / C5 1 / C6 9。
- 运行来源集 `run-package/CURRENT-SOURCES.json`（`6e984f6b…`）中 531 个 worktree 路径现值全部相等（含 4,254,944 B 的 release 二进制；这只是当前文件身份，不替代根的 D1 补跑），6 个 helper 副本 SHA 与其登记值相等。
- 远端 `main` 仍是 `f95a7f3`（gh 只读），未前进；#1414 票面 OPEN（`issue1414-live.json`）。

## 阻断发现

**B1 · HIGH · C5（连带 C6）· `test_tb02d_projection_cache.py` 在新 CI 步下必然报错**

- `.github/workflows/ci.yml:304` 执行 `unittest discover -s s_session/tests -p 'test_tb02d_*.py'`，匹配到 `s_session/tests/test_tb02d_projection_cache.py`。
- 该文件 `:12` 导入 `tb01c_query_tests as base`，`:17` 的 `ProjectionCacheTests(base.QueryTests)` 继承基类 31 个测试并自带 14 个。基类 `tb01c_query_tests.py:44-48` 的 `setUpClass` 经 `:60` 用 `subprocess.run([str(BINARY), "init", ...])` 建库；`BINARY` 在 `:29` 默认 `None`，只有 `__main__` 带 `--binary` 时赋值（`test_tb02d_projection_cache.py:331-332`）。discover 不走 `__main__`，于是执行名为 `None` 的程序，`FileNotFoundError`，整类 ERROR，步骤退出码 1。文件内无 `load_tests`、`setUpClass` 覆盖或 skip 守卫。
- `s-observer` job 不构建 Rust，CI 里补不出二进制。
- 冒称：`ci.yml:293`「这些测试只用标准库与 node」不成立；`ACCEPTANCE-REPORT.md:195` 称该步「覆盖 contract、discovery、projection_cache、validate_once、parse_pool、key_reuse 六个文件」不成立。CI-WIRING 第 3 条早已指出「收据里没有对最终字节执行这条完整 discover 命令的记录」，本项即该缺口的实际后果。
- 建议修法（只动 C5/C6）：把 discover 换成显式模块清单，只跑不依赖二进制的五个文件，例如 `cd s_session/tests && python -B -m unittest -v test_tb02d_contract test_tb02d_discovery test_tb02d_validate_once test_tb02d_parse_pool test_tb02d_key_reuse`；projection_cache 要么在构建了 `--features s_session` 二进制的 job 里以 `--binary` 方式单独执行（需先确认它对解释器版本的依赖），要么照实写为「未进 CI」并说明理由。改测试文件加 skip 会改动运行来源字节，须根另行裁定，不建议。修后以本机 CPython 3.14.6 先跑一遍最终命令并留收据，再复审 `ci.yml` 与报告的改动字节。

## 非阻断发现

**N1 · MEDIUM · 生产解析器的 strict 语义变更波及面超出 S 会话，且最终 Rust 字节没有全量 cargo test 收据。** `rust/src/theta_v0/parser/feature_seq.rs:666-683` 的 `frozen_first` 改变了所有 `FeatureSeqState` 使用者的结果（`IncrSegments` 经 `theta_v0/classifier/streaming.rs:42` 进入 classifier，`divide_segments` 进入 `bin/tangency_probe.rs`、`parser/profile.rs`）。语义上它落实 `067:170/196`【答疑】「被第一笔封闭才是第一种」，属票面 AC1「strict 封闭按已裁语义」，判在范围内、无概念矛盾；但 core 实现记录只有靶向运行（D 测 5 项、旧 C parser 8 项；bin 过滤一轮 >360 s 后 TERM，`tb1414-core-freeze/IMPLEMENTATION.md:22`），没有 `cargo test --all-targets` 与 `--lib --features backtest_bin` 对最终字节的结果。根须在 G5/CI 前跑全量。另建议另开文档票（可并入 #813 线）把「首笔 c 冻结」写进 `xianduan.md` §4 及其受影响代码清单，与已登记的行号漂移一并处理。

**N2 · MEDIUM · 非 observe 生产路径的分配成本上升，未测。** `feature_seq.rs:299-397` 的 `second_sequence` 现在对每个缺口候选构建完整证据（`FeatureElement` 含 `Vec`、steps、clone），`:728` 每次非 provisional 求值再整体 clone 进 `self.second`；默认 `second_seq_scan_window=0`（无限，`theta_v0/config.rs:65`），无分型的单调段会在增量路径上重复全扫。语义不变，成本未在生产负载上测量。建议后续票只在 observe 时构建证据；现在改会作废运行证据，不在本提交面处理。

**N3 · LOW · C2 单独检出时 bin 测试必红。** `rust/src/bin/s_session_v2/tb02c.rs:388-435` 新测试起 `python3` 调 `s_tb02_contract`，C2 时 Python 合同仍是 HEAD 版、不认 `s-segment/2`，故 `cargo test --features s_session --bin s_structure_session` 在 C2 失败（`s_structure_session.rs:4345/4445` 两处既有 python3 对拍预计同样受影响）。逐 commit 只要求可编译，C2 仍可声明可编译，但提交说明须把「Rust bin 测试在 C2 与 C3 之间不保证通过」写明，COMMIT-PLAN 目前只提 Python 合同测试；或合并 C2+C3。

**N4 · LOW · 子工位 F-2：** `s_session/s_tb02_contract.py:383` 先 `evidence.get("scope")` 后做形状检查，非 dict evidence 抛 `AttributeError`，不在 `s_readonly_server.py:984/:1074` 的捕获列表，客户端拿到断连而非 503；仍是失败关闭。修改会动运行来源字节，留后续票。

**N5 · LOW · `tb02d_verify.py` 有三处子集式比较。** `:46` 用 `set(expected) <= set(members)`（ORACLE-DESIGN 已声明 FIRST 全集另列），`:49`、`:58` 以实际对象的键遍历预期，实际缺键不报。合同校验器另行约束键集，判非阻断。

**N6 · LOW · `test_tb02d_snapshot_validate_once.cjs`、`test_tb02d_strict_clone.cjs` 以 `test_` 命名但必须带外部归档参数。** 逐行审：二者只读、`wx` 写输出、先后核源 SHA、比较新旧实现的接受集与错误文本，未发现缺陷；`strict_clone` 依赖的导出标记在现行 client 中恰出现 1 次。风险只在未来有人按 glob 跑它们；`ci.yml:294` 与报告 §8 已注明。

**N7 · LOW · 机器特定路径进入 git 历史。** `run-package/remaining-harness.py:13,80`、`explicit_composite.py:12,15`、`reuse_state.py:41,46` 及 `CURRENT-SOURCES.json` 555 个键含本机用户目录、外卷名与 `/tmp` venv；`tb02d_harness.py:78` 的 socket 固定 `/private/tmp/`（仅 macOS）；`ORACLE-DESIGN.md:23` 引仓外工作稿。均非凭据，报告已披露。全部新增与改动文件未发现凭据、令牌或私钥。

**N8 · LOW · CI 安全扫描的表述需校准。** CodeQL 工作流当前为 `disabled_manually`（#1107，gh 只读核得），run-package `.py` 现在不会产生 CodeQL 增量，报告「会进入 CodeQL Python 扫描面」只在重新启用后成立。DevSkim 零新增门预计因 JSON/MD 中的 SHA-256 触发 DS173237 而变红，处理按 #1404 `CI-DELTA.md` 先例或登记基线，由根定（G7）。

## 必须覆盖项的逐项结论

1. **逐行代码审。** Rust：`feature_seq.rs`、`segment.rs`、`mod.rs`、`tb02c_tests.rs`、`tb02d_tests.rs`、`tb02a_facts.rs`、`tb02c.rs`、`s_structure_session.rs` 全部 diff 逐行读过；SECOND 同向与起点（`from_stroke_idx+1`）、任意分型、合并检测与旧 `apply_inclusion` 一致；`break_facts`（`segment.rs:317`）按 065:44/46 的 `j≥i+2` 枚举（earlier 同向笔、later 自 earlier+3 起的反向笔）正确；`new_segment_formed/segment_broken` 的 `false` 与 `null + scan_complete=false` 区分、终结证书准入条件与 JS/Python 合同一致；未引入新扫描 cap。s_session 产品与测试：JS client、`index.html`、`tb01c_http_capture.cjs`、`test_tb01c_browser.cjs`、`test_tb02c_contract.cjs`、README 由本人逐行审；Python 五件产品与六个新测试由子工位逐行审（`work-A/SEEN.json`）。Snapshot 删去的 5 行确由 `validateTB02CutSources`（client `:496-504`）以同一组校验覆盖，不是放宽。round G 三件运行脚本 `tb02d_harness.py`、`tb02d_runtime.py`、`tb02d_verify.py` 与 `test_tb02d_snapshot_validate_once.cjs` 的当前字节已逐行审：进程组回收、超时、来源快照绑定、收据断言均成立，`collect_core` 内无嵌套 SIGALRM，发现仅 N5、N6、N7 三条低级项。这补上了 INVENTORY 指出的逐行审收据缺口。
2. **逐提交自洽。** C1 只含夹具与设计文档，Rust 未变。C2 的 `tb02d_tests.rs` 与 `tb02c.rs` 以 `include_str!("../../../../s_session/tests/fixtures/tb02d/…")` 引 C1 夹具，路径解析到仓根正确，故 C1 必须在前；C2 可编译、bin 测试红（N3）。C3 的测试不依赖 C4 脚本（导入闭包已核）。C4 依赖 C3 的 client/合同与既有 `tb02a_runtime`、`tb02c_compare`。C5 有 B1。C6 为纯文档与证据副本。需根实跑的清单见 RESULT.json。
3. **CI 改动。** 固定 3.14.6 可取得（`actions/python-versions` 有 `3.14.6-27283001424`）；导入闭包只有标准库与仓内模块；Linux x86-64 上若 `getsizeof` 布局不同，`_layout_matches()` 返回 False，池停用，`test_tb02d_parse_pool.py` 的 4 个方法（`:117`、`:154`、`:169`、`:243`）**失败而非跳过**，诊断行先打印 `False`，属期望行为；按 LP64 同构推断应一致，未实测。前置 3.11 步骤不受影响。唯一的硬错误是 B1。
4. **报告与索引可追溯性。** `EVIDENCE-INDEX.json` 中 168 条带路径收据全部现场存在且 bytes/SHA 相等（`evidence-index-trace.json`，大件 0 条需仅 stat）。`ACCEPTANCE-REPORT.md` 88 个 12 位 SHA 前缀中 84 个落在索引，2 个是 git commit（`f95a7f3`、`efc1ddc4`，后者为基线祖先），2 个 observed 字节（`8fa0d286…`、`59c56bce…`）经索引内 RUN-OC 收据可回绑。三臂 EXIT、RESULT、query-oracle 七 cut、browser、normal-recovery、EXECUTION、CONTROLLER-EXIT、SOURCE-POSTCHECK、DEPENDENCY-POST、COMPOSITE-COMPARISON-RESULT 的数值与报告逐项一致，且与实际独审 RESULT `07495bf9…`、根签收 `c8d6171a…` 一致。D1–D4 写作延后未重算，整体独审、逐 commit 编译、CI、合入写作 PENDING，未称裁整图或关票，observed PASS 未冒充 pure。唯一冒称是 B1 中 §8 对 projection_cache 进 CI 的描述。
5. **run-package 名分。** 判为合宪：宪法 §5 把 review-results 定为绑定票的工作草稿，仓内已有同型先例 `.chanlun/review-results/issue1392-tb02b-20260914/tools/run-faults-r2-original.py`。不会被误当代码执行：pytest `testpaths=["tests"]`，CI unittest 只发现 `s_session/tests`，仓内 CI 无 ruff/mypy 等全仓 Python 检查，CodeQL 停用，`remaining-driver.py`/`remaining-harness.py` 带连字符不可被 import。§1 机械判据会把零调用 `.py` 判孤儿，与 §5 存在字面张力，但 §5 专管 review-results；可选在 run-package 加一行 README 说明。
6. **注释票号与原文回查。** 新增注释均带 `#1414`，无 TODO/FIXME。抽查 `065:44/46`（界 :56 前、无注）、`067:38/46`（界 :62 前、无注）、`067:170/196`（界后，属【答疑】，代码注释未标答疑级，报告与 ORACLE-DESIGN 已标）、`078:54`、`065:20`、`071:42`、`077:62` 逐字可回查；`081:110` 在界 :78 之后，ORACLE-DESIGN 判为续排作者正文，与 `bi.md` 正本一致。未发现伪引文。
7. **范围。** 46 路径 = 36 件 + `ci.yml` + 9 个新文件，均可追到 #1414 的冻结轮次或根裁定，未混入非 #1414 改动。跨片触碰依票面「允许本叶补齐该外部行为必需的同一 R2 owner 支持」，判未越界；N1 提示 strict 语义的实际波及面。
8. **安全。** 无凭据；机器特定值见 N7。

## 已执行的只读检查

字节与状态核对（46/46）；531 项运行来源现值核对；Python 21 件在 CPython 3.11.15 与 3.14.6 下内存编译无语法错误；JS 8 件 `node --check` 通过；6 个改动 Rust 文件 `rustfmt --check --edition 2021` 无差异；`.gitattributes` 不对新增文件启用 LFS；`s_tb02_contract` 等产品模块无第三方导入；EVIDENCE-INDEX 168 条收据回溯；OUT 与 CTRL 小收据数值回读；gh 只读：#1414 票面、远端 main SHA、CodeQL 工作流状态、python-versions 发布标签。
````

## r2 复审全文（逐字）

````markdown
# #1414 / TB-02-D 提交面整体独审复审（r2，增量）

> 名分：独立审查工位 `/root/tb1414_final_surface_review` 对首轮（`final-surface/`，RESULT `24f9332d47cb…`，FAIL_WITH_FINDINGS）的增量复审，工作草稿。只读 worktree，未 stage/commit/push，未构建、未跑测试；只做小文件读、git 只读命令、AST 导入闭包、`actionlint` 与 YAML 解析。

## 结论

**PASS_FINAL_COMMIT_SURFACE_READY_PENDING_RUNTIME_CHECKS。** 首轮唯一阻断 B1 已消除，三件改动字节没有引入新冒称，五提交切分每个提交都能独立成立，其余 43 件与首轮逐字节相同。本轮新增 3 条 LOW 非阻断项，都不涉及提交面上的代码字节。提交面可以进入 S 静止后的根实跑与逐提交编译（见 RESULT.json 清单）；这些实跑收据到达之前，整体验收、CI、合入仍是 PENDING。

## 身份核对

- 新清单 `COMMIT-SURFACE.json`（schema 2）SHA256 `eecc02fdd693ad1ff0bd6965936120d770eff4fa9376474a266467f2cb711a76`，其 `supersedes` 指向首轮清单 `30e3a205…`；`COMMIT-PLAN.md` SHA256 `381e2d735bee98a13fff821f594ef0901d9b2bb1e03ed0acaf5192c8c0c5534f`。
- worktree HEAD 仍为 `f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9`，暂存区为空，`git status --porcelain=v1 -uall` 仍是 46 路径且与清单一致。46/46 件 bytes、SHA256、HEAD 原值和状态全部与新清单相符（`r2/byte-check.json`）。
- 与首轮相比只有三件变化，SHA 与指派一致：`.github/workflows/ci.yml` `5f60ab1b…`（17834 B），`ACCEPTANCE-REPORT.md` `1522aece…`（37300 B），`EVIDENCE-INDEX.json` `eef1c2c8…`（91928 B）。其余 43 件与首轮逐字节相同，35 件运行来源未动。
- 提交归属映射：原 C2 的 8 件与原 C3 的 19 件合为新 C2（27 件），原 C1/C4/C5/C6 依次成为 C1/C3/C4/C5。COMMIT-PLAN 各行文件集与清单逐提交相等（5/27/4/1/9）。

## B1 是否消除

**已消除。**

- `ci.yml` 相对 HEAD 只增 18 行、不删行，与 `ci.yml.proposed.diff` 的增删行逐行相同，`actionlint 1.7.12` 0 条发现，YAML 可解析。新步骤为 `working-directory: s_session/tests`，执行 `python -B -m unittest -v test_tb02d_contract test_tb02d_discovery test_tb02d_validate_once test_tb02d_parse_pool test_tb02d_key_reuse`。`s-observer` job 没有 `defaults`，因此该相对目录以仓根为基准。
- 五个模块的静态导入闭包只有 `test_tb02c_contract.py`、`s_readonly_server.py` 与五者本身，外加标准库，没有未解析的第三方依赖。它们和 `test_tb02c_contract.py` 都不含 `BINARY`、`QueryTests`、`tb01c_query_tests` 或 `--binary`。仓内路径都由 `Path(__file__)` 推出，不依赖 cwd；同目录兄弟模块靠 `-m unittest` 放进 `sys.path` 的 cwd 解析。诊断行的 `sys.path.insert(0, '..')` 指向 `s_session`，`_query_integrity()` 按 `__file__` 加载，可以运行。导入的名字中没有 TestCase 子类，不会重复收集测试。
- 注释已更正：`ci.yml` 第 290–297 行写明五个模块只用标准库、仓内模块与 node，并写明 projection_cache 因 `setUpClass` 需要 `--binary` 写者二进制、本 job 不构建 Rust 而不进 CI，由本机单独实跑留收据。
- 报告已更正：§8 照实写明 projection_cache 未进 CI 及原因，行号 `:12`、`:17`、`tb01c_query_tests.py:44-48/:60/:29`、`:331-332`、`:333` 均与源码一致，并承认上一版说法错误、由 FS-B1 查出；§10 增列“未进 CI，只有本机 `--binary` 收据可作锁，待补”；§11 与 `EVIDENCE-INDEX.json` 的 `ci.not_in_ci` 同步写为 PENDING。
- `--binary` 入口的限制：§8 写明该入口“在 `:333` 只选 `test_projection_*` 方法”，事实正确，见 R2-N1。

## FS-N1–FS-N8 披露

报告 §10 与索引 `limitations` 逐条照录了首轮 FS-N1 到 FS-N8，严重级、文件行号与首轮 RESULT 一致。FS-N3 的处置写为“原 C2 与 C3 合并”，这与新切分相符。FS-N1、FS-N2、FS-N4 写明留给根实跑或后续票，没有写成已解决。报告给实际运行独审的编号加 `AR-` 前缀，给首轮独审的编号加 `FS-` 前缀。正文中未带前缀的 `N1–N8` 已全部替换，索引另设 `finding_id_prefixes` 作说明，唯一遗漏见 R2-N2。

## 五提交逐个自洽

- **C1（5）**：夹具、生成器与设计文档，不被任何测试收集器匹配，Rust 未变，基线可编译。
- **C2（27）**：`tb02d_tests.rs` 与 `tb02c.rs` 的 `include_str!` 引用 C1 夹具，C1 在前，依赖已满足。`tb02c.rs:388-435` 与 `s_structure_session.rs:4345/4445` 起 `python3` 调用的 `s_tb02_contract` 同在本提交，首轮 FS-N3 的中间红因此不复存在。本提交的 Python/JS 测试只依赖本提交文件与基线文件，不依赖 C3 的运行脚本，也不依赖 C1 夹具（已 grep）。projection_cache 依赖的 `tb01c_query_tests.py` 是基线文件。是否可编译须按 G5 实测。
- **C3（4）**：运行脚本依赖 C2 的读口/client、C1 夹具，以及基线 `tb02a_runtime`、`tb02c_compare`、`s_service_control`，都已在前面的提交或基线中。Rust 未变。
- **C4（1）**：CI 引用的五个模块都在 C2，Rust 未变。
- **C5（9）**：两份报告与 run-package 七件，纯文档与证据副本，适用豁免。

COMMIT-PLAN 声明 C1–C3 共 36 件，加 C4、C5 共 46 件，与实际一致。

## 新冒称检查

报告新增的事实都核实无误。首轮的 REVIEW、RESULT、BINDING 三件 SHA 前缀正确。“46 件字节与清单一致”“逐行审只发现 FS-N5–N7”“`test_tb02d_contract.py:46` 起 node”“parse_pool 四处断言 `:117/:154/:169/:243`”“CodeQL disabled_manually”都与首轮收据或源码一致。D1–D4 仍写延后未重算；整体复审、本机实跑、逐 commit 编译、CI、合入都写 PENDING；没有称裁整图或关票。`EVIDENCE-INDEX.json` 中 175 条带路径收据全部现场存在，bytes/SHA 相等（`r2/evidence-index-trace.json`），其中包括新增的首轮独审 6 件收据。

## 本轮新增非阻断发现

**R2-N1 · LOW · `--binary` 入口只覆盖 14 个自有测试，这一点写得不够显式。** §8 只说入口“只选 `test_projection_*`”，没有写出数量，也没有说明从 `tb01c_query_tests.QueryTests` 继承来的 31 个测试不经此入口运行；`ci.yml` 注释和索引 `ci.not_in_ci` 也没提这一点。#1414 在 C2 中改了 `s_query.py`、`s_query_integrity.py`、`s_readonly_server.py`，而那 31 个 TB-01-C Q 测试只有本机 `tb01c_query_tests.py --binary` 一个入口，不在 CI。在控制区小文件里我没有找到它们对最终字节的运行收据。建议根把它加入实跑清单（已列入 RESULT）。补写这条说明属于纯文档改动，可在落盘终审时一并完成。

**R2-N2 · LOW · 索引 `limitations` 第 2 条仍写“（N1）”，没有带前缀。** 按内容它指浏览器 30 秒余量，应为 `AR-N1`；报告正文已正确写为 AR-N1。是否改由根决定，改动属于 C5 纯文档。

**R2-N3 · LOW · 提交面之外：`PR-DRAFT.md`（`d77d3002…`）已过期。** 审查顺序仍是六段式，验证段仍写“实际独审 PENDING”，也没有反映 projection_cache 未进 CI。G6 开 PR 前须按五提交和现状重写。

首轮 FS-N1 到 FS-N8 仍然有效，按报告 §10 的披露处理。
````
