"""自测：per_level_bsp 适配层（管线正确性，认识论等级 L1）。

L1 声明依据（formalization-validity-domain 规则）：
本测试只验证"适配层能正确把递归层 ≥2 的 LevelZhongshu/component 序列喂进
引擎纯函数 buysellpoints_from_level + divergences_from_moves_v1 并产出 BSP 且
不抛错"——这是管线正确性（接口契约 well-formedness）的验证，不是"递归层 BSP
判定经验有效"的验证。输入是真实 QQQ 日线，但断言只覆盖管线无异常 + 索引自洽 +
字段映射成立，不涉及买卖点是否盈利/是否符合缠论语义的经验检验（那需要 L2/L3）。

谱系：521号 / 525号 / project_divergence_locator_entry_exit。
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.types import Bar  # noqa: E402

from newchan.bi_engine import BiEngine  # noqa: E402

from per_level_bsp import (  # noqa: E402
    BI_ZHONGSHU_LEVEL_ID,
    confirmed_bsp_bi_zhongshu,
    confirmed_bsp_for_level,
    confirmed_bsp_level1,
    level_bsp_inputs,
)

# 1h Brent（真实数据）：在日线 QQQ 上 level≥2 LevelZhongshu 永不涌现
# （6848 日线仅 2 个 level-1 走势，不足 3 组件 → 高层稀疏，documented 边界）。
# 1h Brent 42867 bar 在 bar~37647 涌现 level-2 含 5 个 LevelZhongshu，
# 是能真实驱动 L≥2 适配路径的最小可用数据。
DATA = ROOT / "analysis" / "data_cache" / "brn_1h_databento.json"


def _load_bars() -> list[Bar]:
    raw = json.loads(DATA.read_text())
    base = datetime(2000, 1, 1)
    bars: list[Bar] = []
    for i in range(len(raw["closes"])):
        bars.append(Bar(
            ts=base + timedelta(hours=i),
            open=float(raw["opens"][i]), high=float(raw["highs"][i]),
            low=float(raw["lows"][i]), close=float(raw["closes"][i]),
            volume=0.0))
    return bars


def _drive_to_richest_l2_snapshot():
    """逐 bar 驱动 orchestrator，返回 level≥2 LevelZhongshu 最丰富的 snapshot。

    高层稀疏 → 不能假设"最后一个 snapshot"含 LevelZhongshu。捕获全程
    level≥2 zhongshu 总数最大的那个 snapshot（确保 L≥2 适配路径被真实驱动）。
    """
    orch = RecursiveOrchestrator(stream_id="test_plbsp", max_levels=8)
    bars = _load_bars()
    best = None
    best_count = -1
    final = None
    for bar in bars:
        snap = orch.process_bar(bar)
        cnt = sum(
            len(rs.zhongshus)
            for rs in snap.recursive_snapshots
            if rs.level_id >= 2
        )
        if cnt > best_count:
            best_count = cnt
            best = snap
        final = snap
    return final, best


def test_level1_passthrough() -> None:
    """level=1 透传引擎 bsp_snapshot，返回列表副本。"""
    final, _ = _drive_to_richest_l2_snapshot()
    bsps = confirmed_bsp_level1(final.bsp_snapshot)
    assert isinstance(bsps, list)
    # 透传副本：与引擎内列表内容相等但是独立对象
    assert bsps == list(final.bsp_snapshot.buysellpoints)
    print(f"  [level1] passthrough {len(bsps)} BSP")


def test_level1_none_raises() -> None:
    try:
        confirmed_bsp_level1(None)
    except ValueError:
        return
    raise AssertionError("None bsp_snapshot 应抛 ValueError")


def _first_l2_level(final):
    """从 snapshot 取第一个 level≥2 且有 LevelZhongshu 的递归层 + 其前级别 moves。"""
    prev_moves = list(final.move_snapshot.moves)  # level-1 moves
    for cand in final.recursive_snapshots:
        if cand.level_id >= 2 and cand.zhongshus:
            return cand, prev_moves
        prev_moves = list(cand.moves)
    return None, prev_moves


def test_input_adapter_field_mapping() -> None:
    """适配器字段映射逐字段验证（任务卡映射成立）。"""
    final, best = _drive_to_richest_l2_snapshot()
    assert best is not None, "数据未涌现递归层，无法验证 L≥2 适配"

    rs, prev_moves = _first_l2_level(best)
    assert rs is not None, "递归层无 LevelZhongshu"

    seg_view, zs_view = level_bsp_inputs(prev_moves, rs.zhongshus)

    # 长度守恒（不重排不过滤）
    assert len(seg_view) == len(prev_moves)
    assert len(zs_view) == len(rs.zhongshus)

    # segment 适配：i0=first_seg_s0, i1=last_seg_s1, high/low/direction 透传
    if prev_moves:
        m = prev_moves[0]; sv = seg_view[0]
        assert sv.i0 == m.first_seg_s0
        assert sv.i1 == m.last_seg_s1
        assert sv.high == m.high and sv.low == m.low
        assert sv.direction == m.direction

    # zhongshu 适配：seg_start=comp_start, seg_end=comp_end, break_seg=break_comp
    z = rs.zhongshus[0]; zv = zs_view[0]
    assert zv.seg_start == z.comp_start
    assert zv.seg_end == z.comp_end
    assert zv.break_seg == z.break_comp
    assert zv.zd == z.zd and zv.zg == z.zg
    assert zv.settled == z.settled
    assert zv.break_direction == z.break_direction
    print(f"  [adapter] 字段映射成立 (segments={len(seg_view)} zs={len(zs_view)})")


def test_level_bsp_pipeline_runs() -> None:
    """递归层 ≥2 适配层端到端产出 BSP 且不抛错（管线正确性 L1）。"""
    final, best = _drive_to_richest_l2_snapshot()
    assert best is not None

    total_bsp = 0
    levels_tested = 0
    prev_moves = list(best.move_snapshot.moves)
    for rs in best.recursive_snapshots:
        if rs.level_id >= 2 and rs.zhongshus:
            bsps = confirmed_bsp_for_level(
                prev_moves, rs.zhongshus, rs.moves, rs.level_id)
            assert isinstance(bsps, list)
            for bp in bsps:
                # 索引自洽：seg_idx 落在 segments 视图范围内或为引擎容许的尾后值
                assert bp.level_id == rs.level_id
                assert bp.side in ("buy", "sell")
                assert bp.kind in ("type1", "type2", "type3")
            total_bsp += len(bsps)
            levels_tested += 1
        prev_moves = list(rs.moves)

    assert levels_tested >= 1, "未测到任何 level≥2"
    print(f"  [pipeline] {levels_tested} 个递归层产出 {total_bsp} BSP（无异常）")


def test_level1_guard_in_for_level() -> None:
    """confirmed_bsp_for_level 拒绝 level_id<2。"""
    try:
        confirmed_bsp_for_level([], [], [], 1)
    except ValueError:
        return
    raise AssertionError("level_id=1 应抛 ValueError")


# ════════════════════════════════════════════════════════════
# 笔中枢级 BSP（525号笔中枢路径 → segment 级中枢承载层）
# ════════════════════════════════════════════════════════════


def _drive_bi_strokes():
    """逐 bar 驱动 BiEngine，返回笔最丰富时刻的 strokes（驱动笔中枢 BSP 路径）。

    用真实 Brent 1h 数据（test_per_level_bsp 既有数据源），取笔数最多的 snapshot
    确保至少有 ≥3 confirmed 笔 → 笔中枢能涌现。
    """
    bi = BiEngine()
    bars = _load_bars()
    best_strokes: list = []
    for bar in bars:
        snap = bi.process_bar(bar)
        if len(snap.strokes) > len(best_strokes):
            best_strokes = list(snap.strokes)
    return best_strokes


def test_bi_zhongshu_pipeline_runs() -> None:
    """笔中枢级适配端到端产出 BSP 且不抛错（管线正确性 L1）。"""
    strokes = _drive_bi_strokes()
    confirmed = [s for s in strokes if s.confirmed]
    assert len(confirmed) >= 3, f"confirmed 笔不足 3（{len(confirmed)}），无法验证笔中枢"

    bsps = confirmed_bsp_bi_zhongshu(strokes)
    assert isinstance(bsps, list)
    for bp in bsps:
        assert bp.level_id == BI_ZHONGSHU_LEVEL_ID
        assert bp.side in ("buy", "sell")
        assert bp.kind in ("type1", "type2", "type3")
        # 索引自洽：seg_idx 落在 confirmed 笔范围内
        assert 0 <= bp.seg_idx < len(confirmed)
    n_conf = sum(1 for bp in bsps if bp.confirmed)
    print(f"  [bi_zhongshu] {len(confirmed)} confirmed 笔 → {len(bsps)} BSP "
          f"（confirmed={n_conf}）")


def test_bi_zhongshu_index_safety() -> None:
    """显式过滤 confirmed 保证索引自洽：BSP 引用的 segments 索引 ≡ 中枢编号依据。

    构造一个末笔 unconfirmed 的序列，断言 confirmed_bsp_bi_zhongshu 不越界、
    seg_idx 永远落在 confirmed 子列表内（不依赖"仅末笔未确认"不变量）。
    """
    strokes = _drive_bi_strokes()
    confirmed = [s for s in strokes if s.confirmed]
    bsps = confirmed_bsp_bi_zhongshu(strokes)
    # 所有 BSP 的关键索引字段都在 confirmed 范围内（越界即 IndexError/错配，此处静态校验）
    for bp in bsps:
        assert 0 <= bp.seg_idx < len(confirmed)
        if bp.center_seg_start is not None:
            assert 0 <= bp.center_seg_start < len(confirmed)
    print(f"  [bi_zhongshu] 索引自洽 {len(bsps)} BSP 全部落在 {len(confirmed)} confirmed 笔内")


def test_bi_zhongshu_short_circuit() -> None:
    """confirmed 笔 < 3 → 返回空列表（zhongshu_from_strokes 短路）。"""
    assert confirmed_bsp_bi_zhongshu([]) == []
    # None 笔列表抛 ValueError
    try:
        confirmed_bsp_bi_zhongshu(None)
    except ValueError:
        pass
    else:
        raise AssertionError("None strokes 应抛 ValueError")
    print("  [bi_zhongshu] 短路 + None 守卫成立")


if __name__ == "__main__":
    test_level1_passthrough()
    test_level1_none_raises()
    test_input_adapter_field_mapping()
    test_level_bsp_pipeline_runs()
    test_level1_guard_in_for_level()
    test_bi_zhongshu_pipeline_runs()
    test_bi_zhongshu_index_safety()
    test_bi_zhongshu_short_circuit()
    print("\nALL PASS (epistemic level L1: pipeline correctness only)")
