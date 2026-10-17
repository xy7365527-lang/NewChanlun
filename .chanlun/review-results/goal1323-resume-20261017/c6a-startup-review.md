# #1406 C6a Q-STARTUP 独立审查

日期：2026-10-17。名分：绑定 #1406 / #1323 的工作草稿。

**结论：Approve，仅限当前 Q-STARTUP 离线绑定器。未发现本次范围内高置信阻断缺陷；不构成 C6a 完整 runner、远端执行或全图验收。**

## 审查对象

工作树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`。审查前后文件摘要一致；没有修改实现或测试。

| 文件 | SHA256 |
|---|---|
| `.github/scripts/run-c6a-remote.py` | `9f8a6a40a112d91982fe0093e0fdb2f39dd60c757032a16a0ee6c34dd59dca25` |
| `.github/scripts/test-run-c6a-remote.py` | `668c8577a83174bccb1f452c7fecd38164a4bc8a7db5a902f8de6718b5914859` |
| `s_session/tests/tb03c_query_runtime.py` | `902c4eddb9ca55fe0cfb0f0c3a878d7296c8e58761924852cd29c728dd789d6e` |
| `s_session/tests/test_tb03c_query_runtime_contract.py` | `c174c5ba8ddde808c54afb8ce88d30b15e36bcfe30ad0089cf2786e78e691af1` |

已读根 CLAUDE.md、会话提供的项目共同规则和作者报告，调用 Serena initial_instructions。代码图查询新 runner 无节点，随后精读具名文件。

## 核验依据

- `verify_q_startup`（run 文件 166–258 行）与真实 `Case.wire/http/start`（driver 435、540、560 行起）的原件 schema、具名请求和控制流逐项对照。测试通过真实控制流采样，socket/进程/时钟均为替身。
- 独立参数约束 manifest、端口、开始时间、case/work 截止及首末序列；proof 和每次 attempt 不能通过自洽改写扩大窗口。绝对结束点为 `min(started_at + 10, case_deadline, work_deadline)`，最终验证必须严格早于结束点。
- 首末区间每个 transport 和两份 wire 原件都必须存在；文件名、原字节 SHA256、响应长度、固定零 body ready GET、host/port 均绑定。
- 仅非末次 connect 阶段的精确 ConnectionRefusedError/ECONNREFUSED、未连接、未发送、零响应可以进入下一 attempt；末次必须 complete、无异常并完成连接/发送。跨 attempt 时间次序及最终验证时间均受约束。
- 成功 HTTP 通过已有真实 parser 验长度、状态及 JSON；manifest 摘要、book、session、generation、epoch 和完整 owner 域集合对独立 manifest。域重复或缺失无法用重算全部摘要绕过。

## 本次实际执行

`python3 -B .github/scripts/test-run-c6a-remote.py`：25 项通过，0.365 秒，exit 0。

另执行 11 个临时、纯替身独立核验：

1. 将真实样本所有时钟及可信锚平移至非零原点 12000：接受。
2. 外部 manifest 含 E 与两个 B 域、ready 域顺序变化：接受。此项为合成多域绑定检查，不冒称真实多 owner 进程启动。
3. 上述多域响应遗漏一个 B 域，重算 HTTP 长度及全部摘要：拒绝。
4. proof 核验时刻改为 `1e9999`：拒绝。
5. 外部首序号变更：拒绝。
6. 外部末序号变更：拒绝。
7. 外部 manifest 会话变更：拒绝。
8. observed_at 早于该 attempt 开始：拒绝。
9. 拒绝连接的观察时刻晚于最终核验：拒绝。
10. attempt 注入额外准许重试字段：拒绝。
11. 成功 HTTP 截断一字节，重算 transport/proof 摘要及记录长度：仍被 HTTP 原长度拒绝。

`python3 -B .github/scripts/run-c6a-remote.py --preflight`：exit 2，`BLOCKED_PREPARATION`、`CONTRACT_READY=false`、`real_execution=false`。

作者已报告的另 48 项原合同测试，本次没有重复运行；不计为本审查新执行证据。没有真实服务、HTTP、浏览器、cargo、外盘访问、远端 dispatch、commit 或 push。

## 保留边界

调用者仍必须从可信调度和完整序列账本取得参数；本函数没有能力证明上层传参未从待验 proof 反取。它也不证明采集来源、进程身份、取消回收、资源监督、完整清单、WE5 20 步 / WE7 21 步 replay 或独立 oracle 已接通。上述是本次明示范围外且仍须完成的真实 runner 工作，不能把该 Approve 外推。

`errno.ECONNREFUSED` 使用核验进程的平台值；当前设计需采集和核验在同一目标平台。若将 Linux 原件带回 macOS 直接调用此函数，拒绝连接的 errno 数值差异会导致拒收，跨平台离线核验不在本次批准范围。

preflight 文案仍含资源原合同未恢复等旧状态；主线已另行恢复原派工与资源数值，该文案应由主线按新证据更新。本审查未更改它，也不以旧文案否定主线新证据。
