# #1372 最终提交的远端 pytest 独立验收

结论：`041c8e2ef7916a465d8b352ff9dc1469707f8ac1` 的既有 CI `test` job 已成功。日志与源码集合交叉核对支持新增五项 DevSkim 回归测试已实际执行并通过，没有被跳过。本轮只读下载既有运行记录，没有重跑本地或远端测试，没有修改源码、CI 或先前独评。

## 远端运行及源码绑定

- Run：`34755511547`，job：`103719090618`（`test`）。
- 结果链接：https://github.com/xy7365527-lang/NewChanlun/actions/runs/34755511547/job/103719090618
- head：`041c8e2ef7916a465d8b352ff9dc1469707f8ac1`。
- 实际 checkout：`1098c0cacf74e4a79e3966d4e1a280d6a9a37081`，父提交为 `e46cf3bca6a67da821f9ab046507563b8133f6d9` 与上述 head。
- GitHub API 给出的实际合并树与本地 head 树均为 `4ebe9ffe886229634640a0e7c5da57f9d8ecacb8`，逐树相同。
- 受测 Python 文件 SHA-256：`6ca76cb11f3ac7cb40ac767adf0440fe3e95a6f9e95ac9233755044205aeb33c`；Bash gate：`c3154bd748c796ba950d1b4926308cd37a1cf54b0a802f644b6a8877132ce5fc`。五份相关 Git 原字节保存在 `committed-source/`，没有拿工作树的后续状态替代已运行提交。

## 整体 test 结果

既有命令仍为 `pytest -m "not slow" --tb=short -q -rs --cov=newchan --cov-report=term-missing --cov-report=xml:coverage.xml`。API 中 `Run tests with coverage` 步骤及整个 `test` job 均为 `success`；该步骤没有 continue-on-error，pytest 命令按成功退出收束。日志没有另外打印一个独立的数值 exit_code 字段。

| 结果 | 相邻旧提交 acd30cb61e | 当前提交 041c8e2ef7 |
|---|---:|---:|
| passed | 5054 | 5059 |
| skipped | 107 | 107 |
| deselected | 11 | 11 |
| xfailed | 1 | 1 |
| warnings | 23 | 23 |
| pytest 时间 | 143.10 秒 | 166.40 秒 |

当前运行在 Python 3.13.15。时间是整个 pytest 套件的日志统计；没有用两次整套耗时之差推算新增五项耗时。

## 新五项确实运行的核验方法及边界

`-q -rs` 不逐个打印成功节点名，因此本报告不声称日志有五条具名 PASS。结论来自完整集合的排除与差量证据：

1. 实际 checkout 树与目标 head 相同；相邻两次 checkout 的共同 main 父提交都是 e46cf3bc，排除了默认分支变化污染比较。
2. 两个 head 的完整差异仅为报告文件、Bash gate 与新 `tests/test_devskim_sarif_gate.py`；tests 范围只有这一个新增文件。该文件恰好定义五项 `test_` 方法，且源码与先前实际收集五项的独评候选相同。
3. 既有 `testpaths = ["tests"]`、测试收集命令、`not slow` 选择器及 conftest 均未改；新文件无 slow 标记、无 xfail 标记或专属排除规则。
4. 两份完整日志的 skip 摘要各有 54 行，数量合计正好 107；去掉日志时间戳后，全部摘要逐字相同。没有 `test_devskim_sarif_gate.py`，也没有其依赖门卫原因 `门禁测试需要 Bash 与 jq`。
5. 在 skipped、deselected、xfailed 均不变的条件下，passed 恰好增加 5。因此新增五项已进入实际通过集合，不能解释为缺 jq 或缺 Bash 而集体 skip。

完整 54 行 skip 原文、计数与 head 差异存于 `COMPARISON.json` 和 `complete-head-diff.txt`；没有删减既有跳过或把它们计作通过。本报告不把这 107 项未执行的既有检查变成已验证。

## Bash 与 jq 的观察证据

- **Bash 直接运行证据**：当前 job 原日志在多个步骤明确记载 `shell: /usr/bin/bash -e {0}`，包括 pytest 步骤。
- **runner 身份**：原日志声明 `ubuntu-24.04` / `20260907.300.1`，并给出固定镜像的软件清单 URL。对应原始 Markdown 已通过 GitHub API 只读保存为 `runner-image-software.raw`。
- **清单证据**：该固定清单列出 Bash `5.2.21(1)-release`、jq 软件项 `1.7`，APT 表中 jq 包版本为 `1.7.1-3ubuntu0.24.04.2`。这是镜像清单，不是本 job 现场运行 `jq --version` 或 `bash --version` 的结果。
- **jq 在测试进程中可用的证据**：新增 TestCase 的类级 skipUnless 同时检查 Bash 与 `shutil.which("jq")`。完整 skip 集合未增加且 passed 增加 5，说明此门卫没有触发；用例实际 subprocess 又依赖 jq 解析 SARIF。日志没有单独打印 jq 路径或版本，本报告不补造该读数。

## 原始收据

- 当前完整 job 日志 `test.log`：134,077 bytes，SHA-256 `c7edb76b9d5322cf1d26b88768012d7304355f9aa4669133e574908e397ec8ab`。
- 相邻旧完整 job 日志 `previous-test.log`：134,629 bytes，SHA-256 `772a4d5334745360f7f6e29b8efa10b276d1f5701e9b1db7376c13a9c1b4859e`。
- `job.json`、`run.json` 及旧 run/job JSON 为 GitHub API 原响应；`current-merge.json`、`previous-merge.json` 为合并树与父提交原响应。
- `INITIAL-RECEIPT.json`、`COMPARISON-RETRIEVAL.json`、`FINAL-RETRIEVAL.json`、`RUNNER-IMAGE-RETRIEVAL.json` 保存读取命令、退出码和内容摘要。
- `FACTS.json` 保存结论、全部身份信息和关键原日志行号；`MANIFEST.json` 对本目录全部既有文件逐项绑定 SHA。

本次验收仅覆盖 `test` job。其他三个常规 CI job、DevSkim 结果和 C 合入 main 的授权由根代理分别核验，本报告不代为批准。
