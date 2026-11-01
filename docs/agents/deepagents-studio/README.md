# 在官方 Studio 里查看、继续、恢复和中断 v2 run

本指南用于已由执行器 CLI 登记并首跑过的 v2 run。登记、首跑、对账和释放都使用 CLI，见 [EXECUTOR.md](../../../.sandcastle/deepagents-pilot/EXECUTOR.md)。Studio 接入的实现说明见 [STUDIO.md](../../../.sandcastle/deepagents-pilot/STUDIO.md)。

## 开始前

- 确认 run 已由执行器 CLI 登记并首跑过。
- 确认 `executor status --run-id "$RUN_ID"` 能读到这个 run。
- 准备已安装 `requirements-studio.txt` 依赖的 venv。
- 准备 App 私钥文件的绝对路径。文件须位于所有 Git 检出之外，属于当前用户，权限为 `600` 或 `400`。把路径传给服务，不要把私钥内容放入命令或 Studio。

**如果 run 只登记、从未首跑，不要把它放进 `PILOT1742_STUDIO_RUNS`，先用 CLI 首跑。** Studio 不发起首跑，但对这种 run 点 **Submit** 会在拒绝前先持有这个 run，并向 GitHub 加标签、发持有评论。

在检出根目录、同一个 shell 中定义 CLI 入口。函数使用下一节设置的环境变量。

```sh
executor() { PYTHONPATH=.sandcastle/deepagents-pilot:.sandcastle/approval-signing "$VENV/bin/python" -m pilot1742.executor --state-root "$PILOT1742_STATE_ROOT" "$@"; }
```

这个入口在每次调用时都把 `--state-root` 放在子命令之前。设置 `PILOT1742_STATE_ROOT` 本身不会改变执行器的 `--state-root` 默认值。

如果尚未首跑，先完成下一节第 1、2 步的环境设置，暂不设置 `PILOT1742_STUDIO_RUNS`，再执行下列 CLI 命令。等 CLI 停在中断处或退出后，再设置绑定列表并启动服务。

```sh
executor run --run-id "$RUN_ID" --private-key "$PILOT1742_APP_PRIVATE_KEY"
```

## 启动绑定服务

以下命令在同一个终端执行。把引号内的占位参数换成实际值，run id 使用 `t<票号>-<序号>` 的形式。

1. 进入当前检出的根目录。

   ```sh
   cd '<检出根目录绝对路径>'
   ```

2. 设置端口、环境和绑定列表。状态根必须是 CLI 登记和首跑时使用的那个目录。

   ```sh
   export PORT=2024
   export VENV='<已安装 Studio 依赖的 venv 绝对路径>'
   export STUDIO_STATE='<Studio 服务专用的空目录绝对路径>'
   export PILOT1742_STATE_ROOT='<执行器登记和首跑时使用的状态根绝对路径>'
   export RUN_ID='t<票号>-<序号>'
   export PILOT1742_STUDIO_RUNS="$RUN_ID"
   export PILOT1742_APP_PRIVATE_KEY='<所有 Git 检出之外的私钥文件绝对路径>'
   ```

3. 用前一节的函数核对 run。若读不到，先核对状态根和 run id，暂不启动服务。

   ```sh
   executor status --run-id "$RUN_ID"
   ```

4. 启动本机绑定服务。

   ```sh
   sh .sandcastle/deepagents-pilot/studio.sh
   ```

