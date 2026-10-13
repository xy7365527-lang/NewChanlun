# #1414 / TB-02-D 验收报告

> 名分：工作草稿，活期绑定 [#1414](https://github.com/xy7365527-lang/NewChanlun/issues/1414)（`docs/agents/generation-constitution.md` §5）。状态：`LIMITED_ACCEPTANCE_P2_RERUN_ACCEPTED_INCREMENTAL_REVIEW_PASSED_PENDING_RECHECK_AND_MERGE`。PR #1477 审查意见 P2 已在合入前修正（提交 `aa4039cc`、`179412c3`）；修正后的新字节完成四臂 fresh 再验收，实际运行独审为 `PASS_TB02D_P2_RERUN_ACTUAL_ACCEPTED`，根已签收（`ROOT_ACCEPTS_TB02D_P2_RERUN_ACTUAL`，deferred 为空）。**TB-02-D 有限验收在新字节上成立；不裁整图。** C5 报告提交 `cea15bd7` 的增量整体独审为 `PASS_INCREMENTAL_SURFACE_READY_FOR_MERGE_REQUEST`（阻断 0），PR 上的 CI 与 DevSkim 增量已按实测登记（§8、同目录 `CI-DELTA.md`）；本登记提交的轻量增量复核、审查线程、合入与关票仍 PENDING。787f5cf 字节上的旧运行证据按根裁定 ROOT-RULING（`600207e3a2f2`）降为历史证据（§11），原名分不改。
>
> 证据定位：仓外证据写成「归档根:相对路径」加 SHA256 前 12 位，完整 bytes/SHA256 与逻辑 ID 见同目录 `EVIDENCE-INDEX.json`（下称“索引”）。`CTRL-20261012` 是控制与准备工作区，`OUT-P2-RERUN-20261013` 是本次再验收的运行输出，`OUT-REMAINING-20261013` 是 attempt 2 的历史输出；三者都在仓外，本机绝对路径只在索引里作定位。本报告中关于 P2 再验收的数字与 SHA 都由起草工位从 OUT、启动目录、前门、授权与准备包实物重算（OUT 87354 个文件全量只读哈希），不取自旧报告。

## 1. 结论

**TB-02-D（#1414）有限验收在 P2 修正后的新字节上成立；不裁整图。** 依据是对四臂 fresh 与两对比较的实际运行独审 `CTRL-20261012:tb1414-pr-p2-triage/actual-rerun-review/`：REVIEW `48c3fc552f3c`、RESULT `2c53227fe2e3`、BINDING `7d76eaf76be9`，`status=PASS_TB02D_P2_RERUN_ACTUAL_ACCEPTED`，阻断 0，非阻断 5 条（本报告记为 P2R-N1–N5，§7.11），延后项为空。根已签收 `CTRL-20261012:tb1414-p2-rerun-root-release/ROOT-P2-ACTUAL-ACCEPTANCE.json`（`49dc3b93dfb8`，2026-10-13T12:06:07Z，`ROOT_ACCEPTS_TB02D_P2_RERUN_ACTUAL`），范围写明“四臂 fresh 与两对 221 行比较；不裁整图、不裁合入”，并声明 787f5cf 字节证据按 ROOT-RULING 降为历史证据。

P2 的内容是：CC-012 SECOND 对象的 `scan` 有四个字段取自第二特征序列，唯独 `from_stroke` 取了段起笔，`second_open_up` cut27 因此发布 [0,5]，而 SECOND 实际只扫描 [4,5]。修正后，`scan.from_stroke` 取实际扫描起点；Python 与浏览器合同的身份锚改为 `stroke_refs` 中按数值最小的笔号，FIRST 锁 `scan.from_stroke` 等于锚，SECOND 锁 `scan.from_stroke` 等于 `facts.from_stroke` 且大于锚（§7.1）。新字节下，四臂 Q 安装状态、浏览器卡片和七 cut oracle 都显示 SECOND `scan.from_stroke` 为 4（§7.6）。

再验收按原 full runtime 跑完四臂 UP/a → UP/b → DOWN/a → DOWN/b：controller exit 0，1492.13 秒；四臂 EXIT 0，各 40 输入、124 个真实 Q、221 条 core、七 cut oracle passed、cut27/40 浏览器 passed、正常恢复 equal；两对 UP/a↔UP/b、DOWN/a↔DOWN/b 用原比较器逐字节 PASS；来源 552 件与证据 27 件后核零失配，依赖后核 PASS（§7.3–§7.8）。上一轮延后的大件这次都有运行内 SHA 收据，并经独审当场重算（§7.9）。

| AC（票面原文缩写） | 主要证据 | 实际运行独审（P2 新字节，已签收） | 本机检查与 CI（新字节；合入 PENDING） |
|---|---|---|---|
| AC1 第二序列取正确同向笔、接受规定分型；独立包含与 strict 封闭按已裁语义 | 手算 14 个双向工作例与 `tb02d_tests.rs`；四臂七 cut oracle；浏览器 CC-012 含 FIRST_SEQUENCE/SECOND_SEQUENCE/matched；SECOND `scan.from_stroke`=4 | 成立：四臂 oracle 七 cut 取值一致且 passed；SECOND 扫描范围三口径一致 | TB-02-D 五模块 40 个测试通过（本机与 `cea15bd7` 的 CI `s-observer` 相同）；库测试收据按 ROOT-RULING 复用（库字节自 787f5cf 未变，§8） |
| AC2 只用完整已知前缀或有效终结证据；不设新扫描 cap；不以预算耗尽否定 | `false` / `null + scan_complete=false` 的区分（§3） | 不在实际运行独审范围（属代码与合同裁定） | 五模块 40 个、`--binary` 两组 14+31 个测试通过；`cargo test` 与 `cea15bd7` 的 CI `rust-check` 都只有 3 个既有红（§8、§12） |
| AC3 笔破坏与段破坏各自并存；一般扫描、超 50 与 continuation 归 TB-02-W | `fact_vector`；浏览器 CC-014 含 fact_vector，cut27 developing 同时含 stroke_broken 与 segment_broken | 成立：四臂浏览器业务断言全部通过 | s_session bin 81 passed/1 ignored |
| AC4 正式会话入口运行非真空输入；读公共查询/变化；浏览器核对 | 四臂各 40 输入、124 个公共/恢复查询、共 496 份 Q 收据、浏览器 cut27/40 | 成立：四臂 exit 0，Q 收据全为 `captured_validated_commit`，`page_errors=[]` | s-observer 原步骤与 `--binary` 两组通过 |
| AC5 独立 oracle；同输入新进程重放与所改持久边界恢复对拍 | hand-oracle（`9140b7827d95`）；正常恢复；两对 `compare_exact_traces` | 成立：恢复 equal、epoch 1→2；两对 221 记录逐字节 PASS；大件按运行内收据重算一致 | 生成器自检与夹具再生收据属历史证据（夹具字节未变，§11） |

最右栏只记本机检查与 CI 补到的证据，不是对 AC 的新裁定。`cea15bd7` 提交面的增量整体独审已通过（§12）。

## 2. 范围

本报告只覆盖 #1414 已裁的有限完整前缀域：`hand-oracle.json` 的 14 个双向手算工作例，以及 `second_open_up`、`second_open_down` 两个 40 根自然 OHLC 输入（首次 SECOND 在 cut27 出现、cut39 命中，最终 cut40，恢复核 cut27/40）。

不覆盖：[#1344](https://github.com/xy7365527-lang/NewChanlun/issues/1344)/TB-02-W 的一般扫描窗口、51 项以上与 continuation；CC012/013/014 全轴（目录仍声明 `not_implemented`）；G006 广泛桥；TB07 全历史恢复；父票 [#1360](https://github.com/xy7365527-lang/NewChanlun/issues/1360)、SPEC [#1340](https://github.com/xy7365527-lang/NewChanlun/issues/1340)、图 [#1323](https://github.com/xy7365527-lang/NewChanlun/issues/1323) 的整项义务。本交付合入不关闭这些票。

## 3. 交付的行为变化

基线 `f95a7f3ebc0c` 上，S 只发布 TB-02-C 的线段 seed 与第一种终结（规则 `segment-seed-first/1`），`SegmentCandidateFact` 没有 SECOND、strict 或破坏事实向量字段；需要第二特征序列才能判定的终结无法形成，也无从在 Q 和浏览器里观察。

**结构核心（Rust）。** 同一个 `FeatureSeqState` 产出 SECOND 所用的原段同向笔、独立的方向性包含步骤、任意分型三元组与实际扫描前沿；全量与增量走同一状态机。FIRST 首个有效窗口按 b 的代表笔身份冻结，后来的 c 包含合并不追溯改写第一笔封口，新 b、skip、reset 不继承旧证书。`SegmentCandidateFact` 新增 `second`、`strict`、`fact_vector`，几何段端与真正终结前沿分开；CC014 笔破坏独立枚举，段破坏只来自同次 strict/SECOND、完整 construction 与最小笔数准入证书。完整当前前缀未命中记 `false`；配置的 SECOND 窗口耗尽而无匹配记 `null` 且 `scan_complete=false`，不当作否定。规则版本 `segment-second/1`，typed 合同 `s-segment/2`；旧 C cut 仍按原规则严格读取。

**SECOND 发布的扫描范围（P2）。** CC-012 SECOND 对象的 `scan` 五个字段都描述第二特征序列自己的扫描：`from_stroke` 等于 `facts.from_stroke`（FIRST 中心之后一笔，依 `067:38`【正文】与 `xianduan.md` §3），其余四字段不变。段起点仍由 CC-013 `fact_vector.scan_from_stroke` 与身份锚承载。身份锚是 `stroke_refs` 中按数值最小的笔号（wire 为十进制文本，Python 用 int、JS 用 BigInt 比较）。该修订经根批准写入冻结 schema v1 的“经主控批准的修订”节；`s-segment/2` 尚未进入 main，不升 `schema_revision`。

**合同与 Q 读口（Python/JS）。** 合同校验 SECOND 来源、strict、证书与扫描可知性。新增 `GET /api/discover`：同一次完整存储核验后只返回协议、会话/化身与明确 profile，拒绝查询参数；页面与 HTTP 采集器据此进入 v2 固定分页，发现失败不降级到 v1，只有已核 legacy 标记才走 v1。`/api/state` 保留。

**同语义成本削减。** 真实 40 输入运行撞上原预算门后逐项加入，原门一个不放宽（verify 5 秒、联合 128MiB、单 Q 30 秒、浏览器业务 10 秒及外层 30+5 秒）：分页完整投影单槽缓存、`project_fact` 一次完整核验、canonical 单编码、同次 wire 复用、identity 片段、同次解析池（**仅在 CPython 3.14.6 / 64 位实测 dict 布局匹配时启用，其它解释器自动停用**）、前代同值 canonical key 复用；浏览器侧 Snapshot 删 5 行重复校验、严格递归 clone、meta canonical 懒复用与 UTF-8 码点比较。

**运行脚本。** `s_session/tests/tb02d_{harness,runtime,verify}.py` 与 `tb02d_browser.cjs` 经正式 S/Q/Watch 入口逐前缀运行。整臂调度窗由 300 秒改为 900 秒，属 harness 调度界（来源独审 ADOPT-G-ARM900-AUTHORITY，`2a94e22f85eb`），不是产品性能承诺；单步、资源、矩阵与清理门不变。P2 起 `tb02d_verify.py` 另比对已发布 `scan` 四字段与 oracle `second_scan`。

**跨片触碰。** 本交付修改了 TB-01-C 的 Q/浏览器文件与 TB-02-C 的若干文件，依据是票面“允许本叶补齐该外部行为必需的同一 R2 owner 支持”；没有这些改动，真实 40 输入运行会在 `/api/state` 8MiB 读口、verify 5 秒和浏览器 10 秒门上失败。整体独审首轮判其在票面 R2 owner 条款之内、未越界，并提示 strict 语义的实际波及面（FS-N1）。

## 4. 基线、来源与提交面

基线 commit `f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9`（PR #1473 合入 TB-02-C 的 merge），分支 `codex/1414-second-sequence-20261012`。P2 再验收在 HEAD `179412c33d52`、worktree 干净时执行；C5 报告随 `cea15bd735bc` 入仓，本版随其后的第九个提交入仓。已签来源是仓内 `.chanlun/review-results/issue1339-r2-spec-20260909/`（落于 `efc1ddc4d6c0`，是基线祖先），规范清单 SHA256 `38e3176b6dff`。

基线之后共七个提交：原五个（`28704548` oracle、`212d7758` 核心与合同、`2e03303c` 运行脚本、`f7b46579` CI、`787f5cf1` 文档与证据），以及 P2 两个（`aa4039cc` 修正、`179412c3` 补 L1/L2 测试）。第八个提交 `cea15bd7` 入仓 C5 重写，其提交后字节重核由增量整体独审完成（§12）。本版随第九个提交入仓，只改本目录 `ACCEPTANCE-REPORT.md`、`EVIDENCE-INDEX.json`、`FINAL-INDEPENDENT-REVIEW.md` 并新增 `CI-DELTA.md`，登记 PR 上的 CI 与 DevSkim 实测并收录增量整体独审。

代码与夹具面是 36 个路径（18 个修改、18 个新增），逐件 path/bytes/SHA256 见索引 `reviewed_source_snapshot.changed_files`。其中 35 件与本次再验收冻结的运行来源集 SRC-CURRENT-P2（`CTRL-20261012:tb1414-p2-rerun-preparation/CURRENT-SOURCES.json`，`7f473be69bf7`，552 项 = 531 个正式仓内路径 + 18 个 ijson 依赖文件 + 3 个运行包 helper）逐项相等；唯一不在集的是 `ORACLE-DESIGN.md`（工作草稿文档，不参与运行）。P2 两个提交只改了 5 件：`rust/src/bin/s_session_v2/tb02c.rs`、`s_session/s_tb02_contract.py`、`s_session/browser/tb01c-client.js`、`s_session/tests/tb02d_verify.py`、`s_session/tests/test_tb02d_contract.py`；`rust/` 下 `src/bin/` 以外（即库）零改动。

最终提交面在 36 件之外还有：`.github/workflows/ci.yml`（TB-02-D Python 测试接线，§8）、本目录 `ACCEPTANCE-REPORT.md`、`EVIDENCE-INDEX.json`、`FINAL-INDEPENDENT-REVIEW.md`、`CI-DELTA.md`，以及 `run-package/` 七件证据副本（§9）；共 48 个路径（`CI-DELTA.md` 由第九个提交新增）。

两对比较所用的 `compare_exact_traces` 在基线已入仓：`s_session/tests/tb02c_compare.py`（`96c7bb453aa2`），本交付未改动；四臂快照与 CURRENT 都绑定这份原件。

## 5. 独立 oracle

`s_session/tests/fixtures/tb02d/` 下的 `hand-oracle.json`（`9140b7827d95`，14 个双向工作例）、`raw-ledger.json`、纯整数 OHLC 生成器 `generate-input.py` 与 `MANIFEST.json`（`f056451baa3a`）先于实现冻结，设计说明见本目录 `ORACLE-DESIGN.md`。core 独审 ADOPT-A-REVIEW 引用了该 MANIFEST。oracle 不由实现输出反写；P2 修正前后 oracle 字节不变，`second_scan.from_stroke` 本来就是 4，修正是让发布值回到 oracle。

## 6. 代码与采纳独审（已发生）

每个文件只归到其当前字节首次出现的轮次；逐文件对应见索引 `changed_files[].adoption_round`。A–J 轮的收据都在 `CTRL-20261012`，起草时逐件复核当前字节，与上一版索引所记 SHA 全部相同（§11）。

| 轮 | 内容 | 收据（逻辑 ID，SHA 前 12 位，记录状态） |
|---|---|---|
| A | 核心 SECOND/strict/事实向量与 TB-02-C 适配（11 件） | ADOPT-A-REVIEW `93c23702a28d`；ADOPT-A-CHECKS `95f384e889ef` |
| B | 独立 oracle 夹具与设计 | 由 ADOPT-A-REVIEW 引用 MANIFEST |
| C | `GET /api/discover` | ADOPT-C-REVIEW `2e7aa4579815`；ADOPT-C-RESULT `90092e9daeac` PASS |
| D | 分页完整投影单槽缓存 | ADOPT-D-REVIEW `08e0575f8035`；ADOPT-D-BINDING `f8b098887742` |
| E | Snapshot 删 5 行重复校验 | ADOPT-E-REVIEW `4898f724d180` |
| F | 严格 clone | ADOPT-F-REVIEW `0570678cc1cd`；ADOPT-F-BINDING `2ab1e84b6a9b` PASS |
| G | 正式运行脚本 | ADOPT-G-EARLY-REVIEW `af97b58105bb`；ADOPT-G-READY-REVIEW `1d206d8b8322`；ADOPT-G-ARM900-AUTHORITY `2a94e22f85eb` |
| H | 累计五文件正式采纳 | canonical `d16333af5572`；identity `78ff4af8c4a9`；根决定 `10df2090721b`；补丁独审 `9961036739bc`；MANIFEST `b34d8fb2f021`；映射独审 `a902a6406413` |
| I | browser readiness runner（pure 版） | 静态独审 `f82a61d334b0` |
| J | pure codepoint Client | 静态 `0c1c8157cd65`；有限 `c40fdca3b632`；I+J 采纳映射独审 `9e7833a87408`（只证映射，不证运行） |
| **P2** | SECOND `scan.from_stroke` 与身份锚（`aa4039cc`），L1/L2 测试锁（`179412c3`） | 核实 TRIAGE `defacc9b0c1b`（VALID_P2_REQUIRES_FIX）；根裁定 ROOT-RULING `600207e3a2f2`；修正独审 REVIEW `dcfc1b8d8bb2`、RESULT `a4a630aba25c` PASS_P2_FIX_READY_FOR_FRESH_RERUN（FX-L1/FX-L2 由 `179412c3` 处理） |

J 轮之后，P2 把 `tb01c-client.js` 从 `083f424d9ad9` 改为 `d8ae4af5107f`，因此 J 轮的 pure 字节只在历史证据里出现；新字节的运行证据来自本次再验收。

## 7. P2 修正与再验收

### 7.1 P2 修正与独审

审查线程（Codex bot，commit `787f5cf1`，`tb02c.rs:163`）指出 SECOND `scan.from_stroke` 取了段起笔。核实工位判 `VALID_P2_REQUIRES_FIX`，并补出一处遗漏：两侧合同用 `scan.from_stroke` 推导 `segment_id`，只改值会让身份校验失败（TRIAGE §4）。根裁定接受 P2、合入前修正，批准 schema v1 第 2 项修订（ROOT-RULING，`600207e3a2f2`）；修订文字追加在冻结 schema（`CTRL-20261012:tb1414-schema.md`，`03ef2c2ace7e`；修订前备份 `c44d7091d856`）。

修正提交 `aa4039cc` 改了 `tb02c.rs:163`、两侧合同、合同测试与运行验证器。本机检查（`CTRL-20261012:tb1414-p2-fix/RESULT.json`，`e8ba29e919c2`）：`cargo fmt --check` 通过；release 构建得写者 `58a135103968`（4254944 B）；s_session bin 81 passed/1 ignored；TB-02-D 五模块 38 个；`--binary` 两组 14+31 个；s-observer 原步骤 56/9/14 个 Python 测试与两组 node 测试通过；新测试在旧合同上为红，字符串比较变异被拦截。修正独审 `PASS_P2_FIX_READY_FOR_FRESH_RERUN` 记两条 LOW：FX-L1 未锁 2^53 以上的 BigInt 精度，FX-L2 空 `stroke_refs` 无直接测试。第二提交 `179412c3` 只改测试文件补上两者（`CTRL-20261012:tb1414-p2-fix/c2/C2-RESULT.json`，`c8649f3297ba`）：五模块 40 个测试通过，`--binary` projection_cache 14 个通过，Number 版（两处一起或分别）与删去空引用门的变异体都被拦截。

### 7.2 再验收包与执行前门

再验收包 `CTRL-20261012:tb1414-p2-rerun-preparation/`：PLAN `4a097a2c30c9`、PREPARATION-BINDING `21aa94c410cb`、执行物四件（`remaining-driver.py` `ab80440cdebd`、`remaining-harness.py` `a4934a122db5`、`CURRENT-SOURCES.json` `7f473be69bf7`、`dependency_identity.py` `9a159a9d504d`）、前门脚本 `pregate_rebuild.py` `b90c836e35d0`、有限负例 `119f789f7f93`（35 个负例全部命中目标门，2 个阳性控制、44 项静态检查通过）。与 attempt 2 的模板包相比，包删去了首 composite 门、首侧比较绑定和只为它们服务的三个 helper，harness 改为四臂，原窗与帽、40 输入/124Q/221、七 cut oracle、浏览器、恢复与全部前后核不变。包独审 `CTRL-20261012:tb1414-pr-p2-triage/rerun-package-review/`（REVIEW `eb9beae33abb`、RESULT `c0caf1e5c3ea`、BINDING `432425c2039e`）为 `PASS_P2_RERUN_PACKAGE_READY_FOR_ROOT_AUTHORIZATION`，记三条 LOW（PK-L1–PK-L3），其中 PK-L2（worktree 重建能否得到同一字节只做了静态核查）由下面的前门实测解决。

执行前门 `CTRL-20261012:tb1414-p2-rerun-pregate-1/REBUILD-RECEIPT.json`（`4b6265d757f1`，19:23:20 +08:00）：`PASS_REBUILT_WRITER_MATCHES_EXPECTED`，在 HEAD `179412c3`、已跟踪文件前后都干净的 worktree 上执行一次 `cargo build --release --locked --features s_session --bin s_structure_session`（`CARGO_TARGET_DIR=rust/target`、`CARGO_INCREMENTAL=0`，工具链 cargo/rustc 1.98.0 Homebrew），exit 0，23.3 秒；写者从 `9f2a9f5299dd` 变为 `58a135103968`（4254944 B），与 P2 修正工位在独立 target 构建的副本逐字节相同。

### 7.3 授权、启动与时序

根一次授权 `CTRL-20261012:tb1414-p2-rerun-preparation/ROOT-P2-RERUN-AUTHORIZATION.json`（`5be5cb41ea07`）：`ROOT_APPROVED_P2_RERUN_FOUR_FRESH_ONCE`，绑定 PLAN、四执行物、前门收据、准备绑定、包独审三件、修正独审结果与 ROOT-RULING；四臂资源为端口 18990–18993 与确定性 socket；calls=1、no_retry、first_failure_stop。起草时现场重算，授权所绑全部 SHA 不变。

时序（+08:00）：现场资源核 19:25:54，根签名 19:26:35，controller 启动 19:26:51.152，结束 19:51:43.289；OUT 中最晚的文件为 19:51:43.281，没有晚于运行结束的文件。CONTROLLER-EXIT（`69c510bd8d67`）记 `exit_code=0`、已 wait 并回收、无终止信号，单调时钟 1492.13 秒；controller、supervisor 的 stdout/stderr/log 都是 0 字节。`OUT-P2-RERUN-20261013:EXECUTION.json`（`bf47a5bed5c4`）`cause=null`，四臂都在 completed_fresh，1491.73 秒。OUT 共 87354 个文件、28,687,713,979 字节。

OUT 的 `freeze/` 七件副本（PLAN、授权、四执行物、REBUILD-RECEIPT）与准备目录现件及授权所绑 SHA 逐件相同；`PACKAGE-BINDING.json`（`abd9563d7951`）12 项现场重算全部一致。

### 7.4 四臂 fresh（运行事实）

路径相对 `OUT-P2-RERUN-20261013`，逐件 SHA 见索引 `execution.arms`。耗时由起草工位从 OUT 重算，口径与实际运行独审相同，数值一致：整臂取 `stdout` 创建到 `EXIT.json` 写出；collect_core 上界取 `before-normal-27` 开始与 `revisit-39` 完成的单调时钟差；oracle 取 `query-oracle.json` 与 `normal-recovery.json` 的 mtime 差。

| 字段 | `second_open_up-a`（18990） | `second_open_up-b`（18991） | `second_open_down-a`（18992） | `second_open_down-b`（18993） |
|---|---|---|---|---|
| 臂 `EXIT.json` | `{"exit":0}` | 同左 | 同左 | 同左 |
| 输入 / core 记录 | 40 / 221 | 40 / 221 | 40 / 221 | 40 / 221 |
| 公共/恢复查询（deadline 30）；rejections | 124（120 + 4）；0 | 同左 | 同左 | 同左 |
| Q 收据 | 124 份全为 `captured_validated_commit`、full_state_installed、sources_unchanged | 同左 | 同左 | 同左 |
| 七 cut oracle | passed：15/26 NO_FIRST；27/31/38 PENDING_CASE_TWO；39/40 CASE_TWO | 同左 | 同左 | 同左 |
| 正常恢复 | equal，stopped→recovered→ready，epoch 1→2，cut27/40 | 同左 | 同左 | 同左 |
| 浏览器 | passed，cut 27/40，`page_errors=[]`，Chrome 154.0.8037.99 | 同左 | 同左 | 同左 |
| 整臂 / 单 Q 最大 / Q 合计 | 350.3s / 7.06s / 223.0s | 352.6s / 7.20s / 224.1s | 360.6s / 7.39s / 229.6s | 358.9s / 7.28s / 228.7s |
| collect_core 上界 / oracle | 31.4s / 7.4s | 34.1s / 7.6s | 34.5s / 7.6s | 34.6s / 7.7s |
| 运行时清理错误；`WRAPPER-CLEANUP.json` | `[]`；passed（进程不在、端口 errno 61、socket 不存在） | 同左 | 同左 | 同左 |

四臂共 496 份 Q 收据全部核过；每臂 40 条 ingest 的 operation 为 0..39；6 条控制事件 exit 全为 0；messages 与 clock 的 SHA 等于矩阵（`CTRL-20261012:tb1414-formal-r4-window900-freeze/TEST-MATRIX.json`，`622c8547a467`）的 historical_generated_inputs，124 条命令序列逐项等于矩阵的 public 120 条加 recovery 4 条。窗与帽全部在限内：arm900、collect120、oracle180、单 Q 30、浏览器 30；浏览器 +5 回收分支未触发。七 cut oracle 的回执作用域是“frozen named cuts of actual Q; full natural prefix traces separately captured and compared”，完整前缀轨迹另由 §7.5 比较。

### 7.5 两对原字节比较

`OUT-P2-RERUN-20261013:PAIR-COMPARISON-RESULT.json`（`74fc1abf6397`）`status=PASS`，比较器为原 `compare_exact_traces`，方法 `exact_bytes_and_strict_jsonl`，布局为 harness 的 `expected_layout(40)`（221 行：ingest 40、receipt 40、public 120、表 17、schema 1、lifecycle 1、恢复 cut 27/40）。

| 对 | 记录 | 比较字节 | excluded_fields | identity_renaming | 两侧 SHA256 | 回执 |
|---|---|---|---|---|---|---|
| second_open_up：UP/a ↔ UP/b | 221/221 | 1456959869 | `[]` | `[]` | 两侧均 `5e85f9554906…` | PASS |
| second_open_down：DOWN/a ↔ DOWN/b | 221/221 | 1461830301 | `[]` | `[]` | 两侧均 `0696fb1f8b89…` | PASS |

起草工位全量重算四份 `evidence/semantic-core.jsonl`，SHA 与大小都等于比较回执。两对都在新字节的两次独立 fresh 之间，旧首 composite 锚没有被读取、比较或绑定。UP core 的字节数与 787f5cf 时的锚 core 相同而 SHA 不同：修正把 `"0"` 换成 `"4"`，长度不变，内容变了。

### 7.6 已发布的 SECOND 扫描范围

三个口径独立成立：

1. **Q 安装状态。** 起草工位读取四臂 `known-(c-1)/committed-candidate.json` 的 `state.snapshot.objects`（`CTRL-20261012:tb1414-c5-rewrite-draft/SECOND-SCAN-Q-CHECK.json`，`54548345ba94`）：在 cut 27、31、38、39、40 上都恰有一个 SECOND 对象，`scan.from_stroke` 为 "4"；scan 四字段等于 facts 四字段，也等于 hand-oracle `second_scan` 的 wire 值（through 依次为 5/6/7/8/8，input 为 6/7/8/9/9，resource_truncated 为 false），状态依次为 PENDING/PENDING/PENDING/CONFIRMED/CONFIRMED。cut27 的 SECOND 对象 UP 为 `obj-fc50c19858f5…`、DOWN 为 `obj-cdf498372a73…`；787f5cf 字节下 UP cut27 是 `obj-736ad23a…`（`scan.from_stroke="0"`），对象身份随 payload 改变。
2. **浏览器卡片。** 四臂 cut40 的 `CC-012.txt` 中 SECOND_SEQUENCE 卡片的 scan 为 from_stroke 4、through 8、input 9、tail_cache_window 0（对象 UP `obj-0e8992b5d4b7…`、DOWN `obj-ac711b69fbc1…`，等于 Q 中 cut40 的 SECOND 对象）。
3. **运行内 oracle。** 各臂快照绑定新 `tb02d_verify.py`（`1dc3de9a3933`），它逐 cut 比对已发布 `scan` 四字段与 oracle `second_scan`；七 cut oracle passed 即含这条比对。实际运行独审另抽查 Q 原始响应体（RAW-Q-SECOND-CHECK，`67411ccf3e26`）得同一结论。

### 7.7 浏览器证据

runner `tb02d_browser.cjs`（`a3d32f0be360`）只有在全部业务断言通过后才写 `browser/RESULT.json`。四臂 txt 中以下字符串全部在场：CC-012 含 FIRST_SEQUENCE、SECOND_SEQUENCE、matched；CC-013 含 CASE_TWO、geometric_end、terminated_known_at；CC-014 含 fact_vector；developing（切到 cut 27 AsKnown）含 PENDING_CASE_TWO、stroke_broken、segment_broken。READINESS 记 failure=false、cleanup_errors 0、pageerror 0。

| 臂 | 浏览器窗（mtime）/ 内计 | 距 30 秒帽余量（mtime 口径） | requestfailed（非门项） |
|---|---|---|---|
| UP/a | 29.53s / 29347ms | 0.47s | 25 |
| UP/b | 29.50s / 29318ms | 0.50s | 16 |
| DOWN/a | 29.56s / 29377ms | 0.44s | 19 |
| DOWN/b | 29.38s / 29198ms | 0.62s | 15 |

mtime 口径的窗取 `browser/READINESS-DIAGNOSTICS.json` 与 `evidence/browser-command.json` 的 mtime 差；诊断事件保留 128 个、丢弃 244 个，耗时去向不可见（P2R-N1）。

每臂五张截图（SHA 前 12 位）：

| 截图 | UP/a | UP/b | DOWN/a | DOWN/b |
|---|---|---|---|---|
| CC-011.png | `972f07195601` | `972f07195601` | `86a141f51e61` | `86a141f51e61` |
| CC-012.png | `2e5465ab321e` | `2e5465ab321e` | `43be43347d33` | `43be43347d33` |
| CC-013.png | `0834d5fe77d8` | `0834d5fe77d8` | `ed672adaf79b` | `ed672adaf79b` |
| CC-014.png | `b0f772853fd7` | `b0f772853fd7` | `676bd63410d1` | `676bd63410d1` |
| developing.png | `9de30e438487` | `9de30e438487` | `5766bcb9ebac` | `5766bcb9ebac` |

同一 case 的两臂五张截图逐字节相同。CC-011 截图与 787f5cf 时相同（seed 卡片不含 SECOND），其余四张随 SECOND 对象改变。

### 7.8 来源身份、后核与依赖

四臂 `source-before-run` MANIFEST 各 105 项，逐项等于 RUN-PLAN（`a291ee36bb78`）的 102 项来源加 3 项 prepared；RUN-PLAN 等于 SRC-CURRENT-P2 中 s_session 运行时子集加写者二进制。起草工位按全量清单核对四臂 420 份快照字节，全部等于清单。快照中的新字节身份：写者 `58a135103968`、合同 `7d1389497161`、Client `d8ae4af5107f`、验证器 `1dc3de9a3933`、合同测试 `cdf8229a92ef`、runner `a3d32f0be360`、runtime `e17fd1f6fecc`。`tb02c.rs`（`251811310fbe`）不在运行时快照域内，由 HEAD blob、CURRENT 与前门重建三者共同绑定。

`SOURCE-POSTCHECK.json`（`20cf884f3d1f`）：来源 552 项、证据 27 项，各类失配全为空；起草时现场重算 552 + 27 件仍零失配。`DEPENDENCY-POST.json`（`01b19d5def00`）`status=PASS`（前核 `47ec369c5600`）；python、node、playwright 三项前后收据全等，Chrome 只有 ctime 不同，按事前声明的 `named-chrome-object-content-endpoints/1` 合同核验通过，不宣称整 bundle 已验证。

### 7.9 大件 SHA 收据

attempt 2 的 HAR、session.sqlite、WAL 与 database-backup 运行内没有 SHA 收据，事后只能记基线。本次包在每臂收尾后写 `LARGE-ARTIFACT-SHA256.json`（不参与业务门），四臂 errors 都为空、identity_stable 全为 true，收据 SHA 等于 EXECUTION 中的记录。起草工位重算全部 16 件，SHA 与字节数都等于收据：

| 臂 | network.har | session.sqlite | session.sqlite-wal | database-backup.sqlite |
|---|---|---|---|---|
| UP/a | 36162293 B `c5c5405a0f41` | 119549952 B `ace0d7cbacb5` | 10390672 B `ff0d33479822` | 119549952 B `e9b5902db050` |
| UP/b | 45177767 B `e3243383fddb` | 119549952 B `ace0d7cbacb5` | 10390672 B `1f94ab6f18e9` | 119549952 B `e9b5902db050` |
| DOWN/a | 42504144 B `9f20dc8c21eb` | 120057856 B `c8188a0d2828` | 10440112 B `4ff05ea8e9ee` | 120057856 B `169e6b6a7b43` |
| DOWN/b | 46361188 B `8d9ce5ab0d08` | 120057856 B `c8188a0d2828` | 10440112 B `9c30e6d98eff` | 120057856 B `169e6b6a7b43` |

同一 case 两臂的 session.sqlite 与 database-backup 逐字节相同，WAL 与 HAR 不同。实际运行独审（LARGE-RECHECK，`a9e27bb987c7`）也重算了这 16 件和四份 semantic-core，共读 7,007,878,532 B，结果 PASS；本次没有延后项。

### 7.10 两处现象的解释

**`evidence/RESULT.json` 的 `not_evaluated`（P2R-N2）。** 四臂 RESULT 的 `semantic_oracle`/`browser` 都是初值 `not_evaluated`，tb02d 路径不改写；status 为 `CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER` 恰好意味着恢复、frozen oracle 与真实浏览器已在同一进程内执行并通过。成功门读取 query-oracle、browser、normal-recovery 叶子与双跑比较，全部通过。交付文案不单独引用这两个字段。

**PID 退出观测警告（P2R-N3）。** 四臂停机记录都有“进程命令暂不可读”与“PID 已复用或进程命令改变，拒绝采用旧记录”两类警告，与 attempt 2 同型：停止前按 uid、lstart 与完整命令双核身份后才发信号，stopped 只由 `ps -p` 无行或僵尸态判定，方向是失败关闭。实际运行独审升级只读复核 18990–18993 都是 ECONNREFUSED、四个 socket 不存在、没有本次运行的残留进程（LIVE-RESOURCE-CHECK，`f578c0e809e4`）。

### 7.11 实际运行独审的非阻断发现（照录）

| ID | 发现（RESULT 原文） | 处置 |
|---|---|---|
| P2R-N1 | 浏览器30s帽余量0.44–0.62s，较attempt 2的0.65–1.06s更薄；耗时去向未定位；帽值不得静默放宽 | 再跑或进 CI 前须先查清耗时分布；30 秒上限不放宽 |
| P2R-N2 | RESULT.json semantic_oracle/browser not_evaluated 标签过时（同上次线索一） | 见 §7.10；若要让 RESULT 自洽须走新的采纳修订，本次不改 |
| P2R-N3 | PID退出观测警告措辞与wrapper只核PID的残余同族风险（同上次线索二） | 见 §7.10；建议后续包处理 |
| P2R-N4 | requestfailed 25/16/19/15，不在门内，无pageerror；HAR只做SHA未分析 | 成因需打开 HAR，本次未开 |
| P2R-N5 | Chrome主文件ctime变化，在已声明合同内；整bundle未验证 | 在已声明合同之内 |

根签收把这五条全部列为 carried_non_blocking，其中第一条写明“再跑或进 CI 前须先查清耗时分布，不得悄悄放宽上限”。

## 8. 自动化测试与 CI

**已在 CI 的面。** `rust/src/theta_v0/parser/tb02d_tests.rs` 经 `parser/mod.rs` 的 `#[cfg(test)] mod tb02d_tests;` 挂入库测试，由现有 `cargo test --all-targets` 执行；`tb02c.rs` 内的 bin 测试（含 14 个冻结例投影进 Python 合同）由现有 `--features s_session --bin s_structure_session` 步骤执行；随改动同步的 `test_tb01c_browser.cjs`、`test_tb02c_contract.cjs` 与 `test_tb02c_*.py` 已在 `s-observer` job 中。PR 上的 `rust-check` 实测与 main 一样，在「cargo test — default targets」步失败并跳过其后的 s_session bin 与 `--lib --features backtest_bin` 两步：`cea15bd7`（run 38397240254）为 2955 passed/3 failed/161 ignored，失败恰为 §12 的 3 个既有红；`179412c3`（run 38387197713）另多一个 `incremental_tower_scaling_dominates_full_synthetic` 计时红（n=400 增量/全量比 0.723，门 < 0.7）。该测试在 debug 构建下测墙钟比值，库字节在 `787f5cf1`、`aa4039cc`、`179412c3`、`cea15bd7` 上逐字节相同，同字节在 `aa4039cc` 与 `cea15bd7` 上通过，且它不走本 PR 改过的分段与特征序列代码，因此判为计时抖动；0.7 门不放宽，建议另开票跟踪（`CI-DELTA.md`）。因 bin 全量一步被跳过，`cea15bd7` 字节的 s_session bin 全量读数只有增量整体独审在本机那次（81 passed/0 failed/1 ignored）。

**本交付新增接线（`.github/workflows/ci.yml`）。** `s-observer` 末尾新增固定 CPython 3.14.6 的 setup 步与测试步，在 `s_session/tests` 下执行 `python -B -m unittest -v test_tb02d_contract test_tb02d_discovery test_tb02d_validate_once test_tb02d_parse_pool test_tb02d_key_reuse`，只跑不需要 S 写者二进制的五个模块，执行前先打印解释器版本与解析池布局是否匹配。固定 3.14.6 的原因是同次解析池只在该解释器的实测 dict 布局下启用，`test_tb02d_parse_pool.py` 的布局断言在其它解释器上会失败而不是跳过。P2 没有改 `ci.yml`。**CI 实测：** `cea15bd7` 的 `s-observer`（run 38397240254）成功，CPython 3.14.6 下解析池布局匹配 `True`，五模块 Ran 40 OK；同次 `test` 与 `fixture-drift` 也成功。此前本机 CPython 3.14.6 在 HEAD `179412c3` 的测试字节上逐字执行这条命令，结果相同（`CTRL-20261012:tb1414-p2-fix/c2/steps/11-ci-tb02d-py3146`）。

**新字节的本机检查。** P2 修正与第二提交在独立 target 上取得（`tb1414-p2-fix/`、`tb1414-p2-fix/c2/`）：`cargo fmt --check` 通过；release 构建得写者 `58a135103968`；`cargo test --features s_session --bin s_structure_session` 81 passed/1 ignored；五模块 38→40 个；`test_tb02d_projection_cache.py --binary` 14 个；`tb01c_query_tests.py --binary` 31 个；s-observer 原步骤（CPython 3.11.15 + Node 22）TB-01-C 56、TB-02-A 9、TB-02-C 14 个 Python 测试与浏览器协议、合同 node 测试（6 过 0 败）全部通过。

**库测试。** P2 两个提交没有改动库（`rust/` 下除 `src/bin/` 外零改动），按 ROOT-RULING 与 TRIAGE §6 复用库测试收据：787f5cf 字节上 `cargo test --all-targets --no-fail-fast` 与 `--lib --features backtest_bin` 的结论（失败只有 §12 的 3 个既有红，#1414 新增的 5 个 `tb02d` 库测试全过）见历史证据 §11.4。

**`test_tb02d_projection_cache.py` 与 `tb01c_query_tests.py` 不在 CI。** 前者继承后者的 `QueryTests`，基类 `setUpClass` 用 `--binary` 传入的 S 写者二进制建库，`s-observer` 不构建 Rust。二者只在本机以 `--binary <s_structure_session>` 实跑留收据：projection_cache 入口只选 `test_projection_*`，覆盖 14 个自有测试；31 个 TB-01-C Q 测试另以 `tb01c_query_tests.py --binary` 运行。两者在新字节上都已通过；之后的改动只能靠手动重跑锁住。

**不进 CI 的面。** `test_tb02d_snapshot_validate_once.cjs` 与 `test_tb02d_strict_clone.cjs` 是需要外部归档参数的离线对照探针（缺参即断言失败），不是 CI 单元测试。`tb02d_{harness,runtime,verify}.py` 与 `tb02d_browser.cjs` 是正式运行验收脚本，靠 §7 的具名运行证据；`tb02d_harness.py` 把 socket 固定在 `/private/tmp/`，只适用于 macOS 本机验收。浏览器 runner 的 30 秒帽余量只有 0.44–0.62 秒（P2R-N1），在查清耗时分布之前不宜搬进 CI，上限也不放宽。

**DevSkim 与 CodeQL。** DevSkim 门在 main 上本就红（main 自身对 `.github/devskim-baseline.json` 有 33443 条基线外告警），PR 归因只看相对 main 实测 SARIF 的差集。`cea15bd7`（run 38397240180）相对 main 按 #1404 先例的多重集口径新增 1173 条、消失 0 条：DS173237 1163 条，命中值全是 SHA-256 或本仓 git 对象号；另有 DS117838 5 条（路径名含 key 的「路径 → SHA」行）、DS162092 3 条本机 loopback、DS197836 1 条（把 `runtime` 中的 time 当作时间值）与 DS172411 1 条固定回调提示。全部是误报或提示，需修 0 条；逐项见同目录 `CI-DELTA.md`，按 #1404 先例（`issue1404-tb02c-20261007/CI-DELTA.md`）登记，未改基线。CodeQL 工作流当前为 `disabled_manually`（#1107），`run-package/` 中的 `.py` 只在它重新启用后才进入扫描面（FS-N8）。

## 9. 运行包证据副本（`run-package/`）

本次再验收的执行物与其授权、前门收据按原字节放在本目录 `run-package/`，替换 attempt 2 的七件旧副本：

| 文件 | bytes | SHA256 前 12 位 | 来源 |
|---|---:|---|---|
| `remaining-driver.py` | 21835 | `ab80440cdebd` | 准备包执行物 |
| `remaining-harness.py` | 10567 | `a4934a122db5` | 准备包执行物 |
| `CURRENT-SOURCES.json` | 96356 | `7f473be69bf7` | 准备包执行物（SRC-CURRENT-P2） |
| `dependency_identity.py` | 15239 | `9a159a9d504d` | 准备包执行物（与 attempt 2 逐字节相同） |
| `PLAN.json` | 18789 | `4a097a2c30c9` | 再验收 PLAN |
| `ROOT-P2-RERUN-AUTHORIZATION.json` | 3327 | `5be5cb41ea07` | 根一次授权 |
| `REBUILD-RECEIPT.json` | 2909 | `4b6265d757f1` | 执行前门收据 |

移出的旧副本是 `remaining-driver.py`（`595fa4c00937`）、`remaining-harness.py`（`34de93a4dd3a`）、`explicit_composite.py`（`179c81de2e1f`）、`reuse_state.py`（`337a3c0a3562`）、`completion_io.py`（`b43db7f6fb27`）和旧 `CURRENT-SOURCES.json`（`6e984f6b8c58`）；`dependency_identity.py` 字节不变。旧副本的身份与来源仍记在索引 `historical_787f5cf.prior_run_package`。

**名分：绑定 #1414 的工作草稿证据副本，不是代码。** 它们随本目录活期绑定 #1414（宪法 §5），票关后按月度归档周期打包出仓；不被任何构建、测试或运行入口调用，也不应被修改或执行。保留原文件名，以便与 `CURRENT-SOURCES.json` 的键和授权的 `scripts` 按 SHA 回对。整体独审首轮判此名分合宪（同型先例 `.chanlun/review-results/issue1392-tb02b-20260914/tools/run-faults-r2-original.py`）。

这些文件是机器绑定的一次性执行物，含本机 worktree、外卷与 `/tmp` runtime venv 的绝对路径，从干净 checkout 无法运行（FS-N7 同样适用）。两对比较真正承重的 `compare_exact_traces` 在仓内 `s_session/tests/tb02c_compare.py`。字节身份由三处覆盖：根授权逐个绑定四执行物与前门收据；OUT `freeze/` 七件副本等于准备目录现件；PACKAGE-BINDING 12 项现场重算一致。

## 10. 限制

- 实际运行独审只覆盖 TB-02-D 的 P2 新字节四臂与两对比较，不裁整图。
- 浏览器 30 秒帽余量 0.44–0.62 秒（P2R-N1，mtime 口径；内计 29.20–29.38 秒），比 attempt 2 的 0.65–1.06 秒更薄，耗时去向未定位；再跑或进 CI 前须先查清，上限不得放宽。
- 解析池依赖 CPython 3.14.6 精确布局；其它解释器上自动停用，冷读成本回到旧水平，原 5 秒门是否仍过未测。
- 性能与耗时只属所测输入、Python 3.14.6 runtime venv、Chrome 154.0.8037.99 与 macOS 本机，不外推。
- SHA 只能证明拿到的是当时那份文件，不单独证明内容正确或可取得；跨机器复验依赖两处仓外归档可取得。`CTRL-20261012` 是本机临时目录。
- `.chanlun/definitions/xianduan.md` 引用的 `segment.rs`、`feature_seq.rs` 行号会因本交付下移；不属 #1414 验收项，留给文档线另行处理。
- `test_tb02d_projection_cache.py` 的 14 个与 `tb01c_query_tests.py` 的 31 个测试不在 CI，只有本机 `--binary` 收据可作锁（§8）。

整体独审首轮（787f5cf 提交面）的非阻断发现 FS-N1–FS-N8 仍然有效，P2 没有改变它们所描述的事实：

- **FS-N1（MEDIUM）** strict 首笔冻结（`feature_seq.rs`）改变所有 `FeatureSeqState` 使用者（classifier streaming、`tangency_probe`、`profile`）；语义落实 `067:170/196`【答疑】“被第一笔封闭才是第一种”，在 AC1 范围内。全量库测试收据见 §11.4（3 个既有红之外无失败）。建议另开文档票把“首笔 c 冻结”写进 `xianduan.md` §4。
- **FS-N2（MEDIUM）** 非 observe 生产路径的 `second_sequence` 对每个缺口候选构建完整证据并整体 clone，默认无限窗口，生产负载上的成本未测；留后续票。
- **FS-N3（LOW）** 原 C2 单独检出时 s_session bin 测试必红；已在提交计划中合并原 C2 与 C3。
- **FS-N4（LOW）** `s_tb02_contract.py` 先 `evidence.get("scope")` 后做形状检查，非 dict evidence 抛 `AttributeError`，客户端得到断连而非 503；仍是失败关闭，留后续票。
- **FS-N5（LOW）** `tb02d_verify.py` 的 FIRST 成员用子集比较，以实际对象的键遍历预期；合同校验器另行约束键集。
- **FS-N6（LOW）** 两个 `test_` 命名的离线探针必须带外部归档参数。
- **FS-N7（LOW）** 机器特定绝对路径进入 git 历史（`run-package/`、`tb02d_harness.py` 的 socket、`ORACLE-DESIGN.md` 引仓外工作稿），未发现凭据、令牌或私钥。
- **FS-N8（LOW）** CodeQL 当前停用；DevSkim 零新增门实测为红（main 上本就红），PR 增量全部是误报或提示，需修 0 条（§8、`CI-DELTA.md`）。

另有两条包独审的 LOW 已在本次运行中消解或只涉及说明：PK-L1（README 写“清除 CARGO_*/RUST*”，代码只剥五个前缀；偏差只会失败关闭，实测 `stripped_environment_keys=[]`）、PK-L3（driver 只核独审各件 SHA 不变，不核内容对应本包；根授权已绑定包独审三件）。

## 11. 历史证据（787f5cf 字节，按 ROOT-RULING 降级）

根裁定 ROOT-RULING（`CTRL-20261012:tb1414-pr-p2-triage/ROOT-RULING.json`，`600207e3a2f2`）写明：修正使所有 SECOND 对象的 object_id 级联改变，787f5cf 字节上的首 composite 锚、attempt 2 三 fresh、两对比较与 D1–D4 只作历史证据；根对新字节的签收（§1）声明 attempt 2 签收 `ROOT-ACTUAL-ACCEPTANCE`（`c8d6171af117`）所据证据按该裁定降为历史证据。本节保留这些记录的原名分，不改写、不翻案，也不把它们当作新字节的运行证据。上一版索引中全部 167 条带 SHA 的收据，起草时逐件复核当前字节，全部相同（`CTRL-20261012:tb1414-c5-rewrite-draft/HISTORICAL-RECEIPT-VERIFY.json`，`82c18fb6de38`）；完整条目见索引 `historical_787f5cf`。

### 11.1 失败史

| 运行 | 结果 | 收据 | 后来被采用的部分 |
|---|---|---|---|
| 正式 r1 | FAIL：cut31 `/api/state` 503 QueryBudgetExceeded | 原件在外卷，索引未绑定 SHA | 无 |
| 正式 r2 / r3 | FAIL：300 秒窗内 35 输入 / 首臂 96Q | 同上 | 无 |
| 正式 r4（窗 900） | FAIL：120Q 后 collect_core 30 秒窗超时 | RUN-R4-OUTCOME `186cd22ea5ea` | 120Q/80 收据经补充继承 |
| r4 首臂补充 | FAIL：219 记录后 Q `/api/ready` 503 | RUN-R4S-REVIEW `738027374c2a` | 219 行 core（后被 strict221 取代） |
| 冷 40 只读 verify 诊断 | canonical/identity/parse-pool 三次 FAIL；key-reuse 首次通过原 ready 门 | `851b59eb74d8`、`c0dcc6998f62`、PLAN `38a768b522f5` | 支撑 H 轮采纳决定，不是矩阵验收 |
| key-reuse 补充 r1 | FAIL_VERIFIED_NEW_RUNTIME_METADATA_INTERFACE | RUN-KR1-REVIEW `68672dcf28ec` | 无 |
| key-reuse 补充 r2 | 整体 FAIL（浏览器 networkidle 超时） | RUN-KR2-REVIEW `89d3138d7de1`；CHECK `cd9679367e63` | 4 个真实 Q、正常恢复、七 cut oracle（进入首 composite） |
| browser-only completion | 整体 FAIL（浏览器完整提交超时） | RUN-BO-REVIEW `02f9da493f04`；CHECK `ad2296b9f4c8` | strict221 core、sealed DB/WAL/SHM（进入首 composite） |
| hoist + 观测 completion | FAIL：final40 wait10000 超时；Chrome post 跨期身份 FAIL | 原件在外卷，索引未绑定 SHA | 无 |
| observed codepoint completion | PASS，仅属 observed Client `8fa0d2868e4c` 与 runner `59c56bce97a6` | RUN-OC-RESULT `8cf04d06a070`；RUN-OC-REVIEW `636341d09b1b` | 首 composite 的浏览器层 |

### 11.2 首 composite 锚

四层 composite 的 payload 是 COMP-DRAFT（`d08eb70651f8`），core 为 `c0dbe0db7e66…`、1456959869 字节、221 条记录，其中继承重构 139 条、新重构 0 条；aggregate 独审 `91e91ca8215a` VERIFIED_COMPLETE_COMPOSITE；三来源映射 `1aa99a727837` 及其独审 `f740e3b822e1` PASS_EXACT_THREE_SOURCE_MAPPING；根 detached release `13c68f4881c9`，明写不是 pure 运行验收。该 core 与 787f5cf 的 UP/b core 逐字节相同，携带旧的 SECOND `scan.from_stroke="0"`，因此不能给新字节作比较锚（TRIAGE §6）；新字节以 UP/a↔UP/b 两次 fresh 对拍替代（§7.5）。

### 11.3 attempt 2：三 fresh 与两对比较

最终包 PLAN `1e5d62f9a9ee`、FINAL-PACKAGE-BINDING `a51e1fff57dc`、最终包独审 r3 `541d1d721064`；attempt 1 在依赖预检失败，无 OUT、无产品副作用（`ee8132217021`）。attempt 2 根授权 `9a027504f1c8`，启动 `4ad159d59771`，controller 退出 `470ec8ed4168`，`OUT-REMAINING-20261013:EXECUTION.json`（`a1c811c58707`）三臂 UP/b、DOWN/a、DOWN/b 完成。两对比较 `COMPOSITE-COMPARISON-RESULT.json`（`0d0624c92327`）PASS：首 composite ↔ UP/b 两侧均 `c0dbe0db7e66…`，DOWN/a ↔ DOWN/b 两侧均 `bc6b6afd4502…`。来源后核 `2389a0194385`（555/141 零失配），依赖后核 `3b0cd8e2f32a` PASS。实际独审 `CTRL-20261012:tb1414-explicit-composite-review/actual-remaining/`（REVIEW `87a739e10708`、RESULT `07495bf945fe`、BINDING `10db30c823db`）为 `PASS_WITH_DEFERRED_LARGE_REHASH`，非阻断 AR-N1–AR-N5，根签收 `c8d6171af117`。这些结论只覆盖 787f5cf 字节。

### 11.4 本机实跑与 D1–D4 补跑

本机实跑汇总 `CTRL-20261012:tb1414-runtime-checks/RUNTIME-CHECKS-RESULT.json`（`2f7d2f3f391b`）在 787f5cf 字节上为 `PASS_WITH_KNOWN_BASELINE_EXCEPTION`：TB-02-D 五模块 35 个测试、`--binary` 14+31 个、s-observer 原步骤、`cargo fmt` 与三项 `cargo check`、s_session bin 81 passed/1 ignored、`cargo test --all-targets --no-fail-fast`（55 个 target，3141 passed/3 failed/172 ignored）与 `--lib --features backtest_bin`（2961 passed/3 failed/161 ignored）、生成器自检与夹具再生逐字节相同。失败只有 §12 的 3 个既有红。P2 没有改动库与夹具，库测试与夹具再生的结论对现字节仍适用（§8）；运行与 bin、合同层的检查已在新字节上重取（§7、§8）。

D1–D4 补跑在根授权（`06bb139e1a1a`）下执行，收据 `CTRL-20261012:tb1414-runtime-checks/rehash/DEFERRED-REHASH-execute-full.json`（`4305c0a0fa22`），`PASS_WITH_BASELINES`：813 件 PASS、12 件只记基线（三臂 HAR/DB/WAL/backup 运行内无 SHA 收据），0 件 FAIL。本次再验收已为这类大件留运行内收据（§7.9）。

### 11.5 787f5cf 提交面的两轮整体独审

首轮 `final-surface`（REVIEW `1149a14bd331`、RESULT `24f9332d47cb`、BINDING `47caee912385`）为 `FAIL_WITH_FINDINGS`，唯一阻断 FS-B1（新 CI 步用 `discover` 会收进需要写者二进制的 `test_tb02d_projection_cache.py`）已改为显式五模块；复审 r2（REVIEW `3165f21d94fb`、RESULT `c055481d64a9`、BINDING `f6327077651c`）为 `PASS_FINAL_COMMIT_SURFACE_READY_PENDING_RUNTIME_CHECKS`。两轮审的是 787f5cf 时的提交面；全文仍逐字收录在 `FINAL-INDEPENDENT-REVIEW.md` 的历史节。`cea15bd7` 提交面的增量整体独审已完成（§12）。

## 12. 验收状态与待办

- P2 实际运行独审：**完成**，`PASS_TB02D_P2_RERUN_ACTUAL_ACCEPTED`，核查脚本 174 项全过，阻断 0、延后 0（REVIEW `48c3fc552f3c`、RESULT `2c53227fe2e3`、BINDING `7d76eaf76be9`）；全文见 `FINAL-INDEPENDENT-REVIEW.md`。
- 根签收：**完成**，`ROOT_ACCEPTS_TB02D_P2_RERUN_ACTUAL`（`49dc3b93dfb8`，2026-10-13T12:06:07Z），deferred 为空，五条非阻断项随附。
- `cea15bd7` 提交面（787f5cf1 之后的 `aa4039cc`、`179412c3`、`cea15bd7`，含 C5 重写与 run-package 替换）的增量整体独审：**完成**，`PASS_INCREMENTAL_SURFACE_READY_FOR_MERGE_REQUEST`，阻断 0，审查者 `/root/tb1394_r6_contract`（此前未参与 #1414；REVIEW `c6b0adf14e10`、RESULT `8d8d2a800b60`、BINDING `1b82b5dc3acd`）。两条 MEDIUM 要求随合入请求登记 CI 实测（IR-M1）与 DevSkim 增量（IR-M2），由本版 §8 与 `CI-DELTA.md` 登记；IR-L1 指出的提交前自述已在 §4 与索引 `commits` 订正；IR-L2（跨对象 `facts.from_stroke == strict.elements[1]+1` 未锁）留后续；IR-N1 是 `test_tb02c_compare` 依赖 ijson 的环境项。全文见 `FINAL-INDEPENDENT-REVIEW.md`。
- 本登记提交（第九个提交）的轻量增量复核：**PENDING**。增量整体独审只绑定 `cea15bd7`，任何新提交都要对改动文件做一次轻量增量复核。
- PR 上的 CI 结论：**已实测登记**。`cea15bd7`（run 38397240254）的 `rust-check` 只因下列 3 个既有红失败，`test`、`s-observer`、`fixture-drift` 成功；`179412c3` 上 `incremental_tower_scaling_dominates_full_synthetic` 的计时红判为抖动，0.7 门不放宽，建议另开票跟踪（§8）。DevSkim 增量：**已登记**，需修 0 条（`CI-DELTA.md`）。本登记提交自身的 CI：**PENDING**。
- 审查线程回复与 resolve：**PENDING**。
- 合入 main：**PENDING**，需要用户对具体 PR 号与新 head SHA 的明确批准。
- 关票：**PENDING**，合入 main 后按关票门另写声明，需要用户批准。

**3 个既有红（获批基线例外，不是 #1414 引入）**：`theta_v0::backtest::tick::tests::seam_btc_tick_bars_yield_non_trivial_structure`、`theta_v0::backtest::tick::tests::calibration_sweep_finds_non_collapsed_segments`、`theta_v0::backtest::l3_delta_r_alpha::tests::m3_partition_synthetic_holds`。证据链：基线 `f95a7f3` 的归档源码在全新独立 target 下同样是这 3 个失败，失败文本逐字相同；`d8810bff97`（#1392 的 feat 提交）在全新 target 下 3 个失败，其父 `6f99b36` 全过；main CI [38201220739](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38201220739)（`f95a7f3`）与 [37545808487](https://github.com/xy7365527-lang/NewChanlun/actions/runs/37545808487)（`8619a20f`）同红（证据在历史收据 `tb1414-runtime-checks/steps/05e*`）；PR head `cea15bd7` 的 run [38397240254](https://github.com/xy7365527-lang/NewChanlun/actions/runs/38397240254) 失败的也恰是这 3 个（`CI-DELTA.md`）。机制边界：失败断言是「笔为空」，而 `parse_layer` 先产笔再分段（`rust/src/theta_v0/parser/mod.rs:168-182`），#1414 只改段层，P2 只改 bin、合同与测试，结构上都不改笔数。口径沿用 [#1392 关票评论](https://github.com/xy7365527-lang/NewChanlun/issues/1392#issuecomment-6027480656) 与 [#1404 关票评论](https://github.com/xy7365527-lang/NewChanlun/issues/1404#issuecomment-6117508188) 登记的获批基线例外，后续归 [#1343](https://github.com/xy7365527-lang/NewChanlun/issues/1343)/[#1405](https://github.com/xy7365527-lang/NewChanlun/issues/1405)；本交付不改数据、断言或 ignore 来修绿。这两份例外是用户对当时具体 PR 的批准，#1414 合入时仍须用户就本 PR 一并确认。

关票声明（合入 main 后才能填写）：

`关票声明：resolution=<PENDING>，已并入 main；本评论各项在此刻均已为真；逐 commit 可编译性=<PENDING>；工位=<PENDING>；报告=.chanlun/review-results/issue1414-tb02d-20261012/ACCEPTANCE-REPORT.md；评审面==关票面（PENDING）。主线：origin/main @ <PENDING>；CI run：<PENDING> → <PENDING>`
