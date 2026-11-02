# #1672 FIN/2 交付证据归档

#1672 的 FIN/2 业务交付（WE7 在四个独立 HFS+ 载体上的 Capture/Resolve/Apply、四域恢复、21 个原信封重送）已经以 PR #1702 squash 进入 main，落地提交为 `b9a84b01467772d847f9d222207a305136761bf4`。本目录收齐关票所需的评审链、业务验收报告和两次真实 macOS 运行的原件：一次是候选上的验收运行，一次是 main 落地后的首次运行。两次运行的业务结果与存储读数逐项一致。

报告原件逐字复制，未改历史时态；它们写成时仍"在途"的事项，以本 README 的当前状态为准。

## 版本身份

同一份 FIN/2 交付先后有四个提交身份，证据各自绑定其中一个。

| 身份 | 提交 | 关系 | 绑定的证据 |
|---|---|---|---|
| 验收运行源码 | `9e77756b5038007d081962875d948511284bcef3` | 集成 main `499841b5` 后的候选 | run 40389540213 的 artifact |
| 最终候选 | `0932d3dbcdb18d138ce1e67ed05f971d87e75e3b` | 对 9e777 只做 rustfmt | PR CI 40390534105、DevSkim 40390534300 均 success |
| PR 合入 head | `e7a9668ad1924f1e8cc65896610aed6c65b09259` | 0932 与 main `cf72a9f9` 的无冲突 merge | 集成独审 27Q、远端 Linux 290 passed / 0 failed / 18 ignored |
| main 落地 | `b9a84b01467772d847f9d222207a305136761bf4` | e7a966 的 squash，`rust/` 树与 e7a966 相同 | main CI macOS 40428705046 |

9e777→0932 的复用依据是 `run-40389540213/final-format.diff` 与 `reports/1672-INTEGRATED-INDEPENDENT27.md` 的逐 token 比对。e7a966→b9a84b01 的依据是 `reports/1702-1744-MERGE-RECEIPT27T.md`：先合的 PR #1744 与本交付文件集无交集，合后 `rust/` 树逐字相同。

## 已验结论

**业务验收。** run [40389540213](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40389540213) 与 main run [40428705046](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40428705046) 都实际执行了两个 macOS ignored 测试，结果 `2 passed; 0 failed; 0 ignored`。响应为 6 个 Capture `Recorded`、3 个 Resolve `Resolved`、12 个 Apply `Applied`。三本 B 最终数量 `[2,5,5]`，K `[204,506,506]`，与 HAND oracle 第 3 报逐值相同。四条 `Recovered`，21 对重送请求逐值相同，响应只有顶层 epoch 从 1 变 2。四个最终库 `content_staging=0`，没有第二 application。回收记录 `command_exit=0`、`cleanup_failed=0`、`root_exists=no`。

**容量。** 两次运行四卷实际容量均为 `33,513,472 B`，`carrier_page_limit=1016`，`storage.jsonl` 各 75 行。最大观测 page_count 为 E 209、N1 584、N2 605、N3 622；最小 cleanup 余量 `21,647,744 B`，最小 next-write 余量 `13,144,832 B`。main 运行的读数与候选运行逐项相同，因此 27Q 中"去重不改变持久字节"的推断在 main 上有了实测读数。32 MiB 是用户批准的有限 TestOnly 规格（`reports/1672-SPEC-GATE26BK.md`），这些是观测点读数，不是瞬态峰值保证。

**评审链。** 容量 spec 独审 PASS（26BK）→ 32 MiB 首轮 df02 BLOCK（编译错误）→ 集成候选 9e777/0932 代码 PASS（27）→ PR head e7a966 集成增量静态 PASS（27Q）。代码作者为 GPT 家族，三份代码独审均为 Claude 家族；容量提案作者为 Opus，其独审为 GPT。

**main CI（核于 2026-10-27T01:28:32Z）。**

| run | 结论 |
|---|---|
| CI macOS [40428705046](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40428705046) | completed/success。`macos-session`（含 FIN/2 与原件归档步）、`issue1716-real-s`、`issue1717-real-s` 均 success |
| DevSkim [40428705100](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40428705100) | completed/success |
| CI [40428705199](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40428705199) | 未终态。已完成作业全部 success；`rust-test-s-session (shard 0)`、`extras (native)` 运行中，`extras (owned)` 排队，`macos-session` 按 push 条件 skipped |

## 目录

