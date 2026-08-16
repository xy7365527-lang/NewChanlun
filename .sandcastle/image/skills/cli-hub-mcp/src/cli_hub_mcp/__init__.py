"""cli-hub-mcp integration: CLI-Hub (CLI-Anything registry) over stdio MCP."""
import os
import shutil

from rlm import McpIntegration

_SERVER_DIR = os.path.expanduser("~/.agents/mcp/cli-hub-mcp")
_PYTHON = os.path.join(_SERVER_DIR, ".venv", "bin", "python")
_SCRIPT = os.path.join(_SERVER_DIR, "server.py")


class CliHubMcp(McpIntegration):
    """CLI-Hub registry tools (search / list_clis / install / launch / ...).

    The server is the local stdio FastMCP wrapper around the ``cli_hub``
    package (PyPI: ``cli-anything-hub``); each call opens a fresh session.
    """

    server = "cli-hub"

    async def _open_session(self, stack):
        from mcp import ClientSession, StdioServerParameters
        from mcp.client.stdio import stdio_client

        if not (os.path.exists(_PYTHON) and os.path.exists(_SCRIPT)):
            raise RuntimeError(
                f"cli-hub-mcp server not found at {_SERVER_DIR} "
                "(expected .venv/bin/python + server.py)"
            )
        params = StdioServerParameters(command=_PYTHON, args=[_SCRIPT], env=None)
        read, write = await stack.enter_async_context(stdio_client(params))
        session = await stack.enter_async_context(ClientSession(read, write))
        await session.initialize()
        return session


cli_hub_mcp = CliHubMcp()

_RESERVED = {"run", "__wrapped__", "__call__"}


def __getattr__(name):
    if name.startswith("_") or name in _RESERVED:
        raise AttributeError(name)
    return getattr(cli_hub_mcp, name)
