"""#1424 TB-02-F 独立 oracle：相邻中枢五叶关系（CC-020）与延伸测试单元位置（CC-018）的手算工作例。

纯标准库，不导入项目代码，不读生产输出作期待值。判据只取正本与已冻结裁定：
- seed（zhongshu.md §1.6）：三单元方向交替 ∧ ZD=max(三低) < ZG=min(三高)（Z-2 严格）。
- 延伸（§4.1，020:56）：[d,g] ∩ [ZD,ZG] ≠ ∅（闭区间），核心冻结；全单元测试（#1478 裁定二端点条款）。
- 续扫（§6.1 Z-5，#1419 R8）：结算后自 last+2；不成立 +1。
- 升级（§4.4 S-4）：成员 ≥9 ⟹ S 标 UPGRADE_PENDING_TB02H、动态四变量 null（#1419 R3）；
  塔侧按每 3 段重切、子中枢继承母核心、子窗全单元外缘（§9.5 过渡态，R22 未改）。
- 外缘（§3 S-7，020:52，#1478/#1419 D-A）：DD/GG/D/G 只聚合与 seed 首单元同向的成员（Zn）。
- 五叶（SPEC #1340 CC-020，020:58）：CORE_NOT_STRICTLY_SEPARATED ⟺ ¬(B.ZG<A.ZD ∨ B.ZD>A.ZG)；
  UP_NEWBIRTH ⟺ B.DD>A.GG；DOWN_NEWBIRTH ⟺ B.GG<A.DD；UP_EXPANSION ⟺ B.ZD>A.ZG ∧ B.DD≤A.GG；
  DOWN_EXPANSION ⟺ B.ZG<A.ZD ∧ B.GG≥A.DD。逐叶独立求值，断言恰一。
- 单元位置（CC-018）：STRICT_BELOW u.hi<ZD；LOW_TOUCH u.hi=ZD；INTERIOR_CONTACT u.hi>ZD ∧ u.lo<ZG；
  HIGH_TOUCH u.lo=ZG；STRICT_ABOVE u.lo>ZG。
- 一端为 ≥9 窗口的对：PENDING_UPGRADE_TB02H（#1424 根裁定第 2 问）。

用法：python3 oracle_pairs.py > ../../../rust/tests/fixtures/center_pair_1424_worked_examples.json
来源：备料 /private/tmp/goal1323-native-20261012/tb1424-prep-r2/handcalc.py（同一算法；工作例编号按根派发改为 W0–W12）。"""
import json
import sys

UP, DOWN = "UP", "DOWN"
UPGRADE = 9
ENDPOINTS = ("DD", "ZD", "ZG", "GG")
PROJ = {"CORE_NOT_STRICTLY_SEPARATED": "CoreOverlap", "UP_NEWBIRTH": "UpContinuation",
        "DOWN_NEWBIRTH": "DownContinuation", "UP_EXPANSION": "LevelExpansion", "DOWN_EXPANSION": "LevelExpansion"}


def units_from_endpoints(p):
    return [{"k": k, "dir": UP if b > a else DOWN, "lo": min(a, b), "hi": max(a, b)}
            for k, (a, b) in enumerate(zip(p, p[1:]))]


def seed(a, b, c):
    zd, zg = max(a["lo"], b["lo"], c["lo"]), min(a["hi"], b["hi"], c["hi"])
    if not (a["dir"] != b["dir"] and b["dir"] != c["dir"]):
        return zd, zg, "NOT_ALTERNATING"
    if zd == zg:
        return zd, zg, "TANGENT_ZD_EQ_ZG"
    if zd > zg:
        return zd, zg, "INVERTED_ZD_GT_ZG"
    return zd, zg, None


def position(u, zd, zg):
    if u["hi"] < zd:
        return "STRICT_BELOW"
    if u["hi"] == zd:
        return "LOW_TOUCH"
    if u["lo"] > zg:
        return "STRICT_ABOVE"
    if u["lo"] == zg:
        return "HIGH_TOUCH"
    return "INTERIOR_CONTACT"


def scan(units):
    attempts, windows = [], []
    i, n = 0, len(units)
    while i + 2 < n:
        zd, zg, reason = seed(units[i], units[i + 1], units[i + 2])
        attempts.append({"at": i, "result": "FORMED" if reason is None else "EXCLUDED", "exclusion_reason": reason})
        if reason is not None:
            i += 1
            continue
        j, tested = i + 3, []
        while j < n:
            pos = position(units[j], zd, zg)
            tested.append((j, pos))
            if pos in ("STRICT_BELOW", "STRICT_ABOVE"):
                break
            j += 1
        windows.append({"seed_at": i, "exit": j, "zd": zd, "zg": zg, "upgraded": (j - i) >= UPGRADE, "tested": tested})
        i = j + 1 if j < n else j
    return attempts, windows


