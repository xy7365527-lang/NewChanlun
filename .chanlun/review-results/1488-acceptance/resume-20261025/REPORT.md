# #1488 P2 续接验收报告（2026-10-25）

## 候选与范围

代码候选 `c5f49adc97d8e40ab1753a7d295a5d157c15af94`，分支 `codex/1488-fix-p2-resume-20261023`。唯一工位 `/Volumes/AgentStorage/issue1488-ticket-manager-20261018/worktree-fix-p2-resume-20261023`。本报告接续 0091 的 P2-1/P2-2 及最终材料缺口，不重做 #1488 主体。

本次 GitHub main 读取值 `13ed4d45c8ff85175819ca987690328ab2a247a4`；共同基线 `66c409e7e3236617e4020c27144049ab7c380e15`。没有再次追 main、没有合入或关票。交付须 squash merge，过程提交包含失败检查点，不宣称逐个过程提交均可编译。

## 修复与溯源

1. 原8文件未提交 WIP 完整保存在仓外 `workers/resume-20261025/preflight/` 并逐字提交 `e66fbe7caa`；具体原作者无法追认。其 native 编译失败不是网络：误删 SourceWork/PublicationStageIdentity 导出、测试所需 mut，以及五处 JSON 变体断言。已恢复并按 test/feature 精确门控，未删除或放宽断言。
2. 真实原探针为 S/N/J/JN × P/T，不是旧报告中的 default/B/BN/BJ。本轮8档全部 cargo RC=0；optional_sql 在 JT/JNT 使用、HashSet 在 NT/JNT 使用，原无条件“8组合无调用方”结论撤回。完整矩阵见 P2-CLASSIFICATION.md。
3. native 未用方法/字段按实测 P/T 逐项门控；宏 native_shared 的 reason 改为部分实例未用，避免把一个宏位置误当作全部实例。没有模块级 allow。
4. WIP 保留的 AtomicU64::try_update 为当前工具链 API 替换；不改变闭包、ordering 或返回处理。counts 可见性修复仅供现有同crate测试；unused mut/import 和 progress_handler 返回值消费不改生产控制流。
5. 本轮属性编辑曾误动两处 SQL 字符串转义（ef258130d5），远端报语法错；d8dd159d41 已逐字恢复，随后 native -D warnings 通过。失败原件保留，不计为通过。
6. c5f49adc97 为 WSL cargo fmt 输出；没有改动 content_producer，BSP 锚仍指向既有 :3114。

## 验证与证据适用性

最终执行脚本 validate-final.sh 包括 fmt、default/backtest_bin/s_session/native 四档 -D warnings check、额外 JSON+native check、两档 BSP 锁及两feature的 business_json 受影响测试。最终10项全部RC=0，完整原件为 `logs/20261025-220319-c5f49adc97.log`（293秒）。

| 项 | 结果 |
|---|---|
| fmt | 通过 |
| default/backtest_bin/s_session/native -D warnings | 四档全部通过 |
| JSON+native -D warnings | 通过 |
| BSP default/backtest_bin | 各1通过，0失败 |
| business_json native | 15通过，0失败 |
| business_json JSON+native | 20通过，0失败 |

0085/0091 的97条锁、S4 release、S86、宽对拍及失败集证据继续覆盖此前主体；本轮没有改它们覆盖的算法与合同，不重复全量。旧 CI run 39687482045 不冒充当前候选 CI。

## 关票门与后续消费者

- 主体仍是 testonly 增量构造；生产增量切换归 #1645，测量归 #1642。#1488 通过不等于 Destination ①–⑥通过。
- 0091 第1/4/5/6项需要非作者对本报告、候选/基线关系、关票声明与 PR/CI 复核；由主控安排。
- main 合入、人审和关票尚未发生。#1279保持开放；前端归#1460，K4不并入。资金、隔离、恢复与验收要求不变。
