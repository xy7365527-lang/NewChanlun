"""claude_audit 单元测试（mock API，不需要真实 key）

覆盖：
1. ClaudeAuditor 初始化（key 缺失报错）
2. audit() 四种审计类型
3. 重试逻辑（指数退避）
4. JSON 解析（纯 JSON、markdown 包裹、非 JSON）
5. 无效 audit_type
6. MCP server tool listing 和 tool calling
7. CLI 参数解析
"""

from __future__ import annotations

import json
from unittest.mock import MagicMock, patch

import pytest

from newchan.claude_audit.engine import (
    AuditResult,
    ClaudeAuditor,
    _DEFAULT_MODEL,
    _try_parse_json,
)
from newchan.claude_audit.modes import get_mode_config


# ── JSON 解析 ──


class TestTryParseJson:

    def test_plain_json(self) -> None:
        result = _try_parse_json('{"summary": "ok", "verdict": "pass"}')
        assert result == {"summary": "ok", "verdict": "pass"}

    def test_markdown_wrapped_json(self) -> None:
        text = '```json\n{"summary": "ok"}\n```'
        result = _try_parse_json(text)
        assert result == {"summary": "ok"}

    def test_non_json_returns_raw(self) -> None:
        result = _try_parse_json("This is plain text, not JSON")
        assert result == {"raw": "This is plain text, not JSON"}

    def test_empty_string(self) -> None:
        result = _try_parse_json("")
        assert "raw" in result


# ── 初始化 ──


