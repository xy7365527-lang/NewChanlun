# #1394 本机审计 wrapper 独立 Python 审查

结论：**FAIL / Block**。审查对象为 `LOCAL-audit.py` SHA256 `772446a13655b841c80609c2b9d152cec43deb5d43640273c48aada39a39ebc6`；旧 SHA `e8f31ecd...` 不在本结论范围。只读 `original_gate()` 当前通过，`PREPARE.json` 存在；build、run、report 均未执行。本结论不评价尚未生成的性能数字。

[HIGH] PASS 报告没有重新绑定当前运行原始证据
File: `.scratch/issue1394-reuse-pilot-20261001/LOCAL-audit.py:401`
Issue: `report()` 直接使用已有的 `COMPARISON.json` 和三个 `*-RESULT.json`；只验两个 PROBE 的 SHA 与 result 中字符串一致，未从当前 PROBE 重新运行 `returned_check()`，也未复核 `BUILD-PLAN.json`、两份 `*-PLAN.json`、`build.log`、两份原始运行日志的实际 SHA 与 result 记录值。旧文件或局部改写可被当成当前证据而产生 PASS。`PREPARE.json` 也没有 wrapper 自身 SHA，实际先 prepare 后修订 wrapper 的本次情况无法从证据链辨认。
Fix: 在输出 PASS 前对当前 PROBE 重算 comparison 并逐字段比较；校验所有 plan、raw log、result、binary、源副本与当前 diagnostic/source binding，记录并核验本 wrapper 的 SHA；拒绝任何断链或旧期次文件。

[HIGH] PASS JSON 早于可能失败的 Markdown 渲染写出
File: `.scratch/issue1394-reuse-pilot-20261001/LOCAL-audit.py:436`
Issue: `LOCAL-REPORT.json` 已写 `status=PASS`，随后第 442 行才访问此前从未要求存在的 `reuse_hits` / `reuse_misses`。字段缺失时抛 `KeyError`，留下假 PASS JSON；Markdown 写入失败也同样留下假 PASS。
Fix: 先完整验证报告所需字段并构建两种报告，写入成功后最后原子发布 PASS JSON；失败时留下显式失败状态，避免任何可被误认的 PASS。

[HIGH] 数值门允许 bool 冒充计数及退出码
File: `.scratch/issue1394-reuse-pilot-20261001/LOCAL-audit.py:367`
Issue: Python 的 `True == 1`、`False == 0` 使计数相等/非负判断通过；第 386、407 行也把 JSON `false` 当作退出码 0。只读反例：以旧 PROBE 的两个 repeat 为三臂，候选减少一项 F 并补成功数，但设 `retained_entries_peak=True`、`retained_capacity_peak=True`，`returned_check()` 仍返回两条 comparison。`int(elapsed_ns)` 也会接受 bool、浮点和可截断字符串，不能保证有效计时。
Fix: 对计数、耗时和退出码逐项要求 `type(value) is int`（或明确的十进制字符串模式），再检查范围和不变量；显式要求 hits/misses 与 retained 字段存在且为整数。

已核对的边界：`PREPARE.json` 记载原 A 的 DB/WAL/SHM SHA256 分别为 `1bb9fb2c1660501bc570a52ccb61aa4c89cbe806fd9fc3bea0cfb550de5423fb`、`89032325fe6a2dbf4b3b97e2f335072ce0d7cc5a634c81e0411ebc4dfefe843b`、`757d2d7f80f325accd4a68f0a776bef0d31db8f1ecf0d7ce486bfefb5d616193`；两份克隆同 SHA、不同 inode。原 target 快照 589 文件；`original_gate()` 对当前原源和原 target 哈希/元数据验证通过。旧二进制 SHA `3ea943907f9dc045c93ba8f5cbf2e7957e7fdf99cc6bf1c55273d843ef56e61d`，patch SHA `e18ffbaefceb9c577f00c92bfed9edb8821e498afce481f9a90011f0f8d13ff3`，fullDB SHA `8a7f8f248d1ffad4081555d2900bfd9bc73fb32971167edb544cb15292ea8174` 均由静态门核对。

检查：`git diff -- '*.py'` 无 tracked Python 差异；本文件为 untracked；语法编译通过。ruff、mypy、pylint、black、bandit 在当前 PATH 均不可用。未执行 build/audit/service、未打开原源 SQLite、未改产品文件。当前 wrapper 的 prepare/build 隔离与 300/250 MiB 构建空间门未见直接阻断性缺陷；**不能据此放行 run/report 或最终 PASS**。
