# ADR 0036：经营统计样本构造、边界规则与闸门后参数触发协议

**日期**：2026-10-14
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核），走 #1356（FU-05 样本与边界）与 #1358（FU-05 首次触闸后参数）
**状态**：已采纳。本 ADR 不改 ADR 0022 已裁统计主干的任何数字，不预编闸门后任何参数值；只定样本构造、边界处理、输出分类、触达事件与参数版本协议。

## 背景
ADR 0015 裁定二分段判据（闸门前成本下降速率 / 闸门后净头寸价值 / 全程频率）；ADR 0022 定闸门前主干（事件口径、费后 ≤0、按月 block bootstrap 单边 95% 下界、n≥30、月块≥12、≥3 窗各过线且 ≥50% 覆盖、频率不合成、闸门后参数暂缺+触发条款）；ADR 0031 定 growth_eligible 与 nonpositive_base 零分母；ADR 0033 定批内顺序 Unknown。原文：`049:74`、`031:24/:30`、`035:32`【正文】，`031:169`【作者答疑】。

## 裁定（#1356）
### 一、样本全集与排除闭集
样本 = 该重所有已 Applied 且改变 K 的经济原件，按类型入样（成交含亏损、费用含 fee_only/funding/borrow/代扣税、分红净额、更正净效应另列），与符号无关。排除闭集：`out_of_domain:post_gate`、`not_applied`、`recomputed`、`zero_base_after_merge`、`terminated_chong`。对账锁 `完整事件账 = 样本 + Σ排除` 逐重逐月，报告列各排除原因计数与金额。
### 二、采样单位 = (重, S 稳定点)
同一稳定点内对该重应用的全部 K 变动合成一个「操作」样本（批内顺序不存在）；迟到费用归到达的稳定点，不回填。样本 `{chong_id, stable_point, C_before, C_after, delta, components[], duration}`，相对降幅只在报告层以 typed 有理数出现。
### 三、零时长
合并后仅退化时钟一种来源 ⟹ `NotComputable{zero_duration}`；降幅仍入降幅表，不入速率表；不设 ε。
### 四、跨闸
正向：闸门前 `delta_in_domain = C_before`（相对降幅 = 1）、标 `gate_crossing`，`−C_after` 作闸门后序列首条；反向：按当时 `C_before ≤ 0` 归 post_gate 域外，其后回域取真实 C_before。K 不 clamp。
### 五、多空统一 cost_remaining
多头 = K；空头 = Σ M_b − 费后累计已实现利润；闸门处同为 0，同一函数。
### 六、分母、统计单位、四类输出
C_before 只从该重 book 同源读；统计单位 = 重，汇总只作报告；输出互斥：`Formal{lb, n, months, windows, method}` / `NotComputable{reason}` / `InsufficientEvidence{...}` / `AwaitingPolicy{费率簿}`；更正以 RecomputedWithRevision 另列；频率买/卖分列随报告。

## 裁定（#1358；激活条件未触发，只定容器与协议）
### 七、GateReachedEvent
在 experiment/production 证据类 profile 下，Applied 真实原件使 `growth_eligible` 首次为真，B/E 同源 AsKnown 记录 `{chong_id, profile_id, evidence_class, known_at, stable_point, K, P, P_recovered | ΣM_b 与 Π, triggering_fact_refs, policy_versions, verified_by}`；触发原件未全部 Applied / 有 Pending 覆盖 / 对账不一致 ⟹ `GateReachedCandidate`。TestOnly 与历史双跑 ⟹ `SimulatedGateReached`，可演练不激活。
### 八、触达后、启用前
只阻闸门后正式评价输出（`AwaitingPolicy{post_gate_params}`）与阶段能力首次启用；`growth_enabled = growth_eligible ∧ 参数版本已落盘 ∧ 启用记录`，事实与许可分列。闸门前统计、事实账、G 累积、既有 Voice 经营、结构发布不受阻。
### 九、PostGatePolicy 槽位（全部 Unset 直到用户签署）
`net_position_value.{window, benchmark_ref, reading(固定=ADR 0022 下界), pass_rule(下界与基准的序关系)}`、`frequency.min_evidence_ref`（独立门）、`growth_enable.{conditions, rollback}`（K 回正为固定回退项）、`shift_threshold_ref`（只引用 #1355 容量）。
### 十、时序与有效域
`触达.known_at < 版本.authorized_at < 首次启用.known_at`，三者持久互引；无版本不得启用；早于触达的版本拒收；改参数开新版本只对之后评价窗生效；有效域随签署时 profile，experiment 版本不自动迁移到 production；TB-11 关图须出示三条记录。

## 被否方案
只收成本下降事件；按原件逐条或按自然日聚合；零时长加 ε；跨闸整条入闸门前或整条剔除；空头用开空均价作基价；汇总作门；现在给闸门后任何槽位默认值；人工判定触达。

## 受影响代码清单
见 `.chanlun/review-results/issue1323-grilling-20261013/1356-AFFECTED-CODE.md` 与 `1358-AFFECTED-CODE.md`（基线 main 3da0b2c8）。

## 下游票
#1398 TB-06-I、#1410 TB-06-J、#1366 TB-07、#1446 TB-06-F、#1406、#1403 TB-11-B。

## 溯源标签
采样单位「每次操作」与闸门两侧两域 `[旧缠论]`（`049:74`、`031:24`、`031:169`）；其余 `[新缠论:选择]`。
