"""corpus_extract.py — 从 tmp/downloads/ 提取文本到 S_net corpora 目录。

支持格式：PDF (PyMuPDF), EPUB (ebooklib), MOBI (mobi), RTF (striprtf), TXT, FB2 (xml)
不支持/跳过：AZW3（无开源解析器）

输出：topological-computation/signifier_net/corpora/{author}/{filename}.md
格式与 chanlun/ 下的 .md 语料一致——纯文本，UTF-8。

认识论等级：L0（确定性格式转换，不涉及经验假设）
"""

from __future__ import annotations

import json
import os
import re
import sys
import time
import traceback
import xml.etree.ElementTree as ET
from pathlib import Path

# ---------------------------------------------------------------------------
# 路径常量
# ---------------------------------------------------------------------------

PROJECT_ROOT = Path(__file__).resolve().parent.parent
DOWNLOADS_DIR = PROJECT_ROOT / "tmp" / "downloads"
CORPORA_DIR = PROJECT_ROOT / "topological-computation" / "signifier_net" / "corpora"
REPORT_PATH = PROJECT_ROOT / "tmp" / "corpus_extract_report.json"


# ---------------------------------------------------------------------------
# 作者→域名映射（从文件名前缀提取）
# ---------------------------------------------------------------------------

def author_to_domain(filename: str) -> str:
    """从文件名提取作者名，映射为域目录名。

    文件名格式：Author_Title_Details.ext
    提取第一个下划线之前的部分作为作者名（小写化）。

    特殊规则：
    - Cambridge_Companion → companions
    - Blackwell_Companion → companions
    - Collins/PONS/Langenscheidt → dictionaries
    - David_Harvey → marx (companion to Marx's Capital)
    """
    stem = Path(filename).stem

    # 特殊前缀映射
    special_prefixes = {
        "Cambridge_Companion": "companions",
        "Cambridge_Dictionary": "companions",
        "Blackwell_Companion": "companions",
        "Collins_": "dictionaries",
        "PONS_": "dictionaries",
        "Langenscheidt_": "dictionaries",
        "David_Harvey": "marx",
        "Mao_Zedong": "mao",
        "Mao_On": "mao",
        "Mao_Quotations": "mao",
        "Terry_Pinkard": "hegel",
        "Hyppolite": "hegel",
        "Kojeve": "hegel",
        "Michael_Inwood_Hegel": "hegel",
        "Michael_Inwood_Heidegger": "heidegger",
        "Bennington_Derrida": "derrida",
        "Bruce_Fink": "lacan",
        "Dylan_Evans": "lacan",
        "Laplanche": "lacan",
        "Bartlett_Clemens": "badiou",
        "Deborah_Cook": "adorno",
        "Adrian_Parr": "deleuze",
        "Hans-Johann_Glock": "wittgenstein",
        "Howard_Caygill": "kant",
        "Keith_Ansell-Pearson": "nietzsche",
        "Dreyfus_Being": "heidegger",
        "Tony_Myers": "zizek",
        "Fred_Botting": "bataille",
        "David_Frisby": "simmel",
        "Rex_Butler": "zizek",
        "Bottomore_Dictionary": "marx",
        "Oxford_Dictionary": "dictionaries",
        "Oxford_Collocations": "dictionaries",
        "Oxford_Hachette": "dictionaries",
        "Oxford_Duden": "dictionaries",
        "Oxford_English_Chinese": "dictionaries",
        "Oxford_Handbook": "companions",
        "Duden_": "dictionaries",
        "Wahrig_": "dictionaries",
        "Kluge_": "dictionaries",
        "Le_Petit": "dictionaries",
        "Le_Robert": "dictionaries",
        "Longman_": "dictionaries",
        "Merriam-Webster": "dictionaries",
        "Roget_": "dictionaries",
        "Fowler_": "dictionaries",
        "Routledge_": "companions",
        "Deleuze_Guattari": "deleuze",
        "说文解字": "zh_classical",
        "中国成语": "zh_classical",
        "同义词": "zh_classical",
        "康熙字典": "zh_classical",
        "新华字典": "zh_classical",
        "汉语大词典": "zh_classical",
        "辞海": "zh_classical",
        "辞源": "zh_classical",
    }

    for prefix, domain in special_prefixes.items():
        if stem.startswith(prefix):
            return domain

    # 默认：第一个下划线之前的部分
    parts = stem.split("_")
    if len(parts) >= 2:
        author = parts[0].lower()
        # 标准化
        author_map = {
            "adorno": "adorno",
            "badiou": "badiou",
            "bataille": "bataille",
            "benjamin": "benjamin",
            "deleuze": "deleuze",
            "derrida": "derrida",
            "foucault": "foucault",
            "freud": "freud",
            "hegel": "hegel",
            "heidegger": "heidegger",
            "holderlin": "holderlin",
            "kant": "kant",
            "kierkegaard": "kierkegaard",
            "lacan": "lacan",
            "lenin": "lenin",
            "marx": "marx",
            "merleau-ponty": "merleau_ponty",
            "nietzsche": "nietzsche",
            "schelling": "schelling",
            "simmel": "simmel",
            "spinoza": "spinoza",
            "wittgenstein": "wittgenstein",
            "zizek": "zizek",
            "diestel": "math",
            "rotman": "math",
            "bredon": "math",
            "halmos": "math",
            "milnor": "math",
            "ghrist": "math",
            "mac": "math",
            "kozlov": "math",
            "munkres": "math",
            "hatcher": "math",
        }
        return author_map.get(author, author)

    return "misc"


