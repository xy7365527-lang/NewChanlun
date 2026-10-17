# C6a 独立外层时钟锚审查

名分：绑定 #1406 / #1323 的独审工作草稿。2026-10-17。源码只读；未改实现或运行开关。

**结论：Block（新公开接口类型契约 HIGH），另有 transcript 序号类型 MEDIUM。未发现本切片的计时重基、续租或回收控制流 HIGH。此结论区分静态工程质量与行为缺陷，不能把类型欠账说成已证实的资金或时钟漏洞。** 完整 C6a 仍未验收，真实运行门保持关闭。

## 具名问题

```text
[HIGH] 新公开时钟协议接口没有参数与返回值类型契约
File: s_session/tests/tb03c_query_runtime.py:133
Issue: 新 ExternalClock.__init__/request、Budget.q_start 与 runner 的 ClockBroker.arm/transcript、verify_clock_transcript 等公开边界均无类型注解。work/case/q 返回不同字段集合，broker 的 started/pid/pending/child 在生命周期中允许 None，却只靠调用顺序隐式区分；这些新接口即将承接独立语义器，当前无法由类型检查发现错接和空态访问。按本工位的 Python 审查准则列为 HIGH 静态质量问题；并非本轮已复现的运行绕过。
Fix: 仅在新增协议面补齐明确参数/返回类型，描述三种 anchor、transcript、pending 的字段及可空状态。使用精确 TypedDict/数据类与必要的 socket/clock Protocol 或具体类型，不用 Any 批量遮蔽；无需把旧 driver 全文件改写。保持精确运行时字段/类型验证，因为静态 int 本身不能排除 bool。

[MEDIUM] 外层 transcript 审计接受接收端会拒绝的布尔序号
File: .github/scripts/run-1406-c6a.py:348
Issue: response 仅通过字典相等比较身份。把完整有效 transcript 的首行 response.seq 从 1 改成 true，verify_clock_transcript 仍返回相同三键锚（Python True == 1）。请求侧第 335 行和 driver 第 170 行均显式要求 type(seq) is int，因此外层审计的协议类型口径较宽。真实 ClockBroker 正常不会签出此值，本例不证明可改变锚时间或绕过真实运行；它证明篡改 transcript 类型未被精确拒绝。
Fix: 在 response 身份校验前确认 dict 及序号的精确 int 类型，并加入 bool/float 序号负控。保留现有时间值有限性、字段集合和顺序检查。
```

## 本次实际证据

- 已读根 CLAUDE.md、Serena manual、外层锚设计/实现报告及旧监督器独审；已检查恢复树 `git diff -- '*.py'`。runner 为未跟踪准备件，因此另外直接审了 runner 实体，不以 diff 缺席代表未修改。
- 作者监督器套件在当前源码复跑 **51 项通过，0.039 秒**；driver 合同套件 **61 项通过，1.618 秒**。
- 本工位独立脚本 `c6a-external-anchor-review-repro.py` **16 项通过，0.015 秒**。其中 5 项继承旧独审，只在新增工厂处注入 fake ClockBroker.create；原失败原件上传检查、旧布局回退反控、临时名 stat 竞态回退反控、真实隐藏/嵌套原件保留、archive 禁移动语义与断言保持不变。10 项新增正确性正负控覆盖外部锚投影、原五端点、work/Q 的 EAGAIN 不续租、旧 case 不被新 case 洗掉、取消前/发送中、flood 有界推进、锚篡改拒绝、同一个 Popen 前采样与 FD、嵌套 probe 协议失败仍回收。**第 16 项是“非法布尔序号仍被接受”的缺陷见证；它通过代表缺陷复现，不能算该输入已被拒绝。**
- `git diff --check` 通过。ruff、mypy、black、pylint 不在当前 PATH，未安装依赖、未声称这些检查通过。
- `--preflight` 返回 `BLOCKED_PREPARATION` / exit 2，两个 READY 为 false、source/tree 为 PENDING。未运行 `--run`。
- 全部新增进程/信号/socket 只使用替身；只写临时普通文件和本工位报告/反例。未启动产品、HTTP、真实信号、cargo/rustc、外盘、重回放、远端或浏览器。

## 行为核查结论与边界

`spawn` 使用同一个 started 值，在 Popen 前 arm；等待解释器、文件打开及首次握手已被保守计入 W+600/W+630，未重新采时。五个原外层绝对端点保持 1200/1830/1920/2040/2070；case 为 min(work,C+300)，Q 为 min(Q+10,case,work)。等待发送仅重试冻结字节；过期不能重新签发。请求切 case 前检查旧 case；上级期限不足 10 秒时缩短 Q。

每次 broker tick 至多一次接收/发送；嵌套 probe 的 run(monitor=False) 同样推进 broker，之后仍保留原观测/回收控制流。broker 错误成为失败而非跳过 finally；runtime 回收前关闭通道、清除 active_broker；已取消不发新许可。原 WNOWAIT 组身份纪律未改，未新增 wait 者。真实 Linux 调度、FD 和取消回收尚未执行，此处是静态加替身证据。

只有 runtime 显式 pass_fds，父进程关 child 副本，driver 接管设 non-inheritable；其 E/B/Q/git 仍 close_fds=True 且无该 pass_fds。收到意外 SCM_RIGHTS 会先关收到的 FD 再拒绝。五次应答与 EOF 不齐，正常 exit 0 仍不获 PASS。

可信 transcript 由父内存生成，经外层 request 进入 verify worker；投影独算原点与期限，没有从 driver proof/transport 反取起点。旧“候选自行重基”的架构根因在此候选中已消除；**原语义器尚未真正嵌入发行 runner，所以不能声称完整旧反例已在最终装配上通过**。全 41 步合法 wire 正控、新 source/tree 冻结、两文件尾差审查、真实 Linux 验证及远端 artifact 可取验收仍缺。

## 审阅字节

| 文件 | SHA256 |
|---|---|
| runner `.github/scripts/run-1406-c6a.py` | `fe6392b8d1be652d2be0ac6b5ae80d3597cf253361da2c3723e7fa8069a783f1` |
| driver `s_session/tests/tb03c_query_runtime.py` | `bafc25a7bdeb899d32174e06f7c6ccfbc81d82f74d852876167bf22a12e4f395` |
| driver contract tests | `abd9d67a6f4e325366b7044783e526ebc1c55c20ec5e21000ce8259e5fcf311a` |
| 作者 `test-c6a-linux-supervisor.py` | `01c84436435063fbd2b9a9ed32ee486d60234cdce50bf093a953e2c1ae130ff6` |
| 本工位 `c6a-external-anchor-review-repro.py` | `07d957ae478dd97205e3e6eee9ebe805bcf69faee3754d907cde468fd72d8748` |

工作树：`.sandcastle/worktrees/resume-1406-c6a`。首四项与作者报告一致；结论仅针对这些字节。
