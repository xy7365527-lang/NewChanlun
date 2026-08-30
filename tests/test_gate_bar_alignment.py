"""#1314/#1315 门列/驱动 bar 对齐——对齐诊断 + `load_state_gate` 各对齐臂的行为锁。

覆盖：
  1. 两侧清洗口径复刻锁：驱动只剔 nan（zip 截最短）/ dump 另剔 ≤0 ⇒ dump ⊆ 驱动；
  2. `align_report` 的六个 verdict（头/尾/内部/混合/驱动超集/互不包含）+ 时间戳定位；
  3. `load_state_gate` 的两条截取臂**逐 bar 值正确**（三列各自按位移切片——整体平移会
     跨列错位，#1312 首版即此错，本套是它的回归锁）；
  4. 两端都不锚 ⇒ fail-fast，且错误信息带诊断命令；
  5. #1315 口径差重建臂：门列少 bar 时按"四价全正"逐位映射回驱动坐标，被 dump 剔掉的
     bar fail-closed（多空腿都拒）；数目/两端 close 核不上即 fail-fast；诊断件把这种
     少-bar 与"门列过期"分开判。

本套只依赖标准库（`gate_state_columns` / `_diag_gate_bar_alignment` 均无重依赖），
沙盒无 numpy/newchan_rust 时同样跑得动。
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "analysis"))

G = pytest.importorskip("gate_state_columns")
D = pytest.importorskip("_diag_gate_bar_alignment")

NAN = float("nan")


def write_gate_file(path: Path, up, dn, fat, first_c, last_c, ladder=4) -> None:
    """按 `gate_state_dump.rs::write_gate_columns` 的格式写合成门列。"""
    n = len(up)
    blob = struct.pack(G.HEADER_FMT, G.MAGIC_V3G, n, ladder, 0, first_c, last_c)
    blob += bytes(int(v) for v in up)
    blob += bytes(int(v) for v in dn)
    blob += bytes(fat)
    path.write_bytes(blob)


# ── 1. 两侧清洗口径 ──────────────────────────────────────────────────────

def test_dump_kept_is_subset_of_driver_kept():
    """dump 口径（nan + ≤0）的保留集必是驱动口径（仅 nan）的子集——#1314 核对结论①。"""
    o = [1.0, 2.0, NAN, 4.0, 5.0]
    h = [1.0, 2.0, 3.0, 4.0, 5.0]
    lo = [1.0, 0.0, 3.0, 4.0, 5.0]   # idx1 的 low=0 ⇒ 只被 dump 剔
    c = [1.0, 2.0, 3.0, 4.0, 5.0]
    d_kept = D.driver_kept(o, h, lo, c)
    p_kept = D.dump_kept(o, h, lo, c)
    assert d_kept == [0, 1, 3, 4]
    assert p_kept == [0, 3, 4]
    assert set(p_kept) <= set(d_kept)


def test_driver_zip_truncates_to_shortest_array():
    """驱动侧 `zip` 截到最短数组，dump 侧按 closes 长度遍历——数组不齐时的口径差。"""
    o = [1.0, 2.0]
    h = [1.0, 2.0, 3.0]
    lo = [1.0, 2.0, 3.0]
    c = [1.0, 2.0, 3.0]
    assert D.driver_kept(o, h, lo, c) == [0, 1]
    with pytest.raises(IndexError):  # dump 侧越界 ⇒ 不齐即崩，不静默
        D.dump_kept(o, h, lo, c)


# ── 2. align_report 六个 verdict ─────────────────────────────────────────

def test_align_report_equal():
    ts = [100, 160, 220]
    assert D.align_report(ts, ts)["verdict"] == "EQUAL"


def test_align_report_head_excess_locates_extra_bars():
    gate = [100, 160, 220, 280]
    rep = D.align_report(gate, [220, 280])
    assert rep["verdict"] == "HEAD_EXCESS"
    assert [r["ts"] for r in rep["extra_head"]] == [100, 160]
    assert rep["extra_tail"] == [] and rep["extra_interior"] == []
    assert rep["delta"] == 2


def test_align_report_tail_excess_locates_extra_bars():
    """#1314 DX 的候选形态：门列末 close 不锚 + n 多出 ⇒ 多出的含尾部。"""
    gate = [100, 160, 220, 280, 340, 400]
    rep = D.align_report(gate, [100, 160, 220, 280])
    assert rep["verdict"] == "TAIL_EXCESS"
    assert [r["gate_idx"] for r in rep["extra_tail"]] == [4, 5]
    assert rep["extra_head"] == [] and rep["extra_interior"] == []


