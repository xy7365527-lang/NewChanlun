# 冻结旧入口并让在飞任务收尾

此页属于 #1742 迁移准备。代码默认不冻结。不要在生产切换 D6 批准前运行下面的 freeze 或修改仓库变量。

## 切换前部署

所有本机旧入口共用 `/Volumes/AgentStorage/newchanlun-legacy-drain`。路径不按 worktree 或环境变量分叉。CLI 的 `--root` 只用于隔离测试，部署命令不传它。

部署必须覆盖实际宿主与所有手工入口的这份代码，并在旧 owner 可核的轮次边界换进程。已加载的旧无限循环不会因磁盘替换而自动执行新门。不能直接卸载常驻父服务，否则可能杀掉在飞子进程。先记录现役版本、plist、进程父子关系和每票 checkpoint；让旧一轮自然完成后再由 owner 安排加载具备门禁的新版本。

远端 `agent-*` 工作流还需要先将本次变量门发布到默认分支。发布这些代码不等于启用冻结。D6 期间设置仓库变量 `SANDCASTLE_LEGACY_FREEZE=1` 后，五个工作流不接受新事件。变量不能撤销已排队、已评估或在运行的 job，必须列出并等它们自然完成；不得凭本地状态宣称远端已排空。

## 冻结与确认

从部署检出运行：

```sh
python3 .sandcastle/drain-control.py freeze
python3 .sandcastle/drain-control.py status
```

`freeze` 返回 `mode=draining` 及 generation，重复执行保持同一代次。冻结和准入用同一个系统 `fcntl.flock`。开放准入持共享锁，不互相串行。freeze 和 resume 持排他锁。freeze 成功返回时，先前的“选择到首次副作用”操作已完成，后来的新操作会拒绝。它不杀已认领工蜂。

- main 的选票与 claim 在锁内；REVIEW_ONLY 的沙盒创建同样须准入。
- claim-loop 只在锁内拉起新 main，并在已拉起的子进程结束后停止下一轮。冻结期间不回滚到旧队列。
- decision-claimer 每票首次 claim 在锁内，已领取的票继续完成；下轮退出。
- swarm planner 可以自然结束，但尚未 claim 的计划在冻结后不能认领；已 claim 批次完成作者、独审和归并，不进入下一轮。
- engine 的每个实际图动作及放行都在锁内；freeze 会等待已经开始的动作结束。大起草动作可能延长等待，不用超时杀进程冒充排空。
- health-monitor 保留只读检查，但不能 heal、kickstart、bootstrap 或回滚认领。其变更动作也使用同一准入锁，避免观察到 open 后冻结才生效的竞态。

`harvest.sh` 不承担新增领票。当前策略允许它在排空期间完成已经过审旧分支的 PR 交付，不改它的既有判据。切流前必须等当前 harvest 单轮结束，再停它的定时入口，记录全部已交付 PR。它仍不是新执行器精确 head 发布器。

取得 freeze 确认后核对所有本机控制进程、工蜂、远端 job、workers.jsonl 尾部、decision summary 和分支归属。没有完整清单，不算排空。

单独列出 workers.jsonl 中有 `phase:claim` 却没有 implementer started 的票，并对照 GitHub 当前认领、标签与本地 checkpoint。认领后、启动前失败的票在冻结下不会自动回队列，不能把“没有工蜂进程”计为完成。逐票记录旧 owner、最后一个已证实步骤、分支和未执行副作用。由旧 owner 在批准范围内选择从原 checkpoint 收尾，或保留冻结认领并登记给新执行器的交接方案。需要释放认领或重新排队时，先核该动作属于 D6 批准范围，再按交接方案执行。全部遗留票有去向及接收记录后，才能确认排空。

冻结后循环正常退出，但 launchd 的 `KeepAlive=true` 可能继续节流重拉，形成启动后立即退出的日志噪音。门禁仍拒绝新选择，这些启动记录不算新增工蜂。待各循环达到安全边界、子进程已完成并且 inventory 已对账，再由部署 owner bootout 对应 plist，不能在子进程仍在飞时卸载父服务。核对 health-monitor 的 KeepAlive 和定时任务，保持 heal 禁用直到恢复步骤最后。

