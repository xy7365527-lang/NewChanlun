"""买卖点引擎函数在笔中枢（525号）上的回归守卫。

落实 525号上下游推论（在 candidate 分离的代码中体现）：
- 点1：buysellpoints_from_level 直接消费 zhongshu_from_strokes 产出的笔中枢（笔作组件）。
- 点2：判据 = 组件已完成（confirmed 笔），不依赖递归链完整性、不下钻。
- 点4：confirmed 由本级别结构条件（回试后延续段）决定，与递归链/Move.settled 无关。

证明同一函数不加修改地服务段中枢与笔中枢两条路径（component-source-agnostic）。
"""

from __future__ import annotations

from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_move_v1 import moves_from_zhongshus
from newchan.a_stroke import Stroke
from newchan.a_zhongshu_v1 import zhongshu_from_strokes


def _stroke(i0: int, i1: int, direction: str, high: float, low: float,
            confirmed: bool = True) -> Stroke:
    """构造最小 Stroke（p0/p1 端点价按方向取 high/low）。"""
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction, high=high, low=low,
                  p0=p0, p1=p1, confirmed=confirmed)


def _build_stroke_3b(with_continuation: bool) -> list[Stroke]:
    """笔中枢 3B 场景：

    stroke0-2 → 笔中枢 [zd=12, zg=18]
    stroke3   → 向上突破笔（low=19 > zg=18）
    stroke4   → 回试笔（down, low=19 > zg=18 → 不破，3B candidate）
    stroke5   → 向上延续笔（仅 with_continuation=True）→ confirmed
    """
    strokes = [
        _stroke(0, 5, "up", 18.0, 12.0),
        _stroke(5, 10, "down", 18.0, 12.0),
        _stroke(10, 15, "up", 18.0, 12.0),
        _stroke(15, 20, "up", 30.0, 19.0),   # 突破笔
        _stroke(20, 25, "down", 25.0, 19.0),  # 回试笔 low=19 > zg=18
    ]
    if with_continuation:
        strokes.append(_stroke(25, 30, "up", 32.0, 20.0))  # 延续笔
    return strokes


class TestBuySellPointOnStrokeCenter:
    """buysellpoints_from_level 在笔中枢上的 candidate/confirmed 行为。"""

    def test_stroke_center_forms(self):
        """zhongshu_from_strokes 在该结构上产出一个笔中枢（zd=12, zg=18, 向上突破）。"""
        strokes = _build_stroke_3b(with_continuation=False)
        zss = zhongshu_from_strokes(strokes)
        assert len(zss) == 1
        zs = zss[0]
        assert zs.zd == 12.0 and zs.zg == 18.0
        assert zs.settled is True
        assert zs.break_direction == "up"

    def test_type3_candidate_on_stroke_center(self):
        """笔中枢回试不破、无延续笔 → Type3 buy candidate（confirmed=False）。

        点1/2：买卖点引擎直接吃笔中枢；点4：confirmed 由结构（延续段）决定。
        """
        strokes = _build_stroke_3b(with_continuation=False)
        zss = zhongshu_from_strokes(strokes)
        moves = moves_from_zhongshus(zss, num_segments=len(strokes))
        bsps = buysellpoints_from_level(strokes, zss, moves, [], level_id=1)
        t3 = [b for b in bsps if b.kind == "type3" and b.side == "buy"]
        assert len(t3) == 1
        assert t3[0].seg_idx == 4          # 回试笔索引（笔作组件）
        assert t3[0].price == 19.0         # 回试笔 low
        assert t3[0].confirmed is False    # 候选：延续笔未现

    def test_type3_confirmed_on_stroke_center(self):
        """笔中枢回试后出现向上延续笔 → Type3 buy confirmed=True。"""
        strokes = _build_stroke_3b(with_continuation=True)
        zss = zhongshu_from_strokes(strokes)
        moves = moves_from_zhongshus(zss, num_segments=len(strokes))
        bsps = buysellpoints_from_level(strokes, zss, moves, [], level_id=1)
        t3 = [b for b in bsps if b.kind == "type3" and b.side == "buy"]
        assert len(t3) == 1
        assert t3[0].confirmed is True     # 延续笔出现 → 回试不破确认

    def test_confirmed_independent_of_recursion_chain(self):
        """点4回归守卫：confirmed 不依赖递归链完整性。

        笔中枢的组件是笔（终端递归单位，无更低级别可下钻），confirmed 仍正常翻转——
        证明 confirmed 判据是本级别结构条件，与"能否追溯到 Move[0]"无关。
        """
        # 笔无更低 stroke-index 层（i0/i1 是 merged bar），递归链在此终止
        cand = _build_stroke_3b(with_continuation=False)
        conf = _build_stroke_3b(with_continuation=True)
        zss_c = zhongshu_from_strokes(cand)
        zss_f = zhongshu_from_strokes(conf)
        t3_c = [b for b in buysellpoints_from_level(
            cand, zss_c, moves_from_zhongshus(zss_c, num_segments=len(cand)), [], 1)
            if b.kind == "type3"]
        t3_f = [b for b in buysellpoints_from_level(
            conf, zss_f, moves_from_zhongshus(zss_f, num_segments=len(conf)), [], 1)
            if b.kind == "type3"]
        # 同一终端递归单位级别上，confirmed 仅由结构（延续段）翻转
        assert t3_c[0].confirmed is False
        assert t3_f[0].confirmed is True
