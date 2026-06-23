#!/usr/bin/env python3
"""一次性映射工具：把谱系 531-548 映射进 block-topology（task #8，谱系拓扑映射）。

映射 = (1) block JSON（content-addressed，id=sha256(文件内容)，复刻 530 区块结构）
      + (2) meta.json id_mapping 增项 + block_count/relation_count 更新
      + (3) relations_531_548.jsonl（depends_on 边，复刻 relations_483_484.jsonl 格式）。

depends_on 关系按 frontmatter depends_on 列表解析，目标 id 零填充到 3 位查 id_mapping。
仅产 depends_on 边（well-attested 类型）；negates/derived 为散文/反向，留在 full_text 不另造边
（保守，不污染 DAG）。批内前向引用（如 533→532）在第一遍全部建 id_mapping 后第二遍解析。
"""
import json
import glob
import hashlib
import os
import re
from datetime import datetime, timezone

BT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))  # block-topology/
SETTLED = os.path.join(BT, "..", "genealogy", "settled")
META = os.path.join(BT, "meta.json")
BLOCKS = os.path.join(BT, "blocks")
REL_OUT = os.path.join(BT, "relations_531_548.jsonl")
IDS = [str(n) for n in range(531, 549)]  # 531..548
TS = datetime.now(timezone.utc).isoformat()


def parse_frontmatter(text):
    """提取 --- 块内 title/type/depends_on（无 yaml 依赖，正则）。"""
    title, typ, deps = None, None, []
    if text.startswith("---"):
        end = text.find("\n---", 3)
        fm = text[3:end] if end != -1 else text[3:1500]
    else:
        fm = text[:1500]
    mt = re.search(r'^title:\s*(.+)$', fm, re.M)
    if mt:
        title = mt.group(1).strip().strip('"').strip("'")
    mty = re.search(r'^type:\s*(.+)$', fm, re.M)
    if mty:
        typ = mty.group(1).split('#')[0].strip().strip('"').strip("'")
    md = re.search(r'^depends_on:\s*\n((?:[ \t]*-\s*.*\n?)+)', fm, re.M)
    if md:
        deps = re.findall(r'-\s*["\']?([0-9]+[a-z]?)["\']?', md.group(1))
    return title, typ, deps


def norm_key(d, mapped):
    """谱系 id → id_mapping 键（零填充 3 位；保留字母后缀）。"""
    m = re.match(r'^(\d+)([a-z]?)$', d)
    if not m:
        return d if d in mapped else None
    cand = m.group(1).zfill(3) + m.group(2)
    return cand if cand in mapped else None


def main():
    meta = json.load(open(META, encoding="utf-8"))
    idmap = meta["id_mapping"]
    new_blocks = {}   # id -> (hash, block_dict, raw_deps)
    # 第一遍：建 block + 收 hash 入 id_mapping（供批内前向引用解析）。
    for gid in IDS:
        files = glob.glob(os.path.join(SETTLED, f"{gid}-*.md"))
        if not files:
            print(f"[skip] {gid}: 无文件")
            continue
        path = files[0]
        content = open(path, encoding="utf-8").read()
        h = hashlib.sha256(content.encode("utf-8")).hexdigest()
        title, typ, deps = parse_frontmatter(content)
        block = {
            "id": h,
            "timestamp": TS,
            "content": {
                "full_text": content,
                "source_file": f"settled/{os.path.basename(path)}",
                "id": gid,
            },
            "genealogy_id": gid,
            "title": title,
            "type": typ,
            "depends_on": deps,
        }
        new_blocks[gid] = (h, block, deps)
        idmap[gid] = h  # 键已是 3 位
    # 第二遍：写 block 文件 + 解析 depends_on 边。
    mapped = set(idmap.keys())
    relations = []
    unresolved = []
    for gid, (h, block, deps) in new_blocks.items():
        with open(os.path.join(BLOCKS, f"{h}.json"), "w", encoding="utf-8") as f:
            json.dump(block, f, ensure_ascii=False, indent=2)
        for d in deps:
            tk = norm_key(d, mapped)
            if tk is None:
                unresolved.append((gid, d))
                continue
            relations.append({
                "from": h,
                "to": idmap[tk],
                "relation": "depends_on",
                "order": 1,
                "created_by": h,
                "timestamp": TS,
            })
    # 写 relations 增量文件。
    with open(REL_OUT, "w", encoding="utf-8") as f:
        for r in relations:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    # 更新计数。
    meta["block_count"] = meta.get("block_count", 0) + len(new_blocks)
    meta["relation_count"] = meta.get("relation_count", 0) + len(relations)
    with open(META, "w", encoding="utf-8") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
    print(f"映射 {len(new_blocks)} 谱系；新增 {len(relations)} depends_on 边；"
          f"未解析 {len(unresolved)} {unresolved}")
    print(f"block_count={meta['block_count']} relation_count={meta['relation_count']} "
          f"id_mapping={len(idmap)}")


if __name__ == "__main__":
    main()
