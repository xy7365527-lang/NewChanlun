# #1406 C6a 独立时间锚最小装配建议

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿；仅源码阅读与设计，未修改 driver、runner 或开关，未执行产品。

**建议：单个 Linux socketpair，五次固定顺序请求，由外层唯一监督线程签发并保留时间锚；driver 直接消费外层值。broker 是有界非阻塞状态机，必须在资源 probe 的嵌套等待中同样推进。无需后台线程或独立 broker 进程。**

有一个须由主控明确接受的保守变化：work 的起点应由外层在 runtime `Popen` 前采样并预置，首个请求只能领取该值。不能等收到首个请求才给新的 600/630 秒。这把解释器启动和最初 IPC 等待也计入原额度，比当前 `main()` 第一行采时更早，不增加窗口。case/q 的签发采时是各自新明确的阶段边界；请求前等待受已签上级/前级期限约束，签发后的传输等待全部计入新期限。

## 依据及名分

读取了 `c6a-contract-recovery-v2.md`、`c6a-semantics-rereview-v2.md`、当前真实 runner/driver 和语义器锚入口。Serena manual 已读。图查询对 driver Budget/Case 无匹配，随后精读明确路径。

本方案依赖的概念是同源单调时钟、绝对期限不续租、外部见证与候选原件分离、唯一组所有者；工程合同来源是上述已恢复原派工与当前已提交 driver。查询 `.chanlun/genealogy/` 未发现 C6a/CLOCK_MONOTONIC 的专属概念裁定或相关生成态冲突；本方案不新增缠论语义判据，也不援引未结算教义。它仍是待主控裁定、待实现独审的工程选择，不是已验收事实。

审阅字节：

- `.github/scripts/run-1406-c6a.py` SHA256 `7f2318dab28d447fb3feea5d4f3e0c99cc4853a3f1c67cad141be346732add5b`。
- `s_session/tests/tb03c_query_runtime.py` SHA256 `902c4eddb9ca55fe0cfb0f0c3a878d7296c8e58761924852cd29c728dd789d6e`。
- 现语义器要求每个 case 的外部锚字段恰为 `started_at/case_deadline/work_deadline`，不能把新增 broker 字段直接塞入该三键对象。

## 当前最小落点

| 位置（当前行号） | 当前行为 | 最小改动 |
|---|---|---|
| driver `main:1006`、`:1007` | 本地采 t0 | 最早领取预置 work 锚；Budget 的 t0/work_end/final_end 使用领取值 |
| driver `Budget.begin_case:115`、main `:1040` | 本地 `clock()+300` | 保留旧 `check()`，请求具名 case，赋外层 case_deadline |
| driver `Case.start:589` | Q spawn 前本地采 started_at | 同一位置请求具名 q，直接使用外层 started_at 和对应三层 deadline |
| driver `Supervisor.spawn:153` | close_fds=True，无 pass_fds | 保持此处；broker FD 不能传入 E/B/Q/git |
| runner `spawn:198` / `run:279` | 所有子程序均新组 | 仅 runtime 调用显式传入唯一 child FD；返回后父进程立即关 child 端 |
| runner `probe:262` / `run:279` | probe 递归调用 run，monitor=False | 所有等待循环共用同一 broker tick，不仅 runtime 最外层 |
| runner runtime 调用约 `:495` | 单次 supervised run | 创建 socketpair、预置 work 锚、登记 broker 所属 runtime Owned |
| runner verify worker 约 `:405` | provenance 仅有外层进程起止区间 | 附加父进程自存 broker transcript，验证后投影为 case_startup_anchors |

driver 的现有 `Budget(t0=None, clock=...)` 测试入口可保留，但 C6a CLI 必须要求外层 FD。生产 CLI 缺 FD 不得静默退回本地时钟。不要让两个时钟来源都参与同一正式执行。

## 协议与绝对期限

采用 `AF_UNIX/SOCK_SEQPACKET`（目标为 Linux），无路径、无监听服务。双端设 nonblocking，父端留 supervisor，child 端通过 runtime Popen 的显式 `pass_fds=(fd,)` 传入。父进程关闭 child 端；driver 接管后立即设置 non-inheritable。其他 helper 不传 pass_fds。argv 仅带 FD 数字与非秘密 run_id；run_id 用于防串线，不宣称密码学鉴权。

固定请求序列：

1. `work`
2. `we5-case`
3. `we5-q`
4. `we7-case`
5. `we7-q`

