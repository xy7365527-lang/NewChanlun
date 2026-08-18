#!/usr/bin/env python3
"""冲击成本探针（issue #933）：BingX / Hyperliquid 股票永续「每一重能装多少钱」的采样器。

阶段一交付件三。每轮 = 存活标的 × L2 盘口快照 → 模拟单笔 1k / 5k / 2万 / 10万 USD 市价单
吃单（按盘口逐档成交到目标量，VWAP-mid 换算 bp）→ JSONL 追加落盘。只读公开端点、不下真单、
无需 API key。

端点（均免签名，2026-08-18 实测存活）：

- BingX（v2 是股票永续唯一返回非空簿的版本，v3 对股票返回空簿——在案实证）：
    * 全部合约：GET https://open-api.bingx.com/openApi/swap/v2/quote/contracts
    * 单合约盘口：GET https://open-api.bingx.com/openApi/swap/v2/quote/depth?symbol=X&limit=1000
      返回 {"code":0,"data":{"T":...,"bids":[[px,qty],...],"asks":[[px,qty],...]}}
      股票永续符号形态 `NCSK{AAPL}2USD-USDT`（displayName 为 `AAPL-USDT`）。
- Hyperliquid（builder DEX，股票永续不在主 DEX；`{"type":"metaAndAssetCtxs","dex":D}` 列存活标的，
  `{"type":"l2Book","coin":C,"dex":D}` 拉深度，每档 {px,sz,n} 均为字符串，levels=[bids,asks]）：
    * POST https://api.hyperliquid.xyz/info

存活定义：

- BingX：NCSK* 合约里 `status == 1` 且 `apiStateOpen == "true"`（可开仓）。`status == 1` 但
  apiStateOpen=false 的有簿但 API 开仓关闭；`status == 25` 为暂停（深度报 109415）。本脚本
  默认只采「可开仓」档；其余两档数量照实记入 rounds.jsonl。
- Hyperliquid：`metaAndAssetCtxs` 里 `isDelisted` 为假。默认 DEX = xyz / para / mkts（km 23/23
  全退市、hyna 与主 DEX 为加密币，均不在股票永续范围）。

输出：

- 每条样本一行 JSON 追加 `analysis/data_cache/impact_probe/samples.jsonl`，字段含 UTC 时点 /
  标的 / 场所 / 原始档位数 / 双侧 10bp 深度 / 四档冲击成本（深度不足记 null，不编数）。
- 每轮一份汇总追加 `analysis/data_cache/impact_probe/rounds.jsonl`（各场所存活数 / 采样数 /
  深度失败数——场所本身在萎缩是冲击成本之外更大的信号，每轮记录存活数）。

用法：

    uv run python scripts/probe_impact_cost.py                          # 全宇宙一轮
    uv run python scripts/probe_impact_cost.py --symbols AAPL,NVDA,TSLA # 只测点名标的
    uv run python scripts/probe_impact_cost.py --venue bingx            # 只测 BingX
    uv run python scripts/probe_impact_cost.py --dex xyz,mkts           # 只测 HL 指定 DEX

幂等可重跑（cron 定时用）：append-only，重跑只追加新一轮，不覆盖历史。
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import requests

HL_ENDPOINT = "https://api.hyperliquid.xyz/info"
BINGX_BASE = "https://open-api.bingx.com"
BINGX_CONTRACTS_URL = f"{BINGX_BASE}/openApi/swap/v2/quote/contracts"
BINGX_DEPTH_URL = f"{BINGX_BASE}/openApi/swap/v2/quote/depth"

DEFAULT_SIZES = [1000, 5000, 20000, 100000]
DEFAULT_HL_DEXES = ["xyz", "para", "mkts"]
DEFAULT_OUT_DIR = Path("analysis/data_cache/impact_probe")

# 股票永续符号前缀（BingX）；NCSK* 之外的合约是加密/外汇/商品，不在本票范围。
BINGX_STOCK_PREFIX = "NCSK"


def now_iso() -> str:
    """UTC ISO8601 秒级时点，每条读数与每轮汇总都带上。"""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def http_get_json(url: str, timeout: float, retries: int, delay: float) -> Any:
    """GET + JSON，带重试与退避。失败抛 RuntimeError（不静默）。"""
    last_exc: Exception | None = None
    for attempt in range(retries):
        try:
            resp = requests.get(url, timeout=timeout)
            resp.raise_for_status()
            return resp.json()
        except Exception as exc:  # noqa: BLE001 —— 网络/解析错误统一重试
            last_exc = exc
            if attempt < retries - 1:
                time.sleep(delay * (attempt + 1))
    raise RuntimeError(f"GET {url} failed after {retries} attempts: {last_exc}")


def http_post_json(url: str, payload: dict, timeout: float, retries: int, delay: float) -> Any:
    """POST + JSON，带重试与退避。"""
    last_exc: Exception | None = None
    for attempt in range(retries):
        try:
            resp = requests.post(url, json=payload, timeout=timeout)
            resp.raise_for_status()
            return resp.json()
        except Exception as exc:  # noqa: BLE001
            last_exc = exc
            if attempt < retries - 1:
                time.sleep(delay * (attempt + 1))
    raise RuntimeError(f"POST {url} {payload!r} failed after {retries} attempts: {last_exc}")


def hl_meta_and_asset_ctxs(dex: str, timeout: float, retries: int, delay: float) -> list:
    """返回 [universe, assetCtxs]，两表按下标对齐。"""
    data = http_post_json(
        HL_ENDPOINT, {"type": "metaAndAssetCtxs", "dex": dex}, timeout, retries, delay
    )
    if not isinstance(data, list) or len(data) < 2:
        raise RuntimeError(f"HL metaAndAssetCtxs({dex}) 返回形状异常: {data!r}")
    universe = data[0]
    if not isinstance(universe, dict) or "universe" not in universe:
        raise RuntimeError(f"HL metaAndAssetCtxs({dex}) universe 字段形状异常: {universe!r}")
    return universe["universe"], data[1]


def hl_l2_book(coin: str, dex: str, timeout: float, retries: int, delay: float) -> dict:
    return http_post_json(
        HL_ENDPOINT, {"type": "l2Book", "coin": coin, "dex": dex}, timeout, retries, delay
    )


def bingx_contracts(timeout: float, retries: int, delay: float) -> list:
    data = http_get_json(BINGX_CONTRACTS_URL, timeout, retries, delay)
    if data.get("code") != 0:
        raise RuntimeError(f"BingX contracts 返回 code={data.get('code')} msg={data.get('msg')}")
    return data.get("data", [])


def bingx_depth(symbol: str, limit: int, timeout: float, retries: int, delay: float) -> dict:
    data = http_get_json(
        f"{BINGX_DEPTH_URL}?symbol={symbol}&limit={limit}", timeout, retries, delay
    )
    if data.get("code") != 0:
        raise RuntimeError(f"BingX depth({symbol}) 返回 code={data.get('code')} msg={data.get('msg')}")
    return data.get("data", {})


def _to_float(v: Any) -> float:
    return float(v)


def parse_bingx_levels(data: dict) -> tuple[list[tuple[float, float]], list[tuple[float, float]]]:
    """BingX depth: bids=[[px,qty],...] 降序；asks=[[px,qty],...] 升序。"""
    bids = [(_to_float(p), _to_float(q)) for p, q in data.get("bids", [])]
    asks = [(_to_float(p), _to_float(q)) for p, q in data.get("asks", [])]
    return bids, asks


def parse_hl_levels(book: dict) -> tuple[list[tuple[float, float]], list[tuple[float, float]]]:
    """HL l2Book: levels=[bids,asks]，每档 {px,sz,n} 均为字符串。"""
    levels = book.get("levels", [])
    bids_raw = levels[0] if len(levels) > 0 else []
    asks_raw = levels[1] if len(levels) > 1 else []
    bids = [(_to_float(l["px"]), _to_float(l["sz"])) for l in bids_raw]
    asks = [(_to_float(l["px"]), _to_float(l["sz"])) for l in asks_raw]
    return bids, asks


def walk_book(levels: list[tuple[float, float]], target_notional: float) -> tuple[float, float] | None:
    """按盘口逐档吃到目标名义额，返回 (vwap, 实际成交名义)。

    levels 须按「最优价在前」排序（买吃 asks 升序 / 卖吃 bids 降序）。最后一档按比例部分成交。
    深度不足返回 None（不编数）。
    """
    cum_notional = 0.0
    cum_qty = 0.0
    for px, sz in levels:
        remaining = target_notional - cum_notional
        if px * sz >= remaining:
            take_qty = remaining / px
            cum_notional += px * take_qty
            cum_qty += take_qty
            break
        cum_notional += px * sz
        cum_qty += sz
    if cum_notional + 1e-9 < target_notional:
        return None
    return cum_notional / cum_qty, cum_notional


def depth_within_bp(levels: list[tuple[float, float]], mid: float, bp: float, side: str) -> float:
    """mid ± bp 区间内的名义深度（USD）。side='buy' 用 asks，'sell' 用 bids。"""
    limit = mid * (1.0 + bp / 1e4) if side == "buy" else mid * (1.0 - bp / 1e4)
    acc = 0.0
    for px, sz in levels:
        if side == "buy" and px > limit:
            break
        if side == "sell" and px < limit:
            break
        acc += px * sz
    return acc


def impact_bp_for_sizes(
    levels: list[tuple[float, float]], mid: float, side: str, sizes: list[int]
) -> dict[str, float | None]:
    out: dict[str, float | None] = {}
    for target in sizes:
        filled = walk_book(levels, target)
        if filled is None:
            out[str(target)] = None
            continue
        vwap, _ = filled
        bp = (vwap - mid) / mid * 1e4 if side == "buy" else (mid - vwap) / mid * 1e4
        out[str(target)] = round(bp, 4)
    return out


def sample_symbol(
    *,
    venue: str,
    symbol: str,
    dex: str | None,
    base: str,
    extra: dict,
    bids: list[tuple[float, float]],
    asks: list[tuple[float, float]],
    sizes: list[int],
) -> dict:
    """由盘口两侧构造一条样本记录。书为空/单侧时 mid 记为 null，冲击记为 null（照实）。"""
    rec: dict[str, Any] = {
        "ts": now_iso(),
        "venue": venue,
        "symbol": symbol,
        "base": base,
        "n_levels_bid": len(bids),
        "n_levels_ask": len(asks),
    }
    if dex is not None:
        rec["dex"] = dex
    rec.update(extra)

    if not bids or not asks:
        rec["mid"] = None
        rec["spread_bp"] = None
        rec["depth10bp_bid_usd"] = None
        rec["depth10bp_ask_usd"] = None
        rec["impact_bp_buy"] = {str(s): None for s in sizes}
        rec["impact_bp_sell"] = {str(s): None for s in sizes}
        return rec

    best_bid, best_ask = bids[0][0], asks[0][0]
    mid = (best_bid + best_ask) / 2.0
    rec["mid"] = round(mid, 8)
    rec["spread_bp"] = round((best_ask - best_bid) / mid * 1e4, 4)
    rec["depth10bp_bid_usd"] = round(depth_within_bp(bids, mid, 10.0, "sell"), 2)
    rec["depth10bp_ask_usd"] = round(depth_within_bp(asks, mid, 10.0, "buy"), 2)
    rec["impact_bp_buy"] = impact_bp_for_sizes(asks, mid, "buy", sizes)
    rec["impact_bp_sell"] = impact_bp_for_sizes(bids, mid, "sell", sizes)
    return rec


def base_from_bingx_display_name(display_name: str) -> str:
    """从 displayName 提取 base 标的（剥掉 -USDT/-USDC/-USD 结算后缀）。"""
    for suffix in ("-USDT", "-USDC", "-USD"):
        if display_name.endswith(suffix):
            return display_name[: -len(suffix)]
    return display_name.split("-")[0]


def base_from_hl_symbol(symbol: str) -> str:
    return symbol.split(":")[1] if ":" in symbol else symbol


def discover_bingx_universe(timeout: float, retries: int, delay: float) -> tuple[list[dict], dict]:
    """返回 (可开仓合约列表, 全 NCSK 状态计数)。"""
    contracts = bingx_contracts(timeout, retries, delay)
    stocks = [c for c in contracts if c.get("symbol", "").startswith(BINGX_STOCK_PREFIX)]
    status_counts: dict[str, int] = {}
    tradable: list[dict] = []
    for c in stocks:
        status = c.get("status")
        api_open = c.get("apiStateOpen")
        key = f"status={status},apiOpen={api_open}"
        status_counts[key] = status_counts.get(key, 0) + 1
        if status == 1 and api_open == "true":
            tradable.append(c)
    summary = {"n_ncsk_total": len(stocks), "n_contracts_total": len(contracts)}
    summary.update(status_counts)
    return tradable, summary


def discover_hl_universe(
    dexes: list[str], timeout: float, retries: int, delay: float
) -> tuple[list[dict], dict]:
    """返回 (存活标的列表, 各 DEX 存活/退市计数)。"""
    symbols: list[dict] = []
    summary: dict[str, Any] = {}
    for dex in dexes:
        uni, ctxs = hl_meta_and_asset_ctxs(dex, timeout, retries, delay)
        alive = 0
        dead = 0
        for i, u in enumerate(uni):
            if u.get("isDelisted", False):
                dead += 1
                continue
            alive += 1
            ctx = ctxs[i] if i < len(ctxs) else {}
            symbols.append(
                {
                    "dex": dex,
                    "symbol": u.get("name"),
                    "oi": ctx.get("openInterest"),
                    "day_ntl_vlm": ctx.get("dayNtlVlm"),
                    "mark_px": ctx.get("markPx"),
                    "mid_px": ctx.get("midPx"),
                }
            )
        summary[dex] = {"alive": alive, "delisted": dead}
    return symbols, summary


def main() -> int:
    parser = argparse.ArgumentParser(description="issue #933 冲击成本探针")
    parser.add_argument("--venue", choices=["all", "bingx", "hyperliquid"], default="all")
    parser.add_argument("--dex", default=",".join(DEFAULT_HL_DEXES),
                        help="HL builder DEX 逗号分隔，默认 xyz,para,mkts")
    parser.add_argument("--symbols", default="",
                        help="逗号分隔 base 标的过滤（如 AAPL,NVDA），空 = 全宇宙")
    parser.add_argument("--sizes", default=",".join(map(str, DEFAULT_SIZES)),
                        help="市价单名义额档位（USD，逗号分隔）")
    parser.add_argument("--out", default=str(DEFAULT_OUT_DIR), help="JSONL 输出目录")
    parser.add_argument("--limit", type=int, default=1000, help="BingX 盘口档数上限")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--delay", type=float, default=0.05, help="请求间延时（秒），防限频")
    args = parser.parse_args()

    sizes = [int(x) for x in args.sizes.split(",") if x.strip()]
    dexes = [x.strip() for x in args.dex.split(",") if x.strip()]
    symbol_filter = {x.strip().upper() for x in args.symbols.split(",") if x.strip()}

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    samples_path = out_dir / "samples.jsonl"
    rounds_path = out_dir / "rounds.jsonl"

    round_ts = now_iso()
    round_summary: dict[str, Any] = {"ts": round_ts, "sizes": sizes}
    n_written = 0
    n_depth_fail = 0

    # ---- BingX ----
    if args.venue in ("all", "bingx"):
        tradable, bingx_summary = discover_bingx_universe(args.timeout, args.retries, args.delay)
        round_summary["bingx"] = bingx_summary
        targets = tradable
        if symbol_filter:
            targets = [c for c in targets if base_from_bingx_display_name(c.get("displayName", "")).upper() in symbol_filter]
        round_summary["bingx"]["sampled"] = len(targets)
        print(f"[bingx] 可开仓股票永续 {len(tradable)} 只，本轮采样 {len(targets)} 只", file=sys.stderr)
        for c in targets:
            symbol = c.get("symbol")
            base = base_from_bingx_display_name(c.get("displayName", symbol or ""))
            extra = {
                "status": c.get("status"),
                "api_state": c.get("apiStateOpen"),
                "trigger_fee_rate": c.get("triggerFeeRate"),
                "ensure_trigger": c.get("ensureTrigger"),
            }
            try:
                book = bingx_depth(symbol, args.limit, args.timeout, args.retries, args.delay)
                bids, asks = parse_bingx_levels(book)
                rec = sample_symbol(
                    venue="bingx", symbol=symbol, dex=None, base=base, extra=extra,
                    bids=bids, asks=asks, sizes=sizes,
                )
                rec["book_ts"] = book.get("T")
            # 单标的兜底：除网络/业务码（RuntimeError）外，畸形响应（解析失败/字段缺失/
            # 非数值档位）也会让整轮崩溃——按票面「某标的测不到就写测不到」纪律，逐标的
            # 任何异常都落 error 记录而非拖垮 287 只的全量轮次（成功路径零语义变化）。
            except Exception as exc:
                n_depth_fail += 1
                rec = {
                    "ts": now_iso(), "venue": "bingx", "symbol": symbol, "base": base,
                    "error": str(exc), **extra,
                }
            with samples_path.open("a", encoding="utf-8") as f:
                f.write(json.dumps(rec, ensure_ascii=False) + "\n")
            n_written += 1
            time.sleep(args.delay)

    # ---- Hyperliquid ----
    if args.venue in ("all", "hyperliquid"):
        hl_symbols, hl_summary = discover_hl_universe(dexes, args.timeout, args.retries, args.delay)
        round_summary["hyperliquid"] = hl_summary
        targets = hl_symbols
        if symbol_filter:
            targets = [s for s in targets if base_from_hl_symbol(s["symbol"]).upper() in symbol_filter]
        round_summary["hyperliquid"]["sampled"] = len(targets)
        print(f"[hyperliquid] 存活股票/指数永续 {len(hl_symbols)} 只，本轮采样 {len(targets)} 只", file=sys.stderr)
        for s in targets:
            dex, symbol = s["dex"], s["symbol"]
            base = base_from_hl_symbol(symbol)
            extra = {"oi": s.get("oi"), "day_ntl_vlm": s.get("day_ntl_vlm"), "mark_px": s.get("mark_px")}
            try:
                book = hl_l2_book(symbol, dex, args.timeout, args.retries, args.delay)
                bids, asks = parse_hl_levels(book)
                rec = sample_symbol(
                    venue="hyperliquid", symbol=symbol, dex=dex, base=base, extra=extra,
                    bids=bids, asks=asks, sizes=sizes,
                )
                rec["book_ts"] = book.get("time")
            # 同上（单标的兜底）：HL 畸形响应（levels 缺失/档位字段缺失/非数值 sz）逐标的
            # 落 error 记录，不拖垮全量轮次。
            except Exception as exc:
                n_depth_fail += 1
                rec = {
                    "ts": now_iso(), "venue": "hyperliquid", "dex": dex,
                    "symbol": symbol, "base": base, "error": str(exc), **extra,
                }
            with samples_path.open("a", encoding="utf-8") as f:
                f.write(json.dumps(rec, ensure_ascii=False) + "\n")
            n_written += 1
            time.sleep(args.delay)

    round_summary["n_written"] = n_written
    round_summary["n_depth_fail"] = n_depth_fail
    with rounds_path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(round_summary, ensure_ascii=False) + "\n")

    print(f"[done] 本轮写 {n_written} 条样本（深度失败 {n_depth_fail}），"
          f"汇总落 {rounds_path}，样本落 {samples_path}", file=sys.stderr)
    print(json.dumps(round_summary, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
