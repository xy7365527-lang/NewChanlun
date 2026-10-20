# #1406 C6a 离线准备包 Python 独审

日期：2026-10-17。结论：**Block**。发现 1 项 HIGH、2 项 MEDIUM；本结论只针对当前离线绑定器。`CONTRACT_READY=False` 必须保留，不能因 60 项离线测试通过签完整 C6a 合同。

审查树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`，基线 `b6e690ecae5723fbbcb773f15400c71d1010da87`。审查范围仅新增 `.github/scripts/run-c6a-remote.py`、`.github/scripts/test-run-c6a-remote.py`、`.github/workflows/ci-c6a.yml`；未改源码、未启动 cargo/网络服务、未写外盘、未 push/dispatch。

## 缺陷

### [HIGH] H3 绑定漏掉原请求 token 和 source_event_time，篡改原件仍被接受

File: `.github/scripts/run-c6a-remote.py:136`（expected 字段及 `:153` 比较循环）

Issue: `verify_capture` 只比较 fact_key/fact_hash/raw_utf8/json_pointer/raw_sha256/event，没有将请求的 token/source_event_time 绑定到 after 的记录；Recorded 的 token 也未绑定。使用真实冻结 fixture 构造合法 Capture 请求，将 after 记录 token 改为 `forged-token`、source_event_time 改为 null，Recorded token 同步改为 `forged-token`，重算 image_hash/root 并将回执 cut 对齐 after，函数仍正常返回。它因此不能证明声明的“after 原件各字段对请求”。

这不是新增金融规则：真实生产 `rust/src/economic_session/store.rs:1288` 与 `:1289` 在原消息审计中分别要求 payload token/source_event_time 与存储事实一致；`:1327` 的真实 Recorded 也带 token。测试 `test-run-c6a-remote.py:21` 的正例恰好省略这两个正式请求字段，无法揭露遗漏。

Fix: 使用正式 Capture 请求字段建立原件预期，至少将 token/source_event_time 纳入记录比较，并绑定 Recorded token；用含这两个字段的合法正例，以及保留合法 wire envelope/hash 的自洽篡改负例锁住。外层完整原消息身份绑定仍按作者已声明的装配欠项实施，不在这里声称已经解决。

已执行最小复现（在恢复树运行；只导入测试并用内存数据）：

```python
import copy
import importlib.util
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "contracts", Path(".github/scripts/test-run-c6a-remote.py")
)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
c = m.CaptureBindingTests()
c.setUp()
event = c.record["event"]
c.payload.update(
    token="t-" + event["source"]["external_fact_id"],
    source_event_time=copy.deepcopy(event["source"]["source_event_time"]),
)
row = c.after["image"]["recorded"][0]
row.update(token="forged-token", source_event_time=None)
c.receipt["token"] = "forged-token"
c.after["image_hash"] = m.driver.digest(c.after["image"])
c.after["cut"]["root_hash"] = m.driver.digest([1, c.after["image"]])
c.receipt["cut"] = copy.deepcopy(c.after["cut"])
c.check()  # 当前错误地通过
```

### [MEDIUM] JSON 有限数值约束可被指数溢出绕过

File: `.github/scripts/run-c6a-remote.py:62`

Issue: `parse_constant` 只拦截 NaN/Infinity 字面量；标准指数 `1e9999` 走默认 float parser，产生 `inf`。实测 `load()` 读取 `{"monotonic":1e9999}` 返回 `{'monotonic': inf}`。这违背函数明确声明的有限数值合同。当前 `verify_processes` 与 `remaining` 各自再次检查有限性，故不声称该复现已经绕过它们的期限门。

Fix: 增加 `parse_float`，转换后检查 `math.isfinite`；或使用精确十进制并在转成时钟数值时严格限制。负控包含正/负指数溢出及合法有限小数，并保留重复键/NaN/Infinity 拒绝。

### [MEDIUM] 清单重复检测使用原路径字符串，路径别名可重复登记同一文件

File: `.github/scripts/run-c6a-remote.py:67`、`:86`

Issue: `Path` 在检查 `parts` 前已规范化掉 `.` 和重复斜杠，但 `seen` 使用原字符串。实测在临时目录只建立文件 `a`，清单分别填 `a` 和 `./a`（相同长度及 SHA），`verify_inventory` 返回 `{'a', './a'}`，未拒绝重复。同样存在 `dir/a` 与 `dir//a`。这是当前重复条目保护的具体漏洞；覆盖范围还需外层核验已在 docstring 明说，不列为新 bug。

Fix: 拒绝非规范 POSIX 相对路径，或统一用规范相对路径/解析后的路径检查唯一性；明确返回集合采用同一规范表示。增加路径别名重复负控。并发文件替换和硬链接身份策略不在本轮扩写范围。

## 实际验证与边界

- 已执行 `git diff -- '*.py'`：由于三个文件仍为 untracked，该命令无输出；另用状态和直接文件读取审阅新增内容，未误作“没有变更”。
- `python3 -B .github/scripts/test-run-c6a-remote.py`：12 tests，OK，exit 0。
- `python3 -B -m unittest discover -s s_session/tests -p test_tb03c_query_runtime_contract.py`：48 tests，OK，exit 0。
- `python3 -B .github/scripts/run-c6a-remote.py --preflight`：实际 exit 2，`BLOCKED_PREPARATION`、`CONTRACT_READY=false`、`real_execution=false`。
- 三项新增反例均实际执行并复现误接受；只有临时文件及内存修改。
- `ruff`、`mypy`、`pylint`、`black` 的 CLI 与 Python 模块均不可用；未安装依赖，没有声称 lint/type/format 检查通过。
- `verify_owner_exchange` 对正式 envelope 使用既有 `decode_frame`/`validate_reply`，请求原字节、响应摘要及因果头的现有负控通过。H3 原 raw 篡改及 Recorded cut 篡改两项新增负控通过，但不覆盖本报告 HIGH。
- 进程事件字段与当前 driver 的 spawn/signal/reaped 结构相符；有限时钟、CLI 非零退出、缺 TERM 原件及未回收的测试通过。它只是日志校验器，不证明真实 guardian/进程组回收。
- workflow 只有手动准备入口和离线测试，末尾 preflight 必失败；未发现偷偷启动真实产品的路径。35 分钟 job timeout 不是同 t0 分阶段硬期限证明。

作者已经公开列明的逐序 replay/Q-startup、source/build 可信采集、资源门、guardian/取消、失败封包与 artifact 上传欠项，本审查不重复冒充新发现。即使上述三项修好，也仅能重审离线准备件，不能批准完整 runner 或改变真实执行准入。
