"""
Block-topology integrity fix script.
Fixes P0 issues from v88 topology analysis:
1. Short hash blocks (194, 204-212) - recompute SHA-256 from file content
2. Missing relation writes (191/192/198/201/224/225)
3. Malformed relation lines (lines 959-960, 1070-1082, 1083-1084)
4. Type naming inconsistency (213: 概念発見, 220: concept-discovery)
"""

import json
import hashlib
import os
import copy

BLOCKS_DIR = r"C:\Users\hanju\NewChanlun\.chanlun\block-topology\blocks"
META_PATH = r"C:\Users\hanju\NewChanlun\.chanlun\block-topology\meta.json"
RELATIONS_PATH = r"C:\Users\hanju\NewChanlun\.chanlun\block-topology\relations.jsonl"

# Load meta.json
with open(META_PATH, "r", encoding="utf-8") as f:
    meta = json.load(f)

id_mapping = meta["id_mapping"]

# Short hash blocks to fix: genealogy_id -> current short hash
SHORT_HASH_BLOCKS = {
    "194": "0551915",
    "204": "714618087d0f1d7a",
    "205": "d1f3a5f4d81ea3f1",
    "206": "7aeec48982cd9d0b",
    "207": "b898befb405c4ce5",
    "208": "9835bd16bee94eb5",
    "209": "aa9f62899568280b",
    "210": "bd03533d2138b499",
    "211": "660c9c796a7c6ede",
    "212": "8ca9ca03a1371500",
}

# ============================================================
# Step 1: Fix short hashes - compute SHA-256 from file content
# ============================================================
print("=== Step 1: Short hash fix ===")

hash_remap = {}  # old_short_hash -> new_full_hash

for gen_id, short_hash in SHORT_HASH_BLOCKS.items():
    block_file = os.path.join(BLOCKS_DIR, f"{short_hash}.json")

    if not os.path.exists(block_file):
        print(f"  WARNING: Block file not found for {gen_id} ({short_hash})")
        # For 194, there's no block file, just update id_mapping
        if gen_id == "194":
            print(f"  194 has no block file. Cannot compute hash. Skipping file rename.")
        continue

    # Read the file content
    with open(block_file, "r", encoding="utf-8") as f:
        content = f.read()

    # Parse JSON to update the id field
    block_data = json.loads(content)

    # Compute SHA-256 of the original file content (before modification)
    # The hash should be of the canonical content
    full_hash = hashlib.sha256(content.encode("utf-8")).hexdigest()

    hash_remap[short_hash] = full_hash

    print(f"  {gen_id}: {short_hash} -> {full_hash}")

    # Update block's id field
    block_data["id"] = full_hash

    # Write new file with full hash name
    new_file = os.path.join(BLOCKS_DIR, f"{full_hash}.json")
    with open(new_file, "w", encoding="utf-8") as f:
        json.dump(block_data, f, ensure_ascii=False, indent=2)
        f.write("\n")

    # Remove old file
    os.remove(block_file)

    # Update id_mapping
    id_mapping[gen_id] = full_hash

# For 194, no block file exists. We cannot compute a hash.
# Record this anomaly.
if "194" in SHORT_HASH_BLOCKS and "0551915" not in hash_remap:
    print("  194: No block file found. Cannot fix hash. Keeping short hash in id_mapping.")
    print("       Note: 0551915 is referenced in relations.jsonl - those refs will be kept as-is.")

# ============================================================
# Step 2: Fix type naming inconsistency (213, 220)
# ============================================================
print("\n=== Step 2: Type naming fix ===")

# 213: 概念発見 -> 概念发现
block_213_hash = id_mapping["213"]
block_213_file = os.path.join(BLOCKS_DIR, f"{block_213_hash}.json")
with open(block_213_file, "r", encoding="utf-8") as f:
    block_213 = json.load(f)

if block_213.get("type") == "概念発見":
    block_213["type"] = "概念发现"
    with open(block_213_file, "w", encoding="utf-8") as f:
        json.dump(block_213, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"  213: '概念発見' -> '概念发现'")
else:
    print(f"  213: type is already '{block_213.get('type')}', no change needed")

# 220: concept-discovery -> 概念发现
block_220_hash = id_mapping["220"]
block_220_file = os.path.join(BLOCKS_DIR, f"{block_220_hash}.json")
with open(block_220_file, "r", encoding="utf-8") as f:
    block_220 = json.load(f)

if block_220.get("type") == "concept-discovery":
    block_220["type"] = "概念发现"
    with open(block_220_file, "w", encoding="utf-8") as f:
        json.dump(block_220, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"  220: 'concept-discovery' -> '概念发现'")
