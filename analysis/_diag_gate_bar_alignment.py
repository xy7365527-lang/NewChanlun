"""门列 ↔ 驱动 closes 的 bar 对齐诊断（#1314：DX 6 根错位定位件）。

## 为什么需要这个诊断

第二轮状态门控回测（`m1_e_futures_dual_gated_backtest.py`）把 v3 门状态列按 **bar 下标**
join 到驱动侧 closes 上。两侧 bar 序列来自同一份 1min json，但**清洗口径不同**：

| 侧 | 入口 | 清洗 | 遍历 |
|----|------|------|------|
| 驱动 | `analysis/m1_e_futures_backtest.py:86-93` `load_ohlc` | 只剔任一 OHLC 为 **nan** 的整根 bar | `zip(o,h,l,c)`（**截到最短数组**） |
| dump（门列） | `analysis/_dump_tape_rust.py:69-86` `load_ohlc_with_ts` | 剔 nan **另剔任一 OHLC ≤0** 的 bar | `range(len(closes))` |

⇒ **同一份文件内容下，dump 侧的剔除集是驱动侧的超集，门列 bar 数不可能多于驱动**。
#1314 实测 DX 门列 n=2,058,424 > 驱动 closes=2,058,418（多 6 根），故该多出 6 根**不可能**
由清洗口径差解释——只能是两侧读到的不是同一份文件内容（磁带/门列的 vintage 与当前 json
不同），或 json 各数组长度不齐（驱动侧 `zip` 截尾、dump 侧不截）。本脚本把这两类都验掉：
逐 bar 时间戳比对，给出多出的 bar 落在**头部 / 尾部 / 内部**的定论与具体时间戳。

## 口径复刻声明（防判据分叉）

本脚本**逐字复刻**上表两个 loader 的清洗分支（源行号见上），复刻是诊断必需——诊断的对象
就是"两个口径的差"，无法靠调用其中之一得到。这里复刻的是**数据清洗**（nan/≤0 剔除），
不是缠论判据；判据一律在 Rust 生产函数侧（`gate_state_dump.rs`），本件零判据。
两个 loader 改动时须同步本文件（漂移即诊断失真）。

## 用法

```bash
PYTHONPATH=src:analysis uv run python analysis/_diag_gate_bar_alignment.py DX
PYTHONPATH=src:analysis uv run python analysis/_diag_gate_bar_alignment.py ES GC CL ZN 6E BRN DX
```

退出码：0 = 全部诊断完成（含"已对齐"与"错位已定位"）；2 = 输入不齐（缺 json / 缺门列 /
缺 ts 边车），按卡点口径打印解除条件。

认识论等级：读数搬运 L0（只比对已落盘序列，不产生任何判据读数）。
"""

from __future__ import annotations

import json
import math
import struct
import sys
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "analysis"))

import gate_state_columns as G  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"

# 标的 → 1min 数据文件（与 m1_e_futures_dual_gated_backtest.SYMBOL_FILES 逐项一致；
# 本地镜像而非 import：那些 driver 模块顶部 import newchan_rust，沙盒未构建时会在
# import 阶段先崩，使本诊断在"最需要它"的缺件环境下不可达）。
SYMBOL_FILES = {
    "ES": DATA_DIR / "es_1m_databento_10y.json",
    "GC": DATA_DIR / "gc_1m_databento_10y.json",
    "CL": DATA_DIR / "cl_1m_databento_10y.json",
    "ZN": DATA_DIR / "zn_1m_databento_10y.json",
    "BRN": DATA_DIR / "brn_1m_databento_10y.json",
    "6E": DATA_DIR / "usd6e_1m_databento_10y.json",
    "DX": DATA_DIR / "dx_1m_databento_10y.json",
}


def ts_sidecar_path(sym: str) -> Path:
    """dump 侧逐 bar 时间戳边车（`_dump_tape_rust.py` 落盘，与磁带/门列同长同序）。"""
    return DATA_DIR / f"_tape_v2r_{sym}_ts.i64"


