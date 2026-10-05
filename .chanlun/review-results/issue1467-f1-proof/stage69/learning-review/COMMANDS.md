# Stage69 独立审查实际命令

在2026-10-06本审查会话执行，工作目录均不作为数据输入身份依据；源码和数据使用绝对路径。仅第1条完成两个独立复算模型，第2、3条成功fit计数均为0。未执行作者run-author，也未重新运行作者check-saved-evidence。

```sh
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-review/independent_audit.py > /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-review/independent-audit.stdout.txt
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-review/process_boundary_controls.py > /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-review/boundary-controls.stdout.txt
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-review/supplementary_checks.py > /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-review/supplementary-checks.stdout.txt
```

第3条在发现辅助group计数包含null后修正并再次执行；不调用fit。初版结果及更正前后hash留在 `null-group-count-correction.json`。第1条的统计输出代码同步排除null；训练、概率及评价逻辑未改，不再运行第1条，避免超出两次复算预算。`supplementary_checks.py` 的最终版本禁止自动pycache输出。

第1条另外启动一次Node作为纯JSON.stringify/SHA codec，代码完整保存在 `independent_audit.py` 的 `hash_command`；不导入作者学习器。第2条每臂启动的实际命令、PID、时刻和16条请求/响应保存在 `boundary-controls.json`：

```sh
/Users/silencehan/.hermes/node/bin/node --max-old-space-size=64 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-author/learner-stdin.mjs
```

审查运行时：Python3.14.6，Node v22.23.1，macOS26.6.2 arm64；具体可执行路径见 `supplementary-results.json`。这只是本次运行环境，不是生产性能测试。未下载依赖、采集新数据、调用费用/交易服务或改变任何冻结源文件。

最后运行下列命令，对审查Python文件作AST语法解析、再次核所有输入、清除本目录的Python缓存并生成manifest及SHA；没有fit：

```sh
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/learning-review/finalise_review.py
```
