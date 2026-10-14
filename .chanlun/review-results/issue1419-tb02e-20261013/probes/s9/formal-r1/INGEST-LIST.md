# 入仓候选清单（只列不提交）

建议目标：`.chanlun/review-results/issue1419-tb02e-20261013/s9-formal/`（工作草稿，绑定 #1419）。来源均为 `/private/tmp/goal1323-native-20261012/tb1419-impl/s9-formal/`，字节以本目录 `SHA256SUMS` 为准。

| 文件 | 大小量级 | 说明 |
|---|---|---|
| RUN.md | 6 KB | 结果说明 |
| RESULT.json | 8 KB | 机读结论 |
| PREFLIGHT.json | 5 KB | 门检（头、二进制 SHA、进程、端口、版本、背景） |
| DRIVE.json | 5 KB | 驱动时间线与回收调用 |
| SLOT-SNAPSHOT.json、SLOT-REAP.json | <1 KB | 回收收据 |
| background-samples.jsonl | 45 KB | 运行期间负载采样 |
| ARMS.json、ARM-TIMINGS.json、BIG-ARTIFACTS.json | 各 <10 KB | 四臂结论、计时、外盘大件 SHA |
| s9_drive.py、s9_extract.py、s9_finalize.py | 各 <12 KB | 驱动、抽取、汇总脚本 |
| raw/RUN-PLAN.json | 27 KB | harness 冻结计划 |
| raw/min12_up-a/EXIT.json、INTERRUPTED.json | <1 KB | 首臂退出与中断回收 |
| raw/min12_up-a/evidence/RESULT.json、query-oracle.json、normal-recovery.json、browser.stderr | 各 <3 KB | 首臂采集、冻结账核对、恢复对拍、浏览器失败栈 |
| raw/min12_up-a/evidence/browser/READINESS-DIAGNOSTICS.json | 1 KB | 浏览器事件 |

不建议入仓（留在本目录或外盘，SHA 已记）：`raw/min12_up-a/evidence/browser/network.har`（950 KB，含 503 响应体，可截取该条单独入仓）、`raw/min12_up-a/evidence/events.jsonl`（130 KB）、`control/q.log`（320 KB）、`http-MANIFEST.sha256`（2.9 MB）、各臂 `messages.jsonl`/`clock.json`/`config.json`（可由 harness prepare 重建）、外盘 HTTP 收据与 `semantic-core.jsonl`。

注意：这些文件含本机绝对路径（同 #1414 FS-N7），未含凭据。
