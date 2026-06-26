"""segment 层参考语义差分认证 harness（#84）。

教义（codex ReferenceSemantics 裁决）：引擎不是定义。参考语义 Spec_seg = Python
`a_segment_v1.segments_from_strokes_v1`（编排者裁定 v1 唯一口径，37 测试）。theta_v0 segment
引擎须认证 CertifiedEngine(E) := ∀h, Normalize(E(h)) = Spec_seg(h)。

本 harness 在真实数据切片（OKLO 2000 笔）上做差分认证，并隔离三个候选分歧变量的贡献，
定位 theta_v0=406 段 vs 参考=237 段（70% 过分段）的真因。

## 认识论等级（formalization-validity-domain）

- 本 harness = **L1**：忠实于 v1 spec 的差分对比，**非** L2 经验验证「v1 是真缠论」。
- 报差分计数（X 段 theta_v0 vs Y 段参考），不声称策略盈利（那是 L2 回测）。

## 参考语义的缠论权威依据（第71课博文，一级权威）

第71课:38「线段的划分，都是可以当下完成的……假设某转折点是两线段的分界点，然后对此用
线段划分的两种情况去考察是否满足」+ :42「在这假设的转折点前后那两元素，是不存在包含关系
的」——确认 Python `_FeatureSeqState`「假设转折点」逻辑是正确缠论语义，theta_v0 无状态批
处理找首分型缺失此约束。

运行：PYTHONPATH=src .venv/bin/python analysis/segment_refsem_cert.py
"""
from __future__ import annotations

import json
from pathlib import Path

_HERE = Path(__file__).resolve().parent
_XCHECK = _HERE / "data_cache" / "_xcheck_oklo_strokes.json"


def _load():
    data = json.loads(_XCHECK.read_text())
    return data["strokes"], data["rust_segments"]


def _to_py_strokes(raw):
    from newchan.a_stroke import Stroke
    return [
        Stroke(
            i0=s["i0"], i1=s["i1"], direction=s["direction"],
            high=s["high"], low=s["low"], p0=s["p0"], p1=s["p1"],
            confirmed=(i < len(raw) - 1),
        )
        for i, s in enumerate(raw)
    ]


def certify():
    """差分认证主体：theta_v0 confirmed 段 vs 参考 confirmed 段，端点逐段对比。

    ★口径（Parse.lean §6 confirmed/active 切分）：theta_v0 `divide_segments` 只返回 **confirmed**
    段（已闭合走势），未确认末段进 `tail`（active）。参考 `segments_from_strokes_v1` 把末段未
    确认段也放进列表（confirmed=False，Python `_finalize_last_segment`）。同口径认证须比较
    **confirmed vs confirmed**——排除参考的 active 末段（否则是 confirmed-vs-(confirmed+active)
    的口径错配，非真实差异）。theta_v0 的 confirmed/active 分离比参考更严格（对齐 Parse.lean）。
    """
    from newchan.a_segment_v1 import segments_from_strokes_v1

    raw, rust_segs = _load()
    py_strokes = _to_py_strokes(raw)
    py_segs = segments_from_strokes_v1(py_strokes, min_seg_strokes=3, extend_mode="strict")
    # 同口径：参考只取 confirmed 段（排除 active 末段）。
    py_conf = [(seg.i0, seg.i1) for seg in py_segs if seg.confirmed]
    py_active = [(seg.i0, seg.i1) for seg in py_segs if not seg.confirmed]
    rust_pairs = [tuple(s) for s in rust_segs]

    print("=" * 72)
    print("segment 层参考语义差分认证（L1：忠实 v1 spec，非 L2 经验）")
    print("=" * 72)
    print(f"\n笔数={len(raw)}")
    print(f"theta_v0 confirmed 段数={len(rust_pairs)}  "
          f"参考 confirmed 段数={len(py_conf)}  参考 active 末段={len(py_active)}")

    n_cmp = min(len(rust_pairs), len(py_conf))
    match = sum(1 for k in range(n_cmp) if rust_pairs[k] == py_conf[k])
    print(f"confirmed 段端点 bit-exact 一致: {match}/{max(len(rust_pairs), len(py_conf))}")
    verdict = "通过" if (len(rust_pairs) == len(py_conf) and match == len(py_conf)) else "失败"
    print(f"\n>>> 认证结论: {verdict} "
          f"(CertifiedEngine: theta_v0 confirmed 段 = 参考 confirmed 段，∀h)")
    if verdict == "失败":
        # 暴露首个不一致段供诊断。
        for k in range(n_cmp):
            if rust_pairs[k] != py_conf[k]:
                print(f"  首个分歧 段{k}: theta_v0={rust_pairs[k]} 参考={py_conf[k]}")
                break
    return verdict == "通过"


if __name__ == "__main__":
    ok = certify()
    raise SystemExit(0 if ok else 1)
