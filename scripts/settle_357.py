"""357号谱系结算：pending→settled + block-topology 更新.

运行方式: python scripts/settle_357.py
"""
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "scripts"))

from block_ops import create_block

GENEALOGY_DIR = PROJECT_ROOT / ".chanlun" / "genealogy"
TOPO_DIR = PROJECT_ROOT / ".chanlun" / "block-topology"
BLOCKS_DIR = TOPO_DIR / "blocks"
META_PATH = TOPO_DIR / "meta.json"
RELATIONS_PATH = TOPO_DIR / "relations.jsonl"
PENDING_PATH = GENEALOGY_DIR / "pending" / "357-ceremony-scan-proposer.md"
SETTLED_PATH = GENEALOGY_DIR / "settled" / "357-ceremony-scan-proposer.md"


def main():
    # 1. Verify settled file exists
    if not SETTLED_PATH.is_file():
        print(f"ERROR: settled file not found: {SETTLED_PATH}")
        return 1

    # 2. Create content-addressed block
    block_id = create_block(
        SETTLED_PATH,
        BLOCKS_DIR,
        metadata={
            "genealogy_id": "357",
            "title": "ceremony_scan 消费器到提议器增强——从被动读取到主动发现",
            "type": "architectural-insight",
            "depends_on": ["353", "355", "356"],
            "content": {
                "source_file": "settled/357-ceremony-scan-proposer.md",
                "id": "357",
            },
        },
    )
    print(f"Block created: {block_id}")

    # 3. Load meta.json
    meta = json.loads(META_PATH.read_text(encoding="utf-8"))
    id_mapping = meta["id_mapping"]

    # Resolve dependency block IDs
    dep_353 = id_mapping["353"]
    dep_355 = id_mapping["355"]
    dep_356 = id_mapping["356"]

    # 4. Add relations (357→353, 357→355, 357→356)
    now_iso = datetime.now(timezone.utc).isoformat()
    relations = [
        {"from": block_id, "to": dep_353, "relation": "depends_on", "order": 1,
         "created_by": block_id, "timestamp": now_iso},
        {"from": block_id, "to": dep_355, "relation": "depends_on", "order": 2,
         "created_by": block_id, "timestamp": now_iso},
        {"from": block_id, "to": dep_356, "relation": "depends_on", "order": 3,
         "created_by": block_id, "timestamp": now_iso},
    ]

    with open(RELATIONS_PATH, "a", encoding="utf-8") as f:
        for rel in relations:
            f.write(json.dumps(rel, ensure_ascii=False) + "\n")
    print(f"Added 3 relations to relations.jsonl")

    # 5. Update meta.json
    meta["id_mapping"]["357"] = block_id
    meta["block_count"] = meta.get("block_count", 0) + 1
    meta["relation_count"] = meta.get("relation_count", 0) + 3
    meta["last_mapped_genealogy"] = 357

    META_PATH.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    print(f"Updated meta.json: block_count={meta['block_count']}, "
          f"relation_count={meta['relation_count']}, "
          f"last_mapped_genealogy=357")

    # 6. Delete pending file
    if PENDING_PATH.is_file():
        os.remove(PENDING_PATH)
        print(f"Deleted pending file: {PENDING_PATH}")
    else:
        print(f"Pending file already removed: {PENDING_PATH}")

    print("\n357号谱系结算完成: pending→settled + block-topology updated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
