# C2 遗留裁决材料（task #71 后续）

- 日期：2026-07-14　状态：**READY——原文依据已补（D3/D7），供裁决；本文不执行裁决**
- 来源：`chanlun/review-results/c2-production-replay-20260714.md` §6–§7；`chanlun/review-results/c2-architecture-reassessment-20260713.md` §12；`chanlun/review-results/p55-recursion-consistency-20260713.md` §5.3；assembler-spec-20260712.md:156-164
- 纪律：不修改生产主线、不修改既有裁决与谱系；语义类决策点（D3/D7）原文依据未补前不应结裁

## 裁决顺序与依赖

```
D6（立即，阻塞 /tmp 清理）
D4（确认型，事实已判）
D3（关键路径，需原文）──→ D2 ──→ D1 ──→ D7（#56 重开复核）
D5（并行工程排期）
```

## D1 投影口径（replay §6.1）

- 问题：当前 HEAD（`d01d9b73ed` 起）已非旧 exact-three 塔；生产 seam 诚实 fail-closed。要在当前扩展塔上生产 CompletedMove，必须显式定义"扩展窗口 → immutable exact-three seed"投影，不能暗设。
- 选项 A：定义显式 projection，版本化进 version tuple，seam 对未投影输入维持 fail-closed。
- 选项 B：暂不定义，生产层继续 fail-closed 不产 CompletedMove，直至塔口径统一。
- 推荐：默认 B 维持现状；A 是解锁生产的必要条件，建议在 D3 裁决后一并做（同批进 version tuple）。
- 证据锚：replay §2.1/§6.1；`level_view.rs:161-228`（**仅存于 c71-replay-work worktree，见 D6**）。

## D2 A/C 自动配对 hook（replay §6.2）

- 问题：无显式 divergence pair provider 时 Trend 只能 `Pending(MissingDivergencePair)`（assembler-spec-20260712.md:156-164）。
- 选项 A：实现自动配对 provider——**依赖 D3 先定方向真值**。
- 选项 B：维持显式注入 + Pending fail-closed。
- 推荐：D3 未裁前维持 B；裁后按 D3 结果实现 A。

## D3 方向真值（replay §6.3）【语义——原文依据待补】

- 问题：四个方向 provider 已独立版本化并列重放（version tuple 字段 `direction_provider_version, divergence_pair_provider_version`，replay :58；三方向原始锚 `/private/tmp/c71-c58-exact-c2-asof.log:1-8`，SHA-256 `231a1fff0c5b1fcfc50a6f6c5be700e1c674c442bc810c6fb206f9da56774308`）。本任务未升级任何一项为唯一方向定义。
- 待裁：唯一方向定义（或优先级序）+ 写入 version tuple 的方式。
- 风险：按实现偏好裁语义 = 静默替换理论口径，C2 合格域漂移且难以回溯。
- 原文依据（已精读 0011、0010，2026-07-14）：
  - `chan99/0011:5,9`——走势分盘整/趋势；趋势方向 = 依次同向的同级别中枢的排列方向。方向是走势类型级属性，非段几何属性。
  - `chan99/0011:7,23`——盘整只含一个中枢、无法形成两个同向中枢 ⇒ 盘整在趋势意义上**无方向**；合规 provider 输出域必须为 {上, 下, None}。
  - `chan99/0010:17` 走势中枢中心定理二——可操作判据：后 GG<前 DD ⟺ 下跌及延续；后 DD>前 GG ⟺ 上涨及延续；区间重叠情形 ⟺ 形成高级别中枢（同级方向不存在）。
  - 筛选结论：唯一方向定义应实现"相邻同级别中枢 GG/DD 比较"判据；凡由单段斜率/端点几何推方向、或无法输出 None 的 provider，不合原文口径。
- 剩余待裁：四 provider 与上述判据的映射核对（哪个实现了它）——对照存档 `chanlun/archive/c71-seam-20260714/level_view.rs`。

## D4 PartitionPolicy 晋级（replay §6.4 + reassessment §12.2）

- 事实：仅 `GREEDY_V1` 可用；晋级门 P1–P5 已冻结（P1 零回退、P2 Completed 不降级、P3 全局 U 净减 ≥25%、P4 全边界 prefix equality、P5 历史 as_of 不变）。参照 `U_global=2,028`，P3 ⇒ 净减 ≥507；#64 候选净减 27 / 回退 524，**明显不达门**。X1 触发线已超（2,028/55,624=3.645908%）但 P2/#65 未完成，不能宣布触发完成。
- 推荐：确认维持 `GREEDY_V1`，候选不晋级；shadow gate 保持现状。此项为确认型裁决，无新增调研需求。

