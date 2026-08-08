# ADR 0021：执行场所暂不选定——已测绘的约束、证据状态与重启条件

**日期**：2026-08-08
**裁定人**：编排者，走 [执行场所选定 #936](https://github.com/xy7365527-lang/NewChanlun/issues/936)（map [#931](https://github.com/xy7365527-lang/NewChanlun/issues/931)）当场拍板
**状态**：已采纳（**本 ADR 是 map #931 的收口正本**）

## 裁定一：暂不选定执行场所

**理由（编排者 2026-08-08 原话口径）**：「**可以很多，这个看情况，未必要把这个跑完，我们暂时只有回测也是可以的。**」

⟹ **暂无实盘需求** ⟹ 场所选定的前提（隔离能力、敞口上限、API 凭证）**当前全部不成立**——回测不需要其中任何一项。

**与代码现状一致**（非事后合理化，实测在先）：三条下单链**没有一条 live 跑过**；`trading_system/live/runner.py` 自述「阶段 4+ 骨架，当前不可运行」；`enable_orders` 真实读取点是 `backtest/runner.py:84` 的 argparse（**硬默认 False，非 env var**），本 HEAD 无 `deploy/` 目录、CI 不调用（[#945](https://github.com/xy7365527-lang/NewChanlun/issues/945) / [#949](https://github.com/xy7365527-lang/NewChanlun/issues/949)）。

## 裁定二：倾向 Hyperliquid，但**证据等级已降级，须随倾向一起引**

**倾向的三条理由**（按证据强度排序）：

1. **隔离有官方原文，Binance 只有结构推断** —— HIP-3 §Spec.2「each perp dex features **independent margining**…」；abstraction modes §3「**separate DEX balances. Cross margin applies to each DEX separately.**」；portfolio margin §Liquidations「**Sub-accounts are still treated separately under portfolio margin.**」。而 Binance 侧**没有任何一句官方原文**断言子账户爆仓不传染，仅有独立余额 ＋ `forceLiquidationBar`/`marginCallBar`/`normalBar` 字段的结构性证据链（[#932](https://github.com/xy7365527-lang/NewChanlun/issues/932)）。
2. **不依赖 VPN** —— 【实测】同一时刻 `fapi/v1/ping` = **451**，`api.hyperliquid.xyz/info` = **200**。成因由编排者给出：「**Binance 是要 VPN 才行**」。
3. **深度实测够用** —— `xyz` 上 10 万美元单笔冲击 NVDA 2.6bp／TSLA 4.5bp／GOOGL 4.2bp；2 万档一线大盘股全部 ≤2.5bp（[#939](https://github.com/xy7365527-lang/NewChanlun/issues/939)，**样本量=1，单一时点快照**）。

### ⚠️ 证据降级（引本倾向必须一并引）

**「各层强平互不牵连」这条硬需求第一位，只有上述三句官方原文支撑，零实测。**

[#942](https://github.com/xy7365527-lang/NewChanlun/issues/942) 十问中第 1／3／4／5 问（子账户能否在 `xyz` 下单、`userSetAbstraction` 对子账户单独生效、**爆仓对照实验**、`sendAsset` 划转）**全部因凭证约束未做** —— 卡点是 testnet faucet 要求地址已在主网存过款（错误原文 `Cannot claim drip because user ... does not exist on mainnet.`），而编排者裁定不做主网强平实验（走「乙」）。第 2 问（子账户 abstraction 默认值）**已测但未决**。

**⟹ 反面教材在案**：**IBKR 的 STL 官方自陈**「开仓时每子账户单独计保证金，**但就维持保证金与强平而言所有账户合并计算**」——**「子账户」这个词本身不保证隔离，文档写得漂亮与实际隔离是两回事。**

## 裁定三：把美股腿放在第三方 builder DEX 上的对手方风险（**部署者权力清单**）

美股永续**不在 Hyperliquid 主 DEX 上**（主 DEX 在架 177 个永续，**零美股标的**），而在 HIP-3 **builder-deployed** DEX 上：`xyz` 94 个／`para` 17 个／`mkts` 2 个（美股约 70 个）。⟹ **那是另一套账本，部署者是第三方。**

| 动作 | 作用域 | 对已开仓位 | 预告/时间锁 | 赔付 | 可逆 |
|---|---|---|---|---|---|
| `haltTrading` | 单资产 | 【官方原文】**强制按当前标记价结算 ＋ 取消挂单** | **查不到任何预告/时间锁** | **无** | 可逆 |
| `disableDex` | **整个 DEX** | **查不到** | **查不到** | **查不到** | **查不到** |
| **改保证金表** | 单资产或整表 | 【推断】**改变已开仓位强平价** | **★ 查不到限速** | 不涉及 | 可逆，但对已被强平的仓位无法挽回 |
| 质押罚没（验证人投票） | 部署者 500k HYPE | 不直接结算仓位 | 无 | **明确无赔付——烧掉，不进用户口袋** | — |

**★ 最阴的一条**：`setDeployerFees` **有明文「30 天一次」限速，而保证金表改动没有对应条款** ⟹ **部署者可以随时、无预告地移动你所有层的强平价，且无频率限制。**

⚠️ 「改保证金表会影响已开仓位」是**【推断】不是官方直接陈述** —— 从两处独立公式反推（`l = 1/MAINTENANCE_LEVERAGE`；官方原文「maintenance leverage depends on the **unique margin tier corresponding to the position value at the liquidation price**」＝ **实时求值非开仓快照**；而「Leverage is only checked upon opening a position」只管初始杠杆，是文档里分开的两小节）。**引用时须带此限定。**

### ★★ 三个共模失效点

**不是某一层被打，是所有层同时失效** —— 而这正打在「多层各自带止损」这套架构的心脏上：

1. **`haltTrading`** —— 所有层同时被按标记价结算
2. **改保证金表** —— 所有层的止损位同时被移动
3. **VPN 断线**（Binance 侧）—— 所有层同时断连，**且最可能在网络拥堵时发生，正是行情剧烈最需要下单的时候**

## 裁定四：架构约束（现在就生效，不等场所选定）

> **每建一个 Hyperliquid 子账户，立刻显式 `userSetAbstraction` 设成 `standard`，然后回读校验一次。**

**成因**：新建子账户的 abstraction mode 默认值**测不出来**（[#942](https://github.com/xy7365527-lang/NewChanlun/issues/942) 第 2 问未决），而 mode ＝ `standard` 是 #939 裁定的隔离三条件之一 ⟹ 若默认是 `unified`，隔离**开箱就是破的、且破得静默**（官方原文：unified 下余额「collateralizes all cross margin positions」，portfolio margin「All HIP-3 DEXs are included」——**跨 DEX 传染是设计如此，不是漏洞**）。

**这条把「默认值是什么」从查不到的未知变成一行初始化代码，成本≈0。**

**验收档位**（按 [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十一「按对象性质分派」）：**行为不变式 → 测试锁**。⚠️ #799 自己查出「**CI 根本不跑 `cargo test`**」⟹ 落锁时**必须确认它真进 CI**，否则按该裁定附带原则「**只编译不执行的锁按「无锁」计**」。

**⚠️ 它不能替代验证**：保证「mode 被设对了」，**不保证「设对之后爆仓不传染」**。

## 裁定五：重启条件

**本 ADR 在下列任一条件出现时重启**（重启 ＝ 新开一张图或一张裁定票，**不是恢复 map #931**）：

1. **要上实盘** —— 这是主条件。届时需重跑 [#942](https://github.com/xy7365527-lang/NewChanlun/issues/942) 未做的第 1／3／4／5 问，且**必须解决凭证约束**（用主钱包激活 testnet 地址 ＋ API wallet，或接受主网最小额实测）
2. **`recursive_t` 执刀完成** —— [#951](https://github.com/xy7365527-lang/NewChanlun/issues/951) 关闭后生产正本确定，[ADR 0020](0020-backtest-cash-live-perp-filtered.md)「实盘喂 C」才说得清喂的是哪一套
3. **Hyperliquid 或 Binance 的隔离机制发生变更** —— 尤其 HIP-3 规范对部署者权力的约束（时间锁／赔付／限速）若增补，裁定三整表要重看

## 有效域（引本 ADR 必须一并引）

1. **所有场所侧读数均为 2026-08-07～08 的快照**，深度读数**样本量=1、单一时点**。
2. **隔离零实测**（见裁定二降级说明）。
3. **`disableDex` 语义至今查不到** —— 官方文档、社区 SDK 注释、SDK 测试用例三处零命中；**HL 共识层不开源，无法核实**。
4. **BingX 的两个洞未答** —— `SEPARATE_ISOLATED` 对股票永续生不生效、同一交易对最多几个独立仓。技术上它仍是本图查到的**最优机制**（完整 `positionId` 闭环、352 个美股个股永续），**只是编排者过不了 KYC**。存档在 [#932](https://github.com/xy7365527-lang/NewChanlun/issues/932) 第一条评论，**过了认证可直接照跑**。
5. **落地约束**：NT 适配层从第一天就按「多场所」抽象，**不许写死单一场所**——否则以后接 BingX 要重写。抽象边界未定，见「残项去向」。

## 残项去向（本仓关图判据：每条残雾必须落到三处之一，不许沉默消失）

| 残项 | 去向 |
|---|---|
| Binance 侧三问（TradFi 协议签署等） | **已部分实测（2026-08-08，[#932](https://github.com/xy7365527-lang/NewChanlun/issues/932) 第二程，见下「★ 2026-08-08 补测」）**；残项挂 [#953](https://github.com/xy7365527-lang/NewChanlun/issues/953)，实际待办已从「开 key」变为「**网页端开通子账户功能**」 |
| ADR 0020 裁定二重裁 | **挂 [#946](https://github.com/xy7365527-lang/NewChanlun/issues/946)**，等 [#951](https://github.com/xy7365527-lang/NewChanlun/issues/951) 定生产正本 |
| 冲击成本跨周采样 | **挂 [#933](https://github.com/xy7365527-lang/NewChanlun/issues/933)**，票保持 open，**搁置至有实盘需求** |
| 逐笔独立之后账怎么记 | **挂 [#937](https://github.com/xy7365527-lang/NewChanlun/issues/937)**，**搁置至场所选定重启** |
| Binance 子账户实测 | **挂 [#932](https://github.com/xy7365527-lang/NewChanlun/issues/932)**，**搁置**（场所倾向已定，优先级最低） |
| NT 适配层多场所抽象边界 | **搁置，无人跟进** —— 待 #951 定正本后才谈得上 |
| 单一 builder DEX 敞口上限 | **搁置，无人跟进** —— 编排者 2026-08-08：「可以很多，这个看情况」，无实盘需求故不定 |
| 「重」的数量受不受配额卡 | **搁置，无人跟进** —— 依赖资金量，编排者未给 |
| 多场所并用的账本与调度形态 | **搁置，无人跟进** —— 无实盘需求 |
| 交易所风险的分散策略 | **部分答**：裁定三已给出对手方风险清单；**分散策略本身搁置，无人跟进** |

## ★ 2026-08-08 补测：Binance 侧（[#932](https://github.com/xy7365527-lang/NewChanlun/issues/932) 第二程）

编排者当日授权使用既有 key（`~/.config/binance-liq-anchor/`）跑完 Binance 侧。**本节不改动上述任何裁定**，只补事实与订正。

### 安全门：`enableWithdrawals: false`（不带提现权限）

另：`enableInternalTransfer: false`、`permitsUniversalTransfer: false` ⟹ **该 key 无划转权限**。**操作性订正**：该凭证用 **HMAC-SHA256 而非 Ed25519**，同目录的 `ed25519-*.pem` 与之**不配对**。

### 实测结果

- **【实测】主账户已经能交易股票永续** —— `AAPLUSDT` 市价买入 0.02（名义 6.264 USDT）成交 @313.20，**全程零 `-4411`**，随即 reduceOnly 平掉，仓位回读归零，净成本 **0.00521111 USDT**。
- **★ 授权了但没执行签协议**（`POST /fapi/v1/stock/contract`）：实测已证**无需再签**，而该操作**可逆性未知** ⟹ **在已证无必要的前提下执行是纯粹的不可逆风险**。主控认可。
- **子账户整体不可达**：该账户**子账户功能未开通**（`-8012`／`-9000`），API 创建被拒（`100001002 Unsupported operation`）。
- **与 #932 第一程冲突已登记**：第一程记「VIP0 可直接开子账户、上限 5 个」，而本账户**确为 VIP0 却连创建都被拒** ⟹ 撞到的是**更前面一道门**；「上限 5 个」**既未证实也未否证**，三种成因（网页端未开通／账户类型／KYC）**实测无法区分，未裁**。
- **VIP0 实时费率实测确认**：taker **0.0400%**、maker **0%**（5 标的一致；对照 `BTCUSDT` 0.02%/0.05% ⟹ **确为独立费率表**）。`AAPLUSDT` 最高 20x、首档 MMR 2.5%、**默认已 ISOLATED**、最小名义 5 USDT。

### ⟹ 对裁定二「证据降级」的加强

**Binance 侧「子账户强平不传染」零实测，且比 Hyperliquid 更彻底** —— Hyperliquid 至少有三句官方原文，Binance **零原文**；而本次连**子账户都建不出来**，洞三零推进。

**⟹ 若将来要把 Binance 登记为「被否方案」，理由目前只能写「测不出来」，不能写成「测了不行」。**

### 一处主控推断被实测推翻

主控曾据 testnet 返回推断「主网 `/fapi/v1/stock/contract` 路径存在」。**不成立**：主网 GET 该路径返回的 404 与「明确不存在的路径」的**负控逐字相同**（同一张 HTML 错误页）⟹ **无法区分路径存不存在**。testnet 上能区分（`-5000 Method GET is invalid`）。**那条读数只在 testnet 上成立。**

### testnet 的边界（本次实测）

`testnet.binancefuture.com` 有 **21 个 `EQUITY`** 合约、`/fapi/v1/stock/contract` 路径可辨；**但 `/sapi/v1/*` 返回 301（nginx 级）⟹ testnet 无 `sapi`，子账户管理端点全部不存在** ⟹ **子账户隔离在 testnet 上无法验证**。testnet API key 需网页 OAuth 交互式注册，本会话无法非交互生成。
