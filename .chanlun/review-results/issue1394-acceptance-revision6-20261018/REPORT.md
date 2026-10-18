# #1394 R-F5 实际采样路径窄修06

独审05新增的采样边界 MEDIUM 已作窄修，作者定向自检通过；新字节待管理者固定并独审。独审05对固定旧版本的 BLOCK 保留。本包不宣告整票、P11 或产品通过。

本目录名分：绑定 #1394 的活期工作草稿。固定干净基线 `4b3a84ebba8178e03aa04b2bd2a6a4138d9d438a`，工作区 `/Volumes/AgentStorage/issue1394-sandcastle-integration-20261018`，分支 `codex/1394-acceptance-integration-20261018`。父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/implement_1394_scheduler_v1`。执行者是 Sandcastle 外部 Codex 工蜂，真实 session_id 和实际退出由运行时登记，不声称已注册为父任务的原生子代理。

## 修复范围与因果

源码只改 `s_session/tests/issue1394_acceptance/supervisor.py`（8行新增、7行删除），新增监督锁 `test_review05.py`（5个方法）；`test_review04.py`、product_capture、captured、Q序、driver/scheduler、其他实现和旧报告均保持原字节。

旧路径在 `send_boundary` 先读 clock 判断未到期，再调用会重新读 clock 的 `tick`。两次读数跨过采样点时，tick 绕开有限排队处理，直接检查后代，误拒已排队的合法登记。现在移除发送层的独立采样时间判断，tick 在完成原 work/heartbeat/worker 身份检查并判定实际到期后，才调用内部 `sample_boundary` 回调：

- 尚未收到终止标记：沿用原 `drain(receive(0))`，处理有限立即可读消息后经 `_sample_and_ack` 强制采样，新增 ACK 追加到原顺序。
- 已有 COMPLETE/CANCEL：已有终止边界直接强制 tick 采样，不读取终止后的消息；仍由原发送后检查判工作/心跳耗尽。
- 未到期：tick 仍执行原截止和身份检查，只省去本来尚不需要的实际采样。空接收路径已经观察到空队列边界；直接 ACK 和初始排队路径维持原强制采样。

因此发送路径只由 tick 作到期决策，实际采样服从有限排队或已确认的空/终止边界。`sample_interval`、所有资源帽、原始 `receive_deadline`、阶段截止和身份判据未改；排队心跳不能给当前发送轮续期。没有通过固定 fake clock、epsilon、放宽未知 PID 或取消采样消除误拒。

`drain` 逐字保持。除 tick 外的8个 SupervisorCore 方法逐字保持；移除 tick 新增的回调参数和分派 if 后，其原截止、身份、实际采样及异常处理 AST 与基线相同。STOP/TERM/KILL/reap 仍走原入口，只处理已登记且身份核验通过的 owner。

## 正式反例与定向结果

正式原件先核长度/SHA，再原字节复制到 `references/`：

| 原件 | 字节 | SHA256 |
|---|---:|---|
| REVIEW-05-SUPERVISOR.md | 3737 | `98e40846756c95359964a7bd54309c28ddb37a1268bcf2b40d57c529d0a7ad5a` |
| REVIEW-05-SUPERVISOR-PROBES.json | 99404 | `dcb19ae7681bb71ce325245c5de2626a9f5bc9d7a210ea1aeb759fca76df26f4` |

全部7条原探针的命令/输出保留在 JSON；只重放索引5的新增竞态和正例，其余旧检查不重跑。提取代码保存为 `FORMAL-PROBE-5-SOURCE.txt`，按原字节 compile/exec。精确历史基线 `576a13470566a5ca4625eae3d4e89abcb950e346` 由只读 `git show` 预取，执行时仅将原脚本的这一条 git 子进程读取替换为相同原字节的内存返回；禁止派生真实进程、网络和数据库。不切 checkout，不改探针 clock 或判据。原源码与执行包装分别留存，原始修前输出与正式 JSON 的 stdout 解析值完全一致。

| 场景（每读 clock 前进2微秒） | 修前4b | 精确基线576a | 修后未提交字节 |
|---|---|---|---|
| count=1，delay=.124995，最后emit返回 | INCOMPLETE；四消息未读，仅登记/回收101 | CONTROL_REAPED | CONTROL_REAPED；101/103/104登记、实际采样、回收，队列空 |
| count=2，delay=.124985，下一ACK之前 | INCOMPLETE；仅E ACK，四消息未读 | CONTROL_REAPED | CONTROL_REAPED；E/B ACK，101/102/103/104登记、实际采样、回收 |
| count=1，delay=.1245，近边界正例 | CONTROL_REAPED | CONTROL_REAPED | CONTROL_REAPED |

