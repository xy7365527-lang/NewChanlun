"""品种目录 — 纯数据，无第三方依赖

从 data_databento.py 提取，供 adapters.py 等核心模块安全导入。
"""

from __future__ import annotations

SYMBOL_CATALOG: list[dict] = [
    # 期货 - 能源
    {"symbol": "CL", "type": "FUT", "exchange": "NYMEX", "name": "WTI Crude Oil", "cn": "WTI原油"},
    {"symbol": "BZ", "type": "FUT", "exchange": "CME", "name": "Brent Crude Oil (CME)", "cn": "布伦特原油"},
    {"symbol": "NG", "type": "FUT", "exchange": "NYMEX", "name": "Natural Gas", "cn": "天然气"},
    # 期货 - 贵金属
    {"symbol": "GC", "type": "FUT", "exchange": "COMEX", "name": "Gold", "cn": "黄金"},
    {"symbol": "SI", "type": "FUT", "exchange": "COMEX", "name": "Silver", "cn": "白银"},
    {"symbol": "HG", "type": "FUT", "exchange": "COMEX", "name": "Copper", "cn": "铜"},
    # 期货 - 股指
    {"symbol": "ES", "type": "FUT", "exchange": "CME", "name": "E-mini S&P 500", "cn": "标普500"},
    {"symbol": "NQ", "type": "FUT", "exchange": "CME", "name": "E-mini Nasdaq 100", "cn": "纳指100"},
    {"symbol": "YM", "type": "FUT", "exchange": "CBOT", "name": "E-mini Dow", "cn": "道指"},
    {"symbol": "RTY", "type": "FUT", "exchange": "CME", "name": "E-mini Russell 2000", "cn": "罗素2000"},
    # 期货 - 国债
    {"symbol": "ZB", "type": "FUT", "exchange": "CBOT", "name": "30-Year T-Bond", "cn": "30年国债"},
    {"symbol": "ZN", "type": "FUT", "exchange": "CBOT", "name": "10-Year T-Note", "cn": "10年国债"},
    # 美股
    {"symbol": "AMD", "type": "STK", "exchange": "NASDAQ", "name": "AMD", "cn": "超微半导体"},
    {"symbol": "NVDA", "type": "STK", "exchange": "NASDAQ", "name": "NVIDIA", "cn": "英伟达"},
    {"symbol": "AAPL", "type": "STK", "exchange": "NASDAQ", "name": "Apple", "cn": "苹果"},
    {"symbol": "MSFT", "type": "STK", "exchange": "NASDAQ", "name": "Microsoft", "cn": "微软"},
    {"symbol": "GOOG", "type": "STK", "exchange": "NASDAQ", "name": "Alphabet", "cn": "谷歌"},
    {"symbol": "AMZN", "type": "STK", "exchange": "NASDAQ", "name": "Amazon", "cn": "亚马逊"},
    {"symbol": "META", "type": "STK", "exchange": "NASDAQ", "name": "Meta Platforms", "cn": "Meta"},
    {"symbol": "TSLA", "type": "STK", "exchange": "NASDAQ", "name": "Tesla", "cn": "特斯拉"},
    {"symbol": "NFLX", "type": "STK", "exchange": "NASDAQ", "name": "Netflix", "cn": "奈飞"},
    {"symbol": "SPY", "type": "ETF", "exchange": "NYSE", "name": "S&P 500 ETF", "cn": "标普ETF"},
    {"symbol": "QQQ", "type": "ETF", "exchange": "NASDAQ", "name": "Nasdaq 100 ETF", "cn": "纳指ETF"},
    {"symbol": "IWM", "type": "ETF", "exchange": "NYSE", "name": "Russell 2000 ETF", "cn": "罗素ETF"},
]
