"""有机赋格单元测试 — LOU 转换表（T1-T7）+ 账本不变量 + O0≡P5 合成磁带对账。

运行：PYTHONPATH=src:analysis .venv/bin/python -m pytest analysis/test_organic_fugue.py -q

测试分层：
  1. OrganicLedger：两阶段守恒律（股数守恒/金额守恒）、开腿拒绝计数（非静默）。
  2. FatigueMonitor：证据添加/清空（up move settle）、u(k) 解析、门开闭。
  3. SizeAllocator：振幅占比归一、无存活中枢零权、版本门控。
  4. LOU 转换表：T1（含 div 触发/门拒/冻结拒）/T3/T4/T5（三种 rev_close）/T6/T7。
  5. O0≡P5：合成磁带上 run_organic(O0) 与 run_version_i(P5) 逐笔+逐 trace 对账
     （真实磁带全量对账在 organic_fugue_backtest.py 的 O0 守卫步执行）。
  6. earning 反作用（O4）：master 清仓降格为 REV 腿 + 金额守恒挣股数 + 升级出场。

合成事件序的认识论位置（testing-override 生成态例外）：这些测试验证 FSM 实现
与设计转换表的一致性（L0/L1），不验证经验有效性（L2+ 见回测）。
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from fugue_version_i import (  # noqa: E402
    MAX_LADDER,
    NO_LADDER_DIVS,
    NO_LADDER_EVENTS,
    NO_UP_SETTLED,
    PAIRING_VARIANTS,
    BarSignalI,
    run_version_i,
)
from organic_fugue import (  # noqa: E402
    REV,
    REV_KEY_OFFSET,
    RIDE,
    FatigueMonitor,
    OrganicConfig,
    OrganicLedger,
    SizeAllocator,
    run_organic,
)
from fugue_version_i import LADDER_SEG, OSC_KEY_OFFSET  # noqa: E402


# ════════════════════════════════════════════════════════════
# 合成磁带构造
# ════════════════════════════════════════════════════════════

def _mask(idxs) -> tuple:
    return tuple(i in idxs for i in range(MAX_LADDER))


def bar(close: float, *, buy1=(), sell1=(), sell_any=(), buy_any=(),
        max_ladder: int = 3, type2: bool = False,
        ev: dict | None = None, dv: dict | None = None, ums=()) -> BarSignalI:
    """合成单 bar 信号。ev[ladder] = BSP 事件列表；dv[ladder] = 背驰事件列表。"""
    if ev:
        rows = [()] * MAX_LADDER
        for lad, events in ev.items():
            rows[lad] = tuple(events)
        bsp_events = tuple(rows)
    else:
        bsp_events = NO_LADDER_EVENTS
    if dv:
        drows = [()] * MAX_LADDER
        for lad, events in dv.items():
            drows[lad] = tuple(events)
        div_events = tuple(drows)
    else:
        div_events = NO_LADDER_DIVS
    ums_t = tuple(i in ums for i in range(MAX_LADDER)) if ums else NO_UP_SETTLED
    return BarSignalI(
        close=close, buy1=_mask(buy1), sell1=_mask(sell1),
        sell_any=_mask(sell_any), buy_any=_mask(buy_any),
        max_ladder=max_ladder, type2_buy=type2,
        bsp_events=bsp_events, div_events=div_events, up_move_settled=ums_t)


def bsp(kind, side, seg_idx, confirmed, cs, zd, zg, price=0.0):
    """BSP 事件元组（与 organic_signals._scan_events_rust 输出同构）。"""
    return (kind, side, seg_idx, confirmed, cs, zd, zg, price)


def div(kind, direction, side, seg_idx, fa=2.0, fc=1.0, price=0.0):
    """背驰事件元组（与 organic_signals._scan_div_events 输出同构）。"""
    return (kind, direction, side, seg_idx, fa, fc, price)


def entry_prefix(entry_close=100.0):
    """ARM（buy1[3]）→ 区间套入场（buy_any[0]）的标准两 bar 前缀 + 中枢注册。

    bar1 同时注册 ladder2 存活中枢 (cs=10, zd=95, zg=99)：用 candidate type2
    买事件携带中枢锚（candidate 不触发任何布尔/配对动作，仅被 center book 消费）。
    """
    return [
        bar(99.0, buy1=(3,)),
        bar(entry_close, buy_any=(0,),
            ev={2: [bsp("type2", "buy", 1, False, 10, 95.0, 99.0)]}),
    ]


# ════════════════════════════════════════════════════════════
# 1. OrganicLedger
# ════════════════════════════════════════════════════════════

def test_ledger_cost_phase_share_conservation():
    led = OrganicLedger(entry_price=100.0, total_shares=10.0, cost_basis=100.0,
                        level_frac=0.5)
    assert led.open_diff(2, 0.5, 102.0)
    assert led.total_shares == 10.0          # 股数守恒：虚拟腿不动仓
    led.close_diff(2, 98.0)
    # profit = (102-98)×5 = 20 → cost_basis -= 20/10 = 2
    assert abs(led.cost_basis - 98.0) < 1e-12
    assert abs(led.cumulative_recovered - 20.0) < 1e-12
    assert not led.earning


def test_ledger_earning_amount_conservation():
    led = OrganicLedger(entry_price=100.0, total_shares=10.0, cost_basis=0.0,
                        level_frac=1.0, earning=True)
    assert led.open_diff(2 + REV_KEY_OFFSET, 1.0, 20.0)
    assert led.total_shares == 0.0           # earning 开腿真实减仓（INV-1 构造保证）
    led.close_diff(2 + REV_KEY_OFFSET, 19.0)
    # 金额守恒：10×20/19 = 10.526… 股（43课"20卖1万，19回补1万多股"）
    assert abs(led.total_shares - 10.0 * 20.0 / 19.0) < 1e-12
    assert led.cost_basis == 0.0             # INV-2：cost_basis 锁 0


def test_ledger_open_rejects_visible():
    led = OrganicLedger(entry_price=100.0, total_shares=0.0, cost_basis=100.0)
    assert not led.open_diff(2, 0.5, 102.0)  # shares=0 → 拒绝
    assert led.n_open_rejects_zero == 1
    led2 = OrganicLedger(entry_price=100.0, total_shares=10.0, cost_basis=100.0)
    assert led2.open_diff(2, 0.5, 102.0)
    assert not led2.open_diff(2, 0.5, 103.0)  # 槽占用 → 拒绝（不计 zero 计数）
    assert led2.n_open_rejects_zero == 0


def test_ledger_was_earning_decides_law():
    """守恒律由 open 时阶段决定（open 在 cost 阶段、close 时已 earning → 仍股数守恒）。"""
    led = OrganicLedger(entry_price=100.0, total_shares=10.0, cost_basis=1.0,
                        level_frac=1.0)
    led.open_diff(2, 0.5, 200.0)
    led.open_diff(3, 0.5, 200.0)
    led.close_diff(2, 50.0)                  # profit=750 → cost→0 → earning=True
    assert led.earning
    shares_before = led.total_shares
    led.close_diff(3, 50.0)                  # open 于 cost 阶段 → 股数守恒
    assert led.total_shares == shares_before


# ════════════════════════════════════════════════════════════
# 2. FatigueMonitor
# ════════════════════════════════════════════════════════════

def test_fatigue_evidence_and_clear():
    fm = FatigueMonitor()
    # type1 卖 candidate 即证据（E4：左侧信号 > 右侧确认）
    fm.observe(3, (bsp("type1", "sell", 5, False, 10, 95, 99),), (), False)
    assert fm.gate_open(2, 3)
    # 新向上 move settle → 清空（创新动力否定衰竭）
    fm.observe(3, (), (), True)
    assert not fm.gate_open(2, 3)


def test_fatigue_div_evidence_and_buy_ignored():
    fm = FatigueMonitor()
    fm.observe(3, (bsp("type1", "buy", 5, True, 10, 95, 99),), (), False)
    assert not fm.gate_open(2, 3)            # 买侧不是衰竭证据
    fm.observe(3, (), (div("consolidation", "up", "sell", 7),), False)
    assert fm.gate_open(2, 3)                # 盘整背驰卖侧即可（§4e-1）


def test_fatigue_u_resolution_and_cap():
    fm = FatigueMonitor()
    fm.observe(5, (bsp("type1", "sell", 5, True, 10, 95, 99),), (), False)
    assert fm.u_of(2, 5) == 5                # 3/4 无结构 → 最近=5
    assert fm.u_of(2, 4) is None             # 封顶 entry_ladder=4 → 5 不可见
    assert not fm.gate_open(2, 4)            # u 不存在 → 门恒关（§8.3-f）
    fm.observe(3, (bsp("type2", "sell", 6, True, 11, 90, 94),), (), False)
    assert fm.u_of(2, 5) == 3                # type2 卖非证据但算"有结构"


def test_fatigue_type3_needs_confirmed():
    fm = FatigueMonitor()
    fm.observe(3, (bsp("type3", "sell", 5, False, 10, 95, 99),), (), False)
    assert not fm.gate_open(2, 3)            # candidate type3 卖不是证据（§4e-3）
    fm.observe(3, (bsp("type3", "sell", 5, True, 10, 95, 99),), (), False)
    assert fm.gate_open(2, 3)


# ════════════════════════════════════════════════════════════
# 3. SizeAllocator
# ════════════════════════════════════════════════════════════

def test_allocator_amplitude_normalization():
    al = SizeAllocator()
    lc = {2: (10, 95.0, 99.0), 3: (20, 90.0, 102.0)}   # 振幅 4 与 12
    al.maybe_recompute(1, [2, 3], lc, {}, 100.0)
    assert abs(al.frac[2] - 4 / 16) < 1e-12
    assert abs(al.frac[3] - 12 / 16) < 1e-12


def test_allocator_dead_center_zero_and_version_gate():
    al = SizeAllocator()
    lc = {2: (10, 95.0, 99.0), 3: (20, 90.0, 102.0)}
    al.maybe_recompute(1, [2, 3], lc, {3: {20}}, 100.0)
    assert 3 not in al.frac                   # 死中枢 → 无操作权
    assert abs(al.frac[2] - 1.0) < 1e-12
    al.maybe_recompute(1, [2, 3], lc, {}, 100.0)   # 版本未变 → 不重算
    assert 3 not in al.frac
    al.maybe_recompute(2, [2, 3], lc, {2: {10}, 3: {20}}, 100.0)
    assert al.frac == {}                      # Σ=0 → 全 0（无任何声部操作）


# ════════════════════════════════════════════════════════════
# 4. LOU 转换表（经 run_organic 集成驱动——FSM 不脱离账本语义单测）
# ════════════════════════════════════════════════════════════

def _run(tape, cfg, diag=None):
    return run_organic(tape, floor_ladder=LADDER_SEG, config=cfg, diag=diag)


def test_t1_rev_open_on_confirmed_type3_sell_no_gate():
    """O1 形态：confirmed type3 卖触发 T1，门不设 → REV 开；T5 confirmed type1 买回。"""
    tape = entry_prefix() + [
        # confirmed type3 卖 @102 → 中枢 10 死亡 + T1 触发 → OPEN_REV
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        # confirmed type1 买 @96 → T5 CLOSE_REV（diff=+6）
        bar(96.0, ev={2: [bsp("type1", "buy", 7, True, 11, 92.0, 96.0)]}),
        bar(97.0, sell1=(3,)),                # master 出场
    ]
    diag = []
    trades, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=False),
                         diag=diag)
    cnt = extra["counters"]
    assert cnt["n_rev_open"] == 1 and cnt["n_rev_close_t5"] == 1
    revs = [r for r in diag[0]["diffs"] if r["leg"] == "rev"]
    assert len(revs) == 1 and abs(revs[0]["diff"] - 6.0) < 1e-9


def test_t1_div_trigger_and_t2_gate_closed():
    """T1 的 div(up) 触发分量 + T2：门关（无上级证据）→ 域腿照常、REV 不开。"""
    tape = entry_prefix() + [
        bar(102.0, dv={2: [div("consolidation", "up", "sell", 6)]}),
        bar(97.0, sell1=(3,)),
    ]
    _, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=True))
    cnt = extra["counters"]
    assert cnt["n_rev_attempts"] == 1
    assert cnt["n_rev_gate_rejects"] == 1 and cnt["n_rev_open"] == 0


def test_t1_gate_open_via_upper_evidence():
    """O2 形态：上级别（entry=3）type1 卖 candidate 证据 → 门开 → REV 开。"""
    tape = entry_prefix() + [
        # ladder3 衰竭证据（candidate type1 卖——不触发 sell1 布尔，主仓不动）
        bar(101.0, ev={3: [bsp("type1", "sell", 9, False, 30, 90.0, 98.0)]}),
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        bar(96.0, ev={2: [bsp("type1", "buy", 7, True, 11, 92.0, 96.0)]}),
        bar(97.0, sell1=(3,)),
    ]
    _, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=True))
    cnt = extra["counters"]
    assert cnt["n_rev_open"] == 1 and cnt["n_rev_gate_rejects"] == 0


def test_gate_cleared_by_up_move_settle():
    """证据时效：u 层新向上 move settle → 证据清空 → 门拒。"""
    tape = entry_prefix() + [
        bar(101.0, ev={3: [bsp("type1", "sell", 9, False, 30, 90.0, 98.0)]}),
        bar(101.5, ums=(3,)),                 # 创新动力：衰竭被市场否定
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        bar(97.0, sell1=(3,)),
    ]
    _, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=True))
    cnt = extra["counters"]
    assert cnt["n_rev_open"] == 0 and cnt["n_rev_gate_rejects"] == 1


def test_t6_t7_escape_and_freeze():
    """T6 candidate type3 买预逃逸；T7 confirmed type3 买逃逸 + 冻结 → 后续 T1 拒。"""
    base = entry_prefix()
    # T6：REV 开后 candidate type3 买 → 预逃逸
    tape6 = base + [
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        bar(98.0, ev={2: [bsp("type3", "buy", 7, False, 11, 92.0, 96.0)]}),
        bar(97.0, sell1=(3,)),
    ]
    _, e6 = _run(tape6, OrganicConfig(rev_mode=True, rev_gate=False))
    assert e6["counters"]["n_rev_close_t6"] == 1
    # T7：confirmed type3 买 → 逃逸 + center book 冻结 ladder2 →
    # 同一中枢 11 上的下一次 T1 触发被冻结拒
    tape7 = base + [
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        bar(99.0, ev={2: [bsp("type3", "buy", 7, True, 11, 92.0, 96.0)]}),
        bar(103.0, ev={2: [bsp("type1", "sell", 8, True, 11, 92.0, 96.0)]}),
        bar(97.0, sell1=(3,)),
    ]
    _, e7 = _run(tape7, OrganicConfig(rev_mode=True, rev_gate=False))
    cnt = e7["counters"]
    assert cnt["n_rev_close_t7"] == 1
    assert cnt["n_rev_frozen_rejects"] == 1 and cnt["n_rev_open"] == 1


def test_t5_rev_close_ablation_axes():
    """R_cand：type1 买 candidate 即关；R_nested：candidate ∧ 次级别 buy_any。"""
    base = entry_prefix() + [
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
    ]
    cand_bar = bar(96.0, ev={2: [bsp("type1", "buy", 7, False, 11, 92.0, 96.0)]})
    tail = [bar(97.0, sell1=(3,))]
    # conf：candidate 不关腿（出场强平回补，trace 仍有 1 条 rev，但 t5 计数=0）
    _, ec = _run(base + [cand_bar] + tail,
                 OrganicConfig(rev_mode=True, rev_gate=False, rev_close="conf"))
    assert ec["counters"]["n_rev_close_t5"] == 0
    # cand：candidate 即关
    _, ea = _run(base + [cand_bar] + tail,
                 OrganicConfig(rev_mode=True, rev_gate=False, rev_close="cand"))
    assert ea["counters"]["n_rev_close_t5"] == 1
    # nested：candidate 单独不够，须 ∧ buy_any[k−1]
    _, en0 = _run(base + [cand_bar] + tail,
                  OrganicConfig(rev_mode=True, rev_gate=False, rev_close="nested"))
    assert en0["counters"]["n_rev_close_t5"] == 0
    nested_bar = bar(96.0, buy_any=(1,),
                     ev={2: [bsp("type1", "buy", 7, False, 11, 92.0, 96.0)]})
    _, en1 = _run(base + [nested_bar] + tail,
                  OrganicConfig(rev_mode=True, rev_gate=False, rev_close="nested"))
    assert en1["counters"]["n_rev_close_t5"] == 1


def test_t1_closes_osc_first():
    """T1 动作序：先 CLOSE_OSC（若开）再 OPEN_REV。"""
    tape = entry_prefix() + [
        # 域腿开：c=99.5 ≥ ZG=99 ∧ sell_any[1]
        bar(99.5, sell_any=(1,)),
        # T1：confirmed type3 卖 → 先平 osc 再开 REV
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        bar(96.0, ev={2: [bsp("type1", "buy", 7, True, 11, 92.0, 96.0)]}),
        bar(97.0, sell1=(3,)),
    ]
    diag = []
    _, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=False), diag=diag)
    cnt = extra["counters"]
    assert cnt["n_osc_open"] == 1 and cnt["n_rev_open"] == 1
    legs = [(r["leg"], r["buy_bar"]) for r in diag[0]["diffs"]]
    osc_close = next(b for leg, b in legs if leg == "osc")
    assert osc_close == 3                     # osc 在 T1 bar（index 3）被强制平


def test_same_bar_t5_close_then_t1_reopen():
    """同 bar 优先级锁定：T5 关腿（type1 买）先于 T1 开腿（type1 卖）——
    与主腿"平仓 bar 可重开（开/平由不同事件驱动，事件序合法）"一致。
    同 bar 既有 confirmed type1 买又有 confirmed type1 卖 → REV 先关后重开。"""
    tape = entry_prefix() + [
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        # 同 bar：T5 触发（type1 买）+ T1 触发（type1 卖，卖侧背驰亦可）
        bar(96.0, ev={2: [bsp("type1", "buy", 7, True, 11, 92.0, 96.0),
                          bsp("type1", "sell", 8, True, 11, 92.0, 96.0)]}),
        bar(90.0, ev={2: [bsp("type1", "buy", 9, True, 12, 88.0, 91.0)]}),
        bar(97.0, sell1=(3,)),
    ]
    diag: list = []
    _, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=False),
                    diag=diag)
    cnt = extra["counters"]
    assert cnt["n_rev_open"] == 2 and cnt["n_rev_close_t5"] == 2
    rev_rows = [r for r in diag[0]["diffs"] if r["leg"] == "rev"]
    assert len(rev_rows) == 2
    # 第一腿 102→96，第二腿同 bar 重开 96→90
    assert (rev_rows[0]["sell_price"], rev_rows[0]["buy_price"]) == (102.0, 96.0)
    assert (rev_rows[1]["sell_price"], rev_rows[1]["buy_price"]) == (96.0, 90.0)


def test_structure_sizing_no_center_no_leg():
    """O3：sizing=structure 时无存活中枢的层 frac=0 → 开腿被拒（操作权=结构域）。"""
    # 不注册任何中枢（entry_prefix 之外手工构造）——
    # confirmed type3 卖事件本身使中枢 10 同 bar 死亡 → A_2=0 → frac=0
    tape = [
        bar(99.0, buy1=(3,)),
        bar(100.0, buy_any=(0,)),
        bar(102.0, ev={2: [bsp("type3", "sell", 6, True, 10, 95.0, 99.0)]}),
        bar(97.0, sell1=(3,)),
    ]
    _, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=False,
                                        sizing="structure"))
    cnt = extra["counters"]
    assert cnt["n_rev_open"] == 0
    assert cnt["n_open_rejects_zero"] >= 1    # 拒绝可观测（非静默）


# ════════════════════════════════════════════════════════════
# 5. O0≡P5（合成磁带逐笔+逐 trace 对账；全量真实磁带见回测脚本 O0 步）
# ════════════════════════════════════════════════════════════

def _p5_scenario_tape():
    """覆盖 P5 全机制的合成序列：main 腿开/同锚回补、门拒、osc 开/ZD 平/
    中枢死亡强平、type3 预回补/硬回补+冻结/解冻、master 出场、eod。"""
    t = entry_prefix()
    t += [
        # main 开：confirmed type1 卖 @103 > ZG=99，振幅 3.9%≥θ
        bar(103.0, ev={2: [bsp("type1", "sell", 6, True, 10, 95.0, 99.0)]}),
        # 门拒：confirmed type2 卖但 c≤ZG
        bar(98.0, ev={2: [bsp("type2", "sell", 7, True, 10, 95.0, 99.0)]}),
        # osc 开：c≥ZG ∧ sell_any[1]
        bar(99.2, sell_any=(1,)),
        # main 同锚回补：confirmed type1 买 cs=10
        bar(96.0, ev={2: [bsp("type1", "buy", 8, True, 10, 95.0, 99.0)]}),
        # osc ZD 平：c ≤ 95
        bar(94.5),
        # 新中枢 11 注册 + main 再开
        bar(104.0, ev={2: [bsp("type1", "sell", 9, True, 11, 96.0, 100.0)]}),
        # candidate type3 买 → 预回补
        bar(101.0, ev={2: [bsp("type3", "buy", 10, False, 11, 96.0, 100.0)]}),
        # main 三开 + osc 再开同 bar
        bar(104.5, sell_any=(1,),
            ev={2: [bsp("type2", "sell", 11, True, 11, 96.0, 100.0)]}),
        # confirmed type3 买 → main 硬回补 + osc 中枢死亡强平 + 冻结
        bar(102.0, ev={2: [bsp("type3", "buy", 12, True, 11, 96.0, 100.0)]}),
        # 冻结中：新卖点不开腿
        bar(105.0, ev={2: [bsp("type1", "sell", 13, True, 11, 96.0, 100.0)]}),
        # 新存活中枢 14 → 解冻 + main 开
        bar(106.0, ev={2: [bsp("type1", "sell", 14, True, 14, 99.0, 103.0)]}),
        # master 出场（main 腿出场价强平回补）
        bar(101.0, sell1=(3,)),
        # 第二笔交易：ARM → 超时入场 → eod 强平
        bar(100.0, buy1=(2,), max_ladder=3),
    ]
    t += [bar(100.0 + 0.01 * j) for j in range(70)]   # SUB_EXPIRY 超时入场
    t += [bar(108.0)]
    return t


def test_o0_equals_p5_synthetic():
    tape = _p5_scenario_tape()
    diag_p5: list = []
    trades_p5, extra_p5 = run_version_i(
        tape, floor_ladder=LADDER_SEG, pairing=PAIRING_VARIANTS["P5"],
        diag=diag_p5)
    diag_o0: list = []
    trades_o0, extra_o0 = run_organic(
        tape, floor_ladder=LADDER_SEG, config=OrganicConfig(), diag=diag_o0)
    assert trades_p5 == trades_o0, (trades_p5, trades_o0)
    assert len(trades_p5) >= 2                # 场景确实产生了交易
    # trace 逐字段对账（organic 多出 "leg" 归因键，比较公共键）
    assert len(diag_p5) == len(diag_o0)
    n_diffs = 0
    for a, b in zip(diag_p5, diag_o0):
        assert a["n_diffs"] == b["n_diffs"] and a["pnl_pct"] == b["pnl_pct"]
        for ra, rb in zip(a["diffs"], b["diffs"]):
            for key in ra:
                assert ra[key] == rb[key], (key, ra, rb)
            n_diffs += 1
    assert n_diffs >= 5                       # 场景覆盖了多条短差
    # P5 计数器 ↔ organic 计数器对应
    pc = extra_p5["pairing"]; oc = extra_o0["counters"]
    for k in ("n_close_normal", "n_close_pre_type3", "n_close_hard_type3",
              "n_open_gate_rejects", "n_osc_open", "n_osc_zd_close"):
        assert pc[k] == oc[k], (k, pc[k], oc[k])


# ════════════════════════════════════════════════════════════
# 6. earning 反作用（O4，§5.6）
# ════════════════════════════════════════════════════════════

def test_earning_reaction_master_rev_and_upgraded_exit():
    """cost→0 后 master 段终结降格为 entry 级 REV（金额守恒挣股数），
    清仓权上移到 entry+1 级 type1 卖。"""
    tape = entry_prefix(entry_close=100.0) + [
        # 巨幅域腿把 cost_basis 打到 0：osc 开 @200
        bar(200.0, sell_any=(1,)),
        # osc ZD 平 @94（diff=106 > cost 100 → earning）
        bar(94.0),
        # master 段终结（sell1[3]）→ 不清仓，开 entry 级 REV @150
        bar(150.0, sell1=(3,), max_ladder=4),
        # T5：entry 级 confirmed type1 买 @100 → 金额守恒回补（股数 ×1.5）
        bar(100.0, ev={3: [bsp("type1", "buy", 9, True, 30, 90.0, 98.0)]},
            max_ladder=4),
        # 升级出场：entry+1=4 级 confirmed type1 卖
        bar(160.0, sell1=(4,), max_ladder=4),
    ]
    diag: list = []
    trades, extra = _run(
        tape, OrganicConfig(rev_mode=True, rev_gate=False,
                            earning_reaction=True), diag=diag)
    cnt = extra["counters"]
    assert cnt["n_earning_reached"] == 1
    assert cnt["n_master_rev_open"] == 1 and cnt["n_master_rev_close"] == 1
    assert cnt["n_exit_upgraded"] == 1
    assert trades[0].exit_reason == "exit_recL2_earning_upgrade"
    # 挣股数：REV 腿金额守恒回补 shares×sell/buy = shares×1.5
    # （腿生命周期净增 = ×1.5 − ×1.0 = +50%，43课程式）
    rev_rows = [r for r in diag[0]["diffs"] if r["leg"] == "rev"]
    assert any(abs(r["shares_delta"] - r["shares"] * 1.5) < 1e-9
               for r in rev_rows if r["was_earning"])
    # 升级出场收益含增股：10股@100 本金 → 15股@160 + 落袋
    assert trades[0].pnl_pct > 100


def test_earning_reaction_off_master_closes():
    """earning_reaction=False（O1-O3）：earning 后 master 段终结仍正常清仓。"""
    tape = entry_prefix(entry_close=100.0) + [
        bar(200.0, sell_any=(1,)),
        bar(94.0),
        bar(150.0, sell1=(3,)),
    ]
    trades, extra = _run(tape, OrganicConfig(rev_mode=True, rev_gate=False))
    assert extra["counters"]["n_master_rev_open"] == 0
    assert len(trades) == 1
    assert trades[0].exit_reason == "exit_move(L1)_type1sell"


# ════════════════════════════════════════════════════════════
# 7. 能力守卫（声明=能力）
# ════════════════════════════════════════════════════════════

def test_capability_guards():
    import pytest
    # 布尔磁带（bsp_events=() 默认——旧信号层构造方，如 m1_i_rust_engine）
    boolean_tape = [BarSignalI(
        close=100.0, buy1=_mask(()), sell1=_mask(()), sell_any=_mask(()),
        buy_any=_mask(()), max_ladder=3, type2_buy=False)] * 5
    with pytest.raises(ValueError, match="事件磁带"):
        run_organic(boolean_tape, config=OrganicConfig())
    ev_tape = entry_prefix() + [bar(100.0)]
    with pytest.raises(ValueError, match="背驰磁带"):
        # bsp_events 有但 div_events 全空 → rev_mode 拒绝
        tape2 = [BarSignalI(
            close=s.close, buy1=s.buy1, sell1=s.sell1, sell_any=s.sell_any,
            buy_any=s.buy_any, max_ladder=s.max_ladder, type2_buy=s.type2_buy,
            bsp_events=s.bsp_events) for s in ev_tape]
        run_organic(tape2, config=OrganicConfig(rev_mode=True))
    with pytest.raises(NotImplementedError):
        run_organic(ev_tape, config=OrganicConfig(market_mode="futures"))
    with pytest.raises(ValueError, match="floor_ladder"):
        run_organic(ev_tape, floor_ladder=0, config=OrganicConfig())


if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([__file__, "-q"]))
