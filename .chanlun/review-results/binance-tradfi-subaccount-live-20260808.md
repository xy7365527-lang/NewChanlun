# Binance 侧三问实测：TradFi-Perps 协议签署与子账户（#942 第 8/9/10 问，明账挂 #953）

- 日期：2026-08-08
- 票：[#942](https://github.com/xy7365527-lang/NewChanlun/issues/942) 第 8/9/10 问；凭证前置票 [#953](https://github.com/xy7365527-lang/NewChanlun/issues/953)；前置调研 [#932](https://github.com/xy7365527-lang/NewChanlun/issues/932)（报告 `binance-subaccount-isolation-20260807.md`）
- 凭证：已从约定位置读取，**本报告不含任何凭证片段**
- 网络：VPN 全程可用（`fapi/v1/ping` 与 `api/v3/ping` 均 200，全程未出现 451）
- 标注约定：`【实测】` / `【官方原文】` / `【推断】`

---

## 结论先行

### 0. ★ key 权限检查（第一步，安全门）

**【实测】不带提现权限，继续执行。**

```
GET https://api.binance.com/sapi/v1/account/apiRestrictions
HTTP 200
{"ipRestrict":false,"createTime":1783828637000,"enableReading":true,"enableFutures":true,
 "enableSpotAndMarginTrading":true,"enableWithdrawals":false,"enableInternalTransfer":false,
 "permitsUniversalTransfer":false,"enableVanillaOptions":false,"enablePortfolioMarginTrading":false,
 "enableFixApiTrade":false,"enableFixReadOnly":false,"enableMargin":false}
```

关键位：`enableWithdrawals: false` ⟹ **无提现权限**，安全纪律第 2 条的中止条件不触发。

⚠️ 顺带两条对后续有影响的读数：`enableInternalTransfer: false`、`permitsUniversalTransfer: false` ⟹ **本 key 无划转权限**，这是第 10 问的第二重阻塞（第一重见下）。

### 1. 三问各一句话

| 问 | 一句话结论 |
|---|---|
| **第 8 问**（子账户能否独立签 TradFi-Perps 协议） | **前半答出、后半测不了**：主账户**已经能交易股票永续**（实盘成交为证，全程未见 `-4411`，我**没有**签任何协议）；**子账户那半测不出来——该账户的子账户功能整体未开通**，连列表都读不到（`-8012` / `-9000`），更建不了子账户（`100001002`）。 |
| **第 9 问**（子账户 KYC 是否继承） | **测不了**，同因：无子账户可测，样本量 = 0。 |
| **第 10 问**（`subToSub` 划转是否真收费） | **测不了**，双重阻塞：① 无子账户；② 本 key 无划转权限（`enableInternalTransfer: false`）。 |

⚠️ **照 090：以上「测不了」一律不作「不支持」解，也不作「应该可以」解。**

---

## 逐问详答

### 第 8 问：子账户能不能独立签署 TradFi-Perps 协议

#### 8.1 签名方式的一个前置发现（影响所有后续调用）

**【实测】该凭证用的是 HMAC-SHA256，不是 Ed25519。** 约定位置里的 `ed25519-*.pem` 与本 API key **不配对**：

```
GET /sapi/v1/account/apiRestrictions   签名方式=Ed25519(base64)
HTTP 400
{"code":-1022,"msg":"Signature for this request is not valid."}

GET /sapi/v1/account/apiRestrictions   签名方式=HMAC-SHA256(hex)
HTTP 200   （返回体见上）
```

⟹ 本报告全部签名请求均用 HMAC-SHA256。**未对 pem 做任何改动。**

#### 8.2 主账户当前状态：**已经可以交易股票永续**

【实测】先做无副作用的校验单：

```
POST /fapi/v1/order/test  {symbol:AAPLUSDT, side:BUY, type:MARKET, quantity:0.01}
HTTP 400
{"code":-4164,"msg":"Order's notional must be no smaller than 5 (unless you choose reduce only)."}

POST /fapi/v1/order/test  {symbol:AAPLUSDT, side:BUY, type:MARKET, quantity:0.02}
HTTP 200
{"orderId":0,"symbol":"","status":"", ... }        ← 校验通过
```

⚠️ **`order/test` 通过不足以证明协议已签**——它是纯校验端点，是否跑协议检查未知。所以又下了**真单**（见「写操作清单」）：

```
POST /fapi/v1/order  {symbol:AAPLUSDT, side:BUY, type:MARKET, quantity:0.02, newClientOrderId:chanlun942probe1}
HTTP 200
{"orderId":140176001,"symbol":"AAPLUSDT","status":"NEW","clientOrderId":"chanlun942probe1","price":"0.00000",
 "origQty":"0.02","executedQty":"0.00","cumQty":"0.00","timeInForce":"GTC","type":"MARKET","reduceOnly":false,
 "closePosition":false,"side":"BUY","positionSide":"BOTH","stopPrice":"0.00000","workingType":"CONTRACT_PRICE",
 "priceProtect":false,"origType":"MARKET","priceMatch":"NONE","selfTradePreventionMode":"EXPIRE_MAKER",
 "goodTillDate":0,"updateTime":1786165144675}
```

成交回执：

```
GET /fapi/v1/userTrades  {symbol:AAPLUSDT, limit:10}
HTTP 200
[{"symbol":"AAPLUSDT","id":7342410,"orderId":140176001,"pair":"AAPLUSDT","side":"BUY","price":"313.20000",
  "qty":"0.02","realizedPnl":"0","quoteQty":"6.2640000","commission":"0.00250560","commissionAsset":"USDT",
  "time":1786165144675,"positionSide":"BOTH","baseQty":"0","marginAsset":"USDT","buyer":true,"maker":false},
 {"symbol":"AAPLUSDT","id":7342411,"orderId":140176029,"pair":"AAPLUSDT","side":"SELL","price":"313.19000",
  "qty":"0.02","realizedPnl":"-0.00020000","quoteQty":"6.2638000","commission":"0.00250551","commissionAsset":"USDT",
  "time":1786165147656,"positionSide":"BOTH","baseQty":"0","marginAsset":"USDT","buyer":false,"maker":false}]
```

⟹ **【实测】`AAPLUSDT`（`underlyingType=EQUITY`、`contractType=TRADIFI_PERPETUAL`）在主账户上真实成交，全程零 `-4411`。**

**⟹ 因此我没有调用 `POST /fapi/v1/stock/contract`。** 原授权是「若需签署则执行」，而**实测证明主账户无需再签**——那一步的写操作（可逆性未知）**在本次没有必要，故不做**。这是本次唯一一处「授权了但没做」的写操作，理由在此明账登记。

⚠️ **这条实测能证明什么、不能证明什么**（照 090 划清）：
- **能证明**：该主账户**当下**可以下单成交 `EQUITY` 永续。
- **不能证明**：协议是「早先由编排者在网页端签过」还是「本账户根本不需要签」。两种成因都与观测一致，**本次无法区分**。要区分需要一个未签署过的干净账户作对照——**没有**。

#### 8.3 `POST /fapi/v1/stock/contract` 这个路径在主网上的存在性：**测不出来**

【实测】主网与 testnet 的 404 形状不同：

```
主网   GET https://fapi.binance.com/fapi/v1/stock/contract        HTTP 404  → Binance 通用 HTML 错误页
主网   GET https://fapi.binance.com/fapi/v1/nonexistent-xyz（对照） HTTP 404  → 同一张 HTML 错误页（逐字相同）
主网   GET https://fapi.binance.com/fapi/v1/stock/agreement（探针） HTTP 404  → 同一张 HTML 错误页
testnet GET https://testnet.binancefuture.com/fapi/v1/stock/contract HTTP 404
        {"code":-5000,"msg":"Path /fapi/v1/stock/contract, Method GET is invalid"}
```

⚠️ **主网的返回与「明确不存在的路径」这个负控**逐字相同** ⟹ 无法从 GET 区分「路径不存在」与「路径存在但方法不对」。** testnet 上则能区分（`-5000` 明示「方法无效」＝路径存在）。

⟹ **【实测】主网上该端点的存在性未证实。** 编排者转来的 testnet 读数（路径存在）**只在 testnet 上成立**，不能直接外推到主网——本次实测正好给出反例形状：**同一个探测手法在两个环境返回不同层级的错误**。

#### 8.4 子账户那半：**整体不可达**

【实测】四个端点一致指向同一堵墙：

```
GET  /sapi/v1/sub-account/list                     HTTP 400  {"code":-8012,"msg":"Sub-account function is not enabled."}
GET  /sapi/v1/sub-account/status                   HTTP 400  {"code":-9000,"msg":"Sub-account function is not enabled."}
GET  /sapi/v1/sub-account/futures/accountSummary   HTTP 400  {"code":-9000,"msg":"the api throw error:Sub-account function is not enabled."}
GET  /sapi/v1/sub-account/universalTransfer        HTTP 400  {"code":-8012,"msg":"Sub-account function is not enabled."}
POST /sapi/v1/sub-account/virtualSubAccount  {subAccountString:"chanlunL1"}
                                                   HTTP 400  {"code":100001002,"msg":"Unsupported operation"}
```

一条旁证说明**这个账户本身是主账户、不是子账户**：

```
GET /sapi/v1/sub-account/transfer/subUserHistory   HTTP 400
{"code":-12022,"msg":"This endpoint is allowed to request from sub account only"}
```

⟹ **【实测】该账户是主账户，但其子账户功能未开通；API 侧既读不到子账户列表，也创建不了子账户。**

⚠️ **这与 #932 的调研结论构成一处冲突，必须登记**：#932 记「**VIP0 可以直接开子账户，上限 5 个**，不需要先升 VIP」。本次实测该账户确为 VIP0（见下），却**连 API 创建都被拒**。

三种可能成因，**本次无法区分**（照 090 不裁）：
1. 子账户功能需要先在**网页端**开通/申请（API 只是不能自助开通），⟹ 编排者手工做一次即可解锁；
2. 子账户功能对该账户的 KYC 等级/账户类型有额外门槛；
3. `virtualSubAccount` 端点本身对零售账户不开放（`100001002 Unsupported operation` 的措辞更像**端点级不支持**而非**权限级拒绝**，但这是**【推断】**）。

⟹ **第 8 问后半、第 9 问、第 10 问全部卡在这一堵墙上。**

#### 8.5 第 8 问结论

| 分支 | 结论 | 证据等级 |
|---|---|---|
| 主账户能否交易 133 个股票永续 | **能** | 【实测】真实成交回执 |
| 主账户是否还需签 TradFi 协议 | **不需要**（当下状态即可交易） | 【实测】零 `-4411` |
| 「签过了」还是「不需要签」 | **无法区分** | 缺干净对照账户 |
| 子账户能否独立签 | **未决——测不了** | 子账户功能未开通 |
| ⟹ 子账户能否交易 133 个股票永续 | **未决——测不了** | 同上 |

⚠️ **对 #936「Binance 是否可作多层执行场所」的分量**：**塌方点仍未拆除。** 本次把它从「未测」推进到「**卡在一个已定位、且很可能是编排者手工一步就能解开的前置**（网页端开通子账户功能）」——**这是定位上的进展，不是答案上的进展。**

---

### 第 9 问：子账户是否需各自单独完成 TradFi KYC/风险确认

**【实测】测不了。** 该问的前提是「存在至少一个子账户」，而该账户**零子账户且建不出来**（证据见 §8.4）。

⚠️ **不作任何推断。** 官方文档侧 #932 已查过，结论是「查不到明文允许，也查不到明文禁止」，本次**没有新增文档证据**。

---

### 第 10 问：子账户间 `subToSub` 划转是否真收费

**【实测】测不了，双重阻塞：**

1. **无子账户**（证据见 §8.4）——`subToSub` 需要两个子账户，现有零个；
2. **本 key 无划转权限**——`enableInternalTransfer: false` 且 `permitsUniversalTransfer: false`（证据见 §0）。即便有子账户，主账户侧发起的 `universalTransfer` 也会被 key 权限挡住。

⟹ **「前后余额差与返回体」这项交付本次拿不到。**

⚠️ 照 090：**不写「应该免费」，也不写「收费」。** #932 查到的「限速主账户每分钟 2000 次、费用条款未列明」维持原状，**未获任何实测佐证或否证**。

---

## 顺带的只读结论（成本低，全部拿到）

### 10.1 主账户能不能交易那 133 个 `EQUITY` / `TRADIFI_PERPETUAL`

**【实测】能，已实盘验证（`AAPLUSDT`）。**

合约清单快照（2026-08-08，`GET /fapi/v1/exchangeInfo`，无需签名）：

```
总合约数 = 854
{"COIN":698, "INDEX":3, "COMMODITY":8, "EQUITY":133, "PREMARKET":2, "KR_EQUITY":3, "HK_EQUITY":7}
EQUITY 数 = 133，status 分布 = {"TRADING": 133}
```

⟹ **与 #932 于 2026-08-07 的读数逐位一致**（854 总数、133 个 `EQUITY`、全部 `TRADING`）。

**穷举断言的边界**：检索式 = 单次 `GET https://fapi.binance.com/fapi/v1/exchangeInfo`，对返回的 `symbols` 数组按 `underlyingType` 字段做完整分类，2026-08-08 单次快照。**未覆盖**：币本位合约（`dapi`）、现货、期权、testnet。

杠杆与保证金（`GET /fapi/v1/leverageBracket {symbol:AAPLUSDT}`，HTTP 200）：

| bracket | initialLeverage | notionalFloor | notionalCap | maintMarginRatio | cum |
|---|---|---|---|---|---|
| 1 | 20 | 0 | 100,000 | 0.025 | 0.0 |
| 2 | 15 | 100,000 | 500,000 | 0.0333 | 830.0 |
| 3 | 10 | 500,000 | 2,000,000 | 0.05 | 9,180.0 |
| 4 | 5 | 2,000,000 | 5,000,000 | 0.1 | 109,180.0 |
| 5 | 4 | 5,000,000 | 10,000,000 | 0.125 | 234,180.0 |
| 6 | 3 | 10,000,000 | 25,000,000 | 0.1667 | 651,180.0 |
| 7 | 2 | 25,000,000 | 50,000,000 | 0.25 | 2,733,680.0 |
| 8 | 1 | 50,000,000 | 100,000,000 | 0.5 | 15,233,680.0 |

⟹ 股票永续**最高 20x**，第一档维持保证金率 **2.5%**，名义上限 10 万 USDT 内不降杠杆。

该标的当前配置（`GET /fapi/v1/symbolConfig {symbol:AAPLUSDT}`，HTTP 200）：

```
[{"symbol":"AAPLUSDT","marginType":"ISOLATED","isAutoAddMargin":false,"leverage":20,"maxNotionalValue":"100000"}]
```

⟹ **【实测】该标的默认已是 `ISOLATED`（逐仓）**，本次未做任何 `marginType` 修改。

合约微结构（三个抽样标的 `AAPLUSDT` / `SPYUSDT` / `TQQQUSDT` 完全一致）：
`tickSize=0.01`、`minQty=0.01`、`stepSize=0.01`、`MIN_NOTIONAL=5`、`MAX_NUM_ORDERS=200`、`PERCENT_PRICE` 上下 ±3%、`MARKET_LOT_SIZE.maxQty` 分别为 2000/2000/5000。

⟹ **最小可下名义 = 5 USDT**，这决定了后续任何实测的最小成本量级。

### 10.2 `fee/tradFiFee` 的 VIP0 实时 taker 是否仍是 0.04%

**【实测】是，逐字确认，且五个标的一致。** `GET /fapi/v1/commissionRate`（签名，账户实时费率，非文档转述）：

```
AAPLUSDT  HTTP 200  {"symbol":"AAPLUSDT","makerCommissionRate":"0","takerCommissionRate":"0.000400","rpiCommissionRate":"0"}
NVDAUSDT  HTTP 200  {"symbol":"NVDAUSDT","makerCommissionRate":"0","takerCommissionRate":"0.000400","rpiCommissionRate":"0"}
TSLAUSDT  HTTP 200  {"symbol":"TSLAUSDT","makerCommissionRate":"0","takerCommissionRate":"0.000400","rpiCommissionRate":"0"}
SPYUSDT   HTTP 200  {"symbol":"SPYUSDT","makerCommissionRate":"0","takerCommissionRate":"0.000400","rpiCommissionRate":"0"}
TQQQUSDT  HTTP 200  {"symbol":"TQQQUSDT","makerCommissionRate":"0","takerCommissionRate":"0.000400","rpiCommissionRate":"0"}
```

**加密腿对照组**（证明这确实是股票永续独立费率表，不是全站费率）：

```
BTCUSDT   HTTP 200  {"symbol":"BTCUSDT","makerCommissionRate":"0.000200","takerCommissionRate":"0.000500","rpiCommissionRate":"0"}
```

⟹ **#932 的「maker 全档 0%、taker 0.04%（VIP0）」在 VIP0 上被实测确认**，且 **maker 0% 相对加密腿的 0.02% 确是实打实的优惠**。

⚠️ **边界**：只测了 VIP0（本账户等级），**VIP1–9 各档未验**；抽样 5/133 个标的，**未逐一遍历**；`rpiCommissionRate` 字段含义未查。

实盘手续费与账面回读（与 0.04% 自洽）：买入 `quoteQty=6.2640000` → `commission=0.00250560`，比值 = **0.0004000**，逐位吻合。

### 10.3 VIP 等级与实际子账户配额

**VIP 等级【实测】= VIP0**，两个独立端点交叉确认：

```
GET /sapi/v1/account/info    HTTP 200
{"vipLevel":0,"isMarginEnabled":true,"isFutureEnabled":true,"isOptionsEnabled":true,"isPortfolioMarginRetailEnabled":false}

GET /fapi/v1/accountConfig   HTTP 200
{"feeTier":0,"canTrade":true,"canDeposit":true,"canWithdraw":true,"dualSidePosition":false,
 "updateTime":1785994733025,"multiAssetsMargin":false,"tradeGroupId":-1}
```

顺带读数：`dualSidePosition: false` ⟹ **单向持仓模式**（非双向对冲），`multiAssetsMargin: false` ⟹ **单资产保证金模式**。

**子账户配额【实测】测不出来** —— 不是「配额为 0」，是**功能未开通、端点整体不可达**（见 §8.4）。

⚠️ **⟹ #932 的「VIP0 上限 5 个」本次既未被证实，也未被否证。** 实测到的是**更前面一道门**：这个 VIP0 账户当下**一个子账户都开不了**。「上限 5 个」这个数要成立，前提是子账户功能先开通——**而那个前提在本账户上不成立**。

---

## 做过的写操作清单

| # | 操作 | 端点 | 结果 | 可逆性 | 账户状态影响 |
|---|---|---|---|---|---|
| 1 | 买入 0.02 AAPLUSDT（市价） | `POST /fapi/v1/order` | HTTP 200，orderId `140176001`，成交价 313.20，名义 6.2640 USDT | **可逆，已逆**（见 #2） | 开逐仓多头，占用保证金 0.31068134 USDT |
| 2 | 平掉该仓（市价 reduceOnly） | `POST /fapi/v1/order` | HTTP 200，orderId `140176029`，成交价 313.19 | — | **仓位归零，账户回到平仓态** |
| 3 | 尝试创建虚拟子账户 | `POST /sapi/v1/sub-account/virtualSubAccount` | HTTP 400 `{"code":100001002,"msg":"Unsupported operation"}` | **无需逆转——请求被拒，零状态变更** | 无 |

### 账户状态前后对比（逐字）

```
BEFORE  GET /fapi/v2/balance  HTTP 200
{"accountAlias":"[已打码]","asset":"USDT","balance":"24.30489058","crossWalletBalance":"24.30489058",
 "crossUnPnl":"0.00000000","availableBalance":"24.30489058","maxWithdrawAmount":"24.30489058",
 "marginAvailable":true,"updateTime":1785718214133}

BEFORE  GET /fapi/v2/positionRisk {symbol:AAPLUSDT}  HTTP 200
[{"symbol":"AAPLUSDT","positionAmt":"0.00","entryPrice":"0.0","breakEvenPrice":"0.0","markPrice":"313.19770460",
  "unRealizedProfit":"0.00000000","liquidationPrice":"0","leverage":"20","maxNotionalValue":"100000",
  "marginType":"isolated","isolatedMargin":"0.00000000","isAutoAddMargin":"false","positionSide":"BOTH",
  "notional":"0","isolatedWallet":"0","updateTime":1784092082695,"isolated":true,"adlQuantile":0}]

持仓中  GET /fapi/v2/positionRisk {symbol:AAPLUSDT}  HTTP 200
[{"symbol":"AAPLUSDT","positionAmt":"0.02","entryPrice":"313.20000000000005","breakEvenPrice":"313.32528",
  "markPrice":"313.19672593","unRealizedProfit":"-0.00006548","liquidationPrice":"305.29503487","leverage":"20",
  "maxNotionalValue":"100000","marginType":"isolated","isolatedMargin":"0.31068134","isAutoAddMargin":"false",
  "positionSide":"BOTH","notional":"6.26393451","isolatedWallet":"0.31074682","updateTime":1786165144675,
  "isolated":true,"adlQuantile":0}]

AFTER   GET /fapi/v2/positionRisk {symbol:AAPLUSDT}  HTTP 200
[{"symbol":"AAPLUSDT","positionAmt":"0.00","entryPrice":"0.0","breakEvenPrice":"0.0","markPrice":"313.19000000",
  "unRealizedProfit":"0.00000000","liquidationPrice":"0","leverage":"20","maxNotionalValue":"100000",
  "marginType":"isolated","isolatedMargin":"0.00000000","isAutoAddMargin":"false","positionSide":"BOTH",
  "notional":"0","isolatedWallet":"0","updateTime":1786165147657,"isolated":true,"adlQuantile":0}]

AFTER   GET /fapi/v2/balance  HTTP 200
{"accountAlias":"[已打码]","asset":"USDT","balance":"24.29967947","crossWalletBalance":"24.29967947",
 "crossUnPnl":"0.00000000","availableBalance":"24.29967947","maxWithdrawAmount":"24.29967947",
 "marginAvailable":true,"updateTime":1786165147657}
```

**净成本 = 24.30489058 − 24.29967947 = 0.00521111 USDT**（≈ 0.005 美元），拆开：
- 手续费 0.00250560 + 0.00250551 = **0.00501111**（两笔 taker × 0.04%）
- 已实现盈亏 **−0.00020000**（一个 tick 的买卖价差）

**持仓归零已回读确认**（`positionAmt: "0.00"`、`isolatedMargin: "0"`、`liquidationPrice: "0"`）⟹ **写操作 #1/#2 已完全逆转，账户回到平仓态，唯一不可逆残留是 0.005 USDT 的手续费。**

### ★ 授权了但**没有**执行的写操作（明账登记）

| 操作 | 为什么没做 |
|---|---|
| `POST /fapi/v1/stock/contract`（签 TradFi-Perps 协议） | **实测证明不必要**——主账户已能成交股票永续（§8.2），零 `-4411`。该操作**可逆性未知**，在既已证明无必要的前提下执行它是**纯粹的不可逆风险**，故不做。 |
| 修改 API key 任何设置 | 纪律禁止，未做。 |
| 任何提现 | 纪律禁止，未做；且 key 本身无此权限。 |

---

## 未能完成的部分与原因（照 090 逐条）

| 未完成项 | 原因（逐字证据） | 解阻条件 |
|---|---|---|
| **第 8 问后半：子账户能否独立签 TradFi 协议** | 该账户子账户功能未开通：`{"code":-8012,"msg":"Sub-account function is not enabled."}`；创建被拒：`{"code":100001002,"msg":"Unsupported operation"}` | 编排者在**网页端**开通子账户功能（若可开通），或确认该账户类型不支持 |
| **第 9 问：子账户 TradFi KYC 是否继承** | 同上，样本量 = 0 | 同上 |
| **第 10 问：`subToSub` 是否收费** | 同上；**另加** key 权限 `enableInternalTransfer: false`、`permitsUniversalTransfer: false` | 开通子账户功能 **＋** 给 key 加划转权限（**后者需编排者决定，本次未改任何 key 设置**） |
| **区分「协议早已签过」vs「本账户无需签」** | 缺一个未签署过的干净账户作对照 | 需第二个账户，本次不具备 |
| **主网上 `POST /fapi/v1/stock/contract` 路径是否存在** | 主网 GET 返回的 404 与「明确不存在路径」的负控**逐字相同**，无法区分（testnet 上能区分，返回 `-5000 Method GET is invalid`） | 需要一个能触发 `-4411` 的账户，或官方文档明确 |
| **testnet 预演签协议流程** | testnet API key 需在 `testnet.binancefuture.com` **交互式网页注册**（GitHub OAuth），无法在本会话内以非交互方式生成 | 编排者手工生成一个 testnet key |
| **VIP1–9 各档费率与子账户配额** | 本账户为 VIP0，其余各档无法从账户端点读出 | 只能靠文档，或更高等级账户 |
| **133 个 `EQUITY` 标的逐一验证可交易** | 只实盘验了 `AAPLUSDT` 1 个，另抽样读了 4 个的费率 | 需逐一下单，成本 ≥ 5 USDT × 133，性价比低 |

### 对 #936（执行场所选定）的移交口径

⚠️ **Binance 侧的塌方点（子账户能否交易股票永续）本次仍未答出。** #936 若要把 Binance 列为「被否方案」，**登记的理由目前只能是「测不出来——子账户功能未开通」，不能写成「测了不行」**。

⚠️ 与 Hyperliquid 侧对称的一条：**Binance 的「子账户强平不传染」同样零实测**，本次也**没有**推进（连子账户都建不出来，更不可能做爆仓对照）。#942 resolution 里对 Hyperliquid 写的那句警告——「不许让后来人以为隔离是实测过的」——**对 Binance 侧同样成立，且成立得更彻底**。

### 本次相对 #932 的净增量（三条）

1. **主账户交易股票永续这条链是通的**（从文档推断 → **实盘成交回执**）；
2. **费率 0.04% taker / 0% maker 在 VIP0 上被账户端点实测确认**，且有加密腿 0.05%/0.02% 作对照，证明确是独立费率表；
3. **塌方点的阻塞被重新定位**：不再是「不知道子账户能不能签」，而是「**这个账户连子账户都建不了**」——这是一道更前面、且**可能编排者手工一步就能解开**的门。
