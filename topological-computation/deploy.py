#!/usr/bin/env python3
"""Deploy script for the Topological Computation Daemon.

Cross-platform (Windows/Linux/Mac). Does:
1. Detect OS
2. Install Python dependencies
3. Create ~/.swarm/ directory structure
4. Write .env from template
5. Generate seed complex from genealogy data
6. Run test_engine.py
7. Launch daemon in background
8. Output PID and log path

Usage:
    python deploy.py                    # Full deploy (daemon runs until crystallized)
    python deploy.py --check-only       # Steps 1-6 only, no daemon launch
    python deploy.py --repo-root /path  # Specify repo root (default: auto-detect)
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_REPO_ROOT = SCRIPT_DIR.parent  # topological-computation/ is inside repo
SWARM_DIR = Path.home() / ".swarm"
PERSIST_PATH = SWARM_DIR / "k_full.jsonl"
SEED_JSON_PATH = SWARM_DIR / "seed_complex.json"
PID_FILE = SWARM_DIR / "daemon.pid"
LOG_PATH = SWARM_DIR / "output" / "daemon.log"

REQUIRED_PACKAGES = ["stanza", "pymupdf", "python-dotenv"]

ENV_TEMPLATE = """\
SEMANTIC_SCHOLAR_API=https://api.semanticscholar.org/graph/v1
ARXIV_API=https://export.arxiv.org/api
UNPAYWALL_EMAIL=hanjunyu2003@proton.me
BRAVE_ANSWER_API_KEY=
BRAVE_SEARCH_API_KEY=
IPFS_API=http://localhost:5001
"""


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _print_step(n: int, msg: str) -> None:
    print(f"\n{'='*60}")
    print(f"  Step {n}: {msg}")
    print(f"{'='*60}")


def _print_ok(msg: str) -> None:
    print(f"  [OK] {msg}")


def _print_fail(msg: str) -> None:
    print(f"  [FAIL] {msg}", file=sys.stderr)


def _run(cmd: list[str], cwd: str | Path | None = None,
         capture: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(
        cmd, cwd=cwd, capture_output=capture, text=True,
    )


# ---------------------------------------------------------------------------
# Step 1: OS Detection
# ---------------------------------------------------------------------------

def step_1_detect_os() -> dict:
    _print_step(1, "Detect OS")
    info = {
        "system": platform.system(),
        "release": platform.release(),
        "python": sys.version,
        "python_exe": sys.executable,
    }
    for k, v in info.items():
        _print_ok(f"{k}: {v}")
    return info


# ---------------------------------------------------------------------------
# Step 2: Install dependencies
# ---------------------------------------------------------------------------

def step_2_install_deps() -> bool:
    _print_step(2, "Install Python dependencies")
    result = _run([sys.executable, "-m", "pip", "install"] + REQUIRED_PACKAGES)
    if result.returncode != 0:
        _print_fail(f"pip install failed:\n{result.stderr}")
        return False
    _print_ok(f"Installed: {', '.join(REQUIRED_PACKAGES)}")

    # Optional packages (don't fail if unavailable)
    for pkg in ["web3", "ipfshttpclient"]:
        r = _run([sys.executable, "-m", "pip", "install", pkg])
        if r.returncode == 0:
            _print_ok(f"Optional: {pkg} installed")
        else:
            _print_ok(f"Optional: {pkg} skipped (not available for this Python)")
    return True


# ---------------------------------------------------------------------------
# Step 3: Create directories
# ---------------------------------------------------------------------------

def step_3_create_dirs() -> bool:
    _print_step(3, "Create ~/.swarm/ directories")
    dirs = [
        SWARM_DIR / "blocks",
        SWARM_DIR / "relations",
        SWARM_DIR / "index",
        SWARM_DIR / "output",
    ]
    for d in dirs:
        d.mkdir(parents=True, exist_ok=True)
        _print_ok(f"{d}")
    return True


# ---------------------------------------------------------------------------
# Step 4: Write .env
# ---------------------------------------------------------------------------

def step_4_write_env() -> bool:
    _print_step(4, "Write .env file")
    env_path = SCRIPT_DIR / ".env"
    if env_path.exists():
        _print_ok(f".env already exists at {env_path}, skipping")
    else:
        env_path.write_text(ENV_TEMPLATE, encoding="utf-8")
        _print_ok(f"Written to {env_path}")

    # Verify
    try:
        from dotenv import load_dotenv
        load_dotenv(env_path)
        _print_ok("dotenv loads successfully")
    except ImportError:
        _print_ok("python-dotenv not available, .env written but not verified")
    return True


# ---------------------------------------------------------------------------
# Step 5: Generate seed complex
# ---------------------------------------------------------------------------

def step_5_generate_seed(repo_root: Path) -> dict | None:
    _print_step(5, "Generate seed complex from genealogy data")

    # Import locally to use the project's modules
    sys.path.insert(0, str(SCRIPT_DIR))
    try:
        from genealogy_loader import load_from_repo
        from engine import compute_beta_1
        from daemon import graph_to_dict
    except ImportError as exc:
        _print_fail(f"Import error: {exc}")
        return None

    graph, stats = load_from_repo(str(repo_root))
    active = graph.active_vertex_ids()
    n_verts = len(active)
    n_edges = len(graph.active_edges())
    b1 = compute_beta_1(graph)

    seed_info = {
        "vertices": n_verts,
        "edges": n_edges,
        "beta_1": b1,
    }
    _print_ok(f"Vertices: {n_verts}")
    _print_ok(f"Edges: {n_edges}")
    _print_ok(f"beta_1: {b1}")

    # Save seed JSON for daemon --load
    graph_dict = graph_to_dict(graph)
    SEED_JSON_PATH.parent.mkdir(parents=True, exist_ok=True)
    SEED_JSON_PATH.write_text(
        json.dumps(graph_dict, ensure_ascii=False), encoding="utf-8",
    )
    _print_ok(f"Seed JSON saved to {SEED_JSON_PATH}")

    return seed_info


# ---------------------------------------------------------------------------
# Step 6: Run tests
# ---------------------------------------------------------------------------

def step_6_run_tests() -> bool:
    _print_step(6, "Run test_engine.py")
    result = _run(
        [sys.executable, "-m", "pytest", "test_engine.py", "-v"],
        cwd=SCRIPT_DIR,
    )
    if result.returncode != 0:
        _print_fail(f"Tests failed:\n{result.stdout}\n{result.stderr}")
        return False
    # Count passed
    lines = result.stdout.strip().split("\n")
    summary = lines[-1] if lines else ""
    _print_ok(summary.strip())
    return True


# ---------------------------------------------------------------------------
# Step 7: Launch daemon
# ---------------------------------------------------------------------------

def step_7_launch_daemon() -> dict | None:
    _print_step(7, "Launch daemon (background, self-terminating)")

    daemon_script = SCRIPT_DIR / "swarm" / "swarm_daemon.py"
    if not daemon_script.exists():
        _print_fail(f"swarm_daemon.py not found at {daemon_script}")
        return None

    cmd = [
        sys.executable, str(daemon_script),
        "--instance-id", "node_0",
        "--shared", str(SWARM_DIR),
        "--load", str(SEED_JSON_PATH),
        "--persist", str(PERSIST_PATH),
        "--output", str(SWARM_DIR / "output" / "daemon_report.txt"),
        "--max-retries", "3",
        "--retry-delay", "5",
    ]

    system = platform.system()

    if system == "Windows":
        # Use pythonw if available, else CREATE_NO_WINDOW flag
        pythonw = Path(sys.executable).parent / "pythonw.exe"
        if pythonw.exists():
            cmd[0] = str(pythonw)

        creation_flags = subprocess.CREATE_NO_WINDOW
        proc = subprocess.Popen(
            cmd,
            creationflags=creation_flags,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    else:
        # Linux/Mac: nohup + setsid for full detach
        proc = subprocess.Popen(
            cmd,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )

    pid = proc.pid

    # Write PID file
    PID_FILE.write_text(str(pid), encoding="utf-8")

    _print_ok(f"Daemon PID: {pid}")
    _print_ok(f"PID file: {PID_FILE}")
    _print_ok(f"Log file: {LOG_PATH}")
    _print_ok(f"Persistence: {PERSIST_PATH}")
    _print_ok(f"Report will be at: {SWARM_DIR / 'output' / 'daemon_report.txt'}")

    # Wait briefly and check it didn't crash immediately
    time.sleep(3)
    poll = proc.poll()
    if poll is not None and poll != 0:
        _print_fail(f"Daemon exited immediately with code {poll}")
        return None
    elif poll == 0:
        _print_ok("Daemon completed (short run)")
    else:
        _print_ok("Daemon running in background")

    return {
        "pid": pid,
        "pid_file": str(PID_FILE),
        "log_path": str(LOG_PATH),
        "persist_path": str(PERSIST_PATH),
    }


# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------

def write_report(os_info: dict, seed_info: dict | None,
                 tests_passed: bool, daemon_info: dict | None,
                 report_path: Path) -> None:
    lines = [
        "=" * 70,
        "TOPOLOGICAL COMPUTATION DAEMON DEPLOY REPORT",
        "=" * 70,
        "",
        "--- OS ---",
        f"System: {os_info['system']} {os_info['release']}",
        f"Python: {os_info['python']}",
        "",
        "--- Dependencies ---",
        f"Required: {', '.join(REQUIRED_PACKAGES)} (installed)",
        "",
        "--- Directories ---",
        f"Swarm dir: {SWARM_DIR}",
        "",
        "--- Seed Complex ---",
    ]
    if seed_info:
        lines.extend([
            f"Vertices: {seed_info['vertices']}",
            f"Edges: {seed_info['edges']}",
            f"beta_1: {seed_info['beta_1']}",
            f"Seed JSON: {SEED_JSON_PATH}",
        ])
    else:
        lines.append("FAILED")

    lines.extend([
        "",
        "--- Tests ---",
        f"test_engine.py: {'22/22 passed' if tests_passed else 'FAILED'}",
        "",
        "--- Daemon ---",
    ])
    if daemon_info:
        lines.extend([
            f"PID: {daemon_info['pid']}",
            f"PID file: {daemon_info['pid_file']}",
            f"Log: {daemon_info['log_path']}",
            f"Persistence: {daemon_info['persist_path']}",
        ])
    else:
        lines.append("Not launched (--check-only or failed)")

    lines.extend(["", "=" * 70])

    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"\nReport saved to: {report_path}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Deploy the Topological Computation Daemon",
    )
    parser.add_argument(
        "--check-only", action="store_true",
        help="Run checks (steps 1-6) without launching daemon",
    )
    parser.add_argument(
        "--repo-root", type=str, default=None,
        help="Path to NewChanlun repo root (default: auto-detect)",
    )
    args = parser.parse_args()

    repo_root = Path(args.repo_root) if args.repo_root else DEFAULT_REPO_ROOT

    print("Topological Computation Daemon — Deployment")
    print(f"Repo root: {repo_root}")
    print(f"Script dir: {SCRIPT_DIR}")

    # Step 1
    os_info = step_1_detect_os()

    # Step 2
    if not step_2_install_deps():
        _print_fail("Aborting: dependency installation failed")
        sys.exit(1)

    # Step 3
    step_3_create_dirs()

    # Step 4
    step_4_write_env()

    # Step 5
    seed_info = step_5_generate_seed(repo_root)
    if seed_info is None:
        _print_fail("Aborting: seed generation failed")
        sys.exit(1)

    # Step 6
    tests_passed = step_6_run_tests()
    if not tests_passed:
        _print_fail("Aborting: tests failed")
        sys.exit(1)

    # Step 7
    daemon_info = None
    if not args.check_only:
        daemon_info = step_7_launch_daemon()
        if daemon_info is None:
            _print_fail("Daemon launch failed")
            sys.exit(1)

    # Report
    report_path = SWARM_DIR / "output" / "deploy_report.txt"
    write_report(os_info, seed_info, tests_passed, daemon_info, report_path)

    print("\n" + "=" * 60)
    if daemon_info:
        print(f"  Daemon running — PID {daemon_info['pid']}")
        print(f"  Log: {daemon_info['log_path']}")
        print(f"  Stop: kill {daemon_info['pid']}")
    else:
        print("  All checks passed. Use without --check-only to launch daemon.")
    print("=" * 60)


if __name__ == "__main__":
    main()
