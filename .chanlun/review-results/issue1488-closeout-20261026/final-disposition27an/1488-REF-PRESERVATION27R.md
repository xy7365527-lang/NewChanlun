# #1488 遗留分支已完成具名保全

54 条历史分支已在独立恢复库建立具名引用，其中本地 33 条、远端 21 条。每条引用均已回读为同一完整 commit SHA。未删除任何分支或工位。

完整清单为同目录 `1488-REF-PRESERVATION27R.tsv`。清单逐条记录来源、分支名、完整 tip、旧短 SHA、工位占用、新归档引用和实际回读值。

## 本轮实际读数

源仓为 `/Users/silencehan/Projects/NewChanlun`。恢复库为 `/Volumes/AgentStorage/issue1488-closeout-archive-20261026/restore.git`。

- 源引用观察时间为 `2026-10-26T23:55:30.482244+00:00`。
- 保全与逐条回读完成时间为 `2026-10-26T23:56:27.828730+00:00`。
- 本地完整 tip 取自 `git for-each-ref refs/heads`。
- 远端完整 tip 取自 `git ls-remote --heads origin`，没有用本地远端跟踪缓存代替远端事实。
- 54 条 tip 均与 27Q 清单的短 SHA 前缀一致，未发现变化。
- 54 条对应本地分支的 `worktreepath` 均为空。
- 54 个完整 tip 在独立恢复库中均由 `git cat-file -t` 回读为 `commit`。
- 54 个目标归档引用原先均不存在。本轮用一个 `git update-ref --stdin` 事务的 `create` 指令新增，`start`、`prepare`、`commit` 全部返回 `ok`。
- 54 个新引用逐条经 `git rev-parse --verify '<ref>^{commit}'` 回读，与清单完整 tip 全部相同。

本地引用保存在 `refs/archive/issue1488-branch-local/<完整分支名>`。远端引用保存在 `refs/archive/issue1488-branch-remote/<完整分支名>`。同名本地和远端分支分别保全，不合并它们可能不同的 tip。

两个文书分支 `codex/1488-closeout-evidence-20261026`、`codex/1488-six-disposition-archive-20261027` 不在本轮 54 条中。`cursor/critical-bug-inspection-b449` 也不在本轮范围。

## 验证与恢复入口

保全前恢复库共有 882 个引用，保全后为 936 个。集合比较证明恰新增清单列明的 54 个引用，原 882 个引用及其值逐项未变。重新读取 TSV 得到 54 行，所有 `sha` 与 `readback_sha` 相等。

需要恢复某个分支时，在目标仓执行以下命令，并按 TSV 选择对应引用。例子恢复本地历史分支到一个新分支名，不覆盖现有分支。

```bash
git -C /目标仓 fetch /Volumes/AgentStorage/issue1488-closeout-archive-20261026/restore.git \
  refs/archive/issue1488-branch-local/codex/1488-a2-20261015:refs/heads/recovered/issue1488-a2-20261015
```

本轮未执行恢复 fetch，验证仅直接读取独立恢复库中的真实引用和 commit 对象。没有扫描全部对象闭包，也没有计算内容摘要。

若再次运行保全，先核目标引用是否已等于清单完整 tip。相等时无需写入。不同值必须交主控处理，不覆盖。此次原始检查使用 `show-ref --verify` 查询不存在的引用时返回 128，尚未发生写入；随后改为读取现有引用表，再以事务 `create` 保证不覆盖现有引用。

## 后续处置

本轮没有 tip 变化、工位占用或对象缺失的例外。删除历史分支、文书工位及其分支仍未获本单元授权。本报告完成的是删除前的保全准备，不是 #1488 关票结论。主控可直接消费 TSV 的具名恢复入口，无需重复保全或再查全对象闭包。

Prove It Works 使验证落到实际远端 tip、独立库的 commit 类型与逐条引用回读。Make Operations Idempotent 使保全只创建缺失引用，已有不同值不覆盖。技术写作以恢复操作为组织顺序，完整数值留在 TSV。

throughput checkpoint: n/a, scoped non-destructive ref preservation