未知版本、坏 JSON、读取失败或错误 generation 都失败关闭。控制进程崩溃后 helper 的 stdin 关闭，内核释放 flock，不靠 PID 年龄猜锁是否陈旧。持久状态保持 draining，不自动恢复 open。

## 恢复旧入口

先确认新执行器没有活跃副作用。新执行器已持有的票必须完成或正式释放，不得直接重贴旧标签。恢复旧代码和 plist后，使用冻结时记录的同代次解冻：

```sh
python3 .sandcastle/drain-control.py resume --generation 1
```

把示例 `1` 换成当前冻结代次。旧代次不能解开后来的新冻结。恢复远端变量前核已排队 job。回退恢复清单包含 sandcastle-claimer、sandcastle-harvest、wayfinder-engine、decision-claimer 与 `com.newchanlun.arch-hotzone`，最后才恢复 health-monitor 自愈，避免它提前拉起旧执行链。

## 离线验证

```sh
node --import tsx --test .sandcastle/drain-control.test.mts
```

测试仅用临时根，覆盖 freeze/claim 竞争、已准入收尾、冻结后无新增副作用、半写状态、代次恢复、并发开放准入及真实 SIGKILL 后内核释放锁。decision-claimer 相邻测试用独立临时根，覆盖 T 类断点、P 类 pDone 在冻结下保留，以及解冻后不重复评论或提交。真实部署、旧消费者排空和远端变量启用仍未执行。

## 将引擎放行交给新执行器

`bash .sandcastle/cutover-runbook.sh` 只打印命令和核对点，不冻结、不写 GitHub、不卸载服务。参数依次为部署检出、venv、App 私钥、Boat 配置和 Boat 控制端 key 文件的绝对路径。预览中的执行器命令显式携带 `PILOT_BOAT_CONFIG` 与 `PILOT_BOAT_API_KEY_FILE`。例如：

```sh
bash .sandcastle/cutover-runbook.sh "$REPO" "$VENV" "$PEM" "$PILOT_BOAT_CONFIG" "$PILOT_BOAT_API_KEY_FILE"
```

D6 获批后由部署 owner 逐步执行，不把打印成功算成部署。

在 freeze 成功、仓库变量已设为 1 后，先卸载 health-monitor，防止周期自愈重新拉起旧 claimer；回退时最后恢复它。已在飞的 health-monitor 与其他旧控制进程一起进入快照，必须等它退出。执行器的 `cutover snapshot` 记录该代次、存活旧进程的 pid 加启动标识，以及 workers 日志的规范化绝对路径、字节位置和前缀内容身份。verify 拒绝换源、截断或同长度改写；既有未带前缀身份的快照仍可用于原 verify 入口，但不能用于审计释放。快照文件用 `O_EXCL` 创建，不覆盖已有快照。等待这些进程退出和旧工作收尾，再在安全轮次边界 bootout claimer、harvest、旧 engine、decision-claimer 及 `com.newchanlun.arch-hotzone`。arch-hotzone 虽不写 workers 日志，但在旧宿主目录常驻，也必须在审计释放和 verify 前停止。不要直接卸载仍带在飞子进程的父服务。

检查命令须从旧宿主树外启动；使用绝对模块、配置与日志路径。先把控制端 shell 移到 `/`，不能仅在子进程内换 cwd 后留下宿主目录中的父 shell。检查器在宿主树内启动会明确拒绝；它只排除自身 PID 和采样子进程，不忽略其他 shell。旧宿主内的业务 shell（包括 CI 轮询）由其 owner 离开该目录或结束，并确认后代退出，不由迁移程序替它杀进程。

