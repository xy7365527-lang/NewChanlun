# #1669 / PR #1676 当前 head 最终独立复核

- 候选：`3af30f1157d1b6165e7bf2c2dca6b9ad3652f885`
- 基线：`origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 方法：对照 #1669 原验收、ADR 0043、当前完整 diff、直接生产实现、既有三轮评审和 CI 原日志。未修改代码、票据或分支，未运行本机 cargo。

## 结论

**FAIL / BLOCK，当前不可合入或关票。**

唯一需要返修的 HIGH 是：票面要求的正本层 Lean 覆盖面声明在当前候选中不存在，且既有评审声称审过的对应文件不在当前 diff 内。CI 全绿另作为一项独立未满足门，见下文。

## 概念层质询结果：发现矛盾

### HIGH：正本覆盖面声明缺失，PR 正文及旧评审与实际交付不一致

#1669 明确要求在 `.chanlun/definitions/sanjieduan.md` §2 登记实际覆盖面：`TotalWealth.lean` 只有阶段单调性、没有资格蕴含定理，两条分方向蕴含由 Rust 性质测试承担；声明须带票号且原句不删。

当前 `origin/main...3af30f1157` 只有以下五个文件：

- `rust/src/economic_session/tb03c_book_tests.rs`
- `.chanlun/review-results/issue1669-20261022/{MUTATION.md,review-r1.md,review-r2.md,review-r3.md}`

`.chanlun/definitions/` 与 `formal/` 的改动数均为 **0**。当前 `sanjieduan.md:34` 仍只是“交付时须声明实际覆盖面”的义务文字，并不是已完成声明；全文检索不到“只有阶段单调性”“没有资格蕴含定理”“两条蕴含均由 Rust”等所宣称内容。PR 正文第 8 行却明确声称该文件已登记这些内容。

既有 `review-r1.md` 声称 PR 改了 `.chanlun/definitions/sanjieduan.md`，`review-r3.md` 又核对 Lean 注释；这些文件均不在当前候选 diff 中。因此旧 APPROVE 不能证明当前 head 的这项验收，当前评审证据与实际候选不一致。不能通过只改 PR 正文或再写一份评审来替代缺失的正本交付。

**返修要求**：在 `.chanlun/definitions/sanjieduan.md` 对应义务段实际加入带 #1669 的覆盖面声明，保留原句及 #1590 订正；声明应与当前 Lean 源一致，至少明确 `stage_rank_monotone` / `earning_no_regress` 只覆盖阶段单调性，资格蕴含由 Rust 锁承担，不把 `LegalEnterEarningStrict` 误称为 `growth_eligible`。若 Lean 注释也属预期交付，应一并恢复；否则应解释旧评审为何引用了不属于本候选的内容。新 head 必须重新独立复核。

## 工程层审查

概念层未通过，不给出整票工程批准。

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0（本审范围） | — |
| HIGH | 1 | **FAIL** |
| MEDIUM | 0（不展开） | — |
| LOW | 0（不展开） | — |

以下事实仅作剩余范围记录，不抵销上述 HIGH：

- 测试确为 12 个种子、每个 61 个图像，逐步检查 `Eligible ⇒ rank ≥ CapitalRecovered`、不进入 `EarningShares`、latch 与事件史单调、存储资格等于唯一谓词重算，并有 Eligible 与“资格失效而 latch 保留”的非真空断言（`tb03c_book_tests.rs:492-543`）。
- `MUTATION.md` 记录的两类变异均由新增测试捕获；未变异基线记为 `tb03c_book_tests` 8/0。既有三轮评审和变异记录已入当前候选。
- 当前生产代码未改，跨方向换向未在本票实现，符合归 #1378 的边界。

## CI 与剩余门

PR CI run `40007954542` 中，除 `rust-test-s-session-extras` 外所列检查均 SUCCESS。extras 原日志为 **222 passed / 3 failed / 9 ignored**，失败名称是：

1. `each_signed_record_window_gates_alone`
2. `opening_batch_negative_controls_are_typed_and_deterministic`
3. `v2_domain_binds_revision_waits_in_e_and_replays`

main run `40005878482`（同为基线 `3ff38accdd`）也失败这三项；因此它们是 main/#1621 的既有基线问题，不是本 PR 新增回归。但票面写明“CI 全绿”，所以仍须记录为独立未满足项，不能写成全套 CI 全绿。

下一步：先补齐正本覆盖面声明并复审新 head；main/#1621 集成修复后再核相关 CI、用户合入批准和关票门。#1669 保持 OPEN。
