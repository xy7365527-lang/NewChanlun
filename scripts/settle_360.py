"""360号谱系结算：创建 block-topology 区块 + depends_on 关系.

360号——离散 Cerf 分岔点检测（B-M 阶段完成）。直接写入 settled/。

运行方式: python scripts/settle_360.py
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
SETTLED_PATH = PROJECT_ROOT / ".chanlun" / "genealogy" / "settled" / "360-cerf-bifurcation.md"


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
            "genealogy_id": "360",
            "title": "离散 Cerf 分岔点检测——阶段 B-M 完成",
            "type": "experiment-completion",
            "depends_on": ["355", "357"],
            "content": {
                "source_file": "settled/360-cerf-bifurcation.md",
                "id": "360",
            },
        },
    )
    print(f"Block created: {block_id}")

    # 3. Load meta.json
    meta = json.loads(META_PATH.read_text(encoding="utf-8"))
    id_mapping = meta["id_mapping"]

    # Resolve dependency block IDs
    dep_355 = id_mapping["355"]
    dep_357 = id_mapping["357"]

    # 4. Add relations (360→355, 360→357)
    now_iso = datetime.now(timezone.utc).isoformat()
    relations = [
        {"from": block_id, "to": dep_355, "relation": "depends_on", "order": 1,
         "created_by": block_id, "timestamp": now_iso},
        {"from": block_id, "to": dep_357, "relation": "depends_on", "order": 2,
         "created_by": block_id, "timestamp": now_iso},
    ]

    with open(RELATIONS_PATH, "a", encoding="utf-8") as f:
        for rel in relations:
            f.write(json.dumps(rel, ensure_ascii=False) + "\n")
    print(f"Added {len(relations)} relations to relations.jsonl")

    # 5. Update meta.json
    meta["id_mapping"]["360"] = block_id
    meta["block_count"] = meta.get("block_count", 0) + 1
    meta["relation_count"] = meta.get("relation_count", 0) + len(relations)
    meta["last_mapped_genealogy"] = 360

    META_PATH.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    print(f"Updated meta.json: block_count={meta['block_count']}, "
          f"relation_count={meta['relation_count']}, "
          f"last_mapped_genealogy=360")

    print("\n360号谱系 block-topology 更新完成")
    return 0


if __name__ == "__main__":
    sys.exit(main())
