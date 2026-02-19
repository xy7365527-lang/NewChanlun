"""MCP 桥接器 — 通用 LLM <-> MCP 工具桥接。

让任意 LLM 通过 MCP 协议访问代码工具（Serena 等）。
支持两种使用模式：
1. 直接传递 ClientSession 给支持 MCP 的 SDK（如 google-genai）
2. 手动 tool dispatch（对不支持 MCP 的 SDK）

概念溯源: [新缠论] — 异质否定源基础设施
"""

from __future__ import annotations

import logging
from contextlib import asynccontextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, AsyncIterator, Sequence

from mcp import ClientSession
from mcp.client.stdio import StdioServerParameters, stdio_client
from mcp.types import TextContent

logger = logging.getLogger(__name__)

__all__ = [
    "McpBridge",
    "McpServerConfig",
    "SerenaConfig",
    "ToolDefinition",
    "ToolResult",
]

# ── 项目根路径（默认 cwd） ──

_PROJECT_ROOT = str(Path(__file__).resolve().parent.parent.parent)


# ── 不可变数据类 ──


@dataclass(frozen=True, slots=True)
class McpServerConfig:
    """MCP 服务器连接配置（不可变）。

    概念溯源: [新缠论] — MCP 服务器参数冻结快照
    """

    command: str
    args: tuple[str, ...] = ()
    cwd: str | None = None
    env: dict[str, str] | None = None


@dataclass(frozen=True, slots=True)
class ToolDefinition:
    """通用工具定义（从 MCP Tool 转换而来）。

    概念溯源: [新缠论] — MCP 工具的 LLM 无关表示
    """

    name: str
    description: str
    parameters: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True, slots=True)
class ToolResult:
    """工具调用结果。

    概念溯源: [新缠论] — MCP 工具调用结果的 LLM 无关表示
    """

    content: str
    is_error: bool = False


def SerenaConfig(cwd: str | None = None) -> McpServerConfig:
    """Serena MCP 服务器的预定义配置。

    Parameters
    ----------
    cwd : str | None
        工作目录。默认为项目根路径。

    概念溯源: [新缠论] — Serena 代码工具连接模板
    """
    return McpServerConfig(
        command="uvx",
        args=(
            "--from",
            "git+https://github.com/oraios/serena",
            "serena",
            "start-mcp-server",
        ),
        cwd=cwd or _PROJECT_ROOT,
    )


# ── 内部工具函数 ──


def _to_stdio_params(config: McpServerConfig) -> StdioServerParameters:
    """将不可变配置转换为 MCP SDK 参数。"""
    return StdioServerParameters(
        command=config.command,
        args=list(config.args),
        env=config.env,
        cwd=config.cwd,
    )


def _tool_to_definition(tool: Any) -> ToolDefinition:
    """将 MCP Tool 对象转换为通用 ToolDefinition。"""
    return ToolDefinition(
        name=tool.name,
        description=tool.description or "",
        parameters=tool.inputSchema or {},
    )


def _extract_text(result: Any) -> str:
    """从 CallToolResult 提取文本内容。"""
    parts: list[str] = []
    for block in result.content:
        if isinstance(block, TextContent):
            parts.append(block.text)
    return "\n".join(parts)


# ── McpBridge ──


