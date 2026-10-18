# #1394 当前 driver 部分静态包独立审查（2026-10-18）

裁决：可接收为 `PARTIAL_STATIC_ADAPTER_NOT_P11_ACCEPTANCE` 工作草稿；**执行候选 Block**，先修下述 HIGH 项再进入完整调度/运行合同审。不得据此勾选 P11、真实 A/B、恢复、Long、资金链或关票。`execution_authorized=false` 保持。

审查对象：`/Volumes/AgentStorage/issue1394-real-driver-20261018`。未改作者包、主仓 WIP、producer 或旧证据。只在 `/tmp/r1394-9x4hoekg` 复制三份 Python 文件运行轻量测试；未运行 Rust、产品服务、真实数据库、Long、浏览器或外部动作。

## 证据身份与已验证范围

- MANIFEST SHA256：`d5dfa89176866d8ad6ee0b3be73bd9657c8c66ec493eeab6711d2a1079d51a50`。
- driver.py SHA256：`7599af7305679e05be0075757f1f6ef94e46c6c68baf2d1cadeae83a943b14ac`。
- oracle.py SHA256：`87f9537995e543810e180daa06a1193bcee25b328563bcbb0e8cb66b30d44e6d`。
- MANIFEST 31 文件大小/摘要全符；SHA256SUMS 32 项全符；七份 source reference 快照摘要全符。
- 独立运行 `python3 -B -m unittest -v test_driver`：26/26，0.143 秒；`python3 -O -B -m unittest -v test_driver`：26/26，0.144 秒。不是 52 项不同业务测试。
- 输出：`/tmp/r1394-9x4hoekg/independent-tests.json`；边界探针：同目录 `boundary-probes.json`。
- 首次复制到系统长临时目录的两轮各 1 failure/2 errors，均为 AF_UNIX 路径过长；换短 `/tmp` 后原字节通过。首轮输出保留在 `/var/folders/ft/1lyklpls4n16xj54q4l148qm0000gn/T/1394-driver-review-vkkdbe4s/independent-tests.json`，不是产品缺陷或产品失败。
- PATH 和主仓 `.venv/bin` 均没有 ruff/mypy/pylint/black；未安装工具，未假称这些分析已通过。已先查看 `git diff -- '*.py'`，确认主仓两份 Python WIP 是别线变更，未审改它们。

## 协议核认与正向结论

1. 正确名称是 **外层 economic-manifest/1 + protocol_revision=economic-session/2**，不是 economic-manifest/2。快照 `store.rs:20–26,105–112` 与 `finance.rs:53–132` 支持此形状。finance/2 exact 五字段、structure exact 十字段、SessionClosure 非 null、session resources/3 九键、reader resources/1 六键及 Job 八键均对上当前快照。
2. retained_bytes 不超过 268435456，reply 帽关系、混合 session、旧九字段、缺原库、原件 SHA、不可覆盖写入均有拒绝分支和局部测试。这不验证全 manifest/Voice 合法性。
3. `exchange_frame` 每次 connect/send/recv 重算同一最早截止；保留请求摘要及原回复，错误不生成成功业务回执。其 Received 仅为传输状态，未完成业务身份验真。
4. toy 单测使用真实 Unix socket ACK、真实门控 child、真实 marker、SIGKILL 和 wait/reap；并验证收到初始 ACK 前 marker 不存在。只证明这些控制原语，不能外推真实 S/E/B/Q 恢复。
5. READY 与 STARTUP canonical SHA、binary/pid/db/resources/anchor 绑定对应快照；capture 三点不是请求期四点。A/B fresh 目录与逐域 actual 比较没有 expected→actual 复制路径。
6. oracle 保留独算公式，不导入生产 reducer；原 assert 改为显式异常，在 -O 下仍有效。完整业务量词、同 cut、中间态、closure 独立遍历尚未实现，报告已披露。

## 必修问题（均为 driver 自身，不归 producer 最后节点边界）

