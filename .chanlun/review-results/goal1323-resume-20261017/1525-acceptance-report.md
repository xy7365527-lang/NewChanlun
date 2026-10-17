# #1525 RiskGate 与订单责任回路阶段验收

日期：2026-10-17。本文记录当前共享工作树的离线结构接线与阻塞；不宣称真实下单或整图完成。

## 已验证

- RiskLimits 支持可选独立杠杆上限 max_leverage 和禁止开空 allow_short。
- RiskGate.check 在唯一 facade 入口前可拒绝 leverage_over_limit 与 short_not_allowed，并保留原有单笔、净持仓、频率和 kill switch 判定。
- HyperliquidPaperFacade.submit_limit 注入 RiskGate、权益、净持仓和时钟来源；拒绝在调用 Strategy 前写 rejected 事件，放行后仅调用一次 strategy.submit_order。
- kill switch 拒绝路径调用可注入的 cancel_all_orders 与 close_all_positions，并写 kill_switch_actions。
- pytest -q tests/test_hl_paper_launcher.py tests/test_risk_controls.py：26 passed；仅已有 asyncio_mode 未识别配置警告。
- Tailscale Windows 节点 100.64.200.25 当前 offline；tailscale ping、Tailscale SSH 和直连 10.44.0.2:22 均超时。本轮未写入或使用 Windows 空间。

## 未通过/阻塞

- #1523 launcher 当前仍是共享工作树未跟踪候选，未有真实 Nautilus TradingNode 连接或场所回报。
- 尚无 TB-04 正式账本接纳证明：接受、拒绝、撤单、成交事件尚未绑定到唯一持久账本写入口。
- 余额与持仓的唯一正式来源、paper/live 共核、旧写入口排他、真实运行时拒绝证据仍依赖 #1362/#1365/#1367/#1351 等票据。
- 本阶段未执行真实订单发送；不得据此关闭 #1525、#1279 或 #1323。

## 代码与测试证据

- trading_system/execution/risk_controls.py
- trading_system/execution/hl_paper_launcher.py
- tests/test_risk_controls.py
- tests/test_hl_paper_launcher.py
