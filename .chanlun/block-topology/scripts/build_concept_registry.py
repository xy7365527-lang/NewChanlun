#!/usr/bin/env python3
"""Build concept registry from block-topology blocks.

Scans all block files, extracts concept terms from content_analysis.concepts
and content_analysis.new_concepts, and builds a reverse index:
  concept_term -> {blocks: [hash_list], count, first_seen, last_seen}

Output: concept_registry.json in block-topology root.
Also outputs encounter_log.md with unexpected findings.
"""

import json
import os
import glob
import sys
from collections import defaultdict
from pathlib import Path

TOPO_DIR = Path(__file__).resolve().parent.parent
BLOCKS_DIR = TOPO_DIR / "blocks"
META_PATH = TOPO_DIR / "meta.json"
OUTPUT_PATH = TOPO_DIR / "concept_registry.json"


def load_meta():
    with open(META_PATH, encoding="utf-8") as f:
        return json.load(f)


def build_hash_to_id(meta):
    """Reverse id_mapping: hash -> genealogy_id."""
    return {v: k for k, v in meta.get("id_mapping", {}).items()}


def extract_concepts_from_block(block_data):
    """Extract concept terms from a block's content_analysis."""
    content = block_data.get("content", {})
    if not isinstance(content, dict):
        return [], []

    ca = content.get("content_analysis", {})
    if not isinstance(ca, dict):
        return [], []

    concepts = ca.get("concepts", [])
    new_concepts = ca.get("new_concepts", [])
    return concepts, new_concepts


def get_block_date(block_data):
    """Extract date from block — try content.date, then timestamp."""
    content = block_data.get("content", {})
    if isinstance(content, dict):
        date = content.get("date")
        if date:
            return date
    ts = block_data.get("timestamp", "")
    if ts:
        return ts[:10]  # YYYY-MM-DD
    return "unknown"


def get_block_title(block_data):
    """Extract title from block content."""
    content = block_data.get("content", {})
    if isinstance(content, dict):
        ca = content.get("content_analysis", {})
        if isinstance(ca, dict):
            gid = ca.get("genealogy_id", "")
            if gid:
                return f"谱系{gid}号"
        title = content.get("title")
        if title:
            return title
        gid = content.get("id")
        if gid:
            return f"谱系{gid}号"
    return None


def get_original_block(block_data):
    """For rewrite blocks, get the original block hash."""
    content = block_data.get("content", {})
    if isinstance(content, dict):
        return content.get("original_block")
    return None


def build_registry():
    meta = load_meta()
    hash_to_id = build_hash_to_id(meta)

    # concept_term -> {blocks: set, first_seen: date, last_seen: date, is_new: bool}
    registry = defaultdict(lambda: {
        "blocks": set(),
        "first_seen": "9999-99-99",
        "last_seen": "0000-00-00",
        "is_new_concept": False,
        "definitions": {},
    })

    encounters = []
    block_files = glob.glob(str(BLOCKS_DIR / "*.json"))
    processed = 0
    skipped = 0

    for bf in block_files:
        try:
            with open(bf, encoding="utf-8") as f:
                block = json.load(f)
        except (json.JSONDecodeError, UnicodeDecodeError):
            skipped += 1
            continue

        block_hash = block.get("id", os.path.basename(bf).replace(".json", ""))
        block_type = block.get("type", "unknown")
        block_date = get_block_date(block)

        concepts, new_concepts = extract_concepts_from_block(block)

        for c in concepts:
            if not isinstance(c, dict):
                continue
            term = c.get("term", "").strip()
            if not term:
                continue

            entry = registry[term]
            entry["blocks"].add(block_hash)
            if block_date < entry["first_seen"]:
                entry["first_seen"] = block_date
            if block_date > entry["last_seen"]:
                entry["last_seen"] = block_date
            definition = c.get("definition", "")
            if definition:
                entry["definitions"][block_hash] = definition

        for nc in new_concepts:
            if not isinstance(nc, dict):
                continue
            term = nc.get("term", "").strip()
            if not term:
                continue

            entry = registry[term]
            entry["blocks"].add(block_hash)
            entry["is_new_concept"] = True
            if block_date < entry["first_seen"]:
                entry["first_seen"] = block_date
            if block_date > entry["last_seen"]:
                entry["last_seen"] = block_date
            definition = nc.get("definition", "")
            if definition:
                entry["definitions"][block_hash] = definition

        processed += 1

    # Detect encounters: concepts appearing in surprisingly many or few blocks
    concept_counts = {term: len(data["blocks"]) for term, data in registry.items()}
    if concept_counts:
        avg_count = sum(concept_counts.values()) / len(concept_counts)

        # High-frequency concepts (>3x average)
        for term, count in sorted(concept_counts.items(), key=lambda x: -x[1]):
            if count > avg_count * 3:
                encounters.append({
                    "type": "high_frequency_concept",
                    "concept": term,
                    "block_count": count,
                    "average": round(avg_count, 1),
                    "blocks": sorted(registry[term]["blocks"])[:10],
                    "reason": f"出现在{count}个区块中（平均{avg_count:.1f}），是概念密度中心",
                })

        # Concepts with evolving definitions (appear in many blocks with different definitions)
        for term, data in registry.items():
            if len(data["definitions"]) >= 3:
                defs = list(data["definitions"].values())
                unique_defs = set(d[:100] for d in defs)
                if len(unique_defs) >= 3:
                    encounters.append({
                        "type": "evolving_concept",
                        "concept": term,
                        "definition_count": len(unique_defs),
                        "blocks": sorted(data["blocks"])[:5],
                        "reason": f"概念'{term}'有{len(unique_defs)}种不同定义，可能经历了概念分离或演化",
                    })

        # New concepts that also appear frequently
        for term, data in registry.items():
            if data["is_new_concept"] and len(data["blocks"]) >= 5:
                encounters.append({
                    "type": "proliferating_new_concept",
                    "concept": term,
                    "block_count": len(data["blocks"]),
                    "first_seen": data["first_seen"],
                    "reason": f"新概念'{term}'快速扩散到{len(data['blocks'])}个区块",
                })

    # Serialize
    output = {}
    for term in sorted(registry.keys()):
        data = registry[term]
        output[term] = {
            "blocks": sorted(data["blocks"]),
            "count": len(data["blocks"]),
            "first_seen": data["first_seen"] if data["first_seen"] != "9999-99-99" else "unknown",
            "last_seen": data["last_seen"] if data["last_seen"] != "0000-00-00" else "unknown",
            "is_new_concept": data["is_new_concept"],
        }

    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f"Concept registry built: {len(output)} concepts from {processed} blocks ({skipped} skipped)")
    print(f"Output: {OUTPUT_PATH}")
    print(f"Encounters found: {len(encounters)}")

    return output, encounters, hash_to_id


if __name__ == "__main__":
    output, encounters, hash_to_id = build_registry()

    # Print top 20 concepts by frequency
    top = sorted(output.items(), key=lambda x: -x[1]["count"])[:20]
    print("\nTop 20 concepts by frequency:")
    for term, data in top:
        print(f"  {data['count']:3d} blocks | {term}")