[HIGH] 监督 ACK 的截止没有覆盖全部步骤，晚到 ACK 仍能放行
File: driver.py:331（同 :317–319、:345–357、:445–451）
Issue: remaining 在无 timeout 的 ps 之前计算；connect 和 sendall 共用旧相对 timeout，二者会各花时间。recv 前虽再设剩余 timeout，但收到帧、解析、返回后没有截止复查，门控写 stdin 之前也没有复查。独立 fake-clock/socket 探针在绝对 deadline=10、recv 返回时钟=11 时仍获得 ACK。该探针证明控制分支漏检；不冒充真实网络超时实测。根还要求从 ACK 到实际放行/故障动作保持同一截止。
Fix: 一个受界 remaining helper 覆盖 ps/connect/send/recv/解析返回与执行动作前；ps 必须传 timeout，connect 后/send 前重新计算；晚 ACK 或落盘耗尽窗口一律拒绝并进入预留 cleanup。`process_identity` 同样接受固定截止。用 fake clock 验证各段耗尽预算和 ACK 后落盘延迟。

[HIGH] 失败清理会被正常退出竞态中断
File: driver.py:519（尤其 :522–525，及 :316–317、:462–465）
Issue: cleanup 首次 poll 看到活进程后，verify_identity 再次 poll 可能已经退出，直接 Rejected；若在 ps/getpgid 之间退出也可抛错。探针首 poll=None、次 poll=0，结果 `owner 已退出`，stdout/stderr 均未关闭且无清理收据。此时不应把本次正常退出当陌生 PID，更不应阻断其他 owner 清理/封口。
Fix: 区分“已回收的本 Popen child”和“仍活且身份匹配”；退出则 wait/reap 并记录，不再信号；进程消失竞态重核 Popen 状态，不误杀。用 finally 关闭本实例所有流，保留失败原因/回收结果；单 owner 清理失败不得阻断其余登记 owner。构造、ACK 失败和 binary 重哈希失败路径也须进入此统一收尾。补主动退出、ps 时退出、ACK 丢失、TERM 已退出/超时等测试。

[HIGH] SQLite 连接未显式关闭
File: driver.py:281
Issue: `with sqlite3.connect(...) as db` 只管理事务，离开块不会 close connection。rollback 后连接生命周期仍依赖 GC；这违背原件读取必须完成并释放句柄的回收要求，失败 traceback 还能延长占用。独立接口探针确认 rollback=True、close=False。未打开真实或测试数据库。
Fix: 外层 `contextlib.closing(sqlite3.connect(...))` 或 try/finally `db.close()`，内层显式事务/rollback；成功、strict JSON 失败、查询失败均验证 close。busy/progress deadline 与累计读取帽属于下节已披露缺口，须同批接入而不是靠 close 代替。

[MEDIUM] strict JSON 仍接受指数溢出的非有限值
File: driver.py:54–63
Issue: parse_constant 只拦 NaN/Infinity 词法，`strict(b'{"value":1e999}')` 实际得到 `inf`。当前多数金额/资源字段要求字符串，故不外推成已发生资金准入；但任意响应及 DB JSON 已经过这个入口，名称与承诺不符且 canonical 会晚些抛错。
Fix: parse_float 检查有限性或按协议拒绝 JSON 浮点；补正负大指数、嵌套值和重复键测试，保持原数值语义。

## 完整接线缺口：已披露，不能误报已通过

