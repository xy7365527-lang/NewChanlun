# #1672 FIN/2 真实运行验收 27

## 业务验收通过，最终 CI 尚在途

run [40389540213](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40389540213) 的 FIN/2 步骤实际通过两个 macOS ignored 测试。21 个原信封已完整重送，原请求逐值相同，原回执除合法 epoch 更新外逐值相同。最终三本 B 的数量为 `[2,5,5]`，K 为 `[204,506,506]`。四域恢复与暂存清理通过，四卷回收成功。

artifact 的实际源码是 `9e77756b5038007d081962875d948511284bcef3`。`1672-INTEGRATED-INDEPENDENT27.md` 已独立裁定其到最终 PR1702 head `0932d3dbcdb18d138ce1e67ed05f971d87e75e3b` 仅有 rustfmt 差异，可复用本次 macOS 证据。复用仍以最终 head 的常规 CI 和 DevSkim 通过为条件。

本报告只验 #1672 基线，不代表 #1673 全链或 #1323 整图完成。Babysit 使用 background 模式，无 dispatch、重跑、取消或 GitHub 写入。

## 原件与可复核检查

原件在 `1672-32M-evidence27/final-run/artifact/`，artifact ID `12682906680`，名称为 `issue1672-fin2-9e77756b5038007d081962875d948511284bcef3-1`。归档步骤在 G-1 之前执行，已实际 success。

`final-run/verify_artifact.py` 是只读核验脚本。执行 `python3 1672-32M-evidence27/final-run/verify_artifact.py` 可重做原信封、回执、空间及四个最终 SQLite 库的核对。输出保存在 `final-run/verified-summary.json`。数据库以 `mode=ro&immutable=1` 打开，未变更归档库。

`originals.jsonl` 和 `resends.jsonl` 各 21 行。真实响应种类为 6 个 Capture `Recorded`、3 个 Resolve `Resolved`、12 个 Apply `Applied`，不能把三类统写为 `Applied`。21 对请求及 owner 完全相同，21 对响应只有顶层 `producer_epoch` 与 `source_epoch` 从 `1` 变 `2`，payload、message_id、payload_hash、因果引用全部相同。

最终 `next=22`，seq 为 `[10,4,4,4]`，四域 epoch 均为 `2`。`recoveries.jsonl` 有四个真实 `Recovered`，cut 对应 E seq 10 和三个 B seq 4。恢复前后完整图像逐字节对拍、messages 四列整表冻结、receipts/applications/next 不变由实际通过的测试断言提供，代码强度复用 Opus 已完成的独审。归档只保留最终数据库，不虚称另有恢复前快照可独立重算。

最终四库中 `content_staging=0`。E 的 commits/messages/receipts/applications 为 `11/10/10/0`，每个 B 为 `5/4/4/4`。包含初始化 seq 0，因此 commit 行数比最终 seq 多 1。三库共 12 个 application，没有第二 application。脚本把全部原信封与库中 `messages.request_json`、`payload_hash` 和原持久 receipt 逐值核对，并从每轮 commits.image 重核 HAND 三报的数量与 K。

## 容量、页帽与回收

`storage.jsonl` 共 75 行，含 initialized 4、original 21、before_recover 4、after_recover 4、before_resend 21、after_resend 21。逐行包含真实持久图像 BLOB 长度、page_count、freelist、连接页帽、DB/WAL/SHM 字节及 st_blocks 分配量、容量、可用空间和 cleanup 原公式余量。

| 域 | 最大观测 page_count | 最终图像 B | 最大文件分配 B | 最小 cleanup 余量 B |
|---|---:|---:|---:|---:|
| E | 209 | 91,706 | 1,277,952 | 22,950,272 |
| B N1 | 584 | 771,028 | 2,424,832 | 21,803,392 |
| B N2 | 605 | 785,605 | 2,510,848 | 21,717,376 |
| B N3 | 622 | 800,181 | 2,580,480 | 21,647,744 |

四卷实际容量均为 `33,513,472 B`，`carrier_page_limit=1016`。没有发生 32 MiB 容量不足。所有观测点的 cleanup 余量与 next-write 余量均为正，后者最低 `13,144,832 B`。这些是观测点读数，不扩张为未观测瞬态峰值保证。

21 对重送的 seq、writer_epoch、全部 storage 字段保持一致，只有四域首次重送时连接页帽由 SQLite 默认 `4294967294` 设为 `1016`。这不是 page_count 或数据增长。生效页帽、连接设置与诊断预算已由 Opus 结合源码独立解释，本脚本单列并允许的就是这一字段变化，没有放宽业务冻结。

provenance 记录 macOS `26.6.2`、Build `25G83`，四个设备 `/dev/disk31s1` 到 `/dev/disk34s1`，全部为独立 HFS+ 挂载根。`cleanup.txt` 实记 `command_exit=0`、`cleanup_failed=0`、`root_exists=no`。最终 artifact 共 19 个文件，四个 checkpoint 后真实 `owner.db` 均完整保留，RAW/HAND/fin-input/manifest、原信封与回执、恢复与重送、空间日志和完整 cargo 输出齐全。六个 Capture 的原 raw_utf8 与归档 RAW.json 逐字相同。

负控与正例实际输出 `2 passed; 0 failed; 0 ignored; 3531 filtered out; finished in 20.75s`。负控 `fin2_capacity_shared_carrier_and_non_hfs_are_rejected` 实际为 ok，其覆盖范围复用 Opus 的冻结代码审查，不以测试名代替实现核验。

## CI 与剩余运行门

macOS run 已终态 failure。`macos-session` 于 `2026-10-26T20:22:18Z` 终态 success，FIN/2 步 7、归档步 8、#1717 步 9、G-1 步 10、#1716 cap 步 11 及上传步 12 全部 success。G-1 实际 `278 passed; 0 failed; 19 ignored`。独立 `issue1717-real-s` job success，唯一失败为独立 `issue1716-real-s` job。完整日志保存在 `final-run/macos-run-complete.log` 和 `final-run/macos-session.log`，不把相关业务成功写成整 run 成功。

#1716 真实失败日志保存在 `final-run/issue1716-real-s-failed.log`。实际失败是 `finance_real_s_tests.rs:217` 的 Reserve 收到 `Incomplete`，提示最早绝对期限已过，staged_cleanup rows=1310。该线定性由主控的 `diagnose1716_run27` 负责，本验收不修期限、不改判据。

60 分钟 timebox 最后实查为 `2026-10-26T20:54:44Z`。最终 head `0932d3d` 的 DevSkim [40390534300](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40390534300) 已终态 success。常规 CI [40390534105](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40390534105) 仅剩 `rust-test-s-session (shard 0)` job [121810649397](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40390534105/job/121810649397) 在途，其余作业均 success，PR 下预期跳过的 macos-session 为 skipped。真实状态保存为 `final-run/ci-finalhead.json`、`final-run/devskim-finalhead.json` 和 `final-run/run-progress.json`。最终候选复用条件尚缺这次常规 CI 的成功终态。主控接续该原 run 即可，无需重跑 FIN/2。#1716 已交 `repair1716_deadline27` owner 接续，main 放行仍归主控，未作合入或关票。本地观察已停止，GitHub run 保持运行。

## 方法

Prove It Works 决定下载实际原件并读取最终数据库，不以 RESULT 自报或 job 绿色替代验收。Build the Lever 决定保留可复跑的只读脚本与完整输出。检查未计算任何非必要摘要。源码独审由 Opus 独占，本验收复用其已完成的代码判断，没有重复审源码，也未本机编译 Rust。
