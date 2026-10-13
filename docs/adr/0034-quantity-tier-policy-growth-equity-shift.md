# ADR 0034：量档 policy 记录、增长权益与换档权利分拆

**日期**：2026-10-14
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核），走 #1355（FU-02 完整量档与容量经验参数）与 #1357（FU-02 固定 Q 下增长换档）
**状态**：已采纳。本 ADR 只定记录形状、来源门、作用域、转移协议与不变式；不填任何 Q、cap、容量、系数、份额、阈值或有效期数值。

## 背景

ADR 0016 裁定三/四/五与 ADR 0017 裁定二/五/六/七/八已定量层形状：动作量 w(d)×固定 Q、容量与频率分维、空仓不挪钱、撞容量走换档；底档 w≈0.32 只属 1m/δ=0.5/中位/L0→L1，绝对容量只在 L1–L2 干净且 x 未裁。ADR 0031 已定本金批次与 growth_eligible。SPEC #1340 C05.3 要求固定 Q、实际持量、原始义务、增长权益、首次冻结 L 分别记账；C05.6 两类换档事件族；C05.7 转移协议七步。原文：`026:34/:36`、`031:24/:26/:30`、`031:169`（作者答疑）、`035:30`、`038:30`、`040:14/:20`、`049:52/:64/:74/:76`、`053:34`、`079:44`。

## 裁定

### 一、量档 policy 记录（#1355 Q1）
政策 id 族 `#1355/quantity-tier-policy/v1`。记录 ⟨policy_id, version, profile_id, symbol, operation_level, q_ref（引用 ADR 0029 资金授权记录，不复制数值）, tiers[{d≥1, w 整数对 | Unmeasured, source: Measured{probe_run_ref, 粒度, δ, 统计口径, n, 最大月簇, 窗} | UserSigned, tier_quantity 整数版本内冻结, valid_through}], capacities[{level, C | Unknown, source: UserSigned(可引探针), domain{粒度, δ, x, Y, σ 口径}, valid_through}], frequency_evidence_ref, authorizer, authorized_at, valid_through, supersedes⟩。w(0)≡1 不入档；来源域与 profile 不一致即 Unmeasured；0.32 不写入记录，只可作 Measured 来源被引用。

### 二、未定档只阻该深度（#1355 Q2）
`AwaitingPolicy{tier:d}` 只阻深度 d 的短差动作；基线与已定档照常；不以 0.32/1.0 代填。

### 三、更新即新版本，命令投影时冻结（#1355 Q3）
触发：新探针读数（满足 ADR 0022 裁定七窗口纪律并带不确定性带）、用户签署、到期回 Unmeasured。版本对生效后投影的命令起作用；在途与已冻结 L 不重测；回本前同量回补按原卖出冻结的版本与数量；旧版本不删，AsKnown 不回填。

### 四、容量须用户签署；Q_total 只来自资金授权（#1355 Q4）
探针容量是依据不是裁定（x/Y/σ 口径未裁）。Q ≤ C(ℓ) 在开重与换档激活时校验；C(ℓ) Unknown ⟹ `AwaitingPolicy{capacity}`，未知不是无限；溢出部分按 ADR 0016 裁定五(c) 归上一级，账目走裁定六。频率门独立。取整 `tier_quantity = ⌊Q·num/den⌋`（quantum 单位），残量不累积，整数运算。

### 五、固定 Q 与增长权益分账（#1357 Q1）
五账分列：Q（固定，动作基）、q_actual、原始义务（同量回补配对）、G 增长权益 `{units_g, cash_g, cost_g}`、L。G 的来源只有回本后资本化 A、回本后抛出再接回多出的筹码（`031:169`）、留存的现金分红净额；空头重只有 cash_g。Q 只能由 `q_rebase` 政策版本改写，前件 `growth_eligible ∧ 新 Q ≤ 已签 C(ℓ)`。G 不是本金（ADR 0031 裁定一）。

### 六、资金溢出换档：触发、分拆、转移（#1357 Q2/Q3）
触发量 = `Q + units_g`（或资金口径 `q_ref + cash_g`，开重时冻结口径）对已签 C(ℓ)；市值不触发。超出部分 = 分拆量，只从 G 取（先 cash_g 后 units_g），Q 与 P 不动。容量未签署 ⟹ 不触发，G 留重内。
`Transfer{transfer_id, kind ∈ {capital_overflow, market_regime}, source_chong, target, payload{cash, units, carrying_cost}, trigger_evidence_ref, authorization_ref, policy_versions, state}`。目标新重：`principal_contribution{source: transfer}`（amount = cash + carrying_cost）形成 P0；K0 = carrying_cost；units_in = units；Q_new 激活时按目标级量档记录冻结；阶段由 `growth_eligible` 计算，不特判。不变式：I1 守恒、I2 不复制、I3 不挪用闲置 Q（溢出来源只 G）、I4 尾责不迁、I5 唯一激活。多接收者逐份一个 transfer_id。

### 七、行情换档与资金溢出换档分与合（#1357 Q4）
共用裁定六的协议与账目。资金溢出：触发证据 = 量档版本 + G 跨线原件，分拆量确定性。行情换档：触发证据 = 结构事实引用（`040:14`），分拆量由用户签署，可含 P/K/Q 份额按账面价值整数分摊、残量留源；不得动回本前同量回补的在途配对与在途订单。两族互不冒充；原重持续经营。

### 八、失败恢复与验证（#1357 Q5）
状态 `SourceLocked → Escrowed → SourceDebited → TargetPendingActivation → ReceiptsBoth → Activated` 各自持久；崩溃按原 transfer_id 续做；目标未确认不准源退款、无时钟超时；取消 = 显式授权逆事件原路退回 G；TB-10 迁移原样搬运中间态。测试锁：守恒账例 W1–W5（现金溢出 / 零成本筹码溢出生而过闸 / 跨两级两份 / 目标未签档托管等待后取消 / 行情换档含 P 份额）前后逐字节核 I1–I5；双进程相同；六个中断边界恢复与不中断逐位相同。

## 被否方案
常数表 `depth_weights`（资金帽语义已废）；运行时现算（伪造默认）；探针容量自动准入（x 未裁）；按市值触发溢出；增长自动并入 Q；G 记成本金；新重阶段特判；目标超时退款。

## 数值口径
全部整数：w 为整数对，tier_quantity/units/cash/carrying_cost 为规范十进制整数文本解析的 i64，`checked_*`；不出现浮点与除法结果字段。

## 受影响代码清单
见 `.chanlun/review-results/issue1323-grilling-20261013/1355-AFFECTED-CODE.md` 与 `1357-AFFECTED-CODE.md`（基线 main 3da0b2c8，FIN 0d8a6017）。

## 下游票
#1444 TB-06-C、#1446 TB-06-F、#1445 TB-06-E、#1402 TB-10-H、#1406、#1394 FIN（quantity_policy_id 引用、transfer_escrow 类目）、TB-10（depth_weights/depth_weight/oscillation /3 旧入口登记）。

## 溯源标签
裁定五前半（固定 Q 与筹码变多分属两账）`[旧缠论]`（`031:24/:26/:30`、`031:169`）；其余全部 `[新缠论:选择]`。
