#!/usr/bin/env python3
"""跨国 K4 闭合验证 driver（Rust 引擎）。

用真实 1min（US 顶点 + Au/Oil 折叠）+ 1h（FX 货币层 + EU/JP 生产边）数据，
跑 src/newchan/topology/cross_national_pipeline.py 的三层结构：

  层 0  结算尺空间 Σ（FX 货币边，254号定理2 同步断裂候选）
  层 1  各经济体 K4 配置 Γ_i=(σ_P,σ_C,σ_R)（独立边 X/M = 本币序列本身）
  C路径 全局折叠通道 Au=C↔M（GC）/ Oil=C→P（CL），ω=金/油（482号）

引擎：newchan_rust.RecursiveOrchestrator（逐位等价 Python，1min 量级唯一可行）。
认识论等级：L2/L3（真实多标的多分辨率数据，可产生否定性结果）。

逐边计时 + flush，结果落 analysis/cross_national_k4_verification_data.json。
"""
from __future__ import annotations

import json
import time
from pathlib import Path

from newchan.topology import cross_national_pipeline as P
from newchan.topology.graph import Vertex

OUT = Path("analysis/cross_national_k4_verification_data.json")


def _edge_to_dict(r) -> dict:
    return {
        "name": r.name,
        "bar_count": r.bar_count,
        "date_range": list(r.date_range),
        "sigma": r.sigma.name,
        "move_kind": r.move_kind,
        "move_direction": r.move_direction,
        "move_settled": r.move_settled,
        "max_level": r.max_level,
        "last_price": r.last_price,
    }


def _run_edge(label: str, fname: str) -> dict:
    """跑单条边并计时；返回 EdgeReading 的 dict + 耗时。"""
    t = time.time()
    reading = P._run_recursive(label, P._load_series(fname))
    dt = time.time() - t
    d = _edge_to_dict(reading)
    d["seconds"] = round(dt, 2)
    print(
        f"[{dt:8.1f}s] {label:14s} n={d['bar_count']:>7} "
        f"sigma={d['sigma']:5s} kind={d['move_kind']:13s} "
        f"dir={d['move_direction']:5s} settled={d['move_settled']!s:5s} "
        f"maxlev={d['max_level']}",
        flush=True,
    )
    return d


# 边清单：按预估耗时升序（1h 与中国短窗口 1min 先，1min 大边后）。
# (key, label, filename, 角色说明)
EDGES = [
    ("fx_eu", "FX:EUR/USD", "eurusd_1h_databento.json", "层0 货币边 EUR/USD"),
    ("fx_jp", "FX:JPY/USD", "usdjpy_1h_databento.json", "层0 货币边 JPY/USD"),
    ("fx_cn", "FX:CNH/USD", "usdcnh_1h_databento.json", "层0 货币边 CNH/USD"),
    ("eu_p", "EU:P/M", "fesx_1h_databento.json", "层1 EU 生产边 STOXX50/EUR"),
    ("jp_p", "JP:P/M", "nkd_1h_databento.json", "层1 JP 生产边 日经/JPY"),
    ("cn_p", "CN:P/M*", "if_1m_sina.json", "层1 CN 生产边 沪深300/CNY(探索)"),
    ("cn_c", "CN:C/M*", "sc_1m_sina.json", "层1 CN 商品边 SC原油/CNY(探索)"),
    ("cn_au", "CN:Au*", "au_1m_sina.json", "CN 黄金 AU9999/CNY(探索)"),
    ("us_r", "US:R/M", "vnq_1m_tws.json", "层1 US 不动产边 VNQ/USD"),
    ("us_c", "US:C/M", "dbc_1m_tws.json", "层1 US 商品边 DBC/USD"),
    ("oil", "Oil:C->P", "cl_1m_databento.json", "C路径 折叠通道 原油 CL(全局)"),
    ("au", "Au:C<->M", "gc_1m_databento.json", "C路径 折叠通道 黄金 GC(全局)"),
    ("us_p", "US:P/M", "es_1m_databento.json", "层1 US 生产边 ES/USD"),
]


