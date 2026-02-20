"""Tests for newchan.mcp_bridge — MCP 桥接器模块。

外部依赖（MCP 服务器进程）全部 mock，只测试桥接器自身逻辑。
"""

from __future__ import annotations

from dataclasses import FrozenInstanceError
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from newchan.mcp_bridge import (
    McpBridge,
    McpServerConfig,
    SerenaConfig,
    ToolDefinition,
    ToolResult,
    _extract_text,
    _to_stdio_params,
    _tool_to_definition,
)
from mcp.types import TextContent


# ── 数据类不可变性 ──


class TestDataclassImmutability:
    def test_mcp_server_config_frozen(self):
        cfg = McpServerConfig(command="echo", args=("hello",))
        with pytest.raises(FrozenInstanceError):
            cfg.command = "other"

    def test_tool_definition_frozen(self):
        td = ToolDefinition(name="t", description="d")
        with pytest.raises(FrozenInstanceError):
            td.name = "other"

    def test_tool_result_frozen(self):
        tr = ToolResult(content="ok")
        with pytest.raises(FrozenInstanceError):
            tr.content = "changed"


# ── McpServerConfig ──


class TestMcpServerConfig:
    def test_defaults(self):
        cfg = McpServerConfig(command="cmd")
        assert cfg.args == ()
        assert cfg.cwd is None
        assert cfg.env is None

    def test_full_construction(self):
        cfg = McpServerConfig(
            command="uvx",
            args=("--from", "pkg", "run"),
            cwd="/tmp",
            env={"KEY": "VAL"},
        )
        assert cfg.command == "uvx"
        assert cfg.args == ("--from", "pkg", "run")
        assert cfg.cwd == "/tmp"
        assert cfg.env == {"KEY": "VAL"}


# ── SerenaConfig ──


class TestSerenaConfig:
    def test_returns_mcp_server_config(self):
        cfg = SerenaConfig()
        assert isinstance(cfg, McpServerConfig)
        assert cfg.command == "uvx"
        assert "serena" in cfg.args

    def test_custom_cwd(self):
        cfg = SerenaConfig(cwd="/my/project")
        assert cfg.cwd == "/my/project"

    def test_default_cwd_is_project_root(self):
        cfg = SerenaConfig()
        assert cfg.cwd is not None
        assert len(cfg.cwd) > 0


# ── _to_stdio_params ──


class TestToStdioParams:
    def test_converts_config_to_params(self):
        cfg = McpServerConfig(
            command="echo",
            args=("a", "b"),
            cwd="/tmp",
            env={"K": "V"},
        )
        params = _to_stdio_params(cfg)
        assert params.command == "echo"
        assert params.args == ["a", "b"]  # tuple -> list
        assert params.cwd == "/tmp"
        assert params.env == {"K": "V"}


# ── _tool_to_definition ──


class TestToolToDefinition:
    def test_converts_mcp_tool(self):
        mock_tool = SimpleNamespace(
            name="read_file",
            description="Read a file",
            inputSchema={"type": "object", "properties": {"path": {"type": "string"}}},
        )
        td = _tool_to_definition(mock_tool)
        assert td.name == "read_file"
        assert td.description == "Read a file"
        assert "properties" in td.parameters

    def test_handles_none_description(self):
        mock_tool = SimpleNamespace(name="t", description=None, inputSchema=None)
        td = _tool_to_definition(mock_tool)
        assert td.description == ""
        assert td.parameters == {}


# ── _extract_text ──


class TestExtractText:
    def test_extracts_text_content(self):
        result = SimpleNamespace(
            content=[
                TextContent(type="text", text="hello"),
                TextContent(type="text", text="world"),
            ]
        )
        assert _extract_text(result) == "hello\nworld"

    def test_ignores_non_text_content(self):
        non_text = SimpleNamespace(type="image", data="...")
        result = SimpleNamespace(
            content=[
                TextContent(type="text", text="only this"),
                non_text,
            ]
        )
        assert _extract_text(result) == "only this"

    def test_empty_content(self):
        result = SimpleNamespace(content=[])
        assert _extract_text(result) == ""


# ── McpBridge (无连接状态) ──


class TestMcpBridgeNoConnection:
    def test_initial_state(self):
        cfg = McpServerConfig(command="echo")
        bridge = McpBridge(cfg)
        assert bridge.config is cfg
        assert bridge.is_connected is False

    def test_session_raises_when_not_connected(self):
        bridge = McpBridge(McpServerConfig(command="echo"))
        with pytest.raises(RuntimeError, match="尚未连接"):
            _ = bridge.session


# ── McpBridge (mock 连接) ──


class TestMcpBridgeWithMockConnection:
    @pytest.fixture()
    def mock_session(self):
        session = AsyncMock()
        session.initialize = AsyncMock(
            return_value=SimpleNamespace(
                serverInfo=SimpleNamespace(name="test-server"),
                protocolVersion="1.0",
            )
        )
        session.list_tools = AsyncMock(
            return_value=SimpleNamespace(
                tools=[
                    SimpleNamespace(
                        name="tool_a",
                        description="Tool A",
                        inputSchema={"type": "object"},
                    ),
                ]
            )
        )
        session.call_tool = AsyncMock(
            return_value=SimpleNamespace(
                content=[TextContent(type="text", text="result text")],
                isError=False,
            )
        )
        return session

    @pytest.fixture()
    def connected_bridge(self, mock_session):
        """创建一个已 mock 连接的 bridge。"""
        cfg = McpServerConfig(command="echo")
        bridge = McpBridge(cfg)
        bridge._session = mock_session
        bridge._cm_stack = AsyncMock()
        return bridge

    @pytest.mark.asyncio
    async def test_get_tools(self, connected_bridge):
        tools = await connected_bridge.get_tools()
        assert len(tools) == 1
        assert tools[0].name == "tool_a"
        assert isinstance(tools, tuple)

    @pytest.mark.asyncio
    async def test_call_tool_success(self, connected_bridge):
        result = await connected_bridge.call_tool("tool_a", {"arg": "val"})
        assert result.content == "result text"
        assert result.is_error is False

    @pytest.mark.asyncio
    async def test_call_tool_error(self, connected_bridge, mock_session):
        mock_session.call_tool = AsyncMock(
            return_value=SimpleNamespace(
                content=[TextContent(type="text", text="error msg")],
                isError=True,
            )
        )
        result = await connected_bridge.call_tool("bad_tool")
        assert result.is_error is True
        assert result.content == "error msg"

    @pytest.mark.asyncio
    async def test_disconnect(self, connected_bridge):
        assert connected_bridge.is_connected is True
        await connected_bridge.disconnect()
        assert connected_bridge.is_connected is False

    @pytest.mark.asyncio
    async def test_disconnect_idempotent(self, connected_bridge):
        await connected_bridge.disconnect()
        await connected_bridge.disconnect()  # should not raise
        assert connected_bridge.is_connected is False

    @pytest.mark.asyncio
    async def test_connect_raises_if_already_connected(self, connected_bridge):
        with pytest.raises(RuntimeError, match="已连接"):
            await connected_bridge.connect()
