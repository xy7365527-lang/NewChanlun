#!/usr/bin/env python3
"""#1406 C8：ADR 0040 裁定四「确定性推导」的独立 oracle（只用标准库与整数，不 import 产品代码）。

输入逐字取自 ADR 0040「签署记录」：每个 profile 的 Q_total、l_start、C(ℓ)、F_min、margin_leverage，
以及频率证据（#915 报告 :153「BTC L0–L3 ROUNDTRIP n 均 > 0」，此处只记为 买/卖 计数 ≥ 1 的见证，
不是读数本身）。输出每个 profile 的重清单 ⟨S, ℓ, Q⟩、开局批 ⌈Q/L⌉、剩余与停止原因，
以及 L1 重的 w(1) 档量 ⌊Q·num/den⌋（ADR 0034 裁定四取整）。

Rust 侧 `tb03c_funding_tests` 以 include_str! 读 FUNDING-ORACLE.json，与夹具黄金值逐项对拍。

用法：python3 funding_oracle.py          重写 FUNDING-ORACLE.json
      python3 funding_oracle.py --check  只核对磁盘字节与重新推导一致
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "FUNDING-ORACLE.json"

UNKNOWN = None
# ADR 0040 裁定三表格 / policy-record capacities（UserSigned #917）。
CAPACITY = {0: 1110000, 1: 3200000, 2: 9630000, 3: UNKNOWN}
# #915 报告 :153：BTC L0–L3 往返 n 均 > 0 —— 只作 F_min 判据的见证。
FREQUENCY = {lvl: {"buy": 1, "sell": 1} for lvl in range(4)}
# ADR 0040 签署记录 funding-record.cap（= Q_total）与 derivation-input。
PROFILES = [
    {"profile_id": "P0", "q_total": 100000, "l_start": 0, "f_min": 1, "margin_leverage": 1},
    {"profile_id": "P1-hl-testnet", "q_total": 500, "l_start": 0, "f_min": 1, "margin_leverage": 1},
    {"profile_id": "P0-acceptance-5m", "q_total": 5000000, "l_start": 0, "f_min": 1, "margin_leverage": 1},
]
SYMBOL = "BTC"
W1 = (347, 1000)  # ADR 0040 裁定三 w(1)，只用于操作级别 L1 的重


def ceil_div(a, b):
    assert a >= 0 and b > 0
    return -(-a // b)


def derive(p):
    level, remaining = p["l_start"], p["q_total"]
    chongs, stop = [], None
    while True:
        if level not in CAPACITY:
            stop = {"kind": "AwaitingPolicy", "slot": "capacity", "level": f"L{level}"}
            break
        freq = FREQUENCY.get(level)
        if p["f_min"] is None or freq is None or CAPACITY[level] is UNKNOWN:
            slot = "capacity" if CAPACITY[level] is UNKNOWN else "frequency"
            stop = {"kind": "AwaitingPolicy", "slot": slot, "level": f"L{level}"}
            break
        if freq["buy"] < p["f_min"] or freq["sell"] < p["f_min"]:
            level += 1  # NotViable：不开重，看上一级
            continue
        q = min(remaining, CAPACITY[level])
        chong = {"symbol": SYMBOL, "level": f"L{level}", "Q": q, "C": CAPACITY[level],
                 "p0_batch": ceil_div(q, p["margin_leverage"])}
        if level == 1:
            chong["tier_quantity_d1"] = q * W1[0] // W1[1]
        chongs.append(chong)
        remaining -= q
        if remaining == 0:
            stop = {"kind": "完成"}
            break
        level += 1
    return {"profile_id": p["profile_id"], "q_total": p["q_total"], "margin_leverage": p["margin_leverage"],
            "chongs": chongs, "remaining": remaining, "stop": stop}


def render():
    doc = {"source": "ADR 0040 裁定四；签署记录 2026-10-20；独立 oracle（不 import 产品代码）",
           "profiles": [derive(p) for p in PROFILES]}
    return (json.dumps(doc, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()


def main():
    data = render()
    if "--check" in sys.argv[1:]:
        if OUT.read_bytes() != data:
            print("FUNDING-ORACLE.json 与重新推导不一致", file=sys.stderr)
            return 1
        print("ok")
        return 0
    OUT.write_bytes(data)
    return 0


if __name__ == "__main__":
    sys.exit(main())