# ---------------------------------------------------------------------------
# 文本清洗
# ---------------------------------------------------------------------------

def clean_text(text: str) -> str:
    """清洗提取的文本：去除多余空行，修复编码问题。"""
    if not text:
        return ""

    # 替换 \x00 等控制字符
    text = re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f]', '', text)

    # 合并多余空行（保留最多2个连续换行）
    text = re.sub(r'\n{4,}', '\n\n\n', text)

    # 去除行尾空白
    lines = [line.rstrip() for line in text.split('\n')]
    text = '\n'.join(lines)

    return text.strip()


# ---------------------------------------------------------------------------
# 格式提取器
# ---------------------------------------------------------------------------

def extract_pdf(filepath: Path) -> str:
    """使用 PyMuPDF 提取 PDF 全文。"""
    import fitz

    doc = fitz.open(str(filepath))
    pages = []
    for page in doc:
        pages.append(page.get_text())
    doc.close()
    return '\n\n'.join(pages)


def extract_epub(filepath: Path) -> str:
    """使用 ebooklib 提取 EPUB 全文。"""
    from ebooklib import epub
    from bs4 import BeautifulSoup

    book = epub.read_epub(str(filepath))
    texts = []
    for item in book.get_items_of_type(9):  # ITEM_DOCUMENT
        soup = BeautifulSoup(item.get_content(), 'lxml')
        text = soup.get_text(separator='\n')
        if text.strip():
            texts.append(text)
    return '\n\n'.join(texts)


def extract_mobi(filepath: Path) -> str:
    """使用 mobi 库提取 MOBI 全文。"""
    import mobi
    import tempfile

    tmpdir = tempfile.mkdtemp()
    try:
        tempdir, extracted = mobi.extract(str(filepath))
        # extracted 是解压后的 html 文件路径
        if extracted and os.path.isfile(extracted):
            with open(extracted, 'r', encoding='utf-8', errors='replace') as f:
                content = f.read()
            from bs4 import BeautifulSoup
            soup = BeautifulSoup(content, 'lxml')
            return soup.get_text(separator='\n')
        # 如果 extracted 是目录，扫描其中的 HTML 文件
        if extracted and os.path.isdir(extracted):
            texts = []
            for fname in sorted(os.listdir(extracted)):
                if fname.endswith(('.html', '.htm', '.xhtml')):
                    with open(os.path.join(extracted, fname), 'r',
                              encoding='utf-8', errors='replace') as f:
                        content = f.read()
                    from bs4 import BeautifulSoup
                    soup = BeautifulSoup(content, 'lxml')
                    texts.append(soup.get_text(separator='\n'))
            return '\n\n'.join(texts)
        return ""
    except Exception:
        # Fallback: 某些 mobi 文件可能不兼容
        return ""
    finally:
        import shutil
        if os.path.isdir(tmpdir):
            shutil.rmtree(tmpdir, ignore_errors=True)


def extract_rtf(filepath: Path) -> str:
    """使用 striprtf 提取 RTF 全文。"""
    from striprtf.striprtf import rtf_to_text

    with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
        content = f.read()
    return rtf_to_text(content)


def extract_fb2(filepath: Path) -> str:
    """从 FB2 (XML) 中提取文本。"""
    tree = ET.parse(str(filepath))
    root = tree.getroot()

    # FB2 namespace
    ns = ''
    if root.tag.startswith('{'):
        ns = root.tag.split('}')[0] + '}'

    texts = []
    for body in root.iter(f'{ns}body'):
        for p in body.iter(f'{ns}p'):
            text = ''.join(p.itertext()).strip()
            if text:
                texts.append(text)
    return '\n\n'.join(texts)


