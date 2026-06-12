#!/usr/bin/env python3
"""TWS 限价买入腾讯涡轮 28316（全仓）。

合约：SEHK 28316，认购涡轮（WAR/C），正股 700.HK，行权价 650.50，
到期 2026-10-21，换股比率 100，每手 10000 股。

流程：连接 TWS(7496) → 查可用资金 → 取实时 bid → 算全仓手数 →
打印订单预览 → 用户确认(y/n) → 提交 LMT 单 → 监控至成交或超时。

硬约束：只用 LMT 单，限价 = 当前 bid。绝不自动下单——必须人工确认。
"""

import math
import sys
import time

from ib_insync import IB, Contract, Forex, LimitOrder

# ── 配置 ──────────────────────────────────────────────────────────
HOST = "127.0.0.1"
PORT = 7496          # TWS live
CLIENT_ID = 28316

WARRANT_CODE = "28316"
EXPECTED_STRIKE = 650.50
EXPECTED_EXPIRY = "20261021"   # lastTradeDateOrContractMonth，qualify 后核对
EXPECTED_RIGHT = "C"
LOT_SIZE = 10_000              # 港股涡轮每手 10000 股

MKT_DATA_TIMEOUT = 15          # 等实时报价的秒数
ORDER_TIMEOUT = 300            # 下单后监控的秒数


def fail(msg: str) -> None:
    print(f"\n[失败] {msg}")
    sys.exit(1)


def resolve_warrant(ib: IB) -> Contract:
    """解析并核验 28316 合约。strike/到期/right 任一不符即终止。"""
    base = Contract(
        secType="WAR",
        localSymbol=WARRANT_CODE,
        exchange="SEHK",
        currency="HKD",
    )
    qualified = ib.qualifyContracts(base)
    if not qualified:
        # 部分 IB 版本对 SEHK 用 symbol 而非 localSymbol
        base = Contract(
            secType="WAR",
            symbol=WARRANT_CODE,
            exchange="SEHK",
            currency="HKD",
        )
        qualified = ib.qualifyContracts(base)
    if not qualified:
        fail(f"无法解析合约 SEHK:{WARRANT_CODE}（WAR）。检查 TWS 是否登录、代码是否正确。")

    c = qualified[0]
    print(f"\n[合约解析] conId={c.conId}")
    print(f"  localSymbol={c.localSymbol}  secType={c.secType}  exchange={c.exchange}")
    print(f"  right={c.right}  strike={c.strike}  expiry={c.lastTradeDateOrContractMonth}")
    print(f"  multiplier={c.multiplier}  currency={c.currency}")

    if c.right != EXPECTED_RIGHT:
        fail(f"合约 right={c.right}，预期 {EXPECTED_RIGHT}（认购）。拒绝继续。")
    if abs(c.strike - EXPECTED_STRIKE) > 1e-6:
        fail(f"合约 strike={c.strike}，预期 {EXPECTED_STRIKE}。拒绝继续。")
    if not c.lastTradeDateOrContractMonth.startswith("202610"):
        fail(
            f"合约到期={c.lastTradeDateOrContractMonth}，预期 {EXPECTED_EXPIRY} 附近。拒绝继续。"
        )
    if c.lastTradeDateOrContractMonth != EXPECTED_EXPIRY:
        # IB 返回的最后交易日可能与挂牌到期日差几天（HK 涡轮常见），打印差异但不终止
        print(
            f"  [注意] IB 最后交易日 {c.lastTradeDateOrContractMonth} ≠ 预期 {EXPECTED_EXPIRY}"
            "（HK 涡轮最后交易日通常早于到期日，月份一致即放行）"
        )
    return c


def get_available_funds_hkd(ib: IB) -> float:
    """读 AvailableFunds 并换算为 HKD（基础货币非 HKD 时走 IDEALPRO 中间价）。"""
    funds = None
    base_ccy = None
    for row in ib.accountSummary():
        if row.tag == "AvailableFunds":
            funds = float(row.value)
            base_ccy = row.currency
            break
    if funds is None:
        fail("读不到 AvailableFunds。检查 TWS API 权限。")

    print(f"\n[账户] AvailableFunds = {funds:,.2f} {base_ccy}")
    if base_ccy == "HKD":
        return funds

    fx = Forex(f"{base_ccy}HKD")
    ib.qualifyContracts(fx)
    ticker = ib.reqMktData(fx, "", False, False)
    deadline = time.monotonic() + MKT_DATA_TIMEOUT
    while time.monotonic() < deadline:
        ib.sleep(0.5)
        mid = ticker.midpoint()
        if mid and not math.isnan(mid):
            ib.cancelMktData(fx)
            funds_hkd = funds * mid
            print(f"[换汇] {base_ccy}HKD 中间价 {mid:.4f} → 可用资金 ≈ {funds_hkd:,.2f} HKD")
            return funds_hkd
    fail(f"{base_ccy}HKD 汇率超时，无法换算可用资金。")