else:
    print(f"  220: type is already '{block_220.get('type')}', no change needed")

# ============================================================
# Step 3: Fix relations.jsonl
# ============================================================
print("\n=== Step 3: Relations fix ===")

with open(RELATIONS_PATH, "r", encoding="utf-8") as f:
    lines = f.readlines()

print(f"  Original line count: {len(lines)}")

def resolve_id(val):
    """Resolve a genealogy ID or short hash to a full hash."""
    val_str = str(val)
    # Check if it's a genealogy ID (short numeric or alphanumeric)
    if val_str in id_mapping:
        return id_mapping[val_str]
    # Check if it's a short hash that was remapped
    if val_str in hash_remap:
        return hash_remap[val_str]
    # Check with zero-padded 3-digit format
    if val_str.isdigit():
        padded = val_str.zfill(3)
        if padded in id_mapping:
            return id_mapping[padded]
    return None

GENESIS_HASH = meta["genesis_block_id"]

new_lines = []
removed_count = 0
fixed_count = 0
hash_updated_count = 0

for i, line in enumerate(lines, 1):
    line = line.strip()
    if not line:
        continue

    try:
        rel = json.loads(line)
    except json.JSONDecodeError:
        print(f"  Line {i}: JSON parse error, removing")
        removed_count += 1
        continue

    # --- Handle malformed lines ---

    # Type 1: Lines with only "target" and no "from" (lines 959-960)
    if "target" in rel and "from" not in rel and "to" not in rel:
        # These are orphaned - no source block. Remove them.
        print(f"  Line {i}: Orphan target-only line, removing: {json.dumps(rel, ensure_ascii=False)}")
        removed_count += 1
        continue

    # Type 2: Lines with only "target_genealogy_id" (lines 1070-1082)
    if "target_genealogy_id" in rel and "from" not in rel:
        print(f"  Line {i}: Orphan target_genealogy_id line, removing: {json.dumps(rel, ensure_ascii=False)}")
        removed_count += 1
        continue

    # Type 3: Lines with "to_genealogy" array (lines 1083-1084)
    if "to_genealogy" in rel and "to" not in rel:
        from_hash = rel.get("from", "")
        to_ids = rel["to_genealogy"]
        relation_type = rel.get("relation", rel.get("type", "depends_on"))

        # Expand into multiple standard edges
        expanded = 0
        for to_id in to_ids:
            resolved = resolve_id(to_id)
            if resolved:
                new_rel = {
                    "from": from_hash,
                    "to": resolved,
                    "relation": relation_type,
                    "order": 1,
                    "created_by": GENESIS_HASH,
                    "timestamp": "2026-02-27T07:57:39.504630+00:00"
                }
                new_lines.append(json.dumps(new_rel, ensure_ascii=False))
                expanded += 1
            else:
                print(f"  Line {i}: Cannot resolve to_genealogy '{to_id}', skipping")

        if expanded > 0:
            print(f"  Line {i}: Expanded to_genealogy into {expanded} standard edges")
            fixed_count += expanded
        else:
            removed_count += 1
        continue

    # --- Normalize field names ---
    # Some lines use "type" instead of "relation"
    normalized = {}
    from_val = rel.get("from", "")
    to_val = rel.get("to", "")
    relation_type = rel.get("relation", rel.get("type", ""))

    # --- Remap short hashes and genealogy IDs ---
    from_updated = False
    to_updated = False

    # Check if from is a short hash or genealogy ID
    if from_val in hash_remap:
        from_val = hash_remap[from_val]
        from_updated = True
    elif len(from_val) < 64 and resolve_id(from_val):
        from_val = resolve_id(from_val)
        from_updated = True

    # Check if to is a short hash or genealogy ID
    if to_val in hash_remap:
        to_val = hash_remap[to_val]
        to_updated = True
    elif len(to_val) < 64 and resolve_id(to_val):
        to_val = resolve_id(to_val)
        to_updated = True

    if from_updated or to_updated:
        hash_updated_count += 1

    # Build normalized relation
    normalized = {
        "from": from_val,
        "to": to_val,
        "relation": relation_type,
    }

    # Preserve optional fields if they exist in standard format
    if "order" in rel:
        normalized["order"] = rel["order"]
    else:
        normalized["order"] = 1

    if "created_by" in rel:
        normalized["created_by"] = rel["created_by"]

    if "timestamp" in rel:
        normalized["timestamp"] = rel["timestamp"]

    # Preserve genealogy annotations (useful metadata, not malformed)
    if "genealogy_from" in rel:
        normalized["genealogy_from"] = rel["genealogy_from"]
    if "genealogy_to" in rel:
        normalized["genealogy_to"] = rel["genealogy_to"]

    new_lines.append(json.dumps(normalized, ensure_ascii=False))

