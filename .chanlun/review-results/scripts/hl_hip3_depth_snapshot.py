#!/usr/bin/env python3
"""
Hyperliquid HIP-3 builder-deployed perp DEX 深度快照采集器（issue #939 第一程）。

用途：对 `xyz` / `para` / `mkts` 三个 builder DEX 上的标的采一轮 L2 盘口 + 市场上下文，
      算出「单笔 X 美元市价单的冲击成本（bp）」，买卖两边分开记。

依赖：Python 3.9+，仅标准库（urllib / json）。无需 API key，无需任何凭证，全只读。

跑法：
    python3 .chanlun/review-results/scripts/hl_hip3_depth_snapshot.py
    python3 .chanlun/review-results/scripts/hl_hip3_depth_snapshot.py --out /tmp/snap.json --markdown /tmp/snap.md

已知限制（写进产出里，不许在报告里被抹掉）：
  1. `l2Book` 每边最多返回 20 档。若某档位金额吃穿 20 档，本脚本记为 `EXCEEDS_BOOK`，
     **不是** 「冲击成本无穷大」，而是「20 档之内看不到，档外未知」。
  2. 单次运行 = 单一时点 = **样本量 1**，不可作结论。跨时段采样归 issue #933。
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone

INFO_URL = "https://api.hyperliquid.xyz/info"
TIMEOUT = 30  # 秒，与票面纪律一致：失败记错继续，不重试到死

# 采样标的：xyz 取美股大盘股一批 + 指数/商品若干，mkts 取两个指数，para 抽样
COINS = {
    "xyz": [
        "xyz:AAPL", "xyz:NVDA", "xyz:TSLA", "xyz:MSFT", "xyz:META",
        "xyz:GOOGL", "xyz:AMZN", "xyz:NFLX", "xyz:COST", "xyz:MSTR",
        "xyz:COIN", "xyz:PLTR", "xyz:INTC", "xyz:ORCL", "xyz:AMD",
        "xyz:MU", "xyz:TSM", "xyz:LLY", "xyz:XYZ100", "xyz:GOLD",
    ],
    "mkts": ["mkts:US500", "mkts:USTECH"],
    "para": ["para:AVGO", "para:COHR", "para:CRDO", "para:NET", "para:RDDT", "para:IREN"],
}

NOTIONALS = [1_000, 5_000, 20_000, 100_000]


def post(payload: dict) -> object:
    body = json.dumps(payload).encode()
    req = urllib.request.Request(
        INFO_URL, data=body, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=TIMEOUT) as resp:
        return json.loads(resp.read().decode())


def safe_post(payload: dict) -> tuple[object | None, str | None]:
    try:
        return post(payload), None
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError) as exc:
        return None, f"{type(exc).__name__}: {exc}"
    except json.JSONDecodeError as exc:
        return None, f"JSONDecodeError: {exc}"


def impact_bp(levels: list[dict], mid: float, notional: float, side: str) -> dict:
    """吃掉 `notional` 美元名义所需的成交均价 vs 中价的偏离（bp）。

    side='buy' 吃卖盘（asks），side='sell' 吃买盘（bids）。
    返回 {"bp": float|None, "status": "OK"|"EXCEEDS_BOOK", "filled_usd": float}
    """
    remaining = notional
    cost = 0.0
    qty = 0.0
    for lvl in levels:
        px = float(lvl["px"])
        sz = float(lvl["sz"])
        lvl_usd = px * sz
        take_usd = min(remaining, lvl_usd)
        take_qty = take_usd / px
        cost += take_qty * px
        qty += take_qty
        remaining -= take_usd
        if remaining <= 1e-9:
            break
    filled = notional - max(remaining, 0.0)
    if remaining > 1e-9:
        return {"bp": None, "status": "EXCEEDS_BOOK", "filled_usd": round(filled, 2)}
    avg = cost / qty
    bp = (avg - mid) / mid * 1e4
    if side == "sell":
        bp = -bp
    return {"bp": round(bp, 2), "status": "OK", "filled_usd": round(filled, 2)}


def side_depth_usd(levels: list[dict]) -> float:
    return round(sum(float(l["px"]) * float(l["sz"]) for l in levels), 2)


def collect_dex_ctx(dex: str) -> dict:
    data, err = safe_post({"type": "metaAndAssetCtxs", "dex": dex})
    if err:
        return {"error": err}
    meta, ctxs = data[0], data[1]
    out = {}
    for asset, ctx in zip(meta["universe"], ctxs):
        if asset.get("isDelisted"):
            continue
        out[asset["name"]] = {
            "maxLeverage": asset.get("maxLeverage"),
            "marginTableId": asset.get("marginTableId"),
            "marginMode": asset.get("marginMode"),
            "onlyIsolated": asset.get("onlyIsolated"),
            "markPx": ctx.get("markPx"),
            "oraclePx": ctx.get("oraclePx"),
            "midPx": ctx.get("midPx"),
            "dayNtlVlm": ctx.get("dayNtlVlm"),
            "openInterest": ctx.get("openInterest"),
            "funding": ctx.get("funding"),
        }
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=None, help="JSON 落盘路径")
    ap.add_argument("--markdown", default=None, help="Markdown 表格落盘路径")
    args = ap.parse_args()

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    result = {
        "sampled_at_utc": ts,
        "endpoint": INFO_URL,
        "sample_size": 1,
        "caveat": "样本量=1，单一时点快照，不可作结论。l2Book 每边最多 20 档。",
        "perp_dexs": {},
        "books": {},
        "errors": [],
    }

    dexs, err = safe_post({"type": "perpDexs"})
    if err:
        result["errors"].append({"req": "perpDexs", "error": err})
    else:
        for d in dexs:
            if not d:
                continue
            result["perp_dexs"][d["name"]] = {
                "fullName": d.get("fullName"),
                "deployer": d.get("deployer"),
                "oracleUpdater": d.get("oracleUpdater"),
                "feeRecipient": d.get("feeRecipient"),
                "deployerFeeScale": d.get("deployerFeeScale"),
                "lastDeployerFeeScaleChangeTime": d.get("lastDeployerFeeScaleChangeTime"),
                # subDeployers: 部署者把哪些 PerpDeployAction 变体授权给了哪些地址
                "subDeployers": {k: v for k, v in (d.get("subDeployers") or [])},
                "n_oi_cap_entries": len(d.get("assetToStreamingOiCap") or []),
            }

    # 每个 perp dex 的净入金总额（= 该 DEX 独立抵押池的规模）
    result["perp_dex_status"] = {}
    for dex in ["", *result["perp_dexs"].keys()]:
        st, err = safe_post({"type": "perpDexStatus", "dex": dex})
        if err:
            result["errors"].append({"req": f"perpDexStatus:{dex or 'main'}", "error": err})
        else:
            result["perp_dex_status"][dex or "(main)"] = st

    # 隔离性探针：同一个地址在主 DEX 与各 builder DEX 上的 clearinghouseState 是否各自独立
    # 用公开的 xyz 部署者地址（只读查询，不涉及任何凭证）
    probe_addr = "0x88806a71d74ad0a510b350545c9ae490912f0888"
    result["isolation_probe"] = {"user": probe_addr, "states": {}}
    for dex in ["", "xyz", "para", "mkts"]:
        req = {"type": "clearinghouseState", "user": probe_addr}
        if dex:
            req["dex"] = dex
        st, err = safe_post(req)
        if err:
            result["errors"].append({"req": f"clearinghouseState:{dex or 'main'}", "error": err})
        else:
            result["isolation_probe"]["states"][dex or "(main)"] = {
                "accountValue": st.get("marginSummary", {}).get("accountValue"),
                "withdrawable": st.get("withdrawable"),
                "crossMaintenanceMarginUsed": st.get("crossMaintenanceMarginUsed"),
                "n_positions": len(st.get("assetPositions") or []),
            }

    ctxs = {}
    for dex in COINS:
        ctxs[dex] = collect_dex_ctx(dex)
        if isinstance(ctxs[dex], dict) and "error" in ctxs[dex]:
            result["errors"].append({"req": f"metaAndAssetCtxs:{dex}", "error": ctxs[dex]["error"]})
    result["asset_ctx"] = ctxs

    for dex, coins in COINS.items():
        for coin in coins:
            book, err = safe_post({"type": "l2Book", "coin": coin})
            if err:
                result["errors"].append({"req": f"l2Book:{coin}", "error": err})
                continue
            levels = book.get("levels") or [[], []]
            bids, asks = levels[0], levels[1]
            if not bids or not asks:
                result["books"][coin] = {
                    "t_ms": book.get("time"),
                    "note": "单边或双边空盘口",
                    "n_bid_levels": len(bids),
                    "n_ask_levels": len(asks),
                }
                continue
            best_bid, best_ask = float(bids[0]["px"]), float(asks[0]["px"])
            mid = (best_bid + best_ask) / 2
            entry = {
                "t_ms": book.get("time"),
                "best_bid": best_bid,
                "best_ask": best_ask,
                "mid": mid,
                "spread_bp": round((best_ask - best_bid) / mid * 1e4, 2),
                "bid_depth_usd_20lvl": side_depth_usd(bids),
                "ask_depth_usd_20lvl": side_depth_usd(asks),
                "n_bid_levels": len(bids),
                "n_ask_levels": len(asks),
                "impact": {},
            }
            entry["depth_asymmetry_ratio"] = (
                round(entry["bid_depth_usd_20lvl"] / entry["ask_depth_usd_20lvl"], 3)
                if entry["ask_depth_usd_20lvl"] > 0 else None
            )
            for n in NOTIONALS:
                entry["impact"][str(n)] = {
                    "buy": impact_bp(asks, mid, n, "buy"),
                    "sell": impact_bp(bids, mid, n, "sell"),
                }
            ctx = ctxs.get(dex, {}).get(coin, {})
            entry["dayNtlVlm"] = ctx.get("dayNtlVlm")
            entry["openInterest"] = ctx.get("openInterest")
            entry["markPx"] = ctx.get("markPx")
            entry["maxLeverage"] = ctx.get("maxLeverage")
            entry["marginMode"] = ctx.get("marginMode")
            result["books"][coin] = entry
            time.sleep(0.15)  # 轻速率自限，避免打到公共 info 端点的限流

    text = json.dumps(result, ensure_ascii=False, indent=2)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text)
        print(f"JSON -> {args.out}", file=sys.stderr)
    else:
        print(text)

    if args.markdown:
        lines = [
            f"<!-- 采样时点 {ts} UTC / 端点 {INFO_URL} / 样本量=1，不可作结论 -->",
            "",
            "| 标的 | 中价 | 价差bp | 买深20档$ | 卖深20档$ | 不对称(买/卖) | 买1k | 卖1k | 买5k | 卖5k | 买2万 | 卖2万 | 买10万 | 卖10万 | 24h成交$ |",
            "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|",
        ]

        def cell(d):
            return "穿透20档" if d["status"] == "EXCEEDS_BOOK" else f"{d['bp']}"

        for coin, e in result["books"].items():
            if "impact" not in e:
                lines.append(f"| {coin} | — | — | — | — | — | 空盘口 | | | | | | | | |")
                continue
            imp = e["impact"]
            lines.append(
                f"| {coin} | {e['mid']:.4f} | {e['spread_bp']} | {e['bid_depth_usd_20lvl']:,.0f} | "
                f"{e['ask_depth_usd_20lvl']:,.0f} | {e['depth_asymmetry_ratio']} | "
                + " | ".join(
                    cell(imp[str(n)][s]) for n in NOTIONALS for s in ("buy", "sell")
                )
                + f" | {e.get('dayNtlVlm')} |"
            )
        with open(args.markdown, "w") as f:
            f.write("\n".join(lines) + "\n")
        print(f"Markdown -> {args.markdown}", file=sys.stderr)

    if result["errors"]:
        print(f"{len(result['errors'])} 个请求出错（已记录进产出）", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
