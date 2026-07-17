# 裁定草案：nest 证书接入买卖点判定路径的方式（task #101，DRAFT 待主人裁）

日期：2026-07-17　依据：#100 全量对账（chanlun/review-results/nest-cert-bsp-recon-20260717.md）
物化工具：p101_cert_bsp_tag（src/bin/p101_cert_bsp_tag.rs，只读探针）

## 背景（已证事实）

- 两口径证书（A=46 / B=45）在 1440 bar 窗口内 **100%** 有同侧 BSP；过半精确同 bar（median|dt|=0）。
- 反向覆盖：±240 bar 仅 0.17%、±1440 仅 0.97% 的 BSP 附近有证书——证书密度比 BSP 低约 300 倍。
- 结论前提：证书是"极小的高置信子集"，落点与现有买卖点体系一致、无系统性错位。

## 条款（DRAFT）

1. **接入方式 = sidecar 打标**。证书不生成独立买卖信号、不进入判定主路径；
   只作为已确认 BSP 的加权/过滤标注（下游仓位/风控可用，判据层不可见）。
2. **绑定规则**：cert 钟取 `judge_at` 身份钟表的 max（judge_max）；
   绑最近**同侧** BSP 确认 bar；|dt| 相等时 forward（bsp_bar ≥ judge_max）胜出
   ——与 p100/p101 `nearest_dt` 逐字同规则，消歧确定。
3. **强度分档**：|dt| ≤ 240 → strong；240 < |dt| ≤ 1440 → weak；
   \>1440 → 不绑定（当前 0 例；一旦出现即 escalate，不得静默丢弃）。
4. **时钟纪律**：绑定与打标只允许用 judge_max 与 BSP 确认 bar；
   `created_at` 与任何前视字段禁入（与 p92 `created_at=forbidden` 同律）。
5. **层级**：nest exec/top 与 classifier lvl 不做硬映射；`near_lvl` 仅作归因报告字段。
6. **打标不回改**：打标是 append-only 的事后标注；BSP 的产生、确认、撤销一律不受影响。
7. **同 BSP 多证书合并**：同一 (bsp_bar, side) 收到多张证书时，打标语义为布尔（有/无）
   ＋强度取各证书最强档；证书张数仅作归因字段，**不得**按张数线性放大权重。
   实证：末端同一顶层事件（2:4613084）与 39 个不同基例组合出 39 张证书、
   同 bar 判定（judge_max=4613104, dt=0）并绑到同一 BSP——张数是链组合的产物，非独立证据。
8. **稳定主键 = exec-base 判定事件**（#103 双测附注）：中段截断重放显示 top 链身份
   可在快照间迁移（3:725489→3:689893，pan→mixed，base 判定不变）；打标/发布的
   幂等主键取 (caliber, exec, base 事件, judge_max, side)，top 链身份仅作归因字段。

## 开放条款（不在本裁定内强行定）

- B 口径 Long 侧仅 11 张（#102 未归因）。归因落地前，**不建议**把打标用于方向不对称的
  策略权重（避免把口径缺陷固化成方向偏置）。
- strong/weak 的具体权重数值属策略层参数，不属判据裁定范围。

## 物化校验（p101，全量 BTC 4,613,599 bar）

- P101_SUMMARY：certs=91 **bound=91 unbound=0 ties=0** distinct_bsp=31 multi_bsp=22。
- 强度分档：A strong=39 / weak=7；B strong=39 / weak=6（与 #100 的 |dt|≤240 = 39/39 一致）。
- BSP 侧：events_total=28,417（buys=14,762 / sells=13,655），与 #100 逐字一致。
- 完备性通过（unbound=0）；无 tie（forward 消歧条款未被触发，规则保留备用）；
  multi_bsp=22 中最大簇为末端 bar=4613104 的 39 张（见条款 7）。

## 待主人裁

1. 条款 1 的"判据层不可见"是否升格为硬约束（代码层禁止判据模块 import 打标产物）？
2. 强绑定阈值 240 bar（4 小时）是否合适，还是改为按级别自适应？
