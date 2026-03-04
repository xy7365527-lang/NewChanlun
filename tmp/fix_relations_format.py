import json
from datetime import datetime, timezone

relations_file = '.chanlun/block-topology/relations.jsonl'

# 读取现有关系
relations = []
with open(relations_file, 'r') as f:
    for line in f:
        line = line.strip()
        if line:
            try:
                relations.append(json.loads(line))
            except json.JSONDecodeError:
                pass

# 获取281的hash和genesis_block_id（作为created_by）
with open('.chanlun/block-topology/meta.json', 'r') as f:
    meta = json.load(f)

hash_281 = meta['id_mapping']['281']
hash_280 = meta['id_mapping']['280']
hash_267 = meta['id_mapping']['267']
genesis_block_id = meta['genesis_block_id']

timestamp = datetime.now(timezone.utc).isoformat()

# 移除最后两个简化格式的关系（如果存在）
if relations and relations[-1].get('type') == 'depends_on' and 'relation' not in relations[-1]:
    relations.pop()
if relations and relations[-1].get('type') == 'depends_on' and 'relation' not in relations[-1]:
    relations.pop()

# 添加281的完整格式 depends_on 关系
relations.append({
    "from": hash_281,
    "to": hash_280,
    "relation": "depends_on",
    "order": 1,
    "created_by": genesis_block_id,
    "timestamp": timestamp
})

relations.append({
    "from": hash_281,
    "to": hash_267,
    "relation": "depends_on",
    "order": 1,
    "created_by": genesis_block_id,
    "timestamp": timestamp
})

# 写回
with open(relations_file, 'w') as f:
    for rel in relations:
        f.write(json.dumps(rel) + '\n')

print(f"Fixed 281 relations with complete format")
print(f"281 -> 280: depends_on")
print(f"281 -> 267: depends_on")
print(f"Total relations: {len(relations)}")
