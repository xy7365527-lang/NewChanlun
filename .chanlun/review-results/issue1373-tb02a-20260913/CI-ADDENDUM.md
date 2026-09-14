# #1373 / PR #1453：首次远端检查及测试守卫修复

本文件是工作草稿，绑定开放票 #1373。原 `REPORT.md`、`EVIDENCE-INDEX.json` 及仓外冻结证据不改写。本追加件记录首个候选 `4408b0c233f45e33f53cf152112bbcdc3604344f` 的实际远端结果，以及随后六行测试改动的独立验证；不预记新提交的远端结果或 main 批准。

## Rust 检查

[CI run 34764681623](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34764681623) 的 `test`、`s-observer`、`fixture-drift` 成功；`rust-check` 在默认目标测试失败：2938 passed / 1 failed / 161 ignored，唯一失败为 `theta_v0::classifier::bsp::tests::no_stray_bsp_binding_rewrites_outside_registry`。

三处命中均位于 `rust/src/bin/s_session_v2/tb02a_facts.rs:231,241,267`，按组首原始锚查询 `InclusionFacts.groups`：取三组高低价、右组末成员、三组原始成员。载体是 `InclusionGroupFact`，不查询 `BspPoint`、买卖位或确认侧，不是 BSP 绑定规则复制。

修复仅在 `bsp.rs` 的测试白名单增加三条精确 `file:line` 和三行理由。生产代码、扫描正则、扫描范围及 stray/stale 双向断言逐字不变。修复前本地复现相同三条报错（exit 101）；修复后现有 BSP 模块 18 项全部通过（exit 0），格式与差异检查通过。

Codex 原生独立审查为 **APPROVE，零发现**。独评确认实际命中 25、白名单 25、stray 0、stale 0；逐条移除新增登记会恢复对应报错，同文件其他行的新命中仍拒绝，登记点消失仍报 stale。该反例在内存集合中执行，未修改生产文件。原完整运行和 GUI 验证继续覆盖逐字未变的生产代码，本次新增测试改动由上述测试及独评覆盖。

## DevSkim 首次完整差分

[DevSkim run 34764681547](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34764681547) 的扫描及 SARIF 上传成功，zero-findings gate **失败**。扫描实际 checkout 的树与候选树相同。对比真实基线 `65298686cf3e7c9e94e0b9ceeb83aebad2b5530b` 的完整 SARIF：

| 范围 | 实测结果 |
| --- | --- |
| 原始发现数 | 34663 → 34783 |
| gate 唯一键 | 34589 → 34708 |
| 基线外键 | 26402 → 26521 |
| 新增 / 消失 | 121 键（122 条）/ 2 键（2 条） |
| 原 29 个变更路径之外 | 34659 条完整 result 多重集相同，新增 0、消失 0 |

逐项核验新增部分：118 个键（119 条）是证据、fixture、目录完整性摘要或 Git 身份；1 个键是测试驱动声明的本机回环地址；2 个键是既有地址或正则的行号移动。摘要已按报告明列边界重算或绑定签署目录元数据，未发现新增凭据或实际安全缺陷。旧基线外 26400 个键原样保留，另 2 个仅移动行号；原有待审风险继续保留给 #1385，本次差分不证明全仓安全。

gate、baseline、DevSkim workflow、Dockerfile、entrypoint 均与基线逐字相同。没有删除证据摘要、扩宽 baseline/ignore 或关闭检查。日志另外提示摘要超过 1 MiB 上传上限；该显示问题不代替实际 finding 门禁结论。

## 追加证据与交付边界

仓外证据根仍为原报告声明的 `issue1373-tb02a-implementation-20260913`；追加目录为 `ci-monitor/`、`ci-rust-fix/`、`ci-rust-review/`、`ci-devskim/`。关键原件：

| 原件 | SHA-256 |
| --- | --- |
| `ci-rust-fix/FIX.diff` | `4c3365865cdcf36c1a99bf258e38a518a86c94065b4b5ea92d8e1326a160807e` |
| `ci-rust-fix/AFTER.json` | `6b3f9a0e675bd4c20d6d273b3a23495eaf26978ba635008b03ca770f73127d66` |
| `ci-rust-review/READONLY-EVIDENCE.json` | `c3d17a651142a7b52a1a8e1fb1262348a3969d5a55dc4ff5141c4fca7290f6c3` |
| `ci-devskim/devskim-results.sarif` | `dd3f48d873d6c6cd7d74862e9271fc3fc0f1d840f9038319ca0b3c5251b17d40` |

本文件随修复提交前封存；新提交的检查须另取该提交的真实 run，不能复用首次失败 run 作通过声明。main 合入与本候选 DevSkim 例外仍待明确批准；旧 PR #1452 的批准不自动覆盖本 PR。#1373、#1360、#1323 继续开放，#1342/#1343 一般域边界不变。
