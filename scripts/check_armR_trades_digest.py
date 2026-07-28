#!/usr/bin/env python3
"""臂R（treasury 重验 T1，issue #387/#446）trades.jsonl 逐位回归锁——机器可复核。

**性质**：后向回归锁。原 #387 基线因 #446 消除活动集同 ElementId 双计而按红线流程失效；
当前冻结的是「2026-07-28，谱系起点 a12a1022d9、最终核验 HEAD 6e15ceffee +
未提交 #446 修复」臂R 三窗产物的逐字节摘要（并行 #421 前后三窗逐字相同），
不是对 2026-07-20 v4 原始产物的对照——v4 的 `trades.jsonl` 已灭失，无法逐位比。

**摘要算法**：FNV-1a 64，逐字节喂 `trades.jsonl` 原始字节流（禁容差比较）。与
`classifier::signal::extract_signals_bit_exact_digest_guard` / `backtest::runner::order_stream_digest`
同款 golden 冻结协议（同一 FNV-1a 64 常数：offset 0xcbf29ce484222325、prime 0x100000001b3）。

**产物再生成命令**（三窗逐窗跑，`M8_WIN_FILTER` 已实证 bit-exact 中性）：

    cd rust
    for tag in p3fold wf7 wf8; do
      M8_WIN_FILTER=$tag VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 \
        M8_REPORT_PATH=/tmp/446_armR_report_$tag.md OPSEM_DUMP_DIR=/tmp/m8_win_gate/$tag \
        cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
        -- --ignored --nocapture
    done

**本脚本用法**：

    python3 scripts/check_armR_trades_digest.py                 # 校验（默认读 /tmp/m8_win_gate）
    python3 scripts/check_armR_trades_digest.py --dump-dir DIR   # 指定 dump 根目录
    python3 scripts/check_armR_trades_digest.py --regen          # 重新落 golden（改动经审后才允许）

**退出码**：0=无漂移；1=真漂移（打字段级差异）；4=产物缺失（跑批未做，非漂移）。
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
GOLDEN_PATH = REPO_ROOT / "chanlun" / "review-results" / "treasury-reverify-t1-armR-trades-golden-20260727.json"
DEFAULT_DUMP_DIR = pathlib.Path("/tmp/m8_win_gate")
WINDOWS = ("p3fold", "wf7", "wf8")

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK64 = 0xFFFFFFFFFFFFFFFF

EXIT_OK = 0
EXIT_DRIFT = 1
EXIT_MISSING = 4


def fnv1a64(data: bytes) -> int:
    h = FNV_OFFSET
    for b in data:
        h ^= b
        h = (h * FNV_PRIME) & MASK64
    return h


def readings_of(path: pathlib.Path) -> dict:
    """逐笔读数聚合（`certificate.dir` 分向，pnl = `pnl_raw_unlevered` 求和，顺序即文件序）。"""
    raw = path.read_bytes()
    n_lines = 0
    per_dir_n: collections.Counter = collections.Counter()
    per_dir_pnl: dict[str, float] = collections.defaultdict(float)
    depth: collections.Counter = collections.Counter()
    for line in raw.splitlines():
        if not line.strip():
            continue
        n_lines += 1
        o = json.loads(line)
        cert = o.get("certificate") or {}
        direction = cert.get("dir", "<none>")
        per_dir_n[direction] += 1
        per_dir_pnl[direction] += o.get("pnl_raw_unlevered", 0.0)
        nest_depth = cert.get("nest_depth")
        if nest_depth is not None:
            depth[nest_depth] += 1
    return {
        "digest_fnv1a64": f"0x{fnv1a64(raw):016x}",
        "bytes": len(raw),
        "n_trades": n_lines,
        "per_dir_n": {k: per_dir_n[k] for k in sorted(per_dir_n)},
        # float 以 repr 存 ⟹ 逐位比较（同序求和确定性，禁容差）。
        "per_dir_pnl": {k: repr(per_dir_pnl[k]) for k in sorted(per_dir_pnl)},
        "nest_depth_hist": {str(k): depth[k] for k in sorted(depth)},
    }


def collect(dump_dir: pathlib.Path) -> tuple[dict, list[str]]:
    out: dict = {}
    missing: list[str] = []
    for tag in WINDOWS:
        path = dump_dir / tag / "trades.jsonl"
        if not path.exists():
            missing.append(str(path))
            continue
        out[tag] = readings_of(path)
    return out, missing


def diff_report(golden: dict, actual: dict) -> list[str]:
    problems = []
    for tag in WINDOWS:
        g, a = golden.get(tag), actual.get(tag)
        if g is None:
            problems.append(f"[{tag}] golden 缺该窗条目")
            continue
        for field in ("digest_fnv1a64", "bytes", "n_trades", "per_dir_n", "per_dir_pnl", "nest_depth_hist"):
            if g.get(field) != a.get(field):
                problems.append(f"[{tag}] {field}: golden={g.get(field)!r} actual={a.get(field)!r}")
    return problems


def main() -> int:
    ap = argparse.ArgumentParser(description="臂R trades.jsonl 逐位回归锁校验")
    ap.add_argument("--dump-dir", type=pathlib.Path, default=DEFAULT_DUMP_DIR,
                    help=f"OPSEM_DUMP_DIR 根目录（默认 {DEFAULT_DUMP_DIR}）")
    ap.add_argument("--regen", action="store_true", help="重落 golden（改动经审后才允许）")
    args = ap.parse_args()

    actual, missing = collect(args.dump_dir)
    if missing:
        print("产物缺失（跑批未做，非漂移）：", file=sys.stderr)
        for m in missing:
            print(f"  {m}", file=sys.stderr)
        print("再生成命令见本脚本 docstring。", file=sys.stderr)
        return EXIT_MISSING

    if args.regen:
        payload = {
            "_note": "臂R（VOICE_EXEC=1 THETA_NEST_CERT_GATE=1，fee_schedule=None/enforce_level_cap=false）"
                     "三窗 trades.jsonl 逐位冻结；issue #446 消除双计后的 #387 T1 红线重锚。",
            "_source_base_head": "6e15ceffeeb8259c065bf7c0ec9ec7c65935737c",
            "_source_lineage_start": "a12a1022d9ddd8d1cae867a107a3a33c359358cf",
            "_source_worktree": "未提交 issue #446 修复；由编排者验收后提交",
            "_run_date": "2026-07-28",
            "_regen_command": (
                "cd rust && for tag in p3fold wf7 wf8; do "
                "M8_WIN_FILTER=$tag VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 "
                "M8_REPORT_PATH=/tmp/446_armR_report_$tag.md "
                "OPSEM_DUMP_DIR=/tmp/446_armR_dump/$tag "
                "cargo test --release --lib "
                "theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture; done"
            ),
            "_check_command": (
                "python3 scripts/check_armR_trades_digest.py --dump-dir /tmp/446_armR_dump"
            ),
            "windows": actual,
        }
        GOLDEN_PATH.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")
        print(f"golden 已重落：{GOLDEN_PATH}")
        return EXIT_OK

    if not GOLDEN_PATH.exists():
        print(f"golden 文件不存在：{GOLDEN_PATH}（先跑 --regen）", file=sys.stderr)
        return EXIT_MISSING

    golden = json.loads(GOLDEN_PATH.read_text()).get("windows", {})
    problems = diff_report(golden, actual)
    if problems:
        print("臂R trades 漂移（逐字段差异）：", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return EXIT_DRIFT

    for tag in WINDOWS:
        a = actual[tag]
        print(f"{tag}: n_trades={a['n_trades']} digest={a['digest_fnv1a64']} bytes={a['bytes']} ✓")
    print("臂R trades 逐位无漂移。")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
