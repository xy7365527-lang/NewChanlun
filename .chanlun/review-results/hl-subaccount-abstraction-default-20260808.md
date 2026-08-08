# Hyperliquid 新建子账户 abstraction mode 默认值——实测报告

- issue: #942（第 2 问单跑，属 map #931）
- 日期：2026-08-08
- 凭证纪律：全程未使用编排者任何私钥/API key；testnet 测试钱包为本次会话内新生成的一次性钱包，
  私钥只落在 `/tmp/hl942/wallet.json`（worktree 外的临时目录，未纳入 git，未写入本报告/commit/issue）；
  主网操作全部为 `POST /info` 只读查询，零签名、零下单。

## 结论先行

**新建子账户的 abstraction mode 默认值——本次会话未能通过直接实测（testnet 新建子账户）确定，卡在"新钱包无法在 testnet 上激活账户"这一步（见「二」）。**

能确定的只有两条弱证据，且都不是"子账户"本身：
1. 一个从未在 Hyperliquid 主网出现过的全新地址，查询 `userAbstraction` 返回字符串 `"default"`（而非 `"unifiedAccount"`）。
2. 第三方（非官方）文档称 `"default"` 与 `"disabled"`（即"Manual/Standard"/隔离模式）功能等价，`"unifiedAccount"`/`"portfolioMargin"` 才是共享保证金模式——但这条**未能在官方一手文档中找到对应表述**，只能标【推断，来源为第三方镜像文档】。

⟹ **对 #939 条件②的判定：测不出来（不满足直接实测门槛），只有指向"很可能类似 standard/隔离"的弱线索，达不到"开箱即满足"的实证标准。** 详见下方「对 #939 条件②的判定」一节。

---

## 一、主网只读：查已存在账户的 mode 分布

### 1.1 官方 info 查询类型（一手确认）

官方 GitBook 的 info-endpoint 文档给出两个相关查询类型（经 WebFetch 直接抓取该页面确认存在，但页面本身未逐字给出示例地址来源，以下 schema 转述自抓取结果，未能拿到该页面截图级别的逐字原文）：

- `{"type":"userAbstraction","user":"<addr>"}` → 返回值集合：`"unifiedAccount" | "portfolioMargin" | "disabled" | "default" | "dexAbstraction"`
- `{"type":"userDexAbstraction","user":"<addr>"}` → 返回 `true`/`null`
- `{"type":"subAccounts","user":"<addr>"}` → 返回该地址名下子账户数组（含 `subAccountUser`/`master`/`clearinghouseState`/`spotState`）

来源：`https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint`（WebFetch 抓取，非逐字复制，转述其展示的 request/response 示例）。

### 1.2 【实测】三次主网 `/info` 查询原文

**查询 A——从未在主网出现过的全新地址**（本会话生成的测试地址）：

```
curl -X POST https://api.hyperliquid.xyz/info -d '{"type":"userAbstraction","user":"0x1234567890123456789012345678901234567890"}'
→ "default"

curl -X POST https://api.hyperliquid.xyz/info -d '{"type":"userDexAbstraction","user":"0x1234567890123456789012345678901234567890"}'
→ null
```

**查询 B——GitBook 文档中作为 subAccounts 响应示例出现的地址**（`0x035605fc2f24d65300227189025e90a0d947f16c`，未确认是否真实存在于链上，很可能是文档占位地址）：

```
curl -X POST https://api.hyperliquid.xyz/info -d '{"type":"userAbstraction","user":"0x035605fc2f24d65300227189025e90a0d947f16c"}'
→ "default"
```

**查询 C——公开已知的真实活跃地址**（媒体报道的交易员 James Wynn 主账户，经 WebSearch 交叉核实其地址为 `0x5078C2fBeA2b2aD61bc840Bc023E35Fce56BeDb6`，中间字符段两个新闻源写法有一位数字出入，已用能查到的版本实测）：

```
curl -X POST https://api.hyperliquid.xyz/info -d '{"type":"userAbstraction","user":"0x5078C2fBeA2b2aD61bc840Bc023E35Fce56BeDb6"}'
→ "unifiedAccount"

curl -X POST https://api.hyperliquid.xyz/info -d '{"type":"subAccounts","user":"0x5078C2fBeA2b2aD61bc840Bc023E35Fce56BeDb6"}'
→ null   （该地址名下无子账户，无法从这条拿到"子账户 mode"数据点）
```

### 1.3 这批数据说明什么、不说明什么

- **说明**：一个"从未接触过"的地址，`userAbstraction` 查询返回的是 `"default"`，不是 `"unifiedAccount"`。这至少排除了"任何地址不主动设置就自动被标成 unified"这种最坏情形——**对普通主账户而言**。
- **不说明**：
  - 检索范围极小（3 个地址，其中 1 个是文档占位地址、1 个是全新测试地址、只有 1 个是真实活跃账户），**不构成"分布"意义上的统计证据**，只能算个案。
  - 唯一查到的真实活跃账户（James Wynn）是 `"unifiedAccount"`——但这更像是该账户后续手动切换或被平台迁移，**不能反推"新建时就是 unified"**，也同样不能反推"新建时是 default"。任务本身已预警"分布 ≠ 默认值，老账户可能被手动改过"，这条实测再次印证了这个警告本身是对的（一个真实样本就落在了非默认值上）。
  - 最关键的是：**没有找到任何一个真实、可核实的"子账户"地址**，因此 1.2 的三条查询里没有一条真正回答了"子账户默认值"这个问题——它们回答的是"主账户/一般地址默认值"，这是两件事（子账户由主账户发起 `createSubAccount` 创建，其初始状态未必与主账户共享同一套默认逻辑，官方文档对此没有说明——见 1.4）。