## D5 事件持久化 adapter（replay §6.5）

- 问题：seam 内 append-only/CompletedFreeze 已实现；尚未接项目正式 event store、缓存键、migration/reopen reducer。
- 性质：工程排期，非语义裁决。推荐列入任务队列，与 D1 实现同批或其后。

## D6 材料与 seam 代码归档（replay §6.6）【紧急度已升高】

- **新核实事实（2026-07-14）**：`level_view.rs`（C2 生产 seam 唯一实现）不在主线 `rust/src/theta_v0/classifier/`，唯一副本在 `/private/tmp/c71-replay-work` worktree。原计划的 /tmp 清理会销毁该证据与四份必读材料的只读恢复副本。
- 选项 A：归档进权威链（专用分支或 `chanlun/archive/` + SHA-256 清单）。
- 选项 B：最小动作——拷贝存档到 `chanlun/archive/c71-seam-20260714/` 并记 SHA，主线不接线。
- 选项 C：不归档，直接清理（证据即失，D1/D7 复核将无法引用行号锚）。
- 推荐：**先 B 再清理 /tmp**；是否升级为 A 由 D1 裁决结果决定。
- 阻塞：/tmp 清理命令在 D6 执行前不得运行。

## D7 firstRetrace 口径 / #56 例2（replay §6.7 + §7）【语义——原文依据待补】

- 现状：维持 p55 §5.3 "否/未进入严格 C2 合格域"倾向；**撤回**其"history-end 对象按 end 截断即可代表 as-of"的证据方法。#56 例2 继续挂起。
- 澄清：现"否"为资格性（NotEligible/Pending/Unassigned——as-of 时点目标 retest CompletedMove 未冻结、exact-three 主树无同 ID pair），非几何性（重放几何为 SUCCESS）。几何 SUCCESS 不能推翻 firstness/identity 要求，Pending/Unassigned 也不能重述为"回试失败"。
- 待裁："第一次回试"的定义边界（对象宇宙、identity、as-of 计数口径）；重开复核先决条件 = D1 投影 + D2 hook + D3 provider 全部版本化。
- 原文依据（已精读 0010，2026-07-14）：
  - `chan99/0010:13` 走势中枢定理三——"回抽"的定义对象：一个次级别走势离开中枢后，**其后的次级别回抽走势**；终结判据 = 回抽不重新回到中枢区间。
  - ⇒ firstRetrace 计数口径应锚定在（同级别中枢, 次级别离开段）对上：离开段之后的首个次级别回抽即"第一次回试"；identity 属于该对，不属于全局几何。
  - `chan99/0010:44`——定理三的组合仅三种（趋势+盘整 / 趋势+反趋势 / 盘整+反趋势）⇒ 回抽的走势类型不限定为反向趋势，盘整回抽同样合法。
  - `chan99/0010:27`——中枢由**完成的**前三个次级别走势类型构成 ⇒ 完成性口径贯穿；支持 replay 的资格性"否"：目标回试 CompletedMove 未冻结时不可计数，几何 SUCCESS 不能替代完成性。
  - 补充篇目（可选增强，非必需）：`0025`、`0027`、`0016`。
- 剩余待裁：#54 两 pair 的 identity 映射到（中枢, 离开段）对后是否为"第一"——需 D1 投影 + D3 provider 定版后重开复核（同 replay §7）。

## 下一步

1. ~~D6 先行执行（存档 + SHA）~~ **已完成 2026-07-14**：`chanlun/archive/c71-seam-20260714/`（MANIFEST.sha256 与 replay :135 锚哈希核验一致）；/tmp 清理已解除阻塞。
2. ~~原文精读补 D3/D7 两栏~~ **已完成 2026-07-14**（0011 + 0010 核心锚；0014 渲染受限跳过，0016/0025/0027 列为可选增强）。
3. 待办：四 provider ↔ GG/DD 判据映射核对（对照存档 level_view.rs）；完成后 D3 可结裁。
4. 本文已升 READY，等待裁决人对 D1–D7 逐项结裁。
