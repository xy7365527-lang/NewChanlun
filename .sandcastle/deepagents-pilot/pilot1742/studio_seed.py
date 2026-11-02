"""服务就绪后为 PILOT1742_STUDIO_RUNS 里的每个 run 建好官方 Studio 线程。

官方 Studio 不会自己建绑定线程，线程还须带 metadata.graph_id 才能出现在列表里、读到状态。用 if_exists=do_nothing
建，已有线程原样保留不改写；已有线程缺 graph_id 时只报出来。"""

from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request

from .studio_binding import GRAPH_ID, bound_runs

LOOPBACK = "http://127.0.0.1"


def _call(base: str, method: str, path: str, body: dict | None = None) -> dict:
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(base + path, data=data, method=method, headers={"content-type": "application/json"})
    with urllib.request.urlopen(request, timeout=10) as response:
        raw = response.read()
        return json.loads(raw) if raw else {}


def seed(port: int, wait_seconds: float = 120.0) -> dict[str, str]:
    base = f"{LOOPBACK}:{port}"
    deadline = time.monotonic() + wait_seconds
    while True:
        try:
            _call(base, "GET", "/ok")
            break
        except (urllib.error.URLError, OSError):
            if time.monotonic() > deadline:
                raise
            time.sleep(1)
    outcome: dict[str, str] = {}
    for thread, run_id in bound_runs().items():
        _call(base, "POST", "/threads", {"thread_id": thread, "metadata": {"graph_id": GRAPH_ID}, "if_exists": "do_nothing"})
        metadata = _call(base, "GET", f"/threads/{thread}").get("metadata") or {}
        outcome[run_id] = thread if metadata.get("graph_id") == GRAPH_ID else f"{thread} 已存在但缺 graph_id，未改写"
    return outcome


def main(argv: list[str]) -> int:
    if not bound_runs():
        return 0
    for run_id, result in seed(int(argv[0])).items():
        print(f"pilot1742 studio thread {run_id}: {result}", file=sys.stderr, flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
