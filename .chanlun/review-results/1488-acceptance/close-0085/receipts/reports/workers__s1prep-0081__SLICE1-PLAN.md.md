# 收据 workers/s1prep-0081/SLICE1-PLAN.md

- 原件：`workers/s1prep-0081/SLICE1-PLAN.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`eb8f1adc9fd96af940c08b9a00c71218f9a941ebddc23bed80c982b0d1a8246d`
- 字节数：42371；行数：348

## 关键行（行号: 原文）

```
1: # #1612 切片 1 准备计划（s1prep-0081，只读设计）
9: ## 0. 先说结论：批复给切片 1 带来的三处实质张力
15:    - 冷根失败重跑之后（0079 的 P24-adj）。
24:    这部分属于 #1481 方案三，不在 #1612 范围内。增量 g28 用 3750 ms，完整用 2658 ms（accept-s4）。切片 1 的性能读数只能当基线和回归门，不能当达标门。
26: 因此切片 1 能证明的是：增量路径在生产构建下**正确**（与冻结基线逐位一致、双跑确定）、**可降级**、**可测**、**可版本回退**。它证明不了“快”，也做不到“全程无完整分支”。后者要靠切片 2（持久态重建出生、只丢保留的降级）完成，并在切片 5 用“生产入口调用图不可达完整重算分支”锁住。下文按这个口径来定范围，相关取舍集中列在 §5。
30: ## 1. 切片 1 精确范围
32: ### 1.1 升格开关与生产 feature 形态
52: ### 1.2 资源档修订 `s-structure-resources/2`
82: ### 1.3 恢复口径（切片 1 版）
96: ### 1.4 降级（v4 L13）进生产
101:    - 现状：`publication_degrade_verdict`（`audit_owner.rs:6087-6140`）在闭包返回 `Err(String)` 时，靠 `trace.bridges(&e)`（`scratch.rs:591-596`）对渲染文本做 `contains`。0077 的反例 N57_no_bridges / N57_typed_count_only 证明：只看计数无法区分 F1 变体。
108:    - 锁：0072 的 7 条降级锁全部保留；`publication_degrade_verdict_only_for_pure_budget_shortfall` 真值表改为构造类型化错误；N57 两个反例变体加入负控（必须变红）。
112:    - **推荐**：切片 1 保持现形态进生产，按 §1.3 记为“出生步”的一种入口，读数单独标记；切片 2 结合 W_g 和保留帽改为“只丢保留、不重生成”（帽在写前判定，或提交后裁剪保留）。理由：现形态已经在 S86/MIN12 上由 0072 和 0079 实证逐位一致；改形态需要重做预算估算链（`audit_owner.rs:3097-3140` 的 `limit/8`、`s_content.rs:3268-3310`），属于切片 2 的资源合同范围。
116: ### 1.5 生产资格 ADR 要点
121: - `## 迁移开关`：记 D1 的结论，推荐编译期 `s_incremental`、不设运行时开关、运行中不翻转、切片 5 同提交删除 feature 与完整路径。回退＝换回上一版本二进制和对应版本的资源档。可回退的前提是落库逐位相同（#804），P6 之后需要台账可丢弃论证。
136: ## 2. 写集、去门控清单与评审面
138: ### 2.1 写集（文件:行区间，以 `2c2d467e26` 为准）
151: | W10 | `audit_owner.rs:5893-6140`（`renew_lease_after_abandon`、`note_cold_root_rerun`、`renew_after_cold_root_rerun`、`publish_attempt`、`publication_degrade_verdict`）、`:6187-` 真值表测试 | 去门控 + **P57 类型化** | 降级 / P57 |
167: ### 2.2 去门控清单与评审面估计
197: ### 2.3 “生产构建不含测试钩子”编译期检查
213: 4. **字节级不变门（保留 #1488 纪律）**：切片 1 合入前后，**`s_session` 默认构建**（完整路径）的 release 二进制去 build-id 并 strip-all 后的 sha256 应当不变，沿用 fix-0072 §1 的方法。W4 的 `/1` 路径也不动。〔推断〕如果 cfg 改写不可避免地改变了门外行号（panic Location），就改为记录 diff 并给出逐处解释。
217: ## 3. 验收设计
219: ### 3.1 正确性：增量 ≡ 冻结基线（逐位）
225: | A1 | S86（86 代） | `s_session,s_incremental` release，经 writer_cli serve 正式入口，`/2` 档 `incremental_retention_bytes=268,435,456` | 对 BASELINES 中的 S86 r8（main 6badaa6049 双跑摘要）：INVARIANT 全部 EQUAL，包括 16 张表、公开根、可达闭包 344,419、输入 sha、`final_snapshot_root`；逐代回执相等。CONDITIONAL 差异（`s_content_nodes` 全集、binary_sha2
226: | A2 | MIN12（52 代） | 同上 | **`MIN12@256MiB 52/52`**（票面字面）Committed；对 BASELINES 的 main v3 臂 oracle：回执、digest、counts、structure_cut、rule_revision、rows_sha256 逐代相等。accept-s2 v2 oracle 只列出，标“非判据”（0065 §7） | `compare_arms` |
232: | A8 | P57 负控 | N57_no_bridges、N57_typed_count_only（0077 反例）以及 0072 的 a–f 负控 | 都变红；HEAD 全绿 | 悬空 ref |
234: | A10 | 默认构建不变 | §2.3 第 4 层 | sha256 相同 | prod.sh |
236: ### 3.2 增量双跑确定性
238: - **D-双进程**：同一个 `s_incremental` 二进制，两个独立进程、两个空库，相同输入流和 clock plan。逐代公开根、回执、对象摘要清单逐位相等；WAL checkpoint 后最终库 sha256 相等。在 S86、MIN12、G_L 前缀上各做一次。
243: ### 3.3 单事件延迟与吞吐测量协议
277: - 二进制记录构建 SHA、rustc 版本、features、sha256。
281: ### 3.4 性能门槛候选（交主控/用户定，〔需裁 D5〕）
295: ## 4. 风险与先后
297: ### 4.1 接口
301: | #1488 | 切片 1 blocked by #1488（第三稿边）。切片 1 的代码基＝#1488 合并面（今天头 `2c2d467e26`）；降级、P3、P24-adj 都在其中 | #1488 合入 main 后，从 main 开分支。若要提前开工，可从 `2c2d467e26` 开 stacked 分支，但合入顺序不变 |
310: ### 4.2 风险
320: ### 4.3 最快在生产构建上跑通的先后
337: ## 5. 需裁点汇总
347: | D7 | 长前缀范围 | 切片 1 做到 g ≤ min(G_L, 200)，全 G_L 留切片 5 | 切片 1 跑满 G_L |
```
