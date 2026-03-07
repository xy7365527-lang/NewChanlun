"""从缠论 108 课原文中提取 surface forms（概念间的语言连接）。

对每个句子：
1. 白名单匹配找到句中的缠论术语
2. 句子含 >= 2 术语 -> 提取两者之间的文本为 surface form
3. 记录 (term_a, term_b, surface, context, source_file)

输出: data/chanlun_surface_forms.jsonl
"""

from __future__ import annotations

import json
import os
import re
import sys
from dataclasses import dataclass, asdict
from pathlib import Path

# ---------------------------------------------------------------------------
# 术语白名单（按长度降序排列，保证最长匹配优先）
# ---------------------------------------------------------------------------

TERMS: list[str] = sorted([
    # 基础构件
    "K线", "分型", "顶分型", "底分型", "笔", "线段",
    # 中枢相关
    "中枢", "走势中枢", "中枢扩展", "中枢新生", "中枢扩张", "中枢延伸", "中枢震荡",
    # 走势相关
    "走势", "走势类型", "走势必完美", "走势终完美",
    "盘整", "趋势", "上涨", "下跌",
    # 背驰相关
    "背驰", "盘整背驰", "趋势背驰", "背驰段",
    # 买卖点
    "买点", "卖点",
    "第一类买点", "第二类买点", "第三类买点",
    "第一类卖点", "第二类卖点", "第三类卖点",
    "买卖点",
    # 区间套/级别/递归
    "区间套", "级别", "递归",
    # 特征序列
    "特征序列", "标准特征序列",
    # 包含关系
    "包含关系", "包含处理",
    # 结合律 / 分解
    "结合律", "走势分解", "同级别分解", "多义性",
    # 力度 / 指标
    "力度", "MACD", "面积", "斜率",
    # 次级别
    "次级别", "次级别走势", "次级别走势类型",
    # 缺口
    "缺口",
    # 转折（只保留复合形式，避免"转折"单独命中普通语境）
    "转折点", "大转折",
    # 新笔 / 旧笔
    "新笔", "旧笔",
    # 古怪线段
    "古怪线段",
    # 分解定理
    "走势分解定理",
    # 中心定理
    "中心定理",
    # 技术分析基本原理
    "技术分析基本原理",
    # 区间
    "ZG", "ZD", "GG", "DD",
    # 走势连接
    "走势类型连接",
    # 第一种 / 第二种（特征序列分型情况）
    "第一种", "第二种",
    # 笔破坏 / 线段破坏
    "笔破坏", "线段破坏",
    # 回抽 / 回调
    "回抽", "回调",
    # 延伸
    "中枢延伸",
    # 走势完美 / 走势完成
    "走势终完美", "走势必完美",
    # 高低点
    "高点", "低点",
], key=len, reverse=True)


@dataclass(frozen=True)
class SurfaceForm:
    """一条 surface form 记录。"""
    term_a: str
    term_b: str
    surface: str
    context: str
    source_file: str
    lesson: str


# ---------------------------------------------------------------------------
# 分句
# ---------------------------------------------------------------------------

_SENT_DELIMITERS = re.compile(r'[。！？\n]+')
_CLAUSE_DELIMITERS = re.compile(r'[，；,;：]+')


def split_sentences(text: str) -> list[str]:
    """按句号/问号/叹号/换行切分为句子。"""
    parts = _SENT_DELIMITERS.split(text.strip())
    return [s.strip() for s in parts if s.strip() and len(s.strip()) >= 4]


# ---------------------------------------------------------------------------
# 术语匹配：在句子中找到所有术语出现（位置 + 术语）
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class TermHit:
    start: int
    end: int
    term: str


def find_terms(sentence: str) -> list[TermHit]:
    """在句子中找到所有术语出现位置，最长匹配优先，不重叠。"""
    hits: list[TermHit] = []
    occupied: set[int] = set()

    for term in TERMS:
        start = 0
        while True:
            idx = sentence.find(term, start)
            if idx < 0:
                break
            positions = set(range(idx, idx + len(term)))
            if not positions & occupied:
                hits.append(TermHit(start=idx, end=idx + len(term), term=term))
                occupied |= positions
            start = idx + 1

    hits.sort(key=lambda h: h.start)
    return hits


# ---------------------------------------------------------------------------
# 提取 surface form
# ---------------------------------------------------------------------------

# 纯虚词 surface：这些作为两个术语之间的唯一连接词时不携带有意义的关系信息
_PURE_FUNCTION_WORDS = frozenset({
    "的", "与", "或", "和", "及", "到", "为", "是",
    "了", "在", "中", "上", "下", "就", "都", "也",
    "而", "但", "又", "还", "则", "所", "以", "之",
    "其", "某", "该", "各", "每", "有", "那", "这",
    "+", "＋",
    # 术语碎片残余（原文走势类型 / 类型走势 交替写法导致的匹配残余）
    "类型", "类", "的类", "的类型",
    # 句子分割残余
    "买、", "卖、",
    # 数字修饰词残余
    "一个", "两个", "三个",
    # 固定短语碎片
    "转大",  # "小转大" 中 "小" 未被匹配
})


