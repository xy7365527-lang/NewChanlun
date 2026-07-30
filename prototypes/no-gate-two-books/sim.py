# PROTOTYPE — 一次性验证件，非生产代码。用后请随 throwaway 分支归档，勿入 main。
#
# 问题（唯一要回答的）：
#   「无门」行为——本级账与次级别账各自只查自己的前提、无互斥门——
#   在四个典型场景下是否始终：动作归属唯一、账目清晰、没有任何情形看起来"不对"。
#   S1 中枢形成期（第1-3段，无已确认中枢）
#   S2 中枢在场震荡（两前提同时满足 → 两账同时开火）
#   S3 高抛后出三类买点（本级收手规则：三类买点处回补）
#   S4 本级一类卖点（父死子从：命通道独立于账）
#
# 运行：python3 prototypes/no-gate-two-books/sim.py

import sys

# ================= 逻辑层（纯，无打印——TUI 只读它的返回值） =================

class Book:
    """一本账：某级别的双向仓。动作只落在自己身上。"""
    def __init__(self, level, name):
        self.level = level
        self.name = name
        self.long = 0
        self.short = 0
        self.long_cost = 0.0     # 多仓成本总额（均价=成本/数量）
        self.short_cost = 0.0
        self.realized = 0.0
        # 短差暂存（高抛未回补的数量与价格）
        self.sd_out_qty = 0
        self.sd_out_px = 0.0

    def buy_long(self, qty, px):
        self.long_cost += qty * px
        self.long += qty

    def sell_long(self, qty, px):
        if qty <= 0:
            return 0.0
        avg = self.long_cost / self.long if self.long else 0.0
        self.long_cost -= avg * qty
        self.long -= qty
        pnl = (px - avg) * qty
        self.realized += pnl
        return pnl

    def open_short(self, qty, px):
        self.short_cost += qty * px
        self.short += qty

    def close_short(self, qty, px):
        if qty <= 0:
            return 0.0
        avg = self.short_cost / self.short if self.short else 0.0
        self.short_cost -= avg * qty
        self.short -= qty
        pnl = (avg - px) * qty
        self.realized += pnl
        return pnl


class World:
    """两本账 + 中枢（事件确认）+ 段序列。无任何互斥门。"""
    def __init__(self):
        self.px = 100.0
        self.L1 = Book(1, "本级账")
        self.L0 = Book(0, "次级别账")
        self.L1.buy_long(300, 90.0)          # 大级别买点介入（起点持仓）
        self.center = None                    # {"zd":..,"zg":..,"born_seg":n}
        self.segs = []                        # 已完成次级别段 [(dir, lo, hi)]
        self.log = []

    # ---- 结构事件 ----
    def seg_end(self, direction):
        """次级别一段走完。第三段与前两段重叠 => 中枢出生（事件）。"""
        acts = []
        swing = 8 if direction == "down" else 10
        new_px = self.px - swing if direction == "down" else self.px + swing
        lo, hi = (new_px, self.px) if direction == "down" else (self.px, new_px)
        self.px = new_px
        self.segs.append((direction, lo, hi))
        acts.append(f"[结构] 次级别段({direction})走完 [{lo:.0f},{hi:.0f}]")
        if self.center is None and len(self.segs) >= 3:
            zd = max(s[1] for s in self.segs[-3:])
            zg = min(s[2] for s in self.segs[-3:])
            if zd <= zg:
                self.center = {"zd": zd, "zg": zg, "born_seg": len(self.segs)}
                acts.append(f"[事件] ★ 中枢出生 [{zd:.0f},{zg:.0f}]（第3段重叠完成，事件确认）")
        return acts

    def bsp1(self, side):
        """次级别一类买卖点。两本账各自查自己的前提，各自动作。"""
        acts = []
        if side == "sell":
            # 本级账：自己的中枢在场 且 价在上沿区 => 高抛（本级账减补，B 裁定）
            if self.center and self.px >= self.center["zg"] and self.L1.long > 0:
                q = self.L1.long // 3
                if q > 0:
                    self.L1.sell_long(q, self.px)
                    self.L1.sd_out_qty += q
                    self.L1.sd_out_px = self.px
                    acts.append(f"[L1] 高抛减 {q} @ {self.px:.0f}（本级账短差：中枢在场+次级别卖点）")
            # 次级别账：次级别一类卖点 => 首开反向（自己的规则，永远不受本级语境限制）
            self.L0.open_short(50, self.px)
            acts.append(f"[L0] 首开反向 开空 50 @ {self.px:.0f}（次级别账：一类卖点）")
        else:  # buy
            # 本级账：自己的中枢在场 且 有高抛未回补 且 价在下沿区 => 回补
            if self.center and self.L1.sd_out_qty > 0 and self.px <= self.center["zd"]:
                q = self.L1.sd_out_qty
                self.L1.buy_long(q, self.px)
                acts.append(f"[L1] 回补 {q} @ {self.px:.0f}（本级账短差：下沿）")
                self.L1.sd_out_qty = 0
            # 次级别账：一类买点 => 平空
            if self.L0.short > 0:
                q = self.L0.short
                self.L0.close_short(q, self.px)
                acts.append(f"[L0] 平空 {q} @ {self.px:.0f}（次级别账：一类买点）")
        return acts

    def bsp3_buy(self):
        """三类买点：中枢破坏向上。本级收手规则：高抛未回补 => 于三类买点处回补。"""
        acts = []
        if self.center:
            zg = self.center["zg"]
            self.px = zg + 5
            acts.append(f"[结构] 三类买点 @ {self.px:.0f}（回试不破 ZG={zg:.0f}，中枢破坏）")
            if self.L1.sd_out_qty > 0:
                q = self.L1.sd_out_qty
                self.L1.buy_long(q, self.px)
                acts.append(f"[L1] ★ 于三类买点处回补 {q} @ {self.px:.0f}（收手规则：不等下沿了，趋势腿满仓优先）")
                self.L1.sd_out_qty = 0
            self.center = None
            acts.append("[L1] 中枢破坏 → 短差通道关闭（无中枢不做短差）")
            acts.append("[L0] 不受本级收手规则管——L0 的空仓等 L0 自己的信号")
        else:
            acts.append("[结构] 三类买点（无已确认中枢，本级无短差状态可收）")
        return acts

    def l1_bsp1_sell(self):
        """本级一类卖点：本级账全平 + 父死子从（命通道）。"""
        acts = []
        if self.L1.long > 0:
            q = self.L1.long
            self.L1.sell_long(q, self.px)
            acts.append(f"[L1] 一类卖点 全平 {q} @ {self.px:.0f}（本级账：本级证书）")
        pruned = 0
        if self.L0.short > 0:
            pruned = self.L0.short
            self.L0.close_short(pruned, self.px)
        if pruned:
            acts.append(f"[命] ★ 父死子从：L0 空仓 {pruned} 被连坐剪除 @ {self.px:.0f}（AncOK，非账本动作）")
        self.center = None
        return acts

    # ---- 状态快照（TUI 渲染用） ----
    def snapshot(self):
        def b(B):
            avg_l = B.long_cost / B.long if B.long else 0
            avg_s = B.short_cost / B.short if B.short else 0
            return (f"{B.name}: 多 {B.long}(均{avg_l:.1f}) 空 {B.short}(均{avg_s:.1f}) "
                    f"已实现 {B.realized:+.1f} 高抛挂起 {B.sd_out_qty}")
        c = self.center
        cs = f"[{c['zd']:.0f},{c['zg']:.0f}]" if c else "无"
        gross = self.L1.long + self.L1.short + self.L0.long + self.L0.short
        net = (self.L1.long - self.L1.short) + (self.L0.long - self.L0.short)
        return dict(price=self.px, center=cs, L1=b(self.L1), L0=b(self.L0),
                    gross=gross, net=net)


