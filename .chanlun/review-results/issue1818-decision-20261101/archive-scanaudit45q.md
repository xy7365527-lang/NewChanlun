# PR #1843 DevSkim 登记增量独审（45Q）

结论：**PASS（精确源增量）**。正式 CI 尚在运行，合入仍须等待精确 head 必要检查通过并核当前集成。

- 审查 head：`b6dd6bfb07ac185b3af4b56c576b41cb28e479d5`。
- 已审父 head：`b051adceccbfbdff0c96fc0b2711f514e75d71f8`。
- 作者 Opus；非作者独审 Astra。工位 `/Volumes/AgentStorage/decision1818-archive45f/worktree`，本地与 GitHub head 一致，读取时工位干净。
- 复用 `/Volumes/AgentStorage/archive1818-review45i/DELTA-PASS.md` 的文书结论，本轮不重新裁定 ADR0059。

## 实证核验

1. 父到子只改变 `.github/devskim-reviewed-1488.json`。schema、issue 不变；原 112 个文件条目逐项完全相同，仅追加 5 个，共 117 个。22 份封存原件、SOURCES、ADR、baseline、扫描器、规则、workflow 和超时没有增量。
2. 直接读取旧正式扫描的 94,377 条 SARIF result。新增 5 个文件恰好命中 12 条，全部 DS173237；逐条对完整 result 做规范 JSON 身份校验，与新增 identity 集合严格相等，无额外 result 豁免。
3. 从精确 head 用 Git 读取五文件完整 blob，核 SHA256 字节均与登记一致，并与当前工位字节一致。哈希仅用于本门要求的原件绑定与 result identity。
4. 命中值并非只因长度被认定为误报：六个本仓值经 `git cat-file -t` 均证实为 commit；字段是 head/base/diff_base。另一个 `f6217f891ac0bb64f3d375211650a4c1ff8ca1ea` 位于 apple_source_commit，GitHub API 证实它是 Apple XNU 的 `xnu-12377.1.9` 提交。均为版本引用，不是凭据。
5. 阅读现役 identity/load_allowlist/classify 及 shell gate/workflow：绑定路径、规则、完整 result 与文件字节；本次没有按规则或目录整体跳过。
6. 独立分类同一 SARIF：排除数 2,898→2,910，恰好增加 12；基线外键 8→0。独立运行现役 shell gate、沿用 workflow 参数，exit 0，PASS。这里只重算门判定，未重跑扫描器；输入的 scanner/upload 成功来自旧正式日志。

## 新增登记范围

| 封存文件 | result 数 | 行 |
|---|---:|---|
| layout-delta41o.json | 2 | 1 |
| layout-delta41t.json | 3 | 1 |
| layout-final41w.json | 3 | 8、9、39 |
| layout-review41i.json | 2 | 6、7 |
| layout-review41r.json | 2 | 1 |

## CI 与消费条件

本轮单次查询中，PR OPEN 且 head 仍是 b6dd6bfb；DevSkim run `41354533891` IN_PROGRESS；常规 CI run `41354533916` 仍有运行和排队项，fixture-drift SUCCESS、macos-session SKIPPED。本报告不把源审 PASS 当作全部 CI 通过。

主控合入前继续核必要 CI 与当前 main 集成。PR #1844 也修改同一登记文件，集成时应保留两侧完整条目；若候选 head 改变，按真实增量更新验收。旧工位清理须在 PR1843 合入后执行，本审没有清理或关票。

证据：`/Volumes/AgentStorage/archive1818-scanaudit45q/audit.json`（独立计数、差集、文件绑定）、`/Volumes/AgentStorage/archive1818-scanaudit45q/gate-summary.md`（独立 gate 复核）。输入 SARIF 与失败日志位于 `/Volumes/AgentStorage/archive1818-scan45n/`。

外部提交核实：https://github.com/apple-oss-distributions/xnu/commit/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea 。

本次无实现代码或代码注释变更；no-comments 无审查对象。未运行 Rust、未修改作者工位、未 push/merge、未改图台账。
