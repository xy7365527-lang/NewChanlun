# Hyperliquid paper launcher

入口为 `python -m trading_system.execution.hl_paper_launcher --dry-run`。默认使用 testnet；设置 `HL_ENVIRONMENT=mainnet` 才选择 mainnet。凭证变量分别为 `HYPERLIQUID_TESTNET_PK` 与 `HYPERLIQUID_PK`，生产运行缺凭证会在导入 Nautilus 或联网前失败。

`--dry-run` 只校验选择，不构造 TradingNode、不连接网络、不提交订单。实盘/纸面联调仍需遵守 #1351 的凭证和权限边界。
