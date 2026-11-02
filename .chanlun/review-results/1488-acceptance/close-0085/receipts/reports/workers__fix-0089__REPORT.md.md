# 收据 workers/fix-0089/REPORT.md

- 原件：`workers/fix-0089/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`d3bc49d9076d84caa97c5fa8ad4c55150658aaa742ff315500090c20a4af6642`
- 字节数：8958；行数：103

## 关键行（行号: 原文）

```
1: # fix-0089 REPORT：0087 补审两项修补 + S4 锁 CI 形态
3: 工位 budget-handoff-fix-v2-checkout，分支 codex/1488-budget-full-chain-fix-v2-20261018。起点 **f0aeaf2817**（已核对，与 premerge-0086 REPORT 的交付 HEAD 一致），终点 **327aa94c39**。
7: ## 1. 提交
13: | ⑤-P2-1 变异 | eb668db7f0 | S4 锁第 2 代：在各项对拍之后、可达闭包逐位对拍之前，翻转增量臂一个根可达叶节点 body 的末字节 |
14: | ⑤-P2-1 revert | a7481aacb5 | `git diff eb668db7f0^ a7481aacb5` 为 **0 行** |
18: ## 2. ①-F-1
38: | 提交 | sha256 | 字节 |
46: ## 3. ⑤-P2-1 负控
57: ## 4. S4 锁的 CI 形态（负责人裁定）
67:   - 步级 timeout-minutes 50；单锁 `timeout 2700`（45 分钟）；结果追加到 locks.tsv，记为 PASS_RELEASE / FAIL_RELEASE / NOT_LISTED_RELEASE。
76:   - uo (s_session)：4 PASS + 93 NOT_LISTED（Native 门锁），release 步按设计 skipped。
77:   - uo (native)：97 PASS + S4 **PASS_RELEASE**。release 编译 3.5 分钟；S4 跑了 **1962.15 s**（32.7 分钟），离 2700 s 的上限约有 12 分钟余量。
82: ## 5. 验证读数
94: ## 6. 结论
97: - ⑤-P2-1：跨臂可达闭包逐位对拍独立变红的负控已补，revert 零残留。
102: 2. S4 release 步只放在了 uo-iter 工作流里，没有放进 main ci.yml 的 s-fields-native-1488。按裁定这里是“二选一”；合入 main 之后，push main 不会触发 S4。
103: 3. 0087 的 ⑤-P2-2（节点表口径放宽）不在本票范围，仍待负责人确认。
```
