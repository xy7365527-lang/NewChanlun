# wayfinder #62 — venue 真实费率数据源可得性与标定来源（Binance BTCUSDT 永续）

- 工位：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），只读调研 + 公开 API 实测。
- 实测时间：2026-07-21 03:05–03:15 UTC（curl，未认证）。
- 结论先行：**funding 全史免费可得（API 自 2019-09-13 起 = 上市全史；vision 归档 2020-01→2026-06）；maker/taker 费率与借贷利率仅认证端点可得且无公开历史；USDⓈ-M 永续无借贷成本概念（borrow 项对本 venue 照实否定，非保正常数）。**

## 1. 数据源清单（端点 / 格式 / 历史深度 / 认证要求）

### 1.1 funding rate 历史 — `GET /fapi/v1/fundingRate`（免费，无认证）

- 实测最早记录：`{"symbol":"BTCUSDT","fundingTime":1568361600000,"fundingRate":"0.00010000","markPrice":""}` → **2019-09-13 08:00:00 UTC**。
- 对照 `GET /fapi/v1/exchangeInfo`：BTCUSDT `onboardDate=1567965300000`（2019-09-08 17:55 UTC），`contractType=PERPETUAL` → **funding API 覆盖 = 上市全史**。
- 实测最近记录：`fundingTime=1784592000001`（2026-07-21 00:00 UTC），8h 间隔连续。
- 返回格式：JSON array `[{symbol, fundingTime(ms), fundingRate(string), markPrice}]`；早期记录 `markPrice` 为空串，近期记录带 markPrice。
- 分页：`startTime`/`endTime` 毫秒闭区间 + `limit≤1000`（实测 limit=1000 返回正常，52.7KB）；无 startTime 时默认返回最近记录（实测）。无窗口截断——2019-09 窗可查（实测）。
- 全史约 7.4k 条（3 条/日），limit=1000 分页约 8 次调用即取全。

### 1.2 funding 归档 — `data.binance.vision/data/futures/um/monthly/fundingRate/BTCUSDT/`（免费，无认证）

- 实测目录列表（S3 ListBucket）：**78 个月度 zip，2020-01 → 2026-06**（含 .CHECKSUM 旁挂）。
- 无 daily fundingRate 归档（实测 `BTCUSDT-fundingRate-2026-07-15.zip` → 404；2019-09 zip → 404）。**2019-09-13→2019-12-31 的约 330 条只能走 §1.1 API 补**。
- CSV 格式（表头 + 数据）：`calc_time,funding_interval_hours,last_funding_rate`，例：`1577836800000,8,-0.00012359`。
- 全量校验：78 zip 实测合计 **7119 条数据行**（7197 行 − 78 表头）；首条 2020-01-01 00:00 UTC，末条 2026-06-30 16:00 UTC。
- 一致性抽查：2020-01 归档 93 条 = 31 日 × 3/日，与 API 同月条数口径吻合。

### 1.3 funding 参数 — `GET /fapi/v1/fundingInfo`（免费，无认证）

- 实测 BTCUSDT：`fundingIntervalHours=8`，`adjustedFundingRateCap=0.003`，`adjustedFundingRateFloor=-0.003`。
- 归档实测 min/max = ±0.00300000，恰为 cap/floor → 自洽。

### 1.4 maker/taker 费率 — `GET /fapi/v1/commissionRate`（**需认证**）

- 实测无认证调用返回 `{"code":-2014,"msg":"API-key format invalid."}` → **USER_DATA 端点，API key + HMAC signature**（签名参数见官方 Postman 集 `binance/binance-api-postman` → `Binance Derivatives Trading USDS Futures API.json` → `/Account/User Commission Rate (USER_DATA)`，含 `timestamp`+`signature` 必填）。
- 返回 `makerCommissionRate`/`takerCommissionRate`——**仅当前账户当前档费率，无历史**。VIP 费率表为静态发布页，无公开历史 API。→ 标定只能取"当前账户费率"一次性快照 + 费率表静态常量，历史费率变化不可回溯。

### 1.5 借贷利率 — `GET /sapi/v1/margin/interestRateHistory`（需认证，且对本 venue 照实否定）

- 实测无认证调用返回 `-2014`（同 §1.4，需签名）。
- 官方 OpenAPI（`binance/binance-api-swagger` `spot_api.yaml:5357-5363`）：`Margin Interest Rate History (USER_DATA)`，**startTime–endTime 最大间隔 30 天**；返回 `{asset, dailyInterestRate, timestamp, vipLevel}`。
- **口径裁决（照实否定）**：USDⓈ-M 永续持仓的杠杆持有成本就是 funding（多空互付），**不存在独立 borrow 利息**——m6 `borrow_rate_per_bar`（risk.rs:832）对 Binance USDⓈ-M 永续应声明 **N/A**，而非换个保正常数。borrow 端点仅在 venue 切到 spot margin / 杠杆借贷场景才适用（此时用 §1.5 认证端点，30 天/窗分页）。
- 辅助：当前跨币种借贷费率快照 `GET /sapi/v1/margin/crossMarginData`（USER_DATA，同 spec :5423+，`dailyInterest`/`yearlyInterest`），同样无历史。

### 1.6 强平罚金（liq_penalty_rate）— 无公开程序化源

