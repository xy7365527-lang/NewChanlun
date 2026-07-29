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
        M8_REPORT_PATH=/tmp/m8_win_gate_report_$tag.md OPSEM_DUMP_DIR=/tmp/m8_win_gate/$tag \
        cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
        -- --ignored --nocapture
    done

**provenance 口径**：golden 锚中的 `dump_dir` / `regen_command` / `check_command`
保留 `/tmp/446_armR_dump`，只记录历史锚来源；历史锚来源 ≠ 当前 canonical 默认根
`/tmp/m8_win_gate/`。

**provenance 校验边界**：本门只在本仓 git worktree 内运行；每个 head 必须可由
`git cat-file -e <sha>^{commit}` 解析，谱系起点必须通过
`git merge-base --is-ancestor`。无 git 环境时门会如实失败并返回非零，不提供伪成功降级。
但「head 可解析 + 祖先成立」不证明 dump 确由该 HEAD 产生：当前没有 sidecar/manifest
绑定；该机制归后续票，本票不实现。

**本脚本用法**：

    python3 scripts/check_armR_trades_digest.py                 # 校验（默认读 /tmp/m8_win_gate）
    python3 scripts/check_armR_trades_digest.py --dump-dir DIR   # 指定 dump 根目录
    python3 scripts/check_armR_trades_digest.py --dump-dir DIR --regen
        # 追加 provenance 历史锚并重落同 schema golden（改动经审后才允许）