def extract_arrays(raw: dict):
    """两种 schema 的原始列提取。

    bars-schema 分支复刻 `_dump_tape_rust.load_ohlc_with_ts`（dump 侧）。**驱动侧的
    `m1_e_futures_backtest.load_ohlc` 没有该分支**（只读 opens/highs/lows/closes，
    bars-schema 上会 KeyError）——7 个期货标的都是 parallel-array，故两侧在本票范围内
    读的是同一分支；若日后有标的换成 bars-schema，`driver_kept` 的复刻即失真，须同步。
    """
    if "bars" in raw:
        bars = raw["bars"]
        return (
            [b["open"] for b in bars],
            [b["high"] for b in bars],
            [b["low"] for b in bars],
            [b["close"] for b in bars],
            [b["ts"] for b in bars],
        )
    return (
        raw["opens"], raw["highs"], raw["lows"], raw["closes"], raw.get("dates"),
    )


def to_epoch(s) -> int:
    """`_dump_tape_rust.load_ohlc_with_ts` 的时间戳口径（naive；带 tz 的取 UTC naive）。"""
    dt = datetime.fromisoformat(str(s))
    if dt.tzinfo is not None:
        dt = dt.replace(tzinfo=None)
    return int(dt.timestamp())


def driver_kept(o_in, h_in, l_in, c_in) -> list[int]:
    """驱动口径保留的原始下标（`m1_e_futures_backtest.load_ohlc`：zip + 只剔 nan）。"""
    kept: list[int] = []
    for idx, (o, h, l, c) in enumerate(zip(o_in, h_in, l_in, c_in)):
        o, h, l, c = float(o), float(h), float(l), float(c)
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        kept.append(idx)
    return kept


def dump_kept(o_in, h_in, l_in, c_in) -> list[int]:
    """dump 口径保留的原始下标（`_dump_tape_rust.load_ohlc_with_ts`：range + nan + ≤0）。"""
    kept: list[int] = []
    for idx in range(len(c_in)):
        o, h, l, c = float(o_in[idx]), float(h_in[idx]), float(l_in[idx]), float(c_in[idx])
        if math.isnan(o) or math.isnan(h) or math.isnan(l) or math.isnan(c):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        kept.append(idx)
    return kept


def read_gate_header(path: Path) -> dict:
    """v3 门列头（只读头部字节，2M bar 的 6MB 列体不进内存）。"""
    with path.open("rb") as f:
        raw = f.read(G.HEADER_SIZE)
    if len(raw) < G.HEADER_SIZE:
        raise ValueError(f"门列文件过短（{len(raw)}B）——落盘不完整：{path}")
    magic, n, ladder, fat_avail, first_c, last_c = struct.unpack(G.HEADER_FMT, raw)
    if magic != G.MAGIC_V3G:
        raise ValueError(f"门列 magic 不匹配——dump 格式代次错位：{path}")
    return {
        "n": n, "ladder": ladder, "fatigue_available": bool(fat_avail),
        "first_close": first_c, "last_close": last_c,
    }


def read_ts_sidecar(path: Path) -> list[int]:
    """dump 侧逐 bar epoch 秒（小端 i64 数组）。"""
    blob = path.read_bytes()
    if len(blob) % 8:
        raise ValueError(f"ts 边车长度 {len(blob)}B 非 8 的倍数——落盘不完整：{path}")
    return list(struct.unpack(f"<{len(blob) // 8}q", blob))


