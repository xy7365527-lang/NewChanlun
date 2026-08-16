"""cli-hub-mcp — CLI-Hub (CLI-Anything registry) exposed as MCP tools.

薄包装：直接 import cli_hub 的 registry/installer API，不走子进程。
launch 例外——官方实现用 os.execvp（替换进程），server 里不能用，
改为 subprocess 捕获输出（对 agent 更友好：拿回 stdout 而不是接管终端）。
"""
from __future__ import annotations

import shutil
import subprocess

from fastmcp import FastMCP
from cli_hub import installer, registry

mcp = FastMCP("cli-hub")

MAX_OUTPUT = 20000
_SLIM_KEYS = ("name", "display_name", "description", "category", "entry_point", "_source")


def _slim(cli: dict) -> dict:
    return {k: cli[k] for k in _SLIM_KEYS if k in cli}


@mcp.tool()
def search(query: str) -> list[dict]:
    """Search CLI-Hub for agent-native CLIs by keyword (matches name, description, category).

    Use this first when a task needs to operate external software (image/video/3D/office apps, etc.).
    """
    return [_slim(c) for c in registry.search_clis(query)]


@mcp.tool()
def list_clis(category: str | None = None) -> list[dict]:
    """List all CLIs available in CLI-Hub, optionally filtered by category."""
    clis = registry.fetch_all_clis()
    if category:
        clis = [c for c in clis if c.get("category", "").lower() == category.lower()]
    return [_slim(c) for c in clis]


@mcp.tool()
def info(name: str) -> dict:
    """Get full registry details for one CLI (install method, entry point, requirements)."""
    cli = registry.get_cli(name)
    if cli is None:
        return {"error": f"CLI '{name}' not found in registry."}
    return cli


@mcp.tool()
def installed() -> dict:
    """List CLIs already installed on this machine via CLI-Hub."""
    return installer.get_installed()


@mcp.tool()
def install(name: str) -> dict:
    """Install a CLI from CLI-Hub. May require the upstream application (e.g. GIMP) to be present."""
    ok, msg = installer.install_cli(name)
    return {"success": ok, "message": msg}


@mcp.tool()
def uninstall(name: str) -> dict:
    """Uninstall a CLI previously installed from CLI-Hub."""
    ok, msg = installer.uninstall_cli(name)
    return {"success": ok, "message": msg}


@mcp.tool()
def update(name: str) -> dict:
    """Update an installed CLI to the latest registry version."""
    ok, msg = installer.update_cli(name)
    return {"success": ok, "message": msg}


@mcp.tool()
def launch(name: str, args: list[str] | None = None, timeout: int = 120) -> dict:
    """Run an installed CLI with arguments and return its output (usually JSON on stdout).

    The CLI must be installed first (see install). Output is truncated at 20000 chars.
    """
    cli = registry.get_cli(name)
    if cli is None:
        return {"error": f"CLI '{name}' not found in registry."}
    entry = cli["entry_point"]
    if not shutil.which(entry):
        return {"error": f"'{entry}' not on PATH. Install it first: cli-hub install {name}"}
    try:
        proc = subprocess.run(
            [entry, *(args or [])],
            capture_output=True, text=True, timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return {"error": f"timed out after {timeout}s"}
    out = proc.stdout
    if len(out) > MAX_OUTPUT:
        out = out[:MAX_OUTPUT] + f"\n... [truncated, {len(proc.stdout) - MAX_OUTPUT} chars dropped]"
    return {"exit_code": proc.returncode, "stdout": out, "stderr": proc.stderr[-4000:]}


if __name__ == "__main__":
    mcp.run()