- **同域完整 scheduler/Voice**：当前没有阶段调度器；只有候选命令准入格式。必须 writer 完整 Long→回收→custody→financial READY→E/B 新域及同域 Voice→业务/查询/历史/原件→离线恢复→统一回收封口。普通 unit Voice 和异域 Voice 仍不能借用。
- **准入生命周期及防重复**：single_use_receipt 仅阻止第二次构造 ExecutionAdmission；同对象 `check_command` 同 stage/epoch 连续两次均通过（轻量探针已核）。应在准入/登记层维护一次性阶段消费、活 owner 注册和前驱收据，明确失败是否消费；恢复只能用合同中新的 stage/epoch，不能靠“scheduler 应该不重复”空约定。此为当前部分候选未完成的执行接线要求，尚未发生产品重复启动。
- **原件与业务绑定**：source_roles 只核指针文件 SHA，未递归核其语义；oracle.check 只检查 admission 对象类型，没有证明传入 manifest/DB 属于该合同与当前阶段。scheduler 必须把 manifest、原库、owner、epoch、请求/回执键、操作类型和回放 cut 绑定完整，否则不得把读取成功或 Received 当步骤 PASS。原件读取、哈希、SQL busy/progress、行/字节累计必须消耗固定 Job 的同一预算和截止。
- **独立 closure**：还没有从目标根独立解析有类型节点/边并逐原字节遍历，与接收流节点集合严格相等的实现。不得相信生产 PASS 或 local cache 代替该量词。
- **外部 supervisor**：当前只是 ACK 客户端，没有外部 PID/脚本 SHA/独立采样、owner 后代登记、RSS/日志/磁盘/失联处置。toy echo ACK 不是资源监督证据。当前只 kill 直接 child；若后续命令会产生后代，必须禁止这种命令或把后代登记和回收纳入合同，不能声称已回收整进程树。
- **资源对象未整合**：session/reader/Job 形状通过不替代 Q 十八旧帽和 Delta 三项、E/B frame/timeout、旧 basis 16MiB/10秒、总体时间/封口窗及原库累计帽。全部按已有合同独立计量，不能互相抵扣或暗增预算。
- **oracle 数学覆盖**：当前仍以初始/最终图像、单个 slot 和按 decision 查找为主；额外/重复决策、责任归属而非总额、原始请求/回执全集、锁与 Q 的中间态、恢复重送不重不漏须按具名完整域补独立断言。observations 以 owner 为键会覆盖同 owner 多 decision 的展示，不得据其数量判完整。
- **默认拒绝边界**：CLI 只 inspect；oracle CLI 硬拒绝；OwnedProcess 没 admission 在 Popen 前拒绝；准备 false 不会自行变 true。该正确默认门不等于当前候选格式已经可以授予真实运行。

producer `financial_reader.rs` 最后 done=true 前缀预算问题已另有窄修任务，本审不重复列成 driver 缺陷。本包没有尝试绕过它。

## 建议优先级、写集与验收

| 优先级/工位 | 独占写集（在新版本目录保留原封存包） | 必须交付 |
|---|---|---|
| P0 控制原语修复 | 新版 driver.py、test_driver.py | 上述三项 HIGH 与 strict 溢出修复；阶段消费/owner 注册；同截止和清理竞态反例。独审只审尾差，并复用原 26 项 |
| P1 完整调度 | 新 scheduler.py、scheduler_tests.py、具名阶段计划草稿 | 当前协议阶段状态机，原件/命令/前驱/epoch 的绑定、A/B/故障矩阵；以 fake adapters 验证，产品门 false |
| P1 独立核账与遍历 | 新版 oracle.py、closure_oracle.py、独立测试 | 当前完整域的决策/权利/责任/锁/Q/回执量词及有类型 closure 全集比较；无需真实 DB 或产品即可先做拒绝反例 |
| P1 外部监督 | 新 supervisor.py、supervisor_tests.py、监督合同草稿 | 脚本/进程身份、持续资源采样、失联/取消、同截止、进程树登记、失败独立回收封口；先 toy 验证 |
| 根准入/整合 | 最终来源/输入/Voice/资源/工位接口及运行合同 | 消除所有 null 与语义占位，核真实产物另审，固定精确单次运行；只读静态包不能替代此步 |

公共控制 API 应在此次修复加具体参数/返回类型（TypedDict/dataclass 等）以明确合同、命令与收据边界；不要用 Any 覆盖所有对象。P1 三个工位以 P0 固定接口为输入，并行编写隔离模块，不能同时改 driver.py。

验收顺序：先 P0 差异独审及有意义的轻量反例；再整合三个 P1 工作面进行完整静态合同审；真实构建/Long/A/B/故障执行分别服从根的精确准入。原件保留，产品状态仍全 NOT_RUN，#1394/#1361/#1340/#1323/#1279 不因此关闭。
