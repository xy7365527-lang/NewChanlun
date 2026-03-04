import json

# 查看最后几行
with open('.chanlun/block-topology/relations.jsonl', 'r') as f:
    lines = f.readlines()

print(f"Total lines: {len(lines)}")
print("\nLast 5 lines:")
for line in lines[-5:]:
    line = line.strip()
    if line:
        obj = json.loads(line)
        print(f"  Keys: {list(obj.keys())}")
        print(f"  {obj}")
