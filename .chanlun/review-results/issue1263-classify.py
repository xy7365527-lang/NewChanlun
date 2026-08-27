#!/usr/bin/env python3
"""#1263 探针分类器（纯标准库，可复现）：nested_fugue 三处「价格破记录极值 ⟹ 否定」
× 同 bar「背驰段被打破」结构判据交叉读数。

事件 = Rust 探针 `nested_fugue_extreme_probe`（#[cfg(test)] #[ignore]，P1263_DUMP_PATH 落盘）
三处否定 site（窗口清窗 window_clear / 定位失效 located_invalid / 声部解栈 voice_unwind）
落「价格破 ∧ 结构破 任一成立」的事件级 JSONL。每事件带：
  price_broke —— 价格代理（c > extreme for Short / c < extreme for Long，即生产否定条件）；
  struct_broke —— 结构判据「背驰段被打破」（生产结构函数，Rust 侧操作化）：
    Short(卖/顶背驰, 段方向Up) ⟹ 向下打破 = confirmed Sell1/Sell3 ∨ 该层方向翻 Down；
    Long(买/底背驰, 段方向Down) ⟹ 向上打破 = confirmed Buy1/Buy3 ∨ 该层方向翻 Up；
  structural —— 同 bar 结构态原始分量（中枢三态 CenterBook / confirmed BSP / 方向翻转 / up_settled）。

三分类（主口径，事件级）：
  吻合 = 价格破 ∧ 结构破；代理误杀 = 价格破 ∧ 结构未破；代理漏杀 = 结构破 ∧ 价格未破。
对照臂（诊断）：价格破 × 「走势未完成」（#1267 结构判据：走势类型延续 ∨ 中枢未死 ∨ 三买卖未坐实）。

输入: events_jsonl
"""
import json
import sys
from collections import Counter

SITES = ["window_clear", "located_invalid", "voice_unwind"]
SITE_CN = {
    "window_clear": "窗口清窗",
    "located_invalid": "定位失效",
    "voice_unwind": "声部解栈",
}


def struct_broke(e):
    s = e["structural"]
    if e["dir"] == "Short":
        return s["bsp_confirmed_sell1"] or s["bsp_confirmed_sell3"] or s["flip_dir"] == "Down"
    return s["bsp_confirmed_buy1"] or s["bsp_confirmed_buy3"] or s["flip_dir"] == "Up"


def move_unfinished(e):
    """#1267 结构判据「走势未完成」镜像（诊断对照臂）。"""
    s = e["structural"]
    if e["dir"] == "Short":
        return s["dir_now"] == "Up" or s["center_alive"] or not s["center_dead_down"]
    return s["dir_now"] == "Down" or s["center_alive"] or not s["center_frozen"]


def cell(e):
    if e["price_broke"] and struct_broke(e):
        return "吻合"
    if e["price_broke"] and not struct_broke(e):
        return "误杀"
    if not e["price_broke"] and struct_broke(e):
        return "漏杀"
    return "无事件"


