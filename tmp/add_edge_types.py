"""扫描谱系文件内容，提取 tensions_with / triggered / derived 边类型建议（v3 安全版）。

v2 regex 灾难性回溯导致挂起。v3 用行级扫描替代全文 regex。
"""

import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent.parent / ".chanlun" / "genealogy"
DIRS = ["settled", "pending"]

# Valid IDs we'll collect first
ALL_IDS: set[str] = set()


def parse_file(path: Path) -> dict | None:
    text = path.read_text(encoding="utf-8")
    m = re.match(r"^---\n(.+?)\n---\n?(.*)", text, re.DOTALL)
    if not m:
        return None
    fm = yaml.safe_load(m.group(1))
    if not fm or "id" not in fm:
        return None
    fm["_body"] = m.group(2)
    fm["_path"] = path
    return fm


def nid(raw: str) -> str:
    m = re.match(r'^(\d+)([a-z]?)$', raw)
    if not m:
        return raw
    return f"{int(m.group(1)):03d}{m.group(2)}"


def extract_ids_from_line(line: str) -> list[str]:
    """Extract valid genealogy IDs from a line, filtering date false positives."""
    # Remove date strings first
    clean = re.sub(r'20\d{2}-\d{2}-\d{2}', '', line)
    # Remove time strings
    clean = re.sub(r'\d{2}:\d{2}:\d{2}', '', clean)
    ids = []
    for m in re.finditer(r'(?<!\d)(\d{2,3}[a-z]?)(?:号|[-—/、,，]|\s|$|[）)\]】])', clean):
        tid = nid(m.group(1))
        if tid in ALL_IDS:
            ids.append(tid)
    return ids


def scan_file(fm: dict) -> list[tuple[str, str, str, str]]:
    """Return list of (source, target, edge_type, evidence)."""
    sid = fm["id"]
    body = fm["_body"]
    ftype = fm.get("type", "")
    results = []

    existing = set()
    for key in ("depends_on", "related", "negates", "negated_by",
                "tensions_with", "triggered", "derived"):
        for v in fm.get(key) or []:
            existing.add((key, nid(str(v))))

    lines = body.split('\n')

    for i, line in enumerate(lines):
        ctx_window = '\n'.join(lines[max(0, i-1):i+2])
        ids_in_line = [t for t in extract_ids_from_line(line) if t != sid]
        if not ids_in_line:
            continue

        # --- tensions_with ---
        if re.search(r'张力|表面矛盾|↔', line):
            for tid in ids_in_line:
                if ("tensions_with", tid) not in existing:
                    results.append((sid, tid, "tensions_with", line.strip()[:150]))

        # --- triggered ---
        if re.search(r'概念分离|分离为|扬弃[：:]|对.*的扬弃|触发了.*发现|处理.*触发|子发现', line):
            for tid in ids_in_line:
                if ("triggered", tid) not in existing:
                    results.append((sid, tid, "triggered", line.strip()[:150]))

        # "扬弃" in frontmatter header area (first 20 lines of body)
        if i < 20 and re.search(r'\*\*扬弃\*\*', line):
            for tid in ids_in_line:
                if ("triggered", tid) not in existing:
                    results.append((sid, tid, "triggered", f"扬弃: {sid} 扬弃 {tid}"))

        # --- derived ---
        if re.search(r'交叉推论|约束链|的推论|逻辑必然推论|结算依据.*推论', line):
            for tid in ids_in_line:
                if ("derived", tid) not in existing:
                    results.append((sid, tid, "derived", line.strip()[:150]))

    # Type-based derived: 定理 type → derived from depends_on
    if "定理" in ftype and ("推论" in ftype or "逻辑必然" in ftype):
        for dep in fm.get("depends_on") or []:
            tid = nid(str(dep))
            if tid in ALL_IDS and ("derived", tid) not in existing:
                results.append((sid, tid, "derived", f"[type={ftype}] derived from {tid}"))

    # Sub-entry pattern: 019 → 019a/019b/019c/019d
    if re.match(r'^\d{3}$', sid):
        for tid in ALL_IDS:
            if tid.startswith(sid) and tid != sid and len(tid) == 4:
                if ("triggered", tid) not in existing:
                    # Verify sub-entry is mentioned in body
                    if tid in body or tid.lstrip('0') in body:
                        results.append((sid, tid, "triggered", f"子条目: {sid} → {tid}"))

    return results


def main():
    files = []
    for d in DIRS:
        dirp = ROOT / d
        if not dirp.exists():
            continue
        for p in sorted(dirp.glob("*.md")):
            fm = parse_file(p)
            if fm:
                files.append(fm)
                ALL_IDS.add(fm["id"])

    all_suggestions = []
    for fm in files:
        all_suggestions.extend(scan_file(fm))

    # Deduplicate
    seen = set()
    unique = []
    for s in all_suggestions:
        key = (s[0], s[1], s[2])
        if key not in seen:
            seen.add(key)
            unique.append(s)

    by_type = {"tensions_with": [], "triggered": [], "derived": []}
    for src, tgt, et, ev in unique:
        by_type[et].append((src, tgt, ev))

    print("=" * 70)
    print("谱系边类型扫描结果 v3")
    print("=" * 70)

    total = 0
    for et in ("tensions_with", "triggered", "derived"):
        edges = sorted(by_type[et])
        print(f"\n## {et} ({len(edges)} 条)")
        print("-" * 50)
        for src, tgt, ev in edges:
            print(f"  {src} -> {tgt}")
            print(f"    {ev[:120]}")
            print()
        total += len(edges)

    print(f"\n总计: {total} 条建议边")

    out = {"tensions_with": [], "triggered": [], "derived": []}
    for src, tgt, et, ev in unique:
        out[et].append({"from": src, "to": tgt, "evidence": ev[:200]})

    outpath = Path(__file__).resolve().parent / "edge_suggestions_v3.yaml"
    outpath.write_text(
        yaml.dump(out, allow_unicode=True, default_flow_style=False, sort_keys=False),
        encoding="utf-8",
    )
    print(f"\n建议已写入: {outpath}")


if __name__ == "__main__":
    main()
