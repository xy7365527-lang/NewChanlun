from ib_insync import *
import time

ib = IB()
# 试7496先，不行试7497
try:
    ib.connect('127.0.0.1', 7496, clientId=201, timeout=10)
except:
    try:
        ib.connect('127.0.0.1', 7497, clientId=201, timeout=10)
    except:
        print("BOTH PORTS FAILED")
        exit()

ib.reqMarketDataType(4)

# 1. 完整持仓
print("=== POSITIONS ===")
positions = ib.positions()
for p in positions:
    c = p.contract
    print(f"{c.symbol} | {c.secType} | {c.lastTradeDateOrContractMonth} | strike={c.strike} | right={c.right} | qty={p.position} | avgCost={p.avgCost}")

# 2. 账户
print("\n=== ACCOUNT ===")
account = ib.accountSummary()
for item in account:
    if item.tag in ['NetLiquidation', 'AvailableFunds', 'BuyingPower', 'InitMarginReq', 'MaintMarginReq', 'ExcessLiquidity', 'TotalCashValue', 'GrossPositionValue']:
        print(f"{item.tag}: {item.value}")

# 3. whatIf做空MU 100/200/300股
print("\n=== WHATIF SHORT MU ===")
mu = Stock('MU', 'SMART', 'USD')
ib.qualifyContracts(mu)

for qty in [100, 200, 300, 500]:
    order = MarketOrder('SELL', qty)
    try:
        whatif = ib.whatIfOrder(mu, order)
        ib.sleep(1)
        print(f"Short MU {qty}股: initMargin={whatif.initMarginChange}, maintMargin={whatif.maintMarginChange}, equity={whatif.equityWithLoanChange}")
    except Exception as e:
        print(f"Short MU {qty}股: ERROR {e}")

ib.disconnect()
