"""#1312 状态门控第二预注册——门列消费面 + 状态闸的行为锁。

覆盖（判据一律在 Rust 生产函数侧，本套只锁"读列 + 进场准入 + 接缝校验"）：
  1. v3 门列读写往返 + 接缝校验（bar 数 / 首尾 close / magic / 长度）；
  2. 状态闸拦截语义：空腿拒于 l2_up_unexhausted、多腿拒于 l2_down_unexhausted；
  3. 门 sanity：unexhausted=true 区间内无逆势单（#1312 要做 ④ 的机械形态）；
  4. `gate=None` 与 #1309 第一轮逐位等价（无门路径零行为变化）；
  5. fatigue 可选臂：列不可用时开臂 fail-fast（不代理，`fatigue_gate.rs` 头部明令）。

Rust 侧对应锁：`rust/src/trading/gate_state_dump.rs` 的 tests 模块
（列 = 生产函数读数、趋势延续段恒 true、能力缺失哨兵、dir 行缺失 fail-fast）。
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

gate_state_columns = pytest.importorskip(
    "gate_state_columns", reason="analysis/gate_state_columns.py 不可导入"
)
F = pytest.importorskip(
    "fugue_alpha_diagnosis",
    reason="fugue_alpha_diagnosis 依赖 numpy/newchan（未装则跳过）",
)

G = gate_state_columns


def write_gate_file(
    path: Path,
    up: list[bool],
    dn: list[bool],
    fat: list[int],
    closes: list[float],
    ladder: int = 4,
    fatigue_available: bool = False,
    magic: int = G.MAGIC_V3G,
) -> None:
    """按 gate_state_dump.rs 的落盘格式写一个合成门列文件。"""
    n = len(up)
    blob = struct.pack(
        G.HEADER_FMT, magic, n, ladder, int(fatigue_available), closes[0], closes[-1]
    )
    blob += bytes(int(v) for v in up)
    blob += bytes(int(v) for v in dn)
    blob += bytes(fat)
    path.write_bytes(blob)


def make_gate(
    up: list[bool],
    dn: list[bool],
    fat: list[int] | None = None,
    *,
    fatigue_available: bool = False,
    use_fatigue: bool = False,
) -> G.StateGate:
    return G.StateGate(
        symbol="SYN",
        ladder=4,
        up_unexhausted=tuple(up),
        down_unexhausted=tuple(dn),
        fatigue=tuple(fat if fat is not None else [G.FATIGUE_UNAVAILABLE] * len(up)),
        fatigue_available=fatigue_available,
        use_fatigue=use_fatigue,
    )


def sig(
    close: float,
    *,
    down_settled: bool = False,
    up_settled: bool = False,
    entry_ok: bool = False,
    exit_ok: bool = False,
    flip_short: bool = False,
    flip_long: bool = False,
) -> "F.BarSignal":
    return F.BarSignal(
        close=close,
        l0_r1_down=False, l0_r1_up=False,
        l1_nr1_up=False, l1_nr1_down=False,
        l2_flip_long=flip_long, l2_flip_short=flip_short,
        l2_direction=0,
        buy_cands=(), sell_cands=(), buy_invalidates=(),
        new_up_moves=(),
        med_persistence=0.0,
        l1_up_ratio=0.0, refined_gate_ok=False,
        entry_div_ok=entry_ok,
        exit_div_ok=exit_ok,
        down_move_settled=down_settled,
        up_move_settled=up_settled,
    )


# ── 1. 门列读写往返 + 接缝校验 ───────────────────────────────────────────

def test_gate_columns_roundtrip(tmp_path, monkeypatch):
    closes = [10.0, 10.5, 11.0, 10.2]
    up = [True, True, False, False]
    dn = [False, False, True, True]
    fat = [G.FATIGUE_UNAVAILABLE] * 4
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(G.gate_path("SYN"), up, dn, fat, closes)

    gate = G.load_state_gate("SYN", closes)
    assert gate.ladder == 4
    assert list(gate.up_unexhausted) == up
    assert list(gate.down_unexhausted) == dn
    assert gate.fatigue_available is False
    assert len(gate) == 4
    rates = G.gate_open_rates(gate)
    assert rates["up_unexhausted_bars"] == 2
    assert rates["down_unexhausted_bars"] == 2
    assert rates["fatigued_bars"] is None


@pytest.mark.parametrize(
    "mutate,msg",
    [
        ("bars", "bar 数"),
        ("close", "首尾 close"),
        ("magic", "magic"),
    ],
)
def test_gate_columns_seam_checks_fail_fast(tmp_path, monkeypatch, mutate, msg):
    closes = [10.0, 10.5, 11.0, 10.2]
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    kwargs = {}
    if mutate == "magic":
        kwargs["magic"] = 0xDEADBEEF
    write_gate_file(
        G.gate_path("SYN"),
        [True] * 4, [False] * 4, [G.FATIGUE_UNAVAILABLE] * 4, closes, **kwargs,
    )
    # bars 用例取**两端都不锚**的驱动切片（closes[1:3]）：#1314 起 bar 数不等时，
    # 首/末 close 任一锚定即按位移截取（对齐臂锁在 tests/test_gate_bar_alignment.py），
    # 只有两端都不锚才 fail-fast——本用例锁的正是后者。
    drive = closes[1:3] if mutate == "bars" else (
        [99.0] + closes[1:] if mutate == "close" else closes
    )
    with pytest.raises(ValueError, match=msg):
        G.load_state_gate("SYN", drive)


def test_missing_gate_file_names_regen_command(tmp_path, monkeypatch):
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    with pytest.raises(FileNotFoundError, match="gate_state_dump_prereg7"):
        G.load_state_gate("SYN", [1.0, 2.0])


# ── 2/3. 状态闸拦截语义 + 门 sanity ──────────────────────────────────────

def test_short_entry_rejected_inside_up_unexhausted_window():
    """空腿进场信号落在 l2_up_unexhausted=true ⇒ 被拒；出场判据零改动。"""
    signals = [
        sig(10.0),
        # bar1：空腿进场信号（up_move_settled ∧ exit_div_ok），但 up 未衰竭 ⇒ 拒
        sig(11.0, up_settled=True, exit_ok=True),
        # bar2：同样的进场信号，门已开 ⇒ 放行
        sig(12.0, up_settled=True, exit_ok=True),
        # bar3：回补（l2_flip_long ∧ entry_div_ok）
        sig(9.0, flip_long=True, entry_ok=True),
    ]
    gate = make_gate(up=[True, True, False, False], dn=[False] * 4)
    shorts = F._run_swing_leg(signals, "short", gate)
    assert [t.entry_bar for t in shorts] == [2], "空腿只应在门开后进场"
    assert shorts[0].exit_bar == 3
    # 门 sanity：无任何进场落在 up_unexhausted=true 区间
    assert all(not gate.up_unexhausted[t.entry_bar] for t in shorts)


def test_long_entry_rejected_inside_down_unexhausted_window():
    signals = [
        sig(10.0),
        sig(9.0, down_settled=True, entry_ok=True),   # down 未衰竭 ⇒ 拒
        sig(8.0, down_settled=True, entry_ok=True),   # 门开 ⇒ 放行
        sig(11.0, flip_short=True, exit_ok=True),     # L2 顶背驰离场
    ]
    gate = make_gate(up=[False] * 4, dn=[True, True, False, False])
    longs = F._run_swing_leg(signals, "long", gate)
    assert [t.entry_bar for t in longs] == [2]
    assert all(not gate.down_unexhausted[t.entry_bar] for t in longs)


def test_gate_blocks_all_entries_when_window_never_opens():
    """整段趋势延续（unexhausted 恒 true）⇒ 逆势腿零单（门真的在拦）。"""
    signals = [sig(10.0 + i, up_settled=True, exit_ok=True) for i in range(6)]
    gate = make_gate(up=[True] * 6, dn=[False] * 6)
    assert F._run_swing_leg(signals, "short", gate) == []


# ── 4. 无门路径与第一轮逐位等价 ─────────────────────────────────────────

def test_ungated_dual_is_bit_identical_to_round1():
    signals = [
        sig(10.0),
        sig(9.0, down_settled=True, entry_ok=True),
        sig(11.0, up_settled=True, exit_ok=True),
        sig(12.0, flip_short=True, exit_ok=True),
        sig(8.0, flip_long=True, entry_ok=True),
    ]
    baseline_long, _ = F.run_swing_trading(signals, F.MODE_NONE)
    dual_long, dual_short = F.run_swing_trading_dual(signals)
    key = lambda ts: [  # noqa: E731
        (t.entry_bar, t.entry_price, t.exit_bar, t.exit_price, t.pnl_pct, t.exit_reason)
        for t in ts
    ]
    assert key(dual_long) == key(baseline_long), "gate=None 长腿须与 MODE_NONE 逐位等价"
    # 双跑逐位自检（确定性）
    again_long, again_short = F.run_swing_trading_dual(signals)
    assert key(again_long) == key(dual_long)
    assert key(again_short) == key(dual_short)


# ── 5. fatigue 可选臂 ───────────────────────────────────────────────────

def test_fatigue_arm_fail_fast_when_column_unavailable():
    with pytest.raises(RuntimeError, match="run_high"):
        make_gate(up=[False], dn=[False], use_fatigue=True)


def test_fatigue_arm_requires_fatigued_state_for_short_entry():
    """列可用时：空腿进场额外要求处于衰竭段内（Fresh ⇒ 拒）。"""
    gate = make_gate(
        up=[False, False],
        dn=[False, False],
        fat=[G.FATIGUE_FRESH, G.FATIGUE_FATIGUED],
        fatigue_available=True,
        use_fatigue=True,
    )
    assert gate.short_entry_allowed(0) is False
    assert gate.short_entry_allowed(1) is True
    # 多腿不受 fatigue 臂影响（生产侧无下跌侧镜像门，不另造判据）
    assert gate.long_entry_allowed(0) is True
    # 臂关闭 ⇒ 只看 unexhausted
    assert gate.with_fatigue(False).short_entry_allowed(0) is True


# ── 门 sanity 辅助 ──────────────────────────────────────────────────────

def test_longest_unexhausted_window():
    assert G.longest_unexhausted_window((False, True, True, False, True)) == (1, 3)
    assert G.longest_unexhausted_window((True, True, True)) == (0, 3)
    assert G.longest_unexhausted_window((False, False)) == (0, 0)
