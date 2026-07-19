# Harness Ledger — harness-engineering 实装台账（2026-07-19）

方法论：lopopolo/harness-engineering。每环 = 观测缺口 → 干预（尽量可执行约束）→ 验证判据。散文不算干预。

| 环 | 观测缺口（证据） | 干预 | 验证判据 | 状态 |
|---|---|---|---|---|
| 1 | 明文 cargo 禁令被无视，发生越权重建 | 授权门：rust/.cargo/config.toml → cargo_gate.sh + 哨兵 `.chanlun/locks/CARGO_BAN`（任务 #114） | 无哨兵放行时 `cargo build` 被拒 | ✅ 已装已验；哨兵按门之本意提前退休 @2026-07-19（保护对象消失：归并落线 + C-3 禁入 worktree；修订依据留痕 CARGO_BAN.retired-20260719，非 rm） |
| 2 | 长任务两次静默死亡（Candidate A ~3.5M；C-7 被中断连带 SIGTERM，exit 143，停于 2.5M/4.6M） | `runbooks/long-run.md` R1–R5（setsid 解耦 / EXIT 落标 / 心跳判活 / 取证后重跑 / 互斥预检） | 下一个长任务按模板启动；人为中断会话后任务仍在跑且有心跳 | ✅ 文档+模板已装，待下次长任务验证 |
| 3 | 两会话同 worktree 双拉归并险撞车（用户被迫人工中断） | WORKTREE_OWNER 互斥约定 + 进场预检命令（WORKERS.md §2）+ R5 pgrep 预检 | 第二写入者进场前读到 OWNER 并让位/换树 | ✅ 已装已验；并发场景首次实证 @2026-07-19：C-4（#111）首派进场读到 OWNER 让位退场，互斥正常触发（证据：11 关矩阵 mainline-11guan-acceptance-matrix-20260719.md 备注行） |
| 4 | 每次派发都手工重述接线/边界，易漏易漂 | `WORKERS.md` 单一事实源；派发词瘦身为指针+专属段 | C-3 及以后派发词均以指针形式发出 | ✅ 已装，C-3 起生效 |
| 5 | 方法论原文不可本地检索，工作者只能读转述（且首次 vendor 因 cd 失败把主仓 md 灌入 vendor，10G，已隔离） | 上游整仓 49 个 md + LICENSE（CC-BY-4.0）vendor 至 `vendor/harness-engineering/`，PROVENANCE 记 commit 226c8d3 | WORKERS.md §5 指针可达，原文可 grep | ✅ 已装 |
| 6 | R1 模板在 Darwin 不可执行：`setsid` 不存在，C-3 派发静默失败（out 仅一行 `nohup: setsid: No such file or directory`）；且验活 `pgrep -f "codex exec"` 匹配到检查命令自身 shell，假阳性掩盖失败 | R1 改 `nohup … & disown`（Darwin 无 setsid）+ 起后强制验活（存 `$!` → `ps -p`）；R5 验活禁用裸 `pgrep -f`，用字符类模式（如 `code[x]`）或存 PID | 重派后 `ps -p 91981` 存活 + codex 本体可见（session 019f795e） | ✅ 已验（本环即实证） |

## 备注
- 本仓 docs 区（chanlun/）为约定落点；旧 worktree /tmp/kimi-nest-mainline 当前由另一会话独占（归并 PID 72171），本会话不再进入。
- 后续新环继续追加本表，不另开文件。
- 环5 事故收尾：污染拷贝（10G）经隔离后已于 2026-07-19 获用户放行删除，闭环。
- 归并会话（PID 72171）已核实退出 @2026-07-19；worktree 遗留改动只读盘点在档 `chanlun/review-results/worktree-leftover-inventory-20260719.md`，处置权留用户（重新进场须按 WORKERS.md §2 预检）。
