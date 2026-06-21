---
name: COIL Options Scan Schedule
description: 定时任务自动拉TWS COIL期权链算凸性，每交易日01:00 BST
type: reference
---

定时任务 ID: coil-convexity-scan
时间: 01:00 BST，周一到周五（目前改为周日 23:00 一次性）
功能: 连TWS拉COIL期权链真实报价，算目标$227最大凸性call

需要 TWS 在运行且 API 端口 7496 开启。

脚本位置: /Users/silencehan/Projects/trading/tws_coil_convexity.py
调仓脚本: /Users/silencehan/Projects/trading/tws_rebalance.py
凸性结果: /Users/silencehan/Projects/trading/coil_live_scan.csv