新锁另覆盖 count=2 的近边界合法正例，以及未到期 tick 的 worker 身份漂移拒绝、tick 读钟跨越原心跳截止拒绝。正式原反例的 stage=9/work=10/heartbeat=1 和 sample_interval=.125 均保持。新测试检查实际采样包含全部合法后代、队列耗尽、登记 owner 回收及持续推进时钟，避免以跳过采样制造绿灯。

执行前 `TEST-SELECTION.json` 已列影响面及具体方法：**24旧＋5新＝29个不同方法**。修前只跑5个新方法：两条正式反例失败，3个保护方法通过。修后普通与 `-O` 各 **29/29** 通过，各0.015秒；不是58个不同方法。24个旧方法包括独审05已闭合的发送前/返回后 HIGH、work/heartbeat/阶段等号和超期、采样耗尽截止、RSS超帽、陌生PID、有限心跳队列、终止后登记、合法排队和 tick 默认/强制/空接收路径。

首版尝试普通/-O均28/29通过：终止边界调用 `_sample_and_ack`，虽正确拒绝心跳耗尽，却将原“心跳”报错变成通用 ACK 过期。改回终止边界强制 tick、原发送后检查后通过，未改旧测试。`attempt1-*` 保留该次源码、指纹和实际失败输出，不把失败混入最终通过计数。

原命令、输出、退出码和每轮前后31份 Python 指纹在 RED/NORMAL/OPTIMIZED-RESULTS 与 FORMAL-PROBES-BEFORE/AFTER；汇总为 TEST-RESULTS 和 FORMAL-COMPARISON。新增测试从红灯到最终绿灯字节未改；各次执行期间源码指纹一致。RUNNER/RUN-FORMAL 的 `.txt` 是可执行检查命令原文工作草稿，不是产品模块。它们只运行列出的有限 fake，并拒绝 subprocess.Popen/socket.socket/sqlite3.connect。

## 静态保留与交付

变化的2份 Python 已做 AST 与内存 compile（optimize=0/1）。29份未变 Python 按相同长度/SHA复用既有静态证据；原41个driver、27个scheduler方法及整个文件保持。当前 captured_cut/test_capture_bridge/test_capture_order 属后续Q序字节，匹配 Q序封包的30文件指纹，不套用较早 revision5 的源摘要。169份旧报告按长度/SHA核对不变，不复制旧整包。详情为 STATIC-CHECKS、TEST-REUSE、BASELINE-PYTHON、BASELINE-REPORTS。

`git diff --check` 通过；新文件另检查行尾空白。交付清单 `DELIVERY-FILES.json` 绑定代码/测试/本包每项的路径、长度、权限、SHA，不自引自身或未来 commit。源码身份：

| 文件 | 字节 | SHA256 |
|---|---:|---|
| supervisor.py | 15944 | `bd67216a54b35e86d3cd432ff2b1ab8debbff3e4db391e6d86abfb3d5954b603` |
| test_review05.py | 3243 | `7e1c2508edde58aa715ef78bf85fe25de792c8e34a5a3222a800e6b5bf624eae` |

已读 AGENTS/CLAUDE、wayfinder、CODEX-CHILD、ask-matt、世代宪法、revision5 REPORT/FIX-LEDGER、supervisor/test_review04、独审05材料。按 diagnosing-bugs 红绿流程执行；正式原因和反例已给定，省略重新发散假设。无可用图工具，按已知路径及必要文本检索回退。未触碰独审 audit repo；未认领/评论、commit/push/merge、切分支、改Git配置、开启工蜂或读取凭据。

## 仍保留的产品义务

`execution_authorized=false`，产品 `NOT_RUN`；这仅指未获产品执行准入，本次有限监督修复已获授权。未运行真实子进程/OS Probe、网络、DB、cargo/rustc、S/E/B/Q、Long、浏览器或资金；未重跑40/38/37/68/91/82/host123、Q序36或ps/socket集合。

统一source及financial_reader组合语义、两ARTIFACTS、真实同域terminal/完整K/source/独立γ、逐owner Voice与独评、OS/资源/根准入、真实控制传输、fresh A/B、24故障、3额外域、5负域、10compare、全历史、资金、隔离、恢复义务全部仍欠。同步emit内部不能被本修复硬中断；返回后拒绝越界的责任保持，receive(0)非阻塞与真实OS限制仍需产品证明。

read_guard 仍是候选，不升级 spec/authority；Journal 继续保守隔离 candidate，重开不恢复接受资格，恢复代价不变。前端归#1460、K4排除；不勾P11，不关#1394/#1361/#1340/#1323/#1279。缺陷逐项名分见 FIX-LEDGER。作者自检结束后退出，由管理者收包、固定并独审；本文不代写实际退出事件。
