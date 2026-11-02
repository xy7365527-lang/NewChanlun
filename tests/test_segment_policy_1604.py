"""#1604（#1586 A-12）：Python 线段相切口径读记录 `segment-second`。

- 记录读取：生效值 / 其他版本 / 负控（缺记录、缺字段、未签、非法值）。
- v0 / v1 两处谓词随记录取值翻转（常量回退变异体必红）；签署值 `closed` 复现 #1604 前的 v1 口径。
"""

from __future__ import annotations

import json

import pytest

import newchan.a_segment_v0 as v0
import newchan.a_segment_v1 as v1
from newchan.a_stroke import Stroke
import newchan.ab_bridge_newchan as ab_bridge
from newchan.segment_policy import (
    RECORD_PATH,
    SCAN_BUDGET_PATH,
    ScanBudget,
    SegmentPolicyError,
    active_scan_budget,
    active_segment_construction,
    parse_scan_budget,
    parse_segment_construction,
)


def _s(i0, i1, direction, high, low):
    p0, p1 = (low, high) if direction == "up" else (high, low)
    return Stroke(i0=i0, i1=i1, direction=direction, high=high, low=low, p0=p0, p1=p1,
                  confirmed=True)


def _rec(tangency, version=3):
    return parse_segment_construction(json.dumps(
        {"id": "segment-second", "version": version, "fields": {"tangency": {"value": tangency}}}
    ))


def test_active_record_matches_rust_signed_value():
    rule = active_segment_construction()
    assert rule.tangency == "closed"
    assert rule.label == "segment-second/2"
    # 与 Rust `parser::segment::CONSTRUCTION_RULE` 同一 token，读的是同一份文件。
    assert RECORD_PATH.name == "segment-second.json"
    assert json.loads(RECORD_PATH.read_text(encoding="utf-8"))["fields"]["tangency"]["value"] \
        == rule.tangency


def test_other_version_parses_with_own_version():
    rule = _rec("open")
    assert (rule.tangency, rule.label) == ("open", "segment-second/3")


@pytest.mark.parametrize("text", [
    None,
    '{"id":"segment-second","version":2,"fields":{}}',
    '{"id":"segment-second","version":2,"fields":{"tangency":{"value":null}}}',
    '{"id":"segment-second","version":2,"fields":{"tangency":{"value":"AwaitingPolicy"}}}',
    '{"id":"segment-second","version":2,"fields":{"tangency":"AwaitingPolicy"}}',
    '{"id":"segment-second","version":2,"fields":{"tangency":{"value":"half"}}}',
    '{"id":"segment-second","version":2,"fields":{"tangency":{"value":"Closed"}}}',
    '{"id":"segment-second","version":2,"fields":{"tangency":"closed"}}',
    '{"id":"segment-second","version":0,"fields":{"tangency":{"value":"closed"}}}',
    '{"id":"macd-force","version":2,"fields":{"tangency":{"value":"closed"}}}',
    "not json",
])
def test_negative_controls_raise_without_fallback(text):
    with pytest.raises(SegmentPolicyError):
        parse_segment_construction(text)


# 三笔 [5,10]、[0,10]、[0,5]：max(lows)=5 == min(highs)=5，仅在 5 相切。
TANGENT = [_s(0, 4, "up", 10, 5), _s(4, 8, "down", 10, 0), _s(8, 12, "up", 5, 0)]


@pytest.mark.parametrize("tangency, expect", [("closed", True), ("open", False)])
def test_v1_three_stroke_overlap_reads_record(monkeypatch, tangency, expect):
    monkeypatch.setattr(v1, "active_segment_construction", lambda: _rec(tangency))
    assert v1._three_stroke_overlap(*TANGENT) is expect


@pytest.mark.parametrize("tangency, expect", [("closed", False), ("open", True)])
def test_v1_gap_reads_record(monkeypatch, tangency, expect):
    monkeypatch.setattr(v1, "active_segment_construction", lambda: _rec(tangency))
    # 向上段顶分型 a=[5,10]、b=[10,20]、c=[4,8]：b_l == a_h 相切。
    assert v1._is_fractal_and_gap(10, 5, 20, 10, 8, 4, "up") == (True, expect)
    # 向下段底分型镜像：a_l == b_h 相切。
    assert v1._is_fractal_and_gap(20, 10, 10, 3, 18, 14, "down") == (True, expect)


@pytest.mark.parametrize("tangency, expect", [("closed", 1), ("open", 0)])
def test_v0_overlap_reads_record(monkeypatch, tangency, expect):
    monkeypatch.setattr(v0, "active_segment_construction", lambda: _rec(tangency))
    assert len(v0.segments_from_strokes_v0(TANGENT)) == expect


def test_v0_signed_record_counts_tangency_as_overlap():
    """#1604 M2：v0 不再写死严格 `<`，按签署记录 `segment-second/2`（tangency=closed，xianduan §6.3）
    读值——相切的三笔起一段。#1604 前 v0 对同一输入给 0 段（相切不算重合，即写死了 `open`）。
    不打桩，直接读仓库里的生效记录；`ab_bridge_newchan` 的 `segment_algo="v0"` 路径调用的就是这一个函数。"""
    assert active_segment_construction().tangency == "closed"
    segs = v0.segments_from_strokes_v0(TANGENT)
    assert [(s.s0, s.s1, s.direction, s.high, s.low) for s in segs] == [(0, 2, "up", 10, 0)]
    assert ab_bridge.segments_from_strokes_v0 is v0.segments_from_strokes_v0


def test_active_scan_budget_matches_rust_record():
    b = active_scan_budget()
    assert (b.label, b.second_seq_window, b.tail_window) == ("second-seq-scan-budget/2", 0, 7)
    assert json.loads(SCAN_BUDGET_PATH.read_text(encoding="utf-8"))["fields"]["tail_window"]["value"] \
        == b.tail_window


@pytest.mark.parametrize("text", [
    None,
    '{"id":"second-seq-scan-budget","version":1,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":7}}}',
    '{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0}}}',
    '{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":null}}}',
    '{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":-1}}}',
    '{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":7}}',
    '{"id":"second-seq-scan-budget","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":true}}}',
    '{"id":"segment-second","version":2,"fields":{"second_seq_window":{"value":0},"tail_window":{"value":7}}}',
])
def test_scan_budget_negative_controls_raise_without_fallback(text):
    with pytest.raises(SegmentPolicyError):
        parse_scan_budget(text)


def _budget(tw):
    return ScanBudget(record_id="second-seq-scan-budget", version=3, second_seq_window=0, tail_window=tw)


def test_v1_tail_window_reads_record(monkeypatch):
    """L1：v1 的尾窗读记录（类常数已删）；改回常数的变异体会让 tw=0/11 两档不随记录变。"""
    assert not hasattr(v1._FeatureSeqState, "TAIL_WINDOW")
    assert v1._FeatureSeqState("up").tail_window == 7
    for tw in (0, 3, 11):
        monkeypatch.setattr(v1, "active_scan_budget", lambda tw=tw: _budget(tw))
        assert v1._FeatureSeqState("up").tail_window == tw


@pytest.mark.parametrize("tw", [1, 2])
def test_v1_rejects_tail_window_without_truncation_marker(monkeypatch, tw):
    monkeypatch.setattr(v1, "active_scan_budget", lambda: _budget(tw))
    with pytest.raises(SegmentPolicyError):
        v1._FeatureSeqState("up")
