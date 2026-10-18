# #1394 R-F5 发送边界窄修05

两处发送回归已修复，作者定向自检通过；新字节等待管理者固定快照并独审。独审04对旧版本的正式 BLOCK 保持，不据本包宣布整票、P11 或产品通过。

固定基线 `576a13470566a5ca4625eae3d4e89abcb950e346`，工作区 `/Volumes/AgentStorage/issue1394-sandcastle-integration-20261018`，分支 `codex/1394-acceptance-integration-20261018`。父任务 `01a12e01-45a1-7173-a3e8-d338d108fb93`，管理者 `/root/implement_1394_scheduler_v1`。执行者是 Sandcastle 启动的外部 Codex 工蜂，实际 session_id 由运行时登记，不声称注册为父任务的原生子代理。本目录是绑定 #1394 的活期工作草稿。

## 修改及逐项结果

源码只改 `s_session/tests/issue1394_acceptance/supervisor.py`，新增 `test_review04.py`。`monitor` 在每条实际 `emit` 前核原 work/heartbeat 截止、完成适用采样，再核这条 ACK 自身阶段截止；每次 `emit` 返回后再核原截止与到期采样，包括最后一条 ACK。COMPLETE/CANCEL 不再跳过这一步。原 `SupervisorCore` 的十个类方法逐字不变，包括 ack、登记、tick 和 stop；异常继续沿原入口标 INCOMPLETE，仅回收已登记且身份已核的 owner。

发送期间采样到期且尚未收到终止消息时，先用 `receive(0)` 处理立即可读的有限排队消息，再强制采样。原分派块只提取为 `drain`，去除缩进后字节相同；每批仍最多原 sample_interval，且整个发送轮沿用首次接收前冻结的 receive_deadline，排队心跳不延长它。到空队列、批边界或终止消息即采样；终止后的登记不能追认陌生 PID。队列中新增 ACK 追加到原发送顺序，并接受相同发送检查。此处复用原排队处理是为了避免新增采样重现 HEARTBEAT→ACK 或多 REGISTER 的误拒。

正式独审04与全部五条探针原 JSON 已先核 SHA，再原字节复制到 references：

- `REVIEW-04-INDEPENDENT.md`：4762 字节，`7f2c2e00bd339578584fe3d933cb358cf436b375a6f8480f56c0f72bd240381c`。
- `REVIEW-04-INDEPENDENT-PROBES.json`：37709 字节，`010624e5babf3c57944dc831fd0069f7029d291b117773228195014359cce143`。

五条实际命令与输出均已读取；只重放零起算索引 1/2/3 的有限内存发送探针。索引 0/4 的历史静态核验与 N2 不重跑。对索引 2/3，原脚本继续把 `6cb6bbf33eb079f639abae2449ccc9b98d81a11c` 源码载入内存作历史对照；不切换 checkout。原探针中的 HEAD 标签表示 import 当前工作区，after 对应固定 HEAD 上的本次未提交字节，非新 commit。固定基线的 supervisor 与独审04所审 `827ac95f3b9aebc36c0a493fe273aac7699481e0` 完全相同。

| 正式反例 | 修前原输出 | 修后原输出 |
|---|---|---|
| 两 owner；E 截止9秒，B截止0.02秒，第一次 emit 用0.05秒 | 0.053秒仍发 B ACK，CONTROL_REAPED | 只发 E ACK，0.053秒拒绝 B，INCOMPLETE |
| 单 ACK→COMPLETE；emit用10.1秒，work=10秒 | 10.102秒成功，failures为空 | 返回即拒绝，INCOMPLETE |
| 单 ACK→COMPLETE；emit用0.2秒，采样间隔0.125秒，出现999 | 未采样仍成功 | 到期全集检查拒绝陌生PID，INCOMPLETE |
| 两 ACK；第一次emit后RSS=101，帽100 | 仍发第二ACK并成功 | 到期资源采样拒绝，第二ACK未发送 |
| 合法第一次emit用0.005秒 | 成功 | 保持成功 |