def extract_txt(filepath: Path) -> str:
    """读取纯文本文件。"""
    # 尝试 UTF-8，fallback to latin-1
    for enc in ('utf-8', 'utf-8-sig', 'latin-1', 'gbk'):
        try:
            with open(filepath, 'r', encoding=enc) as f:
                return f.read()
        except (UnicodeDecodeError, UnicodeError):
            continue
    return ""


# ---------------------------------------------------------------------------
# 格式分发
# ---------------------------------------------------------------------------

EXTRACTORS = {
    '.pdf': extract_pdf,
    '.epub': extract_epub,
    '.mobi': extract_mobi,
    '.rtf': extract_rtf,
    '.fb2': extract_fb2,
    '.txt': extract_txt,
}

UNSUPPORTED = {'.azw3'}


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------

def process_file(filepath: Path) -> dict:
    """处理单个文件，返回结果记录。"""
    ext = filepath.suffix.lower()
    result = {
        "file": filepath.name,
        "ext": ext,
        "domain": author_to_domain(filepath.name),
        "status": "pending",
        "chars": 0,
        "output": "",
        "error": "",
    }

    if ext in UNSUPPORTED:
        result["status"] = "skipped"
        result["error"] = f"Unsupported format: {ext}"
        return result

    extractor = EXTRACTORS.get(ext)
    if not extractor:
        result["status"] = "skipped"
        result["error"] = f"No extractor for: {ext}"
        return result

    try:
        raw_text = extractor(filepath)
        text = clean_text(raw_text)
        result["chars"] = len(text)

        if len(text) < 100:
            result["status"] = "failed"
            result["error"] = "Extracted text too short (<100 chars)"
            return result

        # 输出路径
        domain = result["domain"]
        out_dir = CORPORA_DIR / domain
        out_dir.mkdir(parents=True, exist_ok=True)

        out_name = filepath.stem + ".md"
        out_path = out_dir / out_name
        out_path.write_text(text, encoding='utf-8')

        result["status"] = "ok"
        result["output"] = str(out_path.relative_to(PROJECT_ROOT))
    except Exception as e:
        result["status"] = "failed"
        result["error"] = f"{type(e).__name__}: {str(e)[:200]}"
        traceback.print_exc()

    return result


def main():
    """扫描 tmp/downloads/ 并批量提取。"""
    if not DOWNLOADS_DIR.exists():
        print(f"ERROR: Downloads directory not found: {DOWNLOADS_DIR}")
        sys.exit(1)

    files = sorted(DOWNLOADS_DIR.iterdir())
    # 过滤掉目录和 failed.txt
    files = [f for f in files if f.is_file() and f.name != 'failed.txt']

    print(f"Found {len(files)} files to process")
    print(f"Output directory: {CORPORA_DIR}")
    print()

    results = []
    ok_count = 0
    skip_count = 0
    fail_count = 0
    total_chars = 0

    for i, filepath in enumerate(files, 1):
        print(f"[{i}/{len(files)}] {filepath.name} ... ", end='', flush=True)
        t0 = time.time()

        result = process_file(filepath)
        results.append(result)

        elapsed = time.time() - t0

        if result["status"] == "ok":
            ok_count += 1
            total_chars += result["chars"]
            print(f"OK ({result['chars']:,} chars, {elapsed:.1f}s) → {result['domain']}/")
        elif result["status"] == "skipped":
            skip_count += 1
            print(f"SKIP ({result['error']})")
        else:
            fail_count += 1
            print(f"FAIL ({result['error'][:80]})")

    # 汇总
    print()
    print("=" * 60)
    print(f"Total: {len(files)} files")
    print(f"  OK:      {ok_count}")
    print(f"  Skipped: {skip_count}")
    print(f"  Failed:  {fail_count}")
    print(f"  Total chars extracted: {total_chars:,}")
    print()

    # 按域统计
    domain_stats: dict[str, dict] = {}
    for r in results:
        if r["status"] == "ok":
            d = r["domain"]
            if d not in domain_stats:
                domain_stats[d] = {"count": 0, "chars": 0}
            domain_stats[d]["count"] += 1
            domain_stats[d]["chars"] += r["chars"]

    print("Per-domain breakdown:")
    for d in sorted(domain_stats.keys()):
        s = domain_stats[d]
        print(f"  {d:20s} {s['count']:3d} files, {s['chars']:>12,} chars")

    # 保存报告
    report = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "total_files": len(files),
        "ok": ok_count,
        "skipped": skip_count,
        "failed": fail_count,
        "total_chars": total_chars,
        "domain_stats": domain_stats,
        "results": results,
    }
    REPORT_PATH.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
    print(f"\nReport saved to: {REPORT_PATH}")


if __name__ == "__main__":
    main()
