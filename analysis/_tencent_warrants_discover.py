"""腾讯 700.HK 港股涡轮发现：拉全部 call warrant 合约详情，dump 字段结构。"""
import json
import math
import sys

from ib_insync import IB, Contract

ib = IB()
ib.connect("127.0.0.1", 7496, clientId=73, timeout=15)
print("connected:", ib.isConnected(), flush=True)

# SEHK 涡轮：secType=WAR，underlying 腾讯
c = Contract(secType="WAR", symbol="700", exchange="SEHK", currency="HKD", right="C")
cds = ib.reqContractDetails(c)
print("call warrants found:", len(cds), flush=True)

if not cds:
    # 备选：用 TENCENT 作 symbol 再试
    c2 = Contract(secType="WAR", symbol="TENCENT", exchange="SEHK", currency="HKD", right="C")
    cds = ib.reqContractDetails(c2)
    print("fallback TENCENT found:", len(cds), flush=True)

rows = []
for cd in cds:
    k = cd.contract
    rows.append(
        {
            "conId": k.conId,
            "localSymbol": k.localSymbol,
            "symbol": k.symbol,
            "strike": k.strike,
            "right": k.right,
            "expiry": k.lastTradeDateOrContractMonth,
            "multiplier": k.multiplier,
            "tradingClass": k.tradingClass,
            "longName": cd.longName,
            "minTick": cd.minTick,
            "underConId": cd.underConId,
            "underSymbol": cd.underSymbol,
            "underSecType": cd.underSecType,
        }
    )

with open("analysis/_tencent_warrants_contracts.json", "w") as f:
    json.dump(rows, f, ensure_ascii=False, indent=1)

# 抽样打印 3 条看字段
for r in rows[:3]:
    print(json.dumps(r, ensure_ascii=False), flush=True)

# 行权价分布
strikes = sorted({r["strike"] for r in rows})
print("strike range:", strikes[:5], "...", strikes[-5:] if len(strikes) > 5 else "")
expiries = sorted({r["expiry"] for r in rows})
print("expiries:", expiries[:10], "..." if len(expiries) > 10 else "")
print("total saved:", len(rows))

ib.disconnect()
