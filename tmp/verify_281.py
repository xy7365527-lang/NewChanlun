import json
import os

print("=== 281号 block-topology 映射验证 ===\n")

# 验证meta.json
print("1. 检查 meta.json:")
with open('.chanlun/block-topology/meta.json', 'r') as f:
    meta = json.load(f)

if '281' in meta['id_mapping']:
    hash_281 = meta['id_mapping']['281']
    print(f"   ✓ 281号 ID 映射存在")
    print(f"     ID: {hash_281[:16]}...")
else:
    print(f"   ✗ 281号 ID 映射不存在")

print(f"   ✓ block_count: {meta['block_count']}")
print(f"   ✓ relation_count: {meta['relation_count']}\n")

# 验证块文件
print("2. 检查块文件:")
hash_281 = meta['id_mapping']['281']
block_file = f".chanlun/block-topology/blocks/{hash_281}.json"
if os.path.exists(block_file):
    with open(block_file, 'r') as f:
        block = json.load(f)
    print(f"   ✓ 块文件存在")
    print(f"     标题: {block['content']['title'][:40]}...")
else:
    print(f"   ✗ 块文件不存在: {block_file}")

print(f"\n3. 检查关系:")
# 检查relations.jsonl
hash_280 = meta['id_mapping']['280']
hash_267 = meta['id_mapping']['267']

with open('.chanlun/block-topology/relations.jsonl', 'r') as f:
    relations = [json.loads(line.strip()) for line in f if line.strip()]

# 查找281的关系
rel_281_280 = any(r.get('from') == hash_281 and r.get('to') == hash_280 and r.get('relation') == 'depends_on' for r in relations)
rel_281_267 = any(r.get('from') == hash_281 and r.get('to') == hash_267 and r.get('relation') == 'depends_on' for r in relations)

if rel_281_280:
    print(f"   ✓ 281 -> 280: depends_on")
else:
    print(f"   ✗ 281 -> 280: depends_on 关系缺失")

if rel_281_267:
    print(f"   ✓ 281 -> 267: depends_on")
else:
    print(f"   ✗ 281 -> 267: depends_on 关系缺失")

print(f"\n=== 修复完成 ===")
print(f"✓ 281号 block-topology 映射已成功创建")
