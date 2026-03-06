"""Execution callbacks for the daemon.

Same architecture as auto_feed.py search callbacks:
engine traverses a special edge -> callback fires -> result injected into K_active.

Pure Python, only stdlib (subprocess). No external dependencies.
"""

from __future__ import annotations

import re
import subprocess

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus


# ---------------------------------------------------------------------------
# Test runner
# ---------------------------------------------------------------------------

def run_tests(test_dir: str = ".", test_file: str | None = None) -> dict:
    """Run pytest and return structured results.

    Returns {passed: int, failed: int, errors: list[str], output: str}
    """
    cmd = ["python", "-m", "pytest", "-v", "--tb=short"]
    if test_file:
        cmd.append(test_file)

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
            cwd=test_dir,
        )
    except subprocess.TimeoutExpired:
        return {
            "passed": 0,
            "failed": 0,
            "errors": ["Test execution timed out after 60 seconds"],
            "output": "",
        }
    except FileNotFoundError:
        return {
            "passed": 0,
            "failed": 0,
            "errors": ["pytest not found"],
            "output": "",
        }

    output = result.stdout + result.stderr

    # Parse pytest summary line: "X passed, Y failed, Z errors"
    passed = 0
    failed = 0
    errors: list[str] = []

    summary_match = re.search(
        r"(\d+)\s+passed",
        output,
    )
    if summary_match:
        passed = int(summary_match.group(1))

    fail_match = re.search(
        r"(\d+)\s+failed",
        output,
    )
    if fail_match:
        failed = int(fail_match.group(1))

    error_match = re.search(
        r"(\d+)\s+error",
        output,
    )
    if error_match:
        errors.append(f"{error_match.group(1)} collection errors")

    # Extract individual failure names
    for match in re.finditer(r"FAILED\s+(\S+)", output):
        errors.append(match.group(1))

    if result.returncode != 0 and not failed and not errors:
        errors.append(f"pytest exited with code {result.returncode}")

    return {
        "passed": passed,
        "failed": failed,
        "errors": errors,
        "output": output,
    }


# ---------------------------------------------------------------------------
# Script runner
# ---------------------------------------------------------------------------

def run_script(
    filepath: str,
    args: list[str] | None = None,
    timeout: int = 30,
) -> dict:
    """Run a Python script and return results.

    Returns {returncode: int, stdout: str, stderr: str, success: bool}
    """
    cmd = ["python", filepath]
    if args:
        cmd.extend(args)

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return {
            "returncode": -1,
            "stdout": "",
            "stderr": f"Script timed out after {timeout} seconds",
            "success": False,
        }
    except FileNotFoundError:
        return {
            "returncode": -1,
            "stdout": "",
            "stderr": f"File not found: {filepath}",
            "success": False,
        }

    return {
        "returncode": result.returncode,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "success": result.returncode == 0,
    }


# ---------------------------------------------------------------------------
# Test results -> edges
# ---------------------------------------------------------------------------

def test_result_to_edges(
    result: dict,
    tested_functions: list[str],
) -> list[Edge]:
    """Convert test results to edges for K_active injection.

    Passed test -> tested_func --[dependency]--> test_func (validated by)
    Failed test -> tested_func --[negation]--> test_func (contradicted by)
    """
    edges: list[Edge] = []

    if not tested_functions:
        return edges

    test_vertex_id = "test_run"

    if result["failed"] == 0 and not result["errors"]:
        # All passed: dependency edges (validated)
        for func_id in tested_functions:
            edges.append(Edge(
                source=func_id,
                target=test_vertex_id,
                edge_type=EdgeType.DEPENDENCY,
            ))
    else:
        # Failures: negation edges (contradicted)
        for func_id in tested_functions:
            edges.append(Edge(
                source=func_id,
                target=test_vertex_id,
                edge_type=EdgeType.NEGATION,
            ))

    return edges


# ---------------------------------------------------------------------------
# Daemon callback
# ---------------------------------------------------------------------------

def execution_callback(edge_data: dict, daemon: object) -> None:
    """Callback for daemon: when engine traverses an 'execute' edge.

    1. Run tests
    2. Convert results to edges
    3. Inject into daemon's K_active

    edge_data should contain:
        test_dir: str (directory to run tests in)
        test_file: str | None (specific test file)
        tested_functions: list[str] (vertex IDs of tested functions)
    """
    test_dir = edge_data.get("test_dir", ".")
    test_file = edge_data.get("test_file")
    tested_functions = edge_data.get("tested_functions", [])

    result = run_tests(test_dir=test_dir, test_file=test_file)
    edges = test_result_to_edges(result, tested_functions)

    # Inject test vertex if not present
    k_active: Graph = daemon.k_active  # type: ignore[attr-defined]
    test_vid = "test_run"
    if k_active.vertex(test_vid) is None:
        test_vertex = Vertex(
            id=test_vid,
            status=VertexStatus.ACTIVE,
            content=f"Test run: {result['passed']} passed, {result['failed']} failed",
        )
        k_active = k_active.add_vertex(test_vertex)

    # Inject edges
    existing_edges = {
        (e.source, e.target, e.edge_type.value)
        for e in k_active.edges
    }
    for e in edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if k_active.vertex(e.source) is not None and k_active.vertex(e.target) is not None:
                k_active = k_active.add_edge(e)
                existing_edges.add(key)

    daemon.k_active = k_active  # type: ignore[attr-defined]