| 路径 | 内容 |
|---|---|
| `reports/1672-CAPACITY-PROPOSAL-ADOPTED.md` | 被采纳的 16→32 MiB 原提案，原名 `CAPACITY-PROPOSAL.md` |
| `reports/1672-CAPACITY-REVIEW26BK.md`、`reports/1672-SPEC-GATE26BK.md` | 容量 spec 独审与提交用户的批准门文本 |
| `reports/1672-32M-IMPLEMENTATION27.md` | 实施 owner 的交付说明 |
| `reports/1672-32M-INDEPENDENT27.md` | df02 首轮独审（BLOCK） |
| `reports/1672-INTEGRATED-INDEPENDENT27.md` | 9e777/0932 代码独审（PASS）与纯格式复用裁定 |
| `reports/1672-RUNTIME-ACCEPTANCE27.md` | 业务验收报告 |
| `reports/1672-BUSINESS-ACCEPTANCE-COMMENT27G.md`、`reports/1672-FINAL-CI27G.md` | 业务验收评论稿、0932 常规 CI 终态记录 |
| `reports/1702-MAIN-INTEGRATION27P.md`、`reports/1702-INTEGRATION-INDEPENDENT27Q.md` | e7a966 集成说明与集成独审 |
| `reports/1702-1744-MERGE-RECEIPT27T.md` | 合入收据 |
| `reports/1672-CONSUMPTION-RECONCILE27U.md` | 自动关票的来源、重开依据与 #1406 承接 |
| `run-40389540213/artifact/` | 候选验收运行 artifact `12682906680` 全部 19 个文件，含四个最终 `owner.db` |
| `run-40389540213/verify_artifact.py`、`verified-summary.json` | 只读核验脚本及其原输出 |
| `run-40389540213/final-format.diff`、`macos-session.log`、`ci-finalhead-terminal.json`、`devskim-finalhead.json` | 9e777→0932 映射、FIN/2 所在 job 日志、0932 的 CI 与 DevSkim 终态 |
| `integration-1702/economic-session-linux.log` | e7a966 远端 Linux `economic_session::` 测试完整输出 |
| `main-b9a84b01-run-40428705046/artifact/` | main 运行 artifact `12701422272` 的 15 个文本文件 |
| `main-b9a84b01-run-40428705046/run.json`、`verified-summary.json` | main run 的 jobs 回读与核验输出 |

报告原文里的 `1672-32M-evidence27/final-run/...` 对应本目录 `run-40389540213/...`，`1702-integration27p-evidence/economic-session-linux.log` 对应 `integration-1702/economic-session-linux.log`。

## 复核方法

在本目录执行 `python3 run-40389540213/verify_artifact.py`，输出与 `run-40389540213/verified-summary.json` 逐字节相同。脚本以 `mode=ro&immutable=1` 打开数据库，不改原件。

main 运行使用同一脚本，只把耗时字面量 `20.75s` 换成该次实际的 `24.79s`。脚本按 `Path(__file__).parent / 'artifact'` 定位原件，main artifact 的四个 `owner.db` 未入仓，复核需先取回 artifact `12701422272`（`gh run download 40428705046 -n issue1672-fin2-b9a84b01467772d847f9d222207a305136761bf4-1 -D <dir>/artifact`），再把替换后的脚本放在 `<dir>` 下执行。输出即 `main-b9a84b01-run-40428705046/verified-summary.json`。

## 未入仓的原件

以下原件保留在 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1672-32M-evidence27/` 与 `/Volumes/AgentStorage/issue1672-evidence-archive-20261027/main-artifact/`，按需取用：main 运行四个 `owner.db`（约 8.3 MB，与候选运行的库重复承担同一类证据）、`final-run/macos-run-complete.log`、`final-run/issue1716-real-s-failed.log`（属 #1716 线）、df02 与 1bbd 两次失败运行的 artifact、nc-remote 日志。

## 尚欠事项

- 常规 CI 40428705199 终态。失败则交原 owner 返修，不重跑通过的作业。
- 工位 `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree` 的处置。
- 本目录合入 main 后，按 `docs/agents/delivery-discipline.md` 模板发布关票声明，报告路径引本目录。
- 27 独审的两条 LOW 已移交图外 debt 票：`content_store.rs` 过期注释 "16 MiB HFS trial" 归 [#1749](https://github.com/xy7365527-lang/NewChanlun/issues/1749)，卷回收失败时 dmg 不上传归 [#1750](https://github.com/xy7365527-lang/NewChanlun/issues/1750)，移交说明见 [#1672 评论 6431666497](https://github.com/xy7365527-lang/NewChanlun/issues/1672#issuecomment-6431666497)。两票不阻本票关闭。

## 下游消费范围

#1406 已在 [评论 6431366625](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6431366625) 接收本交付，作为 AC1（实际 Capture/Resolve/Apply 有限基线）与 AC5（独立账簿 oracle、四域恢复、原信封重送对拍）的首个实例。它不覆盖 AC4 的正式会话入口与浏览器核对，也不覆盖真实 S 全链（#1673），不能单独据此勾选 #1406 的验收项，更不代表 #1323 整图任何一项 Destination 完成。