5. 等终端出现 `pilot1742 studio thread <run_id>: <thread_id>`。记下对应 run 的线程 id。
6. 打开 `https://smith.langchain.com/studio/?baseUrl=http://127.0.0.1:<PORT>`，把 `<PORT>` 换成上面设置的端口。端口为 `2024` 时可打开 [官方 Studio 本地入口](https://smith.langchain.com/studio/?baseUrl=http://127.0.0.1:2024)。登录后选择图 `pilot1742`。
7. 保持启动终端运行。需要停止服务时，在该终端按 `Ctrl-C`。

| 变量 | 含义 | 不设时 |
| --- | --- | --- |
| `PORT` | 本地服务端口。网页 `baseUrl` 的端口必须与它相同。 | 使用 `2024`。 |
| `VENV` | 已安装 Studio 依赖的 venv。CLI 函数和启动脚本使用同一环境。 | 启动脚本使用机器上的固定环境路径。请显式设置；CLI 函数不提供缺省路径。 |
| `STUDIO_STATE` | Studio 服务自己的线程记录目录，不是 run 的 checkpoint 所在目录。首次使用指定一个空目录。 | 使用机器上的固定 Studio 目录，可能含旧线程记录。请显式设置。 |
| `PILOT1742_STATE_ROOT` | 执行器状态根，必须与 CLI 的 `--state-root` 相同。设错时绑定线程会读为空。 | 使用实现中的固定状态根。请显式设置，避免读到别处。 |
| `RUN_ID` | 本指南命令使用的目标 run id。 | 命令没有目标参数。 |
| `PILOT1742_STUDIO_RUNS` | 要绑定的 run id，多个用逗号分隔，且每个都已由 CLI 首跑。 | 没有绑定线程。非法 id 会报 `run-id 只能是小写字母、数字与连字符，最长 41 位`。 |
| `PILOT1742_APP_PRIVATE_KEY` | App 私钥文件路径。仅执行 **Submit** 或 **Resume** 时读取。 | 查看和 **Cancel** 不需要它；继续或恢复会拒绝。 |

脚本只监听 `127.0.0.1`，关闭 tracing 并清掉传给服务的 LangSmith key。本地连接不需要 LangSmith API key。

## 找到 run 的绑定线程

先在启动终端找到 `pilot1742 studio thread <run_id>: <thread_id>`。在 Studio 的线程列表打开这个 `<thread_id>`。

如果终端输出已滚走，在另一个终端进入同一检出根目录，设置相同的 `VENV` 和目标 `RUN_ID`，再计算线程 id。

```sh
PYTHONPATH=.sandcastle/deepagents-pilot:.sandcastle/approval-signing "$VENV/bin/python" -c \
  'import sys; from pilot1742.studio_binding import thread_for; print(thread_for(sys.argv[1]))' \
  "$RUN_ID"
```

这个线程读写 `<PILOT1742_STATE_ROOT>/runs/<run_id>/graph.sqlite`，与 CLI 使用同一条 checkpoint。保持在原绑定线程内操作。不要在 Studio 里新建、复制或分叉线程。

## 判断 run 停在哪里

先核对 run id 和状态根，再查看线程最新 checkpoint 的中断信息和待执行节点。历史 checkpoint 可供查看，执行必须基于最新 checkpoint。

| 你看到的现象 | 状态名 |
| --- | --- |
| 线程没有任何消息。 | [未首跑，返回开始前](#开始前) |
| 本线程有一次从 Studio 发起、仍在运行的 run。 | [运行中](#运行中) |
| 最新 checkpoint 有中断，内容含 `awaiting_approval`。 | [停在签批处](#停在签批处) |
| 最新 checkpoint 有中断，内容含 `operator_interrupt`。 | [停在操作员中断处](#停在操作员中断处) |
| 没有中断，但有待执行节点，且没有控制进程在推进。 | [控制进程被杀后停在待办任务前](#控制进程被杀后停在待办任务前) |
| 没有中断，也没有待执行节点。 | [已结束](#已结束) |

`executor status --run-id "$RUN_ID"` 只读本机，显示业务阶段和 `interrupt_requested`。中断请求存在不代表图已暂停，以线程最新 checkpoint 为准。如果 CLI 还在推进，等它停止或退出后再从 Studio 操作。

## 按状态操作

### 控制进程被杀后停在待办任务前

适用条件：run 已首跑，最新 checkpoint 没有中断、有待执行节点，控制进程已被杀或退出。

1. 确认没有 `executor run` 进程正在推进这个 run。如果有，等它停止或退出。
2. 保持输入框为空。
3. 点 **Submit**。

结果：服务取得 run 锁，在锁内核对 checkpoint 仍是最新，然后执行已有待办。它等同于不带 `--resume` 重跑 `executor run --run-id "$RUN_ID" --private-key "$PILOT1742_APP_PRIVATE_KEY"`。已派工蜂按原键和回执收养，不重复派发。

不会发生的事：服务不会开新一轮任务，也不接受新内容输入。

### 停在签批处

适用条件：最新 checkpoint 的中断内容含 `awaiting_approval`。只有登记时加了 `--require-candidate-approval` 的 v2 run 会停在这里。

1. 阅读中断内容里的签批请求信息。
2. 在签批权威链上完成对该请求的签批。本指南不描述签批操作本身。
3. 点中断下方的 **Resume**。
4. 查看最新 checkpoint。如果又停在签批处，核对当前请求及签批结果，不要反复恢复来代替签批。

结果：执行器忽略恢复载荷，在票级锁内重读签批权威链。签批成立时发布并继续；不成立时再次停在签批处。

不会发生的事：恢复载荷不会成为批准，也不会绕过当前请求的签批判定。

### 停在操作员中断处

适用条件：最新 checkpoint 已实际中断，内容含 `operator_interrupt`，且原先推进这个 run 的运行已停止。

1. 确认看到的是实际中断，而不只是 `interrupt_requested`。
2. 点中断下方的 **Resume**。

结果：服务在 run 锁内清除中断请求，从最新 checkpoint 接着执行。它等同于 `executor run --run-id "$RUN_ID" --private-key "$PILOT1742_APP_PRIVATE_KEY" --resume`。

不会发生的事：服务不会重新首跑，也不会把恢复载荷当作批准。

### 运行中

适用条件：本线程有一次从 Studio 发起、仍在运行的 run。

**如果 run 没在运行，不要点 Cancel。** 服务不检查 run 是否已结束或空闲，仍会写入 `interrupt.request`。如果后续工具调用前请求仍未清除，图会停在操作员中断处。

1. 点这次运行的 **Cancel**。
2. 等待最新 checkpoint 出现含 `operator_interrupt` 的实际中断。
3. 如果出现该中断，按[停在操作员中断处](#停在操作员中断处)继续操作。如果图直接结束，按[已结束](#已结束)处理。

结果：服务返回 `202`，写入中断请求，不取消这次运行。图在下一次工具调用前停下。正在执行的工具调用先跑完；派工调用要等工蜂到终态才返回，因此可能过一段时间才停。如果此后没有下一次工具调用，图可能直接结束而不出现中断。

不会发生的事：已派出的工蜂不会被杀。请求写入不表示图已经暂停。

### 已结束

适用条件：最新 checkpoint 没有中断，也没有待执行节点，且没有控制进程在推进。

1. 只查看线程消息和状态。
2. 不点任何执行按钮，包括 **Submit**、**Resume** 和 **Cancel**。
3. 需要业务阶段时，用 `executor status --run-id "$RUN_ID"` 查看。

结果：你读取现有结果。继续和恢复会以 `run_finished` 被拒；取消不会被拒，但只会留下没有意义的中断请求。

不会发生的事：已结束的图不会因此开启新任务。不要把这个只读操作要求理解成服务会拒绝一切写入。

## 被拒时怎么办

按错误原文查找下表。`<A>`、`<B>`、`<run_id>` 和 `<字段>` 表示错误中的实际值；`<字段>` 可以是逗号分隔的多个字段。错误前还可能带异常类型名。

| 你看到 | 含义 | 处置 |
| --- | --- | --- |
| `checkpoint_not_current：请求基于 <A>，当前为 <B>，不分叉` | 请求选择的 checkpoint 已经不是最新的。 | 回到原绑定线程的最新 checkpoint，重新判断状态。 |
| `checkpoint_moved：请求基于 <A>，当前为 <B>，不分叉` | 点击后、取得 run 锁前，checkpoint 又前进了。 | 刷新到最新 checkpoint，重新判断状态。 |
| `checkpoint_not_allowed` | checkpoint 指针格式、线程或命名空间不被允许。 | 回到原绑定线程的最新 checkpoint，不发起分叉或跨线程请求。 |
| `checkpoint_ambiguous` | 请求里的 checkpoint 指针互相不一致。相同指针值会去重，不因此拒绝。 | 回到最新 checkpoint，用网页默认请求。 |
| `configurable_override_not_allowed` | 请求试图覆盖线程、命名空间或保留配置，或者配置格式不对。 | 回到原绑定线程的最新 checkpoint，不修改高级配置。 |
| `该操作会改写线程状态或存储，不开放` | 正在编辑状态、复制线程，或进行其他未开放的写操作。 | 保留原绑定线程，只按对应状态小节操作。 |
| Studio 这次运行失败，报 `run <run_id> 已有控制进程持锁，本进程未做任何副作用` | CLI 正持有 run 锁，后到的 Studio 执行失败且没做副作用。 | 等 CLI 停止或退出，刷新线程，重新判断状态。 |
| `executor run` 退出码为 `75`，stderr 为 `run <run_id> 已有控制进程持锁，本进程未做任何副作用` | Studio 正持有 run 锁，后到的 CLI 退出且没做副作用。也可能是另一个 CLI 持锁。 | 等正在推进的那次运行结束或停下。同一 run 同一时间只用一边推进。 |
| `409`，`Thread is already running a task. Wait for it to finish or choose a different multitask strategy.` | 同一线程已有一次 Studio 运行。 | 等它结束，不改 `multitask_strategy`。 |
| `该线程未绑定已登记的 v2 run`；`该线程未绑定已登记的 v2 run，不能执行`；`thread_not_bound：只能创建绑定已登记 v2 run 的线程，见 STUDIO.md` | 线程不在绑定列表里，或打开了新建线程。 | 在没有 Studio 运行在飞时停服务。核对 run id 和状态根，把已首跑的 run id 加进 `PILOT1742_STUDIO_RUNS` 后重启，打开 `thread_for` 算出的线程。 |
| `input_not_allowed：首跑由执行器 CLI 发起，Studio 只继续、恢复与中断` | 请求带了新内容，或恢复请求同时带输入。 | 清空输入框。如果停在待办任务前，点 **Submit**；如果已中断，点中断下方的 **Resume**。 |
| `command_only_resume` | 请求中的 command 不是仅含 resume 的恢复命令。 | 用网页默认的恢复操作，不修改 command。 |
| `fields_not_allowed:<字段>` | 请求含字段白名单之外的字段。 | 用网页默认的 **Submit** 或 **Resume**，不改高级选项。 |
| `checkpoint_during_not_allowed` | 请求带了不开放的 checkpoint 写入选项，例如 `checkpoint_during=false`。 | 使用网页默认选项。 |
| `durability_not_allowed` | 请求带了不开放的持久化选项，例如 `durability=exit`。 | 使用网页默认选项。 |
| `nothing_to_resume：当前没有停在中断处，用继续而不是恢复` | 当前有待办，但没有中断。 | 先确认没有控制进程在推进，再按[待办任务前](#控制进程被杀后停在待办任务前)操作，使用 **Submit**。 |
| `interrupted：该 run 停在中断处，只能在中断处恢复` | 当前 checkpoint 有中断。 | 先按中断内容判断类型，再用中断下方的 **Resume**。签批中断须先完成签批。 |
| `run_not_started：首跑由执行器 CLI 发起，Studio 不创造任务` | run 没有首跑 checkpoint。 | 返回[开始前](#开始前)，用 CLI 首跑后再绑定到 Studio。 |
| `run_finished：该 run 的图已结束，Studio 只读` | 图没有中断和待执行节点。 | 不再使用执行按钮，保留只读查看。 |
| 启动终端打印 `已存在但缺 graph_id，未改写` | `STUDIO_STATE` 含缺少 `graph_id` 的旧线程，Studio 列不出或读不到它。 | 在没有 Studio 运行在飞时按 `Ctrl-C`，按下方命令仅换一个空的 `STUDIO_STATE` 重启。 |
| 启动后没有任何 `pilot1742 studio thread` 行 | 绑定列表为空、服务在 120 秒内未就绪，或建线程时出错。 | 检查 `PILOT1742_STUDIO_RUNS` 和终端报错，排除问题后重启。 |
| `Studio 服务未配置 PILOT1742_APP_PRIVATE_KEY，不能执行 v2 run` | 执行服务缺私钥路径。 | 设置 `PILOT1742_APP_PRIVATE_KEY` 后重启。 |
| `私钥必须通过绝对文件路径明确指定` | 私钥路径不是绝对路径。 | 改成仓库外文件的绝对路径后重启。 |
| `私钥必须保存在所有 Git 检出之外` | 私钥文件位于某个 Git 检出内。 | 改用所有 Git 检出之外的私钥路径后重启。 |
| `私钥须由当前用户持有且权限为 600 或 400` | 私钥不是普通文件、属主不对或权限不合要求。 | 改正文件类型、属主或权限后重启。 |
| `无法读取明确指定的私钥文件` | 文件无法打开，例如路径不存在或不可读。 | 核对路径和读取权限后重启，不展示文件内容。 |
| `run <run_id> 的持有已是 halted，只能由主控以新 run 重新持有`；`run <run_id> 的持有已是 released，只能由主控以新 run 重新持有` | 持有已经终止。 | Studio 不能推进这个 run。本指南不提供恢复手段，交给主控处理。 |
| 浏览器显示 `Failed to fetch` | 网页连不上本地服务。 | 确认启动终端仍在运行，执行下方就绪检查，核对链接端口后刷新 Studio。 |

如果旧线程缺 `graph_id`，在没有 Studio 运行在飞且已停止服务的条件下，只覆盖 `STUDIO_STATE` 重启，其他环境变量沿用原值。

```sh
STUDIO_STATE='<新的 Studio 服务专用空目录绝对路径>' sh .sandcastle/deepagents-pilot/studio.sh
```

run 的 checkpoint 仍在 `<PILOT1742_STATE_ROOT>/runs/<run_id>/graph.sqlite`。换 Studio 服务目录不会丢失 run 状态，不要更换状态根或 run id。

如果浏览器显示 `Failed to fetch`，在保留相同 `PORT` 的另一个终端检查本地服务。

```sh
curl --fail "http://127.0.0.1:${PORT}/ok"
```

如果检查成功，确认网页 `baseUrl` 的端口等于 `PORT`，再刷新 Studio。如果失败，查看启动终端的退出或报错信息。服务只允许官方 Studio 来源访问，不需要为此添加 LangSmith key。

## 批准只看签批权威链

恢复载荷从不当作批准，载荷里写什么都不算数。在签批处恢复后，执行器不使用载荷内容判定批准，而是在票级锁内重读 root 保护的 Secure Enclave 签批权威链。只有权威链判定成立，发布动作才继续。

中断内容和 `executor status` 里的签批状态只供查看，不是批准来源。在 Studio 输入框写“批准”不会改变结果，而且会以 `input_not_allowed` 被拒。

## 不要这样恢复

- 不要靠锁冲突抢进度。等持锁的运行结束或停下，再判断最新 checkpoint。
- 不要删除或改动 run 目录里的 `owner.lock`。
- 不要换 run id 或重新登记。新 run 不会接着当前 checkpoint 跑。
- 不要在 Studio 里新建、复制或分叉线程。
- 不要改 `multitask_strategy`。

## 依据

接口以本节文件为准，文件变更后请回查。

- [.sandcastle/deepagents-pilot/STUDIO.md](../../../.sandcastle/deepagents-pilot/STUDIO.md)
- [.sandcastle/deepagents-pilot/EXECUTOR.md](../../../.sandcastle/deepagents-pilot/EXECUTOR.md)
- [.sandcastle/deepagents-pilot/studio.sh](../../../.sandcastle/deepagents-pilot/studio.sh)
- [.sandcastle/deepagents-pilot/langgraph.json](../../../.sandcastle/deepagents-pilot/langgraph.json)
- [.sandcastle/deepagents-pilot/requirements-studio.txt](../../../.sandcastle/deepagents-pilot/requirements-studio.txt)
- [.sandcastle/deepagents-pilot/pilot1742/studio.py](../../../.sandcastle/deepagents-pilot/pilot1742/studio.py)
- [.sandcastle/deepagents-pilot/pilot1742/studio_binding.py](../../../.sandcastle/deepagents-pilot/pilot1742/studio_binding.py)
- [.sandcastle/deepagents-pilot/pilot1742/studio_seed.py](../../../.sandcastle/deepagents-pilot/pilot1742/studio_seed.py)
- [.sandcastle/deepagents-pilot/pilot1742/cli.py](../../../.sandcastle/deepagents-pilot/pilot1742/cli.py)
- [.sandcastle/deepagents-pilot/pilot1742/executor.py](../../../.sandcastle/deepagents-pilot/pilot1742/executor.py)
- [.sandcastle/deepagents-pilot/pilot1742/run.py](../../../.sandcastle/deepagents-pilot/pilot1742/run.py)
- [.sandcastle/deepagents-pilot/pilot1742/agent.py](../../../.sandcastle/deepagents-pilot/pilot1742/agent.py)
- [.sandcastle/deepagents-pilot/pilot1742/approval.py](../../../.sandcastle/deepagents-pilot/pilot1742/approval.py)
- [.sandcastle/deepagents-pilot/pilot1742/github_app.py](../../../.sandcastle/deepagents-pilot/pilot1742/github_app.py)
- [.sandcastle/deepagents-pilot/pilot1742/domain.py](../../../.sandcastle/deepagents-pilot/pilot1742/domain.py)
- venv 内的 `lib/python*/site-packages/langgraph_api/models/run.py`。