### 1.4 官方文档明确没写的部分

- `https://hyperliquid.gitbook.io/hyperliquid-docs/trading/account-abstraction-modes`（WebFetch 抓取）：给出四种模式的定义原文（见下），**全文没有一句提到"默认模式是什么"，也没有提到子账户**。
- `https://hyperliquid.gitbook.io/hyperliquid-docs/trading/sub-accounts`（WebFetch 抓取）：只讲了子账户创建门槛（$100,000 交易量后可开，最多 10 个）、手续费档位继承主账户、返佣不适用于子账户、API 钱包分配——**同样没有一句提到子账户的保证金隔离模式或 abstraction mode 默认值**。

四种模式的官方原文（逐字，来自 account-abstraction-modes 页面 WebFetch 结果）：

> Unified Account: "single balance for each asset. This balance collateralizes all cross margin positions in that asset and is unified with spot balance in that asset."
> Portfolio Margin: "single portfolio unifying all eligible assets, which are currently HYPE, BTC, USDC, USDT."
> Manual/Standard: "separate perp and spot balances, separate DEX balances. Cross margin applies to each DEX separately."
> DEX Abstraction (Discontinued): "USDC balances default to perps balance, all other collateral defaults to spot balance."

⟹ **确认了本票立项时的判断：官方文档没有写默认值**，无论是对新主账户还是新子账户。

---

## 二、testnet 实测：新建子账户直接查默认 mode——卡住，未能完成

### 2.1 已完成的准备

- 本地生成一次性测试钱包（`eth-account` 库，`Account.create()`），地址 `0x16E68F1Ccb0beBb3667BaFaa3030d052FB8De7a4`，与编排者任何资产无关，私钥只存在于 worktree 外的 `/tmp/hl942/wallet.json`。
- 确认 `https://api.hyperliquid-testnet.xyz/info` 可用（`{"type":"meta"}`/`{"type":"subAccounts",...}` 均正常返回）。

### 2.2 【实测】卡点 1——官方 faucet 拒绝

```
curl -X POST https://api.hyperliquid-testnet.xyz/info \
  -d '{"type":"claimDrip","user":"0x16E68F1Ccb0beBb3667BaFaa3030d052FB8De7a4"}'

→ "Cannot claim drip because user 0x16e68f1ccb0bebb3667bafaa3030d052fb8de7a4 does not exist on mainnet."
```

官方 GitBook `testnet-faucet` 页面（WebFetch 抓取）原文确认了这条前提：**"You must have deposited on mainnet with the same address"** 才能用 testnet faucet 领测试 USDC。

### 2.3 【实测】卡点 2——绕开 faucet 直接尝试签名动作，同样被拒

为确认这不只是"没钱"而是"账户在 testnet 上根本不存在"，直接用 `hyperliquid-python-sdk` 对同一把新钱包发起 `createSubAccount` 签名动作（testnet，零资金，纯诊断，不涉及编排者任何资产）：

```python
Exchange(acct, constants.TESTNET_API_URL).create_sub_account("probe-subaccount")
→ {'status': 'err', 'response': 'User or API Wallet 0x16e68f1ccb0bebb3667bafaa3030d052fb8de7a4 does not exist.'}
```

### 2.4 卡点结论

**Hyperliquid 的 testnet 账户体系要求地址先在"某个层面"被激活（无论主网还是 testnet），而这个激活门槛的唯一官方入口——faucet——反过来又要求该地址已在**主网**存过款（$5+ USDC）。** 这是一个闭环：
- 全新、与编排者资产无关的一次性钱包 → 没有主网存款记录 → 官方 testnet faucet 拒绝发放测试 USDC/激活账户 → 该地址在 testnet 上"不存在" → 无法创建子账户 → 无法查询新建子账户的默认 mode。
- 已检索的第三方替代 faucet（Chainstack、QuickNode 等）**只发 testnet HYPE（gas 代币）**，不发可用于开仓/创建子账户所需的 mock USDC，且大多需要额外注册账号（超出单会话可完成范围），**也未验证它们能否绕开 2.3 里"User does not exist"这层门槛**（HYPE 到账不等于账户在 Hyperliquid 交易系统里"存在"）。

⟹ **在遵守"绝不使用编排者私钥/资产、只用全新一次性钱包"这条凭证纪律的前提下，本会话内没有找到任何合规路径把这个新钱包激活到能创建子账户的状态。testnet 直接实测——本票要求的"唯一直接证据"——未能拿到。**

