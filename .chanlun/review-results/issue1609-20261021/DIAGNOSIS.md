# #1609 诊断记录：Linux 上 stop-s/stop-q 报「尚未退出」

## 原始现象（#1405 浏览器复核原件，本机 `/Volumes/AgentStorage/issue1405-rv4-20261019/tb1405-browser-20261019-225737.tgz`）

`browser-run/events.jsonl` 第 29–30 行：`stop-q` exit_code=0、duration 37,500,573 ns，结果 `state=stopped`；随后 `stop-s` exit_code=1、duration 10,079,751,009 ns，stderr `{"ok": false, "error": "s 尚未退出，不自动升级信号"}`。同包 `session-run` 两次 stop 均在 37–55 ms 内成功。观察异常 `exit_observation_warnings` 只在成功路径返回，超时路径此前不输出，所以原件里看不到 ps 当时的原始输出。

## 复现（windows-5090 WSL Ubuntu，Python 3.12.3，procps-ng 4.0.4，非 tty）

`probe_ps_exit.py`（本目录）对两种进程发 SIGTERM 后每 50 ms 记一次原始 ps 输出与 `process_identity` 结果：

- 孤儿进程（中间进程退出，由 init 回收，对应 launcher 实际情形）：218 次观察，12 s 内全部是 `rc=1, stdout="", stderr="your 131072x1 screen size is bogus. expect trouble"`，`process_identity` 每次都抛 `ValueError: 无法确认进程状态，不能将查询失败当作已退出`。
- 本进程的子进程、不回收（对应现有单元测试的情形）：第一次观察即 `rc=0, "1000 Zs ... [sleep] <defunct>"`，判 stopped。

单独核 ps 的告警条件：

| 环境 | stderr | rc |
|---|---|---|
| 无 | `your 131072x1 screen size is bogus. expect trouble` | 1 |
| `LINES=24` | 空 | 1 |
| `COLUMNS=200` | `your 200x1 screen size is bogus. expect trouble` | 1 |
| `LINES=24 COLUMNS=200` | 空 | 1 |
| `TERM=dumb` | `your 131072x1 …` | 1 |

存活进程在 `LINES=24` 下输出正常（`1000 Ss+ <lstart> bash -s`）。

## 根因

procps-ng 4.0.4 在无终端时把行数取成 1，每次调用都往 stderr 写尺寸告警。`process_identity()` 判「进程不存在」要求 rc=1 且 stdout、stderr 都为空，于是进程被 init 回收后 ps 的结果落进「查询失败」分支；`stop()` 把 ValueError 当 unknown 继续观察直到超时。只有轮询恰好先看到僵尸态（Z）时才判 stopped，故为偶发；macOS BSD ps 不打此告警，故 macOS 从不出现。票面候选方向 3（ps 过渡期输出落进 ValueError 分支）成立，方向 1、2、4 不是本次原因。

## 修复与回归

见 PR #1631：ps 调用显式 `LINES=24` 并去掉 `COLUMNS`，判据不变；新增真实进程测试（被他人回收后须判 stopped；忽略 SIGTERM 时仍须超时）与平台无关的模拟测试（ps 在无 LINES 时注入该告警）。修复前新测试在 WSL 上失败（`TimeoutError: q 尚未退出，不自动升级信号`）；修复后 WSL、macOS 各 77 例 `test_tb01c_*` 全过。变异核对：删去 `LINES="24"` 后模拟测试在 macOS 上失败。

## 评审

- `review-claude-fable-5-1.md`：claude-fable-5-1 经本机 claude CLI + CLIProxyLocal，审 `31739cf7ce..b87f226352`，PASS-WITH-NITS（三条意见已在 `e070928783` 处理）。
- `review-tail-e070928783.md`：同一评审对尾部 `e070928783` 补审，PASS-WITH-NITS；余下两条 nit（模拟测试按 `argv[0] == "ps"` 字面匹配、COLUMNS 无 Linux 专属测试锁）未处理，记为已知边界。
