"""MCP Server — 通过 stdio 协议暴露 claude_audit 工具。

使用 mcp SDK 的 MCPServer 高阶类（v2，原 FastMCP），注册 claude_audit 工具。
启动方式：python -m newchan.claude_audit.server

概念溯源: [新缠论] — 异质审计 MCP 服务端
"""

from __future__ import annotations

import json
import logging

from mcp.server.mcpserver import MCPServer
from mcp.types import TextContent

from newchan.claude_audit.engine import AuditResult, ClaudeAuditor

logger = logging.getLogger(__name__)

mcp = MCPServer("claude-audit")

# 延迟初始化：首次调用时创建
_auditor: ClaudeAuditor | None = None


def _get_auditor() -> ClaudeAuditor:
    global _auditor
    if _auditor is None:
        _auditor = ClaudeAuditor()
    return _auditor


@mcp.tool()
async def claude_audit(
    task: str,
    context: str,
    audit_type: str,
    model: str | None = None,
) -> list[TextContent]:
    """Audit code/text using an external Claude instance via Anthropic API.

    Supports: code_review, security, architecture, math_verification.
    """
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


def main() -> None:
    from dotenv import load_dotenv

    load_dotenv()
    logging.basicConfig(level=logging.WARNING)
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
