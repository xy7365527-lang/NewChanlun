# precompact-save.sh 修复报告

## 诊断结果

脚本在 `set -euo pipefail` 模式下运行。`pipefail` 使得管道中任何一个命令的非零退出码都会传播为整个管道的退出码，而 `set -e` 在检测到非零退出码时立即终止脚本。

### 识别的故障点

以下管道在特定条件下会因 `pipefail` 产生非零退出码，导致 `set -e` 终止脚本：

1. **Line 32-33**: `grep -m1 | sed` — 当 grep 未匹配任何内容时返回 exit 1，`|| echo "?"` 会产生双重输出（grep 的空输出 + echo 的 "?"）
2. **Line 51**: `find | wc -l` — find 在特定目录状态下可能返回非零
3. **Line 57**: `git diff --stat | tail -1` — SIGPIPE 风险（当 git diff 输出大量内容而 tail 只取最后一行时）
4. **Line 111**: `ls -t ... | head -1` — 93 个 session 文件时 SIGPIPE 风险最高（ls 输出 93 行，head 只读 1 行后关闭管道，ls 收到 SIGPIPE 信号，退出码 141）
5. **Line 115**: `sed | head -20` — SIGPIPE 风险
6. **Line 166**: `echo -e | grep -c '|' || echo 0` — grep -c 返回 0 匹配时 exit 1，`|| echo 0` 触发，但 stdout 同时包含 grep 的 "0" 和 echo 的 "0"，导致 DEF_COUNT = "0\n0"
7. **Line 170**: `ls -d | wc -l` — 同类 SIGPIPE 风险

### 最可能的触发场景

**Line 111 的 SIGPIPE** 是最可能的根因。随着 session 文件数量增长到 93 个，`ls -t | head -1` 管道中 `head` 读取第一行后关闭 stdin，`ls` 在尝试写入后续行时收到 SIGPIPE（信号 13），退出码变为 141。在 `pipefail` 下，`for f in $(...)` 的命令替换继承了这个非零退出码，`set -e` 随即终止脚本。

SIGPIPE 的触发是概率性的，取决于操作系统调度和缓冲区大小——文件越多概率越高，这解释了为什么该 bug 间歇性出现。

## 修复内容

所有修复保持功能逻辑不变，仅消除管道失败风险：

| 位置 | 原代码 | 修复 |
|------|--------|------|
| L32-33 | `grep \| sed \|\| echo "?"` | `grep \| sed \|\| true` + `[ -z ] && var="?"` |
| L51 | `find \| wc -l` | 添加 `\|\| true` |
| L57 | `git diff --stat \| tail -1` | 添加 `\|\| true` |
| L111 | `for f in $(ls -t \| head -1)` | 改为 `LATEST_SESSION=$(ls -t \| head -1 \|\| true)` + `if` 判断 |
| L166 | `grep -c \|\| echo 0` | `grep -c \|\| true` + `[ -z ] && DEF_COUNT=0` |
| L170 | `ls -d \| wc -l` | 添加 `\|\| true` |

### 修复策略

统一使用 `|| true` 取代 `|| echo fallback`，避免双重输出问题。对于需要默认值的场景，用后续的 `[ -z "$var" ] && var=default` 模式。

## 验证

```
$ echo '{"cwd":"C:/Users/hanju/NewChanlun"}' | bash .claude/hooks/precompact-save.sh
{"continue": true, "suppressOutput": false, "systemMessage": "[Session] ..."}

$ echo '{}' | bash .claude/hooks/precompact-save.sh
{"continue": true, "suppressOutput": false, "systemMessage": "[Session] ..."}

$ echo '' | bash .claude/hooks/precompact-save.sh
{"continue": true, "suppressOutput": false, "systemMessage": "[Session] ..."}
```

所有测试通过，退出码 0。
