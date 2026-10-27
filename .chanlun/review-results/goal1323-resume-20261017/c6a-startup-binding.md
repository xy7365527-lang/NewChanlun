# #1406 C6a Q-STARTUP 离线精确绑定

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。状态：已实现，待独立审查；**CONTRACT_READY=False**。

## 修改范围与前版封存

工作树 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`，基线 `b6e690ecae5723fbbcb773f15400c71d1010da87`。仅改 `.github/scripts/run-c6a-remote.py` 和 `.github/scripts/test-run-c6a-remote.py`；workflow 未改，driver/fixture/产品源码未改。

修改前已将批准版本的两份文件及 MANIFEST 原字节封存到本报告同级 `c6a-binder-approved/`：run SHA256 `595da5c32a0a19f65d78b693b686634c1b5c43109e9a305129dcd68ada1fc83c`；test SHA256 `57bde7d936b4cb89114d30de0310259d3dad8e61b9db9afb4a687aeef7cb85c3`。没有覆盖旧报告。

## 已完成的离线合同

新增 `verify_q_startup(root, *, manifest, port, started_at, case_deadline, work_deadline, first_sequence, success_sequence)`，读取正式 driver 保存的 `Q-STARTUP.json`、`port.json`、连续编号的 transport 与 HTTP request/response 原字节。

- 具名零 body `GET /api/economic/ready`、HTTP/1.0、127.0.0.1、可信分配端口、固定请求头逐字节一致；transport 角色、地址、文件名、请求/响应 SHA256 及响应长度绑定。
- 首末序号对调用者独立账本锚，区间每个编号必须有 transport 和两份原件；每次 attempt 的 sequence 对编号、共享 started_at/deadline/case/work 值对独立锚。不按全 case 的请求集合凑数。
- 每个非末次 attempt 只能是 connect 阶段、ConnectionRefusedError、ECONNREFUSED、connect_completed=False、send_entered=False、原响应实际为零字节；不能用 send/recv/timeout 或有响应原件续试。
- 最末次必须 complete、无异常、连接和发送完成；成功原响应摘要对 proof，并用现有 `driver.parse_http` 解析正式 HTTP 原件。
- 单调时间为有限非 bool 数值，attempt 起止顺序、跨 attempt 顺序、observed_at≤validated_at 逐项核验；deadline 固定为 min(started_at+10, case_deadline, work_deadline)，所有观察和最终核验都严格早于该点。自洽修改所有 proof/attempt 以续租也无法绕过独立锚。
- ready 的 manifest 摘要、book_revision、session_id/session_generation、Q producer_epoch 和 owner 域集合逐值对实际 manifest；重复域同样拒绝。

### 可信锚的责任边界

参数必须来自调用者可信 manifest、真实端口分配、同一单调时钟的调度记录及完整 transport 序列账本；**不得从待验 proof/transport 反取锚**。本函数检查给定原件间的绑定，不自行声称已证明原件采集可信、全 case 清单覆盖、进程身份、资源或真实 guardian 监督。真实 runner 的接线仍未实现。

## 正负控来源

直接调用既有 `test_tb03c_query_runtime_contract.QStartupRetryTests.run_case`：它执行真正 `Case.start/http/wire`，仅 socket/进程/时钟使用既有替身。新增测试通过包装真实 `Case.__init__` 捕获实际构造的 manifest，序列、端口和期限锚来自采样输入/既有替身合同，不读取 proof 生成预期。

正控包括一次即成功、拒绝连接后成功、case 提前截断和 work 提前截断。负控覆盖 proof/attempt 一起篡改期限、首末/attempt 序号漂移、缺原件、成功摘要漂移、非连接拒绝或已发送重试、有响应重试、重算摘要后的错误 GET 路径/method/Host/body、错误地址/port 原件、晚到成功/拒绝/解析核验及时间逆序、bool 时钟、错误成功阶段、重新生成合法 HTTP 长度与全部摘要后的错误 manifest/session/generation/epoch/域集合。

没有真实 HTTP、产品进程、cargo、外盘访问、push 或 dispatch。

## 实际检查

```
python3 -B .github/scripts/test-run-c6a-remote.py
Ran 25 tests in 0.395s / OK / exit 0

python3 -B -m unittest discover -s s_session/tests -p test_tb03c_query_runtime_contract.py
Ran 48 tests in 1.623s / OK / exit 0

python3 -B .github/scripts/run-c6a-remote.py --preflight
BLOCKED_PREPARATION / CONTRACT_READY=false / real_execution=false / exit 2
```

两份 Python 的 AST、三文件的行尾空白检查通过。git status 仍只有三个未跟踪准备文件，未提交。上述 25+48 是离线合同检查，不是远端/产品验收。

| 文件 | 字节 | SHA256 |
|---|---:|---|
| `.github/scripts/run-c6a-remote.py` | 18026 | `9f8a6a40a112d91982fe0093e0fdb2f39dd60c757032a16a0ee6c34dd59dca25` |
| `.github/scripts/test-run-c6a-remote.py` | 23403 | `668c8577a83174bccb1f452c7fecd38164a4bc8a7db5a902f8de6718b5914859` |
| `.github/workflows/ci-c6a.yml`（未改） | 1102 | `120b1b05dc62d4cf888537c20138620e02bef28ee0d07840fa699ddbf6b00e57` |

## 未完成

尚未扩 WE5/WE7 全量逐序 replay；startup 绑定尚未接入真实运行监督。可信构建/执行身份、资源门原数值、guardian/取消和失败封包合同及真实执行仍未恢复/完成。原时间常量和关闭准入保持，不能以本次离线绑定代替 C6a 或全图验收。
