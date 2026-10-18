# #1394 整票静态工具整合报告（2026-10-18）

本次交付为可审查的控制与验收工具差异。两项 P0 HIGH 已修并通过新增定向回归；调度、持久日记、原件核验、独立财务不变量、内容闭包和监督控制核均已推进并留有轻量反例。**整票真实验收仍 BLOCKED**：正式同域 S terminal／γ／K 桥、真实 Voice、两角色 ARTIFACTS、完整实际请求与资源冻结、真实 OS 监督接线及精确单次准入未齐。本报告不签发 P11 或产品 PASS。

`execution_authorized=false`。S/E/B/Q、Long、完整市场历史、AC1..5、P11 全部 `NOT_RUN`。没有执行 cargo/rustc、真实财务库、产品服务、浏览器、资金/场所动作或网络产品操作；没有认领/评论/关票、commit/push/merge、切分支、改 Git 配置或新开工蜂。当前 checkout 外的原封包与生产 Rust 未改。

## 来源角色

工具宿主为 `codex/1394-acceptance-integration-20261018`，基线 `3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd`。它不是 producer 运行来源。原 driver 静态来源仍为 clean b46ba296…／tree f8733c32…；采用指定 P0 v2 和 scheduler v1 原件进行本次新写集整合。七个指定 pin 均实读核对一致，复核结果与源码摘录在 `SOURCE-CHECKS.json`，完整角色在 `SOURCE-ROLES.json`。

producer 末块一字节窄修 `99e819192bd6c91b1dd90e058a93c2a98d5aaaf3`／tree `955cf8fabda358b94619714198e52916ac0738ff` 仅登记为管理者提供的已静审独立来源；未并入本工具或任何执行 source，未运行。新的 Python 闭包核验器依当前类型/帧协议独立编写，不导入该 Rust 修改。

已读 XERJ 原查询/响应，实际命中的 Nautilus 确定性 ID 与固定处理顺序只作控制身份设计参考；`references/XERJ-REFERENCES.json` 保留查询原件，不替代本仓协议和资金判据。原 b46 快照中 s_content.rs、sequence.rs、finance.rs、financial_content.rs、store.rs 的相关声明与源码摘要已归档为 source anchors；没有重装或索引大树。当前没有可用 MCP 图谱工具，代码定位按允许的回退方式完成。

正确协议为 **economic-manifest/1 + economic-session/2**。准备合同 ce719894… 原字节保存在新工具目录的 fixtures 中。9011 为 synthetic TestOnly 长域；#1374 的 50070 Voice 是不同域，不能拼接。现有 A–F 输出未被当作 G terminal。未构造正式 Voice。

## 两项 HIGH

**F1：冻结摘要不再被第二次观测替代。** `OwnedProcess.binary_before` 取 command 中已冻结的 SHA；构造前、ACK/OWNER 落盘后放行前、finish、kill、cleanup 都对同一个预期值比较。新增测试在第一次真实 hash 返回后真实改写 toy shell 文件，确认 Popen 前拒绝、marker 不存在、消费不退还；另一测试在 ACK 落盘后真实改写文件，确认 gate 未放行、已建 child 被回收、冻结 SHA 保留、清理错误和原件保留。它们不是“第一次就 mock wrong hash”。前后哈希仍不是原子 exec 证明。

**F2：只有本实例拥有的稳定目录才可写原件。** `OwnedDirectory` 在成功 mkdir 后持有 directory fd 与 dev/inode；每次写前后核路径身份，以 `dir_fd`、`O_EXCL`、`O_NOFOLLOW` 写入。mkdir 失败不取得所有权，不写 CLEANUP；异常携带 cleanup receipt 返还调用者。新增反例覆盖既有空目录、sentinel 旧目录、文件、symlink，以及注册 gate 后把目录改名并在旧路径放新目录：原路径无污染、未放行 child 回收、失败回执不丢。成功创建后的 ps/ACK/hash 失败仍保留该实例清理原件。文件系统同步 syscall 与目录创建/打开之间仍存在 OS 边界，未宣称完整原子目录创建或不可中断 I/O。

新增 F1/F2 共五个测试方法，含多路径子例。涉及真实 gate 的新增测试在此沙箱中显式注入 fake 身份/ACK；文件改写、Popen、阻止放行、信号和 wait/reap 仍是真实的。它们证明所测控制路径，不证明真实 ps/外部 ACK。

## 整票可独立部分