```sh
REPO=/Users/silencehan/Projects/NewChanlun-sandcastle-host
WORKERS_LOG="$REPO/.sandcastle/logs/workers.jsonl"
cd /
executor cutover snapshot --workers-log "$WORKERS_LOG"
executor cutover verify --workers-log "$WORKERS_LOG" --private-key "$PEM"
executor cutover record --workers-log "$WORKERS_LOG" --private-key "$PEM"
```

`verify` 只读核对冻结代次、快照进程退出且当下无旧写者、旧工作的当前生命周期终态、快照后无新增 claim、App 读取的全部 `agent-*` job 已完成，以及 claimer 和 harvest 的 launchd 服务已卸载。日志按票和 branch 归约：历史重复 claim 不当场永久报错，但仍保留 pending，直到同一生命周期出现 `sandbox/closed` 或 `claim/rolled_back`。独立 reviewer/merger 沿用各自已有的终态。另一 branch 的关闭不能消除本 branch 的未决工作；缺 branch 的历史记录也不猜测归属。

已知两条 `correction/error_record_*` 只保留审计，不打开或关闭工作；`implementer/failed` 和 `implementer/recovery_returned` 都不是排空终态。未知事件、无法读全分页、日志截断、仍未收尾的旧工作都拒绝。快照进程退出与日志终态是两份独立证据；日志不增长、PID 已消失、交接文字或实现完成都不能单独替代生命周期终态。缺失的历史关闭事件由部署 owner 对照原 owner 的运行存储，在生产处置获批后使用下节的审计释放入口；它记录当下处置，不补造历史成功或 sandbox/closed，也不新增逐票真人签批。

### 历史缺关闭事件：审计释放

`cutover pending` 只读导出精确 ticket、branch 和原生命周期引用。首末索引是 **1-based 已解析事件序号**，不是时间排序或物理行号；空 branch 不并入标准分支。

```sh
executor cutover pending --workers-log "$WORKERS_LOG" > "$PENDING"
```

生产写入获批且已取得本代次冻结快照后，选一个已核实的生命周期，保存请求。下面示例针对 #1240；三个位置参数依次是 pending 清单、新请求路径、已保留的证据原件。摘要仅用于把请求绑定到该证据内容，不把证据里的文字自动当作执行成功。

```sh
python3 - "$PENDING" "$REQUEST" "$EVIDENCE" <<'PY'
import hashlib, json, os, sys
from pathlib import Path
pending, output, evidence = map(Path, sys.argv[1:])
matches = [r for r in json.loads(pending.read_text())
           if r["ticket"] == "1240" and r["branch"] == "sandcastle/issue-1240"]
if len(matches) != 1:
    raise SystemExit("需要唯一的精确生命周期")
request = matches[0] | {
    "v": 1, "disposition": "abandoned_no_exit",
    "reason": "原退出记录缺失；冻结后核无在途，保留工位和原证据，不关闭业务票。",
    "evidence": [{"path": str(evidence.resolve()),
                  "sha256": hashlib.sha256(evidence.read_bytes()).hexdigest()}],
}
fd = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
with os.fdopen(fd, "w") as stream:
    json.dump(request, stream, ensure_ascii=False, indent=2)
    stream.write("\n")
PY
executor cutover audit-release --workers-log "$WORKERS_LOG" \
  --snapshot "$SNAPSHOT" --request "$REQUEST" --private-key "$PEM"
```

处置只有三种：`returned`（有执行返回证据）、`terminated`（有中止证据）、`abandoned_no_exit`（原退出记录缺失）。后两者不是成功，returned 也不是业务验收。原事件不含退出码时，记录中的 `original_exit_code` 保持 null。请求不能指定“进程已清空”、新增退出码或用别的分支代替。

