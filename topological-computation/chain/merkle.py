"""Pure Python Merkle tree: build, prove, verify. Zero dependencies."""
from __future__ import annotations

import hashlib


def _hash_pair(a: str, b: str) -> str:
    """Hash two hex-digest strings together (sorted for consistency)."""
    left, right = sorted([a, b])
    return hashlib.sha256((left + right).encode()).hexdigest()


def build_merkle_tree(hashes: list[str]) -> tuple[str, list[list[str]]]:
    """Build Merkle tree from leaf hashes.

    Returns (root_hash, tree_levels) where tree_levels[0] = leaves,
    tree_levels[-1] = [root].

    If the number of nodes at any level is odd, the last node is
    duplicated (standard Merkle padding).
    """
    if not hashes:
        empty = hashlib.sha256(b"").hexdigest()
        return empty, [[empty]]

    levels: list[list[str]] = [list(hashes)]
    current = list(hashes)

    while len(current) > 1:
        if len(current) % 2 == 1:
            current.append(current[-1])  # duplicate last
        next_level = []
        for i in range(0, len(current), 2):
            next_level.append(_hash_pair(current[i], current[i + 1]))
        levels.append(next_level)
        current = next_level

    return current[0], levels


def get_proof(leaf_hash: str, tree_levels: list[list[str]]) -> list[tuple[str, str]]:
    """Get Merkle proof for a leaf.

    Returns list of (sibling_hash, side) tuples where side is "left" or "right",
    indicating the sibling's position relative to the node being proved.
    """
    if not tree_levels or leaf_hash not in tree_levels[0]:
        return []

    proof: list[tuple[str, str]] = []
    idx = tree_levels[0].index(leaf_hash)

    for level in tree_levels[:-1]:
        padded = list(level)
        if len(padded) % 2 == 1:
            padded.append(padded[-1])

        if idx % 2 == 0:
            sibling = padded[idx + 1]
            proof.append((sibling, "right"))
        else:
            sibling = padded[idx - 1]
            proof.append((sibling, "left"))
        idx //= 2

    return proof


def verify_proof(leaf_hash: str, proof: list[tuple[str, str]], root_hash: str) -> bool:
    """Verify a Merkle proof.

    Each proof element is (sibling_hash, side) where side indicates the
    sibling's position.
    """
    current = leaf_hash
    for sibling, side in proof:
        if side == "left":
            current = _hash_pair(sibling, current)
        else:
            current = _hash_pair(current, sibling)
    return current == root_hash
