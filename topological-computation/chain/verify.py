"""End-to-end integrity verification for swarm blocks."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

from chain.merkle import build_merkle_tree, verify_proof


def verify_block(block_path: str, expected_hash: str) -> bool:
    """Verify a single block's content hash matches expected."""
    path = Path(block_path)
    if not path.exists():
        return False
    data = path.read_text(encoding="utf-8")
    # Re-serialize with sorted keys for deterministic hashing
    content = json.loads(data)
    canonical = json.dumps(content, sort_keys=True, ensure_ascii=False)
    actual_hash = hashlib.sha256(canonical.encode()).hexdigest()
    return actual_hash == expected_hash


def verify_merkle(block_hashes: list[str], expected_root: str) -> bool:
    """Verify that block hashes produce expected Merkle root."""
    if not block_hashes:
        return False
    root, _ = build_merkle_tree(block_hashes)
    return root == expected_root


def verify_full(blocks_dir: str) -> dict:
    """Full verification of all blocks in a directory.

    Returns dict with:
      - total: number of blocks found
      - valid: number with valid content hash
      - invalid: list of filenames with hash mismatches
      - merkle_root: computed Merkle root of all valid block hashes
      - block_hashes: list of all block hashes (sorted)
    """
    blocks_path = Path(blocks_dir)
    if not blocks_path.exists():
        return {
            "total": 0,
            "valid": 0,
            "invalid": [],
            "merkle_root": None,
            "block_hashes": [],
        }

    valid_hashes: list[str] = []
    invalid_files: list[str] = []
    total = 0

    for path in sorted(blocks_path.glob("*.json")):
        total += 1
        expected_hash = path.stem  # filename without extension = content hash
        if verify_block(str(path), expected_hash):
            valid_hashes.append(expected_hash)
        else:
            invalid_files.append(path.name)

    merkle_root = None
    if valid_hashes:
        merkle_root, _ = build_merkle_tree(sorted(valid_hashes))

    return {
        "total": total,
        "valid": len(valid_hashes),
        "invalid": invalid_files,
        "merkle_root": merkle_root,
        "block_hashes": sorted(valid_hashes),
    }
