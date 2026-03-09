"""convert_cedict.py — CC-CEDICT → bilingual_cedict_zh_en.jsonl 转换脚本.

CC-CEDICT 格式（每行）：
  Traditional Simplified [pin1 yin1] /English definition 1/definition 2/.../

输出 JSONL 格式（与 ingest_bilingual_dict 的格式B兼容）：
  {"source_term": "汉字", "source_lang": "zh", "target_term": "Chinese character",
   "target_lang": "en", "differential": "", "source": "cc-cedict",
   "domain": "general", "pos": ""}

用法：
  python convert_cedict.py <cedict_file> [output_file]

  <cedict_file>:  CC-CEDICT 原始文本文件路径（UTF-8）
  [output_file]:  输出 JSONL 路径（默认: 同目录下的 bilingual_cedict_zh_en.jsonl）

CC-CEDICT 下载：
  https://www.mdbg.net/chinese/dictionary?page=cedict
  解压后得到 cedict_ts.u8 文本文件

认识论等级：L0（确定性格式转换）
谱系引用：v207-swarm/translation-ingest
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

# CC-CEDICT 行格式正则：
# Traditional Simplified [pinyin] /definition 1/definition 2/
_CEDICT_LINE_RE = re.compile(
    r"^(\S+)\s+(\S+)\s+\[([^\]]+)\]\s+/(.+)/$"
)

# 词性标注正则（部分 CC-CEDICT 条目在定义中标注词性）
_POS_RE = re.compile(r"^\((noun|verb|adj|adv|prep|conj|interj|classifier|idiom)\)\s*")

# 过滤：跳过纯拼音/数字/单字符的英文定义
_SKIP_PATTERNS = re.compile(
    r"^(variant of|old variant of|see |abbr\. for |CL:)"
)


def parse_cedict_line(line: str) -> dict | None:
    """解析单行 CC-CEDICT 条目。

    返回 dict 或 None（注释行/格式不匹配）。
    """
    line = line.strip()
    if not line or line.startswith("#"):
        return None

    m = _CEDICT_LINE_RE.match(line)
    if not m:
        return None

    traditional = m.group(1)
    simplified = m.group(2)
    pinyin = m.group(3)
    definitions_raw = m.group(4)

    # 拆分定义（/分隔）
    definitions = [d.strip() for d in definitions_raw.split("/") if d.strip()]
    if not definitions:
        return None

    # 提取词性（从第一个定义）
    pos = ""
    pos_match = _POS_RE.match(definitions[0])
    if pos_match:
        pos = pos_match.group(1)

    # 过滤引用型条目（variant of, see, abbr. for）
    real_defs = [d for d in definitions if not _SKIP_PATTERNS.match(d)]
    if not real_defs:
        return None

    # 取第一个实质定义作为 target_term
    primary_def = real_defs[0]
    # 去掉词性前缀
    pm = _POS_RE.match(primary_def)
    if pm:
        primary_def = primary_def[pm.end():]

    # 如果定义过长（>60字符），截断到第一个分号或逗号
    if len(primary_def) > 60:
        for sep in (";", ",", " ("):
            idx = primary_def.find(sep)
            if 0 < idx < 60:
                primary_def = primary_def[:idx]
                break
        if len(primary_def) > 60:
            primary_def = primary_def[:60]

    primary_def = primary_def.strip()
    if not primary_def:
        return None

    return {
        "simplified": simplified,
        "traditional": traditional,
        "pinyin": pinyin,
        "primary_def": primary_def,
        "all_defs": real_defs,
        "pos": pos,
    }


def convert_cedict(
    input_path: str | Path,
    output_path: str | Path | None = None,
    domain_filter: str | None = None,
) -> Path:
    """将 CC-CEDICT 文件转换为 bilingual JSONL 格式。

    参数：
      input_path:    CC-CEDICT 文本文件路径
      output_path:   输出 JSONL 路径（None 时自动生成）
      domain_filter: 域过滤（None 时全部转换，设为 "philosophy" 等可过滤）

    返回：
      输出文件路径
    """
    input_path = Path(input_path)
    if not input_path.exists():
        raise FileNotFoundError(f"CC-CEDICT 文件不存在: {input_path}")

    if output_path is None:
        output_path = input_path.parent / "bilingual_cedict_zh_en.jsonl"
    output_path = Path(output_path)

    count = 0
    skipped = 0

    with open(input_path, encoding="utf-8") as fin, \
         open(output_path, "w", encoding="utf-8") as fout:

        for line in fin:
            parsed = parse_cedict_line(line)
            if parsed is None:
                continue

            # 使用简体中文作为 source_term
            source_term = parsed["simplified"]
            target_term = parsed["primary_def"]

            # 跳过太短的条目（单字母英文定义通常无意义）
            if len(target_term) < 2:
                skipped += 1
                continue

            # differential：如果有多个定义，记录为语义展开
            differential = ""
            if len(parsed["all_defs"]) > 1:
                alt_defs = [d for d in parsed["all_defs"][1:4] if d != target_term]
                if alt_defs:
                    differential = f"其他义项: {'; '.join(alt_defs)}"

            entry = {
                "source_term": source_term,
                "source_lang": "zh",
                "target_term": target_term,
                "target_lang": "en",
                "differential": differential,
                "source": "cc-cedict",
                "domain": "general",
                "pos": parsed["pos"],
            }

            fout.write(json.dumps(entry, ensure_ascii=False) + "\n")
            count += 1

    print(f"转换完成: {count} 条写入 {output_path}")
    if skipped:
        print(f"跳过: {skipped} 条（定义过短）")

    return output_path


def main() -> None:
    if len(sys.argv) < 2:
        print(
            "用法: python convert_cedict.py <cedict_file> [output_file]\n"
            "\n"
            "CC-CEDICT 下载: https://www.mdbg.net/chinese/dictionary?page=cedict\n"
            "解压后得到 cedict_ts.u8 文本文件"
        )
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2] if len(sys.argv) > 2 else None

    convert_cedict(input_path, output_path)


if __name__ == "__main__":
    main()
