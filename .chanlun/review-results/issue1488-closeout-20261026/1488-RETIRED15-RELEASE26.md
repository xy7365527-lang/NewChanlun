# #1488 退役 15 工位释放报告（2026-10-26）

## 结果

授权 15 项全部由非 force `git worktree remove` 成功释放。所有路径已不存在、注册已移除；原 branch 保留（原本 detached 的保持 detached 记载），精确 stash 对象均钉到 archive ref，独立归档保留。未修改源码/main/总账或关闭票。

## 验证

每项删除前核 HEAD、staged/unstaged patch（`--binary --full-index`）、untracked 清单与原件字节一致，lsof 无打开文件；stash 后核工作区干净及总 tracked patch 一致。删除后再次逐字核 stash 的 staged、unstaged 及 untracked 内容与独立归档一致，15/15 通过。

初次检查 patch 缩写 object ID 与归档 full-index 不一致；改用归档参数后均通过。十个 detached 工位初次 branch 元数据查询停止，确认 `HEAD is not a symbolic ref` 后记录 DETACHED 并恢复同一串行流程；无 stash/remove 拒绝，无 force 或清理绕行。

## 逐项恢复引用

| ID | 结果 | stash OID | archive ref | branch |
|---|---|---|---|---|
| 01 | released | `9a95aa87bdaffd8775de7d2131717cd9edce0983` | `refs/archive/issue1488-retired-01` | `codex/1488-generation-raii-20261018` |
| 02 | released | `72270ce517ec0105655ac331bc7be7fb2856fdde` | `refs/archive/issue1488-retired-02` | `codex/1488-resume-20261017` |
| 07 | released | `72843001eca54fca39e1956e05e758e893f22164` | `refs/archive/issue1488-retired-07` | `DETACHED` |
| 09 | released | `003e406b94172b5b30bbf2ed35a854339b876b01` | `refs/archive/issue1488-retired-09` | `DETACHED` |
| 10 | released | `ec77c879d78be6e61aaaf9bf529a708b2de19d32` | `refs/archive/issue1488-retired-10` | `DETACHED` |
| 11 | released | `e616ac3433ec37f50e75f7f9630f90d5b98b2a80` | `refs/archive/issue1488-retired-11` | `DETACHED` |
| 12 | released | `800529a82791596a539540cfe0db579ce6e122b8` | `refs/archive/issue1488-retired-12` | `DETACHED` |
| 13 | released | `955989dfe1b27f5f5155642b7d262156ffc40aa0` | `refs/archive/issue1488-retired-13` | `DETACHED` |
| 14 | released | `f9be96998e7f79a45f00392afca4b6a50b6fff21` | `refs/archive/issue1488-retired-14` | `DETACHED` |
| 16 | released | `58b08ec5b341f76996996a4b9da6921e8cf55879` | `refs/archive/issue1488-retired-16` | `codex/1488-v2-scope-sol-20261018` |
| 17 | released | `63890e847e1349d8f38eba95df562d3864567388` | `refs/archive/issue1488-retired-17` | `DETACHED` |
| 18 | released | `daed3d57d0d5b3bf3fdc8422e8dc140d8c0a1af3` | `refs/archive/issue1488-retired-18` | `DETACHED` |
| 22 | released | `81971a7f073d408ab01c155cc693f2da2df5b49c` | `refs/archive/issue1488-retired-22` | `DETACHED` |
| 41 | released | `369c2b535a53c54ace20f2e014a407a6cebeec61` | `refs/archive/issue1488-retired-41` | `codex/1488-squash-20261021` |
| 53 | released | `f1aca5b3ffaccfb72ca1033d14461258ced4a724` | `refs/archive/issue1488-retired-53` | `codex/1488-construct-incremental-20261014` |

## 恢复方式

除 ID41 外，refs 属 `/Users/silencehan/Projects/NewChanlun/.git`。ID41 属 `/Volumes/AgentStorage/issue1488-ticket-manager-20261018/budget-handoff-fix-v2-checkout/.git`。完整路径/common/执行日志见同目录 `1488-RETIRED15-RELEASE26.json`；初始停止记录保留，按同 ID 最后一条为最终状态。

在新的恢复路径用对应 common 创建 worktree，基线取 `<archive-ref>^1`；随后在恢复工位 `git stash apply --index <archive-ref>`。这会恢复 staged/unstaged/untracked；不 pop、不删除引用。独立恢复源仍为 `/Volumes/AgentStorage/issue1488-closeout-archive-20261026/restore.git`、`heads.bundle` 与 `worktrees/<ID>/` 原件，不依赖 export.git alternates。

ID02 三个 ignored Serena 配置已额外逐字保全在 `/Volumes/AgentStorage/issue1488-closeout-archive-20261026/release26-supplement/02/.serena/`，恢复时按原相对路径复制。未打印配置内容。ID41/53 ignored Rust 构建缓存随非 force worktree remove 正常释放。

## 边界与空间

ID43、五个独立 clone/common 宿主、reference-coding、主仓工作区与 #1527 活工位未处理。未杀进程。

开始 available：系统盘 140 GiB，外盘 376 GiB；结束：系统盘 143 GiB，外盘 396 GiB。机器有并行写入，差额不全归因本操作。

报告及 JSON 为本次 #1488 工作草稿，不改全图总账。