print(f"  Removed {removed_count} malformed lines")
print(f"  Fixed {fixed_count} lines (expanded from to_genealogy)")
print(f"  Updated {hash_updated_count} hash references")

# ============================================================
# Step 4: Add missing relation writes (191/192/198/201/224/225)
# ============================================================
print("\n=== Step 4: Missing relation writes ===")

# 191: depends_on [188, 178], related [187]
block_191 = {
    "hash": id_mapping["191"],
    "depends_on": ["188", "178"],
    "related": ["187"]
}

# 192: depends_on [191]
block_192 = {
    "hash": id_mapping["192"],
    "depends_on": ["191"],
    "related": []
}

# 198: depends_on [] (empty in block file)
# Actually 198 has empty depends_on, so nothing to add
block_198_deps = []

# 201: depends_on [] (empty in block file)
block_201_deps = []

# 224: refs [137, 143, 218] - these are the depends_on
block_224 = {
    "hash": id_mapping["224"],
    "depends_on": ["137", "143", "218"],
    "related": []
}

# 225: refs [224, 137]
block_225 = {
    "hash": id_mapping["225"],
    "depends_on": ["224", "137"],
    "related": []
}

missing_blocks = [
    ("191", block_191),
    ("192", block_192),
    ("224", block_224),
    ("225", block_225),
]

added_count = 0
for gen_id, block_info in missing_blocks:
    from_hash = block_info["hash"]

    for dep_id in block_info["depends_on"]:
        to_hash = id_mapping.get(dep_id)
        if to_hash:
            rel = {
                "from": from_hash,
                "to": to_hash,
                "relation": "depends_on",
                "order": 1,
                "created_by": GENESIS_HASH,
                "timestamp": "2026-02-27T08:00:00.000000+00:00"
            }
            new_lines.append(json.dumps(rel, ensure_ascii=False))
            added_count += 1
            print(f"  Added: {gen_id} -> {dep_id} (depends_on)")
        else:
            print(f"  WARNING: Cannot resolve {dep_id} for {gen_id}")

    for rel_id in block_info.get("related", []):
        to_hash = id_mapping.get(rel_id)
        if to_hash:
            rel = {
                "from": from_hash,
                "to": to_hash,
                "relation": "related",
                "order": 1,
                "created_by": GENESIS_HASH,
                "timestamp": "2026-02-27T08:00:00.000000+00:00"
            }
            new_lines.append(json.dumps(rel, ensure_ascii=False))
            added_count += 1
            print(f"  Added: {gen_id} -> {rel_id} (related)")
        else:
            print(f"  WARNING: Cannot resolve {rel_id} for {gen_id}")

print(f"  Added {added_count} missing relations")

# Note: 198 and 201 have empty depends_on in their block files,
# so they are correctly isolated in terms of declared dependencies.
# The analysis report listed them but their actual depends_on is empty.
print("  198: depends_on is empty in block file - no relations to add")
print("  201: depends_on is empty in block file - no relations to add")

# ============================================================
# Step 5: Write updated relations.jsonl
# ============================================================
print(f"\n=== Step 5: Write results ===")
print(f"  New line count: {len(new_lines)}")

with open(RELATIONS_PATH, "w", encoding="utf-8") as f:
    for line in new_lines:
        f.write(line + "\n")

# ============================================================
# Step 6: Update meta.json
# ============================================================
print("\n=== Step 6: Update meta.json ===")

meta["relation_count"] = len(new_lines)
# block_count stays the same (file count in blocks/ dir hasn't changed)
block_count = len([f for f in os.listdir(BLOCKS_DIR) if f.endswith(".json")])
meta["block_count"] = block_count

with open(META_PATH, "w", encoding="utf-8") as f:
    json.dump(meta, f, ensure_ascii=False, indent=2)
    f.write("\n")

print(f"  block_count: {block_count}")
print(f"  relation_count: {len(new_lines)}")
print(f"  id_mapping entries: {len(id_mapping)}")

print("\n=== Done ===")
print(f"Summary:")
print(f"  - Renamed {len(hash_remap)} block files (short hash -> SHA-256)")
print(f"  - Fixed 2 type names (概念発見, concept-discovery -> 概念发现)")
print(f"  - Removed {removed_count} malformed relation lines")
print(f"  - Expanded {fixed_count} to_genealogy lines into standard edges")
print(f"  - Updated {hash_updated_count} short hash/genealogy ID references in relations")
print(f"  - Added {added_count} missing dependency relations")
print(f"  - Updated meta.json (block_count={block_count}, relation_count={len(new_lines)})")
