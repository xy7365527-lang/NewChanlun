#!/usr/bin/env python3
"""Build Morse density landscape from block-topology.

Computes "height" (degree = in-degree + out-degree) for each block based on
relations.jsonl. Identifies peaks (high-density concept centers) and valleys
(sparse/isolated regions).

Output: morse_landscape.json in block-topology root.
"""

import json
import os
from collections import defaultdict, Counter
from pathlib import Path

TOPO_DIR = Path(__file__).resolve().parent.parent
RELATIONS_PATH = TOPO_DIR / "relations.jsonl"
META_PATH = TOPO_DIR / "meta.json"
REGISTRY_PATH = TOPO_DIR / "concept_registry.json"
OUTPUT_PATH = TOPO_DIR / "morse_landscape.json"


def load_meta():
    with open(META_PATH, encoding="utf-8") as f:
        return json.load(f)


def load_registry():
    if REGISTRY_PATH.exists():
        with open(REGISTRY_PATH, encoding="utf-8") as f:
            return json.load(f)
    return {}


def load_relations():
    relations = []
    with open(RELATIONS_PATH, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    relations.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    return relations


def build_landscape():
    meta = load_meta()
    hash_to_id = {v: k for k, v in meta.get("id_mapping", {}).items()}
    id_to_hash = meta.get("id_mapping", {})
    registry = load_registry()
    relations = load_relations()

    # Compute degree for each block
    in_degree = Counter()
    out_degree = Counter()
    relation_types_per_block = defaultdict(lambda: Counter())

    for rel in relations:
        src = rel.get("from", "")
        dst = rel.get("to", "")
        rtype = rel.get("relation", "unknown")
        if src:
            out_degree[src] += 1
            relation_types_per_block[src][rtype] += 1
        if dst:
            in_degree[dst] += 1
            relation_types_per_block[dst][rtype] += 1

    # Collect all block hashes
    all_blocks = set(in_degree.keys()) | set(out_degree.keys())
    # Also include blocks from meta that might have zero degree
    for h in id_to_hash.values():
        all_blocks.add(h)

    # Height = in_degree + out_degree
    heights = {}
    for block_hash in all_blocks:
        ind = in_degree.get(block_hash, 0)
        outd = out_degree.get(block_hash, 0)
        heights[block_hash] = {
            "in_degree": ind,
            "out_degree": outd,
            "height": ind + outd,
            "genealogy_id": hash_to_id.get(block_hash, ""),
        }

    # Sort by height
    sorted_blocks = sorted(heights.items(), key=lambda x: -x[1]["height"])

    # Identify peaks: top 10% or height > mean + 2*std
    height_values = [h["height"] for h in heights.values()]
    if height_values:
        mean_h = sum(height_values) / len(height_values)
        variance = sum((h - mean_h) ** 2 for h in height_values) / len(height_values)
        std_h = variance ** 0.5
        peak_threshold = max(mean_h + 1.5 * std_h, 10)
    else:
        mean_h = 0
        std_h = 0
        peak_threshold = 10

    peaks = []
    for block_hash, data in sorted_blocks:
        if data["height"] >= peak_threshold:
            gid = data["genealogy_id"]
            # Count concepts associated with this block
            concept_count = sum(
                1 for c_data in registry.values()
                if block_hash in c_data.get("blocks", [])
            )
            peaks.append({
                "block_hash": block_hash,
                "genealogy_id": gid,
                "height": data["height"],
                "in_degree": data["in_degree"],
                "out_degree": data["out_degree"],
                "concept_count": concept_count,
                "dominant_relation_types": dict(
                    relation_types_per_block[block_hash].most_common(3)
                ),
            })

    # Identify valleys: blocks with height 0 or 1
    valleys = []
    for block_hash, data in sorted_blocks:
        if data["height"] <= 1:
            gid = data["genealogy_id"]
            valleys.append({
                "block_hash": block_hash,
                "genealogy_id": gid,
                "height": data["height"],
                "in_degree": data["in_degree"],
                "out_degree": data["out_degree"],
            })

    # Height distribution
    dist = Counter()
    for h in height_values:
        bucket = (h // 5) * 5  # buckets of 5
        dist[bucket] += 1
    height_distribution = {str(k): v for k, v in sorted(dist.items())}

    # Saddle points: blocks that connect two otherwise separate clusters
    # Approximate: blocks with high betweenness-like property
    # (high in+out but with diverse relation types)
    saddles = []
    for block_hash, data in sorted_blocks:
        rtypes = relation_types_per_block.get(block_hash, Counter())
        if len(rtypes) >= 4 and data["height"] >= mean_h:
            saddles.append({
                "block_hash": block_hash,
                "genealogy_id": data["genealogy_id"],
                "height": data["height"],
                "relation_diversity": len(rtypes),
                "relation_types": dict(rtypes.most_common(5)),
            })

    output = {
        "statistics": {
            "total_blocks": len(all_blocks),
            "total_relations": len(relations),
            "mean_height": round(mean_h, 2),
            "std_height": round(std_h, 2),
            "peak_threshold": round(peak_threshold, 2),
            "max_height": max(height_values) if height_values else 0,
            "min_height": min(height_values) if height_values else 0,
        },
        "peaks": peaks[:50],
        "valleys": valleys[:50],
        "saddles": saddles[:30],
        "height_distribution": height_distribution,
        "block_heights": {
            bh: {
                "genealogy_id": data["genealogy_id"],
                "height": data["height"],
                "in": data["in_degree"],
                "out": data["out_degree"],
            }
            for bh, data in sorted_blocks
        },
    }

    with open(OUTPUT_PATH, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    print(f"Morse landscape built: {len(all_blocks)} blocks, {len(relations)} relations")
    print(f"  Peaks (height >= {peak_threshold:.0f}): {len(peaks)}")
    print(f"  Valleys (height <= 1): {len(valleys)}")
    print(f"  Saddles (relation diversity >= 4): {len(saddles)}")
    print(f"  Mean height: {mean_h:.2f}, Std: {std_h:.2f}")
    print(f"Output: {OUTPUT_PATH}")

    return output


if __name__ == "__main__":
    landscape = build_landscape()
    print("\nTop 10 peaks:")
    for p in landscape["peaks"][:10]:
        gid = p["genealogy_id"] or "?"
        print(f"  {gid:>5s} | height={p['height']:3d} (in={p['in_degree']}, out={p['out_degree']}) concepts={p['concept_count']}")
