# #1488 收尾证据与剩余容器

本报告为 #1488 绑定的工作草稿，供主控核关票门。2026-10-26 已释放旧归档 42 个容器中的 36 个，剩余 6 个保留。代码已合入 main，本票尚不能关闭。

## 已交付实现与主线证据

[PR #1711](https://github.com/xy7365527-lang/NewChanlun/pull/1711) 的候选为 `ddf6c6bfe88f2e752bf940e85d550a31e2c28811`，squash 落地为 `8bbe4499d983e674d435261b207a6556715600b4`。合入事实见 [#1488 评论](https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6403234483)。

|证据|结果与采用范围|
|---|---|
|[候选 CI 40210836834](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40210836834)|成功，覆盖合入前精确候选。|
|[落地 main CI 40233542465](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40233542465)|cancelled。extras job `121290180450` 达到 60 分钟上限，不能记为成功。|
|[后继 main CI 40233642065](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40233642065)|`025d2eb5a717811cd9f8b3bcaa35a19da7478ba0` 成功。补足原 run 未完成的 Owned history 末步。|

原 extras 的 Native 测试为 384 passed、0 failed、9 ignored；随后编译 Owned history 时耗尽 job 时限，末步没有测试结果。后继 extras job `121290473039` 的 Owned history 为 35 passed、0 failed、1 ignored、699 filtered out。两者末步命令相同：

```sh
cargo test --locked --features s_session,s_allocation_json_testonly --bin s_structure_session v3::history --jobs 1
```

主控采用前，已核 `8bbe4499` 是 `025d2eb5` 的祖先，且两提交间 `.github/`、`rust/Cargo.toml`、`rust/Cargo.lock`、`rust/src/bin/s_structure_session*`、`rust/src/s_content*` 以及 economic_session、session_transport、session_protocol 目录均无差异。后继提交改动了其他交易消费者，不声称整个 crate 字节相同。

该末步补证已由主控采纳，见 [进度评论](https://github.com/xy7365527-lang/NewChanlun/issues/1488#issuecomment-6404778357)。PR #1723 独立承接 CI 调度改进，不再作为 #1488 此项补证的前置。本次仅整理已有结果，没有重跑历史整套。

## 已释放 36 个旧容器，内容仍可恢复

以下两份执行报告及验收字段 JSON 已随本文入版本控制。干净批次 JSON 仅删除 ignored 文件清单，保留 HEAD、状态、移除结果、后验及环境核查字段；退役批次 JSON 保持原件。报告中的“仍保留”段是各次执行结束时的快照，当前剩余范围以下一节为准。

|批次|归档 ID|实体报告|
|---|---|---|
|21 个干净 linked 工位|05、06、08、15、19、20、21、23、24、25、26、27、33、34、35、36、37、38、39、40、42|[释放报告](1488-CLEAN-LINKED-RELEASE26.md)、[逐项结果](1488-CLEAN-LINKED-RELEASE26.json)|
|15 个封存退役 WIP 工位|01、02、07、09、10、11、12、13、14、16、17、18、22、41、53|[释放与恢复报告](1488-RETIRED15-RELEASE26.md)、[逐项结果](1488-RETIRED15-RELEASE26.json)|

两批均使用非 force `git worktree remove`。第一批后验为 21 个目录及注册均已移除，归档引用仍可解析。第二批删除后，stash 中 staged、unstaged、untracked 内容与独立归档逐字核验 15/15 通过。JSON 中同 ID 的初次停止记录不是最终失败，按该 ID 最后一条记录读取结果。

主控已判定这 15 份历史 WIP 由最终 main 承接或修正，封存退役，不再向 #1527 或其他下游重放。原 branch 与精确 `refs/archive/issue1488-retired-<ID>` 保留。分支仍需在最终关票时逐项处置，不能把工位目录移除当成分支已删除。

恢复原件位于 `/Volumes/AgentStorage/issue1488-closeout-archive-20261026/`。`restore.git` 与 `heads.bundle` 为独立恢复来源，`worktrees/<ID>/` 保存差异和额外原件。`export.git` 依赖源 alternates，只是归档制作中间件，不作为独立恢复入口。

恢复补充已覆盖 42 个归档 HEAD 对应的 31 个不同 HEAD 所需的两件 LFS 实体，合计 304,471,992 字节。两次新 probe.git 的 `git lfs smudge` 均退出 0，输出与源逐字相同，细节留在归档 `lfs-supplement/verification.json`。可达历史共有 12 个不同 LFS 指针，另 10 个旧 `relations.jsonl` 版本在六个源库也不存在。因此恢复覆盖的是这些归档 HEAD，不是任意历史提交。

ID02 的三个 ignored Serena 配置另存于归档 `release26-supplement/02/.serena/`，本报告不包含配置内容。恢复时仍需这些仓外原件。本次入仓的是报告，不是把恢复数据或备份复制进仓库。

## 旧归档剩余 6 个容器及解除条件

以下路径均相对 `/Volumes/AgentStorage/issue1488-ticket-manager-20261018/`。本次整理时再次核到 ID29 的 Git 注册只含自身及 ID43；PID21953 仍持有 ID43 脚本的两个句柄。

|ID|路径|当前依赖与解除条件|
|---|---|---|
|43|`worktree-fix-p2-resume-20261023`|Desktop Commander MCP 的 PID21953 打开 `.github/scripts/run-1488-driver-ci.py`。服务正常释放句柄后，主控核无在飞 owner、未跟踪内容仍已保全，才能处置此 linked 工位。|
|29|`budget-handoff-fix-v2-checkout`|ID43 的 common dir；对象来自 ID28。ID43 释放且 `refs/archive/issue1488-retired-41` 的恢复需要妥善保全后才能处置。|
|30|`budget-handoff-review-checkout`|对象来自 ID28。独立 clone 需由主控按恢复与消费者核查结果处置，不能用 `worktree remove` 删除其主工作树。|
|32|`coverage-checkout`|对象来自 ID31。与 ID30 同样按独立 clone 处置。|
|28|`budget-handoff-checkout`|对象来自 ID31，向 ID29 和 ID30 提供对象。逆依赖消除后才可处置。|
|31|`checkout`|对象来自主仓，向 ID28 和 ID32 提供对象。逆依赖消除后才可处置；主仓 `.git` 不在清理范围。|

这些依赖允许先处置已满足条件的 ID30 或 ID32，但不允许删除 ID43 所需的 29、28、31 链。主控仍保留全部六项，未授权本次文档任务执行删除。不得为了关票停止共享 MCP 或重启 Codex。

六个 reference-coding 工位属于参考索引用途，排除在这 42 项之外。它们不因 #1488 收尾而删除。父目录的 `workers/` 与验收包也不在容器清理范围。

## 主控关票前仍需完成

1. 核实际消费者解除，按对象依赖完成剩余六个旧容器及原 branch 的处置，保留独立恢复原件。
2. 独立评审本文档候选，取得 main 批准后合入。本次纯文档提交豁免编译，不替代已有实现验证。
3. 处置本次新增文档工位及分支。它不在旧归档 42 项内，登记于 [本次 roster](../../agent-roster-1488-closeout-doc-20261026.md)。
4. 依 `docs/agents/delivery-discipline.md` 发布真实 resolution，引用实现落地 SHA、文档落地 SHA、上述仓内报告路径与主线 CI 事实。

在这些条件成立前，#1488 保持 open，#1491 和 #1640 的原生依赖不手动放行。#1279 保持 open，六项 Destination 验收范围不变。
