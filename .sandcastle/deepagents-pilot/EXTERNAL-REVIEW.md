# #1742：审阅已有 PR 并发布 App 状态

原业务、harvest 或人工创建 PR 后，用此入口审阅整个 PR。原 owner 继续拥有代码、分支和 PR；入口不认领票、不提交、不 push、不改 PR，也不代替 main 的合入批准。

## 部署后审一张 PR

在已部署且 `.sandcastle` 干净的 controller checkout 根目录运行：

```sh
export PYTHONPATH=.sandcastle/approval-signing:.sandcastle/deepagents-pilot
"$VENV/bin/python" -m pilot1742.external_review review-pr \
  --pr <PR号> --private-key <仓库外PEM绝对路径>
```

入口读取固定仓库 `Newchanlun/NewChanlun` 的 PR 身份，接受以 main 为目标的同仓库 open PR，包括 draft 和非 App 作者。fork 返回 `unsupported_head_repository`。

审阅冻结 base、head 和 merge-base，范围为 `merge_base..head` 的完整修改，不只审最后一次提交。对象必须已在当前 controller 的本地对象库；缺失时返回 `needs_source_sync`，由原 owner 按其既有同步授权补齐。入口和 reviewer 都不 fetch。

两位 reviewer 固定请求 `external_review.MODELS` 中的两个模型（常规为 `gpt-6.1-sol`、`claude-opus-5-5`；当前是下节的临时组合 `kimi-k3.1-pro`、`claude-opus-5-5`），各自独立 checkout、进程和新 session，不要求作者自报模型。模型字段记录 launcher 所选路由；实际上游模型身份仍须由独立路由证据确认。

## Codex 额度耗尽时的临时组合

触发条件：CLIProxy 的 Codex/OpenAI 上游没有可用额度，`gpt-6.1-sol` 请求全部失败（2026-11-07 实测 401，唯一账号额度为 0，reset_at=2026-11-11T01:00Z），此时任何新审都无法得到两份 PASS。

临时组合是 `MODELS = ("kimi-k3.1-pro", "claude-opus-5-5")`、`POLICY = "external-pr-v1-kimi"`。两位仍属不同家族（moonshot 与 anthropic），仍要两份 PASS 才发 success。`kimi-k3.1-pro` 经同一个 CLIProxyLocal 走 OAuth 直连，CLIProxy 在国内 `kimi@api.kimi.com` 与国际 `kimi-ai@api.kimi.ai` 两个上游间切换，预检两者都认。

POLICY 与模型都进了 `REVIEW_CONTRACT`，换组合后旧组合的 PASS 不能沿用，旧组合的 STALE 也不能 `migrate-stale`，都须在新组合下重审。旧 run 里已退役 reviewer 留下的派发与 BLOCK 证据仍计入完结与否决判断：控制器按 run 目录里实有的 worker 读，不只读当前 `MODELS`。

部署：Sandcastle 主模型另行切换，设 `SANDCASTLE_MAIN_MODEL=kimi-k3.1-pro`。审查员派发不依赖主模型取值，`.sandcastle/codex-child.ts` 的 `EXTERNAL_REVIEW_MODELS` 总在 `ALLOWED_MODELS` 里。部署后照常先跑 `review_preflight.py`，它探测的就是当前两位。

到期条件：Codex 上游恢复（`gpt-6.1-sol` 预检的 `route` 与 `roundtrip` 都通过）后换回常规组合。改这三处，保持一致（测试核对）：

- `pilot1742/external_review.py`：`MODELS = ("gpt-6.1-sol", "claude-opus-5-5")`、`POLICY = "external-pr-v1"`；
- `review_preflight.py`：`MODELS` 同上；
- `../codex-child.ts`：`EXTERNAL_REVIEW_MODELS = ["gpt-6.1-sol", "claude-opus-5-5"]`。

同时改回测试里写死的临时组合（`tests/test_external_review.py` 的 `test_fallback_pair_*`、`tests/test_review_preflight.py` 的 `KIMI`/`OFFICIAL`、`../codex-child.test.ts` 的审查员用例）。换回同样会改变合同：本组合下的 PASS 不沿用，本组合的 STALE 不迁移。

## 启用前完成部署验收

此入口要求当前部署代码 head 已有正式 accepted Docker 记录。代码、镜像或合同改变后旧记录不再适用；不借用旧受控验收行，也不回退宿主执行。

部署 owner 须完成当前 head 的真实验收：

