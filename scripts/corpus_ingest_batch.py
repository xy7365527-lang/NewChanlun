"""corpus_ingest_batch.py — 独立批量语料摄入脚本。

从 signifier_net/corpora/ 下的所有 .md 文件批量摄入到 S_net。

流程：
  1. Bootstrap SNet (Layer A from K_active + Layer B/C)
  2. Ingest dictionaries (同 daemon 启动流程)
  3. Ingest all text corpora (334 files, 283MB)
  4. 输出统计报告

支持：
  - --dry-run: 只扫描文件不摄入，报告文件列表和大小
  - --domain X: 只摄入指定域（如 --domain hegel）
  - --limit N: 每个域最多处理 N 个文件
  - --resume: 跳过上次已完成的文件（基于 resume log）

认识论等级：L0（文件读取 + 段落分割 + 确定性字符串匹配）
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

# Add topological-computation to path
SCRIPT_DIR = Path(__file__).resolve().parent
TOPO_DIR = SCRIPT_DIR.parent / "topological-computation"
sys.path.insert(0, str(TOPO_DIR))

from signifier_net import SNet


CORPORA_ROOT = TOPO_DIR / "signifier_net" / "corpora"
RESUME_LOG = SCRIPT_DIR / ".corpus_ingest_resume.json"


def scan_corpora(
    root: Path,
    domain_filter: str | None = None,
) -> list[tuple[str, Path]]:
    """扫描语料目录，返回 (domain, file_path) 列表。"""
    results: list[tuple[str, Path]] = []
    if not root.is_dir():
        return results

    domain_dirs = sorted(
        d for d in root.iterdir()
        if d.is_dir()
    )

    for domain_dir in domain_dirs:
        domain = domain_dir.name
        if domain_filter and domain != domain_filter:
            continue

        files = sorted(
            f for f in domain_dir.iterdir()
            if f.is_file() and f.suffix.lower() in {".txt", ".md"}
        )
        for f in files:
            results.append((domain, f))

    return results


def dry_run(corpus_files: list[tuple[str, Path]]) -> None:
    """只扫描，不摄入。"""
    total_size = 0
    domain_stats: dict[str, dict] = {}

    for domain, fpath in corpus_files:
        size = fpath.stat().st_size
        total_size += size
        if domain not in domain_stats:
            domain_stats[domain] = {"count": 0, "size": 0}
        domain_stats[domain]["count"] += 1
        domain_stats[domain]["size"] += size

    print("=== 语料扫描报告 (dry-run) ===")
    for domain in sorted(domain_stats):
        s = domain_stats[domain]
        print(f"  {domain}: {s['count']} 文件, {s['size'] / 1024 / 1024:.1f} MB")

    print(f"\n  合计: {len(corpus_files)} 文件, {total_size / 1024 / 1024:.1f} MB")


def load_resume_log() -> set[str]:
    """加载断点续传日志。"""
    if not RESUME_LOG.exists():
        return set()
    try:
        data = json.loads(RESUME_LOG.read_text(encoding="utf-8"))
        return set(data.get("completed", []))
    except (json.JSONDecodeError, KeyError):
        return set()


def save_resume_log(completed: set[str]) -> None:
    """保存断点续传日志。"""
    RESUME_LOG.write_text(
        json.dumps({"completed": sorted(completed)}, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def bootstrap_snet() -> SNet:
    """Bootstrap SNet + dictionaries (同 daemon 流程)。

    Graph loading priority:
      1. graph_*.json files in data/ (same as daemon._load_experiment_graphs)
      2. Empty graph (graceful degradation)
    """
    from engine import Graph
    from snet_bootstrap import bootstrap_snet as _bootstrap
    from daemon import _load_experiment_graphs

    # Load experiment graphs (same method as daemon)
    t0 = time.time()
    k_active, concept_names = _load_experiment_graphs()
    n_active = len(k_active.active_vertex_ids())
    print(f"K_active loaded: {n_active} active vertices ({time.time() - t0:.1f}s)", file=sys.stderr)

    # Bootstrap SNet (Layer A + B + C)
    t0 = time.time()
    sf_path = TOPO_DIR / "data" / "chanlun_surface_forms.jsonl"
    snet, stats = _bootstrap(
        graph=k_active,
        surface_forms_path=str(sf_path) if sf_path.exists() else None,
        pmi_threshold=0.0,
    )
    print(f"SNet bootstrap: {len(snet.signifiers)} signifiers, {len(snet.edges)} edges ({time.time() - t0:.1f}s)", file=sys.stderr)

    # Ingest dictionaries (unified loop from signifier_net_ingest)
    t0 = time.time()
    try:
        from signifier_net_ingest import ingest_all_dict_types

        dict_dir = TOPO_DIR / "signifier_net" / "dictionaries"
        snet = ingest_all_dict_types(snet, dict_dir, output=sys.stderr)
        print(f"SNet after dict ingest: {len(snet.signifiers)} signifiers, {len(snet.edges)} edges ({time.time() - t0:.1f}s)", file=sys.stderr)
    except Exception as exc:
        print(f"Dictionary ingest failed: {exc}", file=sys.stderr)

    return snet


def ingest_corpora(
    snet: SNet,
    corpus_files: list[tuple[str, Path]],
    resume: bool = False,
    limit_per_domain: int | None = None,
) -> tuple[SNet, dict]:
    """批量摄入语料。"""
    from text_corpus_loader import load_text_file, format_corpus_report

    completed = load_resume_log() if resume else set()
    domain_counts: dict[str, int] = {}

    total_files = len(corpus_files)
    total_entries = 0
    total_skipped = 0
    total_errors = 0
    files_processed = 0

    t_start = time.time()

    for i, (domain, fpath) in enumerate(corpus_files):
        file_key = f"{domain}/{fpath.name}"

        # Resume: skip already completed
        if resume and file_key in completed:
            total_skipped += 1
            continue

        # Limit per domain
        if limit_per_domain is not None:
            domain_counts.setdefault(domain, 0)
            if domain_counts[domain] >= limit_per_domain:
                continue

        t_file = time.time()
        try:
            snet, entries = load_text_file(snet, fpath, domain)

            elapsed = time.time() - t_file
            files_processed += 1
            total_entries += len(entries)
            domain_counts[domain] = domain_counts.get(domain, 0) + 1

            # Mark completed for resume
            completed.add(file_key)
            if files_processed % 10 == 0:
                save_resume_log(completed)

            size_mb = fpath.stat().st_size / 1024 / 1024
            print(
                f"  [{files_processed}/{total_files}] {file_key}: "
                f"{len(entries)} entries, {size_mb:.1f} MB, {elapsed:.1f}s",
            )

        except Exception as exc:
            total_errors += 1
            print(f"  [{i+1}/{total_files}] {file_key}: ERROR - {exc}", file=sys.stderr)

    # Final save
    save_resume_log(completed)

    elapsed_total = time.time() - t_start
    summary = {
        "total_files": total_files,
        "files_processed": files_processed,
        "files_skipped": total_skipped,
        "files_errors": total_errors,
        "total_entries": total_entries,
        "elapsed_seconds": elapsed_total,
        "final_signifiers": len(snet.signifiers),
        "final_edges": len(snet.edges),
    }

    return snet, summary


def main() -> None:
    parser = argparse.ArgumentParser(description="S_net 批量语料摄入")
    parser.add_argument("--dry-run", action="store_true", help="只扫描不摄入")
    parser.add_argument("--domain", type=str, default=None, help="只摄入指定域")
    parser.add_argument("--limit", type=int, default=None, help="每域最多处理N个文件")
    parser.add_argument("--resume", action="store_true", help="断点续传")
    args = parser.parse_args()

    # Scan corpus files
    corpus_files = scan_corpora(CORPORA_ROOT, domain_filter=args.domain)
    if not corpus_files:
        print("未找到语料文件")
        return

    print(f"扫描到 {len(corpus_files)} 个语料文件")

    if args.dry_run:
        dry_run(corpus_files)
        return

    # Bootstrap SNet
    print("\n=== Bootstrap SNet ===")
    snet = bootstrap_snet()

    # Ingest corpora
    print(f"\n=== 开始批量摄入 ({len(corpus_files)} files) ===")
    snet, summary = ingest_corpora(
        snet, corpus_files,
        resume=args.resume,
        limit_per_domain=args.limit,
    )

    # Report
    print(f"\n=== 摄入完成 ===")
    print(f"  处理: {summary['files_processed']}/{summary['total_files']} 文件")
    if summary['files_skipped']:
        print(f"  跳过(已完成): {summary['files_skipped']}")
    if summary['files_errors']:
        print(f"  错误: {summary['files_errors']}")
    print(f"  共现条目: {summary['total_entries']}")
    print(f"  耗时: {summary['elapsed_seconds']:.1f}s")
    print(f"  最终 SNet: {summary['final_signifiers']} signifiers, {summary['final_edges']} edges")


if __name__ == "__main__":
    main()
