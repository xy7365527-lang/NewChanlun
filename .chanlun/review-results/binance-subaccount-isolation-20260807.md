# Binance 子账户隔离勘察——喂 #932 / #936（多层资金执行场所，续 #878）

调研范围：只读公开资料（Binance 官方文档、公告、Academy、免签名公开端点）。**不涉及需要
API key 签名的私有端点**（如实际下单、实际子账户创建、实际划转），这部分在「未查到的部分」和
「必须拿 key 实测」两节里单列。

前置：本票是 issue #878（`.chanlun/review-results/issue878-exchange-isolated-margin-20260804.md`）
的延伸。#878 已裁定「交易所侧最细隔离粒度 = symbol×方向，按重逐仓须走子账户映射」，且已查过
Binance 子账户机械要件（独立持仓/逐仓、划转即时免费、每子 30 API Key、上限 VIP1=20→VIP9=200）。
本票**不重复**这些，只新查 TradFi 股票永续专属的资格问题和 #878 未覆盖的细节，并在用到 #878
结论处标注引用来源。

---

## 结论先行（七问各一句话）

1. **【推断，非官方明文】** 没有查到任何官方文档明确写「子账户不能交易 TRADIFI_PERPETUAL」，
   但也没有查到明确写「子账户可以交易」——可查到的唯一强制门槛是**产品级**的
   `POST /fapi/v1/stock/contract`（"Sign TradFi-Perps agreement"）签署，这是账户级/API-key 级
   动作，理论上子账户各自持有独立 API key、应各自可签；**这一条必须拿子账户 API key 实测**，
   不能当作已证实。
2. 【官方原文，间接层面】子账户的隔离边界官方给出的是**账户级**描述（子账户是独立的资金/持仓/
   保证金主体，主账户可查询但资产互不计入），但**没有查到**官方逐字写明「子账户爆仓不影响主账户
   或其他子账户」这句断言本身——最接近的证据是 API 设计上每个子账户有独立的
   `forceLiquidationBar`/`marginCallBar`/`normalBar`（强平/追保/起始保证金率），指向账户级隔离，
   但这是**推断**不是原文断言。
3. 【官方 FAQ】普通用户（Regular User，即通常说的 VIP0）**可以**开子账户，上限 **5 个**；
   VIP 等级越高上限越高，第三方整理的分档表（20/30/40/50/60/70/80/100/200 对应 VIP1–VIP9）与
   官方页面结构一致但**具体数字未能从实时渲染页面逐字核实**（页面需要登录/JS 渲染）。
   **机构/经纪商子账户（Managed Sub-Account）是完全不同的一套**——面向"投资人委托交易团队"的
   资管场景，不是本方案要用的"重"容器，本票认定与本方案无关。
4. 【官方文档】子账户可以各自持有 API key（每子最多 30 个，#878 已查），主账户可通过 API
   创建子账户（`Create a Virtual Sub-account`）、开通期货（`Enable Futures for Sub-account`）、
   查询列表（`Query Sub-account List`）、查询持仓风险（`Get Futures Position-Risk of Sub-account`）。
5. 【官方文档】划转端点比 #878 记录的更细：除主↔子外，**存在子账户之间的直接划转端点**
   `POST /sapi/v1/sub-account/transfer/subToSub`（同一主账户下），以及主账户发起的
   `futures/internalTransfer`（限速：主账户每分钟最多 2000 次），均为 IP Weight 1、即时到账；
   官方页面未提及手续费，结合 #878「即时免费」的结论方向一致。
6. 【官方文档】Hedge Mode（双向持仓）**证实**每个 symbol 最多 2 个仓位——`LONG` 和 `SHORT`
   各一个（`positionSide` 参数只接受 `BOTH`/`LONG`/`SHORT`），**不提供更细的分仓**；
   Portfolio Margin 与 Isolated Margin **互斥**（官方要求开 PM 前必须没有 Isolated Margin
   持仓），即 PM 是隔离的削弱方向而非加强方向，与 #878 结论一致。
