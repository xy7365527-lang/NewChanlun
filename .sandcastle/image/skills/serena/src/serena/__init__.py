"""Serena MCP integration: local streamable-http daemon (semantic code tools)."""
import os
import socket
import subprocess
import time

from rlm import McpIntegration

_SERENA_PORT = int(os.environ.get("SERENA_MCP_PORT", "8017"))
_URL = f"http://127.0.0.1:{_SERENA_PORT}/mcp"
_DAEMON_LOG = os.path.expanduser("~/.serena/mcp-daemon.log")
_READY_TIMEOUT_S = 150.0


def _port_open(host: str = "127.0.0.1", timeout: float = 0.5) -> bool:
    try:
        with socket.create_connection((host, _SERENA_PORT), timeout=timeout):
            return True
    except OSError:
        return False


def _find_project_root(start: str | None = None) -> str | None:
    d = os.path.abspath(start or os.getcwd())
    while True:
        if os.path.exists(os.path.join(d, ".serena", "project.yml")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            return None
        d = parent


_PROJECT_YML_TEMPLATE = """\
# Auto-provisioned by the sandbox image's serena skill (#1010).
# `.serena/` is gitignored in NewChanlun, so fresh sandcastle worktrees have no
# project.yml; the daemon cannot start without it, hence bootstrap-on-first-call.
project_name: "sandbox-worktree"
encoding: "utf-8"
ignore_all_files_in_gitignore: true
read_only: false
excluded_tools: []
included_optional_tools: []
initial_prompt: ""
language_servers:
- rust
- python
ls_workspace_folders:
- .
"""


def _git_root(start: str) -> str | None:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=start, capture_output=True, text=True, timeout=15,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    return out.stdout.strip() or None


def _ensure_project_root() -> str:
    """Find the serena project root, auto-provisioning .serena/project.yml if absent."""
    root = _find_project_root()
    if root is not None:
        return root
    base = _git_root(os.getcwd()) or os.getcwd()
    serena_dir = os.path.join(base, ".serena")
    os.makedirs(serena_dir, exist_ok=True)
    with open(os.path.join(serena_dir, "project.yml"), "w") as f:
        f.write(_PROJECT_YML_TEMPLATE)
    return base


def ensure_daemon() -> str:
    """Ensure the local Serena MCP daemon runs for the current project; return its URL.

    The daemon serves ONE project (activated at startup). It is spawned from the
    nearest directory holding ``.serena/project.yml`` upward from the kernel cwd.
    """
    if not _port_open():
        root = _ensure_project_root()
        os.makedirs(os.path.dirname(_DAEMON_LOG), exist_ok=True)
        logf = open(_DAEMON_LOG, "ab")  # noqa: SIM115
        subprocess.Popen(
            [
                "serena", "start-mcp-server",
                "--transport", "streamable-http",
                "--host", "127.0.0.1",
                "--port", str(_SERENA_PORT),
                "--project", root,
                "--enable-web-dashboard", "false",
                "--open-web-dashboard", "false",
            ],
            cwd=root,
            stdout=logf,
            stderr=logf,
            start_new_session=True,
        )
        deadline = time.monotonic() + _READY_TIMEOUT_S
        while time.monotonic() < deadline:
            if _port_open():
                break
            time.sleep(1.0)
        else:
            raise RuntimeError(
                f"serena daemon did not come up within {_READY_TIMEOUT_S:.0f}s; "
                f"log: {_DAEMON_LOG}"
            )
    return _URL


class Serena(McpIntegration):
    """Semantic code tools over the local Serena daemon (find_symbol, ...).

    First call in a kernel auto-spawns the daemon (LSP cold start ~30-60s);
    later calls reuse it. Tools are discovered via ``list_tools()``.
    """

    server = "serena"

    async def _open_session(self, stack):
        from mcp import ClientSession
        from mcp.client.streamable_http import streamable_http_client

        url = ensure_daemon()
        cm = streamable_http_client(url)
        read, write, *_ = await stack.enter_async_context(cm)
        session = await stack.enter_async_context(ClientSession(read, write))
        await session.initialize()
        return session


serena = Serena()

_RESERVED = {"run", "__wrapped__", "__call__"}


def __getattr__(name):
    if name.startswith("_") or name in _RESERVED:
        raise AttributeError(name)
    return getattr(serena, name)