写入口持有与 freeze/resume 相同的排他锁；重新读取远端 job/冻结变量/服务屏障、旧控制进程、完整 PID 表、工位 cwd 句柄和 Docker 活容器。旧写者识别以系统 `lsof` 返回的真实 cwd 为根：规范化后位于固定旧宿主树内的进程，以及 `ps` PPID 关系中的全部后代都纳入，后代换 cwd 也不能漏过。受管 runner 所在目录可随轮次改变，i4/l2 或后续目录无需另列白名单。既有入口命令匹配作为补充，不读取旧 RUNTIME.json 猜存活，也不接受请求提供的 cwd。快照后新出现的旧写者同样阻止 verify/record；PID 被无关进程复用则不属于旧写者。采样命令失败、警告、空输出或无法解析都拒绝。工位从原事件或固定旧执行宿主 `/Users/silencehan/Projects/NewChanlun-sandcastle-host` 的同分支 Git worktree 清单取得；共享日志的符号链接不会改变清单来源。请求不能另选宿主。容器按名称与挂载源关联，不读取 Env。存活、采样错误、证据变化、源日志换 inode/改写/并发追加、冻结代次变化均拒绝。

成功只向原日志追加一条 `reconciliation/released`，包含原首末事件、冻结代次、操作者、现场采样和证据身份。相同请求在同一快照下重放不重复写；已释放后不必再重放。换取新快照后，旧请求会得到 `different prior audit release`，不会改写原收据。任何新生命周期不能沿用旧请求。合法结构的陈旧或不匹配 release 只保留审计、不清除 pending；非法结构仍拒绝整份日志。若旧生产者抢在最后一次检查后追加新事件，写入口报错但保留竞态原件，之后真实关闭或新快照下的精确审计处置仍可继续，不需删行。它不会创建 cutover.json、关闭业务票、删除工位或开启新队列；仍须继续 verify/record 与部署验收。

冻结变量单独通过 `gh variable get SANDCASTLE_LEGACY_FREEZE` 回读。现有 App 没有 variables 权限，因此此项运维证据记为 `user-token-readonly`。它不充当人类批准，也不在 App 失败时接替写操作。读不到或值不是 1 都拒绝。

`record` 重做上述验证后，以 0600 和 `O_EXCL` 创建全局 `/Volumes/AgentStorage/deepagents-exec/cutover.json`。内容相同的重复写入成功，内容不同的记录不覆盖。引擎和执行器都读取这份记录，不增加环境变量切流开关。引擎在同一把 drain 共享锁内改挂 `deepagents:queue`，新拆的实装票只先挂 `ready-for-agent`，放行步骤再挂队列标签。

必须在新 wayfinder-engine 与 deepagents-executor bootstrap 之前完成 verify/record；它们启动后也使用旧宿主 cwd，再跑冻结核查会失败关闭。切流成功后是否恢复 arch-hotzone 由部署 owner 在 record 完成后决定。

启动新 engine 后，先运行 `--once --dry-run` 检查新放行标签。`executor serve --once` 是实际执行入口，不是 dry-run。它会为缺签批的票生成请求，只有有效 SPEC 签批才进行票面写入。写集与签批登记方法见 [新执行器队列操作](../deepagents-pilot/EXECUTOR.md#接收引擎放行队列)。

安装 `.sandcastle/launchd/com.newchanlun.deepagents-executor.plist` 前，把 `__REPO__`、`__VENV__`、`__PRIVATE_KEY_PATH__`、`__BOAT_CONFIG__` 和 `__BOAT_API_KEY_FILE__` 替换为已核对的绝对路径。确认模板的 `PATH` 能找到 Boat CLI。模板只保存配置和密钥文件的路径，不保存密钥内容。由部署 owner 加载服务并核对真实队列交付到精确 `awaiting_merge(h)`，代码和离线夹具不代替这一步。

`drain-control resume --generation <当前代次>` 使既有切流记录立即失效。共享动作锁保证 resume 等当前动作完成再开放旧入口，新执行器的下一动作会停下。先让新执行器到安全边界，再完成或正式释放已持有的票，最后恢复旧服务和 health-monitor 自愈。不要同时启动两套消费者。

切流记录与快照在回退后保留供审计。再次切流前，由部署 owner 将失效的 `cutover.json` 和旧快照移入代次归档目录，再创建新代次记录。程序不自动删除或覆盖旧证据。
