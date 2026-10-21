# ADR 0045：系统风险条件清单 SR v1 签署记录（P1-hl-testnet）

> 正本：本文件是具名系统风险条件清单 SR v1（ADR 0030:23 四类）在 `P1-hl-testnet` 上的触发判据、范围与动作签署值的正本。
> 最后裁定：#1560（2026-10-21，用户批复）
> 受影响代码清单：见「下游与受影响票」节（条件检查与冻结态实现归 #1380 TB-06-G）

**日期**：2026-10-21
**裁定人**：用户（2026-10-21 批复 [#1560 评论 6310845841](https://github.com/xy7365527-lang/NewChanlun/issues/1560#issuecomment-6310845841)，原话「不重要你自己定吧」，委托主控按推荐默认值逐项确认）；代理起草，非作者独审
**状态**：已采纳（待 PR 合入）。签署单、考古与独审：`/Volumes/AgentStorage/decisions/1560/{draft.md,archaeology.md,review.attempt4.json}`；落地独审 `.chanlun/review-results/pbatch-land-20261021/`。

## 背景

[ADR 0030](0030-exit-authorization-and-liability-retirement.md) 定了退出授权的三类来源、范围、动作映射表与恢复政策；`:23` 只定 SR 清单 v1 的结构（SR-1 场所/部署者动作、SR-2 账户可用权益低于既有责任、SR-3 TB-09 证据失效、SR-4 用户登记的资金期限），「各条目的具体触发阈值不在本 ADR 给出，随能力验证或生产激活由用户签署清单版本」。签署前系统风险来源不产生任何退出授权；手动急停不依赖本清单（ADR 0030:19）。本 ADR 是第一份签署记录。

原文依据：`026-第26课.md:26`【正文】「任何交易的钱，最好是无限期的，如果真有什么限期，也是足够长的，这是投资中极为关键的一点」；`031-第31课.md:20`【正文】「自己的盘子，一定要自己负责」；`031-第31课.md:26`【正文】「除非证明你买入的理由没有了，技术上出现严重的形态，否则都不能抛一股」。

## 裁定

### 一、沿用（已裁）

- 条目字段照 ADR 0030:18 ⟨id, 触发事实来源, 范围, 动作, 版本, 授权人⟩；每条动作只能是「冻结新承担」或「对声明范围内全部重施同一退出」（ADR 0030:21）。
- 动作表取 ADR 0030:31-36 默认（风险/手动 = 撤范围内全部挂单 + reduce-only 平仓，默认市价/IOC；限价未成不自动升级）；用户改表另记 `action_map_override` 并升版本。
- 场所 API 不可达、断连是失败态，不是系统风险条件（ADR 0030:44），不列入清单。
- SR-3 判据 = [ADR 0028](0028-first-capability-verification-profile.md):46 五项失效触发，动作 = 冻结新发送。P1 是 `experiment` 证据类，按 ADR 0028:46「以 `production` 证据类 profile 启动」的读法，SR-3 在 P1 上恒不触发、P2 激活时生效（照实：ADR 0028:46 后半「任一失效触发后的首次发送前」是否也限 `production` 未明示，此读法为 `[新缠论:推论]`）。
- 授权人 = 用户；SR-2 通知对象 = 用户（ADR 0028:38 触发权归用户，`[新缠论:推论]`），通道归 #1380 实施。

### 二、签署值

| 条目 | 签署值 | 出处与推理链 |
|---|---|---|
| SR-1 触发事实 | 场所对本 profile 所交易标的的单方面动作，三项：(a) 标的下架（公开 meta 中该标的出现 `isDelisted: true` 或从 universe 消失）；(b) 场所暂停或强制结算该标的（该标的持仓被场所清零且回报类型为结算/下架而非强平；字段级判别归 #1380）；(c) 该标的保证金表或最大杠杆变更（meta 中 `marginTableId` 或 `maxLeverage` 变化） | ADR 0030:23 指定来源为 [ADR 0021](0021-venue-selection-deferred.md) 裁定三；ADR 0021:35-40 四行中 `disableDex` 与质押罚没属 builder DEX 部署者，P1 在主 DEX（ADR 0028:19）不适用；(a)(c) 由 Hyperliquid 公开 `meta` 接口读，(b) 由 `clearinghouseState.assetPositions` 与回报读（官方 info 接口快照 2026-10-20T02:54:30Z，`meta` sha256 `aa725f96351e898c95ab7cbc4062208200ea4fa7346478a10f60748e928b82e5`）。从权力表落到可观察字段是 `[新缠论:推论]` |
| SR-1 动作与范围 | 冻结新承担；范围 = 该标的上的全部重；不自动平仓 | (a)(b) 时场所已按标记价结算并撤单（ADR 0021:37），平仓无对象；(c) 不改变买入理由（`031:26`）；要平仓走用户手动来源（ADR 0030:19）。取冻结是 `[新缠论:选择]`，在 ADR 0030:21 允许的两种动作之内 |
| SR-2 口径与阈值 | `marginSummary.accountValue` < Σ既有责任（[ADR 0029](0029-common-cap-contention-policy.md):15 毛锁额口径）即触发；比值 1，不加缓冲；动作 = 冻结新承担 + 通知用户 | 条件名「低于既有责任」本身给出方向与比值 1（ADR 0030:23）；ADR 0029:33 把账户权益定为偿付性核验读数；`withdrawable` 已扣仓位占用保证金，会双重计算；缓冲无先例。映射到 `accountValue` 是 `[新缠论:推论]`。**前提**：`margin_leverage` = 1（已签正本 [ADR 0040](0040-funding-signature-and-chong-derivation.md):41）；改杠杆须同时改签本条责任口径（例如按保证金占用 = 毛锁额 / L），否则 SR-2 会在首笔成交后即冻结且不自解除。代价照实：满仓且 cap≈权益时浮亏 1 USDC 即冻结，解除须另记授权；比值 1 已是最晚触发的取法 |
| SR-4 | v1 不登记资金期限（无生效条目） | `026:26`【正文】；P1 用 testnet 测试币（ADR 0028:30）；期限是用户私人事实，有即改签并同时选到期动作（ADR 0030:21） |
| 绑定 profile | `P1-hl-testnet`；P0 不设 SR 清单；P2 激活时另签新版本 | ADR 0030:23「随能力验证或生产激活」；P0 为确定性回放（ADR 0028:28）；P2 不启用（#1561，ADR 0028「P2 处置」） |
| 有效期 | 签署日 + 365 天 | 已签正本 ADR 0040:45 同批对齐 |

范围编号：正本只定义了 S1 = 该重当前 Voice（ADR 0030:27），S2–S4 无定义；本清单用文字写范围，编号映射交 #1380 实施。

## 签署记录

```sr-list
profile_id: P1-hl-testnet
version: v1
supersedes: none
authorized_at: 2026-10-21
valid_through: 2027-10-21
action_map: ADR 0030:31-36 default
- id: SR-1 source: venue_meta_and_reports(isDelisted|universe_removed; venue_settlement_or_halt; marginTableId|maxLeverage_change) scope: all_chong_on_symbol action: freeze_new version: v1 authorizer: 用户（#1560 2026-10-21 批复）
- id: SR-2 source: clearinghouseState.marginSummary.accountValue<sum_gross_locked_liability(ratio=1,no_buffer,premise:margin_leverage=1) scope: all_chong action: freeze_new version: v1 authorizer: 用户（#1560 2026-10-21 批复）
- id: SR-3 source: ADR0028:46_five_invalidation_triggers(P1_experiment_never_fires) scope: all_chong action: freeze_new_send version: v1 authorizer: 用户（#1560 2026-10-21 批复）
- id: SR-4 status: not_registered version: v1 authorizer: 用户（#1560 2026-10-21 批复）
```

SR-2 的 `freeze_new` 附带通知用户（ADR 0030:23「冻结 + 通知」）。

## 被否方案

- **把 API 不可达、断连列为 SR-1**：ADR 0030:44 判为失败态。
- **SR-1 触发后对该标的全部重自动市价平仓**：作默认被否（结算后无对象、保证金表变更不改变买入理由）；用户可改签为「范围内全部退出」。
- **SR-2 用 `withdrawable` 比较**：双重计算仓位占用保证金。
- **SR-2 加缓冲系数**：无先例，且只会更早冻结。
- **沿用旧 RiskGate 四条（单笔 1%、频率熔断等）**：旧 kill switch「可收编不继承」（`.chanlun/review-results/issue1323-grilling-20261013/1352-AFFECTED-CODE.md:18-19`），不是 ADR 0030 的四类。

## 下游与受影响票

- [#1363](https://github.com/xy7365527-lang/NewChanlun/issues/1363) TB-05、[#1364](https://github.com/xy7365527-lang/NewChanlun/issues/1364) TB-06、[#1367](https://github.com/xy7365527-lang/NewChanlun/issues/1367) TB-08、[#1368](https://github.com/xy7365527-lang/NewChanlun/issues/1368) TB-10：#1560 关票后 blocking 边解除。
- [#1380](https://github.com/xy7365527-lang/NewChanlun/issues/1380) TB-06-G：读本 ADR 版本实现条件检查、冻结态与退出授权写入。
- [#1412](https://github.com/xy7365527-lang/NewChanlun/issues/1412) TB-08-C：P1 急停/冻结序列验收引用本版本。
- ADR 0040 改签 `margin_leverage` 时须同时改签本清单 SR-2。

## 溯源标签

SR-1 动作取冻结为 `[新缠论:选择]`；SR-1 可观察字段、SR-2 映射 `accountValue`、SR-3 在 P1 恒不触发为 `[新缠论:推论]`；SR-4 依 `026:26` `[旧缠论]`；其余为已裁沿用（ADR 0030、ADR 0040）。
