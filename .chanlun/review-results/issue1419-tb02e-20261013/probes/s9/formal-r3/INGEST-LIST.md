# 入仓候选清单（只列不提交）

建议目标：`.chanlun/review-results/issue1419-tb02e-20261013/s9-formal-r3/`（工作草稿，绑定 #1419；前两轮同体例放 `s9-formal/`、`s9-formal-r2/`）。来源均为 `/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal-r3/`，字节以本目录 `SHA256SUMS` 为准。

| 文件 | 大小量级 | 说明 |
|---|---|---|
| RUN.md | 7 KB | 结果说明 |
| RESULT.json | 20 KB | 机读结论（四臂、a/b、恢复、跨轮同一性、耗时、背景） |
| raw/COMPARISON-RESULT.json | 2 KB | harness 总状态 `PASS_EXCEPT_BROWSER_DEFERRED` 与两对 a/b 比对原件 |
| raw/RUN-PLAN.json | 27 KB | harness 冻结计划（含 `browser_deferred_to: "#1497"`） |
| PREFLIGHT.json、DRIVE.json | 各约 5 KB | 门检与驱动时间线 |
| SLOT-SNAPSHOT.json、SLOT-REAP.json | <1 KB | 回收收据 |
| ARMS.json、ARM-TIMINGS.json、BIG-ARTIFACTS.json | 各 <15 KB | 四臂结论、计时、外盘大件 SHA |
| s9_drive.py、s9_extract.py、s9_finalize.py | 各 <15 KB | 驱动、抽取、汇总脚本 |
| raw/<臂>/EXIT.json | <1 KB ×4 | 四臂退出 |
| raw/<臂>/evidence/RESULT.json、query-oracle.json、normal-recovery.json、browser/RESULT.json | 各 <4 KB ×4 | 采集、冻结账核对、恢复对拍、DEFERRED 收据 |

不建议入仓（留在本目录或外盘，SHA 已记）：`background-samples.jsonl`（约 0.5 MB）、各臂 `events.jsonl`、`control/q.log`、`http-MANIFEST.sha256`（每臂 2.9 MB）、`messages.jsonl`/`clock.json`/`config.json`（可由 harness prepare 重建）、外盘 HTTP 收据、`semantic-core.jsonl` 与库。

注意：这些文件含本机绝对路径（同 #1414 FS-N7），未含凭据。
