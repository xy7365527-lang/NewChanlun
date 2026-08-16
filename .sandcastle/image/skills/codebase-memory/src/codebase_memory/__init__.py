"""codebase-memory MCP integration: local stdio server (knowledge-graph code queries)."""
import os
import shutil

from rlm import McpIntegration

_BINARY = os.path.expanduser("~/.local/bin/codebase-memory-mcp")


def _resolve_binary() -> str:
    # 沙盒镜像里二进制经 npm 全局安装（prefix 在 PATH 上），宿主在 ~/.local/bin。
    if os.path.exists(_BINARY):
        return _BINARY
    found = shutil.which("codebase-memory-mcp")
    return found or _BINARY



class CodebaseMemory(McpIntegration):
    """Knowledge-graph MCP (search_graph / trace_path / query_graph / ...).

    The server is the local ``codebase-memory-mcp`` binary over stdio; each
    call opens a fresh session (the base class re-connects per call).
    """

    server = "codebase-memory"

    async def _open_session(self, stack):
        from mcp import ClientSession, StdioServerParameters
        from mcp.client.stdio import stdio_client

        binary = _resolve_binary()
        if not (os.path.exists(binary) or shutil.which(binary)):
            raise RuntimeError(
                f"{binary} not found — install codebase-memory-mcp first"
            )
        params = StdioServerParameters(command=binary, args=[], env=None)
        read, write = await stack.enter_async_context(stdio_client(params))
        session = await stack.enter_async_context(ClientSession(read, write))
        await session.initialize()
        return session


codebase_memory = CodebaseMemory()

# Forward bare module access (`await codebase_memory.search_graph(...)`) to the
# instance, but not the names the kernel bootstrap probes.
_RESERVED = {"run", "__wrapped__", "__call__"}


def __getattr__(name):
    if name.startswith("_") or name in _RESERVED:
        raise AttributeError(name)
    return getattr(codebase_memory, name)