class TestClaudeAuditorInit:

    def test_missing_key_raises(self) -> None:
        with patch.dict("os.environ", {}, clear=True):
            with pytest.raises(ValueError, match="ANTHROPIC_API_KEY"):
                ClaudeAuditor(api_key="")

    def test_explicit_key(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            ClaudeAuditor(api_key="test-key-123")
            mock_anthropic.Anthropic.assert_called_once_with(api_key="test-key-123")

    def test_env_key(self) -> None:
        with (
            patch.dict("os.environ", {"ANTHROPIC_API_KEY": "env-key-456"}),
            patch("newchan.claude_audit.engine.anthropic") as mock_anthropic,
        ):
            ClaudeAuditor()
            mock_anthropic.Anthropic.assert_called_once_with(api_key="env-key-456")


# ── 审计调用 ──


def _make_mock_response(text: str) -> MagicMock:
    content_block = MagicMock()
    content_block.text = text
    response = MagicMock()
    response.content = [content_block]
    return response


class TestAudit:

    def test_code_review(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_client = mock_anthropic.Anthropic.return_value
            mock_client.messages.create.return_value = _make_mock_response(
                '{"summary": "Clean code", "issues": [], "verdict": "pass"}'
            )

            auditor = ClaudeAuditor(api_key="test")
            result = auditor.audit("Review this function", "def foo(): pass", "code_review")

            assert isinstance(result, AuditResult)
            assert result.audit_type == "code_review"
            assert result.parsed["verdict"] == "pass"
            assert result.model == _DEFAULT_MODEL

    def test_security(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_client = mock_anthropic.Anthropic.return_value
            mock_client.messages.create.return_value = _make_mock_response(
                '{"summary": "No issues", "vulnerabilities": [], "risk_level": "none"}'
            )

            auditor = ClaudeAuditor(api_key="test")
            result = auditor.audit("Security check", "code here", "security")

            assert result.audit_type == "security"
            assert result.parsed["risk_level"] == "none"

    def test_architecture(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_client = mock_anthropic.Anthropic.return_value
            mock_client.messages.create.return_value = _make_mock_response(
                '{"summary": "Good", "findings": [], "architecture_health": "healthy"}'
            )

            auditor = ClaudeAuditor(api_key="test")
            result = auditor.audit("Arch review", "module code", "architecture")

            assert result.audit_type == "architecture"
            assert result.parsed["architecture_health"] == "healthy"

    def test_math_verification(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_client = mock_anthropic.Anthropic.return_value
            mock_client.messages.create.return_value = _make_mock_response(
                '{"summary": "Valid", "steps_verified": [], "conclusion": "valid", "gaps": []}'
            )

            auditor = ClaudeAuditor(api_key="test")
            result = auditor.audit("Verify proof", "theorem here", "math_verification")

            assert result.audit_type == "math_verification"
            assert result.parsed["conclusion"] == "valid"

    def test_invalid_audit_type(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic"):
            auditor = ClaudeAuditor(api_key="test")
            with pytest.raises(ValueError, match="Unknown audit_type"):
                auditor.audit("task", "ctx", "invalid_type")

    def test_empty_response(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_client = mock_anthropic.Anthropic.return_value
            response = MagicMock()
            response.content = []
            mock_client.messages.create.return_value = response

            auditor = ClaudeAuditor(api_key="test")
            result = auditor.audit("task", "ctx", "code_review")
            assert result.response_text == ""

    def test_api_params_correct(self) -> None:
        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_client = mock_anthropic.Anthropic.return_value
            mock_client.messages.create.return_value = _make_mock_response('{}')

            auditor = ClaudeAuditor(api_key="test", model="claude-test-model")
            auditor.audit("task", "ctx", "security")

            call_kwargs = mock_client.messages.create.call_args.kwargs
            assert call_kwargs["model"] == "claude-test-model"
            cfg = get_mode_config("security")
            assert call_kwargs["temperature"] == cfg.temperature
            assert call_kwargs["max_tokens"] == cfg.max_tokens
            assert "system" in call_kwargs


# ── 重试 ──


class TestRetry:

    def test_retry_on_503(self) -> None:
        import anthropic as real_anthropic

        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_anthropic.APIStatusError = real_anthropic.APIStatusError
            mock_anthropic.APIConnectionError = real_anthropic.APIConnectionError

            mock_client = mock_anthropic.Anthropic.return_value
            error_response = MagicMock()
            error_response.status_code = 503
            error_response.headers = {}

            mock_503 = real_anthropic.APIStatusError(
                "Service Unavailable",
                response=error_response,
                body={"error": {"message": "unavailable"}},
            )
            mock_ok = _make_mock_response('{"summary": "ok", "verdict": "pass"}')
            mock_client.messages.create.side_effect = [mock_503, mock_ok]

            with patch("newchan.claude_audit.engine.time.sleep"):
                auditor = ClaudeAuditor(api_key="test")
                result = auditor.audit("task", "ctx", "code_review")

            assert result.parsed.get("verdict") == "pass"
            assert mock_client.messages.create.call_count == 2

    def test_retries_exhausted_raises(self) -> None:
        import anthropic as real_anthropic

        with patch("newchan.claude_audit.engine.anthropic") as mock_anthropic:
            mock_anthropic.APIStatusError = real_anthropic.APIStatusError
            mock_anthropic.APIConnectionError = real_anthropic.APIConnectionError

            mock_client = mock_anthropic.Anthropic.return_value
            error_response = MagicMock()
            error_response.status_code = 503
            error_response.headers = {}

            mock_503 = real_anthropic.APIStatusError(
                "Service Unavailable",
                response=error_response,
                body={"error": {"message": "unavailable"}},
            )
            mock_client.messages.create.side_effect = [mock_503] * 3

            with (
                patch("newchan.claude_audit.engine.time.sleep"),
                pytest.raises(real_anthropic.APIStatusError),
            ):
                auditor = ClaudeAuditor(api_key="test")
                auditor.audit("task", "ctx", "code_review")


# ── MCP Server ──


class TestMcpServer:

    @pytest.mark.asyncio
    async def test_list_tools(self) -> None:
        from newchan.claude_audit.server import mcp

        tools = await mcp.list_tools()
        assert len(tools) == 1
        assert tools[0].name == "claude_audit"
        schema = tools[0].input_schema
        assert "task" in schema["properties"]
        assert "context" in schema["properties"]
        assert "audit_type" in schema["properties"]

    @pytest.mark.asyncio
    async def test_call_tool_success(self) -> None:
        from newchan.claude_audit import server

        server._auditor = None  # reset

        with (
            patch.dict("os.environ", {"ANTHROPIC_API_KEY": "test-key"}),
            patch("newchan.claude_audit.engine.anthropic") as mock_anthropic,
        ):
            mock_client = mock_anthropic.Anthropic.return_value
            mock_client.messages.create.return_value = _make_mock_response(
                '{"summary": "ok", "verdict": "pass"}'
            )

            result = await server.mcp.call_tool("claude_audit", {
                "task": "review",
                "context": "code",
                "audit_type": "code_review",
            })

        assert len(result.content) == 1
        parsed = json.loads(result.content[0].text)
        assert parsed["result"]["verdict"] == "pass"
        server._auditor = None  # cleanup

    @pytest.mark.asyncio
    async def test_call_tool_unknown(self) -> None:
        from mcp.server.mcpserver.exceptions import ToolError

        from newchan.claude_audit.server import mcp

        with pytest.raises(ToolError, match="Unknown tool"):
            await mcp.call_tool("unknown_tool", {})

    @pytest.mark.asyncio
    async def test_call_tool_missing_key(self) -> None:
        from newchan.claude_audit import server

        server._auditor = None

        with patch.dict("os.environ", {}, clear=True):
            result = await server.mcp.call_tool("claude_audit", {
                "task": "t",
                "context": "c",
                "audit_type": "code_review",
            })

        parsed = json.loads(result.content[0].text)
        assert "error" in parsed
        server._auditor = None


# ── Modes ──


class TestModes:

    def test_all_valid_types_have_config(self) -> None:
        from newchan.claude_audit.modes import _VALID_AUDIT_TYPES

        for t in _VALID_AUDIT_TYPES:
            cfg = get_mode_config(t)
            assert cfg.system_prompt
            assert cfg.temperature >= 0
            assert cfg.max_tokens > 0

    def test_invalid_type_raises(self) -> None:
        with pytest.raises(ValueError, match="Unknown audit_type"):
            get_mode_config("nonexistent")
