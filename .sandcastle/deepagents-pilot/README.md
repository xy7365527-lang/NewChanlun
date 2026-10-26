# #1742 Deep Agents 试点：如何启动、恢复与查看一个 run

本目录让官方 Deep Agents（LangGraph runtime）里的真实模型读取 pstack `poteto-mode`，按选中的 playbook 推进一个小任务。Feature 任务的一次完整 run 依次做五件事：

1. 并行派两个不同家族的只读设计候选（architect Phase B 的 arena runner）。
2. 派一个与主控不同家族的只读交叉裁判，主控自己选 base、嫁接并合成设计。
3. 派 GPT 作者工蜂实现，宿主做受管本地提交。
4. 派 Opus 独审工蜂审这个精确 head。
5. 在门上停为 `awaiting_approval`。

所有工蜂只经现役 `.sandcastle/codex-child.mts` 启动，隔离与现役相同。试点不 push、不开 PR、不合 main，也不碰常驻队列与服务。

## 在浏览器看图

打开官方 Studio 的方法见 [在官方 Studio 查看试点执行图](STUDIO.md)。此入口只读，不接管原 run。

## 准备环境

依赖锁在 `requirements.lock`（deepagents 0.7.32、langgraph 1.2.17、langgraph-checkpoint-sqlite 3.1.2、langchain-openai 1.7.2、类型检查用的 types-PyYAML）。

```bash
python3.12 -m venv "$VENV" && "$VENV/bin/pip" install -r .sandcastle/deepagents-pilot/requirements.lock
npm ci   # codex-child.mts 需要仓库根的 node_modules
```

模型经本机 CLIProxyLocal（`http://127.0.0.1:8318/v1`）。密钥沿用 codex-child 的受管机制：进程环境有 `CLIPROXY_LOCAL_KEY` 就用，否则 `launchctl getenv` 取一次，只留在内存。启动时关闭 LangSmith 外发。

run 创建时从 `~/.agents/pstack-models.md` 冻结六个角色的模型：

| 角色 | 取自 | 本机当前值 |
|---|---|---|
| Deep Agent 主控 | `hardest tasks` | claude-opus-5-5 |
| 作者 | `feature, refactoring` | gpt-6.1-sol |
| 独审 | `judgment and prose` | claude-opus-5-5 |
| 设计候选 `design-a`、`design-b` | `architect runners` 前两个 | claude-opus-5-5、gpt-6.1-sol |
| 交叉裁判 `judge` | `arena cross-judge pool` 中第一个与主控不同家族的 | gpt-6.1-sol |

作者与独审同家族、两个设计候选同家族，或裁判池里没有与主控不同家族的模型时，run 拒绝创建。

主控推理档是 `high`，单次输出上限 64000 tokens。poteto-mode 规定判断角色比 xhigh 预算低一档；首轮实测 xhigh 把 CLIProxy 缺省的 32000 输出上限全部用于思考，回复为空，图随之结束。工蜂推理档仍由 codex-child 固定为 xhigh。

## 启动一个 run

在仓库根运行：

```bash
cd /Users/silencehan/.codex/worktrees/deepagents-pilot-1742/NewChanlun
PYTHONPATH=.sandcastle/deepagents-pilot "$VENV/bin/python" -m pilot1742 run \
	--run-id r1 --task .sandcastle/deepagents-pilot/demo-task.md
```

任务文件头写 `title`、`allowed_prefixes`，可选 `require_design: true`。设了 `require_design` 时，两个设计候选和裁判都完成前，作者派工会被拒绝。`demo-task.md` 设了它。

`--base` 缺省为本工作区 HEAD。`<home>` 缺省是 `/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027`，可用 `--home` 改。

## Deep Agent 能做什么

主控只有四个业务工具，加上 `write_todos` 和只读的 `ls`、`read_file`、`glob`、`grep`。没有 `execute`、`task`，也没有写文件工具。技能源是整个 `installed/skills/` 树，只读。

`run_worker(role, brief)` 的 `role` 只有四个值。head、模型、checkout 与输出目录都由宿主从回执推导，模型只给 brief。

- `design` 只在首个作者之前、在基线上可用。它把同一个 brief 同时派给 `design-a` 与 `design-b` 两个只读工蜂（codex-child `review` 模式）。两者各用自己的 checkout，即 `<home>/checkouts/<run-id>/design-a` 与 `design-b`，返回两份设计包。
- `judge` 要等两份设计包都完成。裁判在 `judge` checkout 里只读运行，brief 是主控的 rubric，提示里只有“候选 1/候选 2”标签，不出现模型名。
- `author` 在作者 checkout 里写；`review` 在独审 checkout 里只读审当前候选提交。

另外三个业务工具是 `commit_candidate()`、`check_gate()` 和 `run_status()`。arena 的 Phase D 与 E（选 base、嫁接、合成）由主控自己完成，结果写进作者 brief。设计 arena 之后的首个作者 brief 必须单独一行写且只写一个 `BASE: 候选 1` 或 `BASE: 候选 2`，否则作者派工被拒绝；宿主把声明的 base 与裁判首行一起记进 `trail.jsonl` 的 `synthesis` 事件。

每个工蜂的幂等键是 run 加角色加目标 head。同键再调用只会收养已有工蜂，`design` 的两个槽位各自独立收养。

## 恢复被杀掉的 run

原样再跑一次 `run --run-id r1`，不带 `--task`。命令从 SQLite checkpoint 续跑同一个 `thread_id`，重放的工具调用先重认回执：

