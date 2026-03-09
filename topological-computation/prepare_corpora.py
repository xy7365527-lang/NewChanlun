"""prepare_corpora.py — 预处理原文文本，输出到 signifier_net/corpora/ 目录。

将 docs/chanlun/text/blog/*.md（缠论108课）清理后放入
signifier_net/corpora/chanlun/，供 daemon 启动时 text_corpus_loader 自动加载。

清理规则：
  - 去除 YAML-like metadata（> 来源：、> 发表日期：）
  - 去除 markdown 图片标签 ![alt](url)
  - 去除 markdown 分隔线（---、---------↑正文---------）
  - 去除重复标题行（保留第一个标题作为段落上下文）
  - 去除作者行（作者：缠中说禅(...)）
  - 去除 HTML 标签
  - 保留纯文本段落

认识论等级：L0（确定性文本转换）

用法：
  python prepare_corpora.py                    # 预处理缠论108课
  python prepare_corpora.py --dry-run          # 仅统计，不写入
  python prepare_corpora.py --source-dir PATH  # 自定义源目录
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


# 项目根目录和默认路径
_SCRIPT_DIR = Path(__file__).parent
_DEFAULT_SOURCE_DIR = _SCRIPT_DIR.parent / "docs" / "chanlun" / "text" / "blog"
_DEFAULT_OUTPUT_DIR = _SCRIPT_DIR / "signifier_net" / "corpora" / "chanlun"


def clean_chanlun_md(text: str) -> str:
    """清理缠论课文 markdown，返回纯文本。

    清理规则按顺序应用，确保各模式不互相干扰。

    认识论等级：L0（确定性字符串操作）
    """
    lines = text.split("\n")
    cleaned: list[str] = []
    seen_title = False
    in_metadata = False

    for line in lines:
        stripped = line.strip()

        # 跳过空行之前的 metadata 区域
        if stripped.startswith("> 来源：") or stripped.startswith("> 发表日期："):
            in_metadata = True
            continue

        # 跳过分隔线
        if re.match(r"^-{3,}", stripped):
            in_metadata = False
            continue

        # 跳过尾部分隔线标记
        if re.match(r"^-+↑正文-+$", stripped):
            continue

        # 跳过图片标签（整行是图片）
        if re.match(r"^!\[.*?\]\(.*?\)$", stripped):
            continue

        # 跳过作者行
        if re.match(r"^作者：缠中说禅\(", stripped):
            continue

        # 标题处理：保留第一个标题，跳过重复标题
        if stripped.startswith("# "):
            if not seen_title:
                seen_title = True
                # 保留标题内容但去除 # 标记
                title_text = stripped.lstrip("# ").strip()
                cleaned.append(title_text)
                cleaned.append("")
            continue

        # 去除行内图片标签
        line_cleaned = re.sub(r"!\[.*?\]\(.*?\)", "", stripped)

        # 去除 HTML 标签
        line_cleaned = re.sub(r"<[^>]+>", "", line_cleaned)

        # 去除 markdown 粗体/斜体标记
        line_cleaned = re.sub(r"\*{1,3}([^*]+)\*{1,3}", r"\1", line_cleaned)

        # 去除 markdown 链接但保留文本
        line_cleaned = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", line_cleaned)

        cleaned.append(line_cleaned)

    # 合并连续空行为单个空行
    result: list[str] = []
    prev_empty = False
    for line in cleaned:
        if not line.strip():
            if not prev_empty:
                result.append("")
            prev_empty = True
        else:
            result.append(line)
            prev_empty = False

    # 去除首尾空行
    text_out = "\n".join(result).strip()

    return text_out


def prepare_chanlun_corpora(
    source_dir: Path = _DEFAULT_SOURCE_DIR,
    output_dir: Path = _DEFAULT_OUTPUT_DIR,
    dry_run: bool = False,
) -> dict:
    """预处理缠论108课，输出到 corpora/chanlun/ 目录。

    返回统计报告。
    """
    if not source_dir.is_dir():
        raise FileNotFoundError(f"源目录不存在: {source_dir}")

    # 列出所有课文文件（排除 INDEX.md）
    md_files = sorted(
        f for f in source_dir.iterdir()
        if f.is_file()
        and f.suffix.lower() == ".md"
        and f.name != "INDEX.md"
    )

    if not md_files:
        raise FileNotFoundError(f"源目录中没有 .md 文件: {source_dir}")

    if not dry_run:
        output_dir.mkdir(parents=True, exist_ok=True)

    stats = {
        "source_dir": str(source_dir),
        "output_dir": str(output_dir),
        "files_total": len(md_files),
        "files_processed": 0,
        "files_skipped": 0,
        "total_chars_in": 0,
        "total_chars_out": 0,
        "total_paragraphs": 0,
        "dry_run": dry_run,
    }

    for md_file in md_files:
        raw_text = md_file.read_text(encoding="utf-8")
        stats["total_chars_in"] += len(raw_text)

        cleaned = clean_chanlun_md(raw_text)
        if not cleaned.strip():
            stats["files_skipped"] += 1
            continue

        stats["total_chars_out"] += len(cleaned)

        # 统计段落数（与 _split_paragraphs 的逻辑一致：空行分割，>= 10 字符）
        paragraphs = [
            p.strip() for p in cleaned.split("\n\n")
            if len(p.strip()) >= 10
        ]
        stats["total_paragraphs"] += len(paragraphs)

        if not dry_run:
            out_path = output_dir / md_file.name
            out_path.write_text(cleaned, encoding="utf-8")

        stats["files_processed"] += 1

    return stats


def format_stats(stats: dict) -> str:
    """格式化统计报告。"""
    ratio = (
        stats["total_chars_out"] / stats["total_chars_in"] * 100
        if stats["total_chars_in"] > 0
        else 0
    )
    lines = [
        "=== 语料预处理报告 ===",
        f"  源目录: {stats['source_dir']}",
        f"  输出目录: {stats['output_dir']}",
        f"  文件总数: {stats['files_total']}",
        f"  已处理: {stats['files_processed']}",
        f"  跳过（空内容）: {stats['files_skipped']}",
        f"  输入字符数: {stats['total_chars_in']:,}",
        f"  输出字符数: {stats['total_chars_out']:,} ({ratio:.1f}%)",
        f"  段落总数: {stats['total_paragraphs']:,}",
    ]
    if stats["dry_run"]:
        lines.append("  [dry-run 模式，未写入文件]")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="预处理缠论课文，输出到 signifier_net/corpora/"
    )
    parser.add_argument(
        "--source-dir",
        default=str(_DEFAULT_SOURCE_DIR),
        help=f"源目录路径 (default: {_DEFAULT_SOURCE_DIR})",
    )
    parser.add_argument(
        "--output-dir",
        default=str(_DEFAULT_OUTPUT_DIR),
        help=f"输出目录路径 (default: {_DEFAULT_OUTPUT_DIR})",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="仅统计，不写入文件",
    )

    args = parser.parse_args()

    stats = prepare_chanlun_corpora(
        source_dir=Path(args.source_dir),
        output_dir=Path(args.output_dir),
        dry_run=args.dry_run,
    )
    print(format_stats(stats))


if __name__ == "__main__":
    main()
