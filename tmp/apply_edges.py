"""将审查通过的 tensions_with / triggered / derived 边写入谱系 frontmatter。

使用 regex 插入，不重排已有 frontmatter。
"""

import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent / ".chanlun" / "genealogy"

# ============================================================
# 审查通过的边（全部有文本证据）
# ============================================================

TENSIONS = [
    ("007", "008"),
    ("007", "009"),
    ("008", "009"),
    ("012", "013"),
    ("012", "019d"),
    ("062", "065"),
    ("064", "065"),
]

TRIGGERED = [
    ("005", "005a"),
    ("005", "005b"),
    ("019", "019a"),
    ("019", "019b"),
    ("019", "019c"),
    ("019", "019d"),
    ("019d", "020"),
    ("030", "030a"),
]

DERIVED = [
    ("010", "007"),
    ("010", "008"),
    ("010", "009"),
    ("011", "008"),
    ("020a", "020"),
    ("024", "005a"),
    ("024", "005b"),
    ("036", "016"),
    ("036", "032"),
    ("036", "033"),
    ("036", "034"),
    ("063", "020"),
    ("063", "062"),
]


def build_id_to_path() -> dict[str, Path]:
    mapping = {}
    for d in ["settled", "pending"]:
        dirp = ROOT / d
        if not dirp.exists():
            continue
        for p in dirp.glob("*.md"):
            text = p.read_text(encoding="utf-8")
            m = re.match(r'^---\n(.+?)\n---', text, re.DOTALL)
            if m:
                fm = yaml.safe_load(m.group(1))
                if fm and "id" in fm:
                    mapping[str(fm["id"])] = p
    return mapping


def add_field_to_frontmatter(path: Path, field: str, values: list[str]) -> bool:
    """Add values to a frontmatter list field. Preserves existing formatting."""
    text = path.read_text(encoding="utf-8")
    m = re.match(r'^(---\n)(.+?)(\n---)', text, re.DOTALL)
    if not m:
        return False

    fm_text = m.group(2)
    fm = yaml.safe_load(fm_text)
    existing = [str(v) for v in (fm.get(field) or [])]
    new_vals = sorted(set(existing + values))
    if new_vals == sorted(existing):
        return False

    # Format the new field value
    new_field_str = f'{field}: {yaml.dump(new_vals, default_flow_style=True, allow_unicode=True).strip()}'

    # Check if field already exists in frontmatter
    field_pat = re.compile(rf'^{re.escape(field)}:\s*.*$', re.MULTILINE)
    if field_pat.search(fm_text):
        new_fm_text = field_pat.sub(new_field_str, fm_text)
    else:
        # Insert before the last field (negates or negated_by) or at end
        # Try to insert after negates line
        insert_pat = re.compile(r'(^negates:\s*.*$)', re.MULTILINE)
        if insert_pat.search(fm_text):
            new_fm_text = insert_pat.sub(r'\1\n' + new_field_str, fm_text)
        else:
            new_fm_text = fm_text + '\n' + new_field_str

    new_text = m.group(1) + new_fm_text + m.group(3) + text[m.end():]
    path.write_text(new_text, encoding="utf-8")
    return True


def main():
    id_to_path = build_id_to_path()
    modified = set()

    print("=== tensions_with (双向) ===")
    for a, b in TENSIONS:
        pa, pb = id_to_path.get(a), id_to_path.get(b)
        if pa and add_field_to_frontmatter(pa, "tensions_with", [b]):
            modified.add(a)
            print(f"  {a} += tensions_with: {b}")
        if pb and add_field_to_frontmatter(pb, "tensions_with", [a]):
            modified.add(b)
            print(f"  {b} += tensions_with: {a}")

    print("\n=== triggered (单向) ===")
    for src, tgt in TRIGGERED:
        p = id_to_path.get(src)
        if p and add_field_to_frontmatter(p, "triggered", [tgt]):
            modified.add(src)
            print(f"  {src} += triggered: {tgt}")

    print("\n=== derived (单向) ===")
    for src, tgt in DERIVED:
        p = id_to_path.get(src)
        if p and add_field_to_frontmatter(p, "derived", [tgt]):
            modified.add(src)
            print(f"  {src} += derived: {tgt}")

    print(f"\n修改了 {len(modified)} 个文件: {sorted(modified)}")
    print("\n下一步: python tmp/generate_dag.py")


if __name__ == "__main__":
    main()
