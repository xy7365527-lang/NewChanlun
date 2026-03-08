"""text_corpus_loader.py — 文本语料加载器。

从目录加载文本文件（.txt, .md），按段落分割后调用 ingest_text_passage
将术语共现和 surface forms 写入 S_net。

不修改 K_active。文本语料只丰富 S_net 的语言材料——
K_active 的修改由 articulation feedback 在穿越中完成。

语料目录结构：
  signifier_net/corpora/
    hegel/
      phenomenology_of_spirit.md
    marx/
      capital_vol1.md
    ...

认识论等级：L0（文件读取 + 段落分割 + 委托给 ingest_text_passage）

谱系引用：此模块由 v198-swarm/text-ingest-pipeline 创建。
"""

from __future__ import annotations

import sys
from pathlib import Path

from signifier_net import SNet
from signifier_net_ingest import ingest_text_passage, _split_paragraphs


_SUPPORTED_EXTENSIONS = {".txt", ".md"}


def load_text_file(
    snet: SNet,
    file_path: str | Path,
    domain: str,
) -> tuple[SNet, list[dict]]:
    """加载单个文本文件，逐段落 ingest 到 S_net。

    参数：
      snet:      当前 S_net 实例（不会被修改）
      file_path: 文本文件路径
      domain:    语料域标记（如 "hegel"）

    返回：
      (new_snet, all_log_entries)

    认识论等级：L0
    """
    path = Path(file_path)
    if not path.exists():
        raise FileNotFoundError(f"文本文件不存在: {path}")

    if path.suffix.lower() not in _SUPPORTED_EXTENSIONS:
        return snet, []

    text = path.read_text(encoding="utf-8")
    if not text.strip():
        return snet, []

    paragraphs = _split_paragraphs(text)
    if not paragraphs:
        return snet, []

    source = path.name
    new_snet = snet
    all_log_entries: list[dict] = []

    for para in paragraphs:
        new_snet, entries = ingest_text_passage(
            new_snet, para, domain, source,
        )
        all_log_entries.extend(entries)

    return new_snet, all_log_entries


def load_text_corpus(
    snet: SNet,
    corpus_dir: str | Path,
    domain: str,
) -> tuple[SNet, list[dict]]:
    """加载目录下的所有文本文件，逐段落 ingest 到 S_net。

    遍历 corpus_dir 下的所有 .txt 和 .md 文件（非递归），
    按文件名排序（确定性顺序）处理。

    参数：
      snet:       当前 S_net 实例（不会被修改）
      corpus_dir: 语料目录路径
      domain:     语料域标记（如 "hegel"）

    返回：
      (new_snet, all_log_entries)

    认识论等级：L0
    """
    dir_path = Path(corpus_dir)
    if not dir_path.is_dir():
        return snet, []

    text_files = sorted(
        f for f in dir_path.iterdir()
        if f.is_file() and f.suffix.lower() in _SUPPORTED_EXTENSIONS
    )

    if not text_files:
        return snet, []

    new_snet = snet
    all_log_entries: list[dict] = []

    for text_file in text_files:
        try:
            new_snet, entries = load_text_file(
                new_snet, text_file, domain,
            )
            all_log_entries.extend(entries)
        except Exception as exc:
            print(
                f"文本摄入失败 ({text_file.name}): {exc}",
                file=sys.stderr,
            )

    return new_snet, all_log_entries


def format_corpus_report(
    domain: str,
    log_entries: list[dict],
) -> str:
    """格式化语料摄入统计报告。"""
    cooccurrence_count = sum(
        1 for e in log_entries if e.get("type") == "cooccurrence"
    )
    unique_pairs = len({
        (e["source"], e["target"])
        for e in log_entries
        if e.get("type") == "cooccurrence"
    })

    return (
        f"  corpus:{domain}: "
        f"{len(log_entries)} log entries, "
        f"{cooccurrence_count} cooccurrences, "
        f"{unique_pairs} unique pairs"
    )
