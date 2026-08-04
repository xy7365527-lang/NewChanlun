# #878 调研报告：交易所侧「按重逐仓」可行性（Binance / Bybit / OKX）

- 票：GitHub issue #878（仓外事实调研，喂 SPEC #847 S7「毛敞口接进决策路径」）
- 日期：2026-08-04
- 口径约定：每条断言标注【官方文档】／【官方论坛-社区回答】／【社区说法】／【未证实】。社区说法仅作线索；查不到的标【未证实】，不猜。
- 背景定义：「重」= ⟨标的 S, 操作级别 ℓ, 专属筹码 Q⟩（ADR 0010），要求各重保证金逐仓分开计提。

---

## Q1：Binance USDT-M 永续逐仓（ISOLATED）的真实粒度

### 1a. 保证金模式按 symbol 设置 —— 【官方文档】明说

`POST /fapi/v1/marginType`（Change Margin Type）的 API Description 原文：**"Change symbol level margin type"**，参数为 `symbol` + `marginType`（ISOLATED / CROSSED）。

- 来源：https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/Change-Margin-Type

### 1b. 逐仓保证金的计提/调整粒度 = symbol × 持仓方向 —— 【官方文档】明说

`POST /fapi/v1/positionMargin`（Modify Isolated Position Margin）参数：`symbol` + `positionSide`（原文："Default `BOTH` for One-way Mode; `LONG` or `SHORT` for Hedge Mode. **It must be sent with Hedge Mode.**"）+ `amount` + `type`（1 加 / 2 减），并注明 "Only for isolated symbol"。即：Hedge 模式下多空单的逐仓保证金**分开调整、分桶计提**。

- 来源：https://developers.binance.com/legacy-docs/derivatives/usds-margined-futures/trade/rest-api/Modify-Isolated-Position-Margin

仓位查询 `GET /fapi/v2/positionRisk`（Position Information V2）按 **(symbol, positionSide) 二元组**返回行，每行各带 `marginType` / `isolatedMargin` / `isolatedWallet` / `leverage` / `liquidationPrice`——官方响应示例中 BTCUSDT 的 LONG 与 SHORT 是两行、各自独立的逐仓状态与强平价。

- 来源：https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/Position-Information-V2

### 1c. 持仓方向模式是账户级开关 —— 【官方文档】明说

`POST /fapi/v1/positionSide/dual`（Change Position Mode）："Change user's position mode (Hedge Mode or One-way Mode) on **EVERY symbol**"，且账户有持仓或挂单时拒绝切换（-4067 / -4068）。即无法对单个 symbol 单独开 hedge 模式。

- 来源：https://developers.binance.com/docs/derivatives/usds-margined-futures/trade/rest-api/Change-Position-Mode

### 1d. 同一 symbol 同一方向能否持有多个各自独立逐仓的「子仓位」—— 不能

证据链：

- 【官方文档】Position Information V2 的数据模型就是 (symbol, positionSide) 一行一仓：同一 symbol 同一方向只有一行 `positionAmt`，API 中不存在任何「子仓位 ID」概念（见 1b/1c 来源）。
- 【官方论坛-社区回答】Binance 官方开发者社区（dev.binance.vision）2021-06 问答：用户在 Hedge 模式下持有同方向多笔"仓位"，问如何平掉其中特定一笔；回答：**"orders and positions are different in futures"**——同方向多笔订单聚合成一个仓位，平仓只能下反方向订单并带 `positionSide`。无逐笔分仓。
  - 来源：https://dev.binance.vision/t/binance-futures-hedge-mode-close-one-specific-position/4359
- 【社区说法，仅作旁证】nautilus_trader 的 Binance 集成文档亦按 symbol × position side 建模仓位（`ETHUSDT-PERP.BINANCE-LONG`）。
  - 来源：https://nautilustrader.io/docs/latest/integrations/binance/

**Q1 小结**：Binance USDT-M 逐仓真实粒度 = **symbol × 持仓方向**（one-way 下退化为 symbol）。同一 symbol 同一方向**不可能**持有多个各自独立逐仓的子仓位——订单净额聚合成一仓。「自定义分区逐仓」在单账户内不存在。

---

## Q2：Binance 子账户——能否各自独立持同一 symbol 并各自逐仓 + 机械与限制

### 2a. 独立性 —— 可以（子账户 = 独立账户）

【官方文档】Binance 子账户 FAQ（360020632811）：

- 子账户用于 "trade with multiple strategies, or those who need to **separate their assets for risk management**"；
- 主子/子子之间划转 "instantly without any fees"，但**只有主账户能管理资产划转**；"Asset transfer between sub-accounts and third-party accounts are not supported"；子账户**不能提现**（"only the master account can manage the asset transfer"）；
- 每个子账户独立开通 Futures 账户、独立持仓（各子账户是独立账户，同一 symbol 可在不同子账户各自开仓、各自逐仓——这是"独立账户"的题中之义；FAQ 未逐字写"同一 symbol"，但仓位/保证金天然按账户隔离）。

