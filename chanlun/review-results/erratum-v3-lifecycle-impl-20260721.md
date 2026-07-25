# 勘误登记：v3-lifecycle-statemachine-impl-20260720 §2③「confirm_times Some→None 可观察面」

- 日期：2026-07-21
- 性质：**勘误（erratum）**。被勘文档 `chanlun/review-results/v3-lifecycle-statemachine-impl-20260720.md` 按不可篡改纪律原样保留，零改动；本登记为其修订指针。
- 发现者：#64 实装工位（agent-69），证据已写入 `rust/src/theta_v0/classifier/nest_lifecycle.rs:117-122` 模块头。
- 评审确认：`chanlun/review-results/code-review-v1-v3-64-20260721.md` Spec 轴 (a)1 同判（「实装 T1 用合成事件直接喂 divergence_confirmed 翻转，绕过真实验证」）。

## 被勘论断

v3-lifecycle-statemachine-impl-20260720.md §2③：trend 域 ForceOvertake（反超证伪）的可观察面 = provider `confirm_times` 的 **Some→None 翻转**（即同一身份先产确认时点、后被终假收回）。

## 勘正

**trend 域真实 ForceOvertake 不可达。** `trend_confirm_time` 扫描在首个 T2∧T5-OR 同真点即返回 t*，proxy 单调 ⟹ 同一身份内 `confirm_times` 的 Some 前缀稳定，Some→None 在真实 provider 事件流上不可达。后果：

- 真实 ForceOvertake 只能落在 **pan 活窗通道**（`provide_pan_live_windows` + NestLifecycleBook advance 反超分支，T11 `t11_real_provider_pan_live_force_overtake_auditable` 已实证）。
- trend 域的 T1 合成反超路径保留为合成测试，不代表真实可达性。
- 后续引用 v3 实装报告 §2③ 处，以本勘误 + nest_lifecycle.rs:117-122 模块头为准。

## 关联

- V3 卡 `v3-lifecycle-statemachine-implementation-card-20260720.md` §7 T1「判据源 = level_view.rs:538-539 终假路径」的验收表述同步受影响：trend 终假路径存在但不构成 Some→None 可观察面。
- 裁定 #64（`r43-lifecycle-ruling-20260721.md`）不受影响：其四问裁定不依赖 trend 域反超可达性。
