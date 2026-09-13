# #1372 DevSkim 门禁 Python 回归测试独立评审

结论：APPROVE（限 Python 测试质量、资源安全及既有 pytest 收集兼容）。当前没有未解决的 CRITICAL、HIGH 或 MEDIUM 问题。初审发现的摘要写入失败退出码覆盖缺口，已由根代理增加第五项用例并通过本次独立复核。本评审不替代另一路 Bash 语义评审、原新脚本 33 例对拍或远端 CI 结果，也不构成 C 合入 main 的批准。

## 对象与源码身份

- 工作树：`/Users/silencehan/Projects/NewChanlun-1372-codex`。
- `tests/test_devskim_sarif_gate.py`：5,365 bytes，SHA-256 `6ca76cb11f3ac7cb40ac767adf0440fe3e95a6f9e95ac9233755044205aeb33c`。
- 受测 `scripts/devskim_sarif_gate.sh`：9,706 bytes，SHA-256 `c3154bd748c796ba950d1b4926308cd37a1cf54b0a802f644b6a8877132ce5fc`。
- 测试、门禁、pyproject、CI 与 conftest 共五份源码在本轮独立检查前后 SHA 完全相同。详见 `FINAL-CHECKS.json`。
- 评审者未改动工作树，只在本评审目录写入源码快照、检查日志和报告。初始 `git diff -- '*.py'` 为空是因为测试文件当时仍未跟踪；随后按未跟踪文件的完整内容审查，未把空 diff 当作无改动。

## 已解决问题

[MEDIUM] 原有多错误用例不能独立验证摘要写入失败覆盖 PASS 退出码
File: tests/test_devskim_sarif_gate.py:94
Issue: 原用例先制造 scanner、artifact、tool 与 findings 错误，门禁本身已经返回 1；即使摘要追加失败分支错误地继续返回既有状态，该用例也不会失败。
Fix: 已在 tests/test_devskim_sarif_gate.py:113 增加 `test_summary_write_failure_overrides_an_otherwise_passing_gate`：使用空 findings，stdout 明确包含 PASS，将摘要路径指向临时目录，要求最终退出 1 且 stderr 保留追加失败信息。第五项独立运行通过。

## 当前覆盖与安全检查

1. 空扫描：要求退出 0、stderr 为空、PASS 可见；不提供摘要路径时不建立测试的摘要文件。
2. 既有基线与已消失键：包含 `file://` 标准化输入，要求退出 0、消失计数为 1，且没有 Failure details。
3. 大差集：输入反序 12,000 条长路径，含中文、美元符号和反引号；逐条比较最终记录列表及顺序，检查新增计数、结尾换行、stderr 为空与退出 1。摘要以非空中文内容预置，并逐字要求最终文件为原内容加完整 stdout，能发现截断、遗漏、重排、覆盖摘要和文本转义破坏。30 秒 subprocess 超时保留；当前完整五项耗时约 1.4 秒，未通过放宽时限使慢实现过关。
4. 多个失败原因：逐项查找原输出信息并比较出现顺序，同时要求摘要追加失败仍可见。
5. 原本 PASS 的摘要写入失败：补齐失败分支必须改变退出状态的关键行为。

测试使用参数列表调用 subprocess，无 `shell=True`、eval、动态代码执行或凭据；SARIF、基线与摘要都位于 `TemporaryDirectory`，通过 `addCleanup` 注册清理，失败时同样回收。捕获 bytes 后显式 UTF-8 解码与逐字比较，避免依赖平台默认编码。每个 TestCase 使用独立目录，没有共享可写测试状态。目录写入失败不会依赖 root 用户仍可绕过的权限位，因而比 chmod 模拟更稳定。

本文件关注本次摘要输出优化的回归面，不把它当作 SARIF schema、全部参数错误或 Bash 所有分支的完整测试集；这些边界由另一路原新脚本对拍审查。本评审未发现需要额外扩大产品实现或测试规模的缺口。

## 收集与运行兼容

- `pyproject.toml:40` 的 pytest 配置包含 `testpaths = ["tests"]`；文件名 `test_devskim_sarif_gate.py`、`unittest.TestCase` 及五个 `test_` 方法都满足默认收集规则，无 slow 标记。
- `.github/workflows/ci.yml:102` 的既有入口为 `pytest -m "not slow" --tb=short -q -rs --cov=newchan --cov-report=term-missing --cov-report=xml:coverage.xml`。本次不改 workflow；这五项会进入该 test job 的默认收集与选择范围。
- 类级 `skipUnless` 仍要求 Bash 与 jq 可用；本机两者存在，本次为五项实际执行、零 skip。若将来 runner 缺少任一依赖，会以该门卫明示 skip，不能据收集成功宣称已执行。此处不替尚未取得的最终远端 job 结果作保证。
- Python 语法仅使用项目已支持的 Python >=3.10 能力。独立实跑 Python 3.11.15 的 unittest，以及 Homebrew Python 3.12 的 pytest；没有声称在本机执行 CI 的 Python 3.13。

## 独立检查记录

| 检查 | 结果 |
|---|---|
| `pytest tests/test_devskim_sarif_gate.py -m 'not slow' --collect-only -q -rs` | 5 项被选择；exit 0 |
| `pytest tests/test_devskim_sarif_gate.py -m 'not slow' --tb=short -q -rs` | 5 passed，零 skip，pytest 报告 1.41 秒；exit 0 |
| `python3.11 tests/test_devskim_sarif_gate.py -v` | Ran 5 tests，全部通过，1.385 秒；exit 0 |
| Python 3.11 `compile` 完整测试文件 | PASS，无仓内 pyc 写入 |
| ruff / mypy / pylint / black | 当前 PATH、Python 3.11 与默认 Python 3.14 均不可用，未安装或伪称已运行 |

本地 pytest 有一条 `Unknown config option: asyncio_mode` 警告，来自现有项目配置与本地未安装 pytest-asyncio 的组合；用例仍全部实际运行。CI 的现有 `[test]` 依赖声明包含 pytest-asyncio。本评审没有把该本地警告改写成测试失败或远端已验证。

原始输出见 `final-collection.*`、`final-pytest.*`、`final-python311.*`。`FINAL-CHECKS.json` 记录命令、退出码、实际 wall time、日志 SHA 及全部源码绑定。初始快照与检查保留，最终结论以 final 文件为准。