补充锁覆盖最后一条双 ACK 返回后 RSS 超帽、单 ACK 的 COMPLETE/CANCEL、work/heartbeat 等号与超期、下一阶段耗尽、采样自身耗尽阶段或心跳、合法0.125/0.2秒发送延迟，以及发送期间排队的合法心跳/多个 REGISTER/ACK内部路径。陌生999不收养、不发信号；持续心跳队列不能无限推迟采样，排队心跳不能延长原截止。

## 定向验证与证据

执行前 TEST-SELECTION.json 枚举 **24个旧方法＋16个新方法＝40个不同方法**，附选择理由。普通与 `-O` 各40/40通过，均0.023秒；两模式不是80个不同方法。修前只跑16个新增方法：12个方法出现25个失败断言，4个排队保护方法通过。错误正是过期确认、成功误报或漏采样。原输出为 red-targeted.txt、normal-targeted.txt、optimized-targeted.txt，实际命令、cwd、退出码、前后 Python 指纹分别存于 RED/NORMAL/OPTIMIZED-RESULTS.json，并由 TEST-RESULTS.json汇总。

红灯后测试文件只澄清一处注释，未改测试行为；已反算红灯原 SHA 并核 AST 相同。普通与优化执行前后的29份 Python 指纹完全一致。正式探针 before/after 六份原输出及实际结果索引完整保留，FORMAL-COMPARISON.json 逐项核对22条结果（含8条历史基线对照），不以脚本退出0代替反例已拒绝。

STATIC-CHECKS.json 记录29个 Python 的内存 AST、普通/优化 compile、`git diff --check`，原41个 driver 与27个 scheduler 方法及文件逐字保留。27个未变 Python 与固定基线及既有验证指纹匹配，含 captured_cut、product_capture 和其测试。revision4 与 product-capture 两包共43个报告文件均与固定 HEAD 原字节一致；44项既有来源/原输出绑定按长度和 SHA 复核。详见 TEST-REUSE.json。保留 product_capture 作者 run10 的全部既有内容；其新字节仍未独审，本包不为该适配器补签独审。

未重跑37/68/91/82/host123集合、ps/socket三项、adapter的38方法或N2集合；没有真实子进程/OS Probe、cargo/rustc、真实S/E/B/Q、Long、财务DB、网络、浏览器、资金动作或凭据读取。没有认领、评论、commit、push、merge、切分支、修改Git配置或开启工蜂。代码发现工具列表无可用图工具，按已知路径与必要rg回退；已读AGENTS/CLAUDE、wayfinder、世代宪法、CODEX-CHILD、ask-matt及两包报告。调试与完成核验采用 diagnosing-bugs、verification-before-completion 的红绿与证据流程，不扩大任务授权。

## 抽象传输限制与正式产品义务

0.05秒首发送仍处于自身9秒阶段及原work/heartbeat内；第一处回归不归咎传输。10.1秒或1.1秒 fake 故意让同步 emit 跨原截止，用于证明返回后必须拒绝；本修复不声称能在阻塞的 emit 中硬中断。`receive(0)` 非阻塞、实际传输自身受原截止、时钟持续推进及真实OS观察仍是正式实现义务。采样发现资源异常时停止，不通过延长期限、放宽资源帽或自动收养陌生PID兜底。

`execution_authorized=false`，产品全部 `NOT_RUN`，P11及整票不通过。真实统一新source及financial_reader组合语义重审、两角色ARTIFACTS、真实同域terminal全标签/完整K/source/独立γ、逐owner Voice与独评、全部逐对象资源/精确准入/OS及真实控制传输、fresh A/B、24故障、3额外域、5负域、10compare、完整历史、资金、隔离、恢复全部保留。路线1488→1419→组合C1→统一新SHA保持，不新增全TB02 blocker或资源帽。

Journal继续保守隔离candidate，重开不自动恢复接受资格，其恢复代价不变。前端归#1460，K4排除；不关闭#1394/#1361/#1340/#1323/#1279及图票。完整身份状态见FIX-LEDGER.md。DELIVERY-FILES.json绑定本轮代码/测试/报告的长度、权限与SHA，不写未来commit或自身摘要；管理者负责固定差异并独审。
