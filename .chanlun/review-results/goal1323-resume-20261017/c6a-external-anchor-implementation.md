# #1406 C6a 独立时钟锚实现候选

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。主控已批准 `c6a-external-anchor-plan.md` 的工程实现方向，尤其批准 work 原点保守前移；本报告不是独立审查或运行准入。

**已实现外层 broker 与 driver 的五步独立锚装配，监督器 51 项、driver 61 项纯替身测试通过。真实运行继续禁止：verify_runtime 仍为 PENDING，CONTRACT_READY/SEMANTICS_READY 仍为 False，source/tree 仍为 PENDING。**

## 修改归属与精确字节

工作树：`.sandcastle/worktrees/resume-1406-c6a`，旧 source 基线 `b6e690ecae5723fbbcb773f15400c71d1010da87`。本轮只改下列四文件，没有修改 workflow 或语义器。

| 文件 | SHA256 |
|---|---|
| `.github/scripts/run-1406-c6a.py` | `fe6392b8d1be652d2be0ac6b5ae80d3597cf253361da2c3723e7fa8069a783f1` |
| `s_session/tests/tb03c_query_runtime.py` | `bafc25a7bdeb899d32174e06f7c6ccfbc81d82f74d852876167bf22a12e4f395` |
| `s_session/tests/test_tb03c_query_runtime_contract.py` | `abd9d67a6f4e325366b7044783e526ebc1c55c20ec5e21000ce8259e5fcf311a` |
| 证据根 `test-c6a-linux-supervisor.py` | `01c84436435063fbd2b9a9ed32ee486d60234cdce50bf093a953e2c1ae130ff6` |

runner 是既有未跟踪准备件；driver 与 contract tests 为已跟踪修改。没有 commit/push/dispatch，也没有访问外盘、cargo/rustc、产品服务、HTTP 或浏览器。

## 已落行为

runner `ClockBroker:193` 使用单个 Unix SOCK_SEQPACKET；`LinuxSupervisor.spawn:388` 将原有 `started=clock()` 的同一个值，在 runtime Popen 之前交给 `broker.arm`。传入 child 的 bootstrap 绝对期限为该值 +600；600/630 的起点从原 driver `main` 第一行前移到外层 spawn 采样，解释器启动、参数解析及首次 IPC 都算入原额度。**这是明确批准的保守前移，不是恢复旧实现字节。**

固定顺序为 work、we5-case、we5-q、we7-case、we7-q。每次 tick 至多一收一发，packet 最大1024字节，只保存五请求/五应答，不做文件 I/O、hash、派生进程或无界 drain。run_id 防串线，不冒称密钥或密码学鉴权。重复键/乱序/重放/超包/附带 FD/提前 EOF/第六请求/响应断连等拒绝；收到 SCM_RIGHTS 时先关闭已接收 FD 再拒绝。

work 只领取预置起点；case 保留旧 case 的先 check 再切换语义；Q 的 deadline 始终为 `min(issued+10, case_end, work_end)`。应答 EAGAIN 仅重试固定字节，不重采时间。已签期限之后等待、调度、Q spawn、解析与就绪验证仍消耗原期限。

`clock_tick:411` 在所有 `run:485` 等待循环执行，因此 `probe()` 的 monitor=False 嵌套等待也推进同一 broker。broker 错误先写父内存失败态，再由既有异常路径执行 cleanup；runtime finally 先关闭通道、停签发，再走原 WNOWAIT/TERM/KILL/reap。没有新后台线程、helper 或第二个 wait 者。已收到取消或进入回收阶段不能为了补全握手签新锚。

仅 runtime Popen 的 `pass_fds` 含 child 端。父进程在 Popen/登记后关闭 child 副本；driver 接管立即 non-inheritable。driver E/B/Q/git 继续 `close_fds=True`，不传 pass_fds。第五应答消费后 driver 关闭通道，父端须见 EOF；五步不齐即使 runtime exit0 也不获 PASS。

