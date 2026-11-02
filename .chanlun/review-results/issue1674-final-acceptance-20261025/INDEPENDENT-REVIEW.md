# #1674 / PR #1684 最终非作者独审

- 候选：`055618db67d07bd418503dbf164078afbf8a2516`
- 基线：`origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：#1674 原 AC、完整四文件 diff、生产调用与测试结构、前序两份 BLOCK 报告、实施/commit 记录、当前 GitHub CI。
- 方法：只读；未在本机运行 cargo/rustc。前序独审已核的同根双跑与 104 次 SIGKILL 证据，在最终 delta 仅为 `#[cfg(test)]` 的前提下复用；当前 CI 再覆盖同一 29 个测试，不重复无改动的重型矩阵。

## 结论

**代码与资金/恢复合同 PASS；整票交付 WARNING。** 前序两项 HIGH 均已闭合，未发现新的 CRITICAL/HIGH 代码问题。当前还不能关票：PR 正文没有票面要求的 Linux 用时且仍是旧提交清单，仓内也缺最终验收报告；extras 的三项 funding 红仍须按 #1621 集成门处理，不能写成 CI 全绿。

## 概念层质询结果：通过

1. **定义回溯**：`tb03c_v2_kill_tests.rs` 的 manifest 为 `tb03c-book/2`，资金出处、`⌈Q/L⌉` 开局批、三 profile 与 `FUNDING-ORACLE.json` 对拍符合 ADR 0040；追加投入与 `q_rebase` 未签时仍是 typed `AwaitingPolicy{funding_revision|q_rebase}`，符合 ADR 0041，未代填默认值。
2. **反例构造**：提交前/后 SIGKILL、旧写者 StaleWriter、提交后原收据、重复重送、异内容恢复、E Pending 两类等待、双进程重放均有明确断言。目标步强制要求前后快照不同，未签 Apply 强制所有域业务表不变，避免负控退化。
3. **推论检验**：`Clock::of_at` 只增加真实日期起始商，S3 的 `Clock::of` 保持原 0 基线；production 文件与冻结夹具均未改。最终 `6848feea22..055618db67` 仅给测试模块增加 `#[cfg(test)]`。
4. **谱系比对**：前序报告中的目录复用与 test-only 模块泄漏均已修复，没有发现新的同构矛盾或宽严两档生产判据。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 1 | 交付材料未闭合 |
| LOW | 0 | note |

结论：**WARNING（代码 PASS，交付材料待补）**。

### 前序 HIGH 复核

- **同根重跑**：`root()` 在外部根下追加纳秒运行号，不删除旧现场。前次非作者审查已核同一 `TB03C_KILL_ROOT` 两轮各 29 passed/1 ignored、每轮 52 次 SIGKILL，共 104 次；当前 final delta 不改该逻辑。
- **非 test 构建泄漏**：`mod.rs` 现为 `#[cfg(test)] mod tb03c_v2_kill_tests;`。当前 `rust-check`、四个 s_session shard、coverage 均 SUCCESS，旧 E0432 已消失。

### AC 对照

| AC | 判定 | 证据 |
|---|---|---|
| 目标步 × before/after × 两次新进程恢复 | PASS | 13 个目标步，含三 profile 开局 Capture/Apply、两类 E Pending Capture 与 5m L2 Apply；`kill_and_recover`/`matrix` 逐轨检查 SIGKILL=9、recover、StaleWriter/原收据、续跑与全表/公开查询对拍。当前 CI 29 个命名测试全 pass，子进程入口 1 个 ignored。 |
| 双跑确定、meta `/2`、E/B audit、黄金值 | PASS | `build_reference` 比较两次完整终态 canonical bytes 与 hash；`golden` 核 meta、资金出处、本金和 typed 等待；`audit_all` 覆盖 E/B。 |
| pause 位移变异负控 | PASS（既有实证） | 实施日志记录两个临时变异 worktree：`before_commit` 挪到 commit 后使 P0 s00/s01/s02 的 before_commit 3/3 红，`after_commit` 挪到 commit 前使对应 3/3 红；结果写入 `625fab529e` commit message。生产 pause 语义在后续 diff 中未改。 |
| Linux 用时写进 PR | **未满足** | commit message 有 19.9–26.5s（矩阵）与 440.6s（当时 `tb03c_` 全集），但当前 PR 正文没有这些读数，且仍列旧提交 `9e66d34ad4/f2d49bfbfe/1df1d0635c`。须更新为当前 head 的交付说明。 |
| CI 全绿 | **未满足，归 #1621 集成门** | 当前 CI 40086376719 仅 extras FAILURE：250 passed/3 failed/10 ignored。三项失败与 main run 40005878482 完全相同：`each_signed_record_window_gates_alone`、`opening_batch_negative_controls_are_typed_and_deterministic`、`v2_domain_binds_revision_waits_in_e_and_replays`。其余 jobs 与 DevSkim 均 SUCCESS。不得宣称全绿，也不得把三项归给 #1674 新实现。 |
| 非作者独审 | 本报告完成 | 代码/合同 PASS；交付材料 WARNING。 |

### 唯一剩余交付缺口（MEDIUM）

PR 描述与仓内证据尚未达到关票可用状态：

1. 更新 PR 正文：当前 head/实际提交面、Linux 用时、pause 变异结果、当前 CI 与三项基线红的真实归属。
2. 关票前把最终验收报告加入 `.chanlun/review-results/`，用仓内相对路径引用；外部 worktree/log 不能替代关票门第 6 子句要求的仓内实体报告。
3. main 合入与工位清理仍走原批准和关票门；本报告不授权合入或关闭 #1674。

## 给主控的下一步

代码返修不再是当前阻断。先补 PR/仓内验收材料，再将本候选与 #1621 的主线 funding 修复一起安排集成验收；若选择先合入，关票声明必须照实保留 main 红及其 #1621 去向，不能写成 CI 全绿。