请求仅含固定 schema、run_id、序号与 op，不允许 driver 提供时间或期限。字段集合精确比较，重复 JSON 键、非有限数字、额外字段、未知版本/操作、乱序、重放、额外第六请求直接使本次候选失格。

父进程状态最多保存 5 请求与 5 应答，单包建议不超过 1024 bytes。这是新 IPC 防失控界，不修改产品 frame/reply 帽。`recvmsg` 检查 MSG_TRUNC/MSG_CTRUNC；超包、非预期 ancillary FD 均拒绝，接到的任何 FD 必须关闭，不能泄漏。每次 tick 最多处理一个接收和一个发送，不使用 drain-until-empty 循环。

设外层 runtime 即将 Popen 前采样为 W，outer 原 runtime 端点为 R：

- work：只返回预置 `t0=W, work_deadline=W+600, final_deadline=W+630`，任何后续领取/重试不得重置 W。外层仍以 R 监督；有效执行同时受两套原绝对界约束，不能把 R 改成 W+630。
- we5-case：外层检查未取消且 `now < work_deadline` 后采 C5，签 `case_deadline=min(work_deadline,C5+300)`。
- we5-q：检查已签 case/work 尚有效后采 Q5，签 `started_at=Q5, deadline=min(Q5+10,case_deadline,work_deadline)`。
- we7-case：**先核旧 we5 case 与 work 尚有效**，保持原 `Budget.begin_case()` 先 check 的语义，然后采 C7、签 `min(work_deadline,C7+300)`。不能先覆盖旧 case_end 再检查。
- we7-q：同 we5-q。

每次成功校验请求即固定一次应答和对应时间，不因 EAGAIN、调度延迟、读回超时或重试重新采时。应答发送阻塞时留在唯一 pending 槽，下个 tick 仅重发同一包；其截止已在走。迟到再发送没有价值，应标失败并走原回收流程。

精确值采用正常有限 float JSON 往返，拒绝 bool 冒充数字；同一值从父进程应答字节、父进程保存 transcript 到 driver 都不得自行四舍五入。实际 Linux 内核 CLOCK_MONOTONIC 是共同坐标；driver 正式 Budget clock 应显式使用同 CLOCK_MONOTONIC，测试可注入假时钟。此方案要求不进入另一个 time namespace，当前受审启动链不调用 unshare/setns。

**边界区别**：不能让 driver 先自行采时再要求父进程“认证”那个值；也不能从 Q-STARTUP/proof/transport 反取值。外层只证明自己签发的时刻与期限，冻结 driver 源码证明请求位于 Q spawn 前的指定调用点。它不是针对任意恶意同 UID 程序的安全沙箱，也不声称证明未受审代码从未在请求前工作。

## broker 如何不阻断 guardian

现 `probe()` 会调用 `run(... monitor=False)`。若 broker 仅放在 runtime 外层 while，probe 及其清理期间不能响应，可能占用整个 Q 的 10 秒；因此采用 supervisor 实例上的一个 `active_broker`，所有 run 等待 tick 在已有取消/绝对期限检查之后调用。

- runtime/probe 的正常等待中均可推进；每 tick 有固定常数上限，无文件读写、hash、子进程、无阻塞 send/recv、无等待 ACK。
- 保留原资源观测节拍和绝对期限。每轮处理 broker 后仍进行资源观测/leader 检查，不能因持续有 IPC 就跳过 guardian。
- broker 失效写入父内存 failure，**不要让 probe 的 finally 或 wait 所有权动作被任意回调异常跳过**。由 run 的正常异常/失败路径回收；任何失败后停止签发。
- 进入 runtime cleanup 时先禁用签发并关端点；无需为“完成握手”阻塞 TERM/KILL/reap。取消后同样不得签新锚；handler 继续只记取消状态。
- 只需在正常嵌套等待中推进；在 failure/reap 阶段不得继续为 driver 发新许可。父控制流保留 WNOWAIT 身份纪律，不新建另一个 wait 者。
- 五次握手结束后，driver 立即关闭端点；父端应观察 EOF。提前 EOF、未完成五步、pending response 时断连均是失败。runtime 正常退出但五步未齐或无法证实完整收发，也拒绝 PASS。