# ================= TUI 层（一次性壳） =================

KEYS = ("[1]段↓ [2]段↑ [b]次级别一类卖 [a]次级别一类买 "
        "[3]三类买 [L]本级一类卖 [s]剧本演示 [r]重置 [q]退出")

def render(w, note=""):
    print("\033[2J\033[H", end="")
    s = w.snapshot()
    print("\x1b[1m「无门」两本账原型\x1b[0m  \x1b[2m（throwaway，验证用）\x1b[0m")
    print("─" * 64)
    print(f"\x1b[1m价格\x1b[0m {s['price']:.0f}   \x1b[1m已确认中枢\x1b[0m {s['center']}")
    print(f"  {s['L1']}")
    print(f"  {s['L0']}")
    print(f"  毛敞口 {s['gross']}   净敞口 {s['net']}")
    print("─" * 64)
    print("\x1b[1m最近动作\x1b[0m")
    for line in w.log[-10:]:
        print("  " + line)
    if note:
        print("\x1b[1m" + note + "\x1b[0m")
    print("─" * 64)
    print(KEYS)

def do(w, k):
    if k == "1":
        return w.seg_end("down")
    if k == "2":
        return w.seg_end("up")
    if k == "b":
        return w.bsp1("sell")
    if k == "a":
        return w.bsp1("buy")
    if k == "3":
        return w.bsp3_buy()
    if k == "L":
        return w.l1_bsp1_sell()
    return []

SCENARIO = [
    ("S1 中枢形成期：次级别点只有次级别账动", ["1", "2", "b", "a"]),
    ("S2 中枢在场：两前提同时满足 → 两账同时开火", ["1", "2", "b", "1", "1", "a"]),
    ("S3 高抛后三类买点：本级于三类买点回补收手", ["2", "2", "b", "3"]),
    ("S4 本级一类卖点：全平 + 父死子从", ["2", "L"]),
]

def run_scenario(w):
    for title, keys in SCENARIO:
        w.log.append(f"──── {title} ────")
        for k in keys:
            acts = do(w, k)
            w.log.extend(acts or [f"[{k}]（无动作）"])
            render(w)
            input("  <Enter 下一步>")
    w.log.append("──── 剧本完。看点：S2 两账同时开火、各记各账；S3 收手只关本级通道；S4 命通道独立 ────")

def main():
    w = World()
    note = "驱动事件流，看两本账各自查自己前提（无门）"
    while True:
        render(w, note)
        note = ""
        k = input("> ").strip()
        if k == "q":
            break
        if k == "r":
            w = World()
            continue
        if k == "s":
            run_scenario(w)
            continue
        acts = do(w, k)
        w.log.extend(acts or ([f"[{k}]（无动作）"] if k in "12ba3L" else []))

if __name__ == "__main__":
    main()
