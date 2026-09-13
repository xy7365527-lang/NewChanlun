# #1372 主线落地及末尾报告补齐

名分：#1372 交付工作草稿。本包只补收最终扫描/测试复核报告及实际主线结果，不修改产品、测试、扫描基线或忽略规则。原冻结报告保持原字节，其中当时尚未批准、未合入或待后续评审的状态由本说明按实际时间接续，不回写历史。

用户已明确批准最终候选 `041c8e2ef7916a465d8b352ff9dc1469707f8ac1` 的 main 合入及已说明的 DevSkim 红灯例外。[PR #1451](https://github.com/xy7365527-lang/NewChanlun/pull/1451) 于 2026-09-13 12:28:54 UTC 合入，主线提交为 `21b7898a19baa4d17c5885d122ea57ed882e4f73`。两个父提交为原主线 `e46cf3bca6a67da821f9ab046507563b8133f6d9` 与批准候选；主线和候选 tree 都是 `4ebe9ffe886229634640a0e7c5da57f9d8ecacb8`，没有额外产品差异。

## 实际主线检查

[CI 34757202243](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34757202243) 的 test、rust-check、fixture-drift、s-observer 四项均 SUCCESS。主线 test 实际为 5059 passed、107 skipped、11 deselected、1 xfailed、23 warnings，与批准候选一致；新增五项 gate 回归的来源/收集/跳过清单复核见 [candidate-ci/REPORT.md](candidate-ci/REPORT.md)。普通 pytest 的逐点输出不冒称逐个测试的具名 PASS 日志。

[主线 DevSkim 34757202251](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34757202251) 实际 FAILURE。scanner 与 artifact 上传成功，gate 用时 9 秒，完整输出 25,691 个基线外键并返回 1；之后 3037k 摘要超过展示上限，不替代此前的真实差集失败。主线全量 33,952 条结果及 33,878 个唯一键，与批准候选的规范化完整结果多重集相同，新增 0、移除 0；日志差集与 SARIF 减 baseline 逐项相同。两份 SARIF 的结果排列次序不同，原字节与 SHA 不同，results 之外的 JSON 相同；这里不宣称原件逐字一致。原件、比较方法与边界见 [主线独立报告](main-ci/REPORT.md)。

候选到主线只复用已核同树的产品行为证据，不把候选 CI 称为主线执行。本包尚未进入 main；它自身后续触发的检查应另按实际结果声明，不能使用本段冒充。

## 已有产品验收与末尾评审

[C 主审阅包](../issue1372-tb01c-20260913/README.md) 已随 `acd30cb61e59762114061b2d0b394aa270ed35ec` 入仓并合入主线：AC1–7 在具名 TestOnly CC-006/local_shape 域通过，AC8 浏览器/记录子项通过。持续输入、固定 cut 完整分页、有效续接、真实双进程故障与两轮 495 条全核心记录逐字段/原字节一致，保留该报告的实际负载、资源与失败轨迹边界。完整结构、经济经营、真实场所或全图完成均不由此推出。

主报告末尾曾待补的 R9/R10、无损编码 memo 和 CI 配置独评现有明确落点：`../issue1372-tb01c-20260913/reports/q-encoding-memo-independent/` 下的 `R9-REVIEW.json`、`REVIEW.md/json` 和 `CI-REVIEW.json`；最终行为复核见该主包 `reports/runtime/r5-pair/independent-review/REVIEW.md`。gate 和五项 Python 回归尾增量的两份独评已随 041c 入主线，见 [gate 修复审阅包](../issue1372-devskim-summary-20260913/README.md)。本次未新增代码尾差，也未重复运行产品试验。

原 e46 到 3ff 的 18 个提交及逐项真实编译/语法证据见主包 `reports/delivery-preparation/commit-check-inventory/INVENTORY.md/json`；这里只接续它的历史状态。`68198a6cdabbdbfeee916a00321442999c3de673` 为纯文档豁免。其后 `acd30cb61e59762114061b2d0b394aa270ed35ec` 仅 82 个报告/工位文档路径，也属纯文档豁免；`041c8e2ef7916a465d8b352ff9dc1469707f8ac1` 修改 gate 与测试，不豁免，五项测试及远端常规 CI 已通过。不能从末端成功倒推每个旧提交的所有功能当时已无缺陷。

## 扫描例外的精确范围

[候选最终分诊](candidate-scan/SUMMARY.md) 保留 25,691 个基线外键：10,888 个继承项、2 个原 baseline 内行移、345 个 C 产品新增、14,315 个 acd 报告新增和 141 个 041c 报告新增。141 个新增均为七份报告 JSON 的输出/文件/gate 回放摘要或 Git 身份；gate 与 Python 测试源新增命中为零。全量分诊方法、字段定位及原件索引保留在 candidate-scan。

例外仅覆盖已说明候选的合入，扫描规则、baseline 与失败状态保持。旧主包 README 的“#1385 既有314条策略待核”在这里订正：它指 `rust/src/theta_v0/nautilus/strategy.rs` **第 314 行**的 DS176209 待核项，不能读作 314 个缺陷。该项继续由 #1385 跟进；本次没有完成全仓实际安全风险审计。

## 报告与工位收尾边界

`REPORT-SOURCES.json` 逐件绑定本包报告的来源、长度及 SHA，所有副本保持原字节。完整 SARIF、原始 API、日志、数据库及工具仍按原件索引保全于仓外，不把工具与大体积运行原件作为新产品源码入仓，也不要求为每条 CI 链接再制造下一层报告。

原 `codex/1372-codex`、`codex/1372-query`、`codex/1372-rust` 三工位均已核无未提交或未跟踪在制品。query/rust 各四个旧候选提交已由主控接回并演进到正式候选；为保留原历史，三分支已备份为 `issue1372-original-workstations.bundle`，1,080,545 字节，SHA256 `8b3ad554ec09a4f2c0afa7f8cc9c8b657757d712d91105938c7ee8ade8250540`，前置 e46，bundle verify 成功。备份位于本任务仓外交付目录的 main-merge-20260913/workstation-preservation。当前没有删除这三个工位。

本纯文档工位为 `codex/1372-delivery-tail`，从实际主线 21b7898 切出；尾包待合入。关 C 前仍需本包实际交付并处置相关工位，在 resolution 引用真实主线 SHA、检查链接、既有代码评审与纯文档豁免。父 #1359 必须另按其四项 AC 汇总 A/B/C，不能只因 C 关闭自动通过；后续 TB 的具体构造/经营/历史义务继续由原映射承担，完整 #1323 六 Destination 不变。
