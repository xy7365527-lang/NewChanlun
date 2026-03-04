"""修复 frontmatter 中新增字段的 ID 引号格式，统一为双引号字符串。"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / ".chanlun" / "genealogy"
FIELDS = ("tensions_with", "triggered", "derived")

count = 0
for d in ["settled", "pending"]:
    dirp = ROOT / d
    if not dirp.exists():
        continue
    for p in sorted(dirp.glob("*.md")):
        text = p.read_text(encoding="utf-8")
        new_text = text
        for field in FIELDS:
            # Match the field line and normalize its values to quoted strings
            def fix_list(m):
                raw = m.group(1)
                # Extract all values (quoted or unquoted)
                vals = re.findall(r"""['"]?(\w+)['"]?""", raw)
                if not vals:
                    return f'{field}: []'
                quoted = ', '.join(f'"{v}"' for v in vals)
                return f'{field}: [{quoted}]'
            new_text = re.sub(rf'^({field}:\s*\[)(.*?)(\])$',
                              lambda m: fix_list(type('M', (), {'group': lambda s, i: m.group(2)})()),
                              new_text, flags=re.MULTILINE)
            # Simpler approach: just match the whole line
            pat = re.compile(rf'^{field}:\s*\[.*\]$', re.MULTILINE)
            match = pat.search(new_text)
            if match:
                line = match.group(0)
                vals = re.findall(r"""['"]?([0-9a-z]+)['"]?""",
                                  line.split(':', 1)[1])
                if vals:
                    quoted = ', '.join(f'"{v}"' for v in vals)
                    new_line = f'{field}: [{quoted}]'
                    new_text = new_text[:match.start()] + new_line + new_text[match.end():]

        if new_text != text:
            p.write_text(new_text, encoding="utf-8")
            count += 1
            print(f"  fixed: {p.name}")

print(f"\n修复了 {count} 个文件")