def test_align_report_interior_and_mixed():
    assert D.align_report([100, 160, 220], [100, 220])["verdict"] == "INTERIOR_EXCESS"
    mixed = D.align_report([100, 160, 220, 280], [160, 220])
    assert mixed["verdict"] == "MIXED_EXCESS"


def test_align_report_driver_superset_and_not_nested():
    assert D.align_report([160], [100, 160, 220])["verdict"] == "DRIVER_SUPERSET"
    assert D.align_report([100, 999], [100, 160])["verdict"] == "NOT_NESTED"


# ── 3. load_state_gate 截取臂（逐 bar 值正确） ───────────────────────────

def test_tail_anchored_arm_drops_head_bars_per_column(tmp_path, monkeypatch):
    """门列末 close 锚定 ⇒ 舍弃头部；三列必须各自按位移切片（整体平移会跨列错位）。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    up = [1, 1, 0, 1, 0]
    dn = [0, 1, 1, 0, 1]
    fat = [G.FATIGUE_UNAVAILABLE] * 5
    write_gate_file(G.gate_path("SYN"), up, dn, fat, first_c=10.0, last_c=14.0)

    gate = G.load_state_gate("SYN", [12.0, 13.0, 14.0])  # 末 close 锚定，舍头 2 根
    assert list(gate.up_unexhausted) == [False, True, False]
    assert list(gate.down_unexhausted) == [True, False, True]
    assert len(gate) == 3


def test_head_anchored_arm_drops_tail_bars_per_column(tmp_path, monkeypatch):
    """门列首 close 锚定、末不锚 ⇒ 舍弃尾部（#1314 DX 形态）。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    up = [1, 1, 0, 1, 0]
    dn = [0, 1, 1, 0, 1]
    fat = [G.FATIGUE_UNAVAILABLE] * 5
    write_gate_file(G.gate_path("SYN"), up, dn, fat, first_c=10.0, last_c=14.0)

    gate = G.load_state_gate("SYN", [10.0, 11.0, 12.0])  # 首 close 锚定，舍尾 2 根
    assert list(gate.up_unexhausted) == [True, True, False]
    assert list(gate.down_unexhausted) == [False, True, True]
    assert len(gate) == 3


def test_exact_length_still_checks_both_ends(tmp_path, monkeypatch):
    """bar 数相等 ⇒ 首末两端都核（#1312 原行为不变）。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(
        G.gate_path("SYN"), [1, 0, 1], [0, 1, 0], [G.FATIGUE_UNAVAILABLE] * 3,
        first_c=10.0, last_c=12.0,
    )
    with pytest.raises(ValueError, match="首尾 close"):
        G.load_state_gate("SYN", [10.0, 11.0, 99.0])


# ── 4. 两端都不锚 ⇒ fail-fast ───────────────────────────────────────────

def test_unanchored_length_mismatch_names_diag_command(tmp_path, monkeypatch):
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(
        G.gate_path("SYN"), [1, 0, 1, 0], [0, 1, 0, 1], [G.FATIGUE_UNAVAILABLE] * 4,
        first_c=10.0, last_c=13.0,
    )
    with pytest.raises(ValueError) as e:
        G.load_state_gate("SYN", [11.0, 12.0])  # 首末都不锚
    assert "bar 数" in str(e.value)
    assert "_diag_gate_bar_alignment.py" in str(e.value)


def test_both_ends_anchored_with_length_mismatch_is_fail_fast(tmp_path, monkeypatch):
    """首末都锚定但 bar 数不等 ⇒ 多出的在内部，截哪端都错位 ⇒ 拒绝对齐（不静默截头）。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(
        G.gate_path("SYN"), [1, 0, 1, 0], [0, 1, 0, 1], [G.FATIGUE_UNAVAILABLE] * 4,
        first_c=10.0, last_c=13.0,
    )
    with pytest.raises(ValueError) as e:
        G.load_state_gate("SYN", [10.0, 11.5, 13.0])
    assert "内部" in str(e.value)


