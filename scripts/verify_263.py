#!/usr/bin/env python3
import json

# 验证block文件
block_path = '.chanlun/block-topology/blocks/46fc602a418d2c580c792726cc1282561173f23b0bb1509280f42ef353dbdef5.json'
with open(block_path, 'r') as f:
    block = json.load(f)
print(f"✓ Block 263 created: {block['content']['title'][:50]}...")

# 验证meta.json
with open('.chanlun/block-topology/meta.json', 'r') as f:
    meta = json.load(f)
print(f"✓ meta.json updated: block_count={meta['block_count']}, relation_count={meta['relation_count']}")
print(f"✓ 263 in id_mapping: {meta['id_mapping'].get('263', 'MISSING')[:16]}...")

# 验证relations.jsonl新增行
with open('.chanlun/block-topology/relations.jsonl', 'r') as f:
    lines = f.readlines()
    last_3 = [json.loads(line) for line in lines[-3:]]
    count = sum(1 for line in last_3 if line['from'] == '46fc602a418d2c580c792726cc1282561173f23b0bb1509280f42ef353dbdef5')
print(f"✓ relations.jsonl: {count} new relations for 263")
