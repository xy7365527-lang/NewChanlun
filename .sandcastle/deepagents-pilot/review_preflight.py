#!/usr/bin/env python3
"""#1742 正式独审 attempt 的启动前预检。只报告环境能否启动，不是审阅结果。"""
import argparse
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

# 与 pilot1742.external_review.SPACE_RESERVE 相同；预检不导入控制器，避免改变其加载面。
SPACE_RESERVE = 256 * 1024 * 1024
MODELS = ("gpt-6.1-sol", "claude-opus-5-5")


def _git(source: Path, *args: str, data: bytes | None = None) -> bytes:
    return subprocess.run(["git", "-C", str(source), *args], input=data, capture_output=True, check=True).stdout


def snapshot_bytes(source: Path, base: str, head: str, merge_base: str) -> int:
    """单个模型审阅快照的所需字节，逐项复刻 external_review.prepare_review 的公式。"""
    snapshots = (base, head, merge_base)
    objects = {line.split(b" ", 1)[0] for line in _git(source, "rev-list", "--objects", "--no-walk", *snapshots).splitlines()}
    objects.update(_git(source, "rev-list", base, head).splitlines())
    sizes = _git(source, "cat-file", "--batch-check=%(objectsize)", data=b"\n".join(sorted(objects)) + b"\n").splitlines()
    head_files = _git(source, "ls-tree", "-rl", head).splitlines()
    worktree = sum(int(line.split(b"\t", 1)[0].split()[-1]) for line in head_files if line.split()[1] == b"blob")
    return sum(int(size) for size in sizes) + worktree + SPACE_RESERVE


def _existing(path: Path) -> Path:
    while not path.exists():
        path = path.parent
    return path


def check_space(args) -> dict:
    per = snapshot_bytes(Path(args.source), args.base, args.head, args.merge_base)
    need = per * len(MODELS)
    free = shutil.disk_usage(_existing(Path(args.checkout_root))).free
    return {"name": "checkout_root_free_space", "class": "space", "ok": free >= need,
            "detail": {"required_bytes": need, "per_snapshot_bytes": per, "snapshots": len(MODELS), "free_bytes": free}}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--source", required=True)
    ap.add_argument("--base", required=True)
    ap.add_argument("--head", required=True)
    ap.add_argument("--merge-base", required=True)
    ap.add_argument("--checkout-root", required=True)
    ap.add_argument("--proxy-url")
    ap.add_argument("--proxy-log-dir")
    args = ap.parse_args(argv)
    t0 = time.monotonic()
    checks = []
    for check in (check_space,):
        try:
            checks.append(check(args))
        except Exception as error:  # 检查自身失败按未通过处理，不放行。
            checks.append({"name": check.__name__, "class": "tool", "ok": False, "detail": f"{type(error).__name__}: {error}"[:500]})
    ready = all(c["ok"] for c in checks)
    report = {"kind": "review-preflight", "is_review_result": False, "ready": ready, "checks": checks,
              "wall_seconds": round(time.monotonic() - t0, 3)}
    print(json.dumps(report, ensure_ascii=False))
    return 0 if ready else 2


if __name__ == "__main__":
    sys.exit(main())
