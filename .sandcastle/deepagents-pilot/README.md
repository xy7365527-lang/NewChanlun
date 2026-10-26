# #1742 Deep Agents 试点：如何启动、恢复与查看一个 run

本目录让官方 Deep Agents（LangGraph runtime）里的真实模型读取 pstack `poteto-mode` 并按 playbook 推进一个小任务：派 GPT 作者工蜂、宿主受管本地提交、派 Opus 独审工蜂，最后在门上停为 `awaiting_approval`。工蜂只经现役 `.sandcastle/codex-child.mts` 启动，隔离与现役相同。试点不 push、不开 PR、不合 main，也不碰常驻队列与服务。

## 准备环境

依赖锁在 `requirements.lock`（deepagents 0.7.32、langgraph 1.2.17、langgraph-checkpoint-sqlite 3.1.2、langchain-openai 1.7.2）。

```bash
python3.12 -m venv "$VENV" && "$VENV/bin/pip" install -r .sandcastle/deepagents-pilot/requirements.lock
npm ci   # codex-child.mts 需要仓库根的 node_modules
```

模型经本机 CLIProxyLocal（`http://127.0.0.1:8318/v1`）。密钥沿用 codex-child 的受管机制：进程环境有 `CLIPROXY_LOCAL_KEY` 就用，否则 `launchctl getenv` 取一次，只留在内存。启动时关闭 LangSmith 外发。角色模型取 `~/.agents/pstack-models.md`：`feature, refactoring` 行是作者，`judgment and prose` 行是独审，`hardest tasks` 行是 Deep Agent 主控。作者与独审同家族时 run 拒绝创建。

## 启动一个 run

在仓库根运行：

```bash
cd /Users/silencehan/.codex/worktrees/deepagents-pilot-1742/NewChanlun
PYTHONPATH=.sandcastle/deepagents-pilot "$VENV/bin/python" -m pilot1742 run \
	--run-id r1 --task .sandcastle/deepagents-pilot/demo-task.md
```

`--base` 缺省为本工作区 HEAD。新 run 会在 `<home>/checkouts/<run-id>/` 下建作者与独审两个 `codex/deepagents-pilot-1742-<run-id>-*` 工作树。`<home>` 缺省是 `/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027`，可用 `--home` 改。

## 恢复被杀掉的 run

原样再跑一次 `run --run-id r1`，不带 `--task`。命令从 SQLite checkpoint 续跑同一个 `thread_id`，重放的工具调用先重认回执：

- 工蜂监督进程独立于控制进程，控制进程被 `kill -9` 后工蜂照常跑完，恢复时收养同一个结果。
- 提交已做、回执未落时，按 parent、tree、`Pilot-1742-Run` trailer 认领已有提交，不再提交。
- 输出目录已认领却没有终态、也没有存活监督进程时，run 停为 `halted`，不换键重派。

同一 run 已有控制进程时，第二个 `run` 在任何副作用前以退出码 75 退出。

要复现窄崩溃窗口，加 `--crash-at after_intent_before_spawn`、`after_spawn_before_launch_record` 或 `after_commit_before_receipt`。控制进程会在该点对自己发 SIGKILL。

## 批准与门

门只在当前 head 有异家族 `VERDICT: PASS`、且 `<home>/runs/<run-id>/approvals.jsonl` 里有同票、同 scope、同 head 的人类批准时放行。试点代码从不写这个账本。人类批准后，由主控逐行登记，`source` 必须指向本票评论 URL 或用户原始消息：

```json
{"ticket":1742,"scope":"pilot_acceptance","head":"<40 位 sha>","decision":"approve","source":{"kind":"tracker_comment","reference":"https://github.com/xy7365527-lang/NewChanlun/issues/1742#issuecomment-<id>","registered_by":"<登记人>"}}
```

`pilot_acceptance` 不推出 `main_merge`。候选 head 一变，旧批准不再匹配。登记后运行 `run --run-id r1 --recheck-approval`，图从 `check_gate` 的暂停点继续。

## 查看状态与证据

`status --run-id r1` 只读，不启动模型或工蜂。run 目录里：

| 文件 | 内容 |
|---|---|
| `run.json` | 冻结的任务、基线、写集与三个模型 |
| `agent-trace.jsonl` | 每次模型请求的工具名集合、模型输出、每次工具调用与返回 |
| `trail.jsonl` | 派工、收养、提交、门与崩溃注入的决策记录 |
| `workers/<role>-<head>/` | codex-child 原始 `state.json`、`result.json`、`events.jsonl`、`prompt.md` |
| `commits/*.intent.json`、`*.receipt.json` | 提交意图与回执 |
| `graph.sqlite` | LangGraph checkpoint |

## 运行测试

```bash
cd .sandcastle/deepagents-pilot && "$VENV/bin/python" -m pytest -q tests
```

测试用与 codex-child 同参数、同排他认领语义的替身工蜂和真实 git，覆盖控制进程 SIGKILL 后收养、提交后回执前崩溃、意图后派工前崩溃、结果不明停机、并发控制进程、越界改动和门的正负控。门测试里的批准记录是测试内合成的，不进入任何 run 的账本。
