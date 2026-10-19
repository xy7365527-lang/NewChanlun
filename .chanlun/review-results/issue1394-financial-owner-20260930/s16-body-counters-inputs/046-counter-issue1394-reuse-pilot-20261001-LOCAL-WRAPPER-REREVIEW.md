# #1394 本机审计 wrapper 增量复审

结论：**FAIL / Block**。固定审查对象 `LOCAL-audit.py` SHA256 `da2829ad31d5a3508f2a5070dd1159d85237507f9123721af37c2714ca470eeb`。旧 `LOCAL-WRAPPER-REVIEW.md/json` 的 `772446a...` 结论保留为历史，不适用于本版本。未运行 rebind、build、audit、report，也未重新哈希 589 个 target 或展开 fullDB。

旧三项 HIGH 已在代码中修复：`report_evidence()` 对当前 PROBE 重算 comparison 并核对 plan、日志、二进制、源和 DB；`report()` 先生成 Markdown 与 JSON 字节，最后以 rename 发布 PASS；`checked_counts()`、`unsigned()`、`decimal()` 拒绝布尔/浮点和缺字段。`LOCAL-output/LOCAL-SELFTEST.json` 声明当前 SHA 下 7 个负例与两 repeat 正例通过，所见 `LOCAL-selftest.py` 代码与这些测试名一致。

[HIGH] 优化模式会移除全部审计准入检查
File: `.scratch/issue1394-reuse-pilot-20261001/LOCAL-audit.py:88`
Issue: `unsigned()`、`checked_counts()`、`checked_artifact_sha()`、`static_gate()`、`rebind()`、`bound_gate()`、`report_evidence()` 等关键门均由 `assert` 承担。脚本没有拒绝 `PYTHONOPTIMIZE=1` / `python3 -O`。在该模式下 Python 直接删除 assert 语句；纯内存反例执行 `PYTHONOPTIMIZE=1 python3` 加载当前 wrapper，`unsigned(False, 'exit')` 返回 `False`，`checked_counts({}, True)` 返回 `{}`，没有拒绝。相同机制会让 source/target/binary/patch/log/comparison 的断链检查失效，并可能输出 PASS。
Fix: 在模块入口、任何副作用之前显式拒绝 `sys.flags.optimize != 0`（不能用 assert 实现）；更稳妥的做法是把承重的验证改为显式 `if ...: raise`。纯内存自检增加 optimized-mode 负例。冻结新 SHA 后再审并运行后续门。

静态审查边界：本次只读 `LOCAL-audit.py`、`LOCAL-selftest.py`、`LOCAL-SELFTEST.json` 和 `SOURCE-BINDING.json` 的路径字段；语法与缩进可解析。`git diff -- '*.py'` 为空，因为 wrapper 是 untracked；未将自测的 PASS 当作实际审计完成。现阶段不能放行 rebind/build/run/report。若保持此版本，必须由外部执行入口强制检查 `sys.flags.optimize == 0` 才有条件继续，但 wrapper 自身仍不符合独立可运行门的要求。
