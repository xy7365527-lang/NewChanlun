#!/usr/bin/env python3
"""原脚本原样执行；可将参考输入只保留 prefixes，产物始终写入本审查目录。"""
from __future__ import annotations

import hashlib
import json
import runpy
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
AUTHOR = ROOT.parent / "live-author"
E = ROOT.parents[1]
mode = sys.argv[1]
assert mode in ("baseline", "prefix-only")
destination = ROOT / ("replay-" + mode)
destination.mkdir(exist_ok=True)
reference = E / "stage66/nested-author/run/reference.stdout"
prefixes = E / "stage66/nested-review/independent-prefixes.json"
original = Path.read_text
reads: list[dict[str, object]] = []


def read_text(self: Path, *args: object, **kwargs: object) -> str:
    if self == reference:
        data = ("{\"prefixes\":" + original(prefixes) + "}") if mode == "prefix-only" else original(self, *args, **kwargs)
        # 回执只读顶层键；对照替代文本不含任何最终 groups/strokes。
        reads.append({"path": str(self), "mode": mode, "top_level_keys": list(json.loads(data)),
                      "online_seal_already_exists": (destination / "online-seal.json").exists(),
                      "delivered_text_sha256": hashlib.sha256(data.encode()).hexdigest()})
        return data
    return original(self, *args, **kwargs)


started = time.perf_counter()
Path.read_text = read_text
sys.argv = [str(AUTHOR / "check_live.py"), str(destination)]
try:
    runpy.run_path(str(AUTHOR / "check_live.py"), run_name="__main__")
finally:
    Path.read_text = original
    (ROOT / ("replay-" + mode + "-receipt.json")).write_text(json.dumps({
        "mode": mode, "source_script_unchanged": str(AUTHOR / "check_live.py"),
        "output": str(destination), "reference_reads": reads,
        "elapsed_seconds": time.perf_counter() - started,
    }, ensure_ascii=False, indent=2) + "\n")