| 方案 | 收益 | 代价 / 风险 | 建议 |
|---|---|---|---|
| 主线程有界非阻塞状态机 | 同一时钟/取消/Owned；无新进程，外部记录天然归主控 | 必须覆盖 probe 嵌套等待；严格限制每 tick 工作 | 采用 |
| 受监督 broker helper | 独立响应，不依赖 probe 递归 | 需要第二长寿命组、资源加总、身份回收、父子记录传输；现 run 同步 API 不足以同时监督两组 | 本切片不采用 |
| 后台线程或无监督 helper | 表面改动少 | 新的取消、退出、阻塞写、所有权问题；不能证明 guardian 始终可用 | 排除 |

## 证据装配与取消后保全

broker 原件只在父内存构造，不调用每次 handshake 的 `save()`。执行结束或失败后，在仍受原阶段截止监督的流程中保存外部 transcript；不得写入 driver 提供的 runtime 文档后再读回当外部真值。建议在 artifacts 根单独保存，父进程在 runtime 全组回收后将内存快照交给既有受监督 worker 落盘/核验。未回收或来不及落盘只允许 FAIL/UNSEALED，不能从候选 proof 补齐。

transcript 最少包含：schema/run_id、driver Owned 的 PID/PGID 与启动采样、5 次固定请求/应答原字节或等价规范摘要、每次 received/issued/sent 时间、work/case/q 派生值、EOF/协议失败/取消状态。固定小集合可有明确总字节上限；新上限仅管控制证据，不缩减 41 步原件。

verify worker 从父提供的 transcript 独立重算 `W+600/W+630`、两个 case 的 min 式、两个 q 的 min 式及严格序列，核所有时刻在 runtime 起止和 outer runtime 绝对界内。再生成：

```
case_startup_anchors = {
  "we5": {"started_at": Q5, "case_deadline": D5, "work_deadline": W+600},
  "we7": {"started_at": Q7, "case_deadline": D7, "work_deadline": W+600}
}
```

将这个恰三键投影交现 semantics/binder。原 process ledger 起止检查、first_sequence、manifest 和 wire hash 仍保留；外部锚并不替代这些检查。额外把 receipt 的 work/final 限额与外层 work 锚交叉验证，避免只核 Q 而让全局预算漂移。

## 纯替身验证矩阵

以下为待实现的验证设计，本报告没有运行这些测试。可用 fake clock/fake packet transport/fake Owned/fake probe；不启动产品、HTTP、cargo 或真实信号。

1. 完整五步正控：WE5/WE7 精确顺序；所有三层期限、外部投影、EOF 与运行收据逐值一致。
2. work 请求在 spawn 后拖延 20 秒：领取的 W 不变，剩余只有 580/610；拖过 600 不能签 case。
3. q 签发后应答 EAGAIN 20 秒：不重采 started_at，失败；应答在 9 秒返回只剩 1 秒，不能再给 10 秒。
4. case 请求在旧 we5 截止后：不能用 we7 新 case 洗掉旧超时；上级仅余 4 秒时 q 截止只给 4 秒。
5. 原重基反例：仅改 proof/transport 的起点及期限，父 transcript 不变，原 binder 拒绝。
6. 阻塞 probe 替身：嵌套等待仍推进 broker，资源检查仍有调用；flood 不造成每 tick 无界处理或漏检截止。
7. 取消发生在接收前、签发后/发送前、runtime exit 前：不补签、不回 PASS；原 TERM/KILL/reap 路径及未reap身份仍执行。
8. 提前 EOF、最终应答发送失败、重复/乱序/跨 run_id/第六请求、超包/trunc/重复键/非法数：全部拒绝，无补签续租。
9. FD 所有权：fake Popen 参数断言仅 runtime 获 child FD；父失败路径关闭双端；driver E/B/Q/git 的 close_fds=True 且无 pass_fds；fake recvmsg 注入 ancillary FD 后断言关闭。
10. driver 中的纯替身 Q spawn 延迟 20 秒，外层 Q5 固定：原启动 bind 必须拒绝；不能因当前时钟/启动进程账不同重定基点。
11. 源基线与装配：新版 driver 与 contract tests 进入新独审 source SHA/tree；SOURCE_FILES 仍精确 11 条；最终 CI 尾差仍只能两发行文件；旧冻结 oracle/source hash 不得未经复审自动改成新值。

## 仍未解除的门

本建议未解决全 41 步合法 wire 正控、语义器嵌入发行 runner、真实 Linux 组回收验证、远端 run 或全图验收。`CONTRACT_READY=False`、`SEMANTICS_READY=False` 和 source/tree PENDING 保持。主控确认 work 锚保守前移后，方可按文件归属实施；实现后需独审协议反例、源基线、FD 与 guardian 公平推进，不可仅凭此设计文档运行。
