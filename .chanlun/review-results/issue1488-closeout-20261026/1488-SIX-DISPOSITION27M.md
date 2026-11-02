# #1488 余六旧容器已保全并移除

旧归档剩余的 ID28、29、30、31、32、43 六个容器已在保全后全部移除，完成时钟为 `2026-10-26T22:59:53.770872+00:00`。用户的处置批准为“批准保全后移除这六项”。至此旧归档 42 个容器全部处置完毕，恢复只依赖独立 `restore.git`、原 bundle 与仓外文件副本。

本文与 `six-disposition27m/` 下的小型原件随文档入仓。完整执行原件在仓外 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1488-six-disposition27m-evidence/`，含各项状态快照、引用映射和 44 MB 的对象闭包清单，不入仓。

## 六项终态

路径均相对 `/Volumes/AgentStorage/issue1488-ticket-manager-20261018/`。

|ID|路径|原 HEAD|移除方式|
|---|---|---|---|
|43|`worktree-fix-p2-resume-20261023`|`ddf6c6bfe88f2e752bf940e85d550a31e2c28811`|由 ID29 执行非 force `git worktree remove`，退出 0。|
|29|`budget-handoff-fix-v2-checkout`|`0fb96c00ae94de4db602b465d0e7363a34d835b2`|按批准的精确目录 `shutil.rmtree`，退出 0。|
|30|`budget-handoff-review-checkout`|`959d86006584b8bf553fe61bdd15197b0e21d6ba`|同上。|
|32|`coverage-checkout`|`654a7f7a66fae4bb15cf8b5b5dc87afc23f33392`|同上。|
|28|`budget-handoff-checkout`|`813e1f92b3da7cf3780ab4886365d2fb2c6bf933`|同上。|
|31|`checkout`|`654a7f7a66fae4bb15cf8b5b5dc87afc23f33392`|同上。|

表序即执行顺序：先退役 ID43，再按对象依赖由叶到根移除五个独立 clone。六路径的事后存在性见 [six-terminal.json](six-disposition27m/six-terminal.json)，五个 clone 的逐项结果见 [removal-actions.jsonl](six-disposition27m/removal-actions.jsonl)，ID43 的 remove 命令与退出码见 [commands.jsonl](six-disposition27m/commands.jsonl)。执行中没有使用 force、clean、reset 或全局 prune。

执行前全机 `lsof` 对六路径无匹配，此前持有 ID43 脚本句柄的 PID21953 已随其原线程归档退出。五个 clone 无新 WIP；ID43 仍只有三个未跟踪文件和 25 个 ignored 文件，其中七个 `.bak` 备份需保全，其余为 pytest 与 Python 缓存。

## 恢复入口

独立恢复库为 `/Volumes/AgentStorage/issue1488-closeout-archive-20261026/restore.git`，没有 alternates。

- 五个 clone 的全部当前引用按原名导入 `refs/archive/issue1488-clone-<ID>/`。ID28、29、30、31、32 分别为 10、201、10、609、9 个，共 839 个。
- ID41 的 stash `369c2b535a53c54ace20f2e014a407a6cebeec61` 位于 clone29 命名空间内，源库移除后仍可直接读取。
- ID43 退役 stash 为 `025965aa29af6559a8abc773c84699894cfc08cc`，引用 `refs/archive/issue1488-retired-43-27m`。两份 stash 见 [retired-stashes.json](six-disposition27m/retired-stashes.json)。
- 840 个根引用覆盖 346,736 个可达 Git 对象，`git rev-list --objects --missing=print` 退出 0，无缺失，见 [all-closure-summary.json](six-disposition27m/all-closure-summary.json)。

源容器删除后做了实际恢复：从 ID43 stash 的第三父对象取回三个未跟踪文件，逐字节与原件相同；ID41 commit 可读；恢复库无 alternates，见 [postremove-recovery.json](six-disposition27m/postremove-recovery.json)。三个未跟踪文件和七个 `.bak` 备份与归档 `worktrees/43/files/` 副本的逐文件比对见 [43-assets-compared.json](six-disposition27m/43-assets-compared.json)。

恢复 ID43 原 HEAD 及其未提交内容：

```sh
A=/Volumes/AgentStorage/issue1488-closeout-archive-20261026
git init /Volumes/AgentStorage/issue1488-recovered-43
git -C /Volumes/AgentStorage/issue1488-recovered-43 fetch --no-tags "$A/restore.git" refs/archive/issue1488-retired-43-27m:refs/archive/retired-43
GIT_LFS_SKIP_SMUDGE=1 git -C /Volumes/AgentStorage/issue1488-recovered-43 checkout --detach refs/archive/retired-43^1
git -C /Volumes/AgentStorage/issue1488-recovered-43 stash apply refs/archive/retired-43
```

恢复其他 clone 时，从对应 `refs/archive/issue1488-clone-<ID>/` 命名空间 fetch。LFS 实体按归档 `lfs-supplement/README.md` 恢复，跳过 smudge 得到的只是指针。

## 恢复范围的两处限制

已知十个历史 `relations.jsonl` LFS 版本在原源库本就缺失。Git 对象闭包完整不代表这十个 LFS 文件可恢复；归档 HEAD 所需的两件 LFS 实体仍由 `lfs-supplement/` 覆盖。

`export.git` 仍有指向已删 clone 的 alternates，已不可用，不能作为恢复源。

## 空间读数

外盘可用空间从约 355 GiB 变为约 364 GiB，系统卷保持约 130 GiB。APFS 共享块和同时运行的其他工作都会影响该读数，它不是本次精确净回收量。五个 clone 删除前逻辑大小合计 8,398,140 KiB。

## 对 #1488 收尾的影响

旧归档 42 项已无剩余容器。#1488 仍需本文档候选经独审和 main 批准合入，再处置本文档工位与相关分支、发布 resolution 后关票。本次处置没有关闭 #1488，也没有放行 #1491、#1640 等下游。
