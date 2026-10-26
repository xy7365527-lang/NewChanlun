# #1626 已落地交付的证据索引

本目录补齐 #1626 的最终集成独审与 main CI 原件。文书归档尚待非作者尾审、必要 CI 与用户 main 合入批准。#1626 已由 PR #1693 自动关闭，工位仍保留，不能据此宣布关票门全部通过。

## 交付与评审对象

PR [#1693](https://github.com/xy7365527-lang/NewChanlun/pull/1693) 的最终候选为 `2fee8af14fdcdb317024b0a201bb36ea85ee3063`，squash 后落地为 `df7beb0c8da5d8f3b6053f85813b86613dcc6849`。main CI [40268271555](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40268271555) 在该落地提交上已完成且成功。

| 原件 | 用途 |
| --- | --- |
| [最终独审](1693-MAIN-INDEPENDENT-REVIEW26B.md) | 非作者 Claude 对精确候选的 PASS，覆盖两处手工冲突及自动合并交集。 |
| [独审消费与订正](1693-MAIN-REVIEW-CONSUMPTION26B.md) | 订正独审中塔文件路径，保留原报告不改。 |
| [集成报告](1693-MAIN-INTEGRATION26B.md) | 记录冲突解法、两项交叉回归与行为证据复用条件。 |
| [main CI 终态核对](1693-MAIN-CI-TERMINAL26T.md) | 区分已落地代码、自动关票和未完成收尾。 |
| [main CI 原始响应](1693-main-ci-40268271555.json) | 保存精确 `headSha`、终态与各 job 结果。 |
| [工位处置调查](1626-WORKTREE-DISPOSITION26U.md) | 三工位在制品、ignored 文件、持有者与恢复去向。 |

原件中的 OPEN、待 CI、未合入措辞均指报告形成时的状态。当前落地事实以上述 main 提交及 CI 终态为准。

## 复用已有运行证据

两份远端日志按原字节复制到 [1693-main-integration26b-validation](1693-main-integration26b-validation/)。首份绑定 `2554f669bd9923fbb4bc0fee53d86defaef04dd6`，实际通过 positional 20、funding_record 6、policy_records 12 项测试，但首轮 fmt 失败。第二份绑定最终候选，fmt、positional_fusion 78 项、BSP 1 项和 `cargo check --lib` 通过。

合计 117 项测试证据来自上述两份日志。前 38 项由仅一处测试断行的有序 diff 支持复用，未重跑 Rust。main CI 另行覆盖落地提交，不能把靶向测试数量当作完整贯通验收。

原独审 §不变文件核验误写 `rust/src/trading/recursive_tower.rs`。实际路径为 `rust/src/theta_v0/classifier/recursive_tower.rs`。本次复跑 `git diff --name-only 9f13b8efac9231eadf5ff755ca3943b21d8200a2 2fee8af14fdcdb317024b0a201bb36ea85ee3063 -- '*recursive_tower.rs' formal/`，输出为空。错误路径命令不作为证据。

## 原票验收与后续集成审查链

[整票 85b866 审查](1626-85b866-FULL-REVIEW.md) 对 Q-4、Q-2、Q-6～Q-8 判 PASS，对 X-38 的 open 记录未驱动行为判 BLOCK。[b936311 尾审](1626-b936311-X38-REVIEW.md) 以实际 Lean 执行解除该项，原始输出保存在 [Lean 靶向日志](1626-b936311-lean-targeted.log)。[f1f336 尾审](1626-f1f336-BSP-REVIEW.md) 覆盖其后的 BSP 白名单锚订正。

[第一轮 main 集成尾审](1693-INTEGRATION-REVIEW.md) 对 `3e587ab2d05478cbe721d030565eeeb89617a76f` 判 PASS，对应 [远端日志](1693-integration-remote.log)。其后第二轮集成的最终审查与日志见上表。历史 BLOCK 原件照录，不能脱离解除它的尾审单独解释为当前失败。

已有实施报告位于 [issue1626-versioned-fields-20261025/IMPLEMENTATION.md](../issue1626-versioned-fields-20261025/IMPLEMENTATION.md)。本目录只补归档和索引，不重做原票实现或运行验证。

BSP 尾审引用的 [default 与 backtest-bin 验证末段](1626-bsp-anchor-validation/) 一并归档。default 原件只保留末段，不能据此声明总测试数。backtest-bin 末段实际为 2914 passed、0 failed、153 ignored。它们不替代最终 main CI。