7. 【官方公告，逐字引用见下】TradFi Perps 有独立于普通加密永续的费率表：**maker 全档 0%**，
   taker 按 VIP 从 **0.04%（普通用户）降到 0.0085%（VIP4–9）**，自 **2026-03-31 02:00 UTC
   起生效、"until further notice"**（未查到失效日期，需视为仍在生效，但页面本身"No records
   found"未登录不可核实实时状态）；比普通加密永续（maker 0.02%/taker 0.05%）明显更低。
   **止损/条件单额外费率——查不到**，官方文档均未提及条件单单独计费，一般 Binance 惯例是
   触发后按对应订单类型（LIMIT/MARKET）计费，但这条对 TradFi Perps 本身**未见逐字确认**。

---

## 逐问详答

### 问 1：133 个 TRADIFI_PERPETUAL 股票永续，子账户能不能交易？

**这是七问里唯一"不成立就塌方案"的一条，也是查得最不确定的一条。**

【实测，2026-08-07】`GET https://fapi.binance.com/fapi/v1/exchangeInfo` 返回 133 个
`underlyingType=EQUITY` 且 `contractType=TRADIFI_PERPETUAL` 的合约，抽样 `TSLAUSDT` 字段：
```
{'symbol': 'TSLAUSDT', 'contractType': 'TRADIFI_PERPETUAL', 'status': 'TRADING',
 'underlyingType': 'EQUITY', 'underlyingSubType': ['TradFi'],
 'permissionSets': ['GRID', 'COPY', 'RPI', 'DCA', 'PSB']}
```
`permissionSets` 字段里没有任何指向"排除子账户"的标记，但这个字段本身语义未知（可能是
"该合约支持哪些交易模式"，不是"哪些账户类型可用"），**不能当证据用**。

【官方文档，逐字引用】USDⓈ-M 期货 API 有专属前置端点：
`POST /fapi/v1/stock/contract`——"Sign TradFi-Perps agreement"
（`https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/TradFi-Perps`）。
Portfolio Margin API 有对应版本：`POST /papi/v1/um/stock/contract`，request weight 5，
成功返回 `{"code": 200, "msg": "success"}`
（`https://developers.binance.com/docs/derivatives/portfolio-margin/trade/PM-TradFi-Perps`）。

【实证，第三方开发者踩坑，非官方但可信度高——freqtrade GitHub issue #12900】未签署时下单报错：
```
binance {"code":-4411,"msg":"Please sign TradFi-Perps agreement contract fapi."}
```
这证明：**交易 TRADIFI_PERPETUAL 前必须显式调用一个签名端点签协议**，这是产品级门槛，
不是天然放开的。

【查不到】以下三件事，翻遍官方公告、Academy 文章、FAQ（`Perpetual Futures on Traditional
Assets`：`https://www.binance.com/en/support/faq/detail/fe7dcdf24f1943d98b368f5f9f744398`）、
上线公告（2026-06-01/2026-07-02 两批公告）均**未提及**：
- 子账户能否独立调用 `/fapi/v1/stock/contract`（即：主账户签过协议后，子账户是否自动继承，
  还是每个子账户要单独签）；
- 是否存在账户类型白名单（比如仅主账户/仅个人账户/仅机构账户）；
- 地区限制清单具体国家名单（公告只有通用免责声明"may not be available in your region"，
  第三方新闻稿提到"excluding U.S."但**未见官方一手确认**）。

**090 照实**：查不到就是查不到，不能写"应该能"。这条必须拿子账户 API key 实测
`POST /fapi/v1/stock/contract` 才能收口。

### 问 2：子账户隔离到底隔到哪一层

【官方文档，间接】Isolated Margin 官方定义（Spot/Margin 侧，逐字）：
> "If a trader's position is liquidated in Isolated Margin mode, instead of their entire margin
> balance, only the Isolated Margin balance gets liquidated" ——
> Binance Isolated Margin Trading Guide
> (`https://www.binance.com/en/support/faq/binance-isolated-margin-trading-guide-0135c8c00a4240f695ee71a0d18efb08`)