来源：https://www.binance.com/en/support/faq/360020632811

### 2b. 子账户个数上限 —— 【官方文档】明说（按主账户 VIP 分档）

同 FAQ 第 6 条 "How many sub-accounts can I open?" 表格（Futures 行）：**VIP1=20，VIP2=30，…，VIP8=100，VIP9=200**（Spot/Leverage Token 同档；Margin 恒 10）。另注：企业账户默认开通子账户功能；个人需 VIP1+（此条为【社区说法】：coinsutra/investguiding 引 Binance 规则；FAQ 表格本身从 VIP1 起排，与之一致）。

- 官方：https://www.binance.com/en/support/faq/360020632811
- 社区：https://coinsutra.com/binance-sub-accounts/

### 2c. API 权限 —— 【官方文档】明说

- 每个子账户最多 **30 个 API Key**；子账户有**各自独立**的 API 下单频限（Futures：300 单/10s、1200 单/min）；
- 虚拟邮箱创建的子账户**只能经 API 由主账户操作**（无登录）；
- API 端点族（legacy 文档 Asset Management 目录 + Transfer to Master 页）：`POST /sapi/v1/sub-account/transfer/subToMaster`（子→主，子账户 API Key 需开 Spot & Margin Trading 权限）、Transfer to Sub account of Same Master（子→同主子）、Universal Transfer（主发起）、Sub account Futures Asset Transfer（主发起，合约账户内划转）、Futures Transfer for Sub account（主发起）、以及 **Move Position for Sub account**（主子间移仓）。
  - 来源：https://developers.binance.com/legacy-docs/sub_account/asset-management/transfer-to-master （含完整目录）
- 另（Brokerage API 文档，【官方文档】但属经纪商线）："Sub account should be enable futures before its api-key's futuresTrade being enabled"。
  - 来源：https://binance-docs.github.io/Brokerage-API/Brokerage_Operation_Endpoints/

### 2d. 费率 —— 【官方文档】明说

同 FAQ 第 1 条："Your VIP tier and correspondent discounts will be defined by the **aggregate trading volume of all sub-accounts and master accounts**"——VIP 档位按主子合计成交量定，子账户共享主账户费率折扣，无额外费率差。

- 来源：https://www.binance.com/en/support/faq/360020632811

**Q2 小结**：子账户路径机制完整——独立持仓/独立逐仓、划转即时免费但只能主账户发起/归口、每子账户 30 个 API Key + 独立频限、费率随主账户合计量、个数按 VIP 分档（20@VIP1 → 200@VIP9）。限制：子账户不可直接充提；个数有硬顶；虚拟邮箱子账户纯 API。

---

## Q3：Bybit / OKX 有无更接近「按自定义分区逐仓」的机制

### 3a. Bybit

- 【官方文档】Classic 账户：逐仓/全仓**按 symbol** 切换（`POST /v5/position/switch-isolated`，"per symbol level"）——但注意 UTA2.0 **不支持**该接口，UTA1.0 仅 inverse。
  - https://bybit-exchange.github.io/docs/v5/position/cross-isolate
- 【官方文档】UTA 账户：保证金模式是**账户级**开关（`POST /v5/account/set-margin-mode`：`ISOLATED_MARGIN` / `REGULAR_MARGIN` / `PORTFOLIO_MARGIN`）。
  - https://bybit-exchange.github.io/docs/v5/account/set-margin-mode
- 【官方帮助中心】UTA FAQ 明说："Can I have different margin modes for different trading pairs? **No, the margin mode selected will be applied to the account level and all trading pairs.**" 且 UTA 逐仓下仅支持 Spot、USDT 永续、USDC 永续&交割；逐仓强平按**仓位**的强平价触发（"when the Mark Price reaches the Liquidation Price of your position"）；双向持仓（Hedge）在 UTA2.0 仅 USDT 永续支持，按 symbol 配置（`POST /v5/position/switch-mode`）；逐仓仓位的手工加减保证金按 `positionIdx`（0 单向 / 1 买侧 / 2 卖侧）——即粒度同样是 symbol×方向，**无自定义分区**。
  - https://www.bybit.com/en/help-center/article/FAQ-Unified-Trading-Account （同文镜像：https://www.bybit.kz/en-KAZ/help-center/article/FAQ-Unified-Trading-Account ）
  - https://bybit-exchange.github.io/docs/v5/position/position-mode
  - https://bybit-exchange.github.io/docs/v5/position/manual-add-margin
- 【官方帮助中心】子账户：每主账户最多 **20 个 Standard Subaccount**（不含 Custodial）；主子互转免费；主↔子双向、子→主单向、子↔子经主账户；费率折扣继承主账户；子账户 P&L 与主账户隔离；支持 API 交易；子账户不可充提。
  - https://www.bybit.com/en/help-center/article/FAQ-Standard-Subaccount
- 【官方文档】API 建子账户：`POST /v5/user/create-sub-member`（主账户 Key + 特定权限）。
  - https://bybit-exchange.github.io/docs/v5/user/create-subuid