- 工蜂监督进程独立于控制进程。控制进程被 `kill -9` 后工蜂照常跑完，恢复时收养同一个结果；并行的两个设计候选同样如此。
- 提交已做、回执未落时，按 parent、tree、`Pilot-1742-Run` trailer 认领已有提交，不再提交。
- 输出目录已认领却没有终态、也没有存活监督进程时，run 停为 `halted`，不换键重派。

同一 run 已有控制进程时，第二个 `run` 在任何副作用前以退出码 75 退出。

要复现窄崩溃窗口，加 `--crash-at after_intent_before_spawn`、`after_spawn_before_launch_record` 或 `after_commit_before_receipt`。控制进程会在该点对自己发 SIGKILL。

## 批准与门

门同时检查三件事，缺一不放行：

1. 作者 checkout 的实际分支与完整 HEAD 等于本 run 最后一份提交回执。
2. 该 head 有异家族独审，其 summary 首行整行等于 `VERDICT: PASS`。
3. `<home>/runs/<run-id>/approvals.jsonl` 里同票、同 scope、同 head 的最新人类记录是批准。

有人在作者 checkout 上另加提交或切走分支时，门返回 `candidate_head_drift` 或 `author_branch_changed`，新派工也被拒绝。`status` 同时列出 `recorded_head` 与 `observed_head`。提交已做、回执未落的那一步不算冲突，恢复时会补回执。

试点代码从不写批准账本。人类批准后，由主控逐行登记，`source` 必须指向本票评论 URL 或用户原始消息：

```json
{"ticket":1742,"scope":"pilot_acceptance","head":"<40 位 sha>","decision":"approve","source":{"kind":"tracker_comment","reference":"https://github.com/xy7365527-lang/NewChanlun/issues/1742#issuecomment-<id>","registered_by":"<登记人>"}}
```

`pilot_acceptance` 不推出 `main_merge`。登记后运行 `run --run-id r1 --recheck-approval`，图从 `check_gate` 的暂停点继续。

## 查看状态与证据

`status --run-id r1` 只读，不启动模型或工蜂。run 目录里：

| 文件 | 内容 |
|---|---|
| `run.json` | 冻结的任务、基线、写集与全部角色模型 |
| `agent-trace.jsonl` | 每次模型请求的工具名集合、模型输出、每次工具调用与返回 |
| `trail.jsonl` | 派工、收养、提交、门与崩溃注入的决策记录 |
| `workers/<role>-<head>/` | codex-child 原始 `state.json`、`result.json`、`events.jsonl`、`prompt.md` |
| `prompts/<role>-<head>.md` | 宿主按固定模板生成的工蜂任务；裁判提示里含两份设计包 |
| `commits/*.intent.json`、`*.receipt.json` | 提交意图与回执 |
| `graph.sqlite` | LangGraph checkpoint |

## 当前限制

- 设计候选或裁判失败时，run 停为 `halted`。arena 允许的“少一个候选继续”没有实现。
- 只支持设计 arena。Feature 第 4 步的实现 arena（多个作者并行写代码）没有通道，作者只有一个，审阅分离靠异家族独审。
- 合成说明与合成设计只在作者 brief（`prompts/author-<head>.md`）里，宿主只校验其中的 `BASE` 行，嫁接与拒绝理由的内容不做检查。
- 每个 run 最多建五个工作树，作者、独审、两个设计、裁判各一个；本仓每个约 1.1 GB。
- 人类批准链没有端到端演示过，需要真人批准与主控登记。

## 运行测试

```bash
cd .sandcastle/deepagents-pilot && "$VENV/bin/python" -m pytest -q tests
```

测试用与 codex-child 同参数、同排他认领语义的替身工蜂和真实 git，覆盖以下场景：

- 恢复：控制进程 SIGKILL 后收养单个工蜂与并行设计候选；提交后回执前崩溃；意图后派工前崩溃，含两个设计意图之间崩溃。
- 停机与并发：结果不明时停机；第二个控制进程在任何副作用前被拒绝。
- 设计通道：槽位、模型与 checkout 固定，作者之前必须完成设计与裁判，作者 brief 必须声明唯一 BASE。
- 门与审阅：作者 checkout 外部加提交或切分支后拒绝旧批准；VERDICT 首行严格匹配；越界改动不提交；门的正负控。

门测试里的批准记录是测试内合成的，不进入任何 run 的账本。

## 在 CI 运行回归检查

现有 `CI` 工作流的 `deepagents-pilot` 作业运行本目录全部测试、Ruff、Mypy 和依赖兼容检查。它沿用工作流的 main 推送、面向 main 的 PR 和手动触发，不另设路径过滤。

在 Python 3.12 独立环境中复现该作业：

```sh
cd .sandcastle/deepagents-pilot
python -m pip install -r requirements-ci.txt
python -m pip check
python -m ruff check --select F,E9,B,I .
python -m mypy pilot1742
LANGSMITH_TRACING=false LANGCHAIN_TRACING_V2=false OTEL_SDK_DISABLED=true python -m pytest -q tests
```

测试使用脚本模型、替身工蜂、临时 Git 仓库和临时技能文件，不需要本机 pstack 安装或模型凭据。技能测试检验文件加载机制，不替代 `real4` 对完整上游技能的真实执行验收。此作业不运行真实模型、Sandcastle 工蜂或 Studio 常驻服务。
