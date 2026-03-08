"""
Fetch and process online dictionary data for S_net.

1. CC-CEDICT: Chinese-English open dictionary
2. OpenThesaurus: German synonym thesaurus

Produces:
- dictionaries/bilingual_zh_en_cedict.jsonl
- dictionaries/thesaurus_de_openthesaurus.jsonl
"""

import gzip
import io
import json
import glob
import os
import re
import sys
import urllib.request
import zipfile
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
DICT_DIR = BASE_DIR / "dictionaries"


def build_zh_whitelist() -> set[str]:
    """Build Chinese term whitelist from all dict_*.jsonl and morpheme files."""
    terms = set()
    for f in glob.glob(str(DICT_DIR / "dict_*.jsonl")):
        for line in open(f, encoding="utf-8"):
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            if isinstance(entry, dict) and "term" in entry:
                terms.add(entry["term"])

    morpheme_zh = DICT_DIR / "morpheme_zh_philosophy.jsonl"
    if morpheme_zh.exists():
        for line in open(morpheme_zh, encoding="utf-8"):
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            if "signifier_id" in entry:
                terms.add(entry["signifier_id"])

    return terms


def build_de_whitelist() -> set[str]:
    """Build German term whitelist from morpheme_de_philosophy.jsonl."""
    terms = set()
    morpheme_de = DICT_DIR / "morpheme_de_philosophy.jsonl"
    if morpheme_de.exists():
        for line in open(morpheme_de, encoding="utf-8"):
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            if "signifier_id" in entry:
                terms.add(entry["signifier_id"])
    return terms


def download_file(url: str, desc: str) -> bytes:
    """Download a file from URL, return raw bytes."""
    print(f"Downloading {desc} from {url} ...")
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (S_net fetcher)"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = resp.read()
    print(f"  Downloaded {len(data)} bytes")
    return data


# ── CC-CEDICT ──────────────────────────────────────────────────────────

def parse_cedict_line(line: str):
    """Parse a CC-CEDICT line.

    Format: 繁體 简体 [pin1 yin1] /definition 1/definition 2/
    Returns (traditional, simplified, pinyin, [definitions]) or None.
    """
    if line.startswith("#") or not line.strip():
        return None
    m = re.match(r"^(\S+)\s+(\S+)\s+\[([^\]]+)\]\s+/(.+)/$", line)
    if not m:
        return None
    traditional = m.group(1)
    simplified = m.group(2)
    pinyin = m.group(3)
    definitions = [d.strip() for d in m.group(4).split("/") if d.strip()]
    return traditional, simplified, pinyin, definitions


def process_cedict(raw_gz: bytes, whitelist: set[str]) -> list[dict]:
    """Process CC-CEDICT gzipped data, match against whitelist."""
    text = gzip.decompress(raw_gz).decode("utf-8")
    lines = text.splitlines()

    # Build index: simplified -> [(traditional, pinyin, definitions)]
    cedict_index: dict[str, list[tuple]] = {}
    for line in lines:
        parsed = parse_cedict_line(line)
        if parsed is None:
            continue
        traditional, simplified, pinyin, definitions = parsed
        cedict_index.setdefault(simplified, []).append((traditional, pinyin, definitions))
        if traditional != simplified:
            cedict_index.setdefault(traditional, []).append((traditional, pinyin, definitions))

    print(f"  CC-CEDICT: {len(cedict_index)} unique entries parsed")

    # Match against whitelist
    results = []
    matched = set()
    for term in sorted(whitelist):
        if term in cedict_index:
            for traditional, pinyin, definitions in cedict_index[term]:
                eng_defs = "; ".join(definitions)
                results.append({
                    "term_a": term,
                    "lang_a": "zh",
                    "term_b": eng_defs,
                    "lang_b": "en",
                    "pinyin": pinyin,
                    "traditional": traditional,
                    "domain": "general",
                    "differential": "",
                    "source": "CC-CEDICT"
                })
                matched.add(term)

    # Also try matching single-character components and multi-char substrings
    # that appear as dictionary headwords
    unmatched = whitelist - matched
    for term in sorted(unmatched):
        # Only try Chinese terms (skip English/German terms)
        if not any('\u4e00' <= c <= '\u9fff' for c in term):
            continue
        # Try exact substring match for compound terms
        if len(term) >= 2 and term in cedict_index:
            for traditional, pinyin, definitions in cedict_index[term]:
                eng_defs = "; ".join(definitions)
                results.append({
                    "term_a": term,
                    "lang_a": "zh",
                    "term_b": eng_defs,
                    "lang_b": "en",
                    "pinyin": pinyin,
                    "traditional": traditional,
                    "domain": "general",
                    "differential": "",
                    "source": "CC-CEDICT"
                })
                matched.add(term)

    print(f"  Matched {len(matched)} whitelist terms, produced {len(results)} entries")
    return results