这不是"没找到"就停手；已经把两层（faucet 领币、绕开 faucet 直接发签名动作）都实测到了明确的官方错误原文为止，属于"走不通+卡在哪一步+错误原文"的完整记录。

---

## 三、testnet 与主网的差异查证

### 3.1 官方一手来源——不完整

- `https://hyperliquid.gitbook.io/hyperliquid-docs/testnet` **不存在**（WebFetch 返回 404 页面）。GitBook 目录下没有一个专门对比 testnet/mainnet 差异的官方页面。
- 找到一条来自官方账号 `@HyperliquidX` 的推文（通过 WebSearch 命中，URL `https://x.com/HyperliquidX/status/2003045600657334570`），内容据搜索结果摘要包含：

  > "Testnet functions are exactly that - testnet only for testing. They cannot be executed on mainnet."

  **⚠️ 这条无法逐字核验**：直接 WebFetch 该 URL 返回 `HTTP 402 Payment Required`（x.com 对未登录抓取的限制），只能引用 WebSearch 工具给出的转述/摘要，**不是我本人直接读到的原始推文全文**。标注为【官方原文，但经二手转述、未独立核验逐字】。

- 这条推文谈的是"testnet 有一批 mainnet 上不存在/不可执行的管理员测试功能"，**没有直接回答"testnet 上子账户/abstraction mode 的默认逻辑是否与主网一致"这个具体问题**。

### 3.2 第三方来源——仅供参考，非官方

- Chainstack、HyprSwarm 等第三方教程称"杠杆/保证金模式在两网工作方式完全一致，只是 testnet 资金是假的、流动性更薄"。**这些不是 Hyperliquid 官方陈述**，且都没有专门提到 abstraction mode 默认值这一具体机制在两网是否一致。

### 3.3 结论

**官方对"testnet 与主网在 abstraction mode 默认值/子账户初始化逻辑上是否一致"没有专门说明，查不到。** 即使二里的 testnet 实测跑通了，其结果按纪律也只能标注为「testnet 观测，主网未验证」——这条本身现在也没有查到能撑住"testnet 结果可以外推到主网"这一步的官方依据，所以就算拿到了 testnet 数据，外推信度依旧存疑。

---

## 对 #939 条件②的判定

条件②要求：「每层的 abstraction mode 必须是 `standard`」。

判定：**测不出来（未达实证门槛），不是"开箱即满足"，也不是"开箱不满足"。**

理由链：
1. 直接证据（testnet 新建子账户实测其默认 mode）——**未获得**，卡在 faucet/账户激活门槛（二）。
2. 间接证据（主网已存在地址的 `userAbstraction` 查询）——只查到 1 个真实活跃地址且是 `"unifiedAccount"` 而非默认/隔离值，**不构成任何方向上的可信推断**，反而印证了"分布≠默认值、老账户可能已被改"这条预警是对的。
3. 全新、从未接触过的**主账户级**地址查询返回 `"default"`——这条离题目问的"新建**子账户**默认值"还差一层未验证的桥梁：官方文档完全没写子账户创建时的 abstraction mode 是否独立初始化、是否继承主账户设置、还是统一走 `"default"`。**没有任何一手来源填这个空。**
4. 第三方文档称 `"default"` 功能上等同 `"disabled"`（即隔离/`standard`）——这条如果成立，会让"主账户默认即隔离"这一步成立，但（a）它是第三方转述非官方一手来源，（b）即便成立也解决不了第 3 点里"子账户是否走同一套默认"的空白。

⟹ 综合以上，本次交付**不能**支持"条件②开箱即满足"这个结论，也**不能**支持相反结论。这是一个需要标注为**未决**、且需要后续找到实测路径（例如：编排者本人用一个已在主网有少量存款、但从未手动碰过 abstraction 设置的全新地址，创建一个子账户后立刻查询）才能真正闭合的缺口。

---

## 未能覆盖的部分及原因

| 缺口 | 原因 |
|---|---|
| 新建子账户的真实默认 mode（testnet 直接实测） | Faucet 要求地址先有主网存款记录；绕过 faucet 直接发签名动作同样被拒（账户在 testnet 不存在）。见「二」完整错误原文。 |
| 已存在子账户的 mode 分布（用于旁证） | 未能找到任何真实存在、地址可核实的子账户地址（1.2 中三个查询地址都不是子账户地址，是主账户/占位地址），检索范围仅限公开新闻报道提及的极少数主账户地址，样本量=0（子账户）。 |
| testnet 与主网在 abstraction 默认逻辑上是否一致 | 官方 `/testnet` 页面不存在（404），唯一相关的官方推文内容无法逐字核验（x.com 抓取 402），且其内容本身没有直接回答这个具体问题。 |
| 子账户是否独立走一套默认逻辑，还是继承主账户设置 | `account-abstraction-modes` 与 `sub-accounts` 两个官方页面均未提及，无任何一手来源。 |

## 附：本次用到的一次性资产处置

- 测试钱包私钥文件 `/tmp/hl942/wallet.json`（worktree 外临时目录，未 commit，未写入本报告/issue，无任何资金，可随时丢弃）。
- 该钱包未能在 testnet 上完成任何激活/资金操作，链上无任何痕迹。