def zn_roles(members):
    zn = [u for u in members if u["dir"] == members[0]["dir"]]
    return {"DD": min(u["lo"] for u in zn), "GG": max(u["hi"] for u in zn),
            "D": max(u["lo"] for u in zn), "G": min(u["hi"] for u in zn), "zn_members": [u["k"] for u in zn]}


def centers_of(units, windows):
    out = []
    for w in windows:
        members = units[w["seed_at"]:w["exit"]]
        settled = w["exit"] < len(units)
        r = zn_roles(members)
        c = {"first": w["seed_at"], "last": w["exit"] - 1, "settled": settled,
             "break_unit": w["exit"] if settled else None,
             "status": "UPGRADE_PENDING_TB02H" if w["upgraded"] else ("SETTLED" if settled else "EXTENDING"),
             "ZD": w["zd"], "ZG": w["zg"], "zn_members": r["zn_members"],
             "DD_full": min(u["lo"] for u in members), "GG_full": max(u["hi"] for u in members)}
        for key in ("DD", "GG", "D", "G"):
            c[key] = None if w["upgraded"] else r[key]
        c["tested"] = [{"unit": k, "position": p,
                        "role": "BREAK_UNIT" if settled and k == w["exit"]
                        else ("UPGRADE_PENDING_MEMBER" if w["upgraded"] else "EXTENSION_MEMBER")}
                       for k, p in w["tested"]]
        out.append(c)
    return out


def cmp(x, y):
    return "<" if x < y else ("=" if x == y else ">")


def leaves_matching(A, B):
    sep = B["ZG"] < A["ZD"] or B["ZD"] > A["ZG"]
    preds = {
        "CORE_NOT_STRICTLY_SEPARATED": not sep,
        "UP_NEWBIRTH": B["DD"] > A["GG"],
        "DOWN_NEWBIRTH": B["GG"] < A["DD"],
        "UP_EXPANSION": B["ZD"] > A["ZG"] and B["DD"] <= A["GG"],
        "DOWN_EXPANSION": B["ZG"] < A["ZD"] and B["GG"] >= A["DD"],
    }
    return [k for k, v in preds.items() if v]


def leaf_of(A, B):
    m = leaves_matching(A, B)
    assert len(m) == 1, (A, B, m)
    return m[0]


def containment(alo, ahi, blo, bhi):
    a_has_b = alo <= blo and bhi <= ahi
    b_has_a = blo <= alo and ahi <= bhi
    return "EQUAL" if a_has_b and b_has_a else "A_CONTAINS_B" if a_has_b else "B_CONTAINS_A" if b_has_a else "NONE"


def pair_record(i, a, b):
    rec = {"left": i, "right": i + 1, "a_first": a["first"], "b_first": b["first"],
           "connectors": list(range(a["last"] + 1, b["first"]))}
    if a["DD"] is None or b["DD"] is None:
        rec.update({"status": "PENDING_UPGRADE_TB02H", "leaf": None, "direction": None, "relation4": None,
                    "matrix": None, "witnesses": None})
        return rec
    leaf = leaf_of(a, b)
    rec.update({"status": "EVALUATED", "leaf": leaf,
                "direction": "UP" if leaf.startswith("UP_") else "DOWN" if leaf.startswith("DOWN_") else None,
                "relation4": PROJ[leaf],
                "matrix": {f"B.{x}?A.{y}": cmp(b[x], a[y]) for x in ENDPOINTS for y in ENDPOINTS},
                "witnesses": {"core_tangent": b["ZD"] == a["ZG"] or b["ZG"] == a["ZD"],
                              "outer_touch": b["DD"] == a["GG"] or b["GG"] == a["DD"],
                              "outer_containment": containment(a["DD"], a["GG"], b["DD"], b["GG"]),
                              "core_containment": containment(a["ZD"], a["ZG"], b["ZD"], b["ZG"])}})
    fa = {"ZD": a["ZD"], "ZG": a["ZG"], "DD": a["DD_full"], "GG": a["GG_full"]}
    fb = {"ZD": b["ZD"], "ZG": b["ZG"], "DD": b["DD_full"], "GG": b["GG_full"]}
    rec["leaf_if_full_unit_outer"] = leaf_of(fa, fb)
    return rec


def tower_centers(units, windows):
    out = []
    for w in windows:
        i, j = w["seed_at"], w["exit"]
        if j - i < UPGRADE:
            r = zn_roles(units[i:j])
            out.append({"win": [i, j - 1], "ZD": w["zd"], "ZG": w["zg"], "DD": r["DD"], "GG": r["GG"], "sub": False})
        else:
            k = (j - i) // 3
            for t in range(k):
                s = i + 3 * t
                e = j - 1 if t + 1 == k else s + 2
                sub = units[s:e + 1]
                out.append({"win": [s, e], "ZD": w["zd"], "ZG": w["zg"], "DD": min(u["lo"] for u in sub),
                            "GG": max(u["hi"] for u in sub), "sub": True})
    return out