class McpBridge:
    """通用 MCP 桥接器 — 管理与 MCP 服务器的生命周期。

    支持两种使用模式：

    1. **async with** 上下文管理器::

        async with McpBridge(config) as bridge:
            session = bridge.session          # 直接传给 google-genai
            tools = await bridge.get_tools()  # 手动 dispatch
            result = await bridge.call_tool("read_file", {"path": "foo.py"})

    2. **手动 connect/disconnect**::

        bridge = McpBridge(config)
        await bridge.connect()
        try:
            ...
        finally:
            await bridge.disconnect()

    概念溯源: [新缠论] — 异质否定源基础设施
    """

    def __init__(self, config: McpServerConfig) -> None:
        self._config = config
        self._session: ClientSession | None = None
        self._cm_stack: Any = None  # async context manager stack

    @property
    def config(self) -> McpServerConfig:
        """连接配置（只读）。"""
        return self._config

    @property
    def session(self) -> ClientSession:
        """已连接的 MCP ClientSession。

        可直接传给支持 MCP 的 SDK（如 google-genai 的 tools 参数）。

        Raises
        ------
        RuntimeError
            如果尚未连接。
        """
        if self._session is None:
            raise RuntimeError(
                "McpBridge 尚未连接。请先调用 connect() 或使用 async with。"
            )
        return self._session

    @property
    def is_connected(self) -> bool:
        """是否已连接。"""
        return self._session is not None

    async def connect(self) -> ClientSession:
        """建立与 MCP 服务器的连接。

        Returns
        -------
        ClientSession
            已初始化的 MCP 会话。

        Raises
        ------
        RuntimeError
            如果已连接（防止重复连接）。
        OSError
            如果服务器进程启动失败。
        """
        if self._session is not None:
            raise RuntimeError("McpBridge 已连接。请先 disconnect() 再重连。")

        params = _to_stdio_params(self._config)
        logger.info(
            "正在连接 MCP 服务器: %s %s (cwd=%s)",
            self._config.command,
            " ".join(self._config.args),
            self._config.cwd,
        )

        # stdio_client 是 async context manager，需要手动管理生命周期
        from contextlib import AsyncExitStack

        stack = AsyncExitStack()
        try:
            read_stream, write_stream = await stack.enter_async_context(
                stdio_client(params)
            )
            session = await stack.enter_async_context(
                ClientSession(read_stream, write_stream)
            )
            init_result = await session.initialize()
            logger.info(
                "MCP 服务器已连接: %s (protocol=%s)",
                init_result.serverInfo.name if init_result.serverInfo else "unknown",
                init_result.protocolVersion,
            )
        except BaseException:
            await stack.aclose()
            raise

        self._session = session
        self._cm_stack = stack
        return session

    async def disconnect(self) -> None:
        """断开与 MCP 服务器的连接。

        安全调用：已断开时不会报错。
        """
        if self._cm_stack is not None:
            logger.info("正在断开 MCP 服务器连接...")
            await self._cm_stack.aclose()
            self._cm_stack = None
        self._session = None

    async def get_tools(self) -> tuple[ToolDefinition, ...]:
        """获取 MCP 服务器提供的所有工具定义。

        Returns
        -------
        tuple[ToolDefinition, ...]
            不可变的工具定义列表。
        """
        result = await self.session.list_tools()
        definitions = tuple(_tool_to_definition(t) for t in result.tools)
        logger.debug("获取到 %d 个工具定义", len(definitions))
        return definitions

    async def call_tool(
        self,
        name: str,
        arguments: dict[str, Any] | None = None,
    ) -> ToolResult:
        """调用 MCP 工具。

        Parameters
        ----------
        name : str
            工具名称。
        arguments : dict | None
            工具参数。

        Returns
        -------
        ToolResult
            工具调用结果。
        """
        logger.debug("调用工具: %s(%s)", name, arguments)
        result = await self.session.call_tool(name, arguments)
        text = _extract_text(result)
        if result.isError:
            logger.warning("工具 %s 返回错误: %s", name, text[:200])
        return ToolResult(content=text, is_error=result.isError)

    async def __aenter__(self) -> McpBridge:
        await self.connect()
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.disconnect()


# ── 便捷上下文管理器 ──


@asynccontextmanager
async def mcp_session(
    config: McpServerConfig,
) -> AsyncIterator[ClientSession]:
    """便捷函数：只获取 ClientSession（适合直接传给 google-genai）。

    Usage::

        async with mcp_session(SerenaConfig()) as session:
            response = await client.aio.models.generate_content(
                ..., tools=[session]
            )

    概念溯源: [新缠论] — MCP session 薄包装
    """
    async with McpBridge(config) as bridge:
        yield bridge.session
