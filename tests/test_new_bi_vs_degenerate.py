"""新笔 vs 退化段 — 端到端验证

谱系 001 假说：退化段（direction=up 但 ep1_price < ep0_price）的根因是旧笔
拒绝了某些应成笔的短促波动，导致线段特征序列产出不合理的分型。

验证方式：直接用 Stroke 对象构造已知会产出退化段的笔序列，
观察 segments_from_strokes_v1 的输出。

注意：这是探索性测试（谱系 001 为生成态）。测试失败可能意味着：
(a) 退化段产出机制与假说不符
(b) 构造的数据不够典型
"""

from __future__ import annotations

from newchan.a_stroke import Stroke
from newchan.a_segment_v1 import segments_from_strokes_v1


def _make_strokes_with_degenerate_potential() -> list[Stroke]:
    """构造一组笔序列，使向上段最终的 top 低于起点 bottom。

    笔序列（7笔，形成向上段）：
    up1:   100→120  (i0=0, i1=4)
    down1: 120→105  (i0=4, i1=8)
    up2:   105→115  (i0=8, i1=12)
    down2: 115→98   (i0=12, i1=16)   ← 深跌，跌破起点
    up3:   98→108   (i0=16, i1=20)
    down3: 108→92   (i0=20, i1=24)   ← 再跌
    up4:   92→99    (i0=24, i1=28)   ← top=99 < 起点bottom=100

    如果 v1 在 up4 处终结第一段，则：
    - direction=up（继承 up1）
    - ep0_price=100（up1 的 bottom）
    - ep1_price=99（up4 的 top）
    - 退化：99 < 100
    """
    return [
        # up1: 100→120
        Stroke(i0=0, i1=4, direction="up",
               high=120.0, low=100.0, p0=100.0, p1=120.0, confirmed=True),
        # down1: 120→105
        Stroke(i0=4, i1=8, direction="down",
               high=120.0, low=105.0, p0=120.0, p1=105.0, confirmed=True),
        # up2: 105→115
        Stroke(i0=8, i1=12, direction="up",
               high=115.0, low=105.0, p0=105.0, p1=115.0, confirmed=True),
        # down2: 115→98
        Stroke(i0=12, i1=16, direction="down",
               high=115.0, low=98.0, p0=115.0, p1=98.0, confirmed=True),
        # up3: 98→108
        Stroke(i0=16, i1=20, direction="up",
               high=108.0, low=98.0, p0=98.0, p1=108.0, confirmed=True),
        # down3: 108→92
        Stroke(i0=20, i1=24, direction="down",
               high=108.0, low=92.0, p0=108.0, p1=92.0, confirmed=True),
        # up4: 92→99
        Stroke(i0=24, i1=28, direction="up",
               high=99.0, low=92.0, p0=92.0, p1=99.0, confirmed=False),
    ]


class TestDegenerateSegmentDetection:
    """探测 v1 是否在给定笔序列上产出退化段。"""

    def test_detect_degenerate_from_strokes(self):
        """观察 v1 对这组笔的输出，记录退化段是否产出。"""
        strokes = _make_strokes_with_degenerate_potential()
        segments = segments_from_strokes_v1(strokes)

        degenerate_found = False
        for seg in segments:
            if seg.ep0_price == 0.0 or seg.ep1_price == 0.0:
                continue
            if seg.direction == "up" and seg.ep1_price < seg.ep0_price:
                degenerate_found = True
            if seg.direction == "down" and seg.ep1_price > seg.ep0_price:
                degenerate_found = True

        # 观察性输出（不做断言——这是探索性测试）
        print(f"\n=== 退化段检测 ===")
        print(f"输入笔数: {len(strokes)}")
        print(f"输出段数: {len(segments)}")
        for i, seg in enumerate(segments):
            print(f"  seg[{i}]: dir={seg.direction}, "
                  f"ep0={seg.ep0_price}, ep1={seg.ep1_price}, "
                  f"s0={seg.s0}, s1={seg.s1}, kind={seg.kind}")
            if seg.ep0_price and seg.ep1_price:
                ok = "OK" if (
                    (seg.direction == "up" and seg.ep1_price >= seg.ep0_price)
                    or (seg.direction == "down" and seg.ep1_price <= seg.ep0_price)
                ) else "DEGENERATE"
                print(f"         → {ok}")
        print(f"退化段存在: {degenerate_found}")


class TestReducedStrokesNoDegeneracy:
    """假说验证：如果新笔过滤掉了 down2 和 up3（间距不足的笔），
    剩余笔序列是否还产出退化段？

    这模拟新笔效果：某些旧笔下成笔的短促波动在新笔下不成笔，
    笔序列变短，退化段消失。
    """

    def test_fewer_strokes_no_degenerate(self):
        """移除中间的短促笔（模拟新笔过滤效果），检查退化段。"""
        # 简化的笔序列：去掉 down2/up3 这种回头波动
        strokes = [
            Stroke(i0=0, i1=4, direction="up",
                   high=120.0, low=100.0, p0=100.0, p1=120.0, confirmed=True),
            Stroke(i0=4, i1=8, direction="down",
                   high=120.0, low=105.0, p0=120.0, p1=105.0, confirmed=True),
            Stroke(i0=8, i1=12, direction="up",
                   high=115.0, low=105.0, p0=105.0, p1=115.0, confirmed=True),
            # 跳过 down2/up3/down3（新笔下不成笔）
            # 直接到终止笔
            Stroke(i0=12, i1=28, direction="down",
                   high=115.0, low=92.0, p0=115.0, p1=92.0, confirmed=False),
        ]
        segments = segments_from_strokes_v1(strokes)

        print(f"\n=== 精简笔序列 ===")
        print(f"输入笔数: {len(strokes)}")
        print(f"输出段数: {len(segments)}")
        for i, seg in enumerate(segments):
            print(f"  seg[{i}]: dir={seg.direction}, "
                  f"ep0={seg.ep0_price}, ep1={seg.ep1_price}, "
                  f"s0={seg.s0}, s1={seg.s1}")

        # 精简后不应有退化段
        for seg in segments:
            if seg.ep0_price == 0.0 or seg.ep1_price == 0.0:
                continue
            if seg.direction == "up":
                assert seg.ep1_price >= seg.ep0_price, (
                    f"退化段仍然存在: up seg ep1={seg.ep1_price} < ep0={seg.ep0_price}"
                )
            else:
                assert seg.ep1_price <= seg.ep0_price, (
                    f"退化段仍然存在: down seg ep1={seg.ep1_price} > ep0={seg.ep0_price}"
                )
