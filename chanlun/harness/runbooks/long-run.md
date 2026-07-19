# Runbook：长任务启动与守护（R1–R5）

## 动机证据（本会话实测缺口，2026-07-18/19）
- 事故一：Candidate A 全量重放在 ~3.5M bar 处静默死亡，无 EXIT 标记、无 rc、无 rusage，死亡在数十分钟后才被人工轮询发现。
- 事故二：C-7 归并链以会话子进程方式后台运行，用户中断会话工具时被连带 SIGTERM（exit 143），归并停在 2.5M/4.6M（`P124M_TERMINAL_PROGRESS bar=2500000/4613599`），同样无落标。
- 事故三：两个会话在同一 worktree（/tmp/kimi-nest-mainline）各自拉起归并脚本（p124_r2_merge_fix.sh 与 p124_r2_merge_and_check.sh），险些双写同一输出。

## 规则
- **R1 解耦启动**：长任务一律 `nohup … >/tmp/<job>.out 2>&1 & disown`，与会话/工具生命周期脱钩；会话被打断不得影响任务。**Darwin 无 `setsid`**（2026-07-19 实测：`nohup: setsid: No such file or directory`，任务静默未起）；Linux 上可加 `setsid` 增强脱钩。起后必须验活：存 `$!` 到 `/tmp/<job>.pid`，sleep 数秒后 `ps -p $(cat /tmp/<job>.pid)` 并查 out 头部有无启动报错。
- **R2 落标**：脚本最后一行必须 `echo "<JOB>_EXIT rc=$rc"`；主命令用 `/usr/bin/time -l`（darwin）包裹以留 rusage。
- **R3 心跳**：进度标记（例：`P124M_TERMINAL_PROGRESS bar=N/M elapsed=…`）≥ 每 60s 一行。判活标准 = 心跳在走，不是进程在表。
- **R4 轮询取证**：监视者只读 `/tmp/<job>.out` + `ps`。心跳停走且进程消失 → 先取证（最后进度行 / EXIT rc / 系统日志），再决定重跑；禁止无取证盲重跑。
- **R5 互斥预检**：启动前 `pgrep -f <二进制名>`；已有同类进程在跑 → 不得重复拉起（双写同一输出文件即撞车）。输出文件命名含 job 名，不共用。**注意自匹配假阳性**：检查命令自身 shell 的 cmdline 含模式串时 `pgrep -f` 会误报（2026-07-19 实测），用字符类破坏字面（如 `pgrep -f "p124_[m]erge"`）或改用存 PID + `ps -p`。

## 启动模板
```bash
JOB=p124_merge_r3
pgrep -f p124_merge && { echo "R5: 已有实例在跑，放弃"; exit 1; }
nohup setsid bash -c '
  /usr/bin/time -l ./target/release/p124_merge <args> 
  rc=$?; echo "'"$JOB"'_EXIT rc=$rc"
' >/tmp/$JOB.out 2>&1 & disown
echo "launched $JOB pid=$!"
```
