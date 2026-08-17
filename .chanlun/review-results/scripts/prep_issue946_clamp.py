#!/usr/bin/env python3
"""#946（任务 #1024）：把重新采集的原始 K 线整成与 #944 **同窗口**的 A/C 结构输入。

为什么不直接用 `prep_issue940.py`：它的窗口 = [max(A 首根, B 首根), min(A 末根, B 末根)]，
今天重跑会把窗口右端推到采集当天，与 #944（2026-08-08 采集，窗口止于 2026-08-07）
不可比。本脚本把逐标的窗口钳到 #944 metrics（`issue944-bsp-metrics-20260808.json`）
里记录的 [window_start_utc, window_end_utc]，其余过滤规则**逐字复用** `prep_issue940.py`
（C 臂 = B 按 Yahoo `meta.tradingPeriods` 的 regular session 相交判定过滤）。

前提：`fetch_issue940.py` 已跑（`data/y940_<SYM>.json` / `data/hl940_<SYM>.json` 在位）。

产出：`../data/issue946_inputs.json`（schema 与 `issue940_inputs.json` 一致；
B 臂保留但本票测量面只用 A/C 两臂）。

用法：`python3 prep_issue946_clamp.py`
"""
import json
import os

import prep_issue940 as P

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.normpath(os.path.join(HERE, "..", "data"))
RESULTS = os.path.normpath(os.path.join(HERE, ".."))

# #944 metrics 里的逐标的窗口（mode=new 记录；两档同窗口）。
M944 = os.path.join(RESULTS, "issue944-bsp-metrics-20260808.json")


def main():
    m944 = json.load(open(M944))
    wins = {r["symbol"]: (r["window"][0], r["window"][1], r["n_sessions"])
            for r in m944["records"] if r["stroke_mode"] == "new"}
    out = {"note": "issue #946 seg-structure-alignment arms; A=Yahoo cash 1h, "
                   "B=Hyperliquid xyz perp 1h 24/7, C=B filtered to US regular session; "
                   "windows clamped to #944 metrics (issue944-bsp-metrics-20260808.json)",
           "symbols": []}
    for sym in P.SYMS:
        lo, hi, ns944 = wins[sym]
        a_all, sessions = P.load_yahoo(sym)
        b_all = P.load_hl(sym)
        A = [r for r in a_all if lo <= r[0] <= hi]
        B = [r for r in b_all if lo <= r[0] <= hi]
        C = [r for r in B if P.in_session(r[0], sessions)]
        assert A and B and C, f"{sym}: empty arm after clamp"
        assert A[-1][0] <= hi and B[-1][0] <= hi, f"{sym}: clamp failed"
        # 逐字节等价性的终验 = 用 #944 探针复跑本产物并对拍 arm 统计/端点层读数
        # （见报告 issue946-seg-structure-alignment-20260817.md「与 #944 的对拍」节）。
        ns = sum(1 for s, e in sessions if lo <= s <= hi)
        bcols = P.cols(B)
        bcols["cls"] = [P.classify(r[0], sessions) for r in B]
        acols = P.cols(A)
        acols["cls"] = ["session"] * len(A)
        ccols = P.cols(C)
        ccols["cls"] = ["session"] * len(C)
        out["symbols"].append({
            "symbol": sym,
            "window_start_utc": lo, "window_end_utc": hi,
            "n_sessions_in_window": ns,
            "A": acols, "B": bcols, "C": ccols,
        })
        flag = "" if ns == ns944 else f"  ⚠ n_sessions {ns} != #944 {ns944}"
        print(f"{sym}: window {lo}..{hi}  A={len(A)} B={len(B)} C={len(C)} "
              f"sessions={ns}{flag}")
    p = os.path.join(DATA, "issue946_inputs.json")
    with open(p, "w") as f:
        json.dump(out, f)
    print("wrote", p, os.path.getsize(p) // 1024, "KiB")


if __name__ == "__main__":
    main()
