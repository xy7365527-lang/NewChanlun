# ADR 0028：首个能力验证 profile——Hyperliquid testnet 主 DEX、可下单不可划转、净额、TB-09 双层重核

**日期**：2026-10-13
**裁定人**：代理依用户 2026-10-13 授权裁定（分析：claude-fable-5-1；根逐条复核并补充两条），走 [[grilling] 首个具名场所账户与验证权限 profile #1351](https://github.com/xy7365527-lang/NewChanlun/issues/1351)（#1323 / SPEC #1340，FU-04）
**状态**：已采纳。本 ADR 只定首个能力验证的范围与 profile 字段，不选定生产场所，不授权任何真实下单、划转或强平实验，不记任何凭据值。

## 背景

SPEC #1340 TB-08 的前置是「场所与验证权限」，TB-09 的前置是「目标部署身份、凭据及权限明确」（`SPEC.md:367-369`）；`SPEC.md:372` 已定独立实验 profile 不替代生产，生产首次新发送前须重核 TB-09。[ADR 0021](0021-venue-selection-deferred.md) 裁定一暂不选定生产场所，裁定二倾向 Hyperliquid 但隔离零实测；[#942](https://github.com/xy7365527-lang/NewChanlun/issues/942) 第 1/3/4/5 问因凭证约束未做，第 4 问用户裁「乙」。[#1319 2026-08-31 收口评论](https://github.com/xy7365527-lang/NewChanlun/issues/1319#issuecomment-5481697997)只读查到编排者主钱包在 testnet 的 accountValue = 999.0 USDC，唯一硬卡点是运行环境无私钥；ADR 0021 所记 faucet 阻塞针对的是全新钱包，对该地址不再成立。

## 裁定

### 一、首个能力验证范围 = profile `P1-hl-testnet` v1

| 字段 | 值 |
|---|---|
| environment | testnet |
| venue | hyperliquid |
| dex | Hyperliquid **主 DEX**（加密永续）。`xyz` 等 builder DEX 上的美股永续不在 v1，作为同一 profile 的 v2 另起，不并入 v1 的证据射程 |
| account | 编排者主钱包地址在 testnet 侧对应的账户（地址不是凭据） |
| endpoint | `api.hyperliquid-testnet.xyz`（info / exchange / ws） |
| credential_scope | `trade_no_transfer`；来源 = 用户自备、与主钱包分离的 API wallet |
| execution_form | Netting |
| allowed_actions | 查询、下单、撤单、reduce-only 平仓；不含划转、提现、子账户创建 |
| evidence_class | experiment |
| authorization | 授权人 / 时间 / 授权记录 id（由用户填） |

任一字段变更即新版本；`environment` 与 `execution_form` 不得在版本内切换。`P0`（确定性回放与受控模拟，TB-04/TB-07 已承担）与 `P2`（目标生产账户，命名与授权另票）是并列槽位。

**前置读数（根补充①）**：TB-08 认领时，先以公开 info 接口只读重读该账户的 testnet 余额。读不到可用余额，则本次能力验证退回 `P0` 的范围（确定性回放 + 受控模拟），不得为此在主网存款。

**P1 证据射程**：能证明身份与 endpoint 可达、幂等/查询/回报契约、下单/撤单/部分成交/拒单的回报形状、Netting 下持仓回读、急停动作序列可执行。**不能证明**：各层强平互不牵连（ADR 0021 裁定二的降级原样保留）、强平行为本身、子账户行为（含子账户在 `xyz` 下单，#942 第 1 问；#942 的处分不变）、builder DEX 上的任何行为（根补充②）、abstraction 默认值（第 2 问未决）、主网清算逻辑与 testnet 一致、生产部署的 X 排他与旧 writer 禁入。

### 二、凭据作用域与急停权限

作用域取「可下单、不可划转」。载体为专供 testnet 的 API wallet，由用户在 TB-08 开工时自备；主钱包私钥不进任何运行时（testnet 与 mainnet 共用地址与私钥，注入主钱包等于事实上授予主网权限）。凭据只记作用域与来源，不入仓、不贴票。

急停 = 撤全部挂单 + reduce-only 平仓，是一次退出授权：触发权归用户手动与 [ADR 0030](0030-exit-authorization-and-liability-retirement.md) 的具名系统风险条件清单，执行权唯一归 X 并经正式 command 进入（[ADR 0027](0027-classification-chong-operating-program-ownership.md) 裁定四）；代理会话与自动化无自主急停权。用户在场所网页端的场外动作是外部事实，系统按回报记账，不对冲回原仓位。

### 三、执行形态

P1 按 Netting 验证（[ADR 0014](0014-intra-chong-unidirectional-inter-chong-no-arbitration.md) 裁定二、[ADR 0024](0024-total-ledger-convention.md) 裁定六）。逐腿（档 B）须另起版本或另一 profile；毛账照 ADR 0024 裁定二继续作归因与杠杆暴露口径。P1 的回报按 [ADR 0032](0032-fill-allocation-policy.md) 的 `netted` 投影分配；拿 P1 与历史回放对拍时，回放须用同一 `projection_kind`（[ADR 0033](0033-history-simulation-bar-batch-model.md) 裁定四）。

### 四、TB-09 重核：双层

**机器层**：正式 launcher 在每次以 `production` 证据类 profile 启动、以及任一失效触发后的首次发送前，核验 TB-09 证据指纹（旧 writer 禁入证据、X 排他锁、profile 身份绑定、凭据作用域版本），与 profile 版本一起持久化；失败即拒绝发送，不回退旧 writer。失效触发：部署身份变更、凭据轮换或作用域变更、profile 版本变更、恢复/重启、launcher 二进制/OS 身份/DB 路径变更。

**人工层**：`P2` 首次新发送是一次具名动作授权，由用户在机器层通过后签署。实验 profile 的证据不得被读为生产有效。

## 被否方案

- **A 只做回放/模拟**：作为首个范围不回答票面问题，TB-08/09 五张叶片会继续没有范围；它保留为裁定一前置读数失败时的退回范围。
- **C 主网独立地址/子账户最小额**：真实外效；#942 第 1/3 问未测；用户已裁不做强平；暴露于 ADR 0021 裁定三的部署者权力。
- **D IBKR paper**：#1283 第二轮；需本地 TWS；ADR 0021 `:29` 记 IBKR 维持保证金与强平合并计算；ROADMAP 支柱 IV 下单执行器未实现。
- **E Binance 只读**：验不了发送与回报；子账户未开通；需 VPN。

## 与 ADR 0021 的关系

不构成重启（裁定五的条件未触发）。本 ADR 在「暂不选定生产场所」的框架内只钉首个能力验证范围；ADR 0021 全部有效域原样保留。

## 受影响代码清单

见 [`.chanlun/review-results/issue1323-grilling-20261013/1351-AFFECTED-CODE.md`](../../.chanlun/review-results/issue1323-grilling-20261013/1351-AFFECTED-CODE.md)（基线 main `465174240b`，FIN 线 `0d8a6017` 单独标注）。要点：R2 `economic_session` 没有 profile 类型（缺口）；FIN `finance.rs:211` 的 `capability_id` 是 TestOnly 字段；旧 Python `trading_system/config/broker_config.py` 与 `trading_system/execution/risk_controls.py` 为可收编资产（ADR 0027 裁定五），不自动成为正本。

## 下游票

TB-08-A/B/C [#1382](https://github.com/xy7365527-lang/NewChanlun/issues/1382)/[#1400](https://github.com/xy7365527-lang/NewChanlun/issues/1400)/[#1412](https://github.com/xy7365527-lang/NewChanlun/issues/1412)；TB-09-A/B [#1383](https://github.com/xy7365527-lang/NewChanlun/issues/1383)/[#1401](https://github.com/xy7365527-lang/NewChanlun/issues/1401)；TB-10-H/I [#1402](https://github.com/xy7365527-lang/NewChanlun/issues/1402)/[#1413](https://github.com/xy7365527-lang/NewChanlun/issues/1413)；[#1319](https://github.com/xy7365527-lang/NewChanlun/issues/1319) 作 TB-08 证据回填对象，不恢复其旧组织。

## 溯源标签

全部 `[新缠论:选择]`。缠师原文对程序化执行场所无论述；`.chanlun/definitions/qujiantao.md:447`（#817）已定「参数……按场所与执行系统给，不入正本」。
