import json

# 读取meta.json
with open('.chanlun/block-topology/meta.json', 'r') as f:
    meta = json.load(f)

# 更新id_mapping
meta['id_mapping']['281'] = 'da1401b412c153e32655bb1895ae185fbdcaf353cfb81c4372db62c192d9de72'

# 更新block_count
meta['block_count'] = 307

# 写回
with open('.chanlun/block-topology/meta.json', 'w') as f:
    json.dump(meta, f, indent=2)

print("meta.json updated successfully")
print(f"Block count: {meta['block_count']}")
print(f"281 mapping: {meta['id_mapping']['281']}")