def align_report(gate_ts: list[int], driver_ts: list[int]) -> dict:
    """逐 bar 时间戳比对：多出的 bar 落在头部/尾部/内部的定论。

    **前提：`gate_ts` 内 ts 两两不同。**两侧都只是按 json 的文件顺序遍历（
    `_dump_tape_rust.load_ohlc_with_ts` 既不排序也不去重），故这条前提由数据源保证、
    不由代码保证。前提成立时"驱动是门列的子序列"等价于"驱动的 ts 集合 ⊆ 门列的 ts
    集合"，位置由 `gate_pos` 的下标给出；不成立时下标分桶会把重复 ts 归错桶，故
    `dup_ts_gate` / `dup_ts_driver` 一并报出（非零 ⇒ verdict 不可信，见 format_report）。

    verdict：
      - `EQUAL`            逐位相同，无需对齐；
      - `HEAD_EXCESS`      多出的全在头部 ⇒ 截头对齐（`_align_start` 尾锚臂）；
      - `TAIL_EXCESS`      多出的全在尾部 ⇒ 截尾对齐（`_align_start` 首锚臂）；
      - `INTERIOR_EXCESS`  多出的全在内部 ⇒ **下标 join 不可救**，须修 dump 口径重落；
      - `MIXED_EXCESS`     头/尾/内部混合 ⇒ 同上，须重落；
      - `DRIVER_SUPERSET`  门列反而少 bar ⇒ 门列过期，重跑 dump；
      - `NOT_NESTED`       互不包含 ⇒ 两侧不同源/不同 vintage，重跑 dump。
    """
    gate_pos = {t: i for i, t in enumerate(gate_ts)}
    out: dict = {
        "n_gate": len(gate_ts),
        "n_driver": len(driver_ts),
        "delta": len(gate_ts) - len(driver_ts),
        "extra_head": [], "extra_interior": [], "extra_tail": [],
        "driver_only": [],
        # 重复 ts 计数：非零即上面 docstring 的前提被破坏，分桶读数不可信。
        "dup_ts_gate": len(gate_ts) - len(set(gate_ts)),
        "dup_ts_driver": len(driver_ts) - len(set(driver_ts)),
    }
    if gate_ts == driver_ts:
        out["verdict"] = "EQUAL"
        return out
    missing = [t for t in driver_ts if t not in gate_pos]
    out["driver_only"] = missing
    if missing:
        driver_set = set(driver_ts)
        gate_only = [t for t in gate_ts if t not in driver_set]
        out["verdict"] = "DRIVER_SUPERSET" if not gate_only else "NOT_NESTED"
        out["gate_only"] = gate_only
        out["n_common"] = len(set(gate_ts) & driver_set)
        return out
    # 驱动 ⊆ 门列：多出的 bar 按位置分桶。
    lo = gate_pos[driver_ts[0]]
    hi = gate_pos[driver_ts[-1]]
    kept = set(gate_pos[t] for t in driver_ts)
    for i, t in enumerate(gate_ts):
        if i in kept:
            continue
        bucket = "extra_head" if i < lo else ("extra_tail" if i > hi else "extra_interior")
        out[bucket].append({"gate_idx": i, "ts": t, "local": _fmt_ts(t)})
    has = {k: bool(out[k]) for k in ("extra_head", "extra_interior", "extra_tail")}
    if has["extra_interior"] or sum(has.values()) > 1:
        out["verdict"] = "MIXED_EXCESS" if sum(has.values()) > 1 else "INTERIOR_EXCESS"
    elif has["extra_head"]:
        out["verdict"] = "HEAD_EXCESS"
    elif has["extra_tail"]:
        out["verdict"] = "TAIL_EXCESS"
    else:  # 无多出但序列不等 ⇒ 驱动内部顺序与门列不同（同源不该出现）
        out["verdict"] = "NOT_NESTED"
    return out


def _fmt_ts(t: int) -> str:
    """epoch → **本地时刻**（非 UTC）。

    `to_epoch` 把 json 的 naive 时刻按本地时区取 epoch，`fromtimestamp` 是它的逆——
    打出来的即数据文件里那一行的原始时刻字符串，正好用来对照数据源核对。
    """
    return datetime.fromtimestamp(t).isoformat()


