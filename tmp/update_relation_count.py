import json

# 更新relation_count
with open('.chanlun/block-topology/meta.json', 'r') as f:
    meta = json.load(f)

meta['relation_count'] = 1235  # 从1233增加到1235（添加了2个关系）

with open('.chanlun/block-topology/meta.json', 'w') as f:
    json.dump(meta, f, indent=2)

print(f"Updated relation_count to 1235")
