#!/usr/bin/env python3
"""Deploy project content to IPFS.

Packages core project files (topological-computation engine, scripts,
genealogy, block-topology, definitions) into a staging directory,
then uploads to IPFS via `ipfs add -r`.

Excludes: node_modules, corpora, __pycache__, .venv, .env, *.pyc, logs.

Usage:
    python scripts/ipfs_deploy.py                  # Full deploy
    python scripts/ipfs_deploy.py --dry-run        # Show what would be deployed
    python scripts/ipfs_deploy.py --pin            # Pin root CID after upload
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Directories/files to deploy (relative to repo root)
DEPLOY_MANIFEST = [
    # Engine core
    "topological-computation/engine.py",
    "topological-computation/daemon.py",
    "topological-computation/ceremony.py",
    "topological-computation/genealogy_loader.py",
    "topological-computation/test_engine.py",
    "topological-computation/deploy.py",
    "topological-computation/CEREMONY.md",
    # Chain (IPFS client, merkle, sync, verify)
    "topological-computation/chain/",
    # Swarm
    "topological-computation/swarm/",
    # Signifier net (code only, not corpora)
    "topological-computation/signifier_net/*.py",
    "topological-computation/signifier_net/**/*.py",
    # Frontend dist (built artifacts only)
    "topological-computation/frontend/dist/",
    "topological-computation/frontend/package.json",
    "topological-computation/frontend/vite.config.ts",
    "topological-computation/frontend/index.html",
    "topological-computation/frontend/src/",
    # Data
    "topological-computation/data/",
    # Config files
    "topological-computation/requirements.txt",
    "topological-computation/pyproject.toml",
    # Scripts
    "scripts/topo_indicators.py",
    "scripts/ceremony_scan.py",
    # Genealogy (419 settled records)
    ".chanlun/genealogy/settled/",
    # Block topology
    ".chanlun/block-topology/",
    # Definitions
    ".chanlun/definitions/",
    # Knowledge base
    "缠论知识库.md",
]

# Patterns to exclude even within included directories
EXCLUDE_PATTERNS = {
    "node_modules",
    "__pycache__",
    ".venv",
    ".env",
    ".pytest_cache",
    ".cache",
    "corpora",  # signifier_net/corpora/ is 316M
    ".pyc",
    "multiproc.log",
    "multiproc_clean.log",
    "multiproc_new.log",
    "daemon_loop.log",
    "gateway_bridge.log",
    ".jsonl.lock",
}


def should_exclude(path: Path) -> bool:
    """Check if a path should be excluded from deployment."""
    name = path.name
    for pattern in EXCLUDE_PATTERNS:
        if pattern in str(path) or name == pattern or name.endswith(pattern):
            return True
    return False


def copy_entry(src: Path, dst: Path) -> int:
    """Copy a file or directory, respecting exclusions. Returns file count."""
    if should_exclude(src):
        return 0

    if src.is_file():
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dst)
        return 1

    if src.is_dir():
        count = 0
        for item in sorted(src.iterdir()):
            if should_exclude(item):
                continue
            count += copy_entry(item, dst / item.name)
        return count

    return 0


def resolve_glob(pattern: str, repo_root: Path) -> list[Path]:
    """Resolve a manifest entry (may contain globs) to actual paths."""
    if "*" in pattern:
        return sorted(repo_root.glob(pattern))
    path = repo_root / pattern
    if path.exists():
        return [path]
    return []


def stage_deployment(staging_dir: Path) -> tuple[int, int]:
    """Copy deployment content to staging directory.

    Returns (file_count, dir_count).
    """
    file_count = 0
    dir_count = 0

    for entry in DEPLOY_MANIFEST:
        paths = resolve_glob(entry, REPO_ROOT)
        for src in paths:
            rel = src.relative_to(REPO_ROOT)
            dst = staging_dir / rel
            if src.is_dir():
                dir_count += 1
                file_count += copy_entry(src, dst)
            elif src.is_file():
                file_count += copy_entry(src, dst)

    return file_count, dir_count


def ipfs_add(staging_dir: Path, pin: bool = False) -> str:
    """Run `ipfs add -r` on staging directory, return root CID."""
    cmd = ["ipfs", "add", "-r", "--quieter", str(staging_dir)]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
    if result.returncode != 0:
        print(f"[FAIL] ipfs add failed:\n{result.stderr}", file=sys.stderr)
        sys.exit(1)

    root_cid = result.stdout.strip().split("\n")[-1].strip()

    if pin:
        pin_cmd = ["ipfs", "pin", "add", root_cid]
        pin_result = subprocess.run(pin_cmd, capture_output=True, text=True, timeout=60)
        if pin_result.returncode != 0:
            print(f"[WARN] pin failed: {pin_result.stderr}", file=sys.stderr)
        else:
            print(f"[OK] Pinned: {root_cid}")

    return root_cid


def main() -> None:
    parser = argparse.ArgumentParser(description="Deploy project content to IPFS")
    parser.add_argument("--dry-run", action="store_true", help="Show deployment plan without uploading")
    parser.add_argument("--pin", action="store_true", help="Pin root CID after upload")
    parser.add_argument("--keep-staging", action="store_true", help="Keep staging directory after upload")
    args = parser.parse_args()

    # Check IPFS availability
    if not args.dry_run:
        try:
            result = subprocess.run(["ipfs", "id"], capture_output=True, text=True, timeout=10)
            if result.returncode != 0:
                print("[FAIL] IPFS daemon not running. Start with: ipfs daemon", file=sys.stderr)
                sys.exit(1)
        except FileNotFoundError:
            print("[FAIL] ipfs CLI not found. Install from: https://docs.ipfs.tech/install/", file=sys.stderr)
            sys.exit(1)

    # Create staging directory
    staging_dir = Path(tempfile.mkdtemp(prefix="ipfs-deploy-"))
    content_dir = staging_dir / "newchanlun-topo"

    print("=" * 60)
    print("  IPFS Deployment — NewChanlun Topological Computation")
    print("=" * 60)
    print(f"  Repo root: {REPO_ROOT}")
    print(f"  Staging:   {content_dir}")
    print()

    # Stage content
    print("--- Staging content ---")
    file_count, dir_count = stage_deployment(content_dir)
    staged_size = sum(f.stat().st_size for f in content_dir.rglob("*") if f.is_file())
    print(f"[OK] Staged {file_count} files in {dir_count} directory trees")
    print(f"[OK] Total size: {staged_size / 1024 / 1024:.1f} MB")

    if args.dry_run:
        print()
        print("--- Deployment manifest (dry run) ---")
        for f in sorted(content_dir.rglob("*")):
            if f.is_file():
                rel = f.relative_to(content_dir)
                size = f.stat().st_size
                print(f"  {rel}  ({size:,} bytes)")
        print()
        print(f"Total: {file_count} files, {staged_size / 1024 / 1024:.1f} MB")
        print("Run without --dry-run to upload to IPFS.")
        if not args.keep_staging:
            shutil.rmtree(staging_dir)
        return

    # Upload to IPFS
    print()
    print("--- Uploading to IPFS ---")
    root_cid = ipfs_add(content_dir, pin=args.pin)
    print(f"[OK] Root CID: {root_cid}")

    # Write deployment record
    record = {
        "root_cid": root_cid,
        "file_count": file_count,
        "size_bytes": staged_size,
        "pinned": args.pin,
        "manifest_entries": len(DEPLOY_MANIFEST),
    }
    record_path = REPO_ROOT / "topological-computation" / "chain" / "last_deploy.json"
    record_path.write_text(json.dumps(record, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"[OK] Deployment record: {record_path}")

    # Cleanup
    if not args.keep_staging:
        shutil.rmtree(staging_dir)
        print("[OK] Staging directory cleaned up")

    print()
    print("=" * 60)
    print(f"  IPFS CID: {root_cid}")
    print(f"  Gateway:  https://ipfs.io/ipfs/{root_cid}")
    print(f"  Local:    http://localhost:8080/ipfs/{root_cid}")
    print(f"  Files:    {file_count}")
    print(f"  Size:     {staged_size / 1024 / 1024:.1f} MB")
    print("=" * 60)


if __name__ == "__main__":
    main()
