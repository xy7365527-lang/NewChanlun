# Hyperliquid HIP-3 builder DEX：隔离模型 + 美股深度基线

- 票：[#939](https://github.com/xy7365527-lang/NewChanlun/issues/939) 第一程（免签名、只读部分）；图 [#931](https://github.com/xy7365527-lang/NewChanlun/issues/931)
- 采样时点：**2026-08-07T21:42:36Z**（UTC）
- 端点：`POST https://api.hyperliquid.xyz/info`（公开只读，全程未使用任何凭证、未下单）
- 原始数据：`.chanlun/review-results/hyperliquid-hip3-depth-snapshot-20260807.json`
- 采集脚本：`.chanlun/review-results/scripts/hl_hip3_depth_snapshot.py`
- 文档语料：`https://hyperliquid.gitbook.io/hyperliquid-docs/llms-full.txt`（2026-08-07 抓取，495,870 字节，全站正文合集；下文所有"全站检索"均指对该文件的 grep）

---

## 结论先行

**一（塌方点判定）**：在 HIP-3 builder DEX 上跑多层资金，「各层强平互不牵连」**有条件成立**。
条件有三条，缺一条即不成立：
1. **每层必须是独立地址**（master 或 sub-account），不能只靠"同一账户内的 isolated 仓位"来分层；
2. **每层的 account abstraction mode 必须设为 `standard`（Manual/Standard）**。若设为 `unifiedAccount` 或 `portfolioMargin`，该地址在**所有** DEX 上的 cross 仓位会合并计算保证金——跨 DEX 传染在官方公式里是明写的；
3. 上述隔离是**保证金/强平层面**的。同一 DEX 内的 **ADL（自动减仓）**仍是跨用户的共享风险，两层若持同一标的的相反方向，一层爆穿可以经 ADL 打到另一层的盈利仓位上——这条**任何账户结构都躲不掉**。

**二（深度基线）**：`xyz` 是唯一一个美股腿有真实容量的场所。快照时点上，`xyz` 的一线大盘股（NVDA / GOOGL / TSLA / INTC / MU / MSTR / PLTR）**单笔 10 万美元市价单冲击成本 1.7–8 bp**，2 万美元 0.6–2.5 bp；`mkts` 两个指数只有 USTECH 能吃 10 万（买 9.8 bp / 卖 43 bp，严重不对称）；**`para` 全线不可用**（价差 13–58 bp，1000 美元单子就要 7–38 bp）。**样本量 = 1，不可作结论。**

**⟹ 可行性总判**：**有条件可行**。条件见文末「可行性判定」节。

---

## 一、HIP-3 builder DEX 的信任与隔离模型

### 1.1 保证金与清算隔在哪一层

**【官方原文】** HIP-3 规范 §Spec 第 2 条：

> Any deployer that meets the staking requirement can deploy one perp dex. As a reminder, **each perp dex features independent margining, order books, and deployer settings.** A future upgrade may support multiple dex deployments sharing the same deployer and staking requirement.

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-3-builder-deployed-perpetuals>

**【官方原文】** Margining §HIP-3 Margin Modes：

> When users have perp positions across multiple DEXs, cross margin behaves different depending on the user's account abstraction. For unified account and portfolio margin, the user's cross margin positions in DEXs with the same collateral all share margin. **For standard abstraction, cross margin only applies to the assets within the same DEX.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/margining>

**【官方原文】** Account abstraction modes，三种模式的原文定义：

> 1. Unified account (recommended for most users): single balance for each asset. **This balance collateralizes all cross margin positions in that asset** and is unified with spot balance in that asset. For example, USDC balance is the single source for validator-operated perps, XYZ perps, and spot trading against USDC as a quote asset. […]
> 2. Portfolio margin (most capital efficient): single portfolio unifying all eligible assets […]
> 3. **Manual / Standard** (recommended for market makers, high volume automated users, and deployers/builders): **separate perp and spot balances, separate DEX balances. Cross margin applies to each DEX separately.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/account-abstraction-modes>

**【官方原文】** Portfolio margin 的维持保证金公式里，跨 DEX 求和是明写的：

> `portfolio_maintenance_requirement(token) = min_borrow_offset + sum_{dex} cross_maintenance_margin(dex) + borrowed_size_for_maintenance(token) * borrow_oracle_price(token)`

并且：

> **All HIP-3 DEXs are included in portfolio margin**, though not all HIP-3 DEX collateral assets are borrowable.

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/portfolio-margin>

**⟹ 回答「在 `xyz` 上爆仓会不会动到主 DEX 余额或其他 builder DEX 仓位」：**
- **`standard` 模式下：不会**（"separate DEX balances"，官方原文）。
- **`unifiedAccount` / `portfolioMargin` 模式下：会**，且是设计如此（cross 仓位跨 DEX 共享同一抵押余额；portfolio margin 的维持要求对所有 DEX 求和）。
- **反过来也一样对称**——主 DEX 爆仓在 unified/portfolio 模式下同样会抽走 `xyz` 的抵押。

**【实测】** 同一个地址在不同 DEX 上确实各有一份独立的 clearinghouse 状态（2026-08-07T21:42:36Z，探针地址 = `xyz` 部署者公开地址 `0x88806a71d74ad0a510b350545c9ae490912f0888`，只读）：

| `dex` 参数 | accountValue | withdrawable |
|---|---|---|
| （省略，主 DEX） | 59.0 | 59.0 |
| `xyz` | 9.999719 | 9.999719 |
| `para` | 0.0 | 0.0 |
| `mkts` | 0.0 | 0.0 |

请求形如 `{"type":"clearinghouseState","user":"0x8880…0888","dex":"xyz"}`。**同一地址、四个不同余额**——per-address-per-dex 的账本切分是实测成立的。

**【实测】** 每个 DEX 有各自的抵押池总量（`{"type":"perpDexStatus","dex":"<name>"}`，同一时点）：

| DEX | totalNetDeposit（USDC） |
|---|---|
| （主 DEX） | 1,869,019,359 |
| `xyz` | 1,317,092,600 |
| `para` | 8,013,034 |
| `mkts` | 5,252,931 |
| `hyna` | 5,226,212 |
| `cash` | 513,919 |
| `km` | 24,128 |
| `vntl` | 22,987 |
| `flx` | 6,471 |
| `abcd` | 0.01 |

**【官方原文】** 每个 DEX 有各自的兜底清算人，且偿付能力是**按 DEX** 保证的：

> Each HIP-3 DEX is associated with a fully onchain strategy at `0x400..00 + {dex_index}` that takes over backstop liquidatable positions from the designated HIP-3 DEX. […] **Each DEX's onchain backstop liquidator is an independent user and falls back to ADL to mathematically guarantee solvency of the DEX.**

来源：HIP-3 规范 §Backstop liquidator（同上 URL）

**【官方原文】** ADL 是同一标的内跨用户的：

> If a user's account value or isolated position value becomes negative, **the users on the opposite side of the position are ranked by unrealized pnl and leverage used.** […] Those traders' positions are closed at the previous mark price against the now underwater user

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/auto-deleveraging>

**【推断】** ADL 是本方案里唯一躲不掉的跨层耦合。推断链：(a) ADL 按「持相反方向、盈利排序」选人，选择键里没有账户归属；(b) 若 A 层做多 `xyz:AAPL`、B 层做空同一标的，B 层爆穿到负值时 A 层的盈利仓位会被按 mark price 强行平掉；(c) 这与账户/子账户/DEX 结构无关，是市场层面的耦合。**未在文档中查到任何按账户归属豁免 ADL 的机制**（全站检索 `auto-deleverag|deleverag|\bADL\b` 命中 5 行——Auto-deleveraging 页 3 行 + HIP-3 §Backstop liquidator 2 行，均无豁免条款）。

### 1.2 子账户在 builder DEX 上还成不成立

**⚠️ 订正票面前提**：票面转述的"官方原文说子账户在 clearinghouse 里被当作独立账户"——**这句原文我没找到**。全站检索 `sub-?accounts? (are|is|share|do|can)` 只命中 4 处（fees / sub-accounts 页 ×2 / portfolio margin），**没有一处使用 "treated as separate accounts in the clearinghouse" 这样的表述**。已查到的、最接近的官方原文有两条：

**【官方原文】** Portfolio margin §Liquidations：

> Portfolio margin is a generalization of cross margin. Instead of margining all perp positions within one DEX together, all cross margin perp positions and spot balances are collectively margined together within one account. **Sub-accounts are still treated separately under portfolio margin.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/portfolio-margin>

这一条承重很高：**即使在合并程度最高的 portfolio margin 模式下，子账户仍然是分开的**。

**【官方原文】** API 速率限制 §Address-based limits：

> **Address-based limits apply per user, with sub-accounts treated as separate users.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits>（正文见 llms-full.txt:7613）

**【官方原文】** Clearinghouse 页（决定隔离粒度是"地址"）：

> The perps clearinghouse is a component of the execution state on HyperCore. **It manages the perps margin state for each address**, which includes balance and positions.

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/hypercore/clearinghouse>

**【官方原文】** Perpetuals 合约规格表一行：`| Account type | Per-wallet cross or isolated margin |`
来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/perpetual-assets>（llms-full.txt:1301）

**【官方原文】** 子账户是通过 master 签名 + `vaultAddress` 字段操作的独立地址：

> Subaccounts and vaults do not have private keys. To perform actions on behalf of a subaccount or vault signing should be done by the master account and the vaultAddress field should be set to the address of the subaccount or vault.

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint>

**【官方原文】** abstraction mode 可以**逐子账户**设置（`userSetAbstraction` 的 `user` 字段）：

> "user": address in 42-character hexadecimal format. **Can be a sub-account of the user**,
> "abstraction": one of the strings \["disabled", "unifiedAccount", "portfolioMargin"],

来源：同上（exchange-endpoint，`userSetAbstraction`）

**【官方原文】** 反面对照——vault 明确**不能**交易 HIP-3 perps，子账户没有同类禁令：

> **Vaults can trade validator-operated perps. They cannot trade spot or HIP-3 perps.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/hypercore/vaults>（llms-full.txt:435）

**【推断】** 子账户可以在 builder DEX 上独立开仓、独立强平。推断链：(a) clearinghouse 按 **address** 管保证金状态（官方原文）；(b) 子账户是一个真实地址（有 42 位地址，只是无私钥）；(c) HIP-3 资产 ID 与主 DEX 统一，交易走同一套 order action，而该 action 支持 `vaultAddress`（官方原文）；(d) 官方明确禁止 vault 交易 HIP-3 却未禁止子账户，且 `userSetAbstraction` 明写可对子账户设置——说明子账户在 HIP-3 语境下是被承认的一等实体；(e) 实测证实 `clearinghouseState` 对任意地址 + 任意 `dex` 参数都返回独立余额。
**未在文档中查到任何一句直接说"子账户可以交易 HIP-3 perps"的原文**（全站检索 `sub-?account` 命中 45 行，逐条读过，无此句）。**这条要落成事实，必须拿 API key 实测**（见文末清单）。

**⚠️ 子账户数量的硬门槛（官方原文，与票面「50 个」的表述需要补条件）**：

> **Up to 10 sub-accounts can be created after reaching $100,000 in volume. Every additional $100M in volume enables the ability to create 1 additional sub-account, up to a maximum of 50 sub-accounts.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/sub-accounts>

即：**冷启动 0 个；$10 万成交量后 10 个；要凑到 50 个需要累计 40 亿美元成交量。** 图上"主私钥统管 50 个子账户"应改记为"10 个（$10万量后），50 个需 $4B 量"。

**【官方原文】** API wallet 配额（多层并行下单的实际约束）：

> The number of API wallets available starts at 3 for all master accounts and increases by 2 per sub-account.

以及 nonce 冲突警告：

> If users want to use multiple subaccounts in parallel, it would easier to generate two separate API wallets under the master account, and use one API wallet for each subaccount. This avoids collisions between the nonce set used by each subaccount.

来源：sub-accounts 页 / <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/nonces-and-api-wallets>

**⟹ 这条对多层资金是直接的实施要求：每层配一个独立 API wallet，否则 nonce 会互相打架。**

### 1.3 oracle 谁更新

**⚠️ 订正票面事实**：票面说 `oracle_updater` "主 DEX 之外全为 `null`"——**不对**。【实测】2026-08-07T21:42:36Z，`{"type":"perpDexs"}` 返回：

| DEX | deployer | oracleUpdater | setOracle 的 subDeployers |
|---|---|---|---|
| `xyz` | `0x88806a71…0888` | **null** | `0x1234567890545d1df9ee64b35fdd16966e08acec` |
| `para` | `0x8888888c…6ed3` | `0x8888888c…6ed3`（= deployer） | `0x764ed9d5…4335`, deployer |
| `mkts` | `0x71f0019c…29ec` | **null** | `0xe99202af…86c4` |
| `flx` | `0x2fab5525…6352` | `0x94757f8d…edd5` | — |
| `hyna` | `0x53e65510…0637` | `0xaab93501…de17e` | — |
| `vntl` / `km` / `abcd` / `cash` | — | null | `cash`: `0xa1607092…73e2`, `0xe2de88eb…9b1d` |

**9 个已注册 DEX 中，oracleUpdater 非空的有 3 个（`para`/`flx`/`hyna`），null 的有 6 个。**

**【官方原文】** null 的含义在 HIP-3 deployer actions 的 `PerpDexSchemaInput` 注释里：

> `@param oracleUpdater - User to update oracles. **If not provided, then deployer is assumed to be oracle updater.**`

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions>

**⟹ null ≠ 无人更新，而是"部署者本人是更新者"。** 另外 `subDeployers` 字段显示部署者把 `setOracle` 变体**转授**给了专门的热钱包（`xyz` 转给 `0x1234…acec`）。

**【官方原文】** 更新机制、频率与停更后果，`SetOracle` 的注释逐字：

> The markPxs outer list can be length 0, 1, or 2. The median of these inputs along with the local mark price (median(best bid, best ask, last trade price)) is used as the new mark price update.
> **SetOracle can be called multiple times but there must be at least 2.5 seconds between calls.**
> **Stale mark prices will be updated to the local mark price after 10 seconds of no updates.** This fallback counts as an update for purposes of the maximal update frequency. **This fallback behavior should not be relied upon. Deployers are expected to call setOracle every 3 seconds even with no changes.**
> **All prices are clamped to 10x the start of day value. markPx moves are clamped to 1% from previous markPx.** markPx cannot be updated such that open interest would be 10x the open interest cap.

来源：同上

**⟹ 停更 10 秒后，标记价格自动回落到本地盘口中价（best bid/ask/last trade 的中位数）。**在薄盘口上这是一条真实的强平风险通道：oracle 停更 + 有人砸盘 → mark price 跟着本地盘口走 → 强平按 mark price 触发。1% 的单步夹紧是缓冲，不是防线（连续多步可以走远）。

**【实测】** `xyz` 的资金费率乘数统一 0.5（`assetToFundingMultiplier`，108 个标的全 0.5，`UNITREE` 例外 0.005）；`para` 统一 0.6；`mkts` 走 `assetToFundingInterestRate`（US500/USTECH 各 0.0000456621，即 8 小时利率）。

### 1.4 部署者能改什么、改了对已开仓位什么影响

**【官方原文】** `PerpDeployAction` 的全部变体（逐字，来源同 1.3）：
`registerAsset2` / `registerAsset` / `setOracle` / `setFundingMultipliers` / `setFundingInterestRates` / `haltTrading` / `setMarginTableIds` / `setFeeRecipient` / `setOpenInterestCaps` / `setSubDeployers` / `setMarginModes` / `setDeployerFees` / `setPerpAnnotation` / `disableDex`

逐项对「对已开仓位的影响」：

| 能改什么 | 官方原文依据 | 对已开仓位的影响 |
|---|---|---|
| **下架 / 强制结算** | `haltTrading`：「The deployer may settle an asset using the `haltTrading` action. **This cancels all orders and settles positions to the current mark price.** The same action can be used to resume trading」 | **【官方原文】直接强制平仓，按当时 mark price 结算。** 这是部署者对已开仓位最强的一把刀，且**文档未记载任何时间锁或预告期**。 |
| **保证金表（杠杆上限）** | `setMarginTableIds`；`RawMarginTable` 的 `marginTiers` 最多 3 档，`MaxLeverage` 范围 `[1, 50]` | **查不到**。文档未说明降低 maxLeverage 后已开仓位是否被重算维持保证金。**注意 Margining 页有一句「Leverage is only checked upon opening a position」**——【推断】这暗示初始保证金不重算，但**维持保证金**（= max initial margin fraction 的一半）是按 margin table 推的，改表很可能**直接改掉已开仓位的强平价**。推断链：(a) 维持保证金 = 「Half of maximum initial margin fraction」（合约规格表原文）；(b) 该 fraction 由 asset 的 margin table 决定，不由用户设的 leverage 决定；(c) 故改 table → 改维持保证金 → 改强平价。**这条推断未经实测，也未见官方原文确认或否认。** |
| **保证金模式** | `setMarginModes: Array<[string, "strictIsolated" \| "noCross"]>`；「**IMPORTANT: Enabling cross margin on an asset is irreversible.**」 | 开 cross 不可逆；从 cross 收回是**做不到**的。反方向（收紧到 strictIsolated/noCross）文档未说对已开 cross 仓位的处理，**查不到**。 |
| **费率** | `setDeployerFees`，`scale ∈ [0.0, 3.0]`（growthMode 时 `[0.0, 10.0)`）；「**On Mainnet, rate limited to one change per 30 days.**」 | 这是**唯一一条查到的时间锁：每 30 天一次**。【实测】`xyz`/`para`/`mkts` 当前 `deployerFeeScale` 均为 `"1.0"`。 |
| **未平仓上限** | `setOpenInterestCaps`；「Open interest caps must be at least the maximum of 1\_000\_000 (1 size unit of collateral asset) or **half of the current open interest**」 | 有下限保护：不能把上限压到当前 OI 的一半以下。**【实测】`xyz` 的 `perpDexLimits`：`totalOiCap = 100 亿`，`oiSzCapPerPerp = 200 亿`，`maxTransferNtl = 30 亿`；`xyz:AAPL` 单标的 OI cap = 1.5 亿美元。** |
| **资金费** | `setFundingMultipliers`（0–10）、`setFundingInterestRates`（8 小时利率 ∈ [-0.01, 0.01]） | 只影响未来资金费流，不动仓位。**未查到调整频率限制。** |
| **授权转让** | `setSubDeployers`：按 action 变体逐个授权地址 | **【实测】`xyz` 把 11 类动作分别授权给了 3 个地址**（`setOracle` 单独给热钱包 `0x1234…acec`，`registerAsset`/`haltTrading`/`setMarginTableIds` 等给 `0x7d16f116…6657` 与 `0xc0892b4f…059d`）。**风险面 = 这些热钱包的私钥安全。** |
| **停用整个 DEX** | `disableDex: string` | **查不到**：类型签名之外，文档对 `disableDex` **没有任何一句说明**（全站检索 `disableDex` 只命中 1 处，即类型定义本身）。对已开仓位的后果**未知**。 |

**治理约束**：查到的只有两条——(a) 费率每 30 天改一次；(b) cross margin 的开启受**验证人强制的资格标准**约束：

> To better protect users, **mainnet validators will enforce that HIP-3 deployers only enable cross margin on assets that satisfy defined eligibility standards**, including: sufficient observable liquidity, a reliable external oracle source, and resilience to price manipulation. In particular, **each time the `externalPerpPx` of an asset moves more than 50% relative to the start of day price, validators will conduct a review to determine whether the deployer should be slashed due to manipulation.**

来源：HIP-3 规范 §Cross margin

**其余动作（含 `haltTrading` 强制结算）未查到任何时间锁、预告期或治理否决机制。**

### 1.5 资金怎么进出 builder DEX

**【官方原文】** 需要**单独划转**，端点是 `sendAsset`（或可由 agent 签名的 `agentSendAsset`）：

> This generalized method is used to transfer tokens between different perp DEXs, spot balance, users, and/or sub-accounts. **Use "" to specify the default USDC perp DEX and "spot" to specify spot. Only the collateral token can be transferred to or from a perp DEX.**

action 结构（逐字）：

> { "type": "sendAsset", "hyperliquidChain": "Mainnet", "signatureChainId": …, "destination": address…, **"sourceDex": name of perp dex to transfer from, "destinationDex": name of the perp dex to transfer to,** "token": tokenName:tokenId, "amount": …, **"fromSubAccount": address in 42-character hexadecimal format or empty string if not from a subaccount,** "nonce": … }

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint>

**⟹ 一个 action 同时跨「DEX × 子账户」两个维度**——`sourceDex`/`destinationDex` + `fromSubAccount`。这正是多层资金调度需要的原语。

**【官方原文】** 划转有额度：`perpDexLimits` 的 `maxTransferNtl`（【实测】`xyz` = 30 亿 USDC，不构成约束）。

**【官方原文】** 另有一条被废弃的自动化通道，**多层隔离场景下必须关掉**：

> **NOTE: deprecated. Prefer `userSetAbstraction`.** If set, actions on HIP-3 perps will **automatically transfer collateral from validator-operated USDC perps balance** for HIP-3 DEXs where USDC is the collateral token, and spot otherwise.

以及 abstraction 页的警告：

> DEX abstraction (discontinued): […] **IMPORTANT: Cross margin on HIP-3 DEXs does not behave intuitively for DEX abstraction users.**

**⟹ 这条一旦开着，`xyz` 上的动作会自动去主 DEX 抽钱——直接击穿层间隔离。必须确认每层都是 `standard`/`disabled`。**

**【官方原文】** API 用户的一个坑：

> For API users, unified account and portfolio margin **show all balances and holds in the spot clearinghouse state. Individual perp dex user states are not meaningful.**

**⟹ 若某层误设成 unified，风控读 `clearinghouseState(dex=xyz)` 得到的数字将失去意义——这既是隔离被破坏的后果，也是它的检测手段。**

### 1.6 HYPE 质押要求与罚没

**【官方原文】** 数额与期限：

> 1. **The staking requirement for mainnet will be 500k HYPE.** This requirement is expected to decrease over time as the infrastructure matures. Any amount staked above the most recent requirement can be unstaked. **The staking requirement is maintained for a minimum of 183 days after the dex is deployed.**

**【官方原文】** 罚没条件与流程：

> To ensure high quality markets and protect users, deployers must maintain 500k staked HYPE. **In the event of malicious market operation, validators have the authority to slash the deployer's stake by conducting a stake-weighted vote.** Even if the deployer has unstaked and initiated a staking withdrawal, the stake is still slashable during the 7-day unstaking queue.
> […] **Slashing is technical and does not distinguish between malicious and incompetent behavior.** Relatedly, slashing does not distinguish between 1. A deployer that deviates from a well-designed contract spec 2. A deployer that faithfully follows a poorly designed contract spec 3. **A deployer whose private keys are compromised**
> […] **Even attempted malicious deployer inputs that do not cause protocol issues are slashable.** Similarly, inputs that do cause protocol issues but that are not irregular are not slashable.
> […] 罚没比例：**irregular inputs that cause invalid state transitions or prolonged network downtime can be slashed up to 100%.** Irregular inputs causing brief network downtime can be partially slashed up to 50%. Invalid inputs that cause network degradation or performance issues can be partially slashed up to 20%.

**⟹ 这构成对用户的什么保障——【官方原文】直接给了答案，是「不构成」：**

> Relatedly, **the slashed stake by the deployer is burned instead of being distributed to affected users.** This is again based on proof-of-stake principles and prevents some forms of misaligned incentives between users and deployers.

以及罚没的守备范围**只护协议不护用户**：

> The guiding principle is that **slashing is to prevent behavior that jeopardizes protocol correctness, uptime, or performance.** […] Some malicious behavior is valid by protocol definition, but incorrect by certain subjective interpretations. **The slashing principle provides that the protocol should not intervene in subjective matters.**

**⟹ 一句话：50 万 HYPE 的质押是对「协议不被打坏」的保证金，不是对「用户不被坑」的赔付基金。被罚的币是烧掉的，受害用户拿不到一分。部署者用 `haltTrading` 在坏价位把你强制结算——只要没破坏协议正确性，按原文这属于"subjective matters"，协议不介入。**

**【官方原文】** 唯一真正落到用户头上的数学保证：

> **Barring implementation issues, HIP-3 inherits Hyperliquid's carefully designed mathematical solvency guarantees.**

即：不会有坏账、不会社会化亏损（"a user who has no open positions will not socialize any losses of the platform"，ADL 页原文）；但**价格质量、市场运营、下架时点**全在部署者手里。

### 1.7 塌方点判定（本节结论）

> **「各层强平互不牵连」在 HIP-3 builder DEX 上——有条件成立。**
>
> **成立的条件**（三条同时满足）：
> 1. 每层 = 一个独立地址（master 或 sub-account）；
> 2. 每层的 abstraction mode = `standard`（不是 `unifiedAccount`、不是 `portfolioMargin`、不是已废弃的 DEX abstraction）；
> 3. 接受一条残余耦合：同一 DEX 同一标的上的 ADL 是跨用户的，两层若在同一标的持相反方向仍可互相打到。
>
> **承重原文**（两句缺一不可）：
> - 「each perp dex features independent margining, order books, and deployer settings」（HIP-3 规范 §Spec.2）——**DEX 之间隔**
> - 「Manual / Standard: separate perp and spot balances, **separate DEX balances. Cross margin applies to each DEX separately.**」（Account abstraction modes）——**模式选对才隔**
> - 「**Sub-accounts are still treated separately under portfolio margin.**」（Portfolio margin §Liquidations）——**子账户在最松的模式下都还是分开的**
>
> **不成立的具体情形（必须写清，因为默认值可能踩中）**：用 `unifiedAccount` 或 `portfolioMargin` 时，同一地址在**所有** DEX 上的 cross 仓位共享抵押，A 层爆仓**会**拖累 B 层。**这不是漏洞，是官方设计。**

---

## 二、`xyz` / `para` / `mkts` 美股深度基线

### 2.0 采样与限制声明（不许被抹掉）

- **采样时点 2026-08-07T21:42:36Z，端点 `POST https://api.hyperliquid.xyz/info`。**
- **样本量 = 1。单一时点快照，不可作结论。** 跨时段采样归 [#933](https://github.com/xy7365527-lang/NewChanlun/issues/933)。
- `l2Book` **每边最多返回 20 档**。表中「穿透20档」= 该金额在 20 档之内吃不完，**不等于冲击成本无穷大**，而是**档外深度未知**。
- 深度不对称按买/卖分列，比值 = 20 档买盘美元 / 20 档卖盘美元。**单边挂单随时可撤。**
- 中价 = (best bid + best ask) / 2；冲击成本 bp = 成交均价相对中价的偏离，买卖两边都取正号表示"不利偏离"。

### 2.1 在架标的的口径订正

**【实测】** 非下架标的数（2026-08-07T21:42:36Z，`{"type":"metaAndAssetCtxs","dex":…}` 剔除 `isDelisted`）：`xyz` = 94，`para` = 17，`mkts` = 2，合计 113——与票面一致。

**但 113 ≠ 113 个美股。**【推断】按"在美国交易所上市的普通股 / ADR / ETF"这一判据人工归类 `xyz` 的 94 个：可辨认为美股的约 **57 个**（AAPL AMAT AMD AMZN ARM ASML AVGO BABA BB BE BX COIN COST CRCL CRWV DELL DKNG EBAY EWJ EWT EWY EWZ GEV GME GOOGL HIMS HOOD IBM INTC LITE LLY META MRVL MSFT MSTR MU NBIS NFLX NOK NOW NVDA ORCL PLTR QCOM RIVN RKLB SMH SNDK SOXL STRC TSLA TSM URNM USAR WDC XLE ZM，其中 ASML/BABA/NOK/TSM 为 ADR，EWJ/EWT/EWY/EWZ/SMH/SOXL/URNM/XLE 为 ETF）；其余是 FX（EUR/GBP/JPY/KRW）、商品（GOLD/SILVER/COPPER/BRENTOIL/CL/NATGAS/PLATINUM/PALLADIUM/URANIUM/WHEAT/CORN/ALUMINIUM/TTF）、非美股（HYUNDAI/SOFTBANK/SMSN/KIOXIA/IBIDEN/SKHX/SKHY/CXMT/ZHIPU/MINIMAX/UNITREE 等）、指数（SP500/JP225/KR200/NIFTY/IBOV/DXY/VIX/XYZ100）和主题合成品（DRAM/H100/GIGADEV/NCLD/BOT/BIRD/SHAZ/LYTE/QNT/PURRDAT/SPCX/CBRS/KORU 等）。
`para` 17 个里美股约 **13 个**（AAOI AVGO CIEN COHR CRDO GLW IREN LRCX NET RDDT STX TER VST），其余为加密指数（BTCD/TOTAL2/OTHERS）与 pre-IPO（UNITREE）。
`mkts` 2 个**全是指数**（US500 / USTECH），无个股。
**⟹ 美股口径的真实数量约 70，不是 113。此归类是人工判断，判据已写明，边界品（如 SPCX、CBRS、DRAM 这类合成主题）未计入。**

### 2.2 冲击成本表

**采样时点 2026-08-07T21:42:36Z UTC ｜ 端点 `POST https://api.hyperliquid.xyz/info` ｜ 样本量 = 1，不可作结论**

单位：bp（相对中价的不利偏离）。「穿透」= 20 档之内吃不完，档外未知。

| 标的 | 中价 | 价差bp | 买深20档 $ | 卖深20档 $ | 不对称(买/卖) | 买1k | 卖1k | 买5k | 卖5k | 买2万 | 卖2万 | 买10万 | 卖10万 | 24h成交额 $ |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| xyz:AAPL | 313.14 | 0.96 | 67,600 | 74,277 | 0.91 | 0.48 | 0.73 | 1.04 | 0.79 | 2.47 | 2.18 | 穿透 | 穿透 | 20,062,661 |
| xyz:NVDA | 223.32 | 0.45 | 421,071 | 191,657 | 2.20 | 0.22 | 0.22 | 0.30 | 0.41 | 1.06 | 1.21 | **2.61** | **1.71** | 42,944,479 |
| xyz:TSLA | 328.85 | 1.22 | 173,293 | 115,894 | 1.50 | 0.97 | 0.61 | 1.45 | 0.61 | 1.75 | 1.19 | **4.53** | **3.28** | 23,162,994 |
| xyz:MSFT | 499.48 | 1.80 | 64,629 | 123,581 | 0.52 | 0.90 | 0.90 | 1.38 | 1.40 | 2.44 | 2.41 | 4.33 | 穿透 | 22,203,709 |
| xyz:META | 592.93 | 1.35 | 132,207 | 56,312 | 2.35 | 0.84 | 0.67 | 0.84 | 0.67 | 2.03 | 1.71 | 穿透 | 4.71 | 12,981,638 |
| xyz:GOOGL | 354.76 | 0.28 | 175,814 | 130,869 | 1.34 | 0.14 | 0.29 | 0.49 | 0.50 | 1.86 | 0.65 | **4.22** | **2.90** | 24,715,892 |
| xyz:AMZN | 274.22 | 1.09 | 37,306 | 67,215 | 0.56 | 0.55 | 0.87 | 1.46 | 1.55 | 2.18 | 4.09 | 穿透 | 穿透 | 7,068,403 |
| xyz:NFLX | 74.24 | 3.37 | 83,091 | 58,163 | 1.43 | 5.50 | 3.61 | 7.08 | 3.69 | 9.39 | 6.40 | 穿透 | 穿透 | 941,210 |
| xyz:COST | 946.26 | 5.07 | 99,133 | 33,941 | 2.92 | 2.54 | 4.81 | 3.05 | 9.62 | 11.53 | 17.13 | 穿透 | 穿透 | 74,886 |
| xyz:MSTR | 100.14 | 2.00 | 173,514 | 189,205 | 0.92 | 1.32 | 1.00 | 1.86 | 1.60 | 3.85 | 2.92 | **7.96** | **7.22** | 15,819,907 |
| xyz:COIN | 154.06 | 2.60 | 111,491 | 134,209 | 0.83 | 1.92 | 1.63 | 2.46 | 3.27 | 3.98 | 6.07 | **9.87** | **9.70** | 7,934,763 |
| xyz:PLTR | 171.08 | 2.34 | 91,750 | 194,337 | 0.47 | 1.17 | 1.58 | 1.59 | 1.72 | 2.12 | 2.53 | 4.51 | 穿透 | 55,962,885 |
| xyz:INTC | 101.37 | 1.97 | 482,926 | 504,259 | 0.96 | 0.99 | 0.99 | 0.99 | 1.13 | 1.58 | 1.97 | **4.23** | **4.53** | 84,608,626 |
| xyz:ORCL | 146.56 | 1.36 | 66,231 | 272,963 | 0.24 | 0.87 | 1.72 | 1.27 | 2.29 | 2.38 | 4.07 | 4.65 | 穿透 | 32,324,539 |
| xyz:AMD | 482.66 | 1.04 | 101,657 | 68,662 | 1.48 | 0.52 | 0.52 | 1.43 | 1.42 | 2.31 | 2.70 | 穿透 | 4.61 | 17,930,890 |
| xyz:MU | 874.12 | 0.11 | 189,540 | 211,709 | 0.90 | 0.94 | 0.06 | 0.97 | 0.06 | 1.25 | 0.69 | **2.01** | **1.70** | 355,912,226 |
| xyz:TSM | 421.09 | 1.19 | 92,251 | 22,199 | 4.16 | 0.81 | 1.28 | 1.02 | 2.60 | 6.17 | 4.90 | 穿透 | 穿透 | 2,143,480 |
| xyz:LLY | 1183.60 | 8.45 | 82,358 | 146,731 | 0.56 | 4.22 | 4.22 | 5.58 | 4.22 | 7.73 | 11.38 | 28.38 | 穿透 | 2,035,415 |
| xyz:XYZ100（指数） | 29734.50 | 0.34 | 847,428 | 1,080,545 | 0.78 | 0.25 | 0.17 | 0.45 | 0.17 | 0.71 | 0.17 | **1.19** | **0.63** | 210,213,918 |
| xyz:GOLD（商品） | 4350.05 | 0.23 | 560,947 | 429,316 | 1.31 | 0.11 | 0.11 | 0.11 | 0.11 | 0.11 | 0.30 | **0.11** | **1.59** | 170,374,636 |
| mkts:US500 | 773.54 | 3.75 | 7,851 | 11,687 | 0.67 | 2.71 | 2.20 | 4.82 | 7.56 | 穿透 | 穿透 | 穿透 | 穿透 | 1,318,508 |
| mkts:USTECH | 723.16 | 2.21 | 410,529 | 139,569 | 2.94 | 1.18 | 1.96 | 1.38 | 2.16 | 1.74 | 2.20 | 9.81 | **43.15** | 4,404,916 |
| para:AVGO | 425.45 | 28.21 | 108,694 | 112,179 | 0.97 | 14.10 | 14.10 | 24.61 | 16.60 | 53.13 | 24.13 | 134.58 | 90.02 | **2,670** |
| para:COHR | 378.75 | 13.47 | 420,088 | 432,926 | 0.97 | 37.97 | 6.73 | 38.96 | 6.73 | 39.27 | 7.60 | 47.14 | 16.93 | 378,902 |
| para:CRDO | 249.48 | 54.11 | 414,567 | 424,572 | 0.98 | 27.06 | 27.06 | 27.06 | 27.06 | 27.06 | 27.50 | 44.23 | 39.42 | 142,992 |
| para:NET | 301.12 | 57.78 | 157,696 | 212,221 | 0.74 | 28.89 | 28.89 | 28.89 | 28.89 | 28.89 | 28.91 | 121.07 | 122.23 | 134,763 |
| para:RDDT | 161.38 | 58.25 | 163,491 | 166,991 | 0.98 | 29.12 | 29.12 | 29.12 | 29.12 | 29.12 | 29.14 | 121.16 | 122.31 | **2,932** |
| para:IREN | 41.32 | 46.71 | 384,330 | 453,130 | 0.85 | 23.36 | 23.36 | 23.36 | 23.36 | 23.36 | 27.34 | 35.34 | 38.57 | 309,354 |

### 2.3 读出来的几条（都限定在这一个时点）

1. **`xyz` 一线大盘股在 2 万美元档已经很便宜**：NVDA / GOOGL / TSLA / MU / INTC / MSFT / AMD / META / AAPL 全部 **≤ 2.5 bp**。10 万美元档，20 档之内能吃完的（NVDA 2.6/1.7、GOOGL 4.2/2.9、TSLA 4.5/3.3、INTC 4.2/4.5、MU 2.0/1.7、MSTR 8.0/7.2、COIN 9.9/9.7、PLTR 4.5/穿透）**冲击成本 1.7–10 bp**。
2. **"穿透 20 档"集中在 AAPL / AMZN / MSFT / META / AMD / NFLX / COST / TSM / LLY 的 10 万美元档**——20 档累计名义常在 2–12 万区间，**不代表这些标的装不下 10 万，只代表 20 档看不到**。要落实必须用 `nSigFigs` 聚合或订阅逐档 WS 重采（见 API-key 清单）。
3. **不对称是常态**：TSM 买/卖 = 4.16，ORCL = 0.24，META = 2.35，mkts:USTECH = 2.94（且卖 10 万要 43 bp 而买 10 万只要 9.8 bp）。**单边建仓的成本可以是另一边的 4 倍。**
4. **`xyz:XYZ100` 与 `xyz:GOLD` 是全场最深**（10 万美元 0.1–1.6 bp，24h 成交 2.1 亿 / 1.7 亿）。**若美股腿只需要"宽基指数敞口"，XYZ100 的容量远超任何个股。**
5. **`para` 全线不可用**：价差 13–58 bp，多数标的在 1k–2万 区间冲击成本恒定 ≈ 半个价差（说明是一张挂在远处的大单撑着，不是真流动性），24h 成交额 AVGO 只有 **2,670 美元**、RDDT **2,932 美元**。
6. **`mkts:US500` 盘口极薄**（20 档买盘 7,851 美元），2 万美元就穿透；`mkts:USTECH` 尚可但卖侧差。
7. **保证金模式分叉（【实测】）**：`xyz` 上 AAPL/NVDA/TSLA/MSFT/META/GOOGL/AMZN/MU/XYZ100/GOLD 与 `mkts:US500`/`USTECH` **开了 cross**（`marginMode` 字段缺省 + `growthMode: enabled`），杠杆上限 20–30x；而 NFLX/COST/MSTR/COIN/PLTR/INTC/ORCL/AMD/TSM/LLY 与 `para` 全线是 **`noCross`**（只能 isolated，杠杆 10x）。
   **⟹ 这对多层隔离是好消息**：`noCross` 标的从协议层就不可能跨 DEX 合并保证金，等于隔离被硬约束住；反过来，**cross 已开的那批（含 AAPL/NVDA/TSLA）才是需要靠 `standard` 模式来守隔离的地方**，而且「Enabling cross margin on an asset is irreversible」——这批标的的 cross 属性再也收不回去了。

---

## 三、未查到的部分与原因

| 问题 | 状态 | 原因 |
|---|---|---|
| 「子账户能在 builder DEX 上交易」的**直接官方原文** | **查不到** | 全站检索 `sub-?account`（llms-full.txt，命中 45 行）逐条读过，无此句。只能靠推断链（§1.2）。**要落成事实必须实测。** |
| 票面转述的「子账户在 clearinghouse 里被当作独立账户」原文 | **查不到** | 当前 sub-accounts 页无此句。最接近的是 portfolio-margin 的「Sub-accounts are still treated separately under portfolio margin」与 rate-limits 的「sub-accounts treated as separate users」。**图上若引用了前一种表述，应改引后两句。** |
| 改 `marginTableIds`（降杠杆）对**已开仓位**的影响 | **查不到** | 文档只说 "Leverage is only checked upon opening a position"，未说维持保证金表变更如何作用于存量仓位。§1.4 给了推断链但未证实。 |
| `disableDex` 动作的语义与对已开仓位的后果 | **查不到** | 全站检索 `disableDex` 仅 1 处命中（类型定义），无任何说明文字。 |
| `setMarginModes` 从 cross 收紧到 `noCross`/`strictIsolated` 对已开 cross 仓位的处理 | **查不到** | 文档只声明"开启 cross 不可逆"，未说反向操作。 |
| 账户新建时 abstraction mode 的**默认值** | **查不到** | 文档标注 unified account 为 "recommended for most users"，但**未说明新账户/新子账户的默认值是什么**。这一条对本方案承重（默认若是 unified，隔离默认是破的）。**必须实测。** |
| 20 档以外的真实深度 | **未采** | `l2Book` 默认返回 20 档。可用 `nSigFigs` 聚合放大覆盖范围，本票未做。 |
| `xyz` / `para` / `mkts` 的抵押代币是什么 | **部分查不到** | `perpDexs` 响应中无 `collateralToken` 字段；`perpDexStatus` 的 `totalNetDeposit` 未标注单位币种。【推断】按 `sendAsset` 文档「Use "" to specify the default USDC perp DEX」与主流用法应为 USDC，但**未见对这三个 DEX 的逐个确认**。 |
| 部署者身份的现实世界主体（谁是 `0x8880…0888`） | **未查** | 超出本票范围；第三方博客不作依据。 |

---

## 四、可行性判定：Hyperliquid 美股腿能不能承载多层隔离资金

### **有条件可行。**

**可行的部分（已有官方原文或实测支撑）**：
- 多层资金隔离的**机制存在**：DEX 级隔离（HIP-3 §Spec.2 原文）+ 地址级隔离（clearinghouse "for each address" 原文）+ 子账户在最激进的合并模式下仍分开（portfolio margin 原文）。实测证实同一地址在不同 DEX 上有各自独立的余额与可提取额。
- **美股标的够多**：`xyz` 上约 57 个美国上市股票/ADR/ETF + 一个宽基指数 XYZ100，`para` 再加 13 个（但流动性不可用）。
- **期货 / 加密永续同场**：主 DEX 177 个加密永续 + `xyz` 上的商品（GOLD/SILVER/COPPER/BRENTOIL/CL/NATGAS…）与 FX，**同一套 API、同一个私钥体系**——编排者"也要能做期货/加密永续"这条一次性满足。
- **可程序化**：统一的 `/exchange` 与 `/info` 端点，HIP-3 资产 ID = `100000 + perp_dex_index * 10000 + index_in_meta`（官方原文），跨 DEX + 跨子账户的资金调度有原生 action（`sendAsset`）。
- **容量够**：`xyz` 净入金 13.2 亿美元，`xyz:AAPL` 单标的 OI 上限 1.5 亿美元，一线大盘股 10 万美元单笔 2–10 bp。

**必须满足的条件（缺一即塌）**：
1. **每层配一个独立地址**（sub-account），且**逐层显式设置 `standard` abstraction**，不依赖默认值。默认值文档没写，**必须实测确认后再上线**。
2. **每层配独立的 API wallet**（官方明确建议，避免 nonce 碰撞）。子账户配额：3 + 2×子账户数。
3. **确认已废弃的 DEX abstraction 处于关闭状态**（它会自动从主 DEX 抽抵押品到 HIP-3 DEX，直接击穿层间隔离）。
4. **接受 ADL 的残余耦合**：两层不要在同一 DEX 同一标的上持相反方向的高杠杆仓位。
5. **接受部署者风险不可对冲**：`haltTrading` 可以随时按当时 mark price 强制结算你的全部仓位，**无时间锁、无预告、无赔付**；50 万 HYPE 的罚没是烧掉的，不赔用户。这是**结构性单点**，只能靠分散场所（不把全部美股腿压在 `xyz`）来缓解。
6. **接受 oracle 停更风险**：停更 10 秒后 mark price 落到本地盘口中价；在薄标的上这是真实的强平通道。`xyz` 的 oracle 由部署者转授的单个热钱包 `0x1234…acec` 推送。

**子账户数量对"多层"的现实约束**：冷启动 **0 层**；跑到 10 万美元成交量后 **10 层**；50 层需要 40 亿美元累计成交量。**若编排者的设计需要超过 10 个资金层，这条会先撞墙。**

**推荐的场所分工（基于本次深度基线，样本量 = 1 需复核）**：
- 美股个股腿 → **只用 `xyz`**，且只在 NVDA / GOOGL / TSLA / MU / INTC / MSTR / COIN / PLTR / MSFT / AAPL / META / AMD 这一批做；
- 宽基指数腿 → **`xyz:XYZ100`**（容量与成本双双碾压 `mkts:US500`）；
- **`para` 排除**；`mkts:US500` 排除，`mkts:USTECH` 仅作备份且注意卖侧 43 bp。

---

## 五、必须拿 API key 或真实下单才能回答的问题（移交第二程）

1. **子账户能不能在 `xyz` 上下单成交？**（`vaultAddress = 子账户地址` + `asset = 100000 + dex_index*10000 + idx`，下一笔最小额限价单）——本票只有推断链，无实测。
2. **新建账户 / 新建子账户的 abstraction mode 默认值是什么？** 查 `{"type":"userDexAbstraction","user":…}` 与 `userSetAbstraction` 的当前态；若默认是 `unifiedAccount`，隔离默认是破的。
3. **`userSetAbstraction` 能不能对子账户单独生效？** 文档说 `user` 字段"Can be a sub-account"，需实测确认签名与生效。
4. **两个子账户在 `xyz` 上各自开仓后，一个爆仓是否真的不动另一个？** 只能用小额真实仓位验证（或先在 testnet 上跑）。
5. **`sendAsset` 跨 DEX + 跨子账户划转的实际耗时、失败模式、最小额。**
6. **20 档以外的真实深度**（不需要 key，但需要 `nSigFigs` 聚合或 WS 逐档订阅，归 [#933](https://github.com/xy7365527-lang/NewChanlun/issues/933)）。
7. **降 margin table 是否改已开仓位的强平价**——只能靠观察部署者的历史操作或实盘验证。
8. **Binance 侧 `POST /fapi/v1/stock/contract` 子账户能否独立签**（票面第三件，本票未涉及）。
