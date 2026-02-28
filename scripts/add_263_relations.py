#!/usr/bin/env python3
import json

block_263 = '46fc602a418d2c580c792726cc1282561173f23b0bb1509280f42ef353dbdef5'
id_mapping = {
    '261': '58718c5f7d1334cadf279916479367c3060381789a5b9f11c26e2bacce6842b9',
    '258': '6d5abbad445ce37b1e5ae5126f288aeae86759ff5b178084c76760736aba50f2',
    '255': 'd26f99fb0721c10e967558aa2f0fd4417e71b0b77f8c1258122ee6736f4e0992'
}

# 读取最后一行获取order规律
with open('.chanlun/block-topology/relations.jsonl', 'r') as f:
    lines = f.readlines()
    last_line = json.loads(lines[-1])
    base_order = last_line['order']

# 追加三条关系
with open('.chanlun/block-topology/relations.jsonl', 'a') as f:
    for idx, (dep_id, hex_id) in enumerate([('261', id_mapping['261']), ('258', id_mapping['258']), ('255', id_mapping['255'])]):
        rel = {
            "from": block_263,
            "to": hex_id,
            "relation": "depends_on",
            "order": base_order + idx + 1,
            "created_by": block_263,
            "timestamp": "2026-02-28T19:10:00.000000+00:00"
        }
        f.write(json.dumps(rel) + '\n')

print("Added 3 relations to relations.jsonl")
