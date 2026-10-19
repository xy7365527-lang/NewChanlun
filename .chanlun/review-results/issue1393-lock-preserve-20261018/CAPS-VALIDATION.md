# #1393 根具名更窄帽：作者验证记录（非独审）

环境：macOS 本机，Python 3.14.6，仅标准库 unittest（另用 PyYAML 解析 workflow）；零网络、零真实子进程（测试将 `subprocess.Popen` 置为断言失败），未运行 cargo/rustc/rustup/lake/Lean/产品/浏览器。

命令（工位根目录）：

    python3 -B .github/scripts/test_issue1393_lock_prepare.py -v > .chanlun/review-results/issue1393-lock-preserve-20261018/CAPS-toy-final.stdout 2> .chanlun/review-results/issue1393-lock-preserve-20261018/CAPS-toy-final.stderr

结果：`Ran 80 tests ... OK`（完整输出见同目录 CAPS-toy-final.stderr/stdout）。run16 原有 47 项全部保留；其中 4 项按新语义改写而未放宽：记录完整性改为从摘录偏移取原文并对拍原 cache 偏移、摘录与原 cache SHA256；原「单文件/总读帽」两项改为「读上限先于打开」「registry 正文 total/files/items 边界」并新增「非 registry 原件不计入 registry 帽」；故障注入的 parse 阶段改挂流式扫描函数。

新增 33 项覆盖：根数值与保持值逐项（AST 读字面量）；合同逐层类型严格（single+1、float、True、0 代 False、未知/缺失嵌套字段、upload 计数口径、可变 @v4、compression≠0、消息帽改动、旧 schema/旧收据 schema、未知顶层字段）；runner 无网络导入、扫描函数无整读/子进程；单条原文等于帽完整与超 1 字节 proof_gap 且摘录无截断内容；大于 total 的 cache 文件因只摘所需记录仍完成且 `NOT_ARCHIVED`；total/files/items 等于帽通过、少 1 拒绝；跨文件累计项数；截断、非所需重复版本、缺 validator、空文件、坏 magic；扫描中途截止与信号；约束父与计数；容器公式对 Python ZIP_STORED 实测不低估；禁字符/非 UTF-8/ZIP64 门拒绝；收据预留为事前常数；目录/symlink 拒绝；写入超合计帽在打开文件前拒绝、等于帽通过；封包预核失败不写尾件不删原件；upload-gate 等于帽通过、少 1 拒绝且不写 FROZEN、请求在目录外；receipt-gate 合计等于/超 1、ACTUAL 超 4096、门后新增或改动散件、无冻结清单时仅主包未上传才放行；False 门、近截止、未知模式拒绝；workflow 两处上传固定 commit、compression 0、门调用与收据三件路径、无 RESOURCE_CAPS 文本解析、默认门 False。

另核：workflow YAML 可解析；两份 Python 源 AST 可解析；`CONTRACT-DRAFT.json` 字段集合与 FIXED+Git/摘要字段完全一致（无未知、无缺失），runner/workflow 摘要已更新，approval/review/prepared_* 保持 UNBOUND。

## 首次独审返修（review-01）

独审 review-01（Opus 5.5 CLI 工蜂，原件 `review-01/REVIEW.json`、`REVIEW.md`）对 `5583547b` 自报 `PASS_LIMITED_STATIC`，但同时列 3 项 HIGH；按评审合同「有 HIGH 即 BLOCKED」，负责人按 BLOCKED 处理，不把首审记为通过。返修：

