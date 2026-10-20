# C6a 外层锚类型与序号返修 v2

名分：#1406 / #1323 工作草稿，2026-10-17；待原独审复核。

两项独审问题已修候选，尚不能称完整 C6a 验收。

## 修改

- 外层 transcript 的 response 先确认 dict，再强制 `type(seq) is int`，之后才比较身份和期限。新增 5 行 × bool/float 序号负控及非 dict response 负控；driver 增加 float 序号负控，既有 bool 负控保留。
- runner 与独立 driver 各自声明 WorkAnchor / CaseAnchor / QAnchor、ClockRequest / ClockResponse TypedDict。runner 另声明 ClockRow、PendingClockResponse、ClockTranscript；sent、started、driver_pid、outer_deadline、failure 与 broker child/pending 明写可空生命周期。
- ExternalClock.request 使用三种 Literal overload 返回对应 anchor；时钟为 Callable[[], float]，传输为具体 socket.socket。Budget 的新协议调用及返回值已注解。broker 的全部新增公开入口、packet 编解码、transcript 验证边界已注解；LinuxSupervisor 仅补新增 broker 参数、spawn/clock_tick 类型，不批量改旧监督器。
- JSON 信任边界接收 object，在精确字段/类型/有限值检查后收窄；未导入 Any、未用 Any 遮盖。新的空态显式失败检查仅拒绝无期限/无端点 spawn 和未完整 arm 的 broker；时钟采样点与所有原绝对期限不变。
- 原独审反例脚本未修改。其最后一项仍是“非法 bool 被接受”的旧缺陷见证，修后应由原审按拒绝语义独立复核，不能原样运行后把见证失败视为产品回归。

## 实际验证

- `PYTHONDONTWRITEBYTECODE=1 python3 .chanlun/review-results/goal1323-resume-20261017/test-c6a-linux-supervisor.py`：53 项通过，最后一次 0.026 秒。
- 恢复树 `s_session/tests` 下 `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest test_tb03c_query_runtime_contract`：62 项通过，1.593 秒。
- stdlib inspect + typing.get_type_hints：29 个新增/受影响协议入口的参数与返回注解可解析；这是注解完整性检查，不是静态类型检查器结论。
- `git diff --check` 通过；runner 为未跟踪准备件，另直接检查其字节，不能因 git diff 不展示而忽略。
- `--preflight` 仍 exit 2 / BLOCKED_PREPARATION；CONTRACT_READY=false、SEMANTICS_READY=false、source/tree=PENDING；1200/1830/1920/2040/2070 端点及资源值保持。
- mypy / pyright 当前 PATH 均不可用，未安装，未声称静态类型检查通过。全部测试为纯数据/替身；没有真实 socket、产品进程、信号、HTTP、cargo/rustc、外盘或远端操作。

## 审阅字节

| 文件（相对仓库根） | SHA256 |
|---|---|
| `.sandcastle/worktrees/resume-1406-c6a/.github/scripts/run-1406-c6a.py` | `1a7ee70a9528867de1e5021216fb34a421122a6000b0876634af2a9a9f837b2d` |
| `.sandcastle/worktrees/resume-1406-c6a/s_session/tests/tb03c_query_runtime.py` | `04e234f278e5adfdba030369e5aac69c606b2a8ae4ee8f296c656b6894e67187` |
| `.sandcastle/worktrees/resume-1406-c6a/s_session/tests/test_tb03c_query_runtime_contract.py` | `a6cf4dc257f040a4b179346c0f5fe676049deb59798ca45485fe908b976ddf4d` |
| `.chanlun/review-results/goal1323-resume-20261017/test-c6a-linux-supervisor.py` | `0404b4251a6a1872234b92c77e6e3a744f47d0cbe46270ffd8b7928cb10f5059` |
| `.chanlun/review-results/goal1323-resume-20261017/c6a-external-anchor-review-repro.py` | `07d957ae478dd97205e3e6eee9ebe805bcf69faee3754d907cde468fd72d8748` |