| 原义务 | 本次实现及轻量证据 | 真实状态与剩余条件 |
|---|---|---|
| 固定来源/原件/输入准入 | P0 SHA/原件门、准备合同原字节与七 pin；旧 source 与新工具宿主分列 | NOT_RUN；缺当前正式同域输入、源角色产物与精确准入 |
| build_artifact_identity_check | 保留产物摘要/合同/命令精确匹配，失败先消费 | NOT_RUN；未构建，两 ARTIFACTS 与精确 toolchain 缺失 |
| S_writer_input_and_finish_custody | P0 gate、wait/reap、DB/WAL/shm custody 与共享 ReadBudget；toy 已测 | NOT_RUN；完整 S writer 命令/输入/帽未冻 |
| S_financial_session_startup_and_READY | 保留当前 READY/STARTUP、binary、anchor、resources 与三点版本核验；共享预算接线 | NOT_RUN；真实 SessionClosure/STARTUP/监督 ACK 未取得 |
| fresh_A/B_full_long_normal | 原合同逐项展开；前驱、manifest、DB、owner、epoch、cut、request/receipt 绑定；请求中的第二 owner DB 也纳入 A/B 互斥 | NOT_RUN；完整真实有序请求全集、同域 Voice、独立新进程原库均未取得 |
| cap80_joint_feasible_testonly_domain | 独算 baseline+各计划数量×价+费尾、整 slot 等待、不新增 grant、私有锁保留；只用手写 toy 原件 | NOT_RUN；toy 90/80 与 90/100 不构成实际资金实例或资源批准 |
| fault_matrix_before_after_commit | 从原合同六操作 × 两持久缝 × A/B 原量词保留 24 组；原 marker/kill/custody/new-epoch 控制路径保留，恢复原件集合拒绝反例已测 | NOT_RUN；未执行 24 组产品故障恢复，原三项 OS 依赖 P0 测试在沙箱中受阻 |
| owner_unavailable_and_common_storage_unknown | scheduler 保留全部三个具名额外域；责任归属、未知监督读数、未登记后代与恢复原件缺漏均拒绝 | NOT_RUN；真实不可用/共同库未知/离线 S 三域仍缺产品证据 |
| same_source_query_history_watch_original | `evidence.py` 核有序原件全集、幂等出现序号、多 owner/decision、同 cut、额外/缺失/重排；临时 SQLite 读取已测 | NOT_RUN；真实 message-key→receipt-key 映射、非持久错误回复与 Watch/History 完整解码 adapter 待接 |
| offline_S_absent_recovery_and_oracle | 新 financial_oracle 独算逐提交中间态，不导生产 reducer；闭包独立有类型遍历；恢复不重不漏反例 | NOT_RUN；真实 S 离线原库/本地节点/完整财务、结构语义与独审未取得 |
| cleanup/seal 独立入口 | P0 所有 owner 统一 cleanup_all；toy ControlBridge 接持久日记；显式拥有根目录和独立冻结预算才写 SEAL；失败不覆成功 | 轻量已测；真实整进程树、OS 硬界、最终产品封包仍未验证 |

原量词仍是 30 个模拟 scenario／510 个静态步骤、24 故障组、5 负域、10 compare，计数不代替任何一项原义务。5 负域为 idempotent_duplicate、stale_writer、identity_conflict、insufficient_cap_wait_without_new_grant、abort_committed_rejected。10 compare 为 canonical_commits、original_receipts、original_requests、accounting_components、q、fixed_Q、locks、liabilities、unique_closure_nodes、same_cut_queries。三额外域名称保持原准备合同；没有自行缩短全域或改变重复次数。

### 接口和失败封闭

`Scheduler` 只能接受显式 `SimulationAdapter`，结果仍标 `SIMULATED_COMPLETE_NOT_PRODUCT_ACCEPTANCE`。`ControlBridge` 不继承该类型，改 `mode` 不能通过准入；其 `perform` 对缺正式桥的产品阶段直接拒绝。它已实现具名 toy 的 P0 启动/结束/回收/封口和共享读取预算接线，不会返回模拟业务 proof 标签。原件核验结果仅 `ORIGINALS_VERIFIED`，不会直接满足 scheduler 的业务验真要求。

`Journal` 使用同一 inode 的跨进程 flock 和只追加 fsync 链，先持久消费再允许控制动作；前驱、step、完整 plan hash、原截止、字节/记录帽一起冻结。失败或中断后的 pending 保留；重开不补跑、不退款、不换帽。两个实际独立解释器争同一阶段，只有一个消费成功；重新打开、损坏尾部、fsync 失败、路径替换均有拒绝测试。它不是权限对手可重写全部历史时的防篡改系统，也不代表宿主重启/物理介质崩溃恢复已经验证。

财务核验要求外部冻结原始资金、初始账、完整操作日程与 seq，不能从 actual 倒推。按 decision 键保存结果，独算累计锁/最坏量/权利版本、保守责任、整 slot 等待与 reserved/committed/aborted/closed 中间态；核相同 owner 原开局整账不变，含 q/Q/成本/收入/unknown。Abort 与 Close 不退款；E 先保留责任，B Commit 时间不得在其前。读取与核账遍历沿同一 ReadBudget，不以 close 代预算。

闭包从当前 s-content/1 帧与有类型边独立展开，校验 header、分支唯一形状、字节块末端、UTF-8、精确数字 token、对象键序与唯一性、json_text 后代语法、ContentRef kind/extent。按 `(摘要, 类型上下文)` 记忆；收到节点与持久节点原 bytes 严格相等；少一节点即使 local cache 有也拒绝，多一不可达节点也拒绝。1100 层图采用显式栈，不用 Python 递归。未接发布模板位置/出生表时一律拒绝模板，不能打开宽松开关。这里只证明内容闭包，不替代六族/七变化集合、完整业务来源承诺和正式 γ 语义。

