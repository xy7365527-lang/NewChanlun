"""364号谱系结算：创建 block-topology 区块 + depends_on 关系.

364号——元观察 v160-swarm 两轮推进。

运行方式: python scripts/settle_364.py
"""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "scripts"))

from block_ops import create_block

TOPO_DIR = PROJECT_ROOT / ".chanlun" / "block-topology"
BLOCKS_DIR = TOPO_DIR / "blocks"
META_PATH = TOPO_DIR / "meta.json"
RELATIONS_PATH = TOPO_DIR / "relations.jsonl"
SETTLED_PATH = PROJECT_ROOT / ".chanlun" / "genealogy" / "settled" / "364-meta-observation-v160-swarm.md"


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
            "genealogy_id": "364",
            "title": "元观察——v160-swarm 两轮推进（B-M研究线L2关闭 + 审计方法论修正 + 张力收缩）",
            "type": "meta-rule",
            "depends_on": ["361", "362", "363", "359"],
            "content": {
                "source_file": "settled/364-meta-observation-v160-swarm.md",
                "id": "364",
            },
        },
    )
    print(f"Block created: {block_id}")

    # 3. Load meta.json
    meta = json.loads(META_PATH.read_text(encoding="utf-8"))
    id_mapping = meta["id_mapping"]

    # Resolve dependency block IDs
    dep_361 = id_mapping["361"]
    dep_362 = id_mapping["362"]
    dep_363 = id_mapping["363"]
    dep_359 = id_mapping["359"]

    # 4. Add relations (364->361, 364->362, 364->363, 364->359)
    now_iso = datetime.now(timezone.utc).isoformat()
    relations = [
        {"from": block_id, "to": dep_361, "relation": "depends_on", "order": 1,
         "created_by": block_id, "timestamp": now_iso},
        {"from": block_id, "to": dep_362, "relation": "depends_on", "order": 2,
         "created_by": block_id, "timestamp": now_iso},
        {"from": block_id, "to": dep_363, "relation": "depends_on", "order": 3,
         "created_by": block_id, "timestamp": now_iso},
        {"from": block_id, "to": dep_359, "relation": "depends_on", "order": 4,
         "created_by": block_id, "timestamp": now_iso},
    ]

    with open(RELATIONS_PATH, "a", encoding="utf-8") as f:
        for rel in relations:
            f.write(json.dumps(rel, ensure_ascii=False) + "\n")
    print(f"Added {len(relations)} relations to relations.jsonl")

    # 5. Update meta.json
    meta["id_mapping"]["364"] = block_id
    meta["block_count"] = meta.get("block_count", 0) + 1
    meta["relation_count"] = meta.get("relation_count", 0) + len(relations)
    meta["last_mapped_genealogy"] = 364

    META_PATH.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    print(f"Updated meta.json: block_count={meta['block_count']}, "
          f"relation_count={meta['relation_count']}, "
          f"last_mapped_genealogy=364")

    print("\n364号谱系 block-topology 更新完成")
    return 0


if __name__ == "__main__":
    sys.exit(main())