def test_gate_shorter_than_driver_is_fail_fast(tmp_path, monkeypatch):
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(
        G.gate_path("SYN"), [1, 0], [0, 1], [G.FATIGUE_UNAVAILABLE] * 2,
        first_c=10.0, last_c=11.0,
    )
    with pytest.raises(ValueError, match="bar 数"):
        G.load_state_gate("SYN", [10.0, 11.0, 12.0])


def test_gate_shorter_than_driver_reason_is_not_self_contradictory(tmp_path, monkeypatch):
    """门列少 bar 且首 close 锚定：拒绝理由须说"门列窗口不足"，不得谎报"两端都不锚"。

    首 close 明明锚定（10.0 == 10.0，同一条信息里就打着），旧文案却断言"首末 close
    都不锚定"——照该文案排查会走去查数据源不同源，而真因是门列窗口比驱动短。
    """
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(
        G.gate_path("SYN"), [1, 0], [0, 1], [G.FATIGUE_UNAVAILABLE] * 2,
        first_c=10.0, last_c=11.0,
    )
    with pytest.raises(ValueError) as e:
        G.load_state_gate("SYN", [10.0, 11.0, 12.0])
    assert "门列 bar 数少于驱动" in str(e.value)
    assert "都不锚定" not in str(e.value)


# ── 4b. #1315 口径差重建臂（门列少 bar = dump 另剔 ≤0） ──────────────────

def _write_gate_for_kept(path: Path, up, dn, fat, closes_kept) -> None:
    write_gate_file(path, up, dn, fat, first_c=closes_kept[0], last_c=closes_kept[-1])


def test_nonpositive_arm_maps_gate_readings_onto_driver_bars(tmp_path, monkeypatch):
    """驱动 5 根、其中 2 根 OHLC ≤0 被 dump 剔 ⇒ 门列 3 根按四价全正逐位落回驱动坐标。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    opens = [10.0, 0.0, 12.0, 13.0, 14.0]   # idx1 open=0
    highs = [10.0, 11.0, 12.0, 13.0, 14.0]
    lows = [10.0, 11.0, 12.0, -1.0, 14.0]   # idx3 low<0
    closes = [10.0, 11.0, 12.0, 13.0, 14.0]
    # 门列 = dump 侧保留的 3 根（driver idx 0/2/4）。
    _write_gate_for_kept(
        G.gate_path("SYN"), [1, 0, 0], [0, 0, 1],
        [G.FATIGUE_FRESH, G.FATIGUE_FATIGUED, G.FATIGUE_FRESH], [10.0, 12.0, 14.0],
    )
    gate = G.load_state_gate("SYN", closes, opens=opens, highs=highs, lows=lows)

    assert len(gate) == 5
    assert gate.no_reading_bars == (1, 3)
    # 读数落在 0/2/4；无读数的 1/3 两向都 fail-closed（True ⇒ 多空腿都拒）。
    assert list(gate.up_unexhausted) == [True, True, False, True, False]
    assert list(gate.down_unexhausted) == [False, True, False, True, True]
    assert list(gate.fatigue) == [
        G.FATIGUE_FRESH, G.FATIGUE_UNAVAILABLE, G.FATIGUE_FATIGUED,
        G.FATIGUE_UNAVAILABLE, G.FATIGUE_FRESH,
    ]
    for i in gate.no_reading_bars:
        assert gate.long_entry_allowed(i) is False
        assert gate.short_entry_allowed(i) is False


def test_nonpositive_arm_counts_no_reading_bars_in_open_rates(tmp_path, monkeypatch):
    """门开率读数须单列无读数 bar 数——fail-closed 填充值不是生产函数读数。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    opens, highs = [10.0, 0.0, 12.0], [10.0, 11.0, 12.0]
    lows, closes = [10.0, 11.0, 12.0], [10.0, 11.0, 12.0]
    _write_gate_for_kept(
        G.gate_path("SYN"), [1, 0], [0, 0], [G.FATIGUE_UNAVAILABLE] * 2, [10.0, 12.0],
    )
    rates = G.gate_open_rates(
        G.load_state_gate("SYN", closes, opens=opens, highs=highs, lows=lows)
    )
    assert rates["n_bars"] == 3 and rates["no_reading_bars"] == 1
    assert rates["up_unexhausted_bars"] == 2  # 读数 1 根 + 填充 1 根