这条讲的是"仓位级隔离"（一个 symbol 一个逐仓账户），**不是"子账户级隔离"的官方原文**。

【官方文档，账户级证据链】子账户资产管理 API 有独立查询端点
`Get Detail on Sub-account's Margin Account (For Master Account)`
（`https://developers.binance.com/docs/sub_account/asset-management/Get-Detail-on-Sub-accounts-Margin-Account`），
返回字段含每个子账户独立的 `forceLiquidationBar`（强平保证金率）、`marginCallBar`（追保线）、
`normalBar`（起始保证金率）——**结构上**每个子账户的强平风险是独立追踪、独立字段，指向
"子账户是独立的保证金/强平主体"。但**没有查到**一句官方原文直接断言"子账户 A 爆仓不会导致
子账户 B 或主账户被强平/扣款"。

【推断链，标注清楚】：子账户在币安架构里等价于一个独立资金账户（有自己的余额、持仓、保证金率
字段），主账户只是"控制中心"（能查询、能划转、能管理 API key），不是"共享保证金池"的一部分
——这个推断建立在①子账户独立余额②子账户独立强平字段③#878 已验证"子账户持仓/逐仓相互独立"
三点上，**但推断链本身不是官方逐字断言，需要用真实测试账户开两个子账户、一个打爆一个观察另一个
是否受影响来验证**（即"必须实测"清单第 2 条）。

### 问 3：子账户数量上限与开通门槛

【官方 FAQ，逐字/摘要引用】
`https://www.binance.com/en/support/faq/binance-sub-account-functions-and-frequently-asked-questions-360020632811`：
> "Regular users are limited to five sub-accounts, while the number of sub-accounts for VIP
> users varies based on the VIP level of their master account."

即：**VIP0（普通用户）可以开子账户，上限 5 个**——这直接回答了任务给的"注意区分普通用户能不能开"
的问法：能开，不需要先升级 VIP。

【第三方整理表，未逐字核实官方原页，标注为间接】VIP 分档上限：
VIP1=20、VIP2=30、VIP3=40、VIP4=50、VIP5=60、VIP6=70、VIP7=80、VIP8=100、VIP9=200
（个人账户口径）。这与 #878 报告记录的"VIP1=20→VIP9=200"方向一致，但**具体中间档位数字
（30/40/50/60/70/80/100）本票未能从 `binance.com/en/vip-portal/sub-account-limit` 实时页面
逐字核实**——该页需登录才渲染表格内容，WebFetch 只抓到静态壳。**090 照实**：中间档具体数字
按"已查到但未逐字核实"处理，不当作确凿事实用于关键决策；两端（VIP0=5、VIP9=200）有独立信源
交叉确认，可信度较高。

VIP1 门槛（编排者若考虑升级）【官方页面，逐字】：
`https://www.binance.com/en/fee/trading` ——30 天交易量 "≥ 1,000,000 USD" 且 BNB 持仓
"≥ 5 BNB"（页面用 "and" 连接，即需同时满足；此为 WebFetch 对渲染页面的摘录，非源 HTML 逐字，
已尽力但建议登录页面复核）。

【机构/经纪商子账户 vs 普通子账户，官方区分，逐字/摘要引用】：
> "The Managed Sub-Account function...is exclusively offered to Institutional clients and
> VIP users." ——
> `https://www.binance.com/en/support/faq/how-to-get-started-with-managed-sub-account-function-and-frequently-asked-questions-0594748722704383a7c369046e489459`

Managed Sub-Account（资管子账户）是"投资人 Master Account 委托专业交易团队代为交易"的产品，
**交易团队不持有资产所有权、投资人看不到交易团队的具体持仓明细**——这套东西是给"资管代客"用的，
和本方案"编排者自己在同一标的上跑多个独立资金层"的需求**完全不是一回事**，本票判定**不适用**，
后续不必再查这条线。本方案要用的是**普通（Regular）子账户**。

### 问 4：子账户的 API 能力