- Binance USDⓈ-M 强平清算费（liquidation clearance fee，进保险基金）只在官方静态费率/文档页发布，**无公开 API、无历史序列**。→ 只能作 venue 文档常量注入（现保底 0.5%，wverify_run.rs:1069-1071 注明"清算费+滑点近似"），标定路径 = 人工核费率表后改常量 + 版本哈希落档。

## 2. 既有抓取能力（代码锚）

- `analysis/fetch_btc_binance_full.py:36`：`BASE = https://data.binance.vision/data/spot`——**仅现货 1m klines**（月归档 :41-54 + 日归档 :57-69），落盘列式 JSON（:164-167）。其 zip→CSV 管线（:92-108 `_parse_zip`）与并发下载（:130-141）可直接复用模式抓 futures fundingRate 归档，但**现脚本不支持**。
- `scripts/download_btc_binance.py:38-40`：同样仅 spot monthly klines，功能被前者包含。
- **结论：funding/fee/borrow 三个端点的抓取是两脚本均未覆盖的新工**；机制模式（vision zip→CSV、列式 JSON 落盘）已有先例可照搬。

## 3. 标定替换保底值的最小数据面

现状保底（wverify_run.rs:1076）：`CostModel::new(0.0001, 480, 0.000001, 0.005)` = funding 1bp/8h、borrow 0.01bp/bar、liq 0.5%，全部 `RATE_UNCALIBRATED_LABEL` 强制标注（risk.rs:754）。

| 保底项 | 最小 datum | 来源 | 消费位 |
|---|---|---|---|
| funding（无向 1bp/期） | **全史 signed rate 序列** `(calc_time, rate)` ~7.4k 条 | §1.1 API（2019-09 起全史）+ §1.2 归档（2020-01 起）合并 | `FundingScheduleBook`（risk.rs:711-748 已实装）→ `funding_accrual_signed`（risk.rs:890-896，A10 C2 预冻结接口）；无向保底路径逐字不动（F2 锁死，risk.rs:887-889） |
| funding_period_bars=480 | `fundingIntervalHours=8` | §1.3 fundingInfo 实测 | 现有值 480 已正确，datum 化来源标注即可 |
| comm（fee_rate） | 账户 maker/taker 一次性快照（如 VIP0 taker 费率表常量） | §1.4 认证端点一次 + 费率表人工核 | `ExecConfig.fee_rate`（既有，非本票范围） |
| borrow 0.01bp/bar | **N/A 声明**（本 venue 无借贷成本） | §1.5 口径裁决 | 保留参数但 datum 标注"venue 不适用"；不拿保正常数冒充标定 |
| liq 0.5% | venue 文档常量（费率表人工核） | §1.6 | `liq_penalty_rate` 常量 + 版本哈希 |

**funding 序列描述性概览（描述 datum，非决策依据——v3 禁统计推断作决策基础）**：7119 条归档实测 mean=+0.000109/期、median=+0.000096、p05=−0.000053、p95=+0.000481、83.1% 落在 ±1bp 内、85.5% 为正、极值恰为 cap ±0.003。**保底 1bp/期与 median 同量级，但真实序列带符号且逐期时变——标定替换的是整条序列（逐条 datum 进 FundingScheduleBook），不是"换一个更准的常数"。**

## 4. 落档格式建议（L2 费率标定: datum 版本哈希）

建议 `analysis/data_cache/btc_funding_binance_um.json`，沿用项目列式约定（fetch_btc_binance_full.py:164-167）：

```json
{
  "schema_version": 1,
  "symbol": "BTCUSDT",
  "venue": "binance-um-perp",
  "source": {
    "endpoints": ["/fapi/v1/fundingRate", "data.binance.vision/.../fundingRate/BTCUSDT/*.zip"],
    "retrieved_at_utc": "YYYY-MM-DDTHH:MM:SSZ",
    "coverage": {"first_calc_time_ms": 1568361600000, "last_calc_time_ms": ..., "n_records": ...}
  },
  "funding_interval_hours": 8,
  "times_ms": [1568361600000, ...],
  "rates": [0.0001, ...],
  "datum_sha256": "sha256(canonical json of times_ms+rates)"
}
```

- **版本哈希契约**：`datum_sha256` 进报告标签 `[L2费率标定: datum <hash 前 12 位>]`——对齐 risk.rs:709 升口径契约（datum 注入后升级、不得跳级）；哈希使"标定来源"可审计、可复现。
- funding 序列 → FundingScheduleBook 段构造：相邻等 rate 合并或逐期一段均可，`as_of` 零前视语义（risk.rs:740-747）已保证不借用未来快照。
- borrow=N/A 与 liq=文档常量的口径声明随 datum 文件同目录落一份 `*_provenance.md`，记录端点/抓取时刻/认证范围（funding 免费全史 vs commissionRate 需 key 一次快照 vs liq 人工核表）。

## 5. 未决/缺口（照实）

- commissionRate 与 margin interestRateHistory 的**认证后实际返回深度**未实测（本工位无 API key，也不应在此工位配置密钥）；已知官方 spec 约束（30 天/窗）如上。
- liq_penalty_rate 的 Binance 当前官方数值未在本调研中核到程序化源（费率表为静态 HTML/文档页，未抓取）；保留保底 + 人工核表流程。
- 2019-09-13→2019-12-31 funding 仅 API 可补（归档 404 实测）。