- H1（EvidenceIO.check 合计上界检查无测试覆盖）：新增 `test_check_refuses_existing_output_over_combined_cap_equal_passes`，等于帽通过、少 1 字节时 `EvidenceIO()/check/take_items/run` 均拒绝且原件不动。
- H2（收据多文件 path 的 rootDirectory 未测）：workflow 测试断言三件路径逐字为 `${{ runner.temp }}/<名>`（无子目录，使 destinationPath='/'+名 与上界/门计数一致），主包 path 与 archive 步目录一致。
- H3（帽值与根原件无 CI 级一致性锁）：根原件副本 `ROOT-REGISTRY-CAPS-20261018.json` 入本目录并钉 SHA256 `d4c6e154…`；新增 `test_caps_match_in_repo_root_original` 对 runner 字面量、FIXED 合同各数值/模式/网络布尔/计数口径/保持值与 CONTRACT-DRAFT 逐层类型严格对拍。
- M1：`container_bound` 注释改写为「固定元数据 128B 不含名字 + 名字 LFH/CFH 各一次」。

作者变异复核（工位副本 `/Volumes/AgentStorage/issue1393-narrowcaps-20261019-review/author-mutations/`，副本缺旧 legacy 报告导致基线 1 项环境性 error，与变异无关）：删 check 合计上界、single 字面量改 65537、收据路径加子目录、上界名字只计一次、删 receipt-gate 目录变化比对，五个变异均使测试新增失败。

返修后全量：`Ran 82 tests ... OK`（CAPS-toy-final.*）。

## 第二轮独审返修（review-02）

review-02（新 Opus 5.5 CLI 评审者，原件 `review-02/`）对 `d75d28a4` 给 `PASS_LIMITED_STATIC`，确认首审 3 项 HIGH 闭合，13 个真实变异中 8 个被杀，另列 4 MEDIUM、4 LOW。全部处理：

- F1 失败路径绕过 registry 帽：新增 `bounded_registry_save`，失败时先把 `registry.json` 覆盖为失败记录（不增文件；放不下降为最小记录 `{"status":"FAILED","complete":false}`，必然小于完整摘要），再尽力写 `registry-failure.json`，放不下则不写，失败结论由 result.json/日志承载；upload-gate 与 receipt-gate 另行复核 registry files/total，超帽拒绝上传、原件保留。新增 3 项测试（files/total 帽下失败路径不越帽；帽已满时完整 registry.json 仍被降级；两道门拒绝 registry 超帽）。
- F2 占位先于门的顺序：workflow 测试断言三类占位写入都在 `upload-gate` 调用之前，门之后无写入；ACTUAL 写入先于 `receipt-gate`；步骤顺序 archive→payload→receipt→收据上传。
- F3 截断：新增「所需版本齐全 + 末尾截断的非所需段」三种尾巴均拒绝。
- F4 扫描循环内截止/信号：断言第 2 次读后故障时恰好停在第 2 次读（不会读完再兜底）。
- F5 同长度改写：receipt-gate 对同长度不同内容拒绝。
- F6 精确断言两处上传的 `if` 与 `timeout-minutes` 表达式。
- F8 恢复构造器分支 `opened.assert_not_called()`。
- F7（LOW，未改代码）：已扫描版本的去重集合只随文件内条目数增长，受同一 900 秒截止约束，只保留短 version 字符串（≤256B），未另设根未给的扫描条目帽；保留的版本记录仍受 items=512。

返修后全量：`Ran 86 tests ... OK`。

## 第三轮独审（review-03）与审后纯测试补充

review-03（新 Opus 5.5 CLI 评审者，原件 `review-03/`）对 `57f41578` / tree `ba94072c` 给 **PASS_LIMITED_STATIC**：副本基线 86 项全绿，24 个真实实现/workflow 变异杀 22 个；review-02 存活的 M4/M7/M8/M10/M11 全部被杀，F1 八个变异杀 7 个；无 CRITICAL/HIGH/MEDIUM，仅 5 项 LOW（R3-3 扫描条目帽与 R3-4 上传期间变化仅事后发现属已知限制，R3-5 为可选日志）。

审后仅追加测试、实现与 workflow 字节未变（runner SHA256 与 workflow SHA256 与 review-03 所审相同）：补 R3-1（完整失败值放不下时降为最小记录、最小记录也放不下则不写）与 R3-2（收据步 ACTUAL 4096 字面量锁定）。作者复核：两项存活变异 F1f、X6 现均被杀。全量 `Ran 87 tests ... OK`。