def diagnose(sym: str) -> dict:
    """单标的完整对齐诊断（缺件时抛 FileNotFoundError，由 main 归为卡点）。"""
    path = SYMBOL_FILES[sym]
    if not path.exists():
        raise FileNotFoundError(f"[{sym}] 缺 1min 数据文件 {path}")
    gpath = G.gate_path(sym)
    if not gpath.exists():
        raise FileNotFoundError(f"[{sym}] 缺 v3 门列 {gpath}")
    raw = json.loads(path.read_text())
    o_in, h_in, l_in, c_in, d_in = extract_arrays(raw)
    lens = {
        "opens": len(o_in), "highs": len(h_in), "lows": len(l_in),
        "closes": len(c_in), "dates": (len(d_in) if d_in is not None else None),
    }
    d_kept = driver_kept(o_in, h_in, l_in, c_in)
    # dump 口径按 `len(closes)` 遍历、不截尾 ⇒ closes 比任一其他 OHLC 列长时，真 dump
    # 会在此越界崩。这正是本诊断要区分的两个假设之一（json 数组不齐），故这里**不复现
    # 那次崩溃**（崩了就出不了报告），而是把它当作一条读数报出：dump_n=None + 标志位。
    ohlc_ragged = lens["closes"] > min(
        lens["opens"], lens["highs"], lens["lows"], lens["closes"]
    )
    p_kept = None if ohlc_ragged else dump_kept(o_in, h_in, l_in, c_in)
    header = read_gate_header(gpath)
    out: dict = {
        "symbol": sym,
        "json": str(path),
        "raw_array_lens": lens,
        "ohlc_arrays_ragged": ohlc_ragged,
        "driver_n": len(d_kept),
        "dump_n": None if p_kept is None else len(p_kept),
        "driver_minus_dump": None if p_kept is None else len(d_kept) - len(p_kept),
        "dropped_by_zip_truncation": max(lens["closes"] - min(
            lens["opens"], lens["highs"], lens["lows"], lens["closes"]), 0),
        "dropped_by_nonpositive_only": (
            None if p_kept is None else len(set(d_kept) - set(p_kept))
        ),
        "gate_header": header,
        "gate_n_minus_driver_n": header["n"] - len(d_kept),
        "driver_first_close": float(c_in[d_kept[0]]) if d_kept else None,
        "driver_last_close": float(c_in[d_kept[-1]]) if d_kept else None,
    }
    out["first_close_anchors"] = (
        out["driver_first_close"] == header["first_close"]
    )
    out["last_close_anchors"] = (
        out["driver_last_close"] == header["last_close"]
    )
    # 逐 bar 时间戳比对（有 ts 边车 + json 有 dates 时才可做；否则只出 close 锚定读数）。
    tpath = ts_sidecar_path(sym)
    if d_in is None:
        out["align"] = {"verdict": "NO_DATES", "note": "json 无 dates/ts 列，无法逐 bar 比对"}
    elif not tpath.exists():
        out["align"] = {
            "verdict": "NO_TS_SIDECAR",
            "note": f"缺 {tpath}（`_dump_tape_rust.py` 与磁带同时落盘）——重跑 dump 即有",
        }
    elif d_kept and len(d_in) <= d_kept[-1]:
        # 驱动侧 zip 只截到最短的 OHLC 列，不看 dates；dates 更短时驱动仍会保留取不到
        # 时刻的 bar ⇒ 逐 bar 比对做不了（这本身也是"数组不齐"的一条实证）。
        out["align"] = {
            "verdict": "DATES_TOO_SHORT",
            "note": (
                f"dates 只有 {len(d_in)} 项，驱动保留的最大下标是 {d_kept[-1]}"
                "（驱动侧 zip 不看 dates）——无法逐 bar 比对，先修数据文件的数组不齐"
            ),
        }
    else:
        gate_ts = read_ts_sidecar(tpath)
        driver_ts = [to_epoch(d_in[i]) for i in d_kept]
        rep = align_report(gate_ts, driver_ts)
        rep["ts_sidecar_n_matches_gate_n"] = len(gate_ts) == header["n"]
        out["align"] = rep
    return out