def run_stream(name, prep_name, endpoints, note):
    units = units_from_endpoints(endpoints)
    attempts, windows = scan(units)
    cs = centers_of(units, windows)
    tower = tower_centers(units, windows)
    return {"name": name, "prep_name": prep_name, "note": note, "endpoints": endpoints,
            "attempts": attempts, "centers": cs,
            "pairs": [pair_record(i, a, b) for i, (a, b) in enumerate(zip(cs, cs[1:]))],
            "tower_centers": tower,
            "tower_pairs": [{"a_win": a["win"], "b_win": b["win"], "a_sub": a["sub"], "b_sub": b["sub"],
                             "leaf": leaf_of(a, b), "relation4": PROJ[leaf_of(a, b)]}
                            for a, b in zip(tower, tower[1:])]}


BASE = [10, 40, 20, 35, 30, 38, 21, 25, 5]
STREAMS = [
    ("W0_MIN_PAIR", "W0_MIN_PAIR", [10, 40, 20, 50, 45, 60, 50, 55], "成对最小流：A=u0..u2、突破段 u3、B=u4..u6；UP_EXPANSION"),
    ("W1_DOWN_EXPANSION", "W1_UP_E3_K12", BASE + [15, 2, 12, 4, 9], "#1419 UP 流 K12（center_scan_1419.rs UP 前 14 点）；外缘相交、向下扩展"),
    ("W2_UP_EXPANSION", "W2_DOWN_MIRROR_K12", [45 - p for p in BASE + [15, 2, 12, 4, 9]], "W1 镜像 p'=45−p；向上扩展"),
    ("W3_DISC_DOWN_NEWBIRTH", "W3_DISC_DOWN", BASE + [9, 2, 8, 3], "R26 判别例：Zn 外缘 DOWN_NEWBIRTH，全单元外缘 DOWN_EXPANSION"),
    ("W4_DISC_UP_NEWBIRTH", "W4_DISC_UP", [45 - p for p in BASE + [9, 2, 8, 3]], "W3 镜像：Zn UP_NEWBIRTH，全单元 UP_EXPANSION"),
    ("W5_OUTER_TOUCH", "W5_OUTER_TOUCH", BASE + [10, 2, 8, 3], "外缘恰等 B.GG=A.DD=10 仍归扩展（A-ST-014-02）"),
    ("W6_CORE_TANGENT", "W6_CORE_TANGENT", [10, 40, 20, 35, 30, 50, 40, 60, 30, 40, 35, 38], "核心相切 B.ZD=A.ZG 归首叶；两单元连接段"),
    ("W7_CONTAIN_ENDPOINT", "W7_CONTAIN_ENDPOINT", [10, 40, 20, 35, 30, 50, 40, 55, 42, 48, 5, 60, 45], "B 外缘包含 A 外缘、UP_EXPANSION；u5 为 #1478 端点条款下的连接段"),
    ("W8_E4_TWO_UNIT_CONNECTOR", "W8_E4_TWO_UNIT_CONNECTOR", BASE + [15, 2, 18, 15, 17], "#1419 E4 流：seed 相切失败后两单元连接段，DOWN_EXPANSION"),
    ("W9_UPGRADE_THEN_B", "W9_UPGRADE_THEN_B", [10, 40, 20, 35, 30, 38, 22, 36, 25, 33, 21, 26, 5, 9, 2, 8, 3], "A 12 成员 UPGRADE_PENDING_TB02H，对 A→B 待定；塔子中枢关系不得照抄"),
    ("W10_TOUCH_POSITIONS", "W10_TOUCH_POSITIONS", [10, 40, 20, 45, 40, 50, 12, 20, 8, 15, 3, 12, 5], "CC-018 HIGH_TOUCH/LOW_TOUCH 都是延伸成员"),
    ("W11_CORE_OVERLAP", "W6b_CORE_OVERLAP", [10, 40, 20, 35, 30, 50, 40, 60, 30, 40, 33, 38], "W6 改 u9 终点：核心重叠，同属首叶"),
]
SERIES = [
    ("W12_LEAF_REVISION", "W3b_LEAF_REVISION", BASE + [9, 2, 8, 3, 12, 4], [11, 12, 13],
     "W3 续：B 开放期 B.GG 9→12，K13 由 DOWN_NEWBIRTH 修订为 DOWN_EXPANSION"),
]


def main():
    doc = {"schema": "tb1424-worked-examples/1",
           "generator": ".chanlun/review-results/issue1424-tb02f-20261014/oracle_pairs.py",
           "endpoint_order": list(ENDPOINTS),
           "streams": [run_stream(*s) for s in STREAMS],
           "series": [{"name": name, "prep_name": prep, "note": note, "endpoints": ep,
                       "cuts": [dict(run_stream(f"{name}@K{k}", prep, ep[:k + 2], note), k=k) for k in ks]}
                      for name, prep, ep, ks, note in SERIES]}
    json.dump(doc, sys.stdout, ensure_ascii=False, indent=1, sort_keys=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