- 两个模型真实运行，并取得不同 session 的完整日志和 `turn.completed`。
- 实际容器镜像 Id、挂载、env 符合合同；源码和 Git 写入拒绝，非秘密宿主 canary 读取和宿主状态目录写入拒绝。
- 一个非执行器拥有的 PR 走通双 PASS、同 head 的 App success 和读回。
- 在隔离临时分支验证普通用户同名 success 不能满足 App 来源绑定，App 正控及新 head 负控成立。

main 的 D5 检查启用由主控另行执行，保留已有保护。所需来源绑定为 `deepagents/review`、App id `5486877`，并保留 strict、enforce_admins；本 CLI 不修改 branch protection。首版仅接 main 目标，临时目标分支的来源测试需要主控独立受控测试工具。

## 看结果与恢复

先写 `pending`，两份 PASS 写 `success`，完整审阅 BLOCK 写 `failure`，启动、监督、解析或隔离失败写 `error`。全新派发时两位 reviewer 并发，各用自己的 worker 目录和 checkout，全部结束后汇总，任一 FAILED 即整轮 FAILED。恢复（任一 worker 已留下状态）时按原顺序串行：第一位 FAILED 后不再派发新的 reviewer，但已派发的照样收养到终态；待对账立即上抛。汇总时，只要还有已派发 reviewer 没有回执，就不写汇总 outcome（上抛待对账）；STALE 可能在 reviewer 仍在运行时写出，所以沿用判断不以 outcome 存在为完结。所有终态发布（新审汇总、中断续发、STALE 恢复）都过同一道门：PR 锁内确认同 head 没有更晚登记的 attempt；沿用结果续发前重新做部署准入并在锁内重读历史、确认来源未变，失效就记 STALE（原 outcome 另存历史）而不发 success；终态已发出或已发布（发布记录为 pending 或 done）的只对账：先读回远端，读到即认领，pending 读不到上抛待对账、不重发；若该 head 的状态已归更新的 run，对账会把本 run 记为 STALE（publication 保留），不重发。主控用 `reconcile` 核实为未生效（reconciled_not_applied）的，等同未发布，恢复时照常过上面的门。派发模型前也先查同 head 是否已有更晚登记的 attempt，有就记 STALE、不发 pending、不派模型。STALE 与否按出处判定：controller 身份不符或 PR 内容与冻结目标不符才是漂移；向 GitHub/App 读取 PR 时的任何失败（认证、token 刷新、安装、传输）都不判 STALE，上抛待对账。已记为 STALE 的 run（包括旧版本误记的），用同一 attempt 重跑即受管恢复，是否补发不看原因文案，只看重验：PR 与冻结目标一致、controller 未变、未被取代、全部回执重验且与原记录一致，才补发，不重跑模型，原 STALE 另存为历史。controller 身份始终是正在执行的代码所在仓库的 HEAD 且 .sandcastle 干净，不可由参数替换。新部署要补发旧 controller 下的 STALE，用显式的 `migrate-stale` 子命令：发布者（当前 controller）须干净并通过部署准入，outcome 的 `migrated` 同时记录原审查 controller 与当前发布者及其准入，不冒充旧 controller；迁移只用于审查合同未变的 run：policy 须与当前一致，target 记了合同的须与当前合同相同，没记的（旧 controller 登记）以各模型实际派发的提示词核对，任一不符就拒绝迁移、须新审。迁移出的结论不作为沿用来源。迁移在发布时中途失败（如 GitHub 401）可再次迁移，从原 STALE 记录重来；迁移被同 head 更晚的 attempt 取代时，磁盘结论回到 STALE。并发进行中控制器被中断（如 Ctrl-C）时，线程不再等回执，立即退出；已派发的 reviewer 之后按 launch 收养恢复，与串行时一致。PR base、head、目标或 controller 代码发生变化时记为 `STALE`，保留已完成的 reviews，旧 run 不把状态写到新 SHA。

