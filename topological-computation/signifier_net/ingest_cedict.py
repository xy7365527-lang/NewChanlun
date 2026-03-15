"""ingest_cedict.py — CC-CEDICT 中英辞典摄入到 S_net JSONL 文件。

从 CC-CEDICT（Creative Commons 中英辞典，122K+ 条目）生成两种 JSONL 格式：

1. dict_cedict.jsonl（dict_* 通道）：
   - 每个中文简体词条作为 term
   - 英文释义合并为 definition
   - 繁体字作为 synonym（如果与简体不同）
   - domain = "general"

2. bilingual_zh_en_cedict.jsonl（bilingual_* 通道）：
   - 每个 (简体, 英文义项) 对产出一条翻译记录
   - 创建中英 PARADIGMATIC 翻译边

来源：CC-CEDICT (https://cc-cedict.org), CC BY-SA 4.0
认识论等级：L0（辞典是手工编纂的定义）

用法：
    cd topological-computation
    python signifier_net/ingest_cedict.py
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path


def parse_cedict_entries() -> list[dict]:
    """解析 CC-CEDICT 全部条目。返回 dict 列表。"""
    from pycccedict.cccedict import CcCedict
    c = CcCedict()
    return c.entries


def generate_dict_jsonl(entries: list[dict], output_path: Path) -> int:
    """产出 dict_cedict.jsonl（dict_* 通道格式）。

    格式：{"term": str, "domain": str, "definition": str,
           "synonyms": [str, ...], "contrasts": []}

    返回写入条目数。
    """
    count = 0
    seen_terms: set[str] = set()

    with open(output_path, "w", encoding="utf-8") as f:
        for entry in entries:
            simplified = entry.get("simplified", "").strip()
            traditional = entry.get("traditional", "").strip()
            pinyin = entry.get("pinyin", "").strip()
            definitions = entry.get("definitions", [])

            if not simplified or not definitions:
                continue

            # 去重：同一简体词只取第一次出现（CC-CEDICT 可能有重复）
            if simplified in seen_terms:
                continue
            seen_terms.add(simplified)

            # 合并所有英文释义为 definition
            definition = "; ".join(d.strip() for d in definitions if d.strip())
            if not definition:
                continue

            # synonyms：繁体（如果不同于简体）
            synonyms: list[str] = []
            if traditional and traditional != simplified:
                synonyms.append(traditional)

            record = {
                "term": simplified,
                "domain": "general",
                "definition": definition,
                "synonyms": synonyms,
                "contrasts": [],
            }

            f.write(json.dumps(record, ensure_ascii=False) + "\n")
            count += 1

    return count


def generate_bilingual_jsonl(entries: list[dict], output_path: Path) -> int:
    """产出 bilingual_zh_en_cedict.jsonl（bilingual_* 通道格式）。

    每个 (简体, 英文义项) 对产出一条翻译记录。
    格式：{"term_a": str, "lang_a": "zh", "term_b": str, "lang_b": "en",
           "pinyin": str, "traditional": str, "domain": "general",
           "differential": "", "source": "CC-CEDICT"}

    返回写入条目数。
    """
    count = 0

    with open(output_path, "w", encoding="utf-8") as f:
        for entry in entries:
            simplified = entry.get("simplified", "").strip()
            traditional = entry.get("traditional", "").strip()
            pinyin = entry.get("pinyin", "").strip()
            definitions = entry.get("definitions", [])

            if not simplified or not definitions:
                continue

            for defn in definitions:
                defn = defn.strip()
                if not defn:
                    continue

                # 跳过纯元数据义项（如 "variant of X", "see X" 等）
                # 但保留包含实质翻译的条目
                record = {
                    "term_a": simplified,
                    "lang_a": "zh",
                    "term_b": defn,
                    "lang_b": "en",
                    "pinyin": pinyin,
                    "traditional": traditional,
                    "domain": "general",
                    "differential": "",
                    "source": "CC-CEDICT",
                }

                f.write(json.dumps(record, ensure_ascii=False) + "\n")
                count += 1

    return count


def main() -> None:
    script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
    dict_dir = script_dir / "dictionaries"
    dict_dir.mkdir(parents=True, exist_ok=True)

    dict_path = dict_dir / "dict_cedict.jsonl"
    bilingual_path = dict_dir / "bilingual_zh_en_cedict.jsonl"

    print("Parsing CC-CEDICT...", file=sys.stderr)
    t0 = time.time()
    entries = parse_cedict_entries()
    print(f"  Parsed {len(entries)} entries in {time.time() - t0:.1f}s", file=sys.stderr)

    print(f"Generating {dict_path.name}...", file=sys.stderr)
    t1 = time.time()
    dict_count = generate_dict_jsonl(entries, dict_path)
    dict_size = dict_path.stat().st_size
    print(
        f"  Written {dict_count} entries ({dict_size / 1024 / 1024:.1f}MB) "
        f"in {time.time() - t1:.1f}s",
        file=sys.stderr,
    )

    print(f"Generating {bilingual_path.name}...", file=sys.stderr)
    t2 = time.time()
    bilingual_count = generate_bilingual_jsonl(entries, bilingual_path)
    bilingual_size = bilingual_path.stat().st_size
    print(
        f"  Written {bilingual_count} entries ({bilingual_size / 1024 / 1024:.1f}MB) "
        f"in {time.time() - t2:.1f}s",
        file=sys.stderr,
    )

    print(
        f"\nDone. dict_cedict: {dict_count} terms, "
        f"bilingual_zh_en_cedict: {bilingual_count} translation pairs",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
