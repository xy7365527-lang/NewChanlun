# 收据 workers/accept-s3/REPORT.md

- 原件：`workers/accept-s3/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`0c3ab2e79c0ff70a5d36b13035c8d8ba9c0645337f5535d46dd7081080dea017`
- 字节数：8688；行数：94

## 关键行（行号: 原文）

```
1: # #1488 S3 交付报告：残留 append 批量化、stage_indexes/assemble_manifest 增量（D5/D6）
4: - 起点 `c2bdf9b662` → 终点 **`bba547492d`**（只在本分支追加提交，未 push 本分支，未合 main）
8: ## 结果总览
13: | CI run **39384343438**（uo-ci-fixed.sh @bba547492d） | **success**；`uo (s_session,s_capture_native_testonly)` 84 条锁全 PASS；`uo (s_session)` 4 条 PASS，其余 80 条 NOT_LISTED（受 feature 门控，与既往一致） |
15: | 宽对拍基线 @c2bdf9b662（同一命令） | 236 passed / 16 failed / 5 ignored，**失败测试名集合与 HEAD 逐条相同** → 都是起点已有的失败，不是 S3 引入（失败集中在 audit_owner 的 effects/q/construction、cross_request_source_cache r11/r13/r14、request_audit r_owner_move/r11、driver source_keeper，都是账本/WorkScope 断言） |
17: ## 1. fractal/bi 逐元素 append → 批量写后缀（D5）
38: ## 2. stage_indexes：aux_index plan 按变更行增量（D6）
59: ## 3. assemble_manifest：核查结论（没有改产品代码）
70: - 结论：
80: ## 4. 锁清单（本工位 `.github/1488-uo-locks.txt` +4）
89: ## 待主控裁定 / 转出
```
