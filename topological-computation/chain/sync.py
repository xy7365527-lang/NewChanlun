"""Sync from IPFS: download blocks, rebuild state.

Graceful degradation: if IPFS unavailable, only local operations work.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

# Ensure parent is importable
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from engine import Graph
from daemon import graph_from_dict
from chain.ipfs_client import IPFSClient


def sync_from_ipfs(ipfs_client: IPFSClient, manifest_cid: str, local_dir: str) -> list[str]:
    """Download all blocks from IPFS manifest, save locally.

    The manifest is a JSON object with:
      {"blocks": [{"cid": "...", "filename": "..."}, ...]}

    Returns list of downloaded filenames.
    """
    blocks_dir = Path(local_dir) / "blocks"
    blocks_dir.mkdir(parents=True, exist_ok=True)

    manifest_data = ipfs_client.download(manifest_cid)
    manifest = json.loads(manifest_data.decode("utf-8"))

    downloaded: list[str] = []
    for entry in manifest.get("blocks", []):
        cid = entry["cid"]
        filename = entry.get("filename", f"{cid}.json")
        target = blocks_dir / filename

        if target.exists():
            continue

        block_data = ipfs_client.download(cid)
        target.write_bytes(block_data)
        downloaded.append(filename)

    return downloaded


def rebuild_state(blocks_dir: str) -> Graph:
    """Rebuild K_full from local blocks directory.

    Reads all .json block files, extracts vertices and edges,
    and reconstructs the Graph. Blocks are ordered by step number.
    """
    blocks_path = Path(blocks_dir)
    if not blocks_path.exists():
        return Graph()

    # Collect all blocks and sort by step
    blocks: list[dict] = []
    for path in sorted(blocks_path.glob("*.json")):
        try:
            content = json.loads(path.read_text(encoding="utf-8"))
            blocks.append(content)
        except (json.JSONDecodeError, OSError):
            continue

    # Sort by step number (if present)
    blocks.sort(key=lambda b: b.get("step", 0))

    # Accumulate vertices and edges
    all_vertices: list[dict] = []
    all_edges: list[dict] = []
    seen_vertex_ids: set[str] = set()
    seen_edge_keys: set[tuple[str, str, str]] = set()

    for block in blocks:
        for v in block.get("vertices", []):
            vid = v["id"]
            if vid not in seen_vertex_ids:
                all_vertices.append(v)
                seen_vertex_ids.add(vid)
        for e in block.get("edges", []):
            key = (e["source"], e["target"], e["edge_type"])
            if key not in seen_edge_keys:
                all_edges.append(e)
                seen_edge_keys.add(key)

    return graph_from_dict({"vertices": all_vertices, "edges": all_edges})
