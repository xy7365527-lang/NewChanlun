评审完毕。以下为结论与逐条发现。

## 结论：PASS-WITH-NITS

修复是最窄的那种：只给 `ps` 子进程加了 `LINES=24`，`process_identity` 的三条判据（rc=1 且 stdout/stderr 皆空 → 已退出；其它非零或空输出 → 查询失败；`Z` → 已退出）一行未动，没有放宽，也没有自动升级信号。两个测试方向正确，但第一个测试的"咬住"能力依赖运行环境，这是主要保留意见。

## 逐条发现

**1.（中）回归测试只在特定环境下咬住缺陷，且会随 procps 升级而静默失效** — `s_session/tests/test_tb01c_service_control.py:586`
测试走真实 `ps`。它在修复前失败的充要条件是：Linux、procps-ng 恰为 4.0.4（3.3.17 的 Ubuntu 22.04 不打这条告警，4.0.5 起上游已修）、且测试进程环境里没有导出 `LINES`。现在 CI 的 `s-observer` 跑 ubuntu-latest（24.04 / 4.0.4）所以当下能咬住；但 runner 一换到更新的发行版，这条测试就变成修前修后都绿，而 `LINES` 这行若被人"顺手清理"掉也不会有任何测试报警。macOS 上它本来就不咬（BSD ps 无此告警，作者已知）。
建议：加一个平台无关的模拟测试——用 `patch.object(control.subprocess, "run")` 包一层真实调用，当返回 rc=1 且 stdout 为空、而传入的 `env` 里没有 `LINES` 时，往 `stderr` 注入 `your 131072x1 screen size is bogus. expect trouble`；断言 `process_identity` 返回 `None`。这样修前在任何平台都红、修后都绿，并把"必须显式传 LINES"锁进测试。现有真实进程测试可以保留作为端到端补充。另外可在测试内 `patch.dict(os.environ)` 并 `pop("LINES")`，避免开发者交互 shell 导出了 `LINES` 时本地跑不出失败。

**2.（低）第二个测试的 600 ms 期限同时约束了启动登记，负载下可能假红（不是假绿）** — `test_tb01c_service_control.py:614`
`startup_timeout_ms` 既是 stop 的观察窗，也是 `start_process → record` 等 `ps` 显示完整命令（macOS 上会短暂显示 `(Python)`）的截止期。真实 Python 子进程 + 多次 `ps` 在拥挤的 CI 上有可能超过 600 ms，届时报的是"启动后立即退出"之类的无关错误。同文件其它真实进程测试用 2000–3000 ms。竞态方面没问题：`ready` 文件在 `SIG_IGN` 装好之后才 touch，stop 在 `ready` 出现后才发信号，信号不可能抢在 handler 之前；超时后 `poll()` 为 None 是确定的。
建议：期限改到 1500–2000 ms，或把"等 ready"之后再实例化 controller 以拆开两段期限（后者改动大，不必）。

**3.（低，既有问题，非本 PR 引入）导出了很小的 `COLUMNS` 时 procps 会截断 `command=` 列** — `s_session/s_service_control.py:160`
procps 在非终端输出时宽度取 131072，但显式导出的 `COLUMNS` 会覆盖它并截断命令列；Q3 的启动命令很长，截断后会落进"参数与本次启动命令不符"→ 查询失败分支，症状和本票一样是"永远观察不到退出"。bash/zsh 默认不导出 `COLUMNS`，所以目前只是潜在。既然这次已经在构造 `environment`，`pop("COLUMNS", None)` 一行就能堵上，macOS BSD ps 不受影响。属于另开票或顺手修的范围，不阻塞合入。

**4.（信息）`TimeoutError` 文案追加 detail：无调用方依赖原文**
`rg` 结果：只有新测试用 `assertRaisesRegex(..., "尚未退出，不自动升级信号")` 子串匹配，仍命中；`tb01c_runtime.py:400` 与各 `assertRaises(TimeoutError)` 按类型捕获；`s_session/README.md:109` 只描述行为不引文案；`launch_s.sh:144` 的 `ps` 走 shell 并丢弃 stderr，与此无关。`detail = "…" + " | ".join(observations) if observations else ""` 的优先级是 `(A + B) if C else ""`，正确。

**5.（信息）根因论证成立，但修复影响面比票名大，建议写进票面**
根因链自洽：生产里 launcher `stop` 与启动进程不是同一进程，子进程已被 reparent 到 init，退出即回收，50 ms 轮询几乎不可能撞见 `Z`；而单元测试里测试进程是父进程且不 `wait`，子进程永远停在 `Z`，所以 CI 一直是绿的——这解释了为什么只有 Linux 实机复现。顺带，修复前 Linux 上任何对已退出服务的 `status()`、`recover()` 前置检查、`start_process:417` 的"已在运行"守卫都会抛"无法确认进程状态"，这次一并修好了，值得在 commit 说明或票面点出。另外我在 macOS 上无法复核 procps 4.0.4 源码，"`LINES=24` 消告警"这一点依据的是作者的 WSL 实测与上游已知回归；建议把原始的 `ps` stdout/stderr/rc 三元组贴到票上留证。

**副作用核查（问题 1 所列情形）**：macOS BSD ps 不读 `LINES`，无影响；环境已存在 `LINES` 时被覆盖为 24，`ps` 只用它决定表头重复行距，对 `-o …=` 无表头输出零影响；`COLUMNS` 很小见第 3 条，本 PR 未改变其行为。现有 `test_process_query_failure_does_not_prove_stopped`（rc=2 + stderr）仍锁住"查询失败不算已退出"，也没有 mock 对 `subprocess.run` 的 kwargs 做精确断言，加 `env=` 不会打破既有测试。
