# #1523 Hyperliquid paper launcher 验收记录

日期：2026-10-17。范围限定为 NautilusTrader 0.62 Hyperliquid paper launcher；本票不提交真实订单，不替代 #1351、#1362、#1365、#1367 的验收。

## 已验证

- `python -m pytest tests/test_hl_paper_launcher.py -q`：5 passed。覆盖 testnet/mainnet 凭证隔离、非法环境、缺凭证早失败、dry-run 不构造节点、日志字段脱敏。
- `python -m trading_system.execution.hl_paper_launcher --dry-run`：成功输出 `environment=testnet` 与脱敏后的凭证环境名；未导入 Nautilus、未联网、未下单。
- `python -m py_compile trading_system/execution/hl_paper_launcher.py`：通过。

## 交付面

`LauncherConfig` 负责环境和凭证 fail-fast；`HyperliquidPaperFacade` 提供限价单、撤单、未结单和净持仓查询，以及 accepted/rejected/filled/canceled 回报；`ReportSink` 以 JSONL 记录并递归脱敏密钥类字段。

## 未宣称

当前机器未执行带真实凭证的 Nautilus TradingNode，也未访问 Hyperliquid testnet/mainnet；因此 adapter 连接、交易所回报、重启恢复、paper/live 同核和旧入口排他仍待对应票据完成。
