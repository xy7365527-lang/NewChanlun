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
from newchan.segment_policy import (
    RECORD_PATH,
    SegmentPolicyError,
    active_segment_construction,
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
