# 1744 与 1702 合入收据（27T）

记录者：land_1702_1744（唯一合入 owner）。时间取 GitHub mergedAt 与 `date -u`。

## 合入
| PR | 精确 head | 合前 main | squash merge SHA | mergedAt |
|---|---|---|---|---|
| #1744 | 11b5597fb581f85706d1f2214aaf08fc01ec8d95 | cf72a9f91050a4b6af7ac4b6e0c2b9f0364a2584 | fcfdb0f71042d7634bb44002d8b915de414ee6e5 | 2026-10-27T00:47:45Z |
| #1702 | e7a9668ad1924f1e8cc65896610aed6c65b09259 | fcfdb0f71042d7634bb44002d8b915de414ee6e5 | b9a84b01467772d847f9d222207a305136761bf4 | 2026-10-27T00:48:16Z |

均用 `gh pr merge --squash --match-head-commit <head>`；未删除分支或工位。

## 授权
- main1744：既有 resolved（含同范围 DevSkim 误报修复）。
- main1702：本轮 resolve，用户明确批准 e7a966 必要 CI 通过且核当前集成后合 main；四个 32 MiB TestOnly 卷与资金/隔离/期限/恢复合同不变，不启用真实交易。

## 合前核验（复用既有证据）
- 1744 精确 head：CI 40419797779 success、DevSkim 40419797875 success；独审 1744-ARCHIVE-INDEPENDENT27N、1744-DEVSKIM-INDEPENDENT27P、集成 1744-INTEGRATION27Q PASS。
- 1702 精确 head：CI 40419849850 success、DevSkim 40419849810 success；集成独审 1702-INTEGRATION-INDEPENDENT27Q PASS；远端 290 passed / 0 failed / 18 ignored（1702-integration27p-evidence/economic-session-linux.log）。
- 1744 先合引入的集成增量：cf72→fcfdb0f 的文件集与 1702 自 cf72 起的文件集交集为空，`git merge-tree` 无冲突；GitHub 重算 CLEAN/MERGEABLE。合后 b9a84b01 与 e7a966 的 `rust/` 树逐字相同，fcfdb0f→b9a84b01 为 5 文件 +953/−22。故不重复独审或完整回放。

## 合后 CI（main）
- fcfdb0f：CI 40428652952、DevSkim 40428652780（记录时在飞）。
- b9a84b01：CI 40428705199、CI macOS 40428705046、DevSkim 40428705100（记录时在飞）。
- macOS 40428705046 是 1702 新 head 内容的首次 macOS FIN/2 运行；旧 run 40389540213 的 page_count/字节/空间读数仍只绑定旧 run，不作新 head 实测。

## 未完成项
- 合后三条 main CI 终态待收取（见下文追加）。
- #1672/#1716 正式下游消费、关父票未做；#1279 保持 OPEN。
- #1488 剩余文书工位/分支处置仍由 cleanup1488-remaining-refs 门管辖。

## 27T checkpoint（2026-10-27T01:16:05Z 实查，无失败，未取消/重跑）
- fcfdb0f CI 40428652952：14 job success（macos-session 按条件 skipped）；在飞 rust-test-s-session shard0/1、extras economic/owned/native。DevSkim 40428652780 success。
- b9a84b01 CI 40428705199：12 job success（macos-session skipped）；在飞 shard0/1、extras economic/native/owned、#1488 固定字段共享核(Native)。DevSkim 40428705100 success。
- b9a84b01 CI macOS 40428705046：issue1716-real-s、issue1717-real-s success；macos-session（1702 新 head 首次 macOS FIN/2）在飞。
- 续接：`gh run view <id>` 收三条 run 终态；全 success 后将 unit land1702-1744-main27t 与 integrate1702-after1743-27p 置 merged-main-ci-pass-formal-consumer-pending；任一失败交原 owner 返修，不重跑通过的 job。