监督实现包含脚本/worker 摘要、pid/start/group/parent 身份、阶段帽/同截止、nonce、owner/后代显式注册、持续采样、心跳失联、传输 EOF、取消、TERM→KILL、逐项 wait 与失败不改成功。持续采样也重核脚本字节。未知后代不自动收养或乱杀；已知 owner 会回收，整体仍 INCOMPLETE。**本次没有完成真实 OS Probe、Unix 服务与产品进程树集成，也没有 cgroup/job-object 等 RSS/磁盘硬界**。核心 Probe 的后代与资源测试使用 fake；另有一个实际短命 toy child 的信号/wait/reap 测试。`monitor` 可运行于外部进程，但本次未启动产品外部监督服务。

## 实际验证

最终 20 个 Python 文件均通过 AST/编译语法检查；非测试代码无 `assert` 准入；原 P0 41 个测试方法与 scheduler 27 个测试方法名称全集保持。原测试仅适配短 `/tmp`、新目录 fd 所有权夹具、当前 fixtures 路径与显式 SimulationAdapter 基类，没有改钱/帽断言。新测试另补 55 个方法；总发现 123。

| 命令 | 实际结果 | 原输出 |
|---|---|---|
| `python3 -B s_session/tests/issue1394_acceptance/run_checks.py` | 123 方法；未全过：3 方法受沙箱阻断，其中一个含两次 error 子例 | `normal-final-all.txt` |
| 同命令加 `-O` | 同样 123 方法及相同阻断集 | `optimized-final-all.txt` |
| 普通命令加 `--offline` | 120/120 通过，明确排除上述 3 项 | `normal-final-offline.txt` |
| `-O` 命令加 `--offline` | 同 120/120 通过 | `optimized-final-offline.txt` |

普通与优化模式不是 240 个不同业务断言。精确命令、解释器路径、耗时、退出码、方法名及排除原因在 `TEST-RESULTS-FINAL.json`；先前一轮 122/119 结果及输出也原样保留。最终三项受阻测试：

1. `test_owned_toy_marker_sigkill_and_reap`：Unix socket bind／`/bin/ps` 被沙箱以 EPERM 拒绝。
2. `test_supervisor_real_unix_ack_and_mismatch`：Unix socket bind 被拒，ACK 等待失败。
3. `test_real_gate_ack_loss_and_late_disk_reap_without_release`：两子例均在 `/bin/ps` 被拒。

没有以 mock 替换这三项原测试来声称通过；`--offline` 显式列出排除项。新增 toy 测试中 fake 身份/资源与真实文件/进程控制边界在测试注释及本报告中分别说明。所有本次创建的 toy child/解释器均 wait/reap，临时 SQLite 显式关闭；未回退到真实数据库。静态检查结果在 `STATIC-CHECKS.json`；未安装或声称运行 Ruff/mypy。新字节独立审查尚未进行，references 的独审仅适用于旧包。

## 精确剩余缺口

1. 正式 S terminal／γ／完整 K 载体到同域真实 Voice 的桥与独立 authority 原件。9011 没有合法 γ 时必须如实拒绝，不能借旧 50070 单根权威或 unit 夹具兜底。
2. 当前 producer/consumer 执行 source 的明确选择、两份 ARTIFACTS、toolchain、新源与工具字节的独审；C1 窄修不会被本工具自动纳入。
3. 全域实际有序输入/请求/回执键与金融核账冻结原件，真实消息持久键映射、错误回复、Watch/History、源语义与完整闭包 wire 页的 adapter。现有有类型纯接口不冒称端到端真实接线。
4. 根冻结全部 session resources/3、reader resources/1、Job 八维、Q 原18项与三 Delta 帽、E/B transport、各阶段与整次最早截止、cleanup/seal 预留和资源帽。准备合同中的 null 一律不是配置；本次没有采纳任何历史提案为新授权。ReadBudget/closure 计数也不是所有 native heap/RSS/磁盘的完整硬界。
5. 真外部 supervisor 的 OS 进程树注册/持续计量/失联回收/服务传输原件与独审；当前沙箱还需具备 ps/Unix socket 能力才能运行保留的三项原 P0 检查。未请求扩大权限或换判据绕过。
6. 完整 fresh A/B、24 真实故障组、3 真实额外域、5 真实负域、10 全比较、正式 Long/完整历史/资金链/隔离/恢复，以及 AC1..5、旧 null 冷读法逐片退场的证据对账。不得用本次 toy 结果销账。

前端 #1460 与 K4 排除边界不变；#1394/#1361/#1340/#1323/#1279 均保持开放。所有新文件为本票工具测试资产或活期绑定 #1394 的 review-results 工作草稿。管理者收包后仍需针对固定新字节独立审查；本次自检不等于独审通过。
