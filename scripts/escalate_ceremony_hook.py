#!/usr/bin/env python
"""Ceremony escalate hook - routes escalates during autonomous ceremony execution.

When a ceremony workstation encounters an /escalate, this hook:
1. Grades the escalate (L1/L2/L3) via escalate_router
2. L1: logs and continues (agent self-resolve)
3. L2/L3: formats package and sends to claude.ai via escalate_send
4. Returns exit code for ceremony to decide: continue or suspend workstation

Usage from ceremony:
    python scripts/escalate_ceremony_hook.py --message "矛盾描述" --source "谱系号" --context "上下文"
    python scripts/escalate_ceremony_hook.py --message "..." --dry-run

Exit codes:
    0: L1 (resolved internally, ceremony continues)
    1: L2/L3 sent successfully (workstation should suspend, others continue)
    2: L2/L3 send failed (log error, ceremony continues without escalate response)
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

SCRIPTS_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPTS_DIR.parent
L1_LOG_PATH = PROJECT_ROOT / "tmp" / "escalate_l1.log"
HOOK_LOG_PATH = PROJECT_ROOT / "tmp" / "escalate_hook.log"
ROUTER_SCRIPT = SCRIPTS_DIR / "escalate_router.py"
SEND_SCRIPT = SCRIPTS_DIR / "escalate_send.py"
OPENCLAW_CONFIG = Path.home() / ".openclaw" / "openclaw.json"


# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------

def _timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _log(path: Path, level: str, message: str, source: str = "") -> None:
    """Append a log entry. Creates parent dirs if needed."""
    path.parent.mkdir(parents=True, exist_ok=True)
    source_tag = f" [{source}]" if source else ""
    line = f"[{_timestamp()}] {level}{source_tag}: {message}\n"
    with open(path, "a", encoding="utf-8") as f:
        f.write(line)


# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

def _load_conversation_url() -> str:
    """Load target conversation URL from OpenClaw config.

    Path: escalate.target_conversation in openclaw.json
    """
    if not OPENCLAW_CONFIG.is_file():
        raise FileNotFoundError(
            f"OpenClaw 配置不存在: {OPENCLAW_CONFIG}\n"
            "escalate hook 需要 escalate.target_conversation 字段"
        )

    data = json.loads(OPENCLAW_CONFIG.read_text(encoding="utf-8"))
    url = data.get("escalate", {}).get("target_conversation", "")
    if not url:
        raise ValueError(
            "OpenClaw 配置中缺少 escalate.target_conversation 字段"
        )
    return url.strip()


# ---------------------------------------------------------------------------
# Grade (delegates to escalate_router)
# ---------------------------------------------------------------------------

def _grade(message: str) -> dict:
    """Call escalate_router.py grade and return parsed JSON result."""
    result = subprocess.run(
        [sys.executable, str(ROUTER_SCRIPT), "grade", message],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"escalate_router.py grade 失败:\n{result.stderr}"
        )
    return json.loads(result.stdout)


# ---------------------------------------------------------------------------
# Send (delegates to escalate_send)
# ---------------------------------------------------------------------------

def _send(package_text: str, conversation_url: str) -> str:
    """Call escalate_send.py to deliver the package via Playwright.

    Returns the assistant response text.
    """
    result = subprocess.run(
        [
            sys.executable, str(SEND_SCRIPT),
            "--message", package_text,
            "--url", conversation_url,
        ],
        capture_output=True,
        text=True,
        timeout=420,  # 7 min (send itself has 5 min timeout)
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"escalate_send.py 失败 (code {result.returncode}):\n"
            f"stderr: {result.stderr}\n"
            f"stdout: {result.stdout}"
        )
    return result.stdout.strip()


# ---------------------------------------------------------------------------
# Format package text
# ---------------------------------------------------------------------------

def _format_package(message: str, source: str, context: str, level: int) -> str:
    """Build the escalate package text for sending."""
    lines = [
        f"/escalate L{level}",
        f"来源: {source}" if source else "/escalate",
        f"描述: {message}",
    ]
    if context:
        lines.append(f"上下文: {context}")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main logic
# ---------------------------------------------------------------------------

def run_hook(
    *,
    message: str,
    source: str = "",
    context: str = "",
    dry_run: bool = False,
) -> int:
    """Execute the escalate ceremony hook.

    Returns exit code: 0 (L1), 1 (L2/L3 sent), 2 (L2/L3 send failed).
    """
    # Grade
    grade_result = _grade(message)
    level = grade_result["level"]
    route = grade_result["route"]
    matched = grade_result.get("matched_keywords", [])

    _log(
        HOOK_LOG_PATH, f"L{level}",
        f"grade={route} matched={matched} message={message!r}",
        source=source,
    )

    # L1: log and return 0
    if level == 1:
        _log(
            L1_LOG_PATH, "L1",
            f"self_resolve: {message}",
            source=source,
        )
        print(json.dumps({
            "level": 1,
            "route": "self_resolve",
            "action": "continue",
            "message": message,
        }, ensure_ascii=False))
        return 0

    # L2/L3: format and send
    package_text = _format_package(message, source, context, level)

    if dry_run:
        _log(HOOK_LOG_PATH, f"L{level}", "DRY_RUN — 不发送", source=source)
        print(json.dumps({
            "level": level,
            "route": route,
            "action": "dry_run",
            "package": package_text,
        }, ensure_ascii=False))
        return 1

    try:
        conversation_url = _load_conversation_url()
        response = _send(package_text, conversation_url)
        _log(
            HOOK_LOG_PATH, f"L{level}",
            f"SENT to {conversation_url} — response length={len(response)}",
            source=source,
        )
        print(json.dumps({
            "level": level,
            "route": route,
            "action": "sent",
            "response_preview": response[:500] if response else "",
        }, ensure_ascii=False))
        return 1
    except Exception as exc:
        _log(
            HOOK_LOG_PATH, f"L{level}",
            f"SEND_FAILED: {exc}",
            source=source,
        )
        print(json.dumps({
            "level": level,
            "route": route,
            "action": "send_failed",
            "error": str(exc),
        }, ensure_ascii=False), file=sys.stderr)
        return 2


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Ceremony escalate hook — grade and route escalates",
        prog="escalate_ceremony_hook.py",
    )
    parser.add_argument(
        "--message", required=True,
        help="矛盾描述文本",
    )
    parser.add_argument(
        "--source", default="",
        help="来源谱系号（如 '410号'）",
    )
    parser.add_argument(
        "--context", default="",
        help="额外上下文信息",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="L2/L3 不实际发送，仅输出包内容",
    )
    return parser


def main() -> None:
    parser = _build_parser()
    args = parser.parse_args()

    exit_code = run_hook(
        message=args.message,
        source=args.source,
        context=args.context,
        dry_run=args.dry_run,
    )
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