def test_nonpositive_arm_rejects_when_count_does_not_reconcile(tmp_path, monkeypatch):
    """门列少的根数 ≠ 四价全正的根数 ⇒ 差不止 ≤0 一条（vintage 不同），拒绝重建。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    opens = highs = lows = closes = [10.0, 11.0, 12.0, 13.0]  # 一根 ≤0 都没有
    _write_gate_for_kept(
        G.gate_path("SYN"), [1, 0], [0, 1], [G.FATIGUE_UNAVAILABLE] * 2, [10.0, 11.0],
    )
    with pytest.raises(ValueError) as e:
        G.load_state_gate("SYN", closes, opens=opens, highs=highs, lows=lows)
    assert "四价全正" in str(e.value)
    assert "_diag_gate_bar_alignment.py" in str(e.value)


def test_nonpositive_arm_rejects_when_no_bar_is_all_positive(tmp_path, monkeypatch):
    """驱动侧一根四价全正的 bar 都没有（门列必空）⇒ 拒绝，且理由不自相矛盾（非 "0 ≠ 0"）。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    opens = highs = lows = closes = [-1.0, 0.0]   # 全部含 ≤0 ⇒ dump 一根都留不下
    write_gate_file(
        G.gate_path("SYN"), [], [], [], first_c=0.0, last_c=0.0,
    )
    with pytest.raises(ValueError) as e:
        G.load_state_gate("SYN", closes, opens=opens, highs=highs, lows=lows)
    assert "无一根四价全正" in str(e.value)
    assert "0 ≠ 0" not in str(e.value)


def test_nonpositive_arm_rejects_when_ends_do_not_anchor(tmp_path, monkeypatch):
    """根数对得上但两端 close 不锚 ⇒ 不同源/不同窗，拒绝（数目相等不足以认同源）。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    opens = [10.0, 0.0, 12.0]           # idx1 被 dump 剔 ⇒ 四价全正的是 idx0/idx2
    highs = lows = closes = [10.0, 11.0, 12.0]
    _write_gate_for_kept(
        G.gate_path("SYN"), [1, 0], [0, 1], [G.FATIGUE_UNAVAILABLE] * 2, [99.0, 12.0],
    )
    with pytest.raises(ValueError, match="两端不锚"):
        G.load_state_gate("SYN", closes, opens=opens, highs=highs, lows=lows)


def test_short_gate_without_ohlc_still_fails_fast_and_names_the_arm(tmp_path, monkeypatch):
    """未传 OHLC 时门列少 bar 仍 fail-fast（closes 单列判不出谁因 ≤0 被剔），并指路重建臂。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(
        G.gate_path("SYN"), [1, 0], [0, 1], [G.FATIGUE_UNAVAILABLE] * 2,
        first_c=10.0, last_c=12.0,
    )
    with pytest.raises(ValueError) as e:
        G.load_state_gate("SYN", [10.0, 11.0, 12.0])
    assert "门列 bar 数少于驱动" in str(e.value)
    assert "opens/highs/lows" in str(e.value)


def test_aligned_symbols_report_zero_no_reading_bars(tmp_path, monkeypatch):
    """bar 数一致的 6 品种路径：传不传 OHLC 都走原臂，无读数 bar 数为 0（#1315 零影响）。"""
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    write_gate_file(
        G.gate_path("SYN"), [1, 0, 1], [0, 1, 0], [G.FATIGUE_UNAVAILABLE] * 3,
        first_c=10.0, last_c=12.0,
    )
    closes = [10.0, 11.0, 12.0]
    plain = G.load_state_gate("SYN", closes)
    withl = G.load_state_gate(
        "SYN", closes, opens=closes, highs=closes, lows=closes,
    )
    assert plain == withl
    assert plain.no_reading_bars == ()
    assert G.gate_open_rates(plain)["no_reading_bars"] == 0


# ── 5. diagnose 端到端（合成 json + 门列 + ts 边车） ─────────────────────