def _is_noise(surface: str) -> bool:
    """过滤无意义的 surface：纯标点、纯空白、太长、纯虚词、单字残余。"""
    cleaned = re.sub(r'[\s，,、：:；;\"""\'\'\"\"（）()\[\]【】《》—\-·]', '', surface)
    if not cleaned:
        return True
    if len(cleaned) > 40:
        return True
    # 纯数字 / 纯标点
    if re.fullmatch(r'[\d.\s]+', cleaned):
        return True
    # 纯虚词（单个虚词或几个虚词的组合）
    if cleaned in _PURE_FUNCTION_WORDS:
        return True
    # 单字残余（一个非术语的单字不构成有意义的 surface）
    if len(cleaned) == 1 and cleaned not in ("不", "无", "非"):
        return True
    return False


def extract_from_sentence(
    sentence: str,
    source_file: str,
    lesson: str,
) -> list[SurfaceForm]:
    """从单个句子中提取所有 (term_a, term_b, surface) 三元组。"""
    hits = find_terms(sentence)
    if len(hits) < 2:
        return []

    results: list[SurfaceForm] = []

    for i in range(len(hits) - 1):
        a = hits[i]
        b = hits[i + 1]

        # 提取两个术语之间的文本
        between = sentence[a.end:b.start].strip()

        # 直接相邻的术语：跳过（通常是最长匹配分裂的残余）
        if not between:
            continue

        # 跳过纯噪声
        if _is_noise(between):
            continue

        results.append(SurfaceForm(
            term_a=a.term,
            term_b=b.term,
            surface=between,
            context=sentence.strip(),
            source_file=source_file,
            lesson=lesson,
        ))

    return results


# ---------------------------------------------------------------------------
# 处理单个文件
# ---------------------------------------------------------------------------

def _clean_markdown(text: str) -> str:
    """去除 Markdown 图片链接、HTML 标签，保留正文。"""
    # 去掉图片标签
    text = re.sub(r'!\[.*?\]\(.*?\)', '', text)
    # 去掉普通链接但保留文本
    text = re.sub(r'\[([^\]]*)\]\([^)]*\)', r'\1', text)
    # 去掉 HTML 标签
    text = re.sub(r'<[^>]+>', '', text)
    # 去掉 Markdown 标题符号
    text = re.sub(r'^#+\s*', '', text, flags=re.MULTILINE)
    # 去掉引用符号
    text = re.sub(r'^>\s*', '', text, flags=re.MULTILINE)
    # 去掉 LaTeX 公式标记
    text = re.sub(r'\\\(', '', text)
    text = re.sub(r'\\\)', '', text)
    return text


def _extract_lesson_id(filename: str) -> str:
    """从文件名提取课号。如 '017-第17课.md' -> '第17课'。"""
    m = re.search(r'(第\d+课)', filename)
    if m:
        return m.group(1)
    m = re.search(r'(序篇)', filename)
    if m:
        return m.group(1)
    return Path(filename).stem


def process_file(filepath: str) -> list[SurfaceForm]:
    """处理单个博客 Markdown 文件，返回所有 surface forms。"""
    with open(filepath, 'r', encoding='utf-8') as f:
        raw = f.read()

    text = _clean_markdown(raw)
    filename = os.path.basename(filepath)
    lesson = _extract_lesson_id(filename)
    source = filename

    sentences = split_sentences(text)
    all_forms: list[SurfaceForm] = []

    for sent in sentences:
        forms = extract_from_sentence(sent, source, lesson)
        all_forms.extend(forms)

    return all_forms


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------

def main():
    blog_dir = Path(__file__).parent.parent / "docs" / "chanlun" / "text" / "blog"
    out_dir = Path(__file__).parent / "data"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "chanlun_surface_forms.jsonl"

    # 收集所有 blog 文件
    md_files = sorted(blog_dir.glob("*.md"))

    # 如果命令行传了 --test，只跑前 5 课
    test_mode = "--test" in sys.argv
    if test_mode:
        # 跑第 17、24、33、36、37 课（概念密度较高的课程）
        test_lessons = {"017", "024", "033", "036", "037"}
        md_files = [f for f in md_files if any(f.name.startswith(t) for t in test_lessons)]
        print(f"[TEST MODE] 选中 {len(md_files)} 个文件: {[f.name for f in md_files]}")

    total_forms = 0
    total_files = 0
    all_results: list[SurfaceForm] = []

    for fp in md_files:
        forms = process_file(str(fp))
        if forms:
            total_files += 1
            total_forms += len(forms)
            all_results.extend(forms)
            if test_mode:
                print(f"  {fp.name}: {len(forms)} surface forms")

    # 写出 JSONL
    with open(out_path, 'w', encoding='utf-8') as f:
        for form in all_results:
            f.write(json.dumps(asdict(form), ensure_ascii=False) + '\n')

    print(f"\n完成: {total_files} 个文件, {total_forms} 条 surface forms")
    print(f"输出: {out_path}")

    # 打印统计
    if test_mode and all_results:
        print("\n--- 样本 (前 20 条) ---")
        for form in all_results[:20]:
            print(f"  [{form.term_a}] --({form.surface})--> [{form.term_b}]")
            print(f"    context: {form.context[:80]}...")
            print()

        # 术语频率统计
        from collections import Counter
        term_counts: Counter[str] = Counter()
        for form in all_results:
            term_counts[form.term_a] += 1
            term_counts[form.term_b] += 1
        print("--- 术语出现频率 (top 20) ---")
        for term, count in term_counts.most_common(20):
            print(f"  {term}: {count}")

        # surface 频率统计
        surface_counts: Counter[str] = Counter()
        for form in all_results:
            surface_counts[form.surface] += 1
        print("\n--- surface 频率 (top 20) ---")
        for surface, count in surface_counts.most_common(20):
            print(f"  '{surface}': {count}")


if __name__ == "__main__":
    main()
