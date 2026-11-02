# #1739 的 Git 提交号误报登记

[DevSkim run 40293659552](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40293659552) 扫描成功，gate 拒绝五个新增三元键。原始 SARIF 实际包含六条 DS173237，其中 `1738-MERGED26AP.json` 同行两条被 gate 合为一个键。

## 六条均为已核实的 Git 提交号

规则消息是 `Do not store tokens or keys in source code.`。逐条核对完整 result、JSON 字段及 GitHub 原件，命中值只有 PR #1738 的候选 head 和 squash 合入提交。`git cat-file -t` 对两个值均返回 `commit`，GitHub PR 回读的 `headRefOid`、`mergeCommit.oid` 与之相同。它们是 CI 和合入证据，不是认证凭据。

| 文件，均位于 `ci/` | JSON 字段 | 第 1 行起始列 |
| --- | --- | --- |
| `1737-MACOS-SUCCESS26AN.json` | `headSha` | 34 |
| `1737-MAIN-CI-SNAPSHOT26AP.json` | `headSha` | 27 |
| `1737-MAIN-MACOS-SNAPSHOT26AP.json` | `headSha` | 27 |
| `1737-PR-CI-SUCCESS26AP.json` | `headSha` | 34 |
| `1738-MERGED26AP.json` | `headRefOid`、`mergeCommit.oid` | 35、99 |

## 仅登记完整 result 身份

`.github/devskim-reviewed-1488.json` 新增这五个文件、六条 result 的精确身份。每条绑定完整 SARIF result，每个文件绑定原件字节，沿用 #1488 既有机制。既有 76 个文件登记逐项不变。没有修改 baseline、扫描范围、规则、gate 或 12 份归档原件。

以同一份 GitHub 全仓 SARIF 回灌既有 gate，修复前为 `EXIT=1`，五个新增键；修复后为 `EXIT=0`，经审排除数从 2835 增至 2841，其余结果仍按原 baseline 核验。这是已上传原件的本地 gate 重放，不是一次新 GitHub 扫描或上传。

将一条真实 result 的起始列加 1，分类器仍保留该新结果，六条原结果精确匹配。`python3 -m unittest discover -s tests -p 'test_devskim*py' -v` 为 22/22 PASS，`git diff --check` 通过。

本次增量仍需非作者差异独审与新 head 的 GitHub CI。原始 SARIF、失败日志、前后 gate 输出、负控与测试日志留在外盘 `1739-DEVSKIM-REPAIR26AR-evidence/`。本登记不关闭 #1737。