def main() -> None:
    t0 = time.time()
    edges: dict[str, dict] = {}
    for key, label, fname, _role in EDGES:
        edges[key] = _run_edge(label, fname)
        # 每条边后增量落盘（防大边中断丢失已算结果）
        OUT.write_text(json.dumps({"edges": edges, "partial": True}, ensure_ascii=False, indent=2))

    # ── 层 1：US 完整 Configuration（P/C/R 三独立边齐） ──
    us_sig = {
        Vertex.P: P.WalkDirection[edges["us_p"]["sigma"]],
        Vertex.C: P.WalkDirection[edges["us_c"]["sigma"]],
        Vertex.R: P.WalkDirection[edges["us_r"]["sigma"]],
    }
    us_config = P.Configuration(
        sigma_p=us_sig[Vertex.P], sigma_c=us_sig[Vertex.C], sigma_r=us_sig[Vertex.R]
    )

    # ── 层 0：结算尺空间同步断裂候选（254号定理2） ──
    fx_keys = ["fx_eu", "fx_jp", "fx_cn"]
    trending = [
        edges[k] for k in fx_keys
        if edges[k]["move_kind"] != "consolidation" and edges[k]["move_settled"]
    ]
    sync_dirs = {e["move_direction"] for e in trending}
    sync_break = len(trending) >= 2 and len(sync_dirs) == 1

    # ── C 路径：ω = 金/油，折叠通道方向（482号） ──
    au, oil = edges["au"], edges["oil"]
    omega = au["last_price"] / oil["last_price"] if oil["last_price"] > 0 else None
    au_v = P.WalkDirection[au["sigma"]].value
    oil_v = P.WalkDirection[oil["sigma"]].value
    omega_dir = "up" if au_v > oil_v else ("down" if au_v < oil_v else "flat")

    summary = {
        "engine": "newchan_rust.RecursiveOrchestrator (max_levels=6, stroke_mode=wide)",
        "epistemic_level": "L2/L3 (真实多标的多分辨率)",
        "total_seconds": round(time.time() - t0, 1),
        "us_configuration": {
            "sigma_p": us_config.sigma_p.name,
            "sigma_c": us_config.sigma_c.name,
            "sigma_r": us_config.sigma_r.name,
            "complete": True,
        },
        "layer0_settlement": {
            "fx_edges": {k: edges[k]["move_direction"] for k in fx_keys},
            "fx_settled": {k: edges[k]["move_settled"] for k in fx_keys},
            "synchronized_break_candidate": sync_break,
        },
        "c_path": {
            "au_last": au["last_price"], "oil_last": oil["last_price"],
            "omega_gold_oil": omega, "omega_direction": omega_dir,
            "au_sigma": au["sigma"], "oil_sigma": oil["sigma"],
        },
        "partial_economies": {
            "EU": {"P": edges["eu_p"]["sigma"], "C": None, "R": None},
            "JP": {"P": edges["jp_p"]["sigma"], "C": None, "R": None},
            "CN_explore": {
                "P": edges["cn_p"]["sigma"], "C": edges["cn_c"]["sigma"],
                "Au": edges["cn_au"]["sigma"],
                "note": "窗口仅 ~1 周（~1k bar），不足涌现最高级别，L0/L1 探索读数",
            },
        },
    }
    OUT.write_text(json.dumps(
        {"edges": edges, "summary": summary, "partial": False},
        ensure_ascii=False, indent=2,
    ))
    print(f"\nDONE total={summary['total_seconds']}s -> {OUT}", flush=True)
    print("US Γ:", summary["us_configuration"], flush=True)
    print("层0 sync_break:", sync_break, "dirs:", summary["layer0_settlement"]["fx_edges"], flush=True)
    print("ω:", omega, omega_dir, flush=True)


if __name__ == "__main__":
    main()