def format_report(d: dict) -> str:
    L = [f"== [{d['symbol']}] 门列 ↔ 驱动 bar 对齐诊断 =="]
    L.append(f"  json：{d['json']}")
    L.append(f"  原始数组长度：{d['raw_array_lens']}")
    if d["ohlc_arrays_ragged"]:
        L.append(
            f"  驱动口径（仅剔 nan，zip 截最短）保留 {d['driver_n']:,} 根；"
            f"dump 口径**算不出**：closes 比最短的 OHLC 列长 "
            f"{d['dropped_by_zip_truncation']:,} 项，dump 按 len(closes) 遍历会越界崩"
            "——⇒ 当前 json 上 `_dump_tape_rust.py` 根本跑不完，已落盘的磁带/门列"
            "必然出自另一份 json（vintage 不同），须先修数据文件再重跑 dump。"
        )
    else:
        L.append(
            f"  驱动口径（仅剔 nan，zip 截最短）保留 {d['driver_n']:,} 根；"
            f"dump 口径（另剔 ≤0）保留 {d['dump_n']:,} 根；"
            f"差 {d['driver_minus_dump']:+,}（其中仅因 ≤0 被 dump 多剔 "
            f"{d['dropped_by_nonpositive_only']:,} 根；"
            f"zip 截尾 {d['dropped_by_zip_truncation']:,} 根）"
        )
    h = d["gate_header"]
    L.append(
        f"  门列头：n={h['n']:,} ladder={h['ladder']} "
        f"first_close={h['first_close']} last_close={h['last_close']} "
        f"fatigue_available={h['fatigue_available']}"
    )
    L.append(
        f"  门列 n − 驱动 n = {d['gate_n_minus_driver_n']:+,}；"
        f"首 close 锚定={d['first_close_anchors']}；末 close 锚定={d['last_close_anchors']}"
    )
    if (d["gate_n_minus_driver_n"] > 0 and d["dump_n"] is not None
            and d["dump_n"] <= d["driver_n"]):
        L.append(
            "  ⚠ 门列比驱动多 bar，但同一份 json 上 dump 口径的保留集是驱动的子集"
            "（dump 另剔 ≤0）——⇒ 门列/磁带与当前 json **不是同一份内容**"
            "（vintage 不同），须重跑 dump 落新磁带与门列。"
        )
    a = d["align"]
    L.append(f"  逐 bar 时间戳比对：verdict={a['verdict']}")
    if a["verdict"] in ("NO_DATES", "NO_TS_SIDECAR", "DATES_TOO_SHORT"):
        L.append(f"    {a['note']}")
        return "\n".join(L)
    L.append(
        f"    门列 {a['n_gate']:,} 根 vs 驱动 {a['n_driver']:,} 根（差 {a['delta']:+,}）；"
        f"ts 边车与门列同长={a.get('ts_sidecar_n_matches_gate_n')}"
    )
    if a["dup_ts_gate"] or a["dup_ts_driver"]:
        L.append(
            f"    ⚠ 时间戳有重复（门列 {a['dup_ts_gate']} 个 / 驱动 {a['dup_ts_driver']} 个）"
            "——下标分桶以 ts 唯一为前提，本行非零时上/下面的 verdict 与落点读数不可信，"
            "须先查数据源的重复 bar。"
        )
    for key, name in (("extra_head", "头部"), ("extra_interior", "内部"),
                      ("extra_tail", "尾部")):
        rows = a.get(key) or []
        if rows:
            L.append(f"    门列多出（{name}）{len(rows)} 根：")
            for r in rows[:20]:
                L.append(f"      gate_idx={r['gate_idx']:,} ts={r['ts']} (本地 {r['local']})")
            if len(rows) > 20:
                L.append(f"      …另 {len(rows) - 20} 根")
    if a.get("driver_only"):
        L.append(f"    驱动独有（门列缺）{len(a['driver_only'])} 根，前 5（本地时刻）："
                 f"{[_fmt_ts(t) for t in a['driver_only'][:5]]}")
    L.append("  → 处置：" + {
        "EQUAL": "已对齐，无需处置。",
        "HEAD_EXCESS": "截头对齐（`gate_state_columns._align_start` 尾锚臂自动处理）。",
        "TAIL_EXCESS": "截尾对齐（`gate_state_columns._align_start` 首锚臂自动处理）。",
        "INTERIOR_EXCESS": "下标 join 不可救——修 dump 口径后重落磁带与门列。",
        "MIXED_EXCESS": "下标 join 不可救——修 dump 口径后重落磁带与门列。",
        "DRIVER_SUPERSET": "门列过期，重跑 dump。",
        "NOT_NESTED": "两侧不同源/不同 vintage，重跑 dump。",
    }.get(a["verdict"], "未知 verdict。"))
    return "\n".join(L)


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    syms = [s.upper() for s in argv] if argv else list(SYMBOL_FILES)
    unknown = [s for s in syms if s not in SYMBOL_FILES]
    if unknown:
        print(f"未知标的 {unknown}；可选：{list(SYMBOL_FILES)}")
        return 2
    blocked: list[str] = []
    for sym in syms:
        try:
            print(format_report(diagnose(sym)), flush=True)
        except FileNotFoundError as e:
            blocked.append(str(e))
    if blocked:
        print("\n卡点：对齐诊断的输入不齐（外部依赖）。", flush=True)
        for b in blocked:
            print(f"  - {b}", flush=True)
        print(
            "解除条件：①1min 数据文件就位（Databento key 或编排层投放，gitignored）；"
            "②构建 Rust 扩展 `cd rust && uv run maturin develop --release`；"
            f"③落磁带与门列：{G.REGEN_CMD.replace('<SYM>', ' '.join(syms))}",
            flush=True,
        )
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
