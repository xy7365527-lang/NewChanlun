# #1414 / TB-02-D 验收报告

> 名分：工作草稿，活期绑定 [#1414](https://github.com/xy7365527-lang/NewChanlun/issues/1414)（`docs/agents/generation-constitution.md` §5）。状态：`LIMITED_ACCEPTANCE_RUNTIME_CHECKS_PASS_WITH_KNOWN_BASELINE_EXCEPTION_PENDING_CI_AND_MERGE`——attempt 2 运行事实的实际独审为 `PASS_WITH_DEFERRED_LARGE_REHASH`，**TB-02-D 有限验收成立**，已由根全文接受；**不裁整图**。覆盖最终提交面的整体独审首轮 FAIL（唯一阻断 FS-B1 属 CI 接线），修正后复审 r2 为 `PASS_FINAL_COMMIT_SURFACE_READY_PENDING_RUNTIME_CHECKS`。S 静止后的本机实跑已完成，结论 `PASS_WITH_KNOWN_BASELINE_EXCEPTION`：唯一失败是 main 自 `d8810bff97`（#1392）起既有的 3 个 Rust 库测试红，无 #1414 新增失败；大件重算 D1–D4 为 `PASS_WITH_BASELINES`（§11）。逐 commit 可编译性、PR 上的 CI、合入 main 与关票仍 PENDING。历史 FAIL 按原收据原名分保留。
>
> 证据定位：每份仓外证据写成「归档根:相对路径」加 SHA256 前 12 位，完整 bytes/SHA256、逻辑 ID 与历史定位见同目录 `EVIDENCE-INDEX.json`（下称“索引”）。`CTRL-20261012` 是控制与准备工作区，`OUT-REMAINING-20261013` 是 attempt 2 的运行输出；两者都在仓外，本机绝对路径只在索引里作历史定位。

## 1. 结论

**TB-02-D（#1414）有限验收成立；实际独审时延后的大件重算 D1–D4 已在本机补跑（`PASS_WITH_BASELINES`）；不裁整图。** 依据是对 attempt 2 三 fresh 与两对比较的实际独审 `CTRL-20261012:tb1414-explicit-composite-review/actual-remaining/`：REVIEW `87a739e10708`、RESULT `07495bf945fe`、BINDING `10db30c823db`，`status=PASS_WITH_DEFERRED_LARGE_REHASH`，`tb02d_limited_acceptance=ACCEPTED`，阻断项 0，非阻断项 AR-N1–AR-N5（本报告给实际运行独审的编号加前缀 AR-，给整体独审首轮的编号加前缀 FS-）。根已全文接受该独审：根签收 ROOT-ACTUAL-ACCEPTANCE（`c8d6171af117`，2026-10-13T01:10:27Z，`ROOT_ACCEPTED_TB02D_REMAINING_ACTUAL_WITH_DEFERRED_LARGE_REHASH`）。该独审的范围写明是“三 fresh UP/b→DOWN/a→DOWN/b 与两对 221 原字节比较；不裁整图”。

D1–D4 是审查者对大于 1MiB 大件的复验延后，不是运行内门的缺口：本审没有亲自重算这些大件的 SHA，实际独审时它们只由运行内原门当场计算的收据和小收据间接支撑（§7.10）。S 静止后已按根授权补跑，结果为 `PASS_WITH_BASELINES`：825 件中 813 件 PASS，12 件只记录首个基线（BASELINE_RECORDED），0 件 FAIL（§7.10）。

覆盖最终提交面的整体独审首轮 `CTRL-20261012:tb1414-explicit-composite-review/final-surface/`（REVIEW `1149a14bd331`、RESULT `24f9332d47cb`、BINDING `47caee912385`）为 `FAIL_WITH_FINDINGS`：概念层通过，46 件字节与清单一致，逐行审补齐了 G 轮运行脚本与离线探针的审阅缺口；唯一阻断 FS-B1 是新 CI 步用 `discover` 会收进需要 S 写者二进制的 `test_tb02d_projection_cache.py`，在不构建 Rust 的 job 里必然 ERROR，而 `ci.yml` 注释与本报告 §8 却把它说成已进 CI。已按根选定修法改为显式列出五个不需要二进制的模块（§8），测试文件未改，35 件运行来源字节不变。非阻断 FS-N1–FS-N8 作为已知限制写入 §10。

同一独立工位的增量复审 r2（`CTRL-20261012:tb1414-explicit-composite-review/final-surface/r2/`，REVIEW `3165f21d94fb`、RESULT `c055481d64a9`、BINDING `f6327077651c`）结论为 `PASS_FINAL_COMMIT_SURFACE_READY_PENDING_RUNTIME_CHECKS`：FS-B1 已消除，三件改动字节没有引入新冒称，五提交逐个自洽，其余 43 件与首轮逐字节相同；新增三条 LOW 非阻断项 R2-N1–R2-N3，均不涉及代码字节，已在本报告、索引与 PR 草稿中处理。两轮审查报告全文逐字收录在本目录 `FINAL-INDEPENDENT-REVIEW.md`。r2 的含义是提交面可以进入根的本机实跑与逐提交编译。

本机实跑已在最终字节上完成（`CTRL-20261012:tb1414-runtime-checks/RUNTIME-CHECKS-RESULT.json`，`2f7d2f3f391b`），结论 `PASS_WITH_KNOWN_BASELINE_EXCEPTION`：r2 清单所列 8 项全部执行，TB-02-D 新 CI 步、`--binary` 两组、s-observer 原步骤、`cargo fmt` 与三项 `cargo check`、s_session bin 全量、生成器再生与 D1–D4 都通过；`cargo test` 只有库测试中的 3 个失败，与基线 `f95a7f3` 上逐字相同，属 main 自 #1392 起的既有红，已在 #1392/#1404 关票评论登记为获批基线例外（§11）。整体验收的其余部分——逐 commit 可编译性、PR 上的 CI、合入 main 与关票——仍 **PENDING**；合入与关票需要用户对具体 PR 号与 head SHA 的批准。

| AC（票面原文缩写） | 主要证据 | 实际运行独审（已接受） | 本机实跑（最终字节；CI 与合入 PENDING） |
|---|---|---|---|
| AC1 第二序列取正确同向笔、接受规定分型；独立包含与 strict 封闭按已裁语义 | 手算 14 个双向工作例与 `tb02d_tests.rs`；core 独审 ADOPT-A-REVIEW（`93c23702a28d`）；三臂七 cut oracle；浏览器 CC-012 含 FIRST_SEQUENCE/SECOND_SEQUENCE/matched | 运行层成立：三臂 oracle 七 cut 取值一致且 passed；D1–D4 延后 | 无新增失败：#1414 新增的 5 个 `tb02d` 库测试随全量通过；夹具再生逐字节相同；D1–D4 重算 `PASS_WITH_BASELINES` |
| AC2 只用完整已知前缀或有效终结证据；不设新扫描 cap；不以预算耗尽否定 | `false` / `null + scan_complete=false` 的区分（§3）；core 独审 | 不在实际运行独审范围（属代码与合同裁定） | TB-02-D 五模块 35 个测试与 `--binary` 两组 14+31 个测试通过；`cargo test` 只有 3 个既有红（§11） |
| AC3 笔破坏与段破坏各自并存；一般扫描、超 50 与 continuation 归 TB-02-W | `fact_vector`；浏览器 CC-014 含 fact_vector，cut27 developing 同时含 stroke_broken 与 segment_broken | 运行层成立：三臂浏览器业务断言全部通过；D1–D4 延后 | s_session bin 81 passed/1 ignored；D3 中 804 件候选与响应大件 PASS，HAR/DB/WAL/备份 12 件只记基线 |
| AC4 正式会话入口运行非真空输入；读公共查询/变化；浏览器核对 | 三臂各 40 输入、124 个公共/恢复查询、372 份 Q 收据、浏览器 cut27/40 | 运行层成立：三臂 exit 0，Q 收据全为 `captured_validated_commit`，`page_errors=[]`；D1–D4 延后 | s-observer 原步骤与 `--binary` 两组通过；D1 二进制与三臂快照 4 件 PASS |
| AC5 独立 oracle；同输入新进程重放与所改持久边界恢复对拍 | hand-oracle（`9140b7827d95`）；正常恢复；两对 `compare_exact_traces` | 运行层成立：恢复 equal、epoch 1→2；两对 221 记录逐字节 PASS；大件 core 的全量重算属 D2，延后 | 生成器两次自检一致、再生三件逐字节相同；D2 锚 core 与三份 fresh core 重算 PASS |

整体独审首轮没有逐条给出 AC 结论。它的逐行审写明：`false` 与 `null + scan_complete=false` 的区分、终结证书准入条件与 JS/Python 合同一致，未引入新扫描 cap；`break_facts`（`segment.rs:317`）按 065:44/46 枚举正确。这些与 AC2、AC3 相关。复审 r2 判提交面通过，同样没有逐条裁 AC。最右栏只记本机实跑在最终字节上补到了哪些证据，不是对 AC 的新裁定；PR 上的 CI 与合入批准仍 PENDING。

## 2. 范围

本报告只覆盖 #1414 已裁的有限完整前缀域：`hand-oracle.json` 的 14 个双向手算工作例，以及 `second_open_up`、`second_open_down` 两个 40 根自然 OHLC 输入（首次 SECOND 在 cut39，最终 cut40，恢复核 cut27/40）。

不覆盖：[#1344](https://github.com/xy7365527-lang/NewChanlun/issues/1344)/TB-02-W 的一般扫描窗口、51 项以上与 continuation；CC012/013/014 全轴（目录仍声明 `not_implemented`）；G006 广泛桥；TB07 全历史恢复；父票 [#1360](https://github.com/xy7365527-lang/NewChanlun/issues/1360)、SPEC [#1340](https://github.com/xy7365527-lang/NewChanlun/issues/1340)、图 [#1323](https://github.com/xy7365527-lang/NewChanlun/issues/1323) 的整项义务。本交付合入不关闭这些票。

## 3. 交付的行为变化

基线 `f95a7f3ebc0c` 上，S 只发布 TB-02-C 的线段 seed 与第一种终结（规则 `segment-seed-first/1`），`SegmentCandidateFact` 没有 SECOND、strict 或破坏事实向量字段；需要第二特征序列才能判定的终结无法形成，也无从在 Q 和浏览器里观察。

**结构核心（Rust）。** 同一个 `FeatureSeqState` 产出 SECOND 所用的原段同向笔、独立的方向性包含步骤、任意分型三元组与实际扫描前沿；全量与增量走同一状态机。FIRST 首个有效窗口按 b 的代表笔身份冻结，后来的 c 包含合并不追溯改写第一笔封口，新 b、skip、reset 不继承旧证书。`SegmentCandidateFact` 新增 `second`、`strict`、`fact_vector`，几何段端与真正终结前沿分开；CC014 笔破坏独立枚举，段破坏只来自同次 strict/SECOND、完整 construction 与最小笔数准入证书。完整当前前缀未命中记 `false`；配置的 SECOND 窗口耗尽而无匹配记 `null` 且 `scan_complete=false`，不当作否定。规则版本 `segment-second/1`，typed 合同 `s-segment/2`；旧 C cut 仍按原规则严格读取。

**合同与 Q 读口（Python/JS）。** 合同校验 SECOND 来源、strict、证书与扫描可知性。新增 `GET /api/discover`：同一次完整存储核验后只返回协议、会话/化身与明确 profile，拒绝查询参数；页面与 HTTP 采集器据此进入 v2 固定分页，发现失败不降级到 v1，只有已核 legacy 标记才走 v1。`/api/state` 保留。

**同语义成本削减。** 真实 40 输入运行撞上原预算门后逐项加入，原门一个不放宽（verify 5 秒、联合 128MiB、单 Q 30 秒、浏览器业务 10 秒及外层 30+5 秒）：分页完整投影单槽缓存、`project_fact` 一次完整核验、canonical 单编码、同次 wire 复用、identity 片段、同次解析池（**仅在 CPython 3.14.6 / 64 位实测 dict 布局匹配时启用，其它解释器自动停用**）、前代同值 canonical key 复用；浏览器侧 Snapshot 删 5 行重复校验、严格递归 clone、meta canonical 懒复用与 UTF-8 码点比较。

**运行脚本。** `s_session/tests/tb02d_{harness,runtime,verify}.py` 与 `tb02d_browser.cjs` 经正式 S/Q/Watch 入口逐前缀运行。整臂调度窗由 300 秒改为 900 秒，属 harness 调度界（来源独审 ADOPT-G-ARM900-AUTHORITY，`2a94e22f85eb`），不是产品性能承诺；单步、资源、矩阵与清理门不变。

**跨片触碰。** 本交付修改了 TB-01-C 的 Q/浏览器文件与 TB-02-C 的若干文件，依据是票面“允许本叶补齐该外部行为必需的同一 R2 owner 支持”；没有这些改动，真实 40 输入运行会在 `/api/state` 8MiB 读口、verify 5 秒和浏览器 10 秒门上失败。整体独审首轮判其在票面 R2 owner 条款之内、未越界，并提示 strict 语义的实际波及面（FS-N1）。

## 4. 基线、来源与提交面

基线 commit `f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9`（PR #1473 合入 TB-02-C 的 merge），分支 `codex/1414-second-sequence-20261012`。已签来源是仓内 `.chanlun/review-results/issue1339-r2-spec-20260909/`（落于 `efc1ddc4d6c0`，是基线祖先），规范清单 SHA256 `38e3176b6dff`。

代码与夹具面是 36 个路径（18 个修改、18 个新增），逐件 path/bytes/SHA256/HEAD SHA256 见索引 `reviewed_source_snapshot.changed_files`。其中 35 件与 attempt 2 冻结的运行来源集 SRC-CURRENT（`CTRL-20261012:tb1414-explicit-composite-preparation/CURRENT-SOURCES.json`，`6e984f6b8c58`，555 项 = 531 个正式仓内路径 + 18 个 ijson 依赖文件 + 6 个运行包 helper）逐项相等，实际独审独立重算确认；唯一不在集的是 `ORACLE-DESIGN.md`（工作草稿文档，不参与运行）。

最终提交面在 36 件之外还有：`.github/workflows/ci.yml`（TB-02-D Python 测试接线，§8）、本目录 `ACCEPTANCE-REPORT.md` 与 `EVIDENCE-INDEX.json`、`run-package/` 七件证据副本（§9），以及逐字收录两轮整体独审报告的 `FINAL-INDEPENDENT-REVIEW.md`；共五个提交、47 个路径。提交后须用 `git show <commit>:<path>` 的字节逐项重核；若任何代码或夹具字节与已跑来源不同，已跑 fresh 证据不再覆盖该字节，须由根裁定补跑或映射独审。

两对比较所用的 `compare_exact_traces` 在基线已入仓：`s_session/tests/tb02c_compare.py`（`96c7bb453aa2`），本交付未改动；实际独审确认导入路径上没有同名模块，只能解析到这份原件。

## 5. 独立 oracle

`s_session/tests/fixtures/tb02d/` 下的 `hand-oracle.json`（`9140b7827d95`，14 个双向工作例）、`raw-ledger.json`、纯整数 OHLC 生成器 `generate-input.py` 与 `MANIFEST.json`（`f056451baa3a`）先于实现冻结，设计说明见本目录 `ORACLE-DESIGN.md`。core 独审 ADOPT-A-REVIEW 引用了该 MANIFEST。oracle 不由实现输出反写。

## 6. 代码与采纳独审（已发生）

每个文件只归到其当前字节首次出现的轮次；逐文件对应见索引 `changed_files[].adoption_round`。

| 轮 | 内容 | 收据（逻辑 ID，SHA 前 12 位，记录状态） |
|---|---|---|
| A | 核心 SECOND/strict/事实向量与 TB-02-C 适配（11 件） | ADOPT-A-REVIEW `93c23702a28d`；ADOPT-A-CHECKS `95f384e889ef`（逐字含 11 件当前 SHA） |
| B | 独立 oracle 夹具与设计 | 由 ADOPT-A-REVIEW 引用 MANIFEST |
| C | `GET /api/discover` | ADOPT-C-REVIEW `2e7aa4579815`；ADOPT-C-RESULT `90092e9daeac` PASS |
| D | 分页完整投影单槽缓存 | ADOPT-D-REVIEW `08e0575f8035`；ADOPT-D-BINDING `f8b098887742` |
| E | Snapshot 删 5 行重复校验 | ADOPT-E-REVIEW `4898f724d180`（含 `index.html` 当前 SHA） |
| F | 严格 clone | ADOPT-F-REVIEW `0570678cc1cd`；ADOPT-F-BINDING `2ab1e84b6a9b` PASS |
| G | 正式运行脚本 | 早期版 ADOPT-G-EARLY-REVIEW `af97b58105bb`；定版身份 ADOPT-G-READY-REVIEW `1d206d8b8322`；调度窗 ADOPT-G-ARM900-AUTHORITY `2a94e22f85eb` |
| H | 累计五文件正式采纳 | canonical `d16333af5572` PASS_CODE_CORRECTNESS_ONLY；identity `78ff4af8c4a9` PASS_CODE_CORRECTNESS_ONLY；根决定 `10df2090721b` ROOT_APPROVED_FORMAL_FIVE_FILE_ADOPTION；补丁独审 `9961036739bc` PASS_SCOPED_PORTABILITY_AND_CUMULATIVE_PATCH；MANIFEST `b34d8fb2f021`；映射独审 `a902a6406413` PASS_FORMAL_FIVE_FILE_MAPPING |
| I | browser readiness runner（pure 版） | 静态独审 `f82a61d334b0` PASS_STATIC_BROWSER_READINESS_CANDIDATE |
| J | pure codepoint Client | 静态 `0c1c8157cd65`；有限 `c40fdca3b632` PASS_FINITE_LOCAL_COST_OBSERVED；I+J 采纳授权 `ec8d19958260`、绑定 `03053f40f22a`、后核 `557eb459947f`、映射独审 `9e7833a87408` PASS_VERIFIED_EXACT_TWO_FILE_ACTUAL_ADOPTION_MAPPING_NOT_RUNTIME_ACCEPTANCE（只证映射，不证运行） |

已知审阅缺口：G 轮三件运行脚本与 E 轮 `test_tb02d_snapshot_validate_once.cjs` 的**当前精确字节**此前只在来源身份核与传递校验里出现，没有逐行代码审收据。实际运行独审读了 `tb02d_browser.cjs:62-83` 与 `tb02d_runtime.py:75-128` 的相关段落，但它不是逐行代码审。整体独审首轮已逐行审这些当前字节（连同 `test_tb02d_strict_clone.cjs` 与 `tb02d_browser.cjs`），只发现 FS-N5–FS-N7 三条低级项，补上了该缺口；复审 r2 判提交面通过。

## 7. 执行与继承谱系

### 7.1 失败史（原名分保留，不改写、不翻案）

| 运行 | 结果 | 收据 | 后来被采用的部分 |
|---|---|---|---|
| 正式 r1 | FAIL：cut31 `/api/state` 503 QueryBudgetExceeded | 原件在外卷，索引未绑定 SHA | 无 |
| 正式 r2 / r3 | FAIL：300 秒窗内 35 输入 / 首臂 96Q | 同上 | 无 |
| 正式 r4（窗 900） | FAIL：120Q 后 collect_core 30 秒窗超时 | RUN-R4-OUTCOME `186cd22ea5ea` | 120Q/80 收据经补充继承 |
| r4 首臂补充 | FAIL：219 记录后 Q `/api/ready` 503 | RUN-R4S-REVIEW `738027374c2a` | 219 行 core（后被 strict221 取代） |
| 冷 40 只读 verify 诊断 | canonical/identity/parse-pool 三次 FAIL；key-reuse 首次通过原 ready 门 | `851b59eb74d8`、`c0dcc6998f62`、PLAN `38a768b522f5`；parse-pool 收据未能回绑 | 支撑 H 轮采纳决定，不是矩阵验收 |
| key-reuse 补充 r1 | FAIL_VERIFIED_NEW_RUNTIME_METADATA_INTERFACE | RUN-KR1-REVIEW `68672dcf28ec` | 无 |
| key-reuse 补充 r2 | 整体 FAIL（浏览器 networkidle 超时） | RUN-KR2-REVIEW `89d3138d7de1`；CHECK `cd9679367e63` | 4 个真实 Q 完整 raw 对拍、S epoch1→2 正常恢复、七 cut oracle |
| browser-only completion | 整体 FAIL（浏览器完整提交超时） | RUN-BO-REVIEW `02f9da493f04`；CHECK `ad2296b9f4c8` | strict221 core、sealed DB/WAL/SHM、17 表与四恢复转移 |
| hoist + 观测 completion | FAIL：final40 wait10000 超时；Chrome post 跨期身份 FAIL | 原件在外卷，索引未绑定 SHA | 无 |
| observed codepoint completion | PASS，**仅属 observed Client `8fa0d2868e4c` 与 runner `59c56bce97a6`** | RUN-OC-RESULT `8cf04d06a070`；RUN-OC-REVIEW `636341d09b1b` | 首 composite 的浏览器层 |

observed 版字节没有进入正式 worktree；它的浏览器 PASS 不能改称本交付的 pure 字节已通过。

### 7.2 显式首 composite

四层 composite（key-reuse r2 的 4Q/恢复/7cut、browser-only 的 strict221 与 sealed 三件、observed 浏览器、I+J 两文件采纳映射）的 payload 是 COMP-DRAFT（`d08eb70651f8`），core 为 `c0dbe0db7e66`、1456959869 字节、221 条记录，其中继承重构 139 条、新重构 0 条，`whole_new_single_run_all221_collected_claim=false`。aggregate 独审 `91e91ca8215a` VERIFIED_COMPLETE_COMPOSITE；三来源映射 `1aa99a727837` 及其独审 `f740e3b822e1` PASS_EXACT_THREE_SOURCE_MAPPING；根 detached release `13c68f4881c9` ROOT_ACCEPTED_EXPLICIT_FIRST_COMPOSITE，明写不是 pure 运行验收、不是执行授权、不是 TB-02-D 完成。

### 7.3 remaining 包与 attempt 2

最终包：PLAN `1e5d62f9a9ee`、FINAL-PACKAGE-BINDING `a51e1fff57dc`（逐个绑定 6 个 helper 的 SHA）、最终包独审 r3 `541d1d721064` PASS_FINAL_REMAINING_PACKAGE_READY_FOR_ROOT_AUTHORIZATION（绑定 SRC-CURRENT）。attempt 1 在依赖预检失败，无 OUT、无产品副作用，授权已消耗且未被重用（`ee8132217021`）。attempt 2 根授权 `9a027504f1c8` ROOT_APPROVED_REMAINING_MATRIX_ONCE，启动记录 `4ad159d59771`。实际独审核得 OUT `freeze/` 的 9 件副本与准备目录现件及授权所绑 SHA 逐件相同，PACKAGE-BINDING 14 项现场重算一致；时序为 slot 现场核 00:18:37Z、根签名 00:19:02Z、启动 00:19:16Z、结束 00:38:12.985Z，OUT 内没有晚于运行结束的文件。

attempt 2 于 2026-10-13 08:19（+08:00）启动、08:38 结束：controller `exit_code=0`、已 wait 并回收，单调时钟 1136.98 秒（CONTROLLER-EXIT `470ec8ed4168`）；`OUT-REMAINING-20261013:EXECUTION.json`（`a1c811c58707`）列三臂完成、`cause=null`、1136.47 秒。

### 7.4 三 fresh（运行事实，经实际独审核验）

路径相对 `OUT-REMAINING-20261013`，逐件 SHA 见索引 `execution.arms`；耗时一栏取自实际独审 RESULT。

| 字段 | `second_open_up-b`（端口 18982） | `second_open_down-a`（18983） | `second_open_down-b`（18984） |
|---|---|---|---|
| 臂 `EXIT.json` | `{"exit":0}` | 同左 | 同左 |
| 输入 / core 记录（`evidence/RESULT.json`） | 40 / 221 | 40 / 221 | 40 / 221 |
| 公共/恢复查询；rejections | 124（public 120 + 恢复 4，deadline 30）；0 | 同左 | 同左 |
| Q 收据 | 全为 `captured_validated_commit`、full_state_installed、sources_unchanged | 同左 | 同左 |
| 七 cut oracle（`evidence/query-oracle.json`） | passed：15/26 NO_FIRST；27/31/38 PENDING_CASE_TWO；39/40 CASE_TWO | passed，同左 | passed，同左 |
| 正常恢复（`evidence/normal-recovery.json`） | equal，stopped→recovered→ready，epoch 1→2，cut27/40 | 同左 | 同左 |
| 浏览器（`evidence/browser/RESULT.json`） | passed，cut 27/40，`page_errors=[]`，Chrome 154.0.8037.99 | 同左 | 同左 |
| 整臂 / 单 Q 最大 / collect_core 上界 / oracle | 351.4s / 7.15s / 31.3s / 7.5s | 357.4s / 7.34s / 34.9s / 7.7s | 358.6s / 7.28s / 34.4s / 7.8s |
| 运行时清理错误；`WRAPPER-CLEANUP.json` | `[]`；passed（进程不在、端口 errno 61、socket 不存在） | 同左 | 同左 |

三臂共 372 份 Q 收据全部核过，每臂 40 条 ingest 事件 operation 0..39 全为 Received，控制事件 6 条 exit 全为 0。整臂 900、collect 120、oracle 180、单 Q 30、浏览器 30 秒各帽都由运行时硬性执行，均未超出，浏览器超时回收分支未触发。七 cut oracle 的回执作用域是“frozen named cuts of actual Q；full natural prefix traces separately captured and compared”，即命名 cut 的结论来自实际 Q，完整前缀轨迹另由 §7.5 比较。

### 7.5 两对原字节比较

`OUT-REMAINING-20261013:COMPOSITE-COMPARISON-RESULT.json`（`0d0624c92327`）`status=PASS`，方法 `exact_bytes_and_strict_jsonl`，配合 harness 的 `expected_layout(40)`（221 行：ingest 40、receipt 40、public 120、表 17、schema 1、lifecycle 1、恢复 cut 27/40）。

| 对 | 记录 | 比较字节 | excluded_fields | identity_renaming | 两侧 SHA256 | 回执 |
|---|---|---|---|---|---|---|
| second_open_up：首 composite ↔ UP/b | 221/221 | 1456959869 | `[]` | `[]` | 两侧均 `c0dbe0db7e66…` | PASS |
| second_open_down：DOWN/a ↔ DOWN/b | 221/221 | 1461830301 | `[]` | `[]` | 两侧均 `bc6b6afd4502…` | PASS |

实际独审确认 `verify_comparison_binding` 要求 UP 一对左侧 SHA 与比较字节等于首 composite 封值（`c0dbe0db7e66`/1456959869B，与 COMP-DRAFT、根 release、aggregate 独审的 core 一致），锚 core 的 lstat 五字段仍等于已审 stamp，三件 fresh core 的大小等于 compared_bytes。四份 semantic-core 的全量重算属 D2，延后。

范围提示：UP 一对的左侧是根已接受的四层 composite，其浏览器层属 observed 字节；pure Client 在 `second_open_up` 上的浏览器 PASS 只来自 UP/b。

### 7.6 浏览器证据

runner `tb02d_browser.cjs`（`a3d32f0be360`）只有在全部业务断言通过后才写 `browser/RESULT.json`（`:62-83`）。实际独审读了每臂五份 txt，以下字符串全部在场：终 cut 40 RecomputedWithRevision 已提交；CC-012 含 FIRST_SEQUENCE、SECOND_SEQUENCE、matched；CC-013 含 CASE_TWO、geometric_end、terminated_known_at；CC-014 含 fact_vector；切到 cut 27 AsKnown 后 developing 含 PENDING_CASE_TWO、stroke_broken、segment_broken；errors 为空。READINESS 记录 failure=false、cleanup_errors 0、pageerror 0。

| 臂 | 浏览器进程窗（mtime）/ 内计 | 距 30 秒帽余量 | requestfailed（非门项） |
|---|---|---|---|
| UP/b | 28.94s / 28747ms | 1.06s | 24 |
| DOWN/a | 29.35s / 29166ms | 0.65s | 17 |
| DOWN/b | 29.29s / 29108ms | 0.71s | 25 |

每臂五张截图（SHA 前 12 位）：

| 截图 | UP/b | DOWN/a | DOWN/b |
|---|---|---|---|
| CC-011.png | `972f07195601` | `86a141f51e61` | `86a141f51e61` |
| CC-012.png | `9455cd64e977` | `701bff479fc5` | `701bff479fc5` |
| CC-013.png | `509c7e78ebb0` | `e404a1a50a26` | `e404a1a50a26` |
| CC-014.png | `ae216559427b` | `919fdfb5c9b2` | `919fdfb5c9b2` |
| developing.png | `ce97f1f64d5a` | `305277df5dbc` | `305277df5dbc` |

DOWN/a 与 DOWN/b 的五张截图逐字节相同。每臂 `network.har`（约 41MB）未打开、未重算，属 D3。

### 7.7 pure 来源身份与后核

三臂 `source-before-run` MANIFEST 各 105 项，等于 RUN-PLAN 的 102 项来源加 3 项 prepared，后者又精确等于 SRC-CURRENT 中 s_session 运行时子集加二进制；其中 Client `083f424d9ad9`、runner `a3d32f0be360`、runtime `e17fd1f6fecc`，即正式 worktree 的 pure 字节。实际独审逐臂重算 104 份 ≤1MiB 快照字节全部一致，对 SRC-CURRENT 中 554/555 个 ≤1MiB 成员与 141/141 份 frozen evidence 重算全部一致（剩下 1 件是 4254944 字节的 S 二进制，属 D1）。

`SOURCE-POSTCHECK.json`（`2389a0194385`）：来源 555 项、证据 141 项，各类失配全为空。`DEPENDENCY-POST.json`（`3b0cd8e2f32a`）`status=PASS`（前核 `1cacbc3933e4`）；python、node、playwright 三项 full receipt equality，Chrome 按事前声明的 `named-chrome-object-content-endpoints/1` 合同核验，不宣称整 bundle 已验证。

### 7.8 两处现象的解释（实际独审结论，非阻断）

**`evidence/RESULT.json` 的 `not_evaluated`（AR-N2）。** 两个字段的唯一写入点是 `tb02a_runtime.py:258-259` 的初值，tb02d 路径不改写。Tb02dRun 在 `collect_core → complete_after_core`（`tb02d_runtime.py:75-128`）中依次执行正常恢复、frozen oracle 与真实浏览器，任一异常都会使 status 置为 FAIL（`tb02a_runtime.py:282-283`）；status 为 `CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER` 恰好意味着三项已在同一进程内执行并通过，只是没有回填标签。成功门按矩阵读取 query-oracle、browser、normal-recovery 叶子与双跑比较，全部通过。交付文案不单独引用这两个字段；若要让 RESULT 自洽，须走新的采纳修订，本次不改。

**PID 复用警告（AR-N3）。** 停止前按 uid + lstart + 完整 command 双核身份后才对本 PID 发信号；警告只在退出瞬间 ps 文本变化或进程观测不可用时记录并继续观察，stopped 只由 `ps -p` 无行或僵尸态判定，方向是失败关闭。wrapper 走 status 早返回路径、未补停；审查者升级只读复核 18982–18984 均 errno 61、socket 不存在、无本运行残留进程（LIVE-RESOURCE-CHECK `18b2f1007ec2`）。与 S86 supervisor 审 H1 不是同一失效模式。残余风险（wrapper 只核 PID、校验与 kill 间的固有微窗、浏览器成功路径不 killpg、措辞误导）不阻断，建议后续包处理。

### 7.9 实际运行独审的非阻断发现

| ID | 发现 | 建议 |
|---|---|---|
| AR-N1 | 浏览器 30 秒帽余量只有 0.65–1.06 秒，与 observed 运行的 29.14 秒同属系统性；诊断 128 个事件止于约 8.3 秒、丢弃 244 个，看不到耗时去处 | 任何重跑或交付 CI 前先定位耗时；帽值不得静默放宽 |
| AR-N2 | RESULT.json `not_evaluated` 为过时标签 | 见 §7.8 |
| AR-N3 | PID 警告措辞与残余同族风险 | 见 §7.8 |
| AR-N4 | 浏览器 requestfailed 24/17/25，均为 `/api/v2/snapshot` 在 200 响应之后；非门项，无 pageerror，与 observed 运行的 22 次同型 | 成因需打开 HAR，未开 |
| AR-N5 | Chrome 主文件 ctime 前后变化 | 在已声明合同之内；整 bundle 未验证 |

### 7.10 延后的大件重算（实际独审时 DEFERRED_LARGE_REHASH；本机已补跑，PASS_WITH_BASELINES）

下列四项是审查者对大于 1MiB 大件的复验延后，不是运行内门的缺口。实际独审时它们没有重算，只有下表的间接支撑；S 静止后已按根授权补跑，结果见表后。

| ID | 大件 | 目前的间接支撑 |
|---|---|---|
| D1 | `rust/target/release/s_structure_session`（4254944B）及三臂快照中的同一二进制 | 运行内 SOURCE-POSTCHECK 555 零失配；harness 逐臂快照 = manifest = SRC-CURRENT；运行时结束前复核来源；现 lstat 五字段 = SOURCE-IDENTITIES |
| D2 | UP 锚 core 与三份 fresh semantic-core（各约 1.46GB） | 比较器运行内对两侧全字节计算 SHA 并逐行校验 221 布局；左侧 SHA = 首 composite 封值；锚 core lstat 未变；fresh 大小 = compared_bytes |
| D3 | HAR、session.sqlite/WAL、database-backup、>1MiB 的 committed-candidate/响应体 | 浏览器断言在运行内完成并留 txt/PNG/RESULT；372 份 Q 小收据全数核过；公共候选已进入被逐字节比较的 core |
| D4 | node（121304480B）正文 | DEPENDENCY-PRE/POST full receipt equality（含 SHA 与 stat） |

**补跑结果（2026-10-13）。** 在根授权（`CTRL-20261012:tb1414-post-acceptance/ROOT-REHASH-AUTHORIZATION.json`，`06bb139e1a1a`）下，用 `deferred_rehash.py`（`5f6738ca951f`）先 dry-run，再以 D1 二进制冒烟，最后全量执行。收据 `CTRL-20261012:tb1414-runtime-checks/rehash/DEFERRED-REHASH-execute-full.json`（`4305c0a0fa22`），状态 `PASS_WITH_BASELINES`：825 件、15,491,333,530 字节，813 件 PASS，12 件 BASELINE_RECORDED，0 件 FAIL/ERROR。按类：D1 二进制 1 件与三臂快照 3 件 PASS；D2 锚 core 1 份与三份 fresh core PASS；D3 committed-candidate 318 件、响应体 243 件、原始响应 243 件 PASS；D4 node 1 件 PASS。12 件 BASELINE_RECORDED 是三臂各自的 HAR、session.sqlite、WAL 与 database-backup：运行内没有为它们留 SHA 收据，这次只记下首个基线 SHA，**不称 PASS**。

## 8. 自动化测试与 CI

**已在 CI 的面。** `rust/src/theta_v0/parser/tb02d_tests.rs` 经 `parser/mod.rs:637` 的 `#[cfg(test)] mod tb02d_tests;` 挂入库测试，由现有 `cargo test --all-targets` 执行；`tb02c.rs` 内的 bin 测试由现有 `--features s_session --bin s_structure_session` 步骤执行；随改动同步的 `test_tb01c_browser.cjs`、`test_tb02c_contract.cjs` 与 `test_tb02c_*.py` 已在 `s-observer` job 中。PR 上的 `rust-check` 预计与 main 一样，在「cargo test — default targets」步因 §11 的 3 个既有红失败，并跳过其后的 s_session bin 与 `--lib --features backtest_bin` 两步；这两步的本机结果见 §11。

**本交付新增接线（`.github/workflows/ci.yml`）。** `s-observer` 末尾新增一个固定 CPython 3.14.6 的 setup 步与一个测试步。测试步在 `s_session/tests` 下执行 `python -B -m unittest -v test_tb02d_contract test_tb02d_discovery test_tb02d_validate_once test_tb02d_parse_pool test_tb02d_key_reuse`，只跑不需要 S 写者二进制的五个文件，执行前先打印解释器版本与解析池布局是否匹配。这五个模块的导入闭包只有标准库与仓内模块，`test_tb02d_contract.py:46` 另起 node（该 job 已装 Node 22）。固定 3.14.6 的原因是同次解析池只在该解释器的实测 dict 布局下启用，`test_tb02d_parse_pool.py` 至少四个方法的断言（`:117`、`:154`、`:169`、`:243`）绑定该布局，在其它解释器上会失败而不是跳过。该 job 前面 3.11 的步骤不变，测试文件本身未改。**CI 实际结论 PENDING。** 本机 CPython 3.14.6 逐字执行这条最终命令：解析池布局匹配为 `True`，五模块 35 个测试 OK、0 skip（§11）。

**`test_tb02d_projection_cache.py` 未进 CI。** 它继承 `tb01c_query_tests.QueryTests`（`:12`、`:17`），基类 `setUpClass` 用 S 写者二进制建库（`tb01c_query_tests.py:44-48`、`:60`），而 `BINARY` 默认 `None`（`:29`），只有以脚本方式带 `--binary` 运行时才赋值（`test_tb02d_projection_cache.py:331-332`）；`s-observer` 不构建 Rust。它只在本机以 `python3 -B s_session/tests/test_tb02d_projection_cache.py --binary <s_structure_session>` 单独实跑并留收据；本机实跑 14 个测试 OK（§11）。该入口在 `:333` 只选 `test_projection_*` 方法，**只覆盖 projection_cache 自有的 14 个测试**；它从 `tb01c_query_tests.QueryTests` 继承的 31 个 TB-01-C Q 测试不经此入口运行，同样不在 CI。C2 改动了这些测试所测的 `s_query.py`、`s_query_integrity.py` 与 `s_readonly_server.py`，所以这 31 个测试将另以 `python3 -B s_session/tests/tb01c_query_tests.py --binary <s_structure_session>`（`tb01c_query_tests.py:545-550`）对最终字节实跑；本机实跑 31 个测试 OK（§11，R2-N1）。本报告上一版与 `ci.yml` 注释曾称它随 `discover` 进 CI，这是错的，由整体独审首轮 FS-B1 查出并已改正。

**不进 CI 的面。** `test_tb02d_snapshot_validate_once.cjs` 必须传入 `--baseline-html --candidate --candidate-sha --output`，`test_tb02d_strict_clone.cjs` 必须传入 `--baseline-client --output`，缺参即断言失败（两文件的参数校验都在第 12 行；snapshot 探针的 `--candidate` 是仓外归档的 cut40 状态 JSON）：二者是需要外部归档参数的离线对照探针，文件名虽以 `test_` 开头但不是 CI 单元测试。`tb02d_{harness,runtime,verify}.py` 与 `tb02d_browser.cjs` 是正式运行验收脚本，靠 §7 的具名运行证据，同 TB-02-C 先例；`tb02d_harness.py` 把 socket 固定在 `/private/tmp/`，只适用于 macOS 本机验收。浏览器 runner 的 30 秒帽余量很薄（AR-N1），不宜在未定位耗时前搬进 CI。

**DevSkim 与 CodeQL。** 本目录 JSON/Markdown 与 `run-package/CURRENT-SOURCES.json` 中的 SHA-256 字符串预计新增 DS173237 密钥规则误报（TB-02-C 先例新增 97 条，记录在 `issue1404-tb02c-20261007/CI-DELTA.md`）；CodeQL 工作流当前为 `disabled_manually`（#1107，整体独审首轮以 gh 只读核得），`run-package/` 中的 `.py` 只在它重新启用后才进入其 Python 扫描面，本次不会产生 CodeQL 增量（FS-N8）。DevSkim 零新增门预计因此变红，按 #1404 先例写 `CI-DELTA.md` 或登记基线，由根定；实际增量 PENDING。

## 9. 运行包证据副本（`run-package/`）

attempt 2 的六个执行 helper 与其冻结来源集按原字节放在本目录 `run-package/`：

| 文件 | bytes | SHA256 前 12 位 |
|---|---:|---|
| `remaining-driver.py` | 15261 | `595fa4c00937` |
| `remaining-harness.py` | 10504 | `34de93a4dd3a` |
| `explicit_composite.py` | 20653 | `179c81de2e1f` |
| `dependency_identity.py` | 15239 | `9a159a9d504d` |
| `reuse_state.py` | 10722 | `337a3c0a3562` |
| `completion_io.py` | 11738 | `b43db7f6fb27` |
| `CURRENT-SOURCES.json` | 96887 | `6e984f6b8c58` |

**名分：绑定 #1414 的工作草稿证据副本，不是代码。** 它们随本目录活期绑定 #1414（宪法 §5），票关后按月度归档周期打包出仓；不被任何构建、测试或运行入口调用，也不应被修改或执行。保留原文件名，以便与 `CURRENT-SOURCES.json` 的键及各绑定按 SHA 回对。整体独审首轮判此名分合宪（宪法 §5；仓内同型先例 `.chanlun/review-results/issue1392-tb02b-20260914/tools/run-faults-r2-original.py`），并核得它们不会被 pytest、CI unittest 或 linter 收集。

这些文件是机器绑定的一次性执行物，含本机 worktree、外卷与 `/tmp` runtime venv 的绝对路径（例如 `remaining-harness.py:13`、`explicit_composite.py:12`、`reuse_state.py:41`），从干净 checkout 无法运行。两对比较真正承重的 `compare_exact_traces` 在仓内 `s_session/tests/tb02c_compare.py`，helper 只负责编排、身份核对与收据写出。字节身份由三处覆盖：FINAL-PACKAGE-BINDING（`a51e1fff57dc`）逐个绑定六个 helper，最终包独审 r3（`541d1d721064`）经 SRC-CURRENT 绑定它们，实际独审核得 OUT `freeze/` 副本 = 准备目录现件 = 授权所绑 SHA。入仓副本与上表 SHA 逐件相同。

## 10. 限制

- 实际运行独审只覆盖 TB-02-D 三 fresh 与两对比较，不裁整图。D1–D4 已在本机补跑（§7.10），但 HAR、session.sqlite、WAL 与 database-backup 共 12 件运行内没有 SHA 收据，补跑只能记下首个基线，不能证明它们与运行当时相同。
- 浏览器 30 秒帽余量 0.65–1.06 秒（AR-N1），重跑有翻转风险，耗时去处未定位。
- 解析池依赖 CPython 3.14.6 精确布局；其它解释器上自动停用，冷读成本回到旧水平，原 5 秒门是否仍过未测。
- 性能与耗时只属所测输入、Python 3.14.6 runtime venv、Chrome 154.0.8037.99 与 macOS 本机，不外推。
- core、DB/WAL/SHM、HAR 只按 SHA 索引或未索引。SHA 只能证明拿到的是当时那份文件，不单独证明内容正确或可取得；跨机器完整复验依赖两处仓外归档可取得。`CTRL-20261012` 是本机临时目录。
- 首 composite 的 221 行含继承重构 139 行，不是一次新运行完整采集。
- `.chanlun/definitions/xianduan.md` 引用的 `segment.rs`、`feature_seq.rs` 行号会因本交付下移；不属 #1414 验收项，留给文档线另行处理。
- `test_tb02d_projection_cache.py` 的 14 个自有测试与 `tb01c_query_tests.py` 的 31 个 TB-01-C Q 测试都不在 CI，只有本机 `--binary` 实跑收据可作锁（§8）；两份收据已在最终字节上取得（14 与 31 个测试 OK），之后的改动只能靠手动重跑锁住。

整体独审首轮的非阻断发现（FS-N1–FS-N8）作为已知限制照录：

- **FS-N1（MEDIUM）strict 首笔冻结的波及面超出 S 会话，且最终 Rust 字节缺全量测试收据。** `feature_seq.rs:666-683` 的 `frozen_first` 改变所有 `FeatureSeqState` 使用者的结果：`IncrSegments` 经 `theta_v0/classifier/streaming.rs:42` 进入 classifier，`divide_segments` 进入 `bin/tangency_probe.rs` 与 `parser/profile.rs`。语义上它落实 `067:170/196`【答疑】“被第一笔封闭才是第一种”，整体独审判其在 AC1 范围内、无概念矛盾。但 core 实现期只有靶向运行（bin 过滤一轮超过 360 秒被终止），最终字节**没有** `cargo test --all-targets` 与 `cargo test --lib --features backtest_bin` 的全量收据，待根补跑（现状：本机实跑已补，`cargo test --all-targets --no-fail-fast` 55 个 target 共 3141 passed/3 failed/172 ignored，`cargo test --lib --features backtest_bin` 2961 passed/3 failed/161 ignored，失败只有 §11 的 3 个既有红）。建议另开文档票（可并入 #813 线）把“首笔 c 冻结”写进 `xianduan.md` §4 及其受影响代码清单。
- **FS-N2（MEDIUM）非 observe 生产路径的成本未测。** `feature_seq.rs:299-397` 的 `second_sequence` 对每个缺口候选构建完整证据，`:728` 每次非 provisional 求值再整体 clone；默认 `second_seq_scan_window=0`（无限，`theta_v0/config.rs:65`），无分型的单调段会在增量路径上重复全扫。语义不变，生产负载上的分配成本没有测过；改动会作废运行证据，留后续票（只在 observe 时构建证据）。
- **FS-N3（LOW）原 C2 单独检出时 s_session bin 测试必红。** `tb02c.rs:388-435` 的新测试起 `python3` 调 `s_tb02_contract`，原 C2 时 Python 合同仍是 HEAD 版、不认 `s-segment/2`（`s_structure_session.rs:4345/4445` 两处既有 python3 对拍预计同样受影响）。处置：提交计划把原 C2 与 C3 合并为一个提交，Rust 与 Python 合同同时落地，共五个提交。
- **FS-N4（LOW）** `s_tb02_contract.py:383` 先 `evidence.get("scope")` 后做形状检查，非 dict evidence 抛 `AttributeError`，不在 `s_readonly_server.py:984/:1074` 的捕获列表，客户端得到断连而非 503；仍是失败关闭。修改会动运行来源字节，留后续票。
- **FS-N5（LOW）** `tb02d_verify.py:46` 用 `set(expected) <= set(members)`（ORACLE-DESIGN 已声明 FIRST 全集另列），`:49`、`:58` 以实际对象的键遍历预期，实际缺键不报；合同校验器另行约束键集。
- **FS-N6（LOW）** `test_tb02d_snapshot_validate_once.cjs`、`test_tb02d_strict_clone.cjs` 以 `test_` 命名，但必须带外部归档参数（各自 `:12`）。逐行审未发现缺陷；风险只在将来有人按 glob 运行它们。
- **FS-N7（LOW）机器特定绝对路径进入 git 历史。** `run-package/remaining-harness.py:13,80`、`explicit_composite.py:12,15`、`reuse_state.py:41,46` 与 `CURRENT-SOURCES.json` 的 555 个键含本机用户目录、外卷名与 `/tmp` venv；`tb02d_harness.py:78` 的 socket 固定在 macOS 的 `/private/tmp/`；`ORACLE-DESIGN.md:23` 引仓外工作稿（历史工作稿不改，已签来源以 §4 的仓内路径与清单 SHA 为准）。全部新增与改动文件均未发现凭据、令牌或私钥。
- **FS-N8（LOW）** CodeQL 当前停用，`run-package/` 的 `.py` 现在不会产生 CodeQL 增量；DevSkim 零新增门预计因 SHA-256 字符串触发 DS173237 而变红（§8）。

## 11. 验收状态与待办

- attempt 2 运行事实的实际独审：**已完成并经根全文接受**——`PASS_WITH_DEFERRED_LARGE_REHASH`，TB-02-D 有限验收成立，阻断 0，不裁整图（REVIEW `87a739e10708`、RESULT `07495bf945fe`、BINDING `10db30c823db`）。
- 覆盖最终提交面的整体独审：首轮（final-surface，RESULT `24f9332d47cb`）**FAIL_WITH_FINDINGS**，唯一阻断 FS-B1 已修正；复审 r2（RESULT `c055481d64a9`）**PASS_FINAL_COMMIT_SURFACE_READY_PENDING_RUNTIME_CHECKS**。两轮报告全文见 `FINAL-INDEPENDENT-REVIEW.md`。
- 本机实跑（S 静止后执行，清单以复审 r2 RESULT 的 `root_checks_after_S_quiescent` 为准）：**PASS_WITH_KNOWN_BASELINE_EXCEPTION**。汇总收据 `CTRL-20261012:tb1414-runtime-checks/RUNTIME-CHECKS-RESULT.json`（114622B，`2f7d2f3f391b`），逐步收据在同目录 `steps/`，索引 `acceptance.runtime_checks` 逐项列出。

| # | 项 | 结论 |
|---|---|---|
| 1 | G2 复核 | PASS：HEAD `f95a7f3`、暂存区空、47 路径与 SHA 等于提交面清单；CURRENT-SOURCES 555 项现值全等；远端 main 未前进 |
| 2 | 新 CI 步 @ CPython 3.14.6（逐字） | PASS：解析池布局匹配 `True`；五模块 35 个测试 OK、0 skip |
| 3 | `--binary` 两组 | PASS：`test_tb02d_projection_cache.py` 14 个 OK；`tb01c_query_tests.py` 31 个 OK（R2-N1） |
| 4 | s-observer 原步骤 @ CPython 3.11 + Node 22 | PASS：TB-01-C 56、TB-02-A 9、TB-02-C 14 个 Python 测试 OK；浏览器 41 例 0 失败；合同 node 测试 6 过 0 败 |
| 5 | `cargo fmt` / `check` / `test` | 仅既有红：`fmt --check` 与三项 `cargo check` PASS；`--features s_session --bin s_structure_session` 81 passed/1 ignored；`--all-targets --no-fail-fast` 55 个 target 3141 passed/3 failed/172 ignored；`--lib --features backtest_bin` 2961 passed/3 failed/161 ignored；三处失败都只是下列 3 个既有红，#1414 新增的 5 个 `tb02d` 库测试全过（FS-N1） |
| 6 | 生成器自检与夹具再生 | PASS：两次自检输出相同，再生的 `raw-ledger.json`、`hand-oracle.json`、`MANIFEST.json` 与冻结字节逐字节相同 |
| 7 | D1–D4 大件重算 | PASS_WITH_BASELINES：813 PASS、12 BASELINE_RECORDED、0 FAIL（§7.10） |
| 8 | 每项后核 worktree 字节 | PASS：21 次核对中 47 件 SHA 与 `git status` 全部不变；工具判 FAIL 的 3 次只因 gitignored `.pyc` 副产物 |

本机环境与 CI 的差异（macOS arm64 与 bash 3.2；`cargo test` 未加 `--jobs 1` 与遥测包装；3.11 步用 uv 管理的 CPython 3.11.15 隔离 venv）照录于汇总收据的 `deviations_from_ci`。

- **3 个既有红（获批基线例外，不是 #1414 引入）**：`theta_v0::backtest::tick::tests::seam_btc_tick_bars_yield_non_trivial_structure`、`theta_v0::backtest::tick::tests::calibration_sweep_finds_non_collapsed_segments`、`theta_v0::backtest::l3_delta_r_alpha::tests::m3_partition_synthetic_holds`。证据链：基线 `f95a7f3` 的归档源码在全新独立 target 下同样是这 3 个失败，失败文本逐字相同；`d8810bff97`（#1392 的 feat 提交）在全新 target 下 3 个失败，其父 `6f99b36` 全过；main CI [38201220739](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38201220739)（`f95a7f3`）与 [37545808487](https://github.com/xy7365527-lang/NewChanlun/actions/runs/37545808487)（`8619a20f`）同红。机制边界：失败断言是「笔为空」，而 `parse_layer` 先产笔再分段（`rust/src/theta_v0/parser/mod.rs:168-182`），#1414 只改段层，结构上不改笔数。口径沿用 [#1392 关票评论](https://github.com/xy7365527-lang/NewChanlun/issues/1392#issuecomment-6027480656) 与 [#1404 关票评论](https://github.com/xy7365527-lang/NewChanlun/issues/1404#issuecomment-6117508188) 登记的获批基线例外，后续归 [#1343](https://github.com/xy7365527-lang/NewChanlun/issues/1343)/[#1405](https://github.com/xy7365527-lang/NewChanlun/issues/1405)；本交付不改数据、断言或 ignore 来修绿。这两份例外是用户对当时具体 PR 的批准，#1414 合入时仍须用户就本 PR 一并确认。
- 提交切分：原 C2 与 C3 合并（FS-N3），共五个提交。
- 逐 commit 可编译性：**PENDING**。C2 提交后由提交工位在独立 target 实测 `cargo check` 并记入提交收据；C1、C3、C4 不改 Rust；C5 为纯文档豁免。本报告随 C5 提交，不回写这些结果。
- 复审豁免：§1、§7.10、§8、§10、§11 中与本机实跑相关的段落，以及 `EVIDENCE-INDEX.json` 的同步更新，都发生在复审 r2 之后，只改这两份文档。豁免声明：复审 r2 之后，提交面只有 C5 三份文档（本报告、`EVIDENCE-INDEX.json` 与新增的 `FINAL-INDEPENDENT-REVIEW.md`）的字节变化；证据：交付准备 `COMMIT-SURFACE.json` schema 4 对复审 r2 的 `byte-check.json`（46 件，`19911a9c3a17`）逐项比对，其余 44 件 path/bytes/SHA256 相同；因此按关票门第 7 项（`docs/agents/delivery-discipline.md:21`「评审完成后仅纯文档/注释提交豁免第 7 项」）豁免复审。
- PR 上的 CI 结论：**PENDING**；`rust-check` 预计因上述 3 个既有红失败（§8）。
- DevSkim 增量：**PENDING**（CodeQL 当前停用，FS-N8）。
- 合入 main：**PENDING**，需要用户对具体 PR 号与 head SHA 的明确批准。主线：`origin/main @ <PENDING>`（== 关票面）；CI run：`<PENDING>` → `<PENDING>`。
- 关票：**PENDING**，合入 main 后按关票门另写声明，需要用户批准。

关票声明（合入 main 后才能填写）：

`关票声明：resolution=<PENDING>，已并入 main；本评论各项在此刻均已为真；逐 commit 可编译性=<PENDING>；工位=<PENDING>；报告=.chanlun/review-results/issue1414-tb02d-20261012/ACCEPTANCE-REPORT.md；评审面==关票面（PENDING）。主线：origin/main @ <PENDING>；CI run：<PENDING> → <PENDING>`