def test_diagnose_end_to_end_tail_excess(tmp_path, monkeypatch):
    """合成一份"门列比驱动多 2 根尾部 bar"的落盘，诊断须定位到尾部并给出处置。"""
    monkeypatch.setattr(D, "DATA_DIR", tmp_path)
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    monkeypatch.setitem(D.SYMBOL_FILES, "DX", tmp_path / "dx.json")

    dates = ["2020-01-01T00:00:00", "2020-01-01T00:01:00", "2020-01-01T00:02:00"]
    (tmp_path / "dx.json").write_text(json.dumps({
        "dates": dates,
        "opens": [10.0, 11.0, 12.0], "highs": [10.0, 11.0, 12.0],
        "lows": [10.0, 11.0, 12.0], "closes": [10.0, 11.0, 12.0],
    }))
    # 门列/边车 = 驱动 3 根 + 尾部多 2 根（磁带 vintage 比 json 多覆盖两根）。
    gate_ts = [D.to_epoch(s) for s in dates] + [D.to_epoch("2020-01-01T00:03:00"),
                                                D.to_epoch("2020-01-01T00:04:00")]
    write_gate_file(
        G.gate_path("DX"), [1, 1, 0, 0, 1], [0, 0, 1, 1, 0],
        [G.FATIGUE_UNAVAILABLE] * 5, first_c=10.0, last_c=14.0,
    )
    D.ts_sidecar_path("DX").write_bytes(struct.pack(f"<{len(gate_ts)}q", *gate_ts))

    d = D.diagnose("DX")
    assert d["driver_n"] == 3 and d["dump_n"] == 3
    assert d["gate_n_minus_driver_n"] == 2
    assert d["first_close_anchors"] is True and d["last_close_anchors"] is False
    assert d["align"]["verdict"] == "TAIL_EXCESS"
    assert len(d["align"]["extra_tail"]) == 2
    assert d["align"]["ts_sidecar_n_matches_gate_n"] is True
    text = D.format_report(d)
    assert "TAIL_EXCESS" in text and "截尾对齐" in text
    # 同一份 json 上 dump ⊆ 驱动，却比驱动多 bar ⇒ 必须点名 vintage 不同。
    assert "vintage" in text