【官方文档，端点清单，逐字引用端点名】
`https://developers.binance.com/docs/sub_account/account-management`：
- `Create a Virtual Sub-account`（主账户创建子账户）
- `Enable Futures for Sub-account` / `Enable Options for Sub-account`
- `Get Futures Position-Risk of Sub-account` / `... V2`
- `Get Sub-account's Status on Margin Or Futures`
- `Query Sub-account List`
- `Query Sub-account Transaction Statistics`

子账户各自可创建 API key：**每子最多 30 个**（#878 已查，本票未重查）。子账户是否也能直接
（不经主账户）自行调用 API 创建/管理——**查不到**，官方文档结构把这些端点归在"主账户视角"
（Account Management，需主账户签名调用），暗示子账户管理动作**主要由主账户发起**，但子账户
自己的 API key 应该能做**交易类**操作（下单、查仓）——这条与问 1 的"子账户能否单独签
TradFi-Perps 协议"是同一类"需要子账户自己 key 才能验证"的问题，归入必须实测清单。

### 问 5：子账户间资金划转

【官方文档，端点清单+限速，逐字/摘要引用】
`https://developers.binance.com/docs/sub_account/asset-management`：
- `POST /sapi/v1/sub-account/futures/transfer`（主账户发起，主子期货账户间）
- `POST /sapi/v1/sub-account/futures/internalTransfer`——限速原文摘录："A master account can
  transfer at most 2000 times per minute"
- `POST /sapi/v1/sub-account/margin/transfer`
- `POST /sapi/v1/sub-account/universalTransfer`（通用划转）
- **`POST /sapi/v1/sub-account/transfer/subToSub`**——子账户直接发起、划给"同一主账户下的另一
  子账户"，不必经过主账户中转
- `POST /sapi/v1/sub-account/transfer/subToMaster`（子→主）

均为 IP Weight 1（低权重，接近免费调用）。官方页面**未提及手续费数字**，结合 #878「主账户
归口即时免费」的结论，本票推断子账户间划转大概率同样免费即时，但**没有查到逐字确认子→子
路径本身是否收费**——标注为【推断，未逐字确认】。

### 问 6：有没有比子账户更轻的隔离机制

【官方文档，逐字/参数引用】Hedge Mode（`Change Position Mode`，
`https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/Change-Position-Mode`）：
`positionSide` 参数取值 "Default `BOTH` for One-way Mode; `LONG` or `SHORT` for Hedge Mode"。
**证实**：Hedge Mode 下一个 symbol 最多 2 个仓位（多头一个、空头一个），**不提供第三个仓位**，
不能当"重"的分仓工具用（编排者需要的是同方向也能分层，Hedge Mode 做不到）。

【官方文档，逐字】Portfolio Margin 与 Isolated Margin **互斥**："for USDⓈ-M Futures, you must
have no open orders, open positions, or negative balances, and not be using the Isolated
Margin or Multi-Assets mode" 才能开 PM——即 PM 是隔离的**反方向**（合并保证金池，削弱隔离），
不是增强工具，与 #878 结论一致。

没有查到其他比子账户更轻量的分仓机制（比如"账户内多子钱包"之类）——**查不到**，说明子账户
仍是当前已知的最小可用隔离容器。

### 问 7：手续费

【官方公告，逐字引用】
`https://www.binance.com/en/support/announcement/detail/a4c3f1957f2b4e69902985154235c3b1`
（Binance Futures Updates Trading Fee Discount for TradFi Perps）：
- Maker 费率：全档 **0%**
- Taker 费率：普通用户至 VIP3 **0.04%–0.0256%**；VIP4–VIP9 **0.015%–0.0085%**
- BNB 支付额外 10% 折扣
- 生效时间：**"2026-03-31 02:00 (UTC) until further notice"**

对比普通加密永续（第三方摘要口径，非本票逐字核实）：maker 约 0.02%、taker 约 0.05%——
TradFi Perps 明显更便宜，尤其 maker 直接免费。

