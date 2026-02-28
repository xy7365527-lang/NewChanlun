#!/usr/bin/env python3
import json

# 更新meta.json
with open('.chanlun/block-topology/meta.json', 'r') as f:
    meta = json.load(f)

# 增加block_count
meta['block_count'] += 1

# 添加263到id_mapping
meta['id_mapping']['263'] = '46fc602a418d2c580c792726cc1282561173f23b0bb1509280f42ef353dbdef5'

# 更新relation_count（+3个新关系）
meta['relation_count'] += 3

with open('.chanlun/block-topology/meta.json', 'w') as f:
    json.dump(meta, f, indent=2)

print(f"Updated meta.json: block_count={meta['block_count']}, relation_count={meta['relation_count']}")
