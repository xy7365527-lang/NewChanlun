import json

relations_file = '.chanlun/block-topology/relations.jsonl'

# 读取现有关系
relations = []
with open(relations_file, 'r') as f:
    for line in f:
        line = line.strip()
        if line:  # 跳过空行
            try:
                relations.append(json.loads(line))
            except json.JSONDecodeError as e:
                print(f"Error parsing line: {line[:100]}")
                raise

# 获取281的hash
with open('.chanlun/block-topology/meta.json', 'r') as f:
    meta = json.load(f)

hash_281 = meta['id_mapping']['281']
hash_280 = meta['id_mapping']['280']
hash_267 = meta['id_mapping']['267']

# 添加281的depends_on关系
relations.append({
    "from": hash_281,
    "to": hash_280,
    "type": "depends_on"
})

relations.append({
    "from": hash_281,
    "to": hash_267,
    "type": "depends_on"
})

# 写回
with open(relations_file, 'w') as f:
    for rel in relations:
        f.write(json.dumps(rel) + '\n')

print(f"Added 2 relations for 281")
print(f"281 -> 280: depends_on")
print(f"281 -> 267: depends_on")
print(f"Total relations: {len(relations)}")
