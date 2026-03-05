"""MCP Server — 通过 stdio 协议暴露 claude_audit 工具。

使用 mcp SDK 的 Server 类，注册 claude_audit 工具。
启动方式：python -m newchan.claude_audit.server

概念溯源: [新缠论] — 异质审计 MCP 服务端
"""

from __future__ import annotations

import json
import logging

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import TextContent, Tool

from newchan.claude_audit.engine import AuditResult, ClaudeAuditor
from newchan.claude_audit.modes import _VALID_AUDIT_TYPES

logger = logging.getLogger(__name__)

_server = Server("claude-audit")

# 延迟初始化：首次调用时创建
_auditor: ClaudeAuditor | None = None


def _get_auditor() -> ClaudeAuditor:
    global _auditor
    if _auditor is None:
        _auditor = ClaudeAuditor()
    return _auditor


@_server.list_tools()
async def list_tools() -> list[Tool]:
    return [
        Tool(
            name="claude_audit",
            description=(
                "Audit code/text using an external Claude instance via Anthropic API. "
                "Supports: code_review, security, architecture, math_verification."
            ),
            inputSchema={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Audit task description",
                    },
                    "context": {
                        "type": "string",
                        "description": "Code or text to audit",
                    },
                    "audit_type": {
                        "type": "string",
                        "enum": sorted(_VALID_AUDIT_TYPES),
                        "description": "Type of audit to perform",
                    },
                    "model": {
                        "type": "string",
                        "description": (
                            "Claude model to use (default: claude-sonnet-4-6-20250514)"
                        ),
                    },
                },
                "required": ["task", "context", "audit_type"],
            },
        ),
    ]


@_server.call_tool()
async def call_tool(name: str, arguments: dict) -> list[TextContent]:
    if name != "claude_audit":
        return [TextContent(type="text", text=f"Unknown tool: {name}")]

    task = arguments.get("task", "")
    context = arguments.get("context", "")
    audit_type = arguments.get("audit_type", "code_review")
    model = arguments.get("model")

    try:
        auditor = _get_auditor() if model is None else ClaudeAuditor(model=model)
        result: AuditResult = auditor.audit(task, context, audit_type)
        output = {
            "audit_type": result.audit_type,
            "model": result.model,
            "result": result.parsed,
        }
        return [TextContent(type="text", text=json.dumps(output, ensure_ascii=False, indent=2))]
    except ValueError as e:
        return [TextContent(type="text", text=json.dumps({"error": str(e)}, ensure_ascii=False))]
    except Exception as e:
        logger.exception("claude_audit failed")
        return [TextContent(type="text", text=json.dumps({"error": str(e)}, ensure_ascii=False))]


async def run_server() -> None:
    async with stdio_server() as (read_stream, write_stream):
        await _server.run(read_stream, write_stream, _server.create_initialization_options())


def main() -> None:
    import asyncio

    from dotenv import load_dotenv

    load_dotenv()
    logging.basicConfig(level=logging.WARNING)
    asyncio.run(run_server())


if __name__ == "__main__":
    main()