def test_diagnose_missing_inputs_is_blocked_report(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(D, "DATA_DIR", tmp_path)
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    monkeypatch.setitem(D.SYMBOL_FILES, "DX", tmp_path / "absent.json")
    assert D.main(["DX"]) == 2
    assert "卡点" in capsys.readouterr().out


def _write_diag_inputs(tmp_path, monkeypatch, *, arrays, gate_ts, first_c, last_c):
    """诊断件的三件输入（json + 门列 + ts 边车）合成落盘。"""
    monkeypatch.setattr(D, "DATA_DIR", tmp_path)
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    monkeypatch.setitem(D.SYMBOL_FILES, "DX", tmp_path / "dx.json")
    (tmp_path / "dx.json").write_text(json.dumps(arrays))
    n = len(gate_ts)
    write_gate_file(
        G.gate_path("DX"), [1] * n, [0] * n, [G.FATIGUE_UNAVAILABLE] * n,
        first_c=first_c, last_c=last_c,
    )
    D.ts_sidecar_path("DX").write_bytes(struct.pack(f"<{n}q", *gate_ts))


def test_diagnose_reports_ragged_ohlc_arrays_instead_of_crashing(tmp_path, monkeypatch):
    """json 各数组长度不齐（#1314 两假设之一）⇒ 出报告点名 vintage，而非 IndexError。

    dump 侧按 `len(closes)` 遍历、不截尾 ⇒ 真 dump 在这份 json 上会越界崩；诊断件若
    照搬那次崩溃就出不了报告，而这恰是最需要它的一种输入。
    """
    dates = ["2020-01-01T00:00:00", "2020-01-01T00:01:00", "2020-01-01T00:02:00"]
    _write_diag_inputs(
        tmp_path, monkeypatch,
        arrays={
            "dates": dates,
            "opens": [10.0, 11.0], "highs": [10.0, 11.0],  # 比 closes 短一项
            "lows": [10.0, 11.0], "closes": [10.0, 11.0, 12.0],
        },
        gate_ts=[D.to_epoch(s) for s in dates], first_c=10.0, last_c=12.0,
    )
    d = D.diagnose("DX")
    assert d["ohlc_arrays_ragged"] is True
    assert d["driver_n"] == 2          # 驱动 zip 截到最短
    assert d["dump_n"] is None         # dump 口径在这份 json 上算不出
    assert d["dropped_by_zip_truncation"] == 1
    text = D.format_report(d)
    assert "越界崩" in text and "vintage" in text


def test_diagnose_reports_dates_shorter_than_kept_bars(tmp_path, monkeypatch):
    """dates 短于驱动保留的最大下标（驱动 zip 不看 dates）⇒ 报 DATES_TOO_SHORT。"""
    _write_diag_inputs(
        tmp_path, monkeypatch,
        arrays={
            "dates": ["2020-01-01T00:00:00"],  # 只有 1 项，驱动保留 3 根
            "opens": [10.0, 11.0, 12.0], "highs": [10.0, 11.0, 12.0],
            "lows": [10.0, 11.0, 12.0], "closes": [10.0, 11.0, 12.0],
        },
        gate_ts=[D.to_epoch("2020-01-01T00:00:00")], first_c=10.0, last_c=12.0,
    )
    d = D.diagnose("DX")
    assert d["align"]["verdict"] == "DATES_TOO_SHORT"
    assert "数组不齐" in D.format_report(d)


def test_diagnose_driver_superset_explained_by_nonpositive(tmp_path, monkeypatch):
    """#1315 DX 形态：门列少 2 根，且少的正是 OHLC ≤0 的两根 ⇒ 判"口径差"而非"过期"。"""
    monkeypatch.setattr(D, "DATA_DIR", tmp_path)
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    monkeypatch.setitem(D.SYMBOL_FILES, "DX", tmp_path / "dx.json")

    dates = [f"2020-01-01T00:0{i}:00" for i in range(5)]
    (tmp_path / "dx.json").write_text(json.dumps({
        "dates": dates,
        "opens": [10.0, 0.0, 12.0, 13.0, 14.0],
        "highs": [10.0, 11.0, 12.0, 13.0, 14.0],
        "lows": [10.0, 11.0, 12.0, -1.0, 14.0],
        "closes": [10.0, 11.0, 12.0, 13.0, 14.0],
    }))
    gate_ts = [D.to_epoch(dates[i]) for i in (0, 2, 4)]
    write_gate_file(
        G.gate_path("DX"), [1, 0, 0], [0, 0, 1], [G.FATIGUE_UNAVAILABLE] * 3,
        first_c=10.0, last_c=14.0,
    )
    D.ts_sidecar_path("DX").write_bytes(struct.pack(f"<{len(gate_ts)}q", *gate_ts))

    d = D.diagnose("DX")
    assert d["driver_n"] == 5 and d["dump_n"] == 3
    assert d["dropped_by_nonpositive_only"] == 2
    assert d["gate_n_minus_driver_n"] == -2
    assert d["align"]["verdict"] == "DRIVER_SUPERSET"
    assert d["driver_excess_is_nonpositive_only"] is True
    text = D.format_report(d)
    assert "口径差重建臂" in text and "非磁带过期" in text
    assert "重跑 dump" not in text  # 处置不得把口径差误导成"重落磁带"


def test_diagnose_driver_superset_not_explained_is_still_stale_tape(tmp_path, monkeypatch):
    """门列少 bar 但少的不是 ≤0 那几根（一根 ≤0 都没有）⇒ 仍判过期/不同源，不放行重建。"""
    monkeypatch.setattr(D, "DATA_DIR", tmp_path)
    monkeypatch.setattr(G, "DATA_DIR", tmp_path)
    monkeypatch.setitem(D.SYMBOL_FILES, "DX", tmp_path / "dx.json")

    dates = [f"2020-01-01T00:0{i}:00" for i in range(3)]
    (tmp_path / "dx.json").write_text(json.dumps({
        "dates": dates,
        "opens": [10.0, 11.0, 12.0], "highs": [10.0, 11.0, 12.0],
        "lows": [10.0, 11.0, 12.0], "closes": [10.0, 11.0, 12.0],
    }))
    gate_ts = [D.to_epoch(dates[i]) for i in (0, 2)]
    write_gate_file(
        G.gate_path("DX"), [1, 0], [0, 1], [G.FATIGUE_UNAVAILABLE] * 2,
        first_c=10.0, last_c=12.0,
    )
    D.ts_sidecar_path("DX").write_bytes(struct.pack(f"<{len(gate_ts)}q", *gate_ts))

    d = D.diagnose("DX")
    assert d["align"]["verdict"] == "DRIVER_SUPERSET"
    assert d["driver_excess_is_nonpositive_only"] is False
    assert "过期" in D.format_report(d)


def test_align_report_flags_duplicate_timestamps(tmp_path):
    """ts 重复 ⇒ 下标分桶的前提被破坏，读数必须自报不可信（不静默给 verdict）。"""
    rep = D.align_report([100, 100, 160], [100, 160])
    assert rep["dup_ts_gate"] == 1 and rep["dup_ts_driver"] == 0