新 head 可以沿用旧 PASS，不调用模型。条件：同 PR 某个旧 head 的正式双审为 PASS 且已发布 success；`patch_id`、`policy_version`、`review_contract`、`controller_code_head` 都相同；并且沿用前通过与正式派发同一组的部署准入（`external-review-worker.mts --admission-only`：controller 已提交且 head 相符、当前镜像有 accepted 记录），准入镜像与原 run 两份回执的 `image_id` 相同。并发合同：同一 PR 的登记（写 target.json）与沿用发布用同一把跨进程 PR 锁串行化；部署准入在锁外先做，锁内重读历史、确认来源仍是同一个 run 后才发布，所以发布时所依据的历史在读后不可能再出现同 PR 的新登记；历史变了就退回双审（补做总准入）。这时新 run 在新 head 上先写 pending 再写 success，描述带 `reuse=<原 run_id> p=<patch_id 前 12 位>`，`outcome.json` 的 `reused_from` 留原 run 的 SHA、patch_id、双审结论、原发布记录和本次准入。以下情况照常双审：当前 head 上已经有过别的 run（对同一 head 再发 attempt 就是要求重新独审）；补丁不同；原结论是 BLOCK、FAILED 或 STALE；本 run 已有任何 worker 状态（双审已经开始，只能恢复双审）；同一补丁的任何其他 run 尚未完结——完结指顶层 outcome 存在，并且所有已派发（有 launch.json）的 reviewer 都有可读回执；outcome 存在本身不算完结，它可能写于另一位 reviewer 仍在运行时（包括在途的显式重审、STALE）；作为沿用来源的 PASS run 也必须已完结；来源只取同补丁最晚登记的那次真实双 PASS，在它之后登记的同补丁 run 只要有一个不是 PASS（例如以 FAILED 或 STALE 收场的显式重审），旧 PASS 就被视为已被取代，不得在追平后复活（由该来源沿用出来的 PASS 不算推翻）；同一补丁的任何 run 里任一 reviewer 留下过 BLOCK 证据——不看整轮结论，逐个读 outcome 的逐位结论、各 worker 的 receipt 结果，以及 events 里的最终消息（receipt 未能消费结果或汇总前中断时 BLOCK 只在这里；能解析成结果对象就按 status 与 summary 首行判定，说明里引用「VERDICT: BLOCK」不算，解析不了才按整段文本匹配），证据读不了（含 `_read` 的权限、属主、符号链接拒绝）按 BLOCK 处理，退回双审，不影响登记；历史 outcome 读不了（JSON 损坏、权限）时不沿用，登记与双审照常；以上判断中，补丁身份算不出的旧 run 一律按同补丁处理（没有 `patch_id` 的旧 run 先按其冻结 SHA 现场重算，`patch_id` 用 plumbing `git diff-tree`，不受本地 diff 配置影响）；policy、模型、提示词或 controller 有变；部署准入不通过或镜像不同；找不到可重算补丁身份的原 run；原 run 本身就是沿用来的。

`patch_id` 是 `merge_base..head` 每个改动路径的路径、前后 mode、前后 blob 清单的 sha256，补丁逐字相同时才相等。不用 `git patch-id`，因为它忽略空白。main 只改了 PR 没碰的文件时 `patch_id` 不变；冲突解决，或 main 也改了 PR 碰过的文件，都会让它变化并触发重审。残余风险：同一补丁放到新 base 上语义可能变（例如 main 改了被调用的函数）。重审时模型会读新 base 上的相关源码，沿用时不读。CI 会在新 head 上重跑，但目前 CI 不是 main 的必需检查，失败不拦合入；要让 CI 兜住这一点，需要把它设为必需检查。

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

登记 attempt（以有效的 `target.json` 是否存在为准，不以目录是否存在为准，目录建好但 target.json 未写时中断也要重新准入）之前先做总准入：单份快照需要导入对象未压缩大小、head 文件大小再加 256 MiB 余量，乘以 reviewer 数（目前是 2，按并发同时存在两份计算），与 checkout_root 所在卷的可用空间比较。不够就不登记 attempt、不写任何状态，报出总需求、单份需求和可用字节数。能沿用旧 PASS 时不建快照，跳过准入，并把「登记时判定可沿用」记进 target.json 的 `reuse_planned`；派发时只在它为真时才考虑沿用，届时若已不能沿用就先补做总准入再双审。每位 reviewer 在 prepare 前仍按单份需求复查一次，模型启动前再核对 256 MiB 余量，防止排队期间空间被占用；复查不通过时明确失败，不启动模型。

宿主先保存 `prepare.json`，再构造 checkout。prepare 中断且尚无 launch 时，同键重跑只回收本 run 的未启动 checkout 并重建。`launch.json` 之后沿用原收养协议，不自动重复派工。监督在读取 launch、获取锁等启动阶段失败也保存失败终态。已记录的监督进程退出却未写终态时，controller 生成 FAILED 记录并发布 error，不永久停留在 pending。

监督在等待容器创建期间收到 SIGTERM 或 SIGINT 时，创建返回后立即关闭容器并记录 `cancelled`，不执行容器命令或启动模型；模型启动前也会再次检查取消。创建结果不明时保留 `container_closed: false`。

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