driver `ExternalClock:111` 强制核 bootstrap/work/case/q 的字段、有限时钟、序号、父期限和原600/630/300/10数值，正式 CLI 三个 anchor 参数全部 required。缺 FD 不得退回本地时钟。`Budget` 的无 anchors 构造仅保留纯替身单测入口；正式 main 总是绑定外层通道。`Case.start` 在原 Q spawn 前采时位置请求外层 Q 锚，其 proof/transport 直接使用返回值。

父内存 transcript 记录每行 request/response/received/issued/sent、run_id、实际 driver PID、spawn 原点、outer runtime 截止、EOF/complete/failure。它在 runtime 已结束或失败回收后进入父进程 runtime receipt，不落在每次 tick 的 I/O 路径。verify worker 通过外层 request 接收此原件，`verify_clock_transcript:314` 独立重算五步顺序、原期限和进程时间区间，再投影精确三键 `case_startup_anchors`。没有从 driver proof 或 transport 反取可信起点。另核 driver RESULT 原600/630与elapsed在父起止区间内；原语义器尚未嵌入，`verify_runtime` 始终抛 PENDING。

## 本轮验证

1. `python3 -B .chanlun/review-results/goal1323-resume-20261017/test-c6a-linux-supervisor.py`：**51项通过**，最后一次0.038秒。原32项在新候选上重跑，新增19项；其中原失败原件保全测试更新了 ClockBroker.create 替身，避免在macOS建立真实Linux seqpacket。
2. 在工作树 `s_session/tests` 运行 `python3 -B -m unittest test_tb03c_query_runtime_contract`：**61项通过**，1.790秒。原48项重跑，新增13项。
3. `git diff --check`：通过。driver及contract tests差异283插入/15删除；runner未跟踪，不以git diff统计冒充其覆盖证明，实际通过import与上述测试加载。
4. runner `--preflight`：输出 `BLOCKED_PREPARATION`，预期exit2；阶段端点仍1200/1830/1920/2040/2070，资源原值不变。

关键正反控包括：真实 broker 与真实 ExternalClock 通过全内存transport完成五步互通及三键投影；work请求延后20秒仍只剩580/610；Q响应延后9秒只剩1秒、10秒/11秒拒绝；we7不能覆盖已过期we5；父帽截短Q十秒；probe嵌套run真实调用broker.tick；收到取消、协议错误与Popen失败的FD和所有权清理；失败先停签再调用原reap；外部transcript起点/期限篡改拒绝；真实Case.start控制流下替身Q spawn延后20秒被原十秒界拒绝。

所有新增进程/socket/信号均为替身。初次重跑旧32项时，1项因未mock新增ClockBroker.create而在macOS的SOCK_SEQPACKET创建处失败，未到其原失败原件测试位置；现已改为显式替身并重新通过51项。不把该初次失败藏成产品故障，也不把本次纯替身通过说成真实Linux内核行为已验证。

## 基线、尾差与未完成项

driver及contract tests现在构成**新的待独审 source 差异**。旧 source SHA/tree 与旧语义器冻结来源不能给新字节背书；需完成新源独审并另行冻结。精确11源码列表未增减，原oracle/input字节未改。

新 source 独审后，最终 CI 相对此新 source 的尾差仍只能是原两发行文件 `.github/scripts/run-1406-c6a.py` 和 `.github/workflows/issue-1406-c6a.yml`。不能把当前driver源码改动误计为获准CI尾差，也不能保留旧source SHA却放宽身份检查。

旧独审监督器5项脚本本轮没有修改、没有重跑；它的真实运行入口替身也需mock新增 ClockBroker.create 后由独审工位重做。**旧32+5结论不是新字节证据**，本报告只签本轮实际51+61结果。

仍需：独立Python/监督器协议审查；真实Linux FD/组回收与取消的具名验证；语义器与新冻结源接口复审及嵌入；完整41步合法wire正控；新source精确身份与两文件尾审；资源/资金/隔离/恢复及全图验收。未开通真run，未关闭任何票。
