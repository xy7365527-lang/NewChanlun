"""
DM2 测试：时间演化实验的管线正确性验证（L1）

验证：
1. 谱系编号排序正确性
2. 子复形序列单调性（K_n ⊂ K_{n+1}）
3. Morse 不等式在全序列上成立
4. 跳跃点检测逻辑
5. 最终快照与 DM2 结果文件一致
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# 确保 scripts 目录可导入
sys.path.insert(0, str(Path(__file__).parent.parent / "scripts"))
sys.path.insert(0, str(Path(__file__).parent.parent / "experiments" / "discrete_morse"))

from morse_temporal import (
    JumpPoint,
    TemporalSnapshot,
    detect_jumps,
    parse_genealogy_key,
    sorted_genealogy_keys,
)


# ─────────────────────────────────────────────────────────────────────────────
# 谱系编号排序测试
# ─────────────────────────────────────────────────────────────────────────────

class TestGenealogyKeySorting:
    def test_parse_numeric(self):
        assert parse_genealogy_key("001") == (1, "")
        assert parse_genealogy_key("353") == (353, "")

    def test_parse_with_suffix(self):
        assert parse_genealogy_key("005a") == (5, "a")
        assert parse_genealogy_key("073b") == (73, "b")
        assert parse_genealogy_key("019d") == (19, "d")

    def test_sort_order_basic(self):
        keys = {"003": "h3", "001": "h1", "002": "h2"}
        result = sorted_genealogy_keys(keys)
        assert result == ["001", "002", "003"]

    def test_sort_order_with_suffixes(self):
        keys = {
            "005b": "h5b", "005": "h5", "005a": "h5a",
            "006": "h6", "004": "h4",
        }
        result = sorted_genealogy_keys(keys)
        assert result == ["004", "005", "005a", "005b", "006"]

    def test_sort_matches_real_ordering(self):
        """验证 019, 019a, 019b, 019c, 019d, 020 的排序。"""
        keys = {
            "020": "h20", "019": "h19", "019d": "h19d",
            "019a": "h19a", "019c": "h19c", "019b": "h19b",
        }
        result = sorted_genealogy_keys(keys)
        assert result == ["019", "019a", "019b", "019c", "019d", "020"]


# ─────────────────────────────────────────────────────────────────────────────
# 跳跃点检测测试
# ─────────────────────────────────────────────────────────────────────────────

def _make_snapshot(step: int, key: str, dm0: int, dm1: int, dm2: int) -> TemporalSnapshot:
    return TemporalSnapshot(
        step=step,
        genealogy_key=key,
        num_vertices=step + 1,
        num_edges=0,
        num_triangles=0,
        beta_0=1,
        beta_1=0,
        beta_2=0,
        euler_char=1,
        m0=step + 1,
        m1=0,
        m2=0,
        morse_all_pass=True,
        delta_vertices=1,
        delta_edges=0,
        delta_triangles=0,
        delta_m0=dm0,
        delta_m1=dm1,
        delta_m2=dm2,
    )


class TestJumpDetection:
    def test_no_jumps_below_threshold(self):
        snaps = [
            _make_snapshot(0, "001", 1, 0, 0),
            _make_snapshot(1, "002", 1, 1, 0),
        ]
        jumps = detect_jumps(snaps, threshold=5)
        assert len(jumps) == 0

    def test_detects_jump_at_threshold(self):
        snaps = [
            _make_snapshot(0, "001", 1, 0, 0),
            _make_snapshot(1, "002", 1, 2, 2),  # |1|+|2|+|2| = 5
        ]
        jumps = detect_jumps(snaps, threshold=5)
        assert len(jumps) == 1
        assert jumps[0].genealogy_key == "002"
        assert jumps[0].magnitude == 5

    def test_magnitude_is_absolute(self):
        snaps = [_make_snapshot(0, "001", -2, -3, 1)]
        jumps = detect_jumps(snaps, threshold=1)
        assert jumps[0].magnitude == 6  # |−2|+|−3|+|1| = 6

    def test_threshold_zero_catches_all(self):
        snaps = [
            _make_snapshot(0, "001", 0, 0, 0),
            _make_snapshot(1, "002", 1, 0, 0),
        ]
        jumps = detect_jumps(snaps, threshold=0)
        # threshold=0: >= 0 catches everything including magnitude=0
        assert len(jumps) == 2

    def test_threshold_one_excludes_zero_magnitude(self):
        snaps = [
            _make_snapshot(0, "001", 0, 0, 0),
            _make_snapshot(1, "002", 1, 0, 0),
        ]
        jumps = detect_jumps(snaps, threshold=1)
        assert len(jumps) == 1
        assert jumps[0].genealogy_key == "002"


# ─────────────────────────────────────────────────────────────────────────────
# DM2 结果文件验证（如果存在）
# ─────────────────────────────────────────────────────────────────────────────

RESULTS_PATH = Path(__file__).parent.parent / "experiments" / "discrete_morse" / "dm2_results.json"


@pytest.fixture(scope="module")
def dm2_results():
    if not RESULTS_PATH.exists():
        pytest.skip("dm2_results.json 不存在，跳过结果验证")
    with open(RESULTS_PATH) as f:
        return json.load(f)


class TestDM2Results:
    def test_all_morse_pass(self, dm2_results):
        """全序列 Morse 不等式成立。"""
        assert dm2_results["all_morse_pass"] is True

    def test_monotonic_vertices(self, dm2_results):
        """子复形序列中顶点数单调递增。"""
        snapshots = dm2_results["snapshots"]
        for i in range(1, len(snapshots)):
            v_prev = snapshots[i - 1]["complex"]["vertices"]
            v_curr = snapshots[i]["complex"]["vertices"]
            assert v_curr >= v_prev, (
                f"顶点数非单调: step {i-1} ({v_prev}) > step {i} ({v_curr})"
            )

    def test_monotonic_edges(self, dm2_results):
        """子复形序列中边数单调递增。"""
        snapshots = dm2_results["snapshots"]
        for i in range(1, len(snapshots)):
            e_prev = snapshots[i - 1]["complex"]["edges"]
            e_curr = snapshots[i]["complex"]["edges"]
            assert e_curr >= e_prev, (
                f"边数非单调: step {i-1} ({e_prev}) > step {i} ({e_curr})"
            )

    def test_monotonic_triangles(self, dm2_results):
        """子复形序列中三角形数单调递增。"""
        snapshots = dm2_results["snapshots"]
        for i in range(1, len(snapshots)):
            t_prev = snapshots[i - 1]["complex"]["triangles"]
            t_curr = snapshots[i]["complex"]["triangles"]
            assert t_curr >= t_prev, (
                f"三角形数非单调: step {i-1} ({t_prev}) > step {i} ({t_curr})"
            )

    def test_euler_consistency(self, dm2_results):
        """每步的 Euler 数 χ = V - E + T = β₀ - β₁ + β₂。"""
        for snap in dm2_results["snapshots"]:
            c = snap["complex"]
            b = snap["betti"]
            chi_comb = c["vertices"] - c["edges"] + c["triangles"]
            chi_betti = b["beta_0"] - b["beta_1"] + b["beta_2"]
            assert chi_comb == chi_betti, (
                f"Euler 不一致 @ {snap['genealogy_key']}: "
                f"χ_comb={chi_comb} != χ_betti={chi_betti}"
            )

    def test_first_step_single_vertex(self, dm2_results):
        """第一步是单个顶点。"""
        first = dm2_results["snapshots"][0]
        assert first["complex"]["vertices"] == 1
        assert first["complex"]["edges"] == 0
        assert first["betti"]["beta_0"] == 1

    def test_jumps_sorted_by_step(self, dm2_results):
        """跳跃点按步骤排序。"""
        jumps = dm2_results["jumps"]
        for i in range(1, len(jumps)):
            assert jumps[i]["step"] >= jumps[i - 1]["step"]

    def test_top10_sorted_by_magnitude(self, dm2_results):
        """Top-10 按幅度降序。"""
        top = dm2_results["top_10_jumps"]
        for i in range(1, len(top)):
            assert top[i]["magnitude"] <= top[i - 1]["magnitude"]

    def test_jump_magnitudes_consistent(self, dm2_results):
        """跳跃点的幅度 = |Δm₀| + |Δm₁| + |Δm₂|。"""
        for j in dm2_results["jumps"]:
            expected = abs(j["delta_m0"]) + abs(j["delta_m1"]) + abs(j["delta_m2"])
            assert j["magnitude"] == expected

    def test_morse_weak_inequalities_all_steps(self, dm2_results):
        """全序列的弱 Morse 不等式 m_k >= β_k。"""
        for snap in dm2_results["snapshots"]:
            b = snap["betti"]
            m = snap["morse"]
            assert m["m0"] >= b["beta_0"], (
                f"m₀ < β₀ @ {snap['genealogy_key']}"
            )
            assert m["m1"] >= b["beta_1"], (
                f"m₁ < β₁ @ {snap['genealogy_key']}"
            )
            assert m["m2"] >= b["beta_2"], (
                f"m₂ < β₂ @ {snap['genealogy_key']}"
            )

    def test_known_top_jump_087(self, dm2_results):
        """087号（编排者否定086——补丁思维第一次否定）应在 top-10 跳跃中。"""
        top_keys = [j["genealogy_key"] for j in dm2_results["top_10_jumps"]]
        assert "087" in top_keys, (
            f"087号不在 top-10 跳跃中: {top_keys}"
        )
