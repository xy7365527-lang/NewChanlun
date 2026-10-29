# #1742：审阅已有 PR 并发布 App 状态

原业务、harvest 或人工创建 PR 后，用此入口审阅整个 PR。原 owner 继续拥有代码、分支和 PR；入口不认领票、不提交、不 push、不改 PR，也不代替 main 的合入批准。

## 部署后审一张 PR

在已部署且 `.sandcastle` 干净的 controller checkout 根目录运行：

```sh
export PYTHONPATH=.sandcastle/approval-signing:.sandcastle/deepagents-pilot
"$VENV/bin/python" -m pilot1742.external_review review-pr \
  --pr <PR号> --private-key <仓库外PEM绝对路径>
```

入口读取固定仓库 `xy7365527-lang/NewChanlun` 的 PR 身份，接受以 main 为目标的同仓库 open PR，包括 draft 和非 App 作者。fork 返回 `unsupported_head_repository`。

审阅冻结 base、head 和 merge-base，范围为 `merge_base..head` 的完整修改，不只审最后一次提交。对象必须已在当前 controller 的本地对象库；缺失时返回 `needs_source_sync`，由原 owner 按其既有同步授权补齐。入口和 reviewer 都不 fetch。

两位 reviewer 固定请求 `gpt-6.1-sol`、`claude-opus-5-5`，各自独立 checkout、进程和新 session，不要求作者自报模型。模型字段记录 launcher 所选路由；实际上游模型身份仍须由独立路由证据确认。

## 启用前完成部署验收

此入口要求当前部署代码 head 已有正式 accepted Docker 记录。代码、镜像或合同改变后旧记录不再适用；不借用旧受控验收行，也不回退宿主执行。

部署 owner 须完成当前 head 的真实验收：

- 两个模型真实运行，并取得不同 session 的完整日志和 `turn.completed`。
- 实际容器镜像 Id、挂载、env 符合合同；源码和 Git 写入拒绝，非秘密宿主 canary 读取和宿主状态目录写入拒绝。
- 一个非执行器拥有的 PR 走通双 PASS、同 head 的 App success 和读回。
- 在隔离临时分支验证普通用户同名 success 不能满足 App 来源绑定，App 正控及新 head 负控成立。

main 的 D5 检查启用由主控另行执行，保留已有保护。所需来源绑定为 `deepagents/review`、App id `5486877`，并保留 strict、enforce_admins；本 CLI 不修改 branch protection。首版仅接 main 目标，临时目标分支的来源测试需要主控独立受控测试工具。

## 看结果与恢复

先写 `pending`，两份 PASS 写 `success`，完整审阅 BLOCK 写 `failure`，启动、监督、解析或隔离失败写 `error`。第一位 reviewer FAILED 后不再调用第二个模型。PR base、head、目标或 controller 代码发生变化时记为 `STALE`，保留已完成的 reviews，不将旧 PASS 迁到新 SHA。

退出码：PASS 或离线命令成功为 0，BLOCK/FAILED/STALE 为 1，输入、身份或待对账错误为 2。

相同 PR、base、head、policy、attempt 重跑会收养原 launch 和终态，不重复调用模型。默认 attempt 为 1。监督已失败或需要人工重试时，显式换新 attempt：

```sh
"$VENV/bin/python" -m pilot1742.external_review review-pr \
  --pr <PR号> --attempt 2 --private-key <仓库外PEM绝对路径>
```

新 run 接管同 head 状态后，旧 run 不能覆盖它。BLOCK 不会自动重试到 PASS。已记录且确认退出的监督进程缺终态时记为 FAILED。launch 后尚无进程记录且未见终态，或 PID 身份不明时保留待处理状态，不自动重派；先核对本次 launch，再决定新 attempt。

以下命令只读本机历史，不联网、不读凭据；其输出不代表当前远端 PR head：

```sh
"$VENV/bin/python" -m pilot1742.external_review status --pr <PR号>
```

状态写请求结果不明时只读回认领，不自动重发。仅当主控已经核实远端未生效，才登记该事实，再原样恢复：

```sh
"$VENV/bin/python" -m pilot1742.external_review reconcile \
  --run-id <宿主生成的UUID> --note "<已核实的远端未生效事实>"
```

`reconcile` 只变更本地待对账记录，不发布状态，也不接受 PASS 文件。

## 查证据

固定宿主状态根为 `/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/external-review`。每个 `runs/<运行键>/` 保存：

| 文件 | 用途 |
|---|---|
| `target.json` | 冻结 PR 身份、SHA、attempt、controller head、注册顺序 |
| `<model>/prepare.json`、`launch.json`、`process.json` | 宿主 checkout 所有权、派工意图与进程 |
| `<model>/checkout-cleanup.json` | checkout 已回收或因活动与容器状态不明而保留 |
| `<model>/supervisor.json`、`receipt.json` | 监督阶段、容器、物理隔离探针及终态 |
| `<model>/events.jsonl`、`session.jsonl` | 实际 stdout 事件与独立 session 日志 |
| `<model>/supervisor.log`、`stderr.log` | 宿主入口和模型诊断；SDK 异常仅记类别与阶段 |
| `outcome.json`、`effects/*.json` | 双审结论与状态写入、读回、对账证据 |

无凭据 detached checkout 位于外盘 `/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/checkouts/external-review/<run-id>/<model>`。每份对象库只保存 base、head、merge-base 的源码树及祖先 commit 元数据，不带无关历史版本的树和 blob。`git merge-base`、完整 PR diff、三端源码和提交元数据可用。其它历史提交的源码不在快照内。Git config、hooks、alternates 和凭据不从源仓复制。

prepare 前按导入对象未压缩大小、head 文件大小及 256 MiB 余量检查空间，模型启动前再次核对余量。空间不足时明确失败，不启动模型。

宿主先保存 `prepare.json`，再构造 checkout。prepare 中断且尚无 launch 时，同键重跑只回收本 run 的未启动 checkout 并重建。`launch.json` 之后沿用原收养协议，不自动重复派工。监督在读取 launch、获取锁等启动阶段失败也保存失败终态。已记录的监督进程退出却未写终态时，controller 生成 FAILED 记录并发布 error，不永久停留在 pending。

宿主在终态、进程结束且容器已确认关闭后回收本 run 创建的 checkout，证据目录保持不动。仍有活动进程、PID 身份不明或容器创建与关闭结果不明时保留 checkout，并记录 `checkout-cleanup.json`。同键重跑会再次尝试安全回收。旧版本没有 prepare intent 的 checkout 不自动回收。

源码和 Git 全只读挂入容器。宿主 home、PEM、GitHub token、状态和 session 目录不挂入。session 从容器临时目录经 SDK 收取，不给 reviewer 可写宿主回执目录。

App token 只申请 metadata/read、contents/read、pull_requests/read、statuses/write；token 留在宿主发布进程内存。GitHub 的 statuses/write 不按 context 缩权，宿主发布器及 OS 因此属于受信边界。

## 运行靶向检查

```sh
PYTHONPATH=.sandcastle/approval-signing:.sandcastle/deepagents-pilot \
  "$VENV/bin/python" -m pytest -q .sandcastle/deepagents-pilot/tests/test_external_review.py
node_modules/.bin/tsx --test .sandcastle/external-review-worker.test.mts .sandcastle/codex-route.test.mts
npm run typecheck
```

这些测试使用真实临时 Git、替身进程和 fake API/Docker，不发模型请求，不替代上面的真实部署验收。