- Portfolio Margin 存在（`PORTFOLIO_MARGIN`），但方向相反（组合统算风险、对冲抵扣），不是分区工具。

**Bybit 小结**：无「按自定义分区逐仓」。UTA 逐仓粒度 = 仓位（symbol×方向），但模式是账户级一刀切；子账户（≤20）是唯一的账户级隔离容器。

### 3b. OKX

- 【官方文档】持仓模式：`POST /api/v5/account/set-position-mode`（`long_short_mode` / `net_mode`；FUTURES/SWAP 支持双向；**Portfolio margin 账户只支持 net**）。
- 【官方文档】每个仓位（`posId` = 品种×方向）各带 `mgnMode`（cross / isolated）；`POST /api/v5/account/position/margin-balance` 按 `instId`+`posSide` 加减**逐仓仓位**保证金——粒度同样 = symbol×方向，无自定义分区。
- 【官方文档】账户模式四档：Spot / Futures / Multi-currency margin / Portfolio margin——组合保证金同样不是分区工具。
- 【官方文档】子账户：`POST /api/v5/users/subaccount/create-subaccount`（仅主账户、且主账户 API Key 须绑 IP；另有错误码 59518 "You can't create a sub-account using the API; please use the app or web"，即部分账户类型只能前端建）；主子间划转由主账户发起（`POST /api/v5/asset/subaccount/transfer`）+ 主可设子账户转出许可（`set-transfer-out`）；子账户错误码 59504 "Sub-accounts don't support withdrawals"；**个数上限存在**（错误码 59603 "Maximum number of subaccounts reached"）但**官方文档未给出数字** → 个数【未证实-官方】；【社区说法】普通用户（Lv1–Lv5）最多 5 个（Cryptohopper 帮助文 2026-07 引 OKX 规则，未获 OKX 官方文本确认）。
- 【官方文档】注意两个「主子共享」：① 仓位限额 `posLmtAmt` / `longPosRemainingQuota` 等 **shared across master and sub-accounts**（用户级）；② 子账户下单频限 1000 单/2s/子账户，VIP5+ 可按成交率升档。
  - 以上全部来源：https://www.okx.com/docs-v5/en/ （单页文档：Trading Account → Set position mode / Get positions / Increase/decrease margin；Sub-account → Create sub-account 等；Rate Limits → Sub-account rate limit；Error Code）

**OKX 小结**：与 Binance 同构——逐仓到 symbol×方向为止，无自定义分区；子账户路径存在但官方文档不公开个数上限（社区口径 5 个，未证实），且**仓位限额主子共享**（对「按重隔离敞口」是个削弱点）。

### 3c. 参照系（非题目所问三家，仅登记线索）

- 【官方公告】**BingX "Separate Isolated Margin Mode"**（2025-01）：USDT-M 永续/标准合约下，**同一交易对可开多个仓位、每个仓位独立保证金/杠杆/止盈止损**——这是目前所见最接近「按自定义分区逐仓」的交易所原生机制。限制：仅单资产模式 + Hedge 模式；与 Multi-Assets/One-way/TWAP/体验金不兼容。
  - https://bingx.com/en/support/articles/11676733405711
- Binance 亦有 Portfolio Margin / Portfolio Margin Pro 产品线（官方文档导航），方向与逐仓相反，略。

---

## 结论形态（Q4）

**单账户内「按重逐仓」在交易所侧 = 不可行，三家同判。** Binance / Bybit / OKX 的逐仓硬粒度都是 **symbol × 持仓方向**（one-way/net 下 = symbol）；同一 symbol 同一方向的订单全部净额聚合成一个仓位，API 不存在「子仓位/分区」概念；Bybit UTA 的逐仓更是账户级一刀切。Portfolio Margin 方向相反，不是分区工具。

**可行路径 = 子账户即「重」的交易所侧容器**：重 ↔ 子账户 1:1（凡共享同一 symbol 的重必须分户；不共享 symbol 的重可 N:1 合户，此时户内仍是 symbol×方向逐仓 + 仓内账本细分）。机械要件已证实：

- Binance：子账户独立持仓独立逐仓；划转即时免费、主账户归口发起；每子 30 API Key + 独立频限；费率按主子合计量；上限 VIP1=20 → VIP9=200（Futures）。子账户不可充提。
- Bybit：≤20 个 Standard 子账户，机制同构。
- OKX：机制同构但官方不公开个数（社区口径 5，未证实），且**仓位限额主子共享**。

**残留「未证实」清单**：① OKX 子账户个数的官方数字；② Binance/Bybit 的子账户是否共享主账户的仓位限额（Binance FAQ 未提；OKX 明写共享）；③ Binance 子账户开通 Futures 的 API 化程度（Brokerage 线文档有 futuresTrade 使能，普通主子线以 UI 开通为准）。

**对 SPEC #847 S7 的一句话**：交易所侧能给的最细隔离是 symbol×方向；「按重逐仓」须走子账户映射 + 仓内记账兜底——先定重↔子账户映射表和个数预算（Binance 20@VIP1 起），再谈实施。
