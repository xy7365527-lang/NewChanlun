"""Hyperliquid 连接验证 —— hyperliquid-python-sdk 直连（mainnet）。

三阶段验证，全部只读或不可成交：
  1. info   —— 账户余额/仓位（REST，只读）
  2. data   —— BTC 中间价 + 1m K线快照 + trade tick WS 订阅（只读）
  3. order  —— LMT 买单 0.001 BTC @ 远低于市价（tif=Alo 双保险）→ 立即撤单 → 确认无残留

安全约束（硬编码，不可被参数覆盖）：
  - 限价必须 < 中间价 × 0.6，否则拒绝下单
  - tif=Alo（add-liquidity-only）：若交易所判定可立即成交，订单直接被拒，不会 taker 成交
  - 数量固定 0.001 BTC

用法：
  HYPERLIQUID_PRIVATE_KEY=... .venv/bin/python trading_system/live/hl_verify_sdk.py [info|data|order|all]
"""

from __future__ import annotations

import os
import sys
import time

from eth_account import Account
from hyperliquid.exchange import Exchange
from hyperliquid.info import Info
from hyperliquid.utils import constants

ACCOUNT_ADDRESS = "0x752f73AD304734C5D4D192d6Db39AE01d80a8562"
COIN = "BTC"
ORDER_SIZE = 0.001  # BTC，极小仓位
MAX_PX_RATIO = 0.6  # 限价上限 = mid × 0.6（远低于市价，确保不成交）


def _fail(msg: str) -> None:
    print(f"[FAIL] {msg}")
    sys.exit(1)


def stage_info(info: Info) -> None:
    """阶段1：连接 + 账户状态（只读）。"""
    state = info.user_state(ACCOUNT_ADDRESS)
    margin = state["marginSummary"]
    print(f"[OK] 连接 mainnet，账户 {ACCOUNT_ADDRESS}")
    print(f"     accountValue   = {margin['accountValue']} USDC")
    print(f"     totalMarginUsed= {margin['totalMarginUsed']} USDC")
    print(f"     withdrawable   = {state['withdrawable']} USDC")
    positions = [p for p in state.get("assetPositions", []) if float(p["position"]["szi"]) != 0]
    if positions:
        for p in positions:
            pos = p["position"]
            print(f"     持仓 {pos['coin']}: szi={pos['szi']} entryPx={pos['entryPx']}")
    else:
        print("     无持仓")
    spot = info.spot_user_state(ACCOUNT_ADDRESS)
    balances = [b for b in spot.get("balances", []) if float(b["total"]) != 0]
    for b in balances:
        print(f"     现货余额 {b['coin']}: {b['total']}")


def stage_data(info: Info) -> float:
    """阶段2：行情（中间价 + 1m K线 + trade tick WS）。返回 BTC mid。"""
    mids = info.all_mids()
    mid = float(mids[COIN])
    print(f"[OK] all_mids: {COIN} mid = {mid}")

    now_ms = int(time.time() * 1000)
    candles = info.candles_snapshot(COIN, "1m", now_ms - 5 * 60_000, now_ms)
    print(f"[OK] 1m K线快照，{len(candles)} 根：")
    for c in candles[-3:]:
        print(f"     t={c['t']} O={c['o']} H={c['h']} L={c['l']} C={c['c']} V={c['v']} n={c['n']}")

    ticks: list[dict] = []
    info.subscribe({"type": "trades", "coin": COIN}, lambda msg: ticks.extend(msg.get("data", [])))
    deadline = time.time() + 10
    while time.time() < deadline and len(ticks) < 5:
        time.sleep(0.5)
    if not ticks:
        _fail("10s 内未收到任何 trade tick（WS 订阅失败或市场静止）")
    print(f"[OK] trade tick WS：10s 内收到 {len(ticks)} 笔，最新 3 笔：")
    for t in ticks[-3:]:
        print(f"     px={t['px']} sz={t['sz']} side={t['side']} time={t['time']}")
    return mid


def stage_order(info: Info, mid: float) -> None:
    """阶段3：下单通道验证（不可成交的 LMT 买单 → 撤单 → 无残留确认）。"""
    pk = os.environ.get("HYPERLIQUID_PRIVATE_KEY")
    if not pk:
        _fail("环境变量 HYPERLIQUID_PRIVATE_KEY 未设置")
    wallet = Account.from_key(pk)
    role = "主钱包" if wallet.address.lower() == ACCOUNT_ADDRESS.lower() else "agent 钱包"
    print(f"[..] 签名钱包 {wallet.address}（{role}）")

    # 限价 = mid 的一半，取整到 100（BTC 价格规则：≤5 有效数字）
    limit_px = int(mid * 0.5 // 100 * 100)
    if limit_px >= mid * MAX_PX_RATIO:
        _fail(f"安全检查失败：limit_px={limit_px} 不低于 mid×{MAX_PX_RATIO}")
    notional = limit_px * ORDER_SIZE
    if notional < 10:
        _fail(f"名义价值 {notional:.2f} < $10（HL 最小订单价值），无法验证")
    print(f"[..] LMT 买单 {COIN} sz={ORDER_SIZE} px={limit_px}（mid={mid}，距市价 -50%，tif=Alo）")

    exchange = Exchange(wallet, constants.MAINNET_API_URL, account_address=ACCOUNT_ADDRESS)
    result = exchange.order(
        COIN, True, ORDER_SIZE, float(limit_px), {"limit": {"tif": "Alo"}}
    )
    if result.get("status") != "ok":
        _fail(f"下单被拒：{result}")
    status = result["response"]["data"]["statuses"][0]
    if "error" in status:
        _fail(f"订单状态错误：{status['error']}")
    oid = status["resting"]["oid"]
    print(f"[OK] 订单已挂出，oid={oid}")

    cancel = exchange.cancel(COIN, oid)
    if cancel.get("status") != "ok":
        _fail(f"撤单失败（须手动处理挂单 oid={oid}！）：{cancel}")
    print(f"[OK] 撤单成功，oid={oid}")

    remaining = info.open_orders(ACCOUNT_ADDRESS)
    if any(o["oid"] == oid for o in remaining):
        _fail(f"撤单后 oid={oid} 仍在挂单列表！须手动处理")
    print(f"[OK] 无残留：open_orders 共 {len(remaining)} 笔，oid={oid} 不在其中")


def main() -> None:
    stage = sys.argv[1] if len(sys.argv) > 1 else "all"
    need_ws = stage in ("data", "all")
    info = Info(constants.MAINNET_API_URL, skip_ws=not need_ws)

    if stage in ("info", "all"):
        stage_info(info)
    mid: float | None = None
    if stage in ("data", "all"):
        mid = stage_data(info)
    if stage in ("order", "all"):
        if mid is None:
            mid = float(info.all_mids()[COIN])
        stage_order(info, mid)
    print("[DONE] 全部阶段通过" if stage == "all" else f"[DONE] 阶段 {stage} 通过")
    # WS 线程是非 daemon 的，显式退出
    os._exit(0)


if __name__ == "__main__":
    main()
