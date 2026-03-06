"""Shared event layer: append-only blocks + appendable relations.

Block topology's Event layer and Relation layer.
Blocks are content-addressed (SHA256 hash filename), naturally deduplicated.
Relations are JSONL append-write.
"""

from __future__ import annotations

import json
import hashlib
import time
from pathlib import Path


class SharedLayer:
    """Content-addressed block store + JSONL relation log."""

    def __init__(self, shared_dir: str | Path):
        self.blocks_dir = Path(shared_dir) / "blocks"
        self.relations_path = Path(shared_dir) / "relations.jsonl"
        self.blocks_dir.mkdir(parents=True, exist_ok=True)
        self.relations_path.parent.mkdir(parents=True, exist_ok=True)

    def write_block(self, content: dict) -> str:
        """Write content-addressed block. Returns block hash."""
        data = json.dumps(content, sort_keys=True, ensure_ascii=False)
        block_hash = hashlib.sha256(data.encode()).hexdigest()
        path = self.blocks_dir / f"{block_hash}.json"
        if not path.exists():
            path.write_text(data, encoding="utf-8")
        return block_hash

    def read_block(self, block_hash: str) -> dict | None:
        """Read a single block by hash."""
        path = self.blocks_dir / f"{block_hash}.json"
        if not path.exists():
            return None
        return json.loads(path.read_text(encoding="utf-8"))

    def write_relation(
        self,
        from_hash: str,
        to_hash: str,
        relation: str,
        instance_id: str,
    ) -> None:
        """Append a relation record."""
        record = {
            "from": from_hash,
            "to": to_hash,
            "relation": relation,
            "instance": instance_id,
            "timestamp": time.time(),
        }
        with open(self.relations_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(record, ensure_ascii=False) + "\n")

    def read_new_blocks(self, known_hashes: set[str]) -> list[dict]:
        """Read blocks not in known_hashes. Returns list of block dicts with 'hash' field."""
        new_blocks: list[dict] = []
        for path in self.blocks_dir.glob("*.json"):
            block_hash = path.stem
            if block_hash not in known_hashes:
                content = json.loads(path.read_text(encoding="utf-8"))
                content["hash"] = block_hash
                new_blocks.append(content)
        return new_blocks

    def all_block_hashes(self) -> set[str]:
        """Return the set of all block hashes currently on disk."""
        return {p.stem for p in self.blocks_dir.glob("*.json")}

    def read_relations(self) -> list[dict]:
        """Read all relation records."""
        if not self.relations_path.exists():
            return []
        records: list[dict] = []
        with open(self.relations_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    records.append(json.loads(line))
        return records