# ── OpenThesaurus ──────────────────────────────────────────────────────

def process_openthesaurus(raw_zip: bytes, whitelist: set[str]) -> list[dict]:
    """Process OpenThesaurus zip data, match against German whitelist."""
    zf = zipfile.ZipFile(io.BytesIO(raw_zip))
    # Find the text file inside
    txt_names = [n for n in zf.namelist() if n.endswith(".txt")]
    if not txt_names:
        raise ValueError(f"No .txt file found in zip. Contents: {zf.namelist()}")

    txt_name = txt_names[0]
    print(f"  Extracting {txt_name}")
    text = zf.read(txt_name).decode("utf-8")
    lines = text.splitlines()

    # Build index: term -> [synonym groups it belongs to]
    term_to_groups: dict[str, list[list[str]]] = {}
    total_groups = 0
    for line in lines:
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        # Format: word1;word2;word3 (semicolon-separated synonym group)
        synonyms = [s.strip() for s in line.split(";") if s.strip()]
        if len(synonyms) < 2:
            continue
        total_groups += 1
        for syn in synonyms:
            term_to_groups.setdefault(syn, []).append(synonyms)

    print(f"  OpenThesaurus: {total_groups} synonym groups, {len(term_to_groups)} unique terms")

    # Match against whitelist
    results = []
    matched = set()
    for term in sorted(whitelist):
        if term in term_to_groups:
            # Merge all synonym groups for this term
            all_synonyms = set()
            for group in term_to_groups[term]:
                all_synonyms.update(group)
            all_synonyms.discard(term)  # Remove the term itself
            if all_synonyms:
                results.append({
                    "term": term,
                    "lang": "de",
                    "synonyms": sorted(all_synonyms),
                    "source": "OpenThesaurus"
                })
                matched.add(term)

    print(f"  Matched {len(matched)} whitelist terms")
    return results


def main():
    errors = []

    # ── 1. CC-CEDICT ──
    print("\n=== CC-CEDICT (Chinese-English) ===")
    zh_whitelist = build_zh_whitelist()
    # Filter to only Chinese terms for CEDICT matching
    zh_terms = {t for t in zh_whitelist if any('\u4e00' <= c <= '\u9fff' for c in t)}
    print(f"Chinese whitelist: {len(zh_terms)} terms with Chinese characters")

    try:
        cedict_url = "https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz"
        raw_gz = download_file(cedict_url, "CC-CEDICT")
        cedict_results = process_cedict(raw_gz, zh_terms)

        out_path = DICT_DIR / "bilingual_zh_en_cedict.jsonl"
        with open(out_path, "w", encoding="utf-8") as f:
            for entry in cedict_results:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        print(f"  Written to {out_path} ({len(cedict_results)} entries)")
    except Exception as e:
        err_msg = f"CC-CEDICT download/processing failed: {e}"
        print(f"  ERROR: {err_msg}")
        errors.append(err_msg)

    # ── 2. OpenThesaurus ──
    print("\n=== OpenThesaurus (German Synonyms) ===")
    de_whitelist = build_de_whitelist()
    print(f"German whitelist: {len(de_whitelist)} terms")

    try:
        ot_url = "https://www.openthesaurus.de/export/OpenThesaurus-Textversion.zip"
        raw_zip = download_file(ot_url, "OpenThesaurus")
        ot_results = process_openthesaurus(raw_zip, de_whitelist)

        out_path = DICT_DIR / "thesaurus_de_openthesaurus.jsonl"
        with open(out_path, "w", encoding="utf-8") as f:
            for entry in ot_results:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        print(f"  Written to {out_path} ({len(ot_results)} entries)")
    except Exception as e:
        err_msg = f"OpenThesaurus download/processing failed: {e}"
        print(f"  ERROR: {err_msg}")
        errors.append(err_msg)

    # ── Summary ──
    print("\n=== Summary ===")
    if errors:
        print(f"ERRORS ({len(errors)}):")
        for e in errors:
            print(f"  - {e}")
        return 1
    else:
        print("All downloads and processing completed successfully.")
        return 0


if __name__ == "__main__":
    sys.exit(main())