【查不到】止损单/条件单（STOP/STOP_MARKET/TAKE_PROFIT 等，exchangeInfo 里
`orderTypes` 已确认 TradFi 合约支持这些类型）是否有额外费率——翻遍上述公告和费率页
（`https://www.binance.com/en/fee/tradFiFee` 需登录才渲染，抓到的是"No records found"空壳）
均未见专门条款。**Binance 一般惯例**是条件单触发后按最终成交类型（限价单/市价单）计费，
不额外收"挂单费"，但这是**行业常识推断**，不是本次查到的 TradFi Perps 专属官方确认。

---

## 未查到的部分与原因

| 编号 | 未查到内容 | 原因 |
|---|---|---|
| U1 | 子账户能否独立调用 `POST /fapi/v1/stock/contract` 签署 TradFi-Perps 协议 | 需要真实子账户 API key 签名调用，公开渠道无记录 |
| U2 | TRADIFI_PERPETUAL 是否有账户类型白名单（子账户/机构/地区） | 官方公告/FAQ/Academy 文章均只有通用免责声明，无逐条清单 |
| U3 | 子账户爆仓是否影响主账户/其他子账户——直接官方断言 | 只找到结构性证据（独立字段），未找到逐字断言句 |
| U4 | 子账户数量上限中间档（VIP2–VIP8）官方逐字数字 | `vip-portal/sub-account-limit` 页面需登录渲染，WebFetch 只抓到静态壳 |
| U5 | `binance.com/en/fee/tradFiFee` 实时费率表（VIP0 具体 taker 数字是否仍是 0.04%） | 页面需登录，"No records found" |
| U6 | TradFi Perps 止损/条件单专属费率条款 | 未见官方专属条款，只有行业惯例推断 |
| U7 | 子账户间划转（`subToSub`）是否收手续费 | 端点文档未列费用字段 |
| U8 | 地区限制具体国家清单（是否含中国大陆/美国等） | 官方只写"may not be available in your region"，无清单 |

---

## 可行性判定：**有条件可行**

**条件**（缺一不可，按优先序）：

1. **必须拿真实子账户 API key 实测 `POST /fapi/v1/stock/contract` 能否成功签署，并实测能否
   对 133 个 TRADIFI_PERPETUAL 合约下单**——这是问 1，也是本方案唯一的塌方点。文档层面查不到
   禁止，也查不到明确允许，是纯粹的"未证实"状态，不能默认放行。
2. 子账户隔离的"账户级独立"在结构上（独立余额、独立强平字段、#878 已证实的独立持仓/逐仓）
   证据链是完整的，但**没有一句官方原文直接断言"爆仓不传染"**——建议实测：开两个测试子账户，
   一个用小额故意触发强平，观察另一个/主账户是否受任何影响（余额、API 限速、风控标记）。
3. VIP0 可以直接开始（5 个子账户免升级门槛），如果编排者要跑的"重"数量 ≤5，现在就能动手；
   超过 5 个才需要考虑冲 VIP1（30 天交易量 ≥100 万美元 或 持 BNB ≥5，具体是 and 还是 or
   本票摘录为 and，建议登录页面复核）。

**死穴（如果 U1 实测为"不能"，方案直接塌）**：TradFi 股票永续本身是 2025 年底才加的新产品线，
门槛是"签一个协议"这种一次性、账户级的动作——如果 Binance 把这个协议签署权限锁定在主账户
（子账户调用被拒），那么"子账户 = 独立资金层可以交易 133 个股票标的"这条链就断了，方案退化成
"子账户只能交易加密永续、股票永续只能在主账户单层跑"，与硬需求②"美股标的越多越好"和硬需求①
"多层资金真隔离"直接冲突。

---

## 与 #878 的关系

本票不重复 #878 已裁定的：symbol×方向逐仓粒度、子账户机械要件（独立持仓/逐仓、划转即时免费、
每子 30 API Key、数量上限方向 VIP1=20→VIP9=200）、Bybit/OKX 对照。本票新增：TradFi 股票永续
专属资格链（问1，含 `-4411` 报错实证）、`subToSub` 直接划转端点、Hedge Mode 仓位数证实、
Managed Sub-Account 与 Regular Sub-Account 的官方区分、TradFi Perps 专属费率表（问7逐字引用）。
