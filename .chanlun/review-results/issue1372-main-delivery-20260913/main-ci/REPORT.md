# #1372 / PR #1451 合入后的主线 CI 证据

结论：实际主线合并提交 `21b7898a19baa4d17c5885d122ea57ed882e4f73` 的四项常规 CI 全部通过；DevSkim 仍为真实 **FAIL / exit 1**，主线 gate 在 **9 秒**内完整结束。下载主线完整 SARIF 后，与用户批准候选 `041c8e2ef7916a465d8b352ff9dc1469707f8ac1` 对拍，新增 0、移除 0，全部结果内容的多重集相同。

本报告只读跟踪既有运行，未重跑测试或扫描，未修改仓库、CI、基线、远端票面或原有证据。用户对 C 合入及此前说明的扫描红灯例外由根代理记录；本报告不另行扩大批准。

## 提交与实际执行绑定

GitHub 原始 commit API 给出 merge 的两个父提交恰为 `e46cf3bca6a67da821f9ab046507563b8133f6d9` 和批准 head `041c8e2ef7916a465d8b352ff9dc1469707f8ac1`。merge 与批准 head 的完整 tree 均为 `4ebe9ffe886229634640a0e7c5da57f9d8ecacb8`。2026-09-13 12:30 UTC 读取 main ref 时也指向此 merge；不把该观察冒充后续时刻永不变化的 main。

五个 job 的原始日志均直接打印 checkout `21b7898a19baa4d17c5885d122ea57ed882e4f73`；具体行号见 `FACTS.json / actual_checkout_log_bindings`。常规 run `34757202243` 与扫描 run `34757202251` 的 event 都为 push，head_branch 为 main。

## 主线实际检查

[常规 CI 34757202243](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34757202243) 已完成并成功。

| Job | 结果 | 原始记录中的范围 |
|---|---|---|
| rust-check / 103723520380 | SUCCESS | check、fmt 与三个 test 命令成功；default 汇总 3121 passed / 172 ignored，s_session 65 passed / 1 ignored，backtest_bin 2941 passed / 161 ignored；失败均为 0 |
| test / 103723520413 | SUCCESS | 5059 passed、107 skipped、11 deselected、1 xfailed、23 warnings；pytest 147.35 秒 |
| fixture-drift / 103723520491 | SUCCESS | 三个 fixture 的语义字段差异均为 0，字节也全部相同 |
| s-observer / 103723520497 | SUCCESS | Python 51 项 OK；浏览器协议 32 项 passed、failed 为空 |

Rust 计数按实际命令及原始 summary 行分别汇总；不同命令有重叠，不能相加为唯一测试数。所有 ignored/skipped 均未当作通过。fixture 包含 `theta_v0_center_parity.json`、`certificate_chain_parity.json`、`theta_v0_parity.json`。

新增五项 DevSkim 回归测试的主线执行依据：本次与批准候选同树、同收集命令，5059/107/11/1 结果数相同；两份完整日志的 54 行 skip 摘要逐字一致，合计 107，没有新文件或“门禁测试需要 Bash 与 jq”门卫跳过。源码仍恰好五个测试，无 slow/xfail 标记。结合已冻结候选相对旧提交 passed 增加 5 的独评证据，支持五项在本次主线实际执行通过。`-q -rs` 日志不逐个打印成功用例名，本报告不补造五条具名 PASS。主线完整对拍存于 `TEST-COMPARISON.json`；复用候选证据及其 SHA 列于 `FACTS.json`。

## 主线扫描与批准候选的完整比较

[DevSkim 34757202251](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34757202251) 的 scanner 和普通 SARIF artifact 上传均成功。gate 从 **12:34:11 到 12:34:20 UTC，9 秒**，失败原因是 **25,691 个基线外键**。日志先记录 exit 1，随后才出现 3037k 摘要超过 1024k 展示限制；展示错误没有替代真实基线失败。

| 量 | 批准候选 | 实际 main |
|---|---:|---:|
| SARIF 原始结果 | 33,952 | 33,952 |
| 唯一 rule/path/line 键 | 33,878 | 33,878 |
| 基线外唯一键 | 25,691 | 25,691 |
| 基线外原始结果 | 25,734 | 25,734 |
| 新增 / 消失键 | — | 0 / 0 |

比较涵盖全部结果，核了原始键多重集、每键 snippet SHA 多重集和每条完整 JSON 结果的规范化多重集，全部相同。主线 gate 日志中的 25,691 个键与主线 SARIF 减基线后的排序列表逐项相同。两份 SARIF 原始文件 SHA 不同；结果排列次序不同，去除 results 后其余 JSON 完全相同。不能宣称两个原始文件逐字相同。

主线 artifact `10317822259` 的 ZIP 共 78,668,965 bytes，其 SHA256 `29c419f91740f6220610cc31095bf9de9168eb45a73d2d40beba1b1f4d54ca2c` 与 GitHub artifact digest 相同。解包仅有 `devskim-results.sarif`，130,106,700 bytes，SHA256 `899b48a581e0d9b6221161c54dd6fb8ce04e86684ddc2126f65e03587778f9f2`。完整比较见 `SARIF-COMPARISON.json` 和 `SARIF-FILE-DIFFERENCE.json`。

批准候选此前 gate 为 6 秒，旧版为 39 分 02 秒；本次主线 9 秒是新的实际观察，不作为锁定同一机器的性能基准。扫描基线、workflow 与 gate 仍是批准树内版本，未为通过而放宽。

## 例外与证据边界

结果完全相同允许复用已冻结候选分诊，不表示本轮重新审完全部安全发现。先前报告的 `DS176209|rust/src/theta_v0/nautilus/strategy.rs|314` 仍由 [#1385](https://github.com/xy7365527-lang/NewChanlun/issues/1385#issuecomment-5648592872) 承接 needs_review；本轮没有重审该项，全仓实际风险总数仍未知。没有新扫描差异需要扩张 C 的既有例外。

主线 CI 成功也不扩大 C 的 TestOnlyCC006/local_shape 验收域，不代替其他构造、经营、真实故障或历史回放验收，不关闭 #1323 总图。

原始 API、每次读取的命令/UTC/退出码/长度/SHA 收据、五份完整日志、完整 SARIF 均保留在本报告的仓外源目录。`ORIGINALS-INDEX.json` 逐件绑定；`FACTS.json` 固定主要事实与复用来源；`REPORT-FREEZE.json` 固定本报告和索引。建议入仓仅收报告与必要小 JSON，原件保持仓外。
