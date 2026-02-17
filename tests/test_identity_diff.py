"""core/diff/identity.py 补全测试 — 覆盖 *_identity_key 函数。"""
from __future__ import annotations

from types import SimpleNamespace

import pytest

from newchan.core.diff.identity import (
    bsp_identity_key,
    move_identity_key,
    same_bsp_identity,
    same_move_identity,
    same_segment_identity,
    same_zhongshu_identity,
    segment_identity_key,
    zhongshu_identity_key,
)


# ── Segment ──────────────────────────────────────────────


class TestSegmentIdentity:
    def _seg(self, s0=0, direction="up", s1=5, high=100.0, low=90.0):
        return SimpleNamespace(s0=s0, direction=direction, s1=s1, high=high, low=low)

    def test_identity_key(self):
        seg = self._seg(s0=3, direction="down")
        assert segment_identity_key(seg) == (3, "down")

    def test_identity_key_type(self):
        seg = self._seg(s0=0, direction="up")
        key = segment_identity_key(seg)
        assert isinstance(key, tuple) and len(key) == 2

    def test_same_identity_true(self):
        a = self._seg(s0=1, direction="up")
        b = self._seg(s0=1, direction="up", s1=10)  # different s1 but same identity
        assert same_segment_identity(a, b)

    def test_same_identity_false_direction(self):
        a = self._seg(s0=1, direction="up")
        b = self._seg(s0=1, direction="down")
        assert not same_segment_identity(a, b)


# ── Zhongshu ─────────────────────────────────────────────


class TestZhongshuIdentity:
    def _zs(self, zd=90.0, zg=100.0, seg_start=0, seg_end=4):
        return SimpleNamespace(zd=zd, zg=zg, seg_start=seg_start, seg_end=seg_end)

    def test_identity_key(self):
        zs = self._zs(zd=88.0, zg=95.0, seg_start=2)
        assert zhongshu_identity_key(zs) == (88.0, 95.0, 2)

    def test_identity_key_type(self):
        zs = self._zs()
        key = zhongshu_identity_key(zs)
        assert isinstance(key, tuple) and len(key) == 3

    def test_same_identity_true(self):
        a = self._zs(zd=90.0, zg=100.0, seg_start=0)
        b = self._zs(zd=90.0, zg=100.0, seg_start=0, seg_end=6)
        assert same_zhongshu_identity(a, b)

    def test_same_identity_false(self):
        a = self._zs(zd=90.0, zg=100.0, seg_start=0)
        b = self._zs(zd=91.0, zg=100.0, seg_start=0)
        assert not same_zhongshu_identity(a, b)


# ── Move ─────────────────────────────────────────────────


class TestMoveIdentity:
    def _move(self, seg_start=0, seg_end=4, direction="up"):
        return SimpleNamespace(seg_start=seg_start, seg_end=seg_end, direction=direction)

    def test_identity_key(self):
        m = self._move(seg_start=5)
        assert move_identity_key(m) == (5,)

    def test_identity_key_type(self):
        m = self._move()
        key = move_identity_key(m)
        assert isinstance(key, tuple) and len(key) == 1

    def test_same_identity_true(self):
        a = self._move(seg_start=3, seg_end=7)
        b = self._move(seg_start=3, seg_end=10)
        assert same_move_identity(a, b)

    def test_same_identity_false(self):
        a = self._move(seg_start=3)
        b = self._move(seg_start=4)
        assert not same_move_identity(a, b)


# ── BuySellPoint ─────────────────────────────────────────


class TestBspIdentity:
    def _bsp(self, seg_idx=0, kind="1", side="buy", level_id=1):
        return SimpleNamespace(seg_idx=seg_idx, kind=kind, side=side, level_id=level_id)

    def test_identity_key(self):
        bp = self._bsp(seg_idx=2, kind="2", side="sell", level_id=3)
        assert bsp_identity_key(bp) == (2, "2", "sell", 3)

    def test_identity_key_type(self):
        bp = self._bsp()
        key = bsp_identity_key(bp)
        assert isinstance(key, tuple) and len(key) == 4

    def test_same_identity_true(self):
        a = self._bsp(seg_idx=1, kind="1", side="buy", level_id=2)
        b = self._bsp(seg_idx=1, kind="1", side="buy", level_id=2)
        assert same_bsp_identity(a, b)

    def test_same_identity_false_kind(self):
        a = self._bsp(seg_idx=1, kind="1", side="buy", level_id=2)
        b = self._bsp(seg_idx=1, kind="2", side="buy", level_id=2)
        assert not same_bsp_identity(a, b)

    def test_same_identity_false_side(self):
        a = self._bsp(seg_idx=1, kind="1", side="buy", level_id=2)
        b = self._bsp(seg_idx=1, kind="1", side="sell", level_id=2)
        assert not same_bsp_identity(a, b)
