#!/usr/bin/env python3
"""
Local proxy that strips Anthropic thinking blocks from request history.

Why:
- Some upstream proxy implementations return thinking blocks with empty
  signatures, which causes second-turn 400 errors when clients replay history.
- This proxy removes replayed thinking blocks before forwarding to upstream.

Usage:
  python scripts/anthropic_thinking_sanitizer_proxy.py \
    --upstream https://cc.zhihuiapi.top --host 127.0.0.1 --port 8787

Then point Claude Code to:
  ANTHROPIC_BASE_URL=http://127.0.0.1:8787
"""

from __future__ import annotations

import argparse
import json
import os
import posixpath
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Dict, List, Tuple
from urllib.parse import urljoin

import requests


HOP_BY_HOP_HEADERS = {
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
}


def _normalize_upstream_base(url: str) -> str:
    normalized = url.strip().rstrip("/")
    if not normalized.startswith("http://") and not normalized.startswith("https://"):
        raise ValueError(f"Invalid upstream URL: {url!r}")
    return normalized + "/"


def _join_upstream(base: str, path_with_query: str) -> str:
    # Keep path+query untouched, but guard against accidental absolute path join issues.
    path = path_with_query if path_with_query.startswith("/") else "/" + path_with_query
    return urljoin(base, path.lstrip("/"))


def _strip_thinking_blocks_in_list(items: List[Any]) -> Tuple[List[Any], int]:
    cleaned: List[Any] = []
    removed = 0
    for item in items:
        if isinstance(item, dict) and item.get("type") == "thinking":
            removed += 1
            continue
        cleaned_item, child_removed = _strip_thinking_recursive(item)
        cleaned.append(cleaned_item)
        removed += child_removed
    return cleaned, removed


def _strip_thinking_recursive(value: Any) -> Tuple[Any, int]:
    removed = 0
    if isinstance(value, list):
        cleaned_list, removed = _strip_thinking_blocks_in_list(value)
        return cleaned_list, removed
    if isinstance(value, dict):
        cleaned_obj: Dict[str, Any] = {}
        for k, v in value.items():
            cleaned_v, child_removed = _strip_thinking_recursive(v)
            cleaned_obj[k] = cleaned_v
            removed += child_removed
        return cleaned_obj, removed
    return value, 0


def sanitize_messages_payload(payload: Dict[str, Any]) -> Tuple[Dict[str, Any], int]:
    """
    Remove:
    - top-level `thinking` field (so new thinking blocks are not requested)
    - any content blocks where `type == "thinking"` (replayed history cleanup)
    """
    cleaned, removed = _strip_thinking_recursive(payload)

    if isinstance(cleaned, dict) and "thinking" in cleaned:
        del cleaned["thinking"]
        removed += 1

    return cleaned, removed


class ProxyHandler(BaseHTTPRequestHandler):
    upstream_base: str = ""
    request_timeout_s: int = 600

    protocol_version = "HTTP/1.1"

    def log_message(self, format: str, *args: Any) -> None:
        sys.stdout.write(
            "%s - - [%s] %s\n"
            % (self.address_string(), self.log_date_time_string(), format % args)
        )
        sys.stdout.flush()

    def do_GET(self) -> None:
        self._forward()

    def do_POST(self) -> None:
        self._forward()

    def do_PUT(self) -> None:
        self._forward()

    def do_PATCH(self) -> None:
        self._forward()

    def do_DELETE(self) -> None:
        self._forward()

    def do_OPTIONS(self) -> None:
        self._forward()

    def _forward(self) -> None:
        try:
            self._forward_impl()
        except Exception as exc:  # pragma: no cover - defensive runtime path
            body = json.dumps({"error": "proxy_internal_error", "detail": str(exc)}).encode(
                "utf-8"
            )
            self.send_response(502)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    def _forward_impl(self) -> None:
        target_url = _join_upstream(self.upstream_base, self.path)

        content_length = int(self.headers.get("Content-Length", "0"))
        request_body = self.rfile.read(content_length) if content_length > 0 else b""

        incoming_headers = {k: v for k, v in self.headers.items()}
        forward_headers = {
            k: v
            for k, v in incoming_headers.items()
            if k.lower() not in HOP_BY_HOP_HEADERS
            and k.lower() not in {"host", "content-length"}
        }
        # Avoid compressed upstream payloads so streaming passthrough is simpler.
        forward_headers["Accept-Encoding"] = "identity"

        is_messages_post = self.command.upper() == "POST" and self.path.startswith("/v1/messages")
        removed_count = 0

        if is_messages_post and request_body:
            content_type = incoming_headers.get("Content-Type", "")
            if "application/json" in content_type:
                parsed = json.loads(request_body.decode("utf-8"))
                if isinstance(parsed, dict):
                    sanitized, removed_count = sanitize_messages_payload(parsed)
                    request_body = json.dumps(
                        sanitized, ensure_ascii=False, separators=(",", ":")
                    ).encode("utf-8")
                    forward_headers["Content-Type"] = "application/json"

        upstream_resp = requests.request(
            method=self.command.upper(),
            url=target_url,
            headers=forward_headers,
            data=request_body if request_body else None,
            stream=True,
            timeout=(30, self.request_timeout_s),
        )

        is_sse = "text/event-stream" in upstream_resp.headers.get("Content-Type", "").lower()

        self.send_response(upstream_resp.status_code)
        for key, value in upstream_resp.headers.items():
            low = key.lower()
            if low in HOP_BY_HOP_HEADERS:
                continue
            if low == "content-length":
                # For stream passthrough we do not know the effective body length here.
                if is_sse:
                    continue
            self.send_header(key, value)
        if is_messages_post:
            self.send_header("X-Thinking-Blocks-Removed", str(removed_count))
        self.send_header("Connection", "close")
        self.end_headers()

        if is_sse:
            for chunk in upstream_resp.iter_content(chunk_size=1024):
                if chunk:
                    self.wfile.write(chunk)
                    self.wfile.flush()
        else:
            body = upstream_resp.content
            if body:
                self.wfile.write(body)
                self.wfile.flush()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Forward Anthropic-compatible traffic and strip replayed thinking blocks."
    )
    parser.add_argument(
        "--upstream",
        default=os.environ.get("UPSTREAM_ANTHROPIC_BASE_URL", ""),
        help="Upstream Anthropic-compatible base URL, e.g. https://cc.zhihuiapi.top",
    )
    parser.add_argument("--host", default="127.0.0.1", help="Bind host (default: 127.0.0.1)")
    parser.add_argument("--port", type=int, default=8787, help="Bind port (default: 8787)")
    parser.add_argument(
        "--timeout",
        type=int,
        default=600,
        help="Upstream read timeout in seconds (default: 600)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.upstream:
        print(
            "Missing upstream URL. Provide --upstream or UPSTREAM_ANTHROPIC_BASE_URL.",
            file=sys.stderr,
        )
        return 2

    upstream_base = _normalize_upstream_base(args.upstream)
    if "127.0.0.1" in upstream_base or "localhost" in upstream_base:
        print("Refusing upstream localhost URL to avoid proxy loops.", file=sys.stderr)
        return 2

    ProxyHandler.upstream_base = upstream_base
    ProxyHandler.request_timeout_s = args.timeout

    server = ThreadingHTTPServer((args.host, args.port), ProxyHandler)
    print(f"Thinking-sanitizer proxy listening on http://{args.host}:{args.port}")
    print(f"Forwarding to {upstream_base}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