def get_live_bid(ib: IB, contract: Contract) -> tuple[float, float]:
    """取实时 bid/ask。拿不到有效 bid 直接终止——不退回任何估计价。"""
    ticker = ib.reqMktData(contract, "", False, False)
    deadline = time.monotonic() + MKT_DATA_TIMEOUT
    while time.monotonic() < deadline:
        ib.sleep(0.5)
        bid, ask = ticker.bid, ticker.ask
        if bid and bid > 0 and not math.isnan(bid):
            ib.cancelMktData(contract)
            print(f"\n[行情] bid={bid}  ask={ask if ask and not math.isnan(ask) else 'N/A'}")
            return bid, (ask if ask and not math.isnan(ask) else float("nan"))
    fail("实时 bid 超时。检查行情订阅（SEHK 涡轮需要港股 L1）与交易时段。")


def main() -> None:
    ib = IB()
    try:
        ib.connect(HOST, PORT, clientId=CLIENT_ID, timeout=15)
    except Exception as e:
        fail(f"连接 TWS {HOST}:{PORT} 失败：{e}")

    try:
        contract = resolve_warrant(ib)
        funds_hkd = get_available_funds_hkd(ib)
        bid, ask = get_live_bid(ib, contract)

        lot_cost = bid * LOT_SIZE
        lots = int(funds_hkd // lot_cost)
        if lots < 1:
            fail(
                f"可用资金 {funds_hkd:,.2f} HKD 不足一手"
                f"（一手 = {LOT_SIZE} 股 × {bid} = {lot_cost:,.2f} HKD）。"
            )
        quantity = lots * LOT_SIZE
        est_cost = quantity * bid

        print("\n" + "=" * 60)
        print("订单预览（尚未提交）")
        print("=" * 60)
        print(f"  合约     : SEHK {WARRANT_CODE} 腾讯认购涡轮 (conId={contract.conId})")
        print(f"             strike={contract.strike}  expiry={contract.lastTradeDateOrContractMonth}")
        print(f"  方向     : BUY")
        print(f"  类型     : LMT（限价单）")
        print(f"  限价     : {bid} HKD（= 当前 bid）")
        print(f"  数量     : {quantity:,} 股（{lots} 手 × {LOT_SIZE}）")
        print(f"  预估成本 : {est_cost:,.2f} HKD（不含佣金）")
        print(f"  可用资金 : {funds_hkd:,.2f} HKD")
        print(f"  当前 ask : {ask}")
        print("=" * 60)

        answer = input("确认下单？y/n: ").strip().lower()
        if answer != "y":
            print("已取消，未提交任何订单。")
            return

        order = LimitOrder("BUY", quantity, bid, tif="DAY")
        trade = ib.placeOrder(contract, order)
        print(f"\n[已提交] orderId={trade.order.orderId}，开始监控（最长 {ORDER_TIMEOUT}s）……")

        deadline = time.monotonic() + ORDER_TIMEOUT
        last_status = ""
        while time.monotonic() < deadline:
            ib.sleep(1)
            st = trade.orderStatus
            line = (
                f"status={st.status}  filled={st.filled}  remaining={st.remaining}"
                f"  avgFillPrice={st.avgFillPrice}"
            )
            if line != last_status:
                print(f"  [{time.strftime('%H:%M:%S')}] {line}")
                last_status = line
            if trade.isDone():
                break

        st = trade.orderStatus
        if st.status == "Filled":
            print(f"\n[成交] {st.filled:,.0f} 股 @ 均价 {st.avgFillPrice}")
        elif trade.isDone():
            print(f"\n[终态] 订单状态：{st.status}（未成交即结束，可能被拒/已撤）")
        else:
            print(f"\n[超时] {ORDER_TIMEOUT}s 内未成交，当前状态：{st.status}，已成交 {st.filled}")
            cancel = input("撤销剩余订单？y/n: ").strip().lower()
            if cancel == "y":
                ib.cancelOrder(order)
                ib.sleep(2)
                print(f"已发出撤单，最终状态：{trade.orderStatus.status}")
            else:
                print("订单保留在 TWS 中继续工作（DAY 单收盘自动失效）。")
    finally:
        ib.disconnect()


if __name__ == "__main__":
    main()