**退出码**：0=无漂移；1=真漂移（打字段级差异）；4=产物缺失（跑批未做，非漂移）。
"""

from __future__ import annotations

import argparse
import collections
import datetime
import json
import pathlib
import re
import shlex
import subprocess
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
GOLDEN_PATH = REPO_ROOT / "chanlun" / "review-results" / "treasury-reverify-t1-armR-trades-golden-20260727.json"
DEFAULT_DUMP_DIR = pathlib.Path("/tmp/m8_win_gate")
WINDOWS = ("p3fold", "wf7", "wf8")
GOLDEN_SCHEMA = "armR-trades-digest/v2"
REQUIRED_ANCHOR_FIELDS = (
    "source_base_head",
    "final_verification_head",
    "source_worktree",
    "run_date",
    "regen_command",
    "check_command",
)
HEAD_FIELDS = (
    "source_base_head",
    "final_verification_head",
    "source_lineage_start",
)
ANCHOR_STRING_FIELDS = (
    "source_worktree",
    "regen_command",
    "check_command",
    "dump_dir",
    "parallel_head_note",
)

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK64 = 0xFFFFFFFFFFFFFFFF

EXIT_OK = 0
EXIT_DRIFT = 1
EXIT_MISSING = 4


def _legacy_anchor(payload: dict) -> dict | None:
    """把 v1 顶层 provenance 原样迁入首个历史锚，禁止 regen 静默丢字段。"""
    mapping = {
        "source_base_head": "_source_base_head",
        "source_worktree": "_source_worktree",
        "run_date": "_run_date",
        "regen_command": "_regen_command",
        "check_command": "_check_command",
    }
    if not all(field in payload for field in mapping.values()):
        return None
    anchor = {field: payload[legacy] for field, legacy in mapping.items()}
    anchor["final_verification_head"] = payload.get(
        "_final_verification_head", payload["_source_base_head"]
    )
    if "_source_lineage_start" in payload:
        anchor["source_lineage_start"] = payload["_source_lineage_start"]
    if "_parallel_head_note" in payload:
        anchor["parallel_head_note"] = payload["_parallel_head_note"]
    return anchor


def _current_regen_anchor(dump_dir: pathlib.Path) -> dict:
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    status = subprocess.run(
        ["git", "status", "--short"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    quoted_dump = shlex.quote(str(dump_dir))
    return {
        "source_base_head": head,
        "final_verification_head": head,
        "source_worktree": "clean" if not status else f"dirty（git status --short: {len(status)} paths）",
        "run_date": datetime.date.today().isoformat(),
        "dump_dir": str(dump_dir),
        "regen_command": f"python3 scripts/check_armR_trades_digest.py --dump-dir {quoted_dump} --regen",
        "check_command": f"python3 scripts/check_armR_trades_digest.py --dump-dir {quoted_dump}",
    }


def _git_commit_problem(head: str) -> str | None:
    try:
        result = subprocess.run(
            ["git", "cat-file", "-e", f"{head}^{{commit}}"],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as exc:
        return f"git 无法执行，不能验证 commit：{exc}"
    if result.returncode != 0:
        return "仓内不可解析 commit"
    return None


def _git_ancestry_problem(
    ancestor: str, descendant: str, descendant_field: str
) -> str | None:
    try:
        result = subprocess.run(
            ["git", "merge-base", "--is-ancestor", ancestor, descendant],
            cwd=REPO_ROOT,
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as exc:
        return f"git 无法执行，不能验证祖先关系：{exc}"
    if result.returncode == 1:
        return f"不是 {descendant_field} 的祖先"
    if result.returncode != 0:
        return f"git merge-base --is-ancestor 校验失败（exit={result.returncode}）"
    return None


def provenance_problems(payload: dict) -> list[str]:
    problems: list[str] = []
    if payload.get("_schema") != GOLDEN_SCHEMA:
        problems.append(f"_schema: expected={GOLDEN_SCHEMA!r} actual={payload.get('_schema')!r}")
    if "_note" in payload and not isinstance(payload["_note"], str):
        problems.append("_note: 必须是字符串")
    provenance = payload.get("provenance")
    anchors = provenance.get("anchors") if isinstance(provenance, dict) else None
    if not isinstance(anchors, list) or not anchors:
        problems.append("provenance.anchors: 必须是非空历史锚数组")
        return problems
    for idx, anchor in enumerate(anchors):
        if not isinstance(anchor, dict):
            problems.append(f"provenance.anchors[{idx}]: 必须是对象")
            continue
        missing = [field for field in REQUIRED_ANCHOR_FIELDS if not anchor.get(field)]
        if missing:
            problems.append(f"provenance.anchors[{idx}]: 缺字段 {missing}")
        for field in ANCHOR_STRING_FIELDS:
            if field in anchor and not isinstance(anchor[field], str):
                problems.append(
                    f"provenance.anchors[{idx}].{field}: 必须是字符串"
                )
        valid_head_fields: set[str] = set()
        for field in HEAD_FIELDS:
            value = anchor.get(field)
            if field == "source_lineage_start" and field not in anchor:
                continue
            if not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{40}", value) is None:
                problems.append(
                    f"provenance.anchors[{idx}].{field}: 必须是 40 位小写 hex"
                )
                continue
            commit_problem = _git_commit_problem(value)
            if commit_problem is not None:
                problems.append(
                    f"provenance.anchors[{idx}].{field}: {commit_problem}"
                )
                continue
            valid_head_fields.add(field)
        lineage_edges = (
            (("source_lineage_start", "source_base_head"),)
            if "source_lineage_start" in anchor
            else ()
        )
        ancestry_edges = (
            *lineage_edges,
            ("source_base_head", "final_verification_head"),
        )
        for ancestor_field, descendant_field in ancestry_edges:
            if (
                ancestor_field not in valid_head_fields
                or descendant_field not in valid_head_fields
            ):
                continue
            ancestry_problem = _git_ancestry_problem(
                anchor[ancestor_field],
                anchor[descendant_field],
                descendant_field,
            )
            if ancestry_problem is not None:
                problems.append(
                    f"provenance.anchors[{idx}].{ancestor_field}: {ancestry_problem}"
                )
        run_date = anchor.get("run_date")
        try:
            parsed_run_date = datetime.date.fromisoformat(run_date) if isinstance(run_date, str) else None
        except ValueError:
            parsed_run_date = None
        if parsed_run_date is None or parsed_run_date.isoformat() != run_date:
            problems.append(
                f"provenance.anchors[{idx}].run_date: 必须是有效 ISO 日期 YYYY-MM-DD"
            )
    windows = payload.get("windows")
    if isinstance(windows, dict):
        for tag in WINDOWS:
            window = windows.get(tag)
            if not isinstance(window, dict):
                continue
            digest = window.get("digest_fnv1a64")
            if not isinstance(digest, str) or re.fullmatch(r"0x[0-9a-f]{16}", digest) is None:
                problems.append(
                    f"windows.{tag}.digest_fnv1a64: 必须是带 0x 前缀的 16 位小写 hex"
                )
            for field in ("n_trades", "bytes"):
                value = window.get(field)
                if type(value) is not int or value <= 0:
                    problems.append(f"windows.{tag}.{field}: 必须是正整数")
    return problems


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
        previous = json.loads(GOLDEN_PATH.read_text()) if GOLDEN_PATH.exists() else {}
        provenance = previous.get("provenance")
        if previous.get("_schema") == GOLDEN_SCHEMA and isinstance(provenance, dict):
            anchors = list(provenance.get("anchors", []))
        else:
            legacy = _legacy_anchor(previous)
            anchors = [legacy] if legacy is not None else []
        anchors.append(_current_regen_anchor(args.dump_dir))
        payload = {
            "_schema": GOLDEN_SCHEMA,
            "_note": "臂R（VOICE_EXEC=1 THETA_NEST_CERT_GATE=1，fee_schedule=None/enforce_level_cap=false）"
                     "三窗 trades.jsonl 逐位冻结；issue #446 消除双计后的 #387 T1 红线重锚。",
            "provenance": {"anchors": anchors},
            "windows": actual,
        }
        GOLDEN_PATH.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")
        print(f"golden 已重落：{GOLDEN_PATH}")
        return EXIT_OK

    if not GOLDEN_PATH.exists():
        print(f"golden 文件不存在：{GOLDEN_PATH}（先跑 --regen）", file=sys.stderr)
        return EXIT_MISSING

    golden_payload = json.loads(GOLDEN_PATH.read_text())
    problems = provenance_problems(golden_payload)
    problems.extend(diff_report(golden_payload.get("windows", {}), actual))
    if problems:
        print("臂R golden/provenance 或 trades 漂移（逐字段差异）：", file=sys.stderr)
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
