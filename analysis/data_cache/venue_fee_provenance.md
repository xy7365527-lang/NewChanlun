# venue 费率 datum 溯源件（#360 落盘 / #374 LOW-F 补件）

报告 §3.1 item 2 要求「照搬 #62 §4 的列式+哈希约定与 `*_provenance.md` 溯源件」。#360 实装把溯源信息
内嵌在 datum JSON 的 `source_url` / `retrieved_at_utc` / `note` 三字段里（信息未丢，但溯源件未落，
且未列入偏离清单）。本件补齐，**不改动 datum 与 sidecar 任何字节**——内容全部转录自两份 datum，
哈希与 `*.json.sha256` 逐字一致。

## 在册 datum

| 文件 | venue | 取数日期 (UTC) | 来源 URL | sha256 |
|---|---|---|---|---|
| `venue_fee_binance_spot_20260726.json` | `BINANCE_SPOT` | 2026-07-26 | https://www.binance.com/en/fee/trading | `cdd23adef9b34b7a86496769b60c07c14055e5ff88061b7dcf02f5343c4c2c7c` |
| `venue_fee_ibkr_pro_20260726.json` | `IBKR_PRO_US_EQUITY` | 2026-07-26 | https://www.interactivebrokers.com/en/pricing/commissions-stocks.php | `cc0d3fa3695dba4e873325cbaa09615c456de8e41ca6b2e77d77d194c9d8e4a8` |

独立复核（不经本仓代码）：

```bash
cd analysis/data_cache && shasum -a 256 -c venue_fee_binance_spot_20260726.json.sha256 \
                                          venue_fee_ibkr_pro_20260726.json.sha256
```

`venue_fee::load_datum` 在装载时做同一校验，不符即 `Err`（禁静默退化）。

## 列式约定（schema_version = 1）

簿级：`schema_version` / `venue` / `source_url` / `retrieved_at_utc` / `entries`。
条目级键 = `(symbol, tier)`，重复即 `Err`；`unit` 决定其余字段的必填集：

| `unit` | 必填字段 | 语义 |
|---|---|---|
| `notional` | `maker_bps`, `taker_bps` | 按名义额计费（bp/side） |
| `per_share` | `commission_per_share_usd`, `min_commission_usd`, `max_commission_frac_of_notional`, `clearing_per_share_usd`, `cat_per_share_usd`, `sell_sec_fee_frac`, `sell_taf_per_share_usd`, `sell_taf_cap_usd`, `passthru_exchange_frac_of_commission`, `passthru_finra_frac_of_commission` | 按股计费 + 最低佣金托底 + 名义额上限 + 卖出侧监管费 |

全部数值字段须**有限非负**，缺一即 `Err`（`datum_io::need`）。未知 `unit`、空 `entries`、
未知 `schema_version` 同样 `Err`。四条构造期分支的测试见 `venue_fee/datum_io.rs`（#374 LOW-G）。

## 一手数字来源

两份 datum 的逐项数字与出处记在 `chanlun/review-results/venue-fee-source-research-20260726.md`：
Binance 见 §2.1（现货 Regular User 0.1%/0.1%，BNB 抵扣档 0.075%），IBKR 见 §2.3/§2.5
（Tiered ≤30 万股档 $0.0035/股、min $0.35、上限 1% 名义额；SEC Section 31 = 0.0000206×卖出额，
FY2026 自 2026-04-04 生效；FINRA TAF = $0.000195/股、单笔上限 $9.79，自 2026-01-01）。

## 有效域（231号，照实）

- **L2 只覆盖佣金/监管/清算科目**。`slippage_bps` 仍是未标定常数（价差/冲击性质，另票），
  持有成本三项（funding/borrow/liq）不在本簿覆盖面 —— 带这两类成本的报告不因成交费率升级而整体升级。
- **venue 口径 = spot**（#303）：BTC 档对应 `data.binance.vision` 现货 1m K 线；perp 费率表未核，不入簿。
- **未列品种一律不在册**：`resolve` 失败即 fail-loud，禁静默退回未标定档。
- **maker 档存在但生产恒 Taker**：本引擎按 bar close 市价撮合，无挂单语义，取 maker 档是声明膨胀（090）。
- **注入通道未落**：`ExecConfig::default()` 恒 `None`，CLI 无 datum 参数 ⟹ 生产口径上本簿尚未被消费
  （#374 MED-B 登记，实装属另票）。