def main():
    events = [json.loads(l) for l in open(sys.argv[1])]
    n = len(events)
    print(f"事件总数 = {n}（价格破 {sum(1 for e in events if e['price_broke'])} / "
          f"仅结构破 {sum(1 for e in events if not e['price_broke'])}）")

    # ── 主口径三分类 ──
    print("\n===== 主口径：价格破 × 背驰段被打破（同 bar 结构判据）=====")
    g = Counter(cell(e) for e in events)
    print(f"总计：吻合={g['吻合']} 误杀={g['误杀']} 漏杀={g['漏杀']} "
          f"（吻合率 {g['吻合']/n*100:.2f}%）")
    for site in SITES:
        sub = [e for e in events if e["site"] == site]
        a = sum(1 for e in sub if cell(e) == "吻合")
        fp = sum(1 for e in sub if cell(e) == "误杀")
        fn = sum(1 for e in sub if cell(e) == "漏杀")
        print(f"  {SITE_CN[site]}({site}) n={len(sub)}：吻合={a} 误杀={fp} 漏杀={fn}")

    print("\n===== 每 site × ladder × dir 明细（吻合/误杀/漏杀）=====")
    for site in SITES:
        sub = [e for e in events if e["site"] == site]
        for lad in sorted({e["ladder"] for e in sub}):
            for d in sorted({e["dir"] for e in sub}):
                ss = [e for e in sub if e["ladder"] == lad and e["dir"] == d]
                if not ss:
                    continue
                a = sum(1 for e in ss if cell(e) == "吻合")
                fp = sum(1 for e in ss if cell(e) == "误杀")
                fn = sum(1 for e in ss if cell(e) == "漏杀")
                print(f"  {site} lad={lad} dir={d}: n={len(ss)} 吻合={a} 误杀={fp} 漏杀={fn}")

    # ── 对照臂：走势未完成（#1267）──
    print("\n===== 对照臂：价格破 × 走势未完成（#1267 结构判据镜像）=====")
    g2 = Counter()
    for e in events:
        mu = move_unfinished(e)
        if e["price_broke"] and mu:
            g2["吻合(未完成)"] += 1
        elif e["price_broke"] and not mu:
            g2["误杀(已完成)"] += 1
        elif not e["price_broke"] and mu:
            g2["漏杀(未完成)"] += 1
        else:
            g2["无事件"] += 1
    print(f"总计：{dict(g2)}")
    for site in SITES:
        sub = [e for e in events if e["site"] == site]
        a = sum(1 for e in sub if e["price_broke"] and move_unfinished(e))
        fp = sum(1 for e in sub if e["price_broke"] and not move_unfinished(e))
        fn = sum(1 for e in sub if not e["price_broke"] and move_unfinished(e))
        print(f"  {SITE_CN[site]} n={len(sub)}：吻合(未完成)={a} 误杀(已完成)={fp} 漏杀(未完成)={fn}")

    # ── 漏杀触发分量 ──
    print("\n===== 漏杀触发分量（结构破的哪个生产分量触发）=====")
    for site in SITES:
        sub = [e for e in events if e["site"] == site and cell(e) == "漏杀"]
        comp = Counter()
        for e in sub:
            s = e["structural"]
            if e["dir"] == "Short":
                if s["bsp_confirmed_sell1"]:
                    comp["confirmed_Sell1"] += 1
                if s["bsp_confirmed_sell3"]:
                    comp["confirmed_Sell3(三卖坐实)"] += 1
                if s["flip_dir"] == "Down":
                    comp["方向翻Down(走势完成)"] += 1
            else:
                if s["bsp_confirmed_buy1"]:
                    comp["confirmed_Buy1"] += 1
                if s["bsp_confirmed_buy3"]:
                    comp["confirmed_Buy3(三买坐实)"] += 1
                if s["flip_dir"] == "Up":
                    comp["方向翻Up(走势完成)"] += 1
        print(f"  {SITE_CN[site]} 漏杀 n={len(sub)}：{dict(comp)}")

    # ── 误杀结构态 ──
    print("\n===== 误杀结构态（价格破时结构面缺什么）=====")
    for site in SITES:
        sub = [e for e in events if e["site"] == site and cell(e) == "误杀"]
        comp = Counter()
        for e in sub:
            s = e["structural"]
            comp[f"confirmed_any={s['bsp_confirmed_any']}"] += 1
            comp[f"center_alive={s['center_alive']}"] += 1
            comp[f"center_dead_down={s['center_dead_down']}"] += 1
        print(f"  {SITE_CN[site]} 误杀 n={len(sub)}：{dict(comp)}")

    # ── 逐格样例键 ──
    print("\n===== 逐格样例键（bar, ladder, dir）=====")
    for site in SITES:
        sub = [e for e in events if e["site"] == site]
        print(f"-- {SITE_CN[site]} --")
        for c in ["吻合", "误杀", "漏杀"]:
            ex = [e for e in sub if cell(e) == c][:6]
            keys = [(e["bar"], e["ladder"], e["dir"]) for e in ex]
            print(f"  {c}: {keys}")


if __name__ == "__main__":
    main()
